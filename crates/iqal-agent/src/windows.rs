//! Windows: Iqal's icon in the notification area. Its balloons are shown
//! by Windows as notifications; clicking the icon or a notification opens
//! the Iqal app.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

use iqal_i18n::t;
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, GetSystemMetricsForDpi, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_LARGE_ICON, NIIF_USER, NIM_ADD, NIM_MODIFY,
    NIM_SETVERSION, NIN_BALLOONUSERCLICK, NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW,
    SetCurrentProcessExplicitAppUserModelID,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DispatchMessageW, GetCursorPos, GetMessageW, HICON, LR_DEFAULTCOLOR, MF_STRING, MSG, RegisterClassExW,
    RegisterWindowMessageW, SM_CXICON, SM_CXSMICON, SetForegroundWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenu, TranslateMessage, WM_APP, WM_CONTEXTMENU, WM_LBUTTONUP, WNDCLASSEXW,
};

const WM_TRAY: u32 = WM_APP + 1;
const ICON_ID: u32 = 1;
const MENU_OPEN: usize = 1;
/// The icon chosen with the keyboard (NIN_SELECT | NINF_KEY).
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;

/// Iqal's icon at the sizes there are pictures for.
const PICTURES: [(i32, &[u8]); 5] = [
    (16, include_bytes!("../../../extension/icon-16.png")),
    (32, include_bytes!("../../../extension/icon-32.png")),
    (48, include_bytes!("../../../extension/icon-48.png")),
    (96, include_bytes!("../../../extension/icon-96.png")),
    (128, include_bytes!("../../../extension/icon-128.png")),
];

/// Windows names notifications after the app with this identity; the
/// installer registers its name and icon (iqal-daemon, scm.rs).
const APP_ID: &str = "Iqal.Iqal";

/// The hidden window that owns the icon.
static WINDOW: AtomicIsize = AtomicIsize::new(0);
/// Sent by Windows when Explorer (and with it the notification area)
/// restarts.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Copies `s` into a fixed-size text field, cut to fit, ending in a zero.
fn fill(field: &mut [u16], s: &str) {
    let text: Vec<u16> = s.encode_utf16().take(field.len() - 1).collect();
    field[..text.len()].copy_from_slice(&text);
    field[text.len()] = 0;
}

/// Holds a mutex named for this session; a second agent in the same
/// session finds it taken and exits.
pub fn single_instance() -> Option<HANDLE> {
    let name = wide(r"Local\IqalAgent");
    // SAFETY: a NUL-terminated name; the handle is kept for the program's life.
    unsafe {
        let mutex = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        if mutex.is_null() {
            return None;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(mutex);
            return None;
        }
        Some(mutex)
    }
}

/// Iqal's icon, `size` pixels square.
fn icon(size: i32) -> HICON {
    let (_, png) = PICTURES.iter().find(|(s, _)| *s >= size).unwrap_or(&PICTURES[PICTURES.len() - 1]);
    // SAFETY: PNG data, which Windows accepts as icon data; it is copied.
    unsafe { CreateIconFromResourceEx(png.as_ptr(), png.len() as u32, 1, 0x0003_0000, size, size, LR_DEFAULTCOLOR) }
}

fn icon_data() -> NOTIFYICONDATAW {
    // SAFETY: plain data; zero is a valid starting value.
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = WINDOW.load(Ordering::Relaxed) as HWND;
    data.uID = ICON_ID;
    data
}

fn system_metric(index: i32) -> i32 {
    // SAFETY: no pointers involved.
    unsafe { GetSystemMetricsForDpi(index, GetDpiForSystem()) }
}

fn add_icon() {
    let mut data = icon_data();
    data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = icon(system_metric(SM_CXSMICON));
    fill(&mut data.szTip, &t("app-name"));
    // SAFETY: a filled-in structure for our window.
    unsafe {
        Shell_NotifyIconW(NIM_ADD, &data);
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        Shell_NotifyIconW(NIM_SETVERSION, &data);
    }
}

/// Shows a notification. Safe to call from any thread.
pub fn notify(title: &str, body: &str) -> bool {
    if WINDOW.load(Ordering::Relaxed) == 0 {
        return false;
    }
    let mut data = icon_data();
    data.uFlags = NIF_INFO;
    fill(&mut data.szInfoTitle, title);
    fill(&mut data.szInfo, body);
    data.dwInfoFlags = NIIF_USER | NIIF_LARGE_ICON;
    // Made once: the notification keeps using it after this returns.
    static LARGE: OnceLock<isize> = OnceLock::new();
    data.hBalloonIcon = *LARGE.get_or_init(|| icon(system_metric(SM_CXICON)) as isize) as HICON;
    // SAFETY: a filled-in structure for our window.
    unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) != 0 }
}

/// Starts the Iqal app, which is installed next to the agent.
fn open_app() {
    if let Ok(exe) = std::env::current_exe() {
        let app = exe.with_file_name("iqal-win32.exe");
        if let Err(e) = std::process::Command::new(&app).spawn() {
            eprintln!("iqal-agent: could not start {}: {e}", app.display());
        }
    }
}

fn menu(hwnd: HWND) {
    let open = wide(&t("tray-open"));
    let mut at = POINT { x: 0, y: 0 };
    // SAFETY: a menu made and destroyed here; our window owns it while shown.
    unsafe {
        let menu = CreatePopupMenu();
        AppendMenuW(menu, MF_STRING, MENU_OPEN, open.as_ptr());
        GetCursorPos(&mut at);
        // Without this the menu does not close when clicking elsewhere.
        SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, at.x, at.y, 0, hwnd, std::ptr::null());
        DestroyMenu(menu);
        if chosen as usize == MENU_OPEN {
            open_app();
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_TRAY {
        // With version 4, the low word of lparam says what happened.
        match (lparam & 0xffff) as u32 {
            WM_LBUTTONUP | NIN_SELECT | NIN_KEYSELECT | NIN_BALLOONUSERCLICK => open_app(),
            WM_CONTEXTMENU => menu(hwnd),
            _ => {}
        }
        return 0;
    }
    if msg != 0 && msg == TASKBAR_CREATED.load(Ordering::Relaxed) {
        add_icon();
        return 0;
    }
    // SAFETY: default handling for everything else.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Shows the icon and handles it until the session ends.
pub fn run() {
    let class = wide("IqalAgent");
    let taskbar_created = wide("TaskbarCreated");
    // SAFETY: registration and creation with NUL-terminated names; a plain
    // message loop.
    unsafe {
        // Sharp icons on every monitor.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let app_id = wide(APP_ID);
        SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());
        TASKBAR_CREATED.store(RegisterWindowMessageW(taskbar_created.as_ptr()), Ordering::Relaxed);
        let instance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassExW(&wc);
        // A window that is never shown: it receives the icon's messages.
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }
        WINDOW.store(hwnd as isize, Ordering::Relaxed);
        add_icon();
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
