//! Dialog windows with their own controls (a panel): the main window waits
//! while one is open, as with Windows' own dialogs.

use std::cell::Cell;
use std::rc::Rc;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, GetWindowRect,
    IDC_ARROW, IsDialogMessageW, IsWindow, LoadCursorW, MSG, PostMessageW, RegisterClassExW, SW_HIDE, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetForegroundWindow, SetWindowPos, ShowWindow, TranslateMessage, WM_APP, WM_CLOSE,
    WM_ERASEBKGND, WM_KEYDOWN, WM_SIZE, WNDCLASSEXW, WS_CAPTION, WS_CLIPCHILDREN, WS_EX_DLGMODALFRAME, WS_EX_LAYOUTRTL, WS_POPUP,
    WS_SYSMENU,
};

use crate::panel::{self, Panel};
use crate::theme;
use crate::win::{self, scale};

const CLASS: &str = "IqalDialog";
/// Ends the dialog's message loop.
const WM_DONE: u32 = WM_APP + 2;
const VK_ESCAPE: usize = 0x1B;

thread_local! {
    /// Dialogs asked to close whose loop has not ended yet.
    static CLOSED: std::cell::RefCell<std::collections::HashSet<isize>> = std::cell::RefCell::new(std::collections::HashSet::new());
}

fn closed(hwnd: HWND) -> bool {
    CLOSED.with(|c| c.borrow().contains(&(hwnd as isize)))
}

/// Closes the dialog it was made for.
#[derive(Clone)]
pub struct Closer {
    hwnd: HWND,
    body: Panel,
    /// The tallest the dialog's client area may get, in device pixels.
    max_height: i32,
}

impl Closer {
    pub fn close(&self) {
        // SAFETY: posting to a window that may already be gone is harmless.
        unsafe { PostMessageW(self.hwnd, WM_CLOSE, 0, 0) };
    }

    /// The dialog window, as the owner of message boxes it shows.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Makes the dialog as tall as its content (up to its full size), after
    /// the content changed.
    pub fn fit(&self) {
        let height = self.body.content_height().clamp(1, self.max_height);
        let client = win::client_rect(self.hwnd);
        let mut window = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: our window.
        unsafe { GetWindowRect(self.hwnd, &mut window) };
        let frame = (window.bottom - window.top) - client.bottom;
        // SAFETY: our window.
        unsafe {
            SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                0,
                0,
                window.right - window.left,
                height + frame,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOMOVE,
            )
        };
    }
}

/// Shows a dialog of `size` (pixels at 96 DPI, client area) over `owner`
/// and returns when it is closed. `build` fills its panel.
pub fn run(owner: HWND, title: &str, size: (i32, i32), build: impl FnOnce(Panel, Closer)) {
    let timer = crate::timing::start();
    register_class();
    let class = win::wide(CLASS);
    let title = win::wide(title);
    let dpi = panel::dpi();
    let style = WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN;
    let ex_style = WS_EX_DLGMODALFRAME | if iqal_i18n::is_rtl() { WS_EX_LAYOUTRTL } else { 0 };
    let mut frame = RECT { left: 0, top: 0, right: scale(size.0, dpi), bottom: scale(size.1, dpi) };
    let mut owner_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: plain structures; the strings outlive the call.
    let hwnd = unsafe {
        AdjustWindowRectEx(&mut frame, style, 0, ex_style);
        GetWindowRect(owner, &mut owner_rect);
        let (w, h) = (frame.right - frame.left, frame.bottom - frame.top);
        let x = owner_rect.left + (owner_rect.right - owner_rect.left - w) / 2;
        let y = owner_rect.top + (owner_rect.bottom - owner_rect.top - h) / 3;
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            title.as_ptr(),
            style,
            x,
            y.max(owner_rect.top),
            w,
            h,
            owner,
            std::ptr::null_mut(),
            win::instance(),
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return;
    }
    theme::window(hwnd);
    let body = Panel::new(hwnd, true);
    resize(hwnd);
    let closer = Closer { hwnd, body, max_height: scale(size.1, dpi) };
    build(body, closer.clone());
    closer.fit();

    let done = Rc::new(Cell::new(false));
    // SAFETY: our windows; a nested message loop while the owner is disabled.
    unsafe {
        EnableWindow(owner, 0);
        ShowWindow(hwnd, SW_SHOW);
        if timer.is_some() {
            windows_sys::Win32::Graphics::Gdi::UpdateWindow(hwnd);
            crate::timing::record("dialog", timer);
        }
        let mut msg: MSG = std::mem::zeroed();
        // `closed` also catches a close handled while another dialog (a
        // message shown on top) was running its own loop.
        while !done.get() && !closed(hwnd) && IsWindow(hwnd) != 0 && GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.hwnd == hwnd && msg.message == WM_DONE {
                done.set(true);
                break;
            }
            // Esc closes the dialog, as Cancel.
            if msg.message == WM_KEYDOWN && msg.wParam == VK_ESCAPE {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
                continue;
            }
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        EnableWindow(owner, 1);
        CLOSED.with(|c| c.borrow_mut().remove(&(hwnd as isize)));
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        SetForegroundWindow(owner);
    }
}

/// The panel fills the dialog.
fn resize(hwnd: HWND) {
    let r = win::client_rect(hwnd);
    // The panel is the dialog's only child.
    // SAFETY: our window.
    let body = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindow(hwnd, 5 /* GW_CHILD */) };
    if !body.is_null() {
        // SAFETY: our panel.
        unsafe { SetWindowPos(body, std::ptr::null_mut(), 0, 0, r.right, r.bottom, SWP_NOZORDER | SWP_NOACTIVATE) };
    }
}

fn register_class() {
    thread_local! { static DONE: Cell<bool> = const { Cell::new(false) }; }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    let class = win::wide(CLASS);
    // SAFETY: the class name is copied by Windows during registration.
    unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: win::instance(),
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            // Painted in WM_ERASEBKGND, in the app's light or dark colour.
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            resize(hwnd);
            0
        }
        WM_ERASEBKGND => theme::erase(hwnd, wparam as HDC),
        WM_CLOSE => {
            // Ends the dialog's loop; the window is destroyed after it. It
            // disappears at once, even if a message on top keeps its loop
            // waiting a little longer.
            CLOSED.with(|c| c.borrow_mut().insert(hwnd as isize));
            // SAFETY: our window.
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
                PostMessageW(hwnd, WM_DONE, 0, 0);
            }
            0
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
