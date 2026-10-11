//! The service state and request handling, independent of the transport.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::Duration;

use al_dir_core::lock::{self, Challenge, LockContext, UnlockOutcome};
use al_dir_core::{
    import, BlockSet, Config, CoreError, Ctx, Decision, Engine, LocationFix, LockKind, NoLists, Now,
    PasswordHash, ProcessInfo, SetId,
};
use al_dir_ipc::{
    ChallengeInfo, CheckQuery, DecisionInfo, ErrorCode, Request, Response, SetInfo, Status, UnlockInfo,
    PROTOCOL_VERSION,
};

use crate::storage::Store;

/// Who sent a request, as established by the transport.
#[derive(Debug, Clone)]
pub struct Caller {
    pub user: String,
    pub admin: bool,
}

pub struct Service {
    inner: Mutex<Inner>,
    engine: RwLock<Arc<Engine>>,
    store: Store,
    boot_id: String,
    started_at: i64,
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
        let engine = Arc::new(Engine::new(&config, &NoLists));
        Ok(Self {
            inner: Mutex::new(Inner { config, challenges: HashMap::new() }),
            engine: RwLock::new(engine),
            store,
            boot_id,
            started_at: Now::system().unix,
        })
    }

    /// The current compiled engine; cheap to call on hot paths.
    pub fn engine(&self) -> Arc<Engine> {
        self.engine.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn lock_ctx(&self, now: Now) -> LockContext<'_> {
        // Location fixes arrive with milestone M6; until then location
        // locks fail closed (count as inside).
        LockContext { now, boot_id: &self.boot_id, location: LocationFix::Unknown }
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
            let engine = Arc::new(Engine::new(&next, &NoLists));
            inner.config = next;
            *self.engine.write().unwrap_or_else(|e| e.into_inner()) = engine;
        }
        Ok(out)
    }

    /// Periodic housekeeping: drops expired locks and applies due unlocks.
    pub fn tick(&self) {
        let now = Now::system();
        let ctx = self.lock_ctx(now);
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
        let mutating = !matches!(request, Request::Status | Request::ListSets | Request::Check { .. });
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
        let now = Now::system();
        let ctx = self.lock_ctx(now);
        match request {
            Request::Status => {
                let inner = self.lock_inner();
                let sets = &inner.config.block_sets;
                Ok(Response::Status {
                    status: Status {
                        version: env!("CARGO_PKG_VERSION").into(),
                        protocol: PROTOCOL_VERSION,
                        user: caller.user.clone(),
                        admin: caller.admin,
                        sets: sets.len(),
                        locked_sets: sets.iter().filter(|s| lock::is_locked(s, &ctx)).count(),
                        started_at: self.started_at,
                    },
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
        let inner = self.lock_inner();
        let name = |id: SetId| inner.config.get(id).map(|s| s.name.clone());
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
