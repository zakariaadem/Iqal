pub mod appindex;
pub mod guard;
pub mod proc_watch;
pub mod procs;
pub mod scm;
pub mod server;
pub mod sessions;
pub mod system;
pub mod titles;
pub mod watcher;

use std::fs::OpenOptions;
use std::os::windows::io::IntoRawHandle;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use windows_sys::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};

use crate::service::Service;

/// How often running programs are re-checked (blocks starting on a
/// schedule, rules just added).
const SWEEP_EVERY: Duration = Duration::from_secs(2);

/// Starts all enforcement. Needs to run as LocalSystem (or an
/// administrator, for testing).
pub fn start_enforcement(service: Arc<Service>) -> std::io::Result<()> {
    let guard = Arc::new(guard::AppGuard::new(service.clone()));
    if let Err(e) = proc_watch::start(service.clone(), guard.clone()) {
        eprintln!("iqal: process watch unavailable ({e}); relying on periodic checks");
    }
    if let Err(e) = sessions::start() {
        eprintln!("iqal: window watchers unavailable ({e})");
    }
    std::thread::Builder::new().name("iqal-sweep".into()).spawn(move || {
        let mut seen = u64::MAX;
        let mut last = std::time::Instant::now();
        loop {
            // Sweep right after a settings change, otherwise periodically.
            let generation = service.generation();
            if generation != seen || last.elapsed() >= SWEEP_EVERY {
                seen = generation;
                last = std::time::Instant::now();
                guard.refresh_index_if_stale();
                guard.sweep();
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    })?;
    Ok(())
}

const LOG: &str = "iqal.log";
/// The log is started afresh (keeping one old copy) beyond this size.
const LOG_MAX: u64 = 1 << 20;

/// A service has no terminal: sends messages to `iqal.log` in the settings
/// folder instead.
pub fn log_to_file(state_dir: &Path) -> std::io::Result<()> {
    let path = state_dir.join(LOG);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > LOG_MAX) {
        let _ = std::fs::rename(&path, state_dir.join(format!("{LOG}.old")));
    }
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    // SAFETY: hands Windows a valid handle, which stays open for good.
    unsafe { SetStdHandle(STD_ERROR_HANDLE, file.into_raw_handle()) };
    Ok(())
}
