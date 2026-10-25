//! Application enforcement shared by the exec hook (fanotify), the process
//! watcher (proc connector) and the periodic sweep of running processes.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::os::unix::fs::MetadataExt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use iqal_core::{Ctx, Engine, Hit, Meter, Now, ProcessInfo, Reason};
use iqal_ipc::EventKind;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use sha2::{Digest, Sha256};

use super::appindex::{self, AppIndex};
use super::browsers;
use super::procs::{self, Users};
use crate::service::Service;

const INDEX_MAX_AGE: Duration = Duration::from_secs(60);
const HASH_CACHE_MAX: usize = 4096;

/// (device, inode, size, mtime seconds, mtime nanoseconds)
type FileKey = (u64, u64, u64, i64, i64);

pub struct AppGuard {
    service: Arc<Service>,
    users: Users,
    index: RwLock<(Arc<AppIndex>, Instant)>,
    hashes: Mutex<HashMap<FileKey, String>>,
    own_pid: i32,
    last_sweep: Mutex<Instant>,
    /// Fractions of a second not yet counted.
    carry: Mutex<f64>,
    /// Browsers seen without the extension, and since when (by this
    /// service: after a restart, extensions get time to reconnect).
    uncovered_since: Mutex<HashMap<i32, Instant>>,
}

struct Verdict {
    hit: Option<Hit>,
    metered: Vec<Meter>,
    user: String,
}

impl AppGuard {
    pub fn new(service: Arc<Service>) -> Self {
        let index = AppIndex::build();
        eprintln!("iqal: app enforcement ready ({} menu apps indexed)", index.len());
        Self {
            service,
            users: Users::load(),
            index: RwLock::new((Arc::new(index), Instant::now())),
            hashes: Mutex::new(HashMap::new()),
            own_pid: std::process::id() as i32,
            last_sweep: Mutex::new(Instant::now()),
            carry: Mutex::new(0.0),
            uncovered_since: Mutex::new(HashMap::new()),
        }
    }

    fn index(&self) -> Arc<AppIndex> {
        self.index.read().unwrap_or_else(|e| e.into_inner()).0.clone()
    }

    /// Rebuilds the app index if it is old (new apps get installed).
    pub fn refresh_index_if_stale(&self) {
        let stale = self.index.read().unwrap_or_else(|e| e.into_inner()).1.elapsed() > INDEX_MAX_AGE;
        if stale {
            let fresh = Arc::new(AppIndex::build());
            *self.index.write().unwrap_or_else(|e| e.into_inner()) = (fresh, Instant::now());
        }
    }

    /// The common decision for an enforced account: the block (if any),
    /// the allowances the program uses up, and the user.
    fn decide(
        &self,
        engine: &Engine,
        now: Now,
        uid: u32,
        exe: &str,
        cmdline: Option<&str>,
        hash: impl FnOnce() -> Option<String>,
    ) -> Option<Verdict> {
        if !self.users.enforced(uid) || appindex::is_essential(exe) {
            return None;
        }
        let user = self.users.name(uid);
        let usage = self.service.usage_for(&user);
        let ctx = Ctx { now, user: &user, usage: usage.as_deref() };
        if browsers::is_unsupported(exe)
            && let Some(set) = engine.web_active_set(&ctx)
        {
            return Some(Verdict { hit: Some(Hit { set, reason: Reason::UnsupportedBrowser }), metered: Vec::new(), user });
        }
        let sha = if engine.wants_exe_hash() { hash() } else { None };
        let process = ProcessInfo { exe_path: exe, cmdline, sha256: sha.as_deref() };
        let decision = engine.check_process(&process, &ctx);
        let hit = decision.block.filter(|hit| {
            // Allow-only mode restricts apps, not system helpers.
            hit.reason != Reason::NotAllowed || self.index().is_user_app(exe)
        });
        Some(Verdict { hit, metered: decision.metered, user })
    }

    /// Exec hook: may process `pid` execute the file open as `file`?
    /// Must never fail or hang: any problem means "allow".
    pub fn exec_allowed(&self, pid: i32, file: BorrowedFd<'_>, engine: &Engine, now: Now) -> bool {
        if pid == self.own_pid {
            return true;
        }
        let Some(uid) = procs::uid_of(pid) else { return true };
        if !self.users.enforced(uid) {
            return true;
        }
        let Some(exe) = procs::fd_path(file.as_raw_fd()) else { return true };
        let hash = || {
            let owned = file.try_clone_to_owned().ok()?;
            self.hash_file(File::from(owned))
        };
        match self.decide(engine, now, uid, &exe, None, hash) {
            Some(Verdict { hit: Some(hit), user, .. }) => {
                self.service.record_block(&user, EventKind::AppStart, &exe, &hit);
                false
            }
            _ => true,
        }
    }

