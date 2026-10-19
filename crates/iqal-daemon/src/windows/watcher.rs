//! Window-title blocking, desktop side: `iqal-daemon --watch-session`,
//! started by the service on each signed-in user's desktop, running as
//! LocalSystem so the user cannot stop it. It reports the visible windows
//! to the service whenever one appears or changes its title (and every
//! couple of seconds, so new rules apply to open windows), and closes the
//! windows the service names: politely first, then by ending the program.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use iqal_ipc::client::Client;
use iqal_ipc::{Request, Response, WindowInfo};
use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_SYSTEM_FOREGROUND, EnumWindows,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible, MSG,
    MsgWaitForMultipleObjects, OBJID_WINDOW, PM_REMOVE, PeekMessageW, PostMessageW, QS_ALLINPUT, TranslateMessage,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_CLOSE,
};

use super::procs;

/// Even without window changes, report this often (rules and schedules
/// change too).
const REPORT_EVERY: Duration = Duration::from_secs(2);
/// After a change, wait this long for more (titles change in bursts).
const SETTLE: Duration = Duration::from_millis(100);
/// How long a window gets to close itself before its program is ended.
const GRACE: Duration = Duration::from_millis(1500);

static CHANGED: AtomicBool = AtomicBool::new(true);

pub fn run() -> Result<(), String> {
    let mut client = Client::connect().map_err(|e| format!("connecting to the service: {e}"))?;
    match client.call(&Request::WatchWindows).map_err(|e| e.to_string())? {
        Response::Subscribed => {}
        other => return Err(format!("service refused the window watch: {other:?}")),
    }
    // One hook per event: a range would also wake us for every mouse move.
    let hooks = [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_SHOW, EVENT_OBJECT_NAMECHANGE].map(|event| {
        // SAFETY: an out-of-context hook; the callback only sets a flag.
        unsafe {
            SetWinEventHook(
                event,
                event,
                std::ptr::null_mut(),
                Some(on_win_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        }
    });
    if hooks.iter().any(|h| h.is_null()) {
        eprintln!("iqal-watch: no window events; checking every {} s", REPORT_EVERY.as_secs());
    }
    // Windows asked to close, and when.
    let mut closing: HashMap<u64, Instant> = HashMap::new();
    let mut last_report = Instant::now() - REPORT_EVERY;
    let mut changed_at: Option<Instant> = None;
    loop {
        pump_messages();
        if CHANGED.swap(false, Ordering::Relaxed) {
            changed_at.get_or_insert_with(Instant::now);
        }
        let due = changed_at.is_some_and(|t| t.elapsed() >= SETTLE) || last_report.elapsed() >= REPORT_EVERY;
        if due {
            changed_at = None;
            last_report = Instant::now();
            let windows = visible_windows();
            let pids: HashMap<u64, u32> = windows.iter().map(|w| (w.id, w.pid)).collect();
            let close = match client.call(&Request::Windows { windows }).map_err(|e| e.to_string())? {
                Response::CloseWindows { windows } => windows,
                other => return Err(format!("unexpected answer: {other:?}")),
            };
            closing.retain(|id, _| close.iter().any(|c| c.id == *id));
            for window in close {
                let hwnd = window.id as usize as HWND;
                let asked = *closing.entry(window.id).or_insert_with(|| {
                    // SAFETY: posting to a window handle that may be gone is harmless.
                    unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
                    Instant::now()
                });
                // SAFETY: as above.
                if window.force && asked.elapsed() >= GRACE && unsafe { IsWindow(hwnd) } != 0 {
                    if let Some(&pid) = pids.get(&window.id) {
                        procs::kill(pid);
                    }
                }
            }
        }
        // Sleep until a message (window event) arrives or the next check.
        let wait = if changed_at.is_some() || !closing.is_empty() { SETTLE } else { REPORT_EVERY };
        // SAFETY: no handles; wakes on any input for this thread.
        unsafe { MsgWaitForMultipleObjects(0, std::ptr::null(), 0, wait.as_millis() as u32, QS_ALLINPUT) };
    }
}

fn pump_messages() {
    // SAFETY: MSG is plain data; standard message loop.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn on_win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    _hwnd: HWND,
    object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    if object == OBJID_WINDOW || event == EVENT_SYSTEM_FOREGROUND {
        CHANGED.store(true, Ordering::Relaxed);
    }
}

/// Visible top-level windows that have a title.
fn visible_windows() -> Vec<WindowInfo> {
    let mut out: Vec<WindowInfo> = Vec::new();
    // SAFETY: the callback receives a pointer to `out` for this call only.
    unsafe { EnumWindows(Some(collect), (&raw mut out) as LPARAM) };
    out
}

unsafe extern "system" fn collect(hwnd: HWND, out: LPARAM) -> i32 {
    // SAFETY: `out` is the Vec passed by visible_windows.
    let out = unsafe { &mut *(out as *mut Vec<WindowInfo>) };
    // SAFETY: hwnd comes from EnumWindows.
    unsafe {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return 1;
        }
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if n > 0 && pid != 0 {
            out.push(WindowInfo { id: hwnd as usize as u64, pid, title: String::from_utf16_lossy(&buf[..n as usize]) });
        }
    }
    1
}
