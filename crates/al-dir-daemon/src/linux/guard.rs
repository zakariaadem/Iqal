//! Application enforcement shared by the exec hook (fanotify), the process
//! watcher (proc connector) and the periodic sweep of running processes.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::os::unix::fs::MetadataExt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use al_dir_core::{Ctx, Engine, Hit, Now, ProcessInfo, Reason};
use al_dir_ipc::EventKind;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use sha2::{Digest, Sha256};

use super::appindex::{self, AppIndex};
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
}

impl AppGuard {
    pub fn new(service: Arc<Service>) -> Self {
        let index = AppIndex::build();
        eprintln!("al-dir: app enforcement ready ({} menu apps indexed)", index.len());
        Self {
            service,
            users: Users::load(),
            index: RwLock::new((Arc::new(index), Instant::now())),
            hashes: Mutex::new(HashMap::new()),
            own_pid: std::process::id() as i32,
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

    /// The common decision. Returns the hit and the user it applies to.
    fn decide(
        &self,
        engine: &Engine,
        now: Now,
        uid: u32,
        exe: &str,
        cmdline: Option<&str>,
        hash: impl FnOnce() -> Option<String>,
    ) -> Option<(Hit, String)> {
        if !self.users.enforced(uid) || appindex::is_essential(exe) {
            return None;
        }
        let user = self.users.name(uid);
        let sha = if engine.wants_exe_hash() { hash() } else { None };
        let process = ProcessInfo { exe_path: exe, cmdline, sha256: sha.as_deref() };
        let hit = engine.check_process(&process, &Ctx::new(now, &user)).block?;
        if hit.reason == Reason::NotAllowed && !self.index().is_user_app(exe) {
            return None; // allow-only mode only restricts apps, not system helpers
        }
        Some((hit, user))
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
            Some((hit, user)) => {
                self.service.record_block(&user, EventKind::AppStart, &exe, &hit);
                false
            }
            None => true,
        }
    }

    /// Checks an already running process; kills it if it is blocked.
    pub fn enforce_running(&self, pid: i32, engine: &Engine, now: Now) {
        if pid == self.own_pid || pid <= 1 {
            return;
        }
        let Some(uid) = procs::uid_of(pid) else { return };
        if !self.users.enforced(uid) {
            return;
        }
        let Some(exe) = procs::exe_of(pid) else { return };
        let cmdline = procs::cmdline_of(pid);
        let hash = || File::open(format!("/proc/{pid}/exe")).ok().and_then(|f| self.hash_file(f));
        if let Some((hit, user)) = self.decide(engine, now, uid, &exe, cmdline.as_deref(), hash) {
            // The process may have exited meanwhile; that is fine.
            if kill(Pid::from_raw(pid), Signal::SIGKILL).is_ok() {
                self.service.record_block(&user, EventKind::AppKilled, &exe, &hit);
            }
        }
    }

    /// Closes every running program that is blocked right now (e.g. when a
    /// scheduled block starts or a rule is added).
    pub fn sweep(&self) {
        let engine = self.service.engine();
        let now = Now::system();
        for pid in procs::pids() {
            self.enforce_running(pid, &engine, now);
        }
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
