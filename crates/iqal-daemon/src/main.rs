//! iqal-daemon: the privileged Iqal service.
//!
//! Usage: `iqal-daemon [--socket PATH] [--state-dir DIR]`
//!
//! Normally started by systemd as root. For development it can run as an
//! ordinary user with its own socket and state directory.

mod clock;
mod dnsmsg;
mod lists;
mod service;
mod stats;
mod storage;
mod usage;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
fn main() {
    if let Err(e) = linux_main() {
        eprintln!("iqal: {e}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
fn linux_main() -> Result<(), String> {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use linux::{server, system};

    linux::install_shutdown_handler().map_err(|e| format!("signal setup: {e}"))?;

    let mut socket = PathBuf::from(iqal_ipc::DEFAULT_SOCKET);
    let mut state_dir = PathBuf::from("/var/lib/iqal");
    let mut enforce = true;
    let mut cleanup = false;
    let mut network = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--socket" => socket = value("--socket")?.into(),
            "--state-dir" => state_dir = value("--state-dir")?.into(),
            "--no-enforce" => enforce = false,
            "--cleanup" => cleanup = true,
            "--no-network" => network = false,
            "--version" => {
                println!("iqal-daemon {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-h" | "--help" => {
                println!("usage: iqal-daemon [--socket PATH] [--state-dir DIR] [--no-enforce] [--no-network] | --cleanup");
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }

    // Iqal was called Al-Dir before: keep its blocks, locks and statistics.
    // (systemd creates the new folder empty before starting us.)
    let legacy = Path::new("/var/lib/al-dir");
    if state_dir == Path::new("/var/lib/iqal")
        && legacy.is_dir()
        && (!state_dir.exists() || std::fs::remove_dir(&state_dir).is_ok())
    {
        match std::fs::rename(legacy, &state_dir) {
            Ok(()) => eprintln!("iqal: moved settings from {}", legacy.display()),
            Err(e) => eprintln!("iqal: could not move settings from {}: {e}", legacy.display()),
        }
    }
    let store = storage::Store::open(&state_dir).map_err(|e| format!("state directory {}: {e}", state_dir.display()))?;
    clock::init(state_dir.clone());
    let service = Arc::new(
        service::Service::new(store, system::boot_id()).map_err(|e| format!("loading settings: {e}"))?,
    );
    if cleanup {
        // Uninstalling: refuse while any block is locked.
        if service.lock_summary().any_locked {
            return Err("a block is locked; Iqal cannot be removed until its locks end".into());
        }
        linux::cleanup();
        return Ok(());
    }
    clock::start_sync().map_err(|e| e.to_string())?;
    let listener = server::bind(&socket).map_err(|e| format!("socket {}: {e}", socket.display()))?;

    // Housekeeping and watchdog.
    {
        let service = service.clone();
        let watchdog = system::watchdog_interval();
        std::thread::Builder::new()
            .name("iqal-tick".into())
            .spawn(move || {
                let period = watchdog.map_or(Duration::from_secs(10), |w| w.min(Duration::from_secs(10)));
                loop {
                    service.tick();
                    if watchdog.is_some() {
                        system::sd_notify("WATCHDOG=1");
                    }
                    std::thread::sleep(period);
                }
            })
            .map_err(|e| e.to_string())?;
    }

    // Pre-made lists: downloads are due at most daily; checked every 10 min.
    {
        let service = service.clone();
        std::thread::Builder::new()
            .name("iqal-lists".into())
            .spawn(move || loop {
                service.update_lists();
                // Wake early when settings change (a list was just added).
                let generation = service.generation();
                for _ in 0..120 {
                    std::thread::sleep(Duration::from_secs(5));
                    if service.generation() != generation {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
    }

    if !enforce {
        eprintln!("iqal: enforcement disabled (--no-enforce)");
    } else if !nix::unistd::geteuid().is_root() {
        eprintln!("iqal: not running as root; rules are stored but not enforced");
    } else {
        linux::start_enforcement(service.clone(), state_dir.clone(), network).map_err(|e| format!("starting enforcement: {e}"))?;
    }

    eprintln!(
        "iqal: service {} started (socket {}, state {})",
        env!("CARGO_PKG_VERSION"),
        socket.display(),
        state_dir.display()
    );
    system::sd_notify("READY=1");
    server::serve(listener, service);
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("iqal-daemon: this platform is not supported yet");
    std::process::exit(1);
}
