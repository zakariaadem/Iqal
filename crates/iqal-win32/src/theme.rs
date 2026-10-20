//! Light and dark appearance. Windows' classic controls have no dark mode
//! of their own, so in dark mode Iqal gives them Windows' dark control
//! themes where those exist (buttons, scroll bars, text boxes, drop-down
//! lists), answers their colour questions (WM_CTLCOLOR*), and draws check
//! boxes, radio buttons, tabs and the status bar itself.
//!
//! Colours are asked for by Windows' system colour numbers (COLOR_WINDOW,
//! COLOR_WINDOWTEXT…): in light mode they are Windows' own, in dark mode
//! their dark counterparts.
//!
//! The on/off switch is drawn here too, in both modes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use iqal_app::settings::GuiSettings;
use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, COLOR_BTNFACE, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_HOTLIGHT, COLOR_WINDOW,
    COLOR_WINDOWTEXT, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, GdiFlush, CreatePen, CreateSolidBrush, DT_END_ELLIPSIS, DT_NOPREFIX,
    DT_RTLREADING, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawFocusRect, DrawTextW, Ellipse, EndPaint, FillRect, FrameRect,
    GetWindowDC, ReleaseDC,
    GetSysColor, GetSysColorBrush, HALFTONE, HBRUSH, HDC, HFONT, PAINTSTRUCT, PS_SOLID,
    Polyline, RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE, RedrawWindow, RoundRect, SRCCOPY, SelectObject, SetBkMode,
    SetBrushOrgEx, SetStretchBltMode, SetTextColor, StretchBlt, TRANSPARENT,
};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::UI::Controls::{
    BST_CHECKED, CDDS_PREPAINT, CDIS_DISABLED, CDIS_FOCUS, CDIS_HOT, CDIS_SHOWKEYBOARDCUES, CDRF_DODEFAULT, CDRF_SKIPDEFAULT, NMCUSTOMDRAW,
    SetWindowTheme, TCM_GETCURSEL, TCM_GETITEMCOUNT, TCM_GETITEMRECT, TCM_GETITEMW, TCIF_TEXT, TCITEMW,
};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_CHECKBOX, BS_PUSHLIKE, BS_RADIOBUTTON, BS_TYPEMASK,
    ES_MULTILINE, EnumChildWindows, EnumThreadWindows, GWL_EXSTYLE, GWL_STYLE, GetWindowLongW, GetWindowRect, WM_NCPAINT, WS_EX_CLIENTEDGE, WM_CTLCOLORBTN, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX,
    WM_CTLCOLORSTATIC, WM_ERASEBKGND, WM_GETFONT, WM_NCDESTROY, WM_PAINT,
};

use crate::win;

/// Sizes at 96 DPI.
const BOX: i32 = 16;
const SWITCH: (i32, i32) = (40, 20);
const GAP: i32 = 8;
/// Shapes are drawn this many times larger and shrunk, for smooth edges.
const SMOOTH: i32 = 4;

thread_local! {
    static DARK: Cell<bool> = const { Cell::new(false) };
    static BRUSHES: RefCell<HashMap<i32, HBRUSH>> = RefCell::new(HashMap::new());
}

/// The text box and drop-down background in dark mode (not a system colour).
pub const FIELD: i32 = -1;

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

pub fn is_dark() -> bool {
    DARK.with(Cell::get)
}

/// Whether Windows is set to dark for apps.
fn system_is_dark() -> bool {
    let key = win::wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value = win::wide("AppsUseLightTheme");
    let mut light: u32 = 1;
    let mut size = 4u32;
    // SAFETY: valid strings and a 4-byte buffer for a DWORD.
    let ok = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), (&raw mut light).cast(), &mut size)
    } == 0;
    ok && light == 0
}

