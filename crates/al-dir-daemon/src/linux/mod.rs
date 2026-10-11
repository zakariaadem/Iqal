pub mod appindex;
pub mod exec_hook;
pub mod guard;
pub mod proc_watch;
pub mod procs;
pub mod server;
pub mod system;

use std::sync::Arc;
use std::time::Duration;

use crate::service::Service;

/// How often running programs are re-checked (blocks starting on a
/// schedule, rules just added).
const SWEEP_EVERY: Duration = Duration::from_secs(2);

/// Starts all enforcement. Needs root.
pub fn start_enforcement(service: Arc<Service>) -> std::io::Result<()> {
    let guard = Arc::new(guard::AppGuard::new(service.clone()));

    if let Err(e) = exec_hook::start(service.clone(), guard.clone()) {
        eprintln!("al-dir: exec hook unavailable ({e}); falling back to closing blocked programs");
    }
    if let Err(e) = proc_watch::start(service.clone(), guard.clone()) {
        eprintln!("al-dir: process watcher unavailable ({e}); relying on periodic checks");
    }

    std::thread::Builder::new().name("al-dir-sweep".into()).spawn(move || {
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
