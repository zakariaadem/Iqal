//! Application enforcement shared by the process watcher (ETW) and the
//! periodic sweep of running processes, including browser supervision.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::windows::fs::MetadataExt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use iqal_core::{Ctx, Engine, Hit, Now, ProcessInfo, Reason, SetId};
use iqal_ipc::EventKind;
use sha2::{Digest, Sha256};

use super::appindex::{self, AppIndex};
use super::browsers;
use super::procs::{self, Users};
use crate::service::Service;

const INDEX_MAX_AGE: Duration = Duration::from_secs(60);
const HASH_CACHE_MAX: usize = 4096;

/// (lower-case path, size, last write time)
type FileKey = (String, u64, u64);

pub struct AppGuard {
    service: Arc<Service>,
    users: Users,
    index: RwLock<(Arc<AppIndex>, Instant)>,
    hashes: Mutex<HashMap<FileKey, String>>,
    own_pid: u32,
    last_sweep: Mutex<Instant>,
    /// Fractions of a second not yet counted.
    carry: Mutex<f64>,
    /// Browsers seen without the extension, and since when (by this
    /// service: after a restart, extensions get time to reconnect).
    uncovered_since: Mutex<HashMap<u32, Instant>>,
}

struct Verdict {
    hit: Option<Hit>,
    metered: Vec<SetId>,
    user: String,
}

impl AppGuard {
    pub fn new(service: Arc<Service>) -> Self {
        let index = AppIndex::build();
        eprintln!("iqal: app enforcement ready ({} shortcut apps indexed)", index.len());
        Self {
            service,
            users: Users::load(),
            index: RwLock::new((Arc::new(index), Instant::now())),
            hashes: Mutex::new(HashMap::new()),
            own_pid: std::process::id(),
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

    /// The common decision for a process of an enforced account: the block
    /// (if any) and the allowances the program uses up.
    fn decide(&self, engine: &Engine, now: Now, user: String, exe: &str, cmdline: Option<&str>) -> Verdict {
        let usage = self.service.usage_for(&user);
        let ctx = Ctx { now, user: &user, usage: usage.as_deref() };
        if browsers::is_unsupported(exe)
            && let Some(set) = engine.web_active_set(&ctx)
        {
            let hit = Some(Hit { set, reason: Reason::UnsupportedBrowser });
            return Verdict { hit, metered: Vec::new(), user };
        }
        let sha = if engine.wants_exe_hash() { self.hash_file(exe) } else { None };
        let process = ProcessInfo { exe_path: exe, cmdline, sha256: sha.as_deref() };
        let decision = engine.check_process(&process, &ctx);
        let hit = decision.block.filter(|hit| {
            // Allow-only mode restricts apps, not system helpers.
            hit.reason != Reason::NotAllowed || self.index().is_user_app(exe)
        });
        Verdict { hit, metered: decision.metered, user }
    }

    /// Checks a running process; closes it if it is blocked. Otherwise
    /// returns the user and the allowances it is using up.
    pub fn enforce_running(&self, pid: u32, engine: &Engine, now: Now) -> Option<(String, Vec<SetId>)> {
        // 0 and 4 are Windows' idle and system processes.
        if pid == self.own_pid || pid <= 4 {
            return None;
        }
        let user = self.users.of_process(pid)?;
        let exe = procs::exe_of(pid)?;
        if appindex::is_essential(&exe) {
            return None;
        }
        let cmdline = procs::cmdline_of(pid);
        let verdict = self.decide(engine, now, user, &exe, cmdline.as_deref());
        match verdict.hit {
            Some(hit) => {
                // The process may have exited meanwhile; that is fine.
                if procs::kill(pid) {
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
        let mut metered: HashMap<String, Vec<SetId>> = HashMap::new();
        for entry in procs::list() {
            if let Some((user, sets)) = self.enforce_running(entry.pid, &engine, now) {
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
        self.close_unsupervised_browsers(&engine, now);
    }

    /// Closes supported browsers that have been without the extension for a
    /// while (as seen by this service) while website rules are in force for
    /// their user. Counting from when the service first saw the browser
    /// uncovered, not from the browser's start, gives extensions time to
    /// reconnect after the service restarts (an update, say).
    fn close_unsupervised_browsers(&self, engine: &Engine, now: Now) {
        let sessions = self.service.extensions();
        let parents = browsers::parents();
        let mut covered = None;
        let mut uncovered_since = self.uncovered_since.lock().unwrap_or_else(|e| e.into_inner());
        let mut still_uncovered = Vec::new();
        for &pid in parents.keys() {
            let Some(exe) = procs::exe_of(pid) else { continue };
            let Some(family) = browsers::family_of(&exe) else { continue };
            if !browsers::is_main_process(pid, family, &parents)
                || browsers::age_secs(pid).is_none_or(|a| a < browsers::EXTENSION_GRACE_SECS)
                || !self.service.extension_proven(family)
            {
                continue;
            }
            let Some(user) = self.users.of_process(pid) else { continue };
            let Some(set) = engine.web_active_set(&Ctx::new(now, &user)) else { continue };
            let covered = covered
                .get_or_insert_with(|| browsers::covered(sessions.iter().map(|s| s.host_pid as u32), &parents));
            if covered.contains(&pid) {
                continue;
            }
            let since = *uncovered_since.entry(pid).or_insert_with(Instant::now);
            if since.elapsed().as_secs() < browsers::EXTENSION_GRACE_SECS {
                still_uncovered.push(pid);
                continue;
            }
            // Not counted further: a browser that is closing is not closed again.
            if procs::kill(pid) {
                let hit = Hit { set, reason: Reason::ExtensionMissing };
                self.service.record_block(&user, EventKind::AppKilled, &exe, &hit);
            }
        }
        // A browser that connected (or closed, or no longer needs it) starts
        // over the next time.
        uncovered_since.retain(|pid, _| still_uncovered.contains(pid));
    }

    /// SHA-256 of a program file, cached by path, size and modification
    /// time so big programs are hashed once.
    fn hash_file(&self, path: &str) -> Option<String> {
        let mut file = File::open(path).ok()?;
        let m = file.metadata().ok()?;
        let key = (path.to_lowercase(), m.file_size(), m.last_write_time());
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