/// Works out light or dark from the setting (or Windows' own), before any
/// window is made, and again when either changes.
pub fn init() {
    let dark = match GuiSettings::load().theme.as_deref() {
        Some("dark") => true,
        Some("light") => false,
        _ => system_is_dark(),
    };
    DARK.with(|d| d.set(dark));
    BRUSHES.with(|b| {
        for brush in b.borrow_mut().drain().map(|(_, b)| b) {
            // SAFETY: brushes we made and no window uses any more.
            unsafe { DeleteObject(brush) };
        }
    });
    // Scroll bars, menus and Windows' own dialogs follow the app's mode
    // (uxtheme's SetPreferredAppMode, #135: 2 dark, 3 light).
    if let Some(set_mode) = uxtheme_fn(135) {
        // SAFETY: SetPreferredAppMode takes one int and returns one.
        let set_mode: unsafe extern "system" fn(i32) -> i32 = unsafe { std::mem::transmute(set_mode) };
        // SAFETY: as above.
        unsafe { set_mode(if dark { 2 } else { 3 }) };
    }
    if let Some(flush) = uxtheme_fn(136) {
        // SAFETY: FlushMenuThemes takes nothing.
        let flush: unsafe extern "system" fn() = unsafe { std::mem::transmute(flush) };
        // SAFETY: as above.
        unsafe { flush() };
    }
}

/// An unnamed uxtheme function, by number.
fn uxtheme_fn(ordinal: usize) -> Option<unsafe extern "system" fn() -> isize> {
    let name = win::wide("uxtheme.dll");
    // SAFETY: loading a system library; a by-number lookup.
    unsafe {
        let module = LoadLibraryW(name.as_ptr());
        if module.is_null() {
            return None;
        }
        GetProcAddress(module, ordinal as *const u8)
    }
}

/// A system colour, or its dark counterpart.
pub fn color(index: i32) -> COLORREF {
    if !is_dark() {
        // SAFETY: no pointers involved.
        return if index == FIELD { unsafe { GetSysColor(COLOR_WINDOW) } } else { unsafe { GetSysColor(index) } };
    }
    match index {
        FIELD => rgb(45, 45, 45),
        i if i == COLOR_WINDOW => rgb(32, 32, 32),
        i if i == COLOR_BTNFACE => rgb(43, 43, 43),
        i if i == COLOR_WINDOWTEXT || i == COLOR_HIGHLIGHTTEXT => rgb(255, 255, 255),
        i if i == COLOR_GRAYTEXT => rgb(160, 160, 160),
        i if i == COLOR_HIGHLIGHT => rgb(0, 95, 184),
        i if i == COLOR_HOTLIGHT => rgb(96, 205, 255),
        // SAFETY: no pointers involved.
        i => unsafe { GetSysColor(i) },
    }
}

/// A brush of `color(index)`, kept until the mode changes.
pub fn brush(index: i32) -> HBRUSH {
    if !is_dark() && index != FIELD {
        // SAFETY: no pointers involved; system brushes need no freeing.
        return unsafe { GetSysColorBrush(index) };
    }
    BRUSHES.with(|b| {
        // SAFETY: a plain colour.
        *b.borrow_mut().entry(index).or_insert_with(|| unsafe { CreateSolidBrush(color(index)) })
    })
}

/// The accent: switches that are on, ticked boxes.
fn accent() -> COLORREF {
    if is_dark() { rgb(76, 194, 255) } else { color(COLOR_HIGHLIGHT) }
}

/// Fills a window's background (WM_ERASEBKGND).
pub fn erase(hwnd: HWND, hdc: HDC) -> LRESULT {
    let r = win::client_rect(hwnd);
    // SAFETY: the DC Windows gave us for erasing.
    unsafe { FillRect(hdc, &r, brush(COLOR_WINDOW)) };
    1
}

/// A top-level window: dark title bar in dark mode.
pub fn window(hwnd: HWND) {
    let dark: i32 = i32::from(is_dark());
    // SAFETY: our window; a 4-byte BOOL.
    unsafe { DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE as u32, (&raw const dark).cast(), 4) };
    if let Some(allow) = uxtheme_fn(133) {
        // SAFETY: AllowDarkModeForWindow(HWND, BOOL) -> BOOL.
        let allow: unsafe extern "system" fn(HWND, i32) -> i32 = unsafe { std::mem::transmute(allow) };
        // SAFETY: as above.
        unsafe { allow(hwnd, dark) };
    }
}

