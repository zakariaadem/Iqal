//! Running as a Windows service, and adding or removing the service.

use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;

use windows_service::service::{
    ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept, ServiceErrorControl,
    ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod, ServiceInfo, ServiceStartType, ServiceState,
    ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};
use windows_sys::Win32::Security::DACL_SECURITY_INFORMATION;
use windows_sys::Win32::Storage::FileSystem::{READ_CONTROL, WRITE_DAC};
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, OpenSCManagerW, OpenServiceW, SC_MANAGER_CONNECT, SetServiceObjectSecurity,
};

pub const SERVICE_NAME: &str = "Iqal";
const DISPLAY_NAME: &str = "Iqal Blocker";
const DESCRIPTION: &str = "Blocks the websites and apps chosen in Iqal Blocker, and keeps locked blocks in force.";

/// Windows' answer when a program that is not a service asks to run as one.
const ERROR_FAILED_SERVICE_CONTROLLER_CONNECT: i32 = 1063;

/// What the service runs: starts everything, then returns. An error stops
/// the service.
static RUN: std::sync::OnceLock<fn() -> Result<(), String>> = std::sync::OnceLock::new();

define_windows_service!(ffi_service_main, service_main);

/// Hands control to Windows' service manager. Returns `Ok(false)` when the
/// program was started some other way (from a terminal).
pub fn run_as_service(run: fn() -> Result<(), String>) -> Result<bool, String> {
    let _ = RUN.set(run);
    match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        Ok(()) => Ok(true),
        Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(ERROR_FAILED_SERVICE_CONTROLLER_CONNECT) => {
            Ok(false)
        }
        Err(e) => Err(format!("service manager: {e}")),
    }
}

/// Tells whether any block is locked right now (set once the service runs).
static LOCKED: std::sync::OnceLock<Box<dyn Fn() -> bool + Send + Sync>> = std::sync::OnceLock::new();
/// How often the lock state is looked at.
const LOCK_CHECK_EVERY: Duration = Duration::from_secs(2);

pub fn set_lock_check(check: impl Fn() -> bool + Send + Sync + 'static) {
    let _ = LOCKED.set(Box::new(check));
}

fn locked() -> bool {
    LOCKED.get().is_some_and(|check| check())
}

/// Who may do what with the service. Windows itself always may do
/// everything (so the service can change this back), administrators too
/// while nothing is locked; signed-in users may look. `OW` limits what the
/// owner may do implicitly to reading these permissions.
const SERVICE_SDDL: &str = "D:(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)\
                            (A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)(A;;RC;;;OW)";
/// While a block is locked administrators may start the service and look,
/// but not stop, reconfigure or remove it, or change these permissions.
const LOCKED_SERVICE_SDDL: &str = "D:(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;SY)(A;;CCLCSWRPLOCRRC;;;BA)\
                                   (A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)(A;;RC;;;OW)";

/// Applies the service permissions for the lock state.
fn protect(locked: bool) {
    let sddl = if locked { LOCKED_SERVICE_SDDL } else { SERVICE_SDDL };
    let result = (|| -> std::io::Result<()> {
        let sd = super::system::SecurityDescriptor::from_sddl(sddl)?;
        let name = super::system::wide(SERVICE_NAME);
        // SAFETY: plain handle calls; every handle opened is closed.
        unsafe {
            let manager = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_CONNECT);
            if manager.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let service = OpenServiceW(manager, name.as_ptr(), WRITE_DAC | READ_CONTROL);
            let ok = !service.is_null() && SetServiceObjectSecurity(service, DACL_SECURITY_INFORMATION, sd.as_ptr()) != 0;
            let error = std::io::Error::last_os_error();
            if !service.is_null() {
                CloseServiceHandle(service);
            }
            CloseServiceHandle(manager);
            if ok { Ok(()) } else { Err(error) }
        }
    })();
    if let Err(e) = result {
        eprintln!("iqal: setting the service's permissions: {e}");
    }
    // Settings > Apps greys out Uninstall while a block is locked (the
    // uninstaller would refuse anyway). Only for an installed Iqal.
    if super::reg::get_string(APPS_KEY, "DisplayName").is_some()
        && let Err(e) = super::reg::set(APPS_KEY, &[("NoRemove", super::reg::Value::Dword(u32::from(locked)))])
    {
        eprintln!("iqal: marking Iqal as not removable: {e}");
    }
}

/// Iqal's entry in Settings > Apps (written by iqal-setup).
const APPS_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Iqal";

fn status(state: ServiceState, exit: u32, stoppable: bool) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: match state {
            ServiceState::Running if stoppable => ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            // While locked, Windows refuses requests to stop the service.
            ServiceState::Running => ServiceControlAccept::SHUTDOWN,
            _ => ServiceControlAccept::empty(),
        },
        exit_code: if exit == 0 { ServiceExitCode::Win32(0) } else { ServiceExitCode::ServiceSpecific(exit) },
        checkpoint: 0,
        wait_hint: Duration::from_secs(if state == ServiceState::Running { 0 } else { 10 }),
        process_id: None,
    }
}

