//! The service state and request handling, independent of the transport.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, RwLock};
use std::time::Duration;

use al_dir_core::lock::{self, Challenge, LockContext, UnlockOutcome};
use al_dir_core::{
    import, BlockSet, Config, CoreError, Ctx, Decision, Engine, GeoArea, Hit, LocationFix, Lock, LockKind, NoLists,
    Now, PasswordHash, ProcessInfo, SetId,
};
use al_dir_ipc::{
    BlockEvent, ChallengeInfo, CheckQuery, DecisionInfo, ErrorCode, EventKind, Request, Response, SetInfo,
    PageInfo, Status, UnlockInfo, PROTOCOL_VERSION,
};

const MAX_EVENTS: usize = 500;
const PROVEN_FILE: &str = "extensions-proven";

/// Where the machine is, as far as the platform can tell.
#[derive(Debug, Clone)]
pub struct LocationState {
    pub fix: LocationFix,
    /// Areas known to contain the machine from learned Wi-Fi networks.
    pub inside: Vec<GeoArea>,
    /// Areas known not to: Wi-Fi works but none of their networks is near.
    pub outside: Vec<GeoArea>,
}

impl Default for LocationState {
    fn default() -> Self {
        Self { fix: LocationFix::Unknown, inside: Vec::new(), outside: Vec::new() }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LockSummary {
    pub any_locked: bool,
    /// Areas of all location locks, locked or not (the location watcher
    /// runs while there are any).
    pub location_areas: Vec<GeoArea>,
    pub location_locked: bool,
    pub hardened: bool,
}

fn describe_location(loc: &LocationState) -> String {
    let mut parts = Vec::new();
    if let LocationFix::Known { accuracy_m, .. } = loc.fix {
        parts.push(format!("position known (accuracy {accuracy_m:.0} m)"));
    }
    if !loc.inside.is_empty() {
        parts.push(format!("inside {} locked area(s) by Wi-Fi", loc.inside.len()));
    }
    if !loc.outside.is_empty() {
        parts.push(format!("away from {} locked area(s) by Wi-Fi", loc.outside.len()));
    }
    if parts.is_empty() { "unknown".into() } else { parts.join(", ") }
}

/// Platform hook: remember the Wi-Fi networks in range as belonging to an
/// area; returns how many are now known for it.
pub type Learner = Box<dyn Fn(&GeoArea) -> std::io::Result<usize> + Send + Sync>;

/// A browser extension connected through its native host.
#[derive(Debug, Clone)]
pub struct ExtensionSession {
    pub browser: String,
    pub user: String,
    /// Process id of the native host (a child of the browser).
    pub host_pid: i32,
}

use crate::storage::Store;

/// Who sent a request, as established by the transport.
#[derive(Debug, Clone)]
pub struct Caller {
    pub user: String,
    pub admin: bool,
}

pub struct Service {
    inner: Mutex<Inner>,
    /// What enforcement threads read. Kept apart from `inner` so they never
    /// wait for a settings write to reach the disk.
    snapshot: RwLock<Arc<Snapshot>>,
    location: RwLock<Arc<LocationState>>,
    learner: std::sync::OnceLock<Learner>,
    /// Bumped on every settings change.
    generation: AtomicU64,
    events: Mutex<VecDeque<BlockEvent>>,
    /// Live event feeds (session agents), by user.
    subscribers: Mutex<Vec<(String, mpsc::Sender<BlockEvent>)>>,
    /// Live browser-extension sessions.
    extensions: Mutex<HashMap<u64, ExtensionSession>>,
    next_session: AtomicU64,
    /// Recently reported block pages, to log each visit once.
    recent_pages: Mutex<HashMap<(String, String), std::time::Instant>>,
    store: Store,
    boot_id: String,
    started_at: i64,
}

struct Snapshot {
    engine: Arc<Engine>,
    names: HashMap<SetId, String>,
}

impl Snapshot {
    fn of(config: &Config) -> Arc<Self> {
        Arc::new(Snapshot {
            engine: Arc::new(Engine::new(config, &NoLists)),
            names: config.block_sets.iter().map(|s| (s.id, s.name.clone())).collect(),
        })
    }
}

struct Inner {
    config: Config,
    /// Random texts handed out for random-text locks, by set.
    challenges: HashMap<SetId, String>,
}

type Result<T> = std::result::Result<T, Response>;

impl Service {
    pub fn new(store: Store, boot_id: String) -> std::io::Result<Self> {
        let config = store.load()?;
        Ok(Self {
            snapshot: RwLock::new(Snapshot::of(&config)),
            location: RwLock::new(Arc::new(LocationState::default())),
            learner: std::sync::OnceLock::new(),
            inner: Mutex::new(Inner { config, challenges: HashMap::new() }),
            generation: AtomicU64::new(0),
            events: Mutex::new(VecDeque::new()),
            subscribers: Mutex::new(Vec::new()),
            extensions: Mutex::new(HashMap::new()),
            next_session: AtomicU64::new(1),
            recent_pages: Mutex::new(HashMap::new()),
            store,
            boot_id,
            started_at: crate::clock::now().unix,
        })
    }

    fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The current compiled engine; cheap to call on hot paths.
    pub fn engine(&self) -> Arc<Engine> {
        self.snapshot().engine.clone()
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Logs a block and keeps it for `RecentEvents`.
    pub fn record_block(&self, user: &str, kind: EventKind, target: &str, hit: &Hit) {
        let set_name = self.snapshot().names.get(&hit.set).cloned().unwrap_or_default();
        let verb = match kind {
            EventKind::AppStart => "stopped",
            EventKind::AppKilled => "closed",
            EventKind::WindowClosed => "closed window",
            EventKind::SiteBlocked => "blocked site",
            EventKind::PageBlocked => "showed block page for",
        };
        eprintln!("al-dir: {verb} {target} for {user} (block \"{set_name}\": {})", hit.reason);
        let event = BlockEvent {
            time: crate::clock::now().unix,
            user: user.to_string(),
            kind,
            target: target.to_string(),
            set: hit.set,
            set_name,
            reason: hit.reason.to_string(),
        };
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if events.len() == MAX_EVENTS {
            events.pop_front();
        }
        events.push_back(event.clone());
        drop(events);
        let mut subs = self.subscribers.lock().unwrap_or_else(|e| e.into_inner());
        subs.retain(|(u, tx)| *u != user || tx.send(event.clone()).is_ok());
    }

    /// A feed of `user`'s block events from now on.
    fn check_page(&self, caller: &Caller, url: &str, now: Now) -> PageInfo {
        let snapshot = self.snapshot();
        let ctx = Ctx::new(now, &caller.user);
        let effective = al_dir_core::web::interstitial_target(url);
        let Some(view) = al_dir_core::UrlView::parse(effective.as_deref().unwrap_or(url)) else {
            return PageInfo::default();
        };
        let decision = snapshot.engine.check_view(&view, &ctx);
        let hide = snapshot.engine.hidden_selectors(&view.host, &ctx).into_iter().map(String::from).collect();
        let Some(hit) = decision.block else {
            return PageInfo { hide, metered: !decision.metered.is_empty(), ..PageInfo::default() };
        };
        let key = (caller.user.clone(), url.to_string());
        let mut recent = self.recent_pages.lock().unwrap_or_else(|e| e.into_inner());
        recent.retain(|_, t| t.elapsed() < Duration::from_secs(10));
        if recent.insert(key, std::time::Instant::now()).is_none() {
            drop(recent);
            self.record_block(&caller.user, EventKind::PageBlocked, url, &hit);
        }
        PageInfo {
            blocked: true,
            set_name: snapshot.names.get(&hit.set).cloned(),
            reason: Some(hit.reason.to_string()),
            hide,
            metered: false,
        }
    }

    /// Registers a connected browser extension; returns its session id.
    pub fn add_extension(&self, session: ExtensionSession) -> u64 {
        let id = self.next_session.fetch_add(1, Ordering::Relaxed);
        self.extensions.lock().unwrap_or_else(|e| e.into_inner()).insert(id, session);
        id
    }

    pub fn remove_extension(&self, id: u64) {
        self.extensions.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
    }

    pub fn extensions(&self) -> Vec<ExtensionSession> {
        self.extensions.lock().unwrap_or_else(|e| e.into_inner()).values().cloned().collect()
    }

    /// Browsers (by family) for which the extension has worked on this
    /// machine. Only those are closed when running without it, so a broken
    /// extension install can never lock anyone out of browsing.
    pub fn extension_proven(&self, browser: &str) -> bool {
        self.store.read_aux(PROVEN_FILE).is_some_and(|t| t.lines().any(|l| l == browser))
    }

    pub fn mark_extension_proven(&self, browser: &str) {
        if self.extension_proven(browser) {
            return;
        }
        let mut text = self.store.read_aux(PROVEN_FILE).unwrap_or_default();
        text.push_str(browser);
        text.push('\n');
        if let Err(e) = self.store.write_aux(PROVEN_FILE, &text) {
            eprintln!("al-dir: {e}");
        }
    }

    pub fn subscribe(&self, user: &str) -> mpsc::Receiver<BlockEvent> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().unwrap_or_else(|e| e.into_inner()).push((user.to_string(), tx));
        rx
    }

    fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn location(&self) -> Arc<LocationState> {
        self.location.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn set_learner(&self, learner: Learner) {
        let _ = self.learner.set(learner);
    }

    /// Latest location facts, from the platform's location watcher.
    pub fn set_location(&self, state: LocationState) {
        *self.location.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(state);
    }

    fn lock_ctx<'a>(&'a self, now: Now, loc: &'a LocationState) -> LockContext<'a> {
        LockContext { now, boot_id: &self.boot_id, location: loc.fix, inside: &loc.inside, outside: &loc.outside }
    }

    /// What the platform must enforce around locks right now.
    pub fn lock_summary(&self) -> LockSummary {
        let loc = self.location();
        let ctx = self.lock_ctx(crate::clock::now(), &loc);
        let inner = self.lock_inner();
        let mut summary = LockSummary { hardened: inner.config.hardened, ..LockSummary::default() };
        for s in &inner.config.block_sets {
            let locked = lock::is_locked(s, &ctx);
            summary.any_locked |= locked;
            if let Some(Lock { kind: LockKind::Location { area }, .. }) = &s.lock {
                summary.location_areas.push(area.clone());
                summary.location_locked |= locked;
            }
        }
        summary
    }

    /// Applies `change` to a copy of the config, persists it, and only then
    /// makes it current. On failure nothing changes.
    fn commit<T>(
        &self,
        inner: &mut Inner,
        change: impl FnOnce(&mut Config) -> std::result::Result<T, CoreError>,
    ) -> Result<T> {
        let mut next = inner.config.clone();
        let out = change(&mut next).map_err(core_error)?;
        if next != inner.config {
            self.store
                .save(&next)
                .map_err(|e| Response::error(ErrorCode::Internal, format!("could not save settings: {e}")))?;
            let snapshot = Snapshot::of(&next);
            inner.config = next;
            *self.snapshot.write().unwrap_or_else(|e| e.into_inner()) = snapshot;
            self.generation.fetch_add(1, Ordering::AcqRel);
        }
        Ok(out)
    }

    /// Periodic housekeeping: drops expired locks and applies due unlocks.
    pub fn tick(&self) {
        let now = crate::clock::now();
        let loc = self.location();
        let ctx = self.lock_ctx(now, &loc);
        let mut inner = self.lock_inner();
        let due = inner.config.block_sets.iter().any(|s| {
            let mut probe = s.clone();
            lock::expire(&mut probe, &ctx)
        });
        if due {
            let result = self.commit(&mut inner, |c| {
                for s in &mut c.block_sets {
                    if lock::expire(s, &ctx) {
                        eprintln!("al-dir: lock on \"{}\" ended", s.name);
                    }
                }
                Ok(())
            });
            if let Err(Response::Error { message, .. }) = result {
                eprintln!("al-dir: {message}");
            }
        }
    }

    pub fn handle(&self, caller: &Caller, request: Request) -> Response {
        let mutating = !matches!(
            request,
            Request::Status | Request::ListSets | Request::Check { .. } | Request::RecentEvents
                | Request::Subscribe
                | Request::CheckPage { .. }
                | Request::ExtensionSession { .. }
        );
        if mutating && !caller.admin {
            return Response::error(
                ErrorCode::PermissionDenied,
                "only administrators can change Al-Dir settings",
            );
        }
        let wrong_answer = matches!(request, Request::UnlockAnswer { .. });
        let response = self.dispatch(caller, request).unwrap_or_else(|e| e);
        if wrong_answer && matches!(response, Response::Error { code: ErrorCode::WrongAnswer, .. }) {
            // Slow down password guessing.
            std::thread::sleep(Duration::from_secs(1));
        }
        response
    }

    fn dispatch(&self, caller: &Caller, request: Request) -> Result<Response> {
        let now = crate::clock::now();
        let loc = self.location();
        let ctx = self.lock_ctx(now, &loc);
        match request {
            Request::Status => {
                let inner = self.lock_inner();
                let sets = &inner.config.block_sets;
                Ok(Response::Status {
                    status: Box::new(Status {
                        version: env!("CARGO_PKG_VERSION").into(),
                        protocol: PROTOCOL_VERSION,
                        user: caller.user.clone(),
                        admin: caller.admin,
                        sets: sets.len(),
                        locked_sets: sets.iter().filter(|s| lock::is_locked(s, &ctx)).count(),
                        hardened: inner.config.hardened,
                        clock: crate::clock::source().describe().into(),
                        location: describe_location(&loc),
                        started_at: self.started_at,
                        extensions: self.extensions().iter().map(|e| format!("{} ({})", e.browser, e.user)).collect(),
                    }),
                })
            }
            Request::ListSets => {
                let engine = self.engine();
                let user_ctx = Ctx::new(now, &caller.user);
                let active: Vec<SetId> = engine.active_sets(&user_ctx).collect();
                let inner = self.lock_inner();
                let sets = inner
                    .config
                    .block_sets
                    .iter()
                    .map(|s| SetInfo {
                        set: if caller.admin { s.clone() } else { redact(s) },
                        locked: lock::is_locked(s, &ctx),
                        active: active.contains(&s.id),
                    })
                    .collect();
                Ok(Response::Sets { sets })
            }
            Request::UpsertSet { set } => {
                let mut set = *set;
                if let Some(l) = &mut set.lock
                    && let LockKind::UntilRestart { boot_id } = &mut l.kind
                    && boot_id.is_empty()
                {
                    *boot_id = self.boot_id.clone();
                }
                let mut inner = self.lock_inner();
                inner.challenges.remove(&set.id);
                self.commit(&mut inner, |c| c.upsert_set(set, &ctx))?;
                Ok(Response::Done)
            }
            Request::LearnLocation { id } => {
                let area = {
                    let inner = self.lock_inner();
                    match &find(&inner.config, id)?.lock {
                        Some(Lock { kind: LockKind::Location { area }, .. }) => area.clone(),
                        _ => return Err(Response::error(ErrorCode::BadRequest, "this block has no location lock")),
                    }
                };
                let learner = self
                    .learner
                    .get()
                    .ok_or_else(|| Response::error(ErrorCode::Internal, "location is not available on this system"))?;
                let known = learner(&area).map_err(|e| Response::error(ErrorCode::Internal, e.to_string()))?;
                Ok(Response::Learned { networks: known })
            }
            Request::SetHardened { on } => {
                let mut inner = self.lock_inner();
                self.commit(&mut inner, |c| c.set_hardened(on, &ctx))?;
                Ok(Response::Done)
            }
            Request::RemoveSet { id } => {
                let mut inner = self.lock_inner();
                self.commit(&mut inner, |c| c.remove_set(id, &ctx).map(drop))?;
                inner.challenges.remove(&id);
                Ok(Response::Done)
            }
            Request::Export { ids } => {
                let inner = self.lock_inner();
                Ok(Response::Exported { json: import::export(&inner.config, ids.as_deref(), now.unix) })
            }
            Request::Import { json } => {
                let file = import::parse_export(&json).map_err(core_error)?;
                let mut inner = self.lock_inner();
                let report = self.commit(&mut inner, |c| Ok(import::apply_import(c, file, &ctx)))?;
                Ok(Response::Imported { added: report.added, replaced: report.replaced, rejected: report.rejected })
            }
            Request::UnlockStart { id } => {
                let mut inner = self.lock_inner();
                let set = find(&inner.config, id)?;
                let challenge = match &set.lock {
                    Some(l) if lock::is_locked(set, &ctx) => lock::challenge_for(l).map_err(core_error)?,
                    _ => return Ok(Response::Challenge { challenge: ChallengeInfo::NotLocked }),
                };
                let challenge = match challenge {
                    Challenge::NotPossible => ChallengeInfo::NotPossible,
                    Challenge::Password => ChallengeInfo::Password,
                    Challenge::RandomText(text) => {
                        inner.challenges.insert(id, text.clone());
                        ChallengeInfo::RandomText { text }
                    }
                };
                Ok(Response::Challenge { challenge })
            }
            Request::UnlockAnswer { id, answer } => {
                let mut inner = self.lock_inner();
                let issued = inner.challenges.get(&id).cloned();
                let outcome = self.commit(&mut inner, |c| {
                    let set = c.get_mut(id).ok_or(CoreError::NotFound)?;
                    lock::try_unlock(set, &answer, issued.as_deref(), &now)
                })?;
                inner.challenges.remove(&id);
                let outcome = match outcome {
                    UnlockOutcome::Unlocked => UnlockInfo::Unlocked,
                    UnlockOutcome::Pending { at } => UnlockInfo::Pending { at },
                };
                Ok(Response::Unlock { outcome })
            }
            Request::CancelUnlock { id } => {
                let mut inner = self.lock_inner();
                self.commit(&mut inner, |c| {
                    c.get_mut(id).ok_or(CoreError::NotFound)?.pending_unlock_at = None;
                    Ok(())
                })?;
                Ok(Response::Done)
            }
            // The transport turns the connection into a feed; see server.rs.
            Request::Subscribe => Ok(Response::Subscribed),
            Request::ExtensionSession { .. } => Ok(Response::Subscribed),
            Request::CheckPage { url } => Ok(Response::Page { page: self.check_page(caller, &url, now) }),
            Request::RecentEvents => {
                let events = self.events.lock().unwrap_or_else(|e| e.into_inner());
                let events = events.iter().filter(|e| caller.admin || e.user == caller.user).cloned().collect();
                Ok(Response::Events { events })
            }
            Request::Check { query, user } => {
                let user = match user {
                    Some(u) if caller.admin => u,
                    Some(_) => {
                        return Err(Response::error(ErrorCode::PermissionDenied, "only administrators can check other users"));
                    }
                    None => caller.user.clone(),
                };
                let engine = self.engine();
                let c = Ctx::new(now, &user);
                let decision = match &query {
                    CheckQuery::Url { url } => engine.check_url(url, &c),
                    CheckQuery::Host { host } => engine.check_host(host, &c),
                    CheckQuery::App { exe_path, cmdline } => {
                        let p = ProcessInfo { exe_path, cmdline: cmdline.as_deref(), sha256: None };
                        engine.check_process(&p, &c)
                    }
                    CheckQuery::Window { title, exe_path } => {
                        let p = ProcessInfo { exe_path, ..Default::default() };
                        engine.check_window(title, &p, &c)
                    }
                };
                Ok(Response::Decision { decision: self.describe(decision) })
            }
        }
    }

    fn describe(&self, d: Decision) -> DecisionInfo {
        let snapshot = self.snapshot();
        let name = |id: SetId| snapshot.names.get(&id).cloned();
        DecisionInfo {
            blocked: d.is_blocked(),
            set: d.block.as_ref().map(|h| h.set),
            set_name: d.block.as_ref().and_then(|h| name(h.set)),
            reason: d.block.as_ref().map(|h| h.reason.to_string()),
            metered: d.metered,
        }
    }
}

fn find(config: &Config, id: SetId) -> Result<&BlockSet> {
    config.get(id).ok_or_else(|| Response::error(ErrorCode::NotFound, "no block with that id"))
}

/// Non-administrators must not get password hashes to attack offline.
fn redact(set: &BlockSet) -> BlockSet {
    let mut s = set.clone();
    if let Some(l) = &mut s.lock
        && let LockKind::Password { hash } = &mut l.kind
    {
        *hash = PasswordHash { salt: String::new(), hash: String::new(), m_kib: 0, t: 0, p: 0 };
    }
    s
}

fn core_error(e: CoreError) -> Response {
    let code = match &e {
        CoreError::Locked(_) => ErrorCode::Locked,
        CoreError::NotFound => ErrorCode::NotFound,
        CoreError::WrongAnswer => ErrorCode::WrongAnswer,
        CoreError::CannotUnlockEarly => ErrorCode::CannotUnlockEarly,
        CoreError::Hash | CoreError::Random => ErrorCode::Internal,
        _ => ErrorCode::BadRequest,
    };
    Response::error(code, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_dir_core::{Lock, WebRule};

    fn service(name: &str) -> (Service, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("al-dir-svc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (Service::new(Store::open(&dir).unwrap(), "boot".into()).unwrap(), dir)
    }

    fn admin() -> Caller {
        Caller { user: "alice".into(), admin: true }
    }

    fn check_url(svc: &Service, url: &str) -> bool {
        let r = svc.handle(&admin(), Request::Check { query: CheckQuery::Url { url: url.into() }, user: None });
        let Response::Decision { decision } = r else { panic!("{r:?}") };
        decision.blocked
    }

    #[test]
    fn upsert_check_persist() {
        let (svc, dir) = service("persist");
        let mut set = BlockSet::new("social");
        set.web.block = vec![WebRule::parse("facebook.com").unwrap()];
        assert!(matches!(svc.handle(&admin(), Request::UpsertSet { set: Box::new(set.clone()) }), Response::Done));
        assert!(check_url(&svc, "https://www.facebook.com"));

        // A new service on the same directory sees the saved block.
        let svc2 = Service::new(Store::open(&dir).unwrap(), "boot".into()).unwrap();
        assert!(check_url(&svc2, "https://www.facebook.com"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn non_admins_cannot_change_anything() {
        let (svc, dir) = service("perm");
        let user = Caller { user: "bob".into(), admin: false };
        let r = svc.handle(&user, Request::UpsertSet { set: Box::new(BlockSet::new("x")) });
        assert!(matches!(r, Response::Error { code: ErrorCode::PermissionDenied, .. }));
        assert!(matches!(svc.handle(&user, Request::Status), Response::Status { .. }));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn locked_sets_resist_and_unlock_with_random_text() {
        let (svc, dir) = service("lock");
        let mut set = BlockSet::new("locked");
        set.web.block = vec![WebRule::parse("a.com").unwrap()];
        set.lock = Some(Lock { kind: LockKind::RandomText { length: 20 }, unlock_delay_secs: 0 });
        let id = set.id;
        svc.handle(&admin(), Request::UpsertSet { set: Box::new(set.clone()) });

        let mut weaker = set.clone();
        weaker.web.block.clear();
        let r = svc.handle(&admin(), Request::UpsertSet { set: Box::new(weaker.clone()) });
        assert!(matches!(r, Response::Error { code: ErrorCode::Locked, .. }), "{r:?}");
        assert!(matches!(svc.handle(&admin(), Request::RemoveSet { id }), Response::Error { code: ErrorCode::Locked, .. }));

        let Response::Challenge { challenge: ChallengeInfo::RandomText { text } } =
            svc.handle(&admin(), Request::UnlockStart { id })
        else {
            panic!()
        };
        let r = svc.handle(&admin(), Request::UnlockAnswer { id, answer: text });
        assert!(matches!(r, Response::Unlock { outcome: UnlockInfo::Unlocked }), "{r:?}");
        assert!(matches!(svc.handle(&admin(), Request::UpsertSet { set: Box::new(weaker) }), Response::Done));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn restart_lock_gets_boot_id() {
        let (svc, dir) = service("restart");
        let mut set = BlockSet::new("r");
        set.lock = Some(Lock { kind: LockKind::UntilRestart { boot_id: String::new() }, unlock_delay_secs: 0 });
        svc.handle(&admin(), Request::UpsertSet { set: Box::new(set) });
        let Response::Sets { sets } = svc.handle(&admin(), Request::ListSets) else { panic!() };
        assert!(sets[0].locked);
        assert_eq!(sets[0].set.lock.as_ref().unwrap().kind, LockKind::UntilRestart { boot_id: "boot".into() });
        std::fs::remove_dir_all(dir).unwrap();
    }
}