/// A control: Windows' dark theme for its kind, where there is one.
pub fn control(hwnd: HWND) {
    let class = class_name(hwnd).to_ascii_lowercase();
    let dark = is_dark();
    // SAFETY: a live control.
    let multiline = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32 & ES_MULTILINE as u32 != 0;
    if matches!(
        class.as_str(),
        "systabcontrol32" | "msctls_statusbar32" | "listbox" | "edit" | "sysdatetimepick32" | "msctls_updown32"
    ) {
        subclass(hwnd);
    }
    if class == "sysdatetimepick32" {
        // Its up and down arrows are a control of their own.
        // SAFETY: a live control; the callback only subclasses its children.
        unsafe { EnumChildWindows(hwnd, Some(each_control), 0) };
    }
    let theme = match class.as_str() {
        // Text boxes of several lines have a scroll bar, which only the
        // Explorer theme makes dark.
        "edit" if dark && multiline => Some("DarkMode_Explorer"),
        "edit" | "combobox" if dark => Some("DarkMode_CFD"),
        "button" | "listbox" | "iqalpanel" | "scrollbar" if dark => Some("DarkMode_Explorer"),
        _ => None,
    };
    if let Some(theme) = theme {
        let theme = win::wide(theme);
        // SAFETY: our control; a NUL-terminated name.
        unsafe { SetWindowTheme(hwnd, theme.as_ptr(), std::ptr::null()) };
    } else if matches!(class.as_str(), "edit" | "combobox" | "button" | "listbox" | "iqalpanel" | "scrollbar") {
        // Back to Windows' normal theme.
        // SAFETY: our control.
        unsafe { SetWindowTheme(hwnd, std::ptr::null(), std::ptr::null()) };
    }
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    // SAFETY: the buffer's length is passed.
    let n = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

unsafe extern "system" fn each_control(hwnd: HWND, _: LPARAM) -> i32 {
    control(hwnd);
    1
}

/// After the mode changed: every window of the app takes on the new one.
pub fn apply_everywhere() {
    unsafe extern "system" fn each_window(hwnd: HWND, _: LPARAM) -> i32 {
        window(hwnd);
        control(hwnd);
        // SAFETY: our window; the callback only restyles controls.
        unsafe {
            EnumChildWindows(hwnd, Some(each_control), 0);
            RedrawWindow(hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_ERASE | RDW_FRAME | RDW_INVALIDATE | RDW_ALLCHILDREN);
        }
        1
    }
    // SAFETY: windows of this thread; the callback restyles them.
    unsafe {
        EnumThreadWindows(windows_sys::Win32::System::Threading::GetCurrentThreadId(), Some(each_window), 0);
    }
}

/// Answers a control's colour question (WM_CTLCOLOR*), or `None` to leave
/// it to Windows. `grey`: the control is a note.
pub fn ctl_color(msg: u32, hdc: HDC, grey: bool) -> Option<LRESULT> {
    let (text, back) = match msg {
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => (if grey { COLOR_GRAYTEXT } else { COLOR_WINDOWTEXT }, COLOR_WINDOW),
        WM_CTLCOLOREDIT if is_dark() => (COLOR_WINDOWTEXT, FIELD),
        WM_CTLCOLORLISTBOX if is_dark() => (COLOR_WINDOWTEXT, COLOR_WINDOW),
        _ => return None,
    };
    // SAFETY: the control's DC during this message.
    unsafe {
        SetTextColor(hdc, color(text));
        windows_sys::Win32::Graphics::Gdi::SetBkColor(hdc, color(back));
    }
    Some(brush(back) as LRESULT)
}

/// What kind of button this is, for drawing it.
#[derive(PartialEq, Clone, Copy)]
enum Kind {
    Check,
    Radio,
    Switch,
}

thread_local! {
    /// Check boxes shown as on/off switches.
    static SWITCHES: RefCell<std::collections::HashSet<isize>> = RefCell::new(std::collections::HashSet::new());
}

/// Shows this check box as an on/off switch.
pub fn make_switch(hwnd: HWND) {
    SWITCHES.with(|s| s.borrow_mut().insert(hwnd as isize));
}

pub fn forget(hwnd: HWND) {
    SWITCHES.with(|s| s.borrow_mut().remove(&(hwnd as isize)));
}

/// Draws switches always, and check boxes and radio buttons in dark mode
/// (Windows draws their text black). For WM_NOTIFY's NM_CUSTOMDRAW from a
/// button; `None` when it is not ours to draw.
pub fn custom_draw(draw: &NMCUSTOMDRAW) -> Option<LRESULT> {
    let hwnd = draw.hdr.hwndFrom;
    // SAFETY: a live control.
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    if style & BS_PUSHLIKE as u32 != 0 {
        return None;
    }
    let kind = match style & BS_TYPEMASK as u32 {
        _ if SWITCHES.with(|s| s.borrow().contains(&(hwnd as isize))) => Kind::Switch,
        t if t == BS_AUTOCHECKBOX as u32 || t == BS_CHECKBOX as u32 => Kind::Check,
        t if t == BS_AUTORADIOBUTTON as u32 || t == BS_RADIOBUTTON as u32 => Kind::Radio,
        _ => return None,
    };
    if kind != Kind::Switch && !is_dark() {
        return None;
    }
    if draw.dwDrawStage != CDDS_PREPAINT {
        return Some(CDRF_DODEFAULT as LRESULT);
    }
    let on = win::send(hwnd, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
    paint_button(hwnd, draw.hdc, draw.rc, kind, on, draw.uItemState);
    Some(CDRF_SKIPDEFAULT as LRESULT)
}

fn paint_button(hwnd: HWND, hdc: HDC, rc: RECT, kind: Kind, on: bool, state: u32) {
    let dpi = win::dpi(hwnd);
    let s = |v: i32| win::scale(v, dpi);
    let disabled = state & CDIS_DISABLED != 0;
    let hot = state & CDIS_HOT != 0;
    // SAFETY: the DC Windows gave us for drawing the control.
    unsafe { FillRect(hdc, &rc, brush(COLOR_WINDOW)) };
    let (w, h) = match kind {
        Kind::Switch => (s(SWITCH.0), s(SWITCH.1)),
        _ => (s(BOX), s(BOX)),
    };
    let top = rc.top + (rc.bottom - rc.top - h) / 2;
    let glyph = RECT { left: rc.left, top, right: rc.left + w, bottom: top + h };
    let fg = if disabled { color(COLOR_GRAYTEXT) } else { color(COLOR_WINDOWTEXT) };
    let border = if disabled { color(COLOR_GRAYTEXT) } else if is_dark() { rgb(200, 200, 200) } else { rgb(96, 96, 96) };
    let fill = if disabled { color(COLOR_GRAYTEXT) } else { accent() };
    let back = color(COLOR_WINDOW);
    let on_mark = if is_dark() { rgb(0, 0, 0) } else { rgb(255, 255, 255) };
    smooth(hdc, glyph, back, |dc, k| {
        let (gw, gh) = (w * k, h * k);
        let line = (s(1) * k).max(k);
        match kind {
            Kind::Switch => {
                let (bg, pen, knob) = if on { (fill, fill, on_mark) } else { (back, border, border) };
                shape(dc, bg, pen, line, |dc| unsafe { RoundRect(dc, 0, 0, gw, gh, gh, gh) });
                // The knob grows a little when on, and under the mouse.
                let d = gh * if on { 60 } else { 50 } / 100 + if hot && !disabled { gh / 10 } else { 0 };
                let centre = if on { gw - gh / 2 } else { gh / 2 };
                let (x, y) = (centre - d / 2, (gh - d) / 2);
                shape(dc, knob, knob, 1, |dc| unsafe { Ellipse(dc, x, y, x + d, y + d) });
            }
            Kind::Check => {
                let r = 4 * line;
                if on {
                    shape(dc, fill, fill, line, |dc| unsafe { RoundRect(dc, 0, 0, gw, gh, r, r) });
                    let points = [
                        POINT { x: gw * 22 / 100, y: gh * 52 / 100 },
                        POINT { x: gw * 42 / 100, y: gh * 72 / 100 },
                        POINT { x: gw * 78 / 100, y: gh * 32 / 100 },
                    ];
                    // SAFETY: drawing into our memory DC.
                    unsafe {
                        let pen = CreatePen(PS_SOLID, 2 * line, on_mark);
                        let old = SelectObject(dc, pen);
                        Polyline(dc, points.as_ptr(), 3);
                        SelectObject(dc, old);
                        DeleteObject(pen);
                    }
                } else {
                    shape(dc, back, border, line, |dc| unsafe { RoundRect(dc, 0, 0, gw, gh, r, r) });
                }
            }
            Kind::Radio => {
                if on {
                    shape(dc, fill, fill, line, |dc| unsafe { Ellipse(dc, 0, 0, gw, gh) });
                    let d = gw * 2 / 5;
                    let at = (gw - d) / 2;
                    shape(dc, on_mark, on_mark, 1, |dc| unsafe { Ellipse(dc, at, at, at + d, at + d) });
                } else {
                    shape(dc, back, border, line, |dc| unsafe { Ellipse(dc, 0, 0, gw, gh) });
                }
            }
        }
    });
    // The label.
    let text = win::text(hwnd);
    let mut r = RECT { left: glyph.right + s(GAP), ..rc };
    let font = win::send(hwnd, WM_GETFONT, 0, 0) as HFONT;
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    // SAFETY: drawing into the control's DC.
    unsafe {
        let old = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, fg);
        DrawTextW(hdc, wide.as_ptr(), wide.len() as i32 - 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX | rtl);
        if state & CDIS_FOCUS != 0 && state & CDIS_SHOWKEYBOARDCUES != 0 {
            let mut measured = r;
            DrawTextW(
                hdc,
                wide.as_ptr(),
                wide.len() as i32 - 1,
                &mut measured,
                DT_SINGLELINE | DT_NOPREFIX | windows_sys::Win32::Graphics::Gdi::DT_CALCRECT | rtl,
            );
            let height = measured.bottom - measured.top;
            let focus = RECT {
                left: r.left - s(2),
                top: r.top + (r.bottom - r.top - height) / 2 - s(1),
                right: (r.left + measured.right - measured.left + s(2)).min(rc.right),
                bottom: r.top + (r.bottom - r.top + height) / 2 + s(1),
            };
            DrawFocusRect(hdc, &focus);
        }
        SelectObject(hdc, old);
    }
}

/// Draws with `paint` at SMOOTH times the size into a memory picture, then
/// shrinks it into `target` of `hdc`: GDI shapes have jagged edges, shrunk
/// ones are smooth.
fn smooth(hdc: HDC, target: RECT, back: COLORREF, paint: impl FnOnce(HDC, i32)) {
    let (w, h) = (target.right - target.left, target.bottom - target.top);
    let (bw, bh) = (w * SMOOTH, h * SMOOTH);
    // SAFETY: a memory DC and bitmap made, used and freed here.
    unsafe {
        let mem = CreateCompatibleDC(hdc);
        let bitmap = CreateCompatibleBitmap(hdc, bw, bh);
        let old_bitmap = SelectObject(mem, bitmap);
        let back_brush = CreateSolidBrush(back);
        FillRect(mem, &RECT { left: 0, top: 0, right: bw, bottom: bh }, back_brush);
        DeleteObject(back_brush);
        paint(mem, SMOOTH);
        SetStretchBltMode(hdc, HALFTONE);
        SetBrushOrgEx(hdc, 0, 0, std::ptr::null_mut());
        StretchBlt(hdc, target.left, target.top, w, h, mem, 0, 0, bw, bh, SRCCOPY);
        SelectObject(mem, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(mem);
    }
}

/// Draws one shape filled with `fill` and outlined with `outline`.
fn shape(dc: HDC, fill: COLORREF, outline: COLORREF, width: i32, draw: impl FnOnce(HDC) -> i32) {
    // SAFETY: drawing into our memory DC; the pen and brush are freed here.
    unsafe {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, width, outline);
        let (old_brush, old_pen) = (SelectObject(dc, brush), SelectObject(dc, pen));
        draw(dc);
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        DeleteObject(brush);
        DeleteObject(pen);
    }
}

/// The width a switch or check box needs for its text, in pixels at 96 DPI.
pub fn switch_width(text: &str) -> i32 {
    crate::panel::text_width(text) * 96 / crate::panel::dpi().max(1) as i32 + SWITCH.0 + GAP + 8
}

// Tabs and the status bar have no dark theme: in dark mode they are drawn
// here (in light mode they are left to Windows).

fn subclass(hwnd: HWND) {
    // SAFETY: our control; the procedure stays valid for the program's life.
    // Subclassing again with the same id only replaces the data.
    unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0) };
}

