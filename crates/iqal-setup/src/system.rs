//! What installing and removing does: files, the service, the Start menu
//! shortcut and the entry in Settings > Apps.

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use iqal_ipc::client::Client;
use iqal_ipc::{Request, Response};
use windows_service::service::{ServiceAccess, ServiceState};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_FILE_NOT_FOUND};
use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW};
use windows_sys::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW,
    RegDeleteTreeW, RegSetValueExW,
};
use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use windows_sys::Win32::UI::Shell::{FOLDERID_CommonPrograms, FOLDERID_ProgramData, FOLDERID_ProgramFiles, SHGetKnownFolderPath};
use windows_sys::core::{GUID, HRESULT};

use std::os::windows::process::CommandExt;

const SERVICE: &str = "Iqal";
/// Settings > Apps lists programs found here.
const APPS_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Iqal";
/// Iqal's programs that may be running, besides the service.
const PROGRAMS: [&str; 5] = ["iqal-agent.exe", "iqal-win32.exe", "iqal-nm-host.exe", "iqal-cli.exe", "iqal-daemon.exe"];

pub enum Failure {
    /// A block is locked: Iqal may be neither updated nor removed.
    Locked,
    Other(String),
}

impl From<String> for Failure {
    fn from(why: String) -> Self {
        Failure::Other(why)
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn known_folder(id: &GUID) -> PathBuf {
    let mut path: *mut u16 = std::ptr::null_mut();
    // SAFETY: Windows allocates the path; it is copied, then freed.
    unsafe {
        let found = SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut path) == 0 && !path.is_null();
        let result = if found {
            let len = (0..).take_while(|&i| *path.add(i) != 0).count();
            PathBuf::from(String::from_utf16_lossy(std::slice::from_raw_parts(path, len)))
        } else {
            PathBuf::new()
        };
        CoTaskMemFree(path as *const c_void);
        result
    }
}

/// Program Files\Iqal.
pub fn install_folder() -> PathBuf {
    known_folder(&FOLDERID_ProgramFiles).join("Iqal")
}

/// Where the service keeps the blocks (kept when Iqal is removed).
pub fn data_folder() -> PathBuf {
    known_folder(&FOLDERID_ProgramData).join("Iqal")
}

fn shortcut_path() -> PathBuf {
    known_folder(&FOLDERID_CommonPrograms).join("Iqal.lnk")
}

pub fn installed(folder: &Path) -> bool {
    folder.join("iqal-daemon.exe").is_file()
}

/// Whether the running service has a locked block (`None`: no service).
fn locked() -> Option<bool> {
    let mut client = Client::connect().ok()?;
    match client.call(&Request::Status).ok()? {
        Response::Status { status } => Some(status.locked_sets > 0),
        _ => None,
    }
}

/// Runs a program without a window and waits; its error output becomes
/// the failure.
fn run(program: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("{}: {e}", program.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if text.is_empty() { format!("{} {}: {}", program.display(), args.join(" "), output.status) } else { text })
}

/// Stops the service (if there is one) and waits until it has.
fn stop_service() -> Result<(), Failure> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| Failure::Other(format!("service manager: {e}")))?;
    let Ok(service) = manager.open_service(SERVICE, ServiceAccess::STOP | ServiceAccess::QUERY_STATUS) else {
        // No service, or (while locked) not allowed to stop it.
        return match locked() {
            Some(true) => Err(Failure::Locked),
            _ => Ok(()),
        };
    };
    if service.query_status().is_ok_and(|s| s.current_state != ServiceState::Stopped) && service.stop().is_err() {
        // The service refuses to stop while a block is locked.
        return Err(match locked() {
            Some(true) => Failure::Locked,
            _ => Failure::Other("the Iqal service could not be stopped".into()),
        });
    }
    let start = Instant::now();
    while service.query_status().is_ok_and(|s| s.current_state != ServiceState::Stopped) {
        if start.elapsed() > Duration::from_secs(30) {
            return Err(Failure::Other("the Iqal service did not stop".into()));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}

/// Ends Iqal's programs in every session (notifications, the app, browser
/// connectors), so their files can be replaced or removed.
fn end_programs() {
    for program in PROGRAMS {
        let _ = Command::new("taskkill").args(["/f", "/im", program]).creation_flags(CREATE_NO_WINDOW).output();
    }
    // Windows releases the files a moment after the processes end.
    std::thread::sleep(Duration::from_millis(500));
}

/// Writes a file; one still in use is renamed out of the way first (a
/// running program can be renamed, not overwritten) and removed later.
fn write(path: &Path, contents: &[u8]) -> Result<(), String> {
    if std::fs::write(path, contents).is_ok() {
        return Ok(());
    }
    let old = path.with_extension("old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(path, &old).map_err(|e| format!("{}: {e}", path.display()))?;
    delete_at_restart(&old);
    std::fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

fn delete_at_restart(path: &Path) {
    let name = wide(&path.to_string_lossy());
    // SAFETY: a NUL-terminated name; no new name means "delete".
    unsafe { MoveFileExW(name.as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT) };
}

pub fn install(folder: &Path, files: &[(&str, &[u8])]) -> Result<(), Failure> {
    if files.is_empty() {
        return Err(Failure::Other("this setup was built without Iqal's programs".into()));
    }
    if locked() == Some(true) {
        return Err(Failure::Locked);
    }
    stop_service()?;
    end_programs();
    for (name, contents) in files {
        let path = folder.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        write(&path, contents)?;
    }
    // A copy of the setup stays for removing Iqal.
    let setup = folder.join("iqal-setup.exe");
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    if !same_file(&me, &setup) {
        let contents = std::fs::read(&me).map_err(|e| format!("{}: {e}", me.display()))?;
        write(&setup, &contents)?;
    }
    // The service: registered, started, and starting the notifications.
    run(&folder.join("iqal-daemon.exe"), &["--install"])?;
    shortcut(&shortcut_path(), &folder.join("iqal-win32.exe"), "Iqal")?;
    register_app(folder, &setup)?;
    Ok(())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

pub fn uninstall(folder: &Path) -> Result<(), Failure> {
    if locked() == Some(true) {
        return Err(Failure::Locked);
    }
    let daemon = folder.join("iqal-daemon.exe");
    if daemon.is_file() {
        // Removes the service and undoes its browser and network settings;
        // refuses (like the service) while a block is locked.
        if let Err(why) = run(&daemon, &["--uninstall"]) {
            return Err(if why.contains("locked") { Failure::Locked } else { Failure::Other(why) });
        }
    }
    let _ = stop_service();
    end_programs();
    let _ = std::fs::remove_file(shortcut_path());
    delete_key(APPS_KEY);
    Ok(())
}

/// Removes the Iqal folder once this program has ended: a copy of it,
/// started from the temporary folder, waits and then removes it.
pub fn remove_folder_later(folder: &Path) {
    let Ok(me) = std::env::current_exe() else { return };
    if !me.starts_with(folder) {
        let _ = std::fs::remove_dir_all(folder);
        return;
    }
    let copy = std::env::temp_dir().join(format!("iqal-remove-{}.exe", std::process::id()));
    if std::fs::copy(&me, &copy).is_ok() {
        let _ = Command::new(&copy)
            .args(["--remove-folder", &folder.to_string_lossy(), &std::process::id().to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

/// The removing copy: waits for the uninstaller (`pid`) to end, removes
/// the folder, and leaves itself to be deleted at the next restart.
pub fn remove_folder_after(folder: &Path, pid: u32) {
    // SAFETY: a handle that may only be waited on, closed after waiting.
    unsafe {
        let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !process.is_null() {
            WaitForSingleObject(process, 30_000);
            CloseHandle(process);
        }
    }
    // Windows may hold on to the files for a moment after.
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(30) {
        if std::fs::remove_dir_all(folder).is_ok() || !folder.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    if let Ok(me) = std::env::current_exe() {
        delete_at_restart(&me);
    }
}

// Registry (machine-wide).

fn set_values(path: &str, values: &[(&str, RegValue)]) -> Result<(), String> {
    let key = wide(path);
    let mut handle: HKEY = std::ptr::null_mut();
    // SAFETY: NUL-terminated key; handle is a valid out pointer.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut handle,
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err(format!("registry {path}: {}", std::io::Error::from_raw_os_error(status as i32)));
    }
    for (name, value) in values {
        let name = wide(name);
        let (kind, bytes): (u32, Vec<u8>) = match value {
            RegValue::Str(s) => (REG_SZ, wide(s).iter().flat_map(|c| c.to_le_bytes()).collect()),
            RegValue::Dword(n) => (REG_DWORD, n.to_le_bytes().to_vec()),
        };
        // SAFETY: an open key; the data outlives the call.
        unsafe { RegSetValueExW(handle, name.as_ptr(), 0, kind, bytes.as_ptr(), bytes.len() as u32) };
    }
    // SAFETY: the key opened above.
    unsafe { RegCloseKey(handle) };
    Ok(())
}

enum RegValue<'a> {
    Str(&'a str),
    Dword(u32),
}

fn delete_key(path: &str) {
    let key = wide(path);
    // SAFETY: a NUL-terminated key name.
    let status = unsafe { RegDeleteTreeW(HKEY_LOCAL_MACHINE, key.as_ptr()) };
    if status != 0 && status != ERROR_FILE_NOT_FOUND {
        eprintln!("iqal-setup: removing {path}: {}", std::io::Error::from_raw_os_error(status as i32));
    }
}

/// The entry in Settings > Apps, whose Uninstall runs `setup --uninstall`.
fn register_app(folder: &Path, setup: &Path) -> Result<(), String> {
    let size_kb: u64 = std::fs::read_dir(folder)
        .map(|entries| entries.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum::<u64>() / 1024)
        .unwrap_or(0);
    let uninstall = format!("\"{}\" --uninstall", setup.display());
    let quiet = format!("\"{}\" --uninstall --quiet", setup.display());
    let icon = format!("{},0", folder.join("iqal-win32.exe").display());
    let location = folder.display().to_string();
    set_values(
        APPS_KEY,
        &[
            ("DisplayName", RegValue::Str("Iqal")),
            ("DisplayVersion", RegValue::Str(env!("CARGO_PKG_VERSION"))),
            ("Publisher", RegValue::Str("Iqal")),
            ("DisplayIcon", RegValue::Str(&icon)),
            ("InstallLocation", RegValue::Str(&location)),
            ("UninstallString", RegValue::Str(&uninstall)),
            ("QuietUninstallString", RegValue::Str(&quiet)),
            ("URLInfoAbout", RegValue::Str("https://github.com/zakariaadem/Iqal")),
            ("NoModify", RegValue::Dword(1)),
            ("NoRepair", RegValue::Dword(1)),
            ("EstimatedSize", RegValue::Dword(size_kb as u32)),
        ],
    )
}

// The Start menu shortcut, through the shell's link object (declared by
// hand: windows-sys has no COM interfaces). Only the methods used are typed.

#[repr(C)]
struct ShellLinkVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_path: usize,
    get_id_list: usize,
    set_id_list: usize,
    get_description: usize,
    set_description: unsafe extern "system" fn(*mut c_void, *const u16) -> HRESULT,
    get_working_directory: usize,
    set_working_directory: unsafe extern "system" fn(*mut c_void, *const u16) -> HRESULT,
    get_arguments: usize,
    set_arguments: usize,
    get_hotkey: usize,
    set_hotkey: usize,
    get_show_cmd: usize,
    set_show_cmd: usize,
    get_icon_location: usize,
    set_icon_location: usize,
    set_relative_path: usize,
    resolve: usize,
    set_path: unsafe extern "system" fn(*mut c_void, *const u16) -> HRESULT,
}

#[repr(C)]
struct PersistFileVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_class_id: usize,
    is_dirty: usize,
    load: usize,
    save: unsafe extern "system" fn(*mut c_void, *const u16, i32) -> HRESULT,
}

const CLSID_SHELL_LINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);
const IID_SHELL_LINK_W: GUID = GUID::from_u128(0x000214f9_0000_0000_c000_000000000046);
const IID_PERSIST_FILE: GUID = GUID::from_u128(0x0000010b_0000_0000_c000_000000000046);

fn shortcut(link: &Path, target: &Path, description: &str) -> Result<(), String> {
    let (link_name, target_name) = (wide(&link.to_string_lossy()), wide(&target.to_string_lossy()));
    let folder = wide(&target.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default());
    let description = wide(description);
    // SAFETY: COM calls through the interfaces' tables; every object made
    // here is released here, and the strings outlive the calls.
    let saved = unsafe {
        let com = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) >= 0;
        let mut object: *mut c_void = std::ptr::null_mut();
        let mut saved = false;
        if CoCreateInstance(&CLSID_SHELL_LINK, std::ptr::null_mut(), CLSCTX_INPROC_SERVER, &IID_SHELL_LINK_W, &mut object) == 0 {
            let link_vtbl = &**(object as *mut *const ShellLinkVtbl);
            (link_vtbl.set_path)(object, target_name.as_ptr());
            (link_vtbl.set_working_directory)(object, folder.as_ptr());
            (link_vtbl.set_description)(object, description.as_ptr());
            let mut persist: *mut c_void = std::ptr::null_mut();
            if (link_vtbl.query_interface)(object, &IID_PERSIST_FILE, &mut persist) == 0 {
                let persist_vtbl = &**(persist as *mut *const PersistFileVtbl);
                saved = (persist_vtbl.save)(persist, link_name.as_ptr(), 1) == 0;
                (persist_vtbl.release)(persist);
            }
            (link_vtbl.release)(object);
        }
        if com {
            CoUninitialize();
        }
        saved
    };
    if saved { Ok(()) } else { Err(format!("could not create {}", link.display())) }
}