fn service_main(_args: Vec<OsString>) {
    let (stop_tx, stop_rx) = mpsc::channel();
    let handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = stop_tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let handle = match service_control_handler::register(SERVICE_NAME, handler) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("iqal: could not register with the service manager: {e}");
            return;
        }
    };
    let _ = handle.set_service_status(status(ServiceState::StartPending, 0, false));
    // Installed by an older version: put Iqal on Safe Mode's lists now.
    if let Err(e) = keep_in_safe_mode() {
        eprintln!("iqal: could not keep Iqal on in Safe Mode: {e}");
    }
    let result = RUN.get().map_or(Err("nothing to run".into()), |run| run());
    if let Err(e) = result {
        eprintln!("iqal: {e}");
        let _ = handle.set_service_status(status(ServiceState::Stopped, 1, false));
        return;
    }
    let mut was_locked = locked();
    protect(was_locked);
    let _ = handle.set_service_status(status(ServiceState::Running, 0, !was_locked));
    // Follow the lock state until Windows asks us to stop (or shuts down).
    loop {
        match stop_rx.recv_timeout(LOCK_CHECK_EVERY) {
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let now_locked = locked();
                if now_locked != was_locked {
                    was_locked = now_locked;
                    protect(now_locked);
                    let _ = handle.set_service_status(status(ServiceState::Running, 0, !now_locked));
                    eprintln!("iqal: service can {}be stopped", if now_locked { "no longer " } else { "now " });
                }
            }
            _ => break,
        }
    }
    eprintln!("iqal: stop requested");
    // Lookups must keep working while we are stopped.
    super::dns::remove_redirect();
    let _ = handle.set_service_status(status(ServiceState::Stopped, 0, false));
}

/// Registers this program as the Iqal service (starting with Windows,
/// restarted if it ever stops unexpectedly) and starts it.
pub fn install() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)
        .map_err(|e| format!("service manager (run as administrator): {e}"))?;
    let info = ServiceInfo {
        name: SERVICE_NAME.into(),
        display_name: DISPLAY_NAME.into(),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe.clone(),
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };
    let access = ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS;
    let service = match manager.create_service(&info, access) {
        Ok(s) => s,
        Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(1073) => {
            // ERROR_SERVICE_EXISTS: update it instead.
            let s = manager.open_service(SERVICE_NAME, access).map_err(|e| e.to_string())?;
            s.change_config(&info).map_err(|e| e.to_string())?;
            s
        }
        Err(e) => return Err(format!("creating the service: {e}")),
    };
    service.set_description(DESCRIPTION).map_err(|e| e.to_string())?;
    let restart = ServiceAction { action_type: ServiceActionType::Restart, delay: Duration::from_secs(1) };
    service
        .update_failure_actions(ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(24 * 3600)),
            reboot_msg: None,
            command: None,
            actions: Some(vec![restart.clone(), restart.clone(), restart]),
        })
        .map_err(|e| e.to_string())?;
    service.set_failure_actions_on_non_crash_failures(true).map_err(|e| e.to_string())?;
    keep_in_safe_mode().map_err(|e| format!("keeping Iqal on in Safe Mode: {e}"))?;
    let running = service.query_status().map(|s| s.current_state == ServiceState::Running).unwrap_or(false);
    if !running {
        service.start::<OsString>(&[]).map_err(|e| format!("starting the service: {e}"))?;
    }
    // The agent (notifications) starts whenever anyone signs in.
    let agent = exe.with_file_name("iqal-agent.exe");
    if agent.exists() {
        let command = format!("\"{}\"", agent.display());
        super::reg::set(RUN_KEY, &[(AGENT_VALUE, super::reg::Value::Str(&command))])
            .map_err(|e| format!("starting the agent at sign-in: {e}"))?;
        // Its notifications are shown under Iqal's name and icon.
        let icon = exe.with_file_name("iqal.png");
        std::fs::write(&icon, ICON).map_err(|e| format!("writing {}: {e}", icon.display()))?;
        let icon = icon.display().to_string();
        super::reg::set(APP_ID_KEY, &[("DisplayName", super::reg::Value::Str("Iqal Blocker")), ("IconUri", super::reg::Value::Str(&icon))])
            .map_err(|e| format!("naming the notifications: {e}"))?;
    }
    Ok(())
}

/// In Safe Mode Windows starts only the services on these lists. Iqal is on
/// them, so it keeps blocking there (and locks keep holding): otherwise
/// starting in Safe Mode would be a way around it.
const SAFE_MODE_KEYS: [&str; 2] =
    [r"SYSTEM\CurrentControlSet\Control\SafeBoot\Minimal\Iqal", r"SYSTEM\CurrentControlSet\Control\SafeBoot\Network\Iqal"];

/// Puts the Iqal service on Safe Mode's lists (see `SAFE_MODE_KEYS`).
pub fn keep_in_safe_mode() -> std::io::Result<()> {
    for key in SAFE_MODE_KEYS {
        if super::reg::get_string(key, "").as_deref() != Some("Service") {
            super::reg::set(key, &[("", super::reg::Value::Str("Service"))])?;
        }
    }
    Ok(())
}

/// Programs Windows starts when anyone signs in.
const RUN_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
const AGENT_VALUE: &str = "Iqal";
/// The agent's app identity (APP_ID in iqal-agent): its name and icon.
const APP_ID_KEY: &str = r"SOFTWARE\Classes\AppUserModelId\Iqal.Iqal";
const ICON: &[u8] = include_bytes!("../../../../extension/icon-96.png");

/// Stops and removes the service.
pub fn uninstall() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| format!("service manager (run as administrator): {e}"))?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS)
        .map_err(|e| format!("opening the service: {e}"))?;
    service.delete().map_err(|e| format!("removing the service: {e}"))?;
    let _ = super::reg::delete_value(RUN_KEY, AGENT_VALUE);
    let _ = super::reg::delete_tree(APP_ID_KEY);
    for key in SAFE_MODE_KEYS {
        let _ = super::reg::delete_tree(key);
    }
    if service.query_status().is_ok_and(|s| s.current_state != ServiceState::Stopped) {
        let _ = service.stop();
    }
    Ok(())
}