    /// Checks an already running process; kills it if it is blocked.
    /// Otherwise returns the user and the allowances it is using up.
    pub fn enforce_running(&self, pid: i32, engine: &Engine, now: Now) -> Option<(String, Vec<Meter>)> {
        if pid == self.own_pid || pid <= 1 {
            return None;
        }
        let uid = procs::uid_of(pid)?;
        if !self.users.enforced(uid) {
            return None;
        }
        let exe = procs::exe_of(pid)?;
        let cmdline = procs::cmdline_of(pid);
        let hash = || File::open(format!("/proc/{pid}/exe")).ok().and_then(|f| self.hash_file(f));
        let verdict = self.decide(engine, now, uid, &exe, cmdline.as_deref(), hash)?;
        match verdict.hit {
            Some(hit) => {
                // The process may have exited meanwhile; that is fine.
                if kill(Pid::from_raw(pid), Signal::SIGKILL).is_ok() {
                    self.service.record_block(&verdict.user, EventKind::AppKilled, &exe, &hit);
                }
                None
            }
            None if verdict.metered.is_empty() => None,
            None => Some((verdict.user, verdict.metered)),
        }
    }

    /// Closes every running program that is blocked right now (e.g. when a
    /// scheduled block starts or a rule is added).
    /// Also counts time for apps that use up an allowance.
    pub fn sweep(&self) {
        let engine = self.service.engine();
        let now = crate::clock::now();
        let pids = procs::pids();
        let mut metered: HashMap<String, Vec<Meter>> = HashMap::new();
        for &pid in &pids {
            if let Some((user, sets)) = self.enforce_running(pid, &engine, now) {
                metered.entry(user).or_default().extend(sets);
            }
        }
        let elapsed = {
            let mut last = self.last_sweep.lock().unwrap_or_else(|e| e.into_inner());
            let secs = last.elapsed().as_secs_f64().min(10.0);
            *last = Instant::now();
            secs
        };
        let mut carry = self.carry.lock().unwrap_or_else(|e| e.into_inner());
        *carry += elapsed;
        let whole = carry.floor() as u32;
        *carry -= whole as f64;
        drop(carry);
        for (user, sets) in metered {
            self.service.add_usage(&user, &sets, whole);
        }
        self.close_unsupervised_browsers(&pids, &engine, now);
    }

    /// Closes supported browsers that have been without the extension for a
    /// while (as seen by this service) while website rules are in force for
    /// their user. Counting from when the service first saw the browser
    /// uncovered, not from the browser's start, gives extensions time to
    /// reconnect after the service restarts (an update, say).
    fn close_unsupervised_browsers(&self, pids: &[i32], engine: &Engine, now: Now) {
        let sessions = self.service.extensions();
        let mut covered = None;
        let mut uncovered_since = self.uncovered_since.lock().unwrap_or_else(|e| e.into_inner());
        let mut still_uncovered = Vec::new();
        for &pid in pids {
            let Some(exe) = procs::exe_of(pid) else { continue };
            let Some(family) = browsers::family_of(&exe) else { continue };
            let Some(uid) = procs::uid_of(pid) else { continue };
            if !self.users.enforced(uid)
                || !browsers::is_main_process(pid, family)
                || browsers::age_secs(pid).is_none_or(|a| a < browsers::EXTENSION_GRACE_SECS)
                || !self.service.extension_proven(family)
            {
                continue;
            }
            let user = self.users.name(uid);
            let Some(set) = engine.web_active_set(&Ctx::new(now, &user)) else { continue };
            let covered = covered.get_or_insert_with(|| browsers::covered(sessions.iter().map(|s| s.host_pid)));
            if covered.contains(&pid) {
                continue;
            }
            let since = *uncovered_since.entry(pid).or_insert_with(Instant::now);
            if since.elapsed().as_secs() < browsers::EXTENSION_GRACE_SECS {
                still_uncovered.push(pid);
                continue;
            }
            // Not counted further: a browser that is closing is not closed again.
            if kill(Pid::from_raw(pid), Signal::SIGKILL).is_ok() {
                let hit = Hit { set, reason: Reason::ExtensionMissing };
                self.service.record_block(&user, EventKind::AppKilled, &exe, &hit);
            }
        }
        // A browser that connected (or closed, or no longer needs it) starts
        // over the next time.
        uncovered_since.retain(|pid, _| still_uncovered.contains(pid));
    }

    /// SHA-256 of a file, cached by identity and modification time so big
    /// programs are hashed once.
    fn hash_file(&self, mut file: File) -> Option<String> {
        let m = file.metadata().ok()?;
        let key = (m.dev(), m.ino(), m.size(), m.mtime(), m.mtime_nsec());
        if let Some(h) = self.hashes.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return Some(h.clone());
        }
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = file.read(&mut buf).ok()?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let hex: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        let mut cache = self.hashes.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= HASH_CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, hex.clone());
        Some(hex)
    }
}
