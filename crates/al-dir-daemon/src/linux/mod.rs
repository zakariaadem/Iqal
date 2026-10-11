pub mod appindex;
pub mod browsers;
pub mod dns;
pub mod exec_hook;
pub mod extension;
pub mod firewall;
pub mod guard;
pub mod policies;
pub mod proc_watch;
pub mod procs;
pub mod server;
pub mod system;
pub mod windows;

use std::sync::Arc;
use std::time::Duration;

use nix::sys::signal::{SigSet, Signal};

use crate::service::Service;

/// How often running programs are re-checked (blocks starting on a
/// schedule, rules just added).
const SWEEP_EVERY: Duration = Duration::from_secs(2);
/// How often firewall rules, DNS listeners and browser policies are
/// re-asserted (new user accounts, rules removed by hand).
const NETWORK_CHECK_EVERY: Duration = Duration::from_secs(30);

/// Handles SIGTERM/SIGINT on a dedicated thread: removes the firewall
/// redirect (so DNS keeps working while we are stopped) and exits. Must be
/// called before any other thread starts, so all threads inherit the mask.
pub fn install_shutdown_handler() -> std::io::Result<()> {
    let mut set = SigSet::empty();
    set.add(Signal::SIGTERM);
    set.add(Signal::SIGINT);
    set.thread_block().map_err(std::io::Error::from)?;
    std::thread::Builder::new().name("al-dir-signals".into()).spawn(move || {
        if let Ok(sig) = set.wait() {
            eprintln!("al-dir: {sig:?} received, stopping");
            if nix::unistd::geteuid().is_root() {
                firewall::remove();
            }
            std::process::exit(0);
        }
    })?;
    Ok(())
}

/// Removes everything the service put into the system (uninstall).
pub fn cleanup() {
    firewall::remove();
    policies::remove();
}

/// Starts all enforcement. Needs root.
pub fn start_enforcement(service: Arc<Service>) -> std::io::Result<()> {
    let guard = Arc::new(guard::AppGuard::new(service.clone()));

    if let Err(e) = exec_hook::start(service.clone(), guard.clone()) {
        eprintln!("al-dir: exec hook unavailable ({e}); falling back to closing blocked programs");
    }
    if let Err(e) = proc_watch::start(service.clone(), guard.clone()) {
        eprintln!("al-dir: process watcher unavailable ({e}); relying on periodic checks");
    }
    if let Err(e) = windows::start(service.clone()) {
        eprintln!("al-dir: window watcher unavailable ({e})");
    }
    start_network(service.clone())?;

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

fn start_network(service: Arc<Service>) -> std::io::Result<()> {
    let filter = dns::DnsFilter::new(service.clone());
    policies::apply(service.engine().has_any_web_rules());
    if let Err(e) = extension::serve_updates() {
        eprintln!("al-dir: extension update server: {e}");
    }
    let mut applied = filter.ensure_listeners();
    match firewall::apply(&applied) {
        Ok(()) => eprintln!("al-dir: DNS filtering active for {} account(s)", applied.len()),
        Err(e) => eprintln!("al-dir: firewall rules failed: {e}"),
    }
    std::thread::Builder::new().name("al-dir-network".into()).spawn(move || {
        let mut generation = service.generation();
        let mut last = std::time::Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let g = service.generation();
            if g != generation {
                // Settings changed: private-window rules may need updating.
                generation = g;
                policies::apply(service.engine().has_any_web_rules());
            }
            if last.elapsed() < NETWORK_CHECK_EVERY {
                continue;
            }
            last = std::time::Instant::now();
            let users = filter.ensure_listeners();
            if users != applied || !firewall::present() {
                match firewall::apply(&users) {
                    Ok(()) => eprintln!("al-dir: firewall rules restored"),
                    Err(e) => eprintln!("al-dir: firewall rules failed: {e}"),
                }
                applied = users;
            }
            policies::apply(service.engine().has_any_web_rules());
        }
    })?;
    Ok(())
}
