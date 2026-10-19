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

pub const SERVICE_NAME: &str = "Iqal";
const DISPLAY_NAME: &str = "Iqal";
const DESCRIPTION: &str = "Blocks the websites and apps chosen in Iqal, and keeps locked blocks in force.";

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

fn status(state: ServiceState, exit: u32) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: if state == ServiceState::Running {
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
        } else {
            ServiceControlAccept::empty()
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
    let _ = handle.set_service_status(status(ServiceState::StartPending, 0));
    let result = RUN.get().map_or(Err("nothing to run".into()), |run| run());
    if let Err(e) = result {
        eprintln!("iqal: {e}");
        let _ = handle.set_service_status(status(ServiceState::Stopped, 1));
        return;
    }
    let _ = handle.set_service_status(status(ServiceState::Running, 0));
    let _ = stop_rx.recv();
    eprintln!("iqal: stop requested");
    let _ = handle.set_service_status(status(ServiceState::Stopped, 0));
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
        executable_path: exe,
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
    let running = service.query_status().map(|s| s.current_state == ServiceState::Running).unwrap_or(false);
    if !running {
        service.start::<OsString>(&[]).map_err(|e| format!("starting the service: {e}"))?;
    }
    Ok(())
}

/// Stops and removes the service.
pub fn uninstall() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| format!("service manager (run as administrator): {e}"))?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS)
        .map_err(|e| format!("opening the service: {e}"))?;
    service.delete().map_err(|e| format!("removing the service: {e}"))?;
    if service.query_status().is_ok_and(|s| s.current_state != ServiceState::Stopped) {
        let _ = service.stop();
    }
    Ok(())
}
