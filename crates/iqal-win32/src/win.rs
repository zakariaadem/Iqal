//! Small helpers over the raw Win32 API: text, fonts that follow the
//! monitor's scaling, and creating child controls.

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DEFAULT_CHARSET, DeleteObject, EnumFontFamiliesExW, FW_BOLD, FW_SEMIBOLD, GetDC, HFONT, LOGFONTW, ReleaseDC,
    TEXTMETRICW,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, SystemParametersInfoForDpi};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetClientRect, GetWindowTextLengthW, GetWindowTextW, HMENU, NONCLIENTMETRICSW,
    SPI_GETNONCLIENTMETRICS, SendMessageW, SetWindowTextW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_SETFONT, WS_EX_CLIENTEDGE,
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
    // Text boxes and lists are made without Windows' sunken edge: their
    // window draws a thin outline around them instead (see theme).
    let outlined = ex_style & WS_EX_CLIENTEDGE != 0 && (class.eq_ignore_ascii_case("edit") || class.eq_ignore_ascii_case("listbox"));
    let ex_style = if outlined { ex_style & !WS_EX_CLIENTEDGE } else { ex_style };
    let edit = class.eq_ignore_ascii_case("edit");
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
    if outlined {
        crate::theme::outline(hwnd, edit);
    }
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
    /// Windows' icon font (Segoe Fluent Icons), for the small icons.
    pub icons: HFONT,
}

/// Text size at 96 DPI, in pixels: Windows 11's body text.
const TEXT_PX: i32 = 14;
const TITLE_PX: i32 = 20;
const ICON_PX: i32 = 16;

impl Fonts {
    /// Windows' message font (Segoe UI) at Windows 11's text size, for
    /// this DPI. (Windows 11's Segoe UI Variable looks almost the same but
    /// takes about 0.4 MB more memory.)
    pub fn new(dpi: u32) -> Self {
        // SAFETY: ncm is sized as SystemParametersInfoForDpi expects.
        let mut ncm: NONCLIENTMETRICSW = unsafe { std::mem::zeroed() };
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        // SAFETY: as above.
        unsafe { SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, ncm.cbSize, (&raw mut ncm).cast(), 0, dpi) };
        let px = |p: i32| -(p * dpi as i32 / 96);
        let mut normal = ncm.lfMessageFont;
        normal.lfHeight = px(TEXT_PX);
        let mut bold = normal;
        bold.lfWeight = FW_SEMIBOLD as i32;
        let mut title = bold;
        title.lfHeight = px(TITLE_PX);
        title.lfWeight = FW_BOLD as i32;
        let mut icons = normal;
        icons.lfHeight = px(ICON_PX);
        icons.lfWeight = 400;
        icons.lfCharSet = DEFAULT_CHARSET as u8;
        set_face(&mut icons, if font_exists("Segoe Fluent Icons") { "Segoe Fluent Icons" } else { "Segoe MDL2 Assets" });
        // SAFETY: valid LOGFONTW values.
        unsafe {
            Self {
                normal: CreateFontIndirectW(&normal),
                bold: CreateFontIndirectW(&bold),
                title: CreateFontIndirectW(&title),
                icons: CreateFontIndirectW(&icons),
            }
        }
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
            DeleteObject(self.icons);
        }
    }
}

fn set_face(font: &mut LOGFONTW, name: &str) {
    font.lfFaceName = [0; 32];
    for (slot, unit) in font.lfFaceName.iter_mut().take(31).zip(name.encode_utf16()) {
        *slot = unit;
    }
}

/// Whether a font family is installed (asked once per name).
fn font_exists(name: &str) -> bool {
    thread_local! {
        static KNOWN: std::cell::RefCell<std::collections::HashMap<String, bool>> = Default::default();
    }
    if let Some(known) = KNOWN.with(|k| k.borrow().get(name).copied()) {
        return known;
    }
    unsafe extern "system" fn found(_: *const LOGFONTW, _: *const TEXTMETRICW, _: u32, flag: LPARAM) -> i32 {
        // SAFETY: `flag` points to the bool below, alive during the call.
        unsafe { *(flag as *mut bool) = true };
        0
    }
    let mut query: LOGFONTW = unsafe { std::mem::zeroed() };
    query.lfCharSet = DEFAULT_CHARSET as u8;
    set_face(&mut query, name);
    let mut exists = false;
    // SAFETY: a screen DC, released again; the callback only sets `exists`.
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        EnumFontFamiliesExW(hdc, &query, Some(found), (&raw mut exists) as LPARAM, 0);
        ReleaseDC(std::ptr::null_mut(), hdc);
    }
    KNOWN.with(|k| k.borrow_mut().insert(name.to_string(), exists));
    exists
}
