//! Small helpers over the raw Win32 API: text, fonts that follow the
//! monitor's scaling, and creating child controls.

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateFontIndirectW, DeleteObject, FW_BOLD, FW_SEMIBOLD, HFONT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, SystemParametersInfoForDpi};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetClientRect, GetWindowTextLengthW, GetWindowTextW, HMENU, NONCLIENTMETRICSW,
    SPI_GETNONCLIENTMETRICS, SendMessageW, SetWindowTextW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_SETFONT,
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn instance() -> *mut std::ffi::c_void {
    // SAFETY: the module of this program; no preconditions.
    unsafe { GetModuleHandleW(std::ptr::null()) }
}

pub fn loword(v: usize) -> u16 {
    (v & 0xffff) as u16
}

pub fn hiword(v: usize) -> u16 {
    ((v >> 16) & 0xffff) as u16
}

/// Creates a child control. `id` identifies it in WM_COMMAND messages.
pub fn child(parent: HWND, class: &str, text: &str, style: WINDOW_STYLE, ex_style: WINDOW_EX_STYLE, id: u16) -> HWND {
    let (class, text) = (wide(class), wide(text));
    // SAFETY: strings are NUL-terminated; parent is a live window.
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            text.as_ptr(),
            style,
            0,
            0,
            0,
            0,
            parent,
            id as usize as HMENU,
            instance(),
            std::ptr::null(),
        )
    };
    crate::theme::control(hwnd);
    hwnd
}

/// Copies a NUL-terminated UTF-16 string.
///
/// # Safety
/// `p` points to a NUL-terminated string.
pub unsafe fn from_wide_ptr(p: *const u16) -> String {
    // SAFETY: as promised by the caller.
    unsafe {
        let len = (0..).take_while(|&i| *p.add(i) != 0).count();
        String::from_utf16_lossy(std::slice::from_raw_parts(p, len))
    }
}

pub fn set_text(hwnd: HWND, text: &str) {
    let text = wide(text);
    // SAFETY: NUL-terminated text.
    unsafe { SetWindowTextW(hwnd, text.as_ptr()) };
}

pub fn text(hwnd: HWND) -> String {
    // SAFETY: the buffer has room for the text and its NUL.
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }
}

pub fn send(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    // SAFETY: plain message; pointer arguments are the caller's to get right.
    unsafe { SendMessageW(hwnd, msg, wparam, lparam) }
}

pub fn client_rect(hwnd: HWND) -> RECT {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: r is a valid out pointer.
    unsafe { GetClientRect(hwnd, &mut r) };
    r
}

pub fn dpi(hwnd: HWND) -> u32 {
    // SAFETY: no pointers involved.
    match unsafe { GetDpiForWindow(hwnd) } {
        0 => 96,
        d => d,
    }
}

/// Device pixels for `dip` pixels at 96 DPI.
pub fn scale(dip: i32, dpi: u32) -> i32 {
    dip * dpi as i32 / 96
}

/// The fonts of the window, made for one DPI; replaced when the window
/// moves to a monitor with another scaling.
pub struct Fonts {
    pub normal: HFONT,
    pub bold: HFONT,
    pub title: HFONT,
}

impl Fonts {
    /// Windows' message font (Segoe UI on Windows 11) at this DPI.
    pub fn new(dpi: u32) -> Self {
        // SAFETY: ncm is sized as SystemParametersInfoForDpi expects.
        let mut ncm: NONCLIENTMETRICSW = unsafe { std::mem::zeroed() };
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        // SAFETY: as above.
        unsafe { SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, ncm.cbSize, (&raw mut ncm).cast(), 0, dpi) };
        let base = ncm.lfMessageFont;
        let mut bold = base;
        bold.lfWeight = FW_SEMIBOLD as i32;
        let mut title = base;
        title.lfWeight = FW_BOLD as i32;
        title.lfHeight = base.lfHeight * 3 / 2;
        // SAFETY: valid LOGFONTW values.
        unsafe { Self { normal: CreateFontIndirectW(&base), bold: CreateFontIndirectW(&bold), title: CreateFontIndirectW(&title) } }
    }

    pub fn apply(&self, hwnd: HWND) {
        send(hwnd, WM_SETFONT, self.normal as usize, 1);
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        // SAFETY: fonts we created and no control uses any more.
        unsafe {
            DeleteObject(self.normal);
            DeleteObject(self.bold);
            DeleteObject(self.title);
        }
    }
}
