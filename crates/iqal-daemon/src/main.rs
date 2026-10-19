//! iqal-daemon: the privileged Iqal service.
//!
//! Usage: `iqal-daemon [--socket PATH] [--state-dir DIR]`
//!
//! Normally started by systemd as root (Linux) or by the service manager as
//! LocalSystem (Windows). For development it can run as an ordinary user
//! with its own socket (pipe) and state directory.

mod clock;
mod dnsmsg;
mod lists;
mod service;
mod stats;
mod storage;
mod usage;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

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

    let watchdog = system::watchdog_interval();
    let period = watchdog.map_or(Duration::from_secs(10), |w| w.min(Duration::from_secs(10)));
    start_housekeeping(&service, period, move || {
        if watchdog.is_some() {
            system::sd_notify("WATCHDOG=1");
        }
    })?;

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

/// Background threads common to every platform: the periodic tick (lock
/// ends, schedules, saving the trusted time), calling `on_tick` after each,
/// and pre-made list downloads.
fn start_housekeeping(
    service: &std::sync::Arc<service::Service>,
    period: std::time::Duration,
    on_tick: impl Fn() + Send + 'static,
) -> Result<(), String> {
    use std::time::Duration;
    {
        let service = service.clone();
        std::thread::Builder::new()
            .name("iqal-tick".into())
            .spawn(move || loop {
                service.tick();
                on_tick();
                std::thread::sleep(period);
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
    Ok(())
}

#[cfg(windows)]
fn main() {
    if let Err(e) = windows_main() {
        eprintln!("iqal: {e}");
        std::process::exit(1);
    }
}

/// `iqal-daemon` alone is what Windows' service manager starts; run from a
/// terminal it serves in the foreground. `--install`/`--uninstall` add or
/// remove the service (as administrator).
#[cfg(windows)]
fn windows_main() -> Result<(), String> {
    use std::path::PathBuf;

    let mut options = WindowsOptions::default();
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().is_none() && windows::scm::run_as_service(run_windows_service)? {
        return Ok(());
    }
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--socket" => options.socket = value("--socket")?,
            "--state-dir" => options.state_dir = PathBuf::from(value("--state-dir")?),
            "--no-enforce" => options.enforce = false,
            "--watch-session" => return windows::watcher::run(),
            "--install" => {
                windows::scm::install()?;
                println!("Iqal service installed and started.");
                return Ok(());
            }
            "--uninstall" => {
                let service = load_service(&options.state_dir)?;
                if service.lock_summary().any_locked {
                    return Err("a block is locked; Iqal cannot be removed until its locks end".into());
                }
                windows::scm::uninstall()?;
                println!("Iqal service removed. Settings are kept in {}.", options.state_dir.display());
                return Ok(());
            }
            "--version" => {
                println!("iqal-daemon {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-h" | "--help" => {
                println!("usage: iqal-daemon [--socket PIPE] [--state-dir DIR] [--no-enforce] | --install | --uninstall");
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let (service, listener) = start_windows(&options)?;
    windows::server::serve(listener, service);
    Ok(())
}

#[cfg(windows)]
struct WindowsOptions {
    socket: String,
    state_dir: std::path::PathBuf,
    enforce: bool,
}

#[cfg(windows)]
impl Default for WindowsOptions {
    fn default() -> Self {
        let program_data = std::env::var_os("ProgramData").unwrap_or_else(|| r"C:\ProgramData".into());
        Self {
            socket: iqal_ipc::DEFAULT_SOCKET.into(),
            state_dir: std::path::Path::new(&program_data).join("Iqal"),
            enforce: true,
        }
    }
}

/// Started by the service manager: default options, messages to the log.
#[cfg(windows)]
fn run_windows_service() -> Result<(), String> {
    let options = WindowsOptions::default();
    prepare_state_dir(&options.state_dir)?;
    if let Err(e) = windows::log_to_file(&options.state_dir) {
        eprintln!("iqal: no log file: {e}");
    }
    let (service, listener) = start_windows(&options)?;
    std::thread::Builder::new()
        .name("iqal-pipe".into())
        .spawn(move || windows::server::serve(listener, service))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Creates the settings folder, limited to Windows and administrators.
/// Only the real one: a development folder stays the developer's own.
#[cfg(windows)]
fn prepare_state_dir(dir: &std::path::Path) -> Result<(), String> {
    if dir.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("state directory {}: {e}", dir.display()))?;
    windows::system::protect_dir(dir).map_err(|e| format!("protecting {}: {e}", dir.display()))
}

#[cfg(windows)]
fn load_service(state_dir: &std::path::Path) -> Result<service::Service, String> {
    let store = storage::Store::open(state_dir).map_err(|e| format!("state directory {}: {e}", state_dir.display()))?;
    service::Service::new(store, windows::system::boot_id()).map_err(|e| format!("loading settings: {e}"))
}

#[cfg(windows)]
fn start_windows(
    options: &WindowsOptions,
) -> Result<(std::sync::Arc<service::Service>, windows::server::Listener), String> {
    use std::sync::Arc;
    use std::time::Duration;

    if options.state_dir == WindowsOptions::default().state_dir {
        prepare_state_dir(&options.state_dir)?;
    }
    clock::init(options.state_dir.clone());
    let service = Arc::new(load_service(&options.state_dir)?);
    clock::start_sync().map_err(|e| e.to_string())?;
    let listener =
        windows::server::bind(&options.socket).map_err(|e| format!("pipe {}: {e}", options.socket))?;
    start_housekeeping(&service, Duration::from_secs(10), || {})?;
    if !options.enforce {
        eprintln!("iqal: enforcement disabled (--no-enforce)");
    } else if !windows::system::is_elevated() {
        eprintln!("iqal: not running as administrator; rules are stored but not enforced");
    } else {
        windows::start_enforcement(service.clone()).map_err(|e| format!("starting enforcement: {e}"))?;
    }
    eprintln!(
        "iqal: service {} started (pipe {}, state {})",
        env!("CARGO_PKG_VERSION"),
        options.socket,
        options.state_dir.display()
    );
    Ok((service, listener))
}

#[cfg(not(any(target_os = "linux", windows)))]
fn main() {
    eprintln!("iqal-daemon: this platform is not supported");
    std::process::exit(1);
}