/// The sunken edge of lists and text boxes is drawn bright in dark mode:
/// paint it over in a quiet grey.
fn paint_edge(hwnd: HWND) {
    // SAFETY: a live control.
    if unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_CLIENTEDGE == 0 {
        return;
    }
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: our control's whole-window DC, released here.
    unsafe {
        GetWindowRect(hwnd, &mut r);
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        let hdc = GetWindowDC(hwnd);
        let edge = CreateSolidBrush(rgb(80, 80, 80));
        let outer = RECT { left: 0, top: 0, right: w, bottom: h };
        FrameRect(hdc, &outer, edge);
        let inner = RECT { left: 1, top: 1, right: w - 1, bottom: h - 1 };
        FrameRect(hdc, &inner, brush(if class_name(hwnd).eq_ignore_ascii_case("edit") { FIELD } else { COLOR_WINDOW }));
        DeleteObject(edge);
        ReleaseDC(hwnd, hdc);
    }
}

unsafe extern "system" fn subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _: usize, _: usize) -> LRESULT {
    let class = class_name(hwnd).to_ascii_lowercase();
    if is_dark() && (class == "sysdatetimepick32" || class == "msctls_updown32") {
        match msg {
            WM_ERASEBKGND => return 1,
            WM_PAINT => {
                if class == "sysdatetimepick32" {
                    // The picker makes its arrows when first sized: catch them.
                    // SAFETY: a live control; the callback only subclasses its children.
                    unsafe { EnumChildWindows(hwnd, Some(each_control), 0) };
                }
                paint_darkened(hwnd);
                return 0;
            }
            _ => {}
        }
    } else if is_dark() && (class == "listbox" || class == "edit") {
        if msg == WM_NCPAINT {
            // SAFETY: default painting first (scroll bars), then the edge.
            let result = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
            paint_edge(hwnd);
            return result;
        }
    } else if is_dark() {
        match msg {
            WM_ERASEBKGND => return 1,
            WM_PAINT => {
                // SAFETY: a PAINTSTRUCT for BeginPaint to fill.
                let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
                // SAFETY: our control, inside WM_PAINT.
                let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
                if class_name(hwnd).eq_ignore_ascii_case("SysTabControl32") {
                    paint_tabs(hwnd, hdc);
                } else {
                    paint_status(hwnd, hdc);
                }
                // SAFETY: matches BeginPaint.
                unsafe { EndPaint(hwnd, &ps) };
                return 0;
            }
            _ => {}
        }
    }
    if msg == WM_NCDESTROY {
        forget(hwnd);
    }
    // SAFETY: default handling.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// For controls with no dark look at all (time and date pickers and their
/// arrows): Windows draws them light into a memory picture, whose greys are
/// then turned around (white becomes the dark field colour, black text
/// becomes white) before it is shown. Coloured parts (the highlighted part
/// of a time) keep their colour.
fn paint_darkened(hwnd: HWND) {
    // SAFETY: a PAINTSTRUCT for BeginPaint to fill.
    let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    // SAFETY: our control, inside WM_PAINT.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    let client = win::client_rect(hwnd);
    let (w, h) = (client.right, client.bottom);
    if w > 0 && h > 0 {
        let field = color(FIELD);
        let (fr, fg, fb) = ((field & 0xff) as i32, ((field >> 8) & 0xff) as i32, ((field >> 16) & 0xff) as i32);
        // SAFETY: a memory DC and 32-bit picture made, used and freed here;
        // the pixel buffer is as large as the picture.
        unsafe {
            let mem = CreateCompatibleDC(hdc);
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = w;
            info.bmiHeader.biHeight = -h;
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(mem, &info, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
            if !bitmap.is_null() && !bits.is_null() {
                let old = SelectObject(mem, bitmap);
                FillRect(mem, &client, GetSysColorBrush(COLOR_WINDOW));
                // Common controls draw into the DC passed with WM_PAINT.
                DefSubclassProc(hwnd, WM_PAINT, mem as WPARAM, 0);
                GdiFlush();
                let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (w * h) as usize);
                for p in pixels.iter_mut() {
                    let (b, g, r) = ((*p & 0xff) as i32, ((*p >> 8) & 0xff) as i32, ((*p >> 16) & 0xff) as i32);
                    // Lightness turned around, hue kept: c + (255 - max - min).
                    // Then white goes to the field colour rather than black.
                    let shift = 255 - r.max(g).max(b) - r.min(g).min(b);
                    let mix = |c: i32, f: i32| (f + (255 - f) * (c + shift).clamp(0, 255) / 255) as u32;
                    *p = mix(b, fb) | mix(g, fg) << 8 | mix(r, fr) << 16;
                }
                BitBlt(hdc, 0, 0, w, h, mem, 0, 0, SRCCOPY);
                SelectObject(mem, old);
                DeleteObject(bitmap);
            }
            DeleteDC(mem);
        }
    }
    // SAFETY: matches BeginPaint.
    unsafe { EndPaint(hwnd, &ps) };
}

/// Tabs as Windows 11 draws them: text, the chosen one bold with an accent
/// line under it.
fn paint_tabs(hwnd: HWND, hdc: HDC) {
    let client = win::client_rect(hwnd);
    let dpi = win::dpi(hwnd);
    let font = win::send(hwnd, WM_GETFONT, 0, 0) as HFONT;
    let fonts = crate::panel::fonts();
    let chosen = win::send(hwnd, TCM_GETCURSEL, 0, 0);
    let count = win::send(hwnd, TCM_GETITEMCOUNT, 0, 0);
    // SAFETY: drawing into the tab control's DC; texts and rectangles are
    // valid for each message.
    unsafe {
        FillRect(hdc, &client, brush(COLOR_WINDOW));
        for i in 0..count {
            let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            win::send(hwnd, TCM_GETITEMRECT, i as usize, (&raw mut r) as isize);
            let mut buf = [0u16; 128];
            let mut item: TCITEMW = std::mem::zeroed();
            item.mask = TCIF_TEXT;
            item.pszText = buf.as_mut_ptr();
            item.cchTextMax = buf.len() as i32;
            win::send(hwnd, TCM_GETITEMW, i as usize, (&raw mut item) as isize);
            let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
            let selected = i == chosen;
            if selected {
                FillRect(hdc, &r, brush(FIELD));
                let line = RECT { left: r.left + win::scale(8, dpi), top: r.bottom - win::scale(3, dpi), right: r.right - win::scale(8, dpi), bottom: r.bottom };
                let accent_brush = CreateSolidBrush(accent());
                FillRect(hdc, &line, accent_brush);
                DeleteObject(accent_brush);
            }
            let use_font = match (&fonts, selected) {
                (Some(f), true) => f.bold,
                _ => font,
            };
            let old = SelectObject(hdc, use_font);
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, color(if selected { COLOR_WINDOWTEXT } else { COLOR_GRAYTEXT }));
            let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
            DrawTextW(hdc, buf.as_ptr(), len as i32, &mut r, DT_SINGLELINE | DT_VCENTER | windows_sys::Win32::Graphics::Gdi::DT_CENTER | DT_NOPREFIX | rtl);
            SelectObject(hdc, old);
        }
    }
}

fn paint_status(hwnd: HWND, hdc: HDC) {
    let client = win::client_rect(hwnd);
    let text = win::text(hwnd);
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let font = win::send(hwnd, WM_GETFONT, 0, 0) as HFONT;
    let pad = win::scale(6, win::dpi(hwnd));
    let mut r = RECT { left: client.left + pad, right: client.right - pad, ..client };
    // SAFETY: drawing into the status bar's DC.
    unsafe {
        FillRect(hdc, &client, brush(COLOR_BTNFACE));
        let old = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, color(COLOR_WINDOWTEXT));
        let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
        DrawTextW(hdc, wide.as_ptr(), wide.len() as i32 - 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX | rtl);
        SelectObject(hdc, old);
    }
}
