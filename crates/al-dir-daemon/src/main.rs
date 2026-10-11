//! al-dir-daemon: the privileged Al-Dir service.
//!
//! Usage: `al-dir-daemon [--socket PATH] [--state-dir DIR]`
//!
//! Normally started by systemd as root. For development it can run as an
//! ordinary user with its own socket and state directory.

mod dnsmsg;
mod service;
mod storage;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
fn main() {
    if let Err(e) = linux_main() {
        eprintln!("al-dir: {e}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
fn linux_main() -> Result<(), String> {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use linux::{server, system};

    linux::install_shutdown_handler().map_err(|e| format!("signal setup: {e}"))?;

    let mut socket = PathBuf::from(al_dir_ipc::DEFAULT_SOCKET);
    let mut state_dir = PathBuf::from("/var/lib/al-dir");
    let mut enforce = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--socket" => socket = value("--socket")?.into(),
            "--state-dir" => state_dir = value("--state-dir")?.into(),
            "--no-enforce" => enforce = false,
            "--cleanup" => {
                linux::cleanup();
                return Ok(());
            }
            "--version" => {
                println!("al-dir-daemon {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-h" | "--help" => {
                println!("usage: al-dir-daemon [--socket PATH] [--state-dir DIR] [--no-enforce] | --cleanup");
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }

    let store = storage::Store::open(&state_dir).map_err(|e| format!("state directory {}: {e}", state_dir.display()))?;
    let service = Arc::new(
        service::Service::new(store, system::boot_id()).map_err(|e| format!("loading settings: {e}"))?,
    );
    let listener = server::bind(&socket).map_err(|e| format!("socket {}: {e}", socket.display()))?;

    // Housekeeping and watchdog.
    {
        let service = service.clone();
        let watchdog = system::watchdog_interval();
        std::thread::Builder::new()
            .name("al-dir-tick".into())
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

    if !enforce {
        eprintln!("al-dir: enforcement disabled (--no-enforce)");
    } else if !nix::unistd::geteuid().is_root() {
        eprintln!("al-dir: not running as root; rules are stored but not enforced");
    } else {
        linux::start_enforcement(service.clone()).map_err(|e| format!("starting enforcement: {e}"))?;
    }

    eprintln!(
        "al-dir: service {} started (socket {}, state {})",
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
    eprintln!("al-dir-daemon: this platform is not supported yet");
    std::process::exit(1);
}
