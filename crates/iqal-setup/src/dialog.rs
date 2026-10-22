//! The setup's questions and messages, in a small window of its own in
//! Iqal's look (ivory and deep red, or charcoal with ivory when Windows is
//! set to dark): an icon, a title, an explanation and rounded buttons, the
//! last one the main action.

use std::cell::{Cell, RefCell};

use iqal_i18n::t;
use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontIndirectW, CreatePen, CreateSolidBrush, DT_CALCRECT,
    DT_CENTER, DT_NOPREFIX, DT_RTLREADING, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, DeleteDC, DeleteObject, DrawFocusRect,
    DrawTextW, Ellipse, EndPaint, FW_SEMIBOLD, FillRect, GetDC, HALFTONE, HDC, HFONT, LOGFONTW, PAINTSTRUCT, PS_SOLID,
    ReleaseDC, RoundRect, SRCCOPY, SelectObject, SetBkMode, SetBrushOrgEx, SetStretchBltMode, SetTextColor, StretchBlt,
    TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_FOCUS, ODS_NOFOCUSRECT, ODS_SELECTED};
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow, SystemParametersInfoForDpi};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BS_OWNERDRAW, CW_USEDEFAULT, CreateWindowExW, DI_NORMAL, DefWindowProcW, DestroyWindow,
    DispatchMessageW, DrawIconEx, GetClientRect, GetMessageW, GetSystemMetrics, HICON, IDC_ARROW, IsDialogMessageW,
    LoadCursorW, LoadImageW, IMAGE_ICON, LR_DEFAULTCOLOR, MSG, NONCLIENTMETRICSW, PostQuitMessage, RegisterClassExW,
    SM_CXSCREEN, SM_CYSCREEN, SPI_GETNONCLIENTMETRICS, SW_SHOW, SWP_NOZORDER, SetForegroundWindow, SetWindowPos,
    ShowWindow, TranslateMessage, WM_CLOSE, WM_COMMAND, WM_CTLCOLORBTN, WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND, WM_KEYDOWN,
    WM_PAINT, WM_SETFONT, WNDCLASSEXW, WS_CAPTION, WS_CHILD, WS_EX_DLGMODALFRAME, WS_EX_LAYOUTRTL, WS_POPUP, WS_SYSMENU,
    WS_TABSTOP, WS_VISIBLE,
};

#[derive(Clone, Copy)]
pub enum Icon {
    /// Iqal's own icon.
    Iqal,
    Warning,
    Error,
}

pub fn message(icon: Icon, title: &str, detail: &str) {
    show(icon, title, detail, &[]);
}

/// Returns the id of the button pressed, or `None` when closed. The last
/// button is the default.
pub fn ask(icon: Icon, title: &str, detail: &str, buttons: &[(i32, String)]) -> Option<i32> {
    show(icon, title, detail, buttons)
}

/// Sizes at 96 DPI.
const WIDTH: i32 = 460;
const MARGIN: i32 = 24;
const ICON: i32 = 32;
const GAP: i32 = 12;
const BUTTON_HEIGHT: i32 = 32;
const RADIUS: i32 = 6;
/// Shapes are drawn this many times larger and shrunk, for smooth edges.
const SMOOTH: i32 = 4;
/// Button ids start here (the setup's own ids can be anything).
const FIRST_BUTTON: u16 = 100;

struct Colours {
    page: COLORREF,
    text: COLORREF,
    grey: COLORREF,
    accent: COLORREF,
    on_accent: COLORREF,
    accent_down: COLORREF,
    button: COLORREF,
    button_down: COLORREF,
    line: COLORREF,
    warn: COLORREF,
    error: COLORREF,
    on_sign: COLORREF,
}

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

/// The same colours as the Iqal app (see iqal-win32's theme).
const LIGHT: Colours = Colours {
    page: rgb(0xF6, 0xF1, 0xE7),
    text: rgb(0x3A, 0x1F, 0x18),
    grey: rgb(0x7A, 0x5F, 0x55),
    accent: rgb(0x6B, 0x1F, 0x2A),
    on_accent: rgb(0xF6, 0xF1, 0xE7),
    accent_down: rgb(0x56, 0x18, 0x21),
    button: rgb(0xFF, 0xFC, 0xF6),
    button_down: rgb(0xE6, 0xD6, 0xC0),
    line: rgb(0xD9, 0xCB, 0xB5),
    warn: rgb(0xA8, 0x62, 0x0A),
    error: rgb(0x6B, 0x1F, 0x2A),
    on_sign: rgb(0xF6, 0xF1, 0xE7),
};

const DARK: Colours = Colours {
    page: rgb(0x1A, 0x1A, 0x1A),
    text: rgb(0xF6, 0xF1, 0xE7),
    grey: rgb(0xB5, 0xAE, 0xA2),
    accent: rgb(0xF6, 0xF1, 0xE7),
    on_accent: rgb(0x6B, 0x1F, 0x2A),
    accent_down: rgb(0xDE, 0xD5, 0xC6),
    button: rgb(0x24, 0x24, 0x24),
    button_down: rgb(0x38, 0x38, 0x38),
    line: rgb(0x38, 0x38, 0x38),
    warn: rgb(0xE8, 0xA8, 0x4C),
    error: rgb(0xE0, 0x6C, 0x7A),
    on_sign: rgb(0x1A, 0x1A, 0x1A),
};

/// What the open window shows and how it ended.
struct State {
    icon: Icon,
    title: Vec<u16>,
    detail: Vec<u16>,
    /// (id, text, window).
    buttons: Vec<(i32, Vec<u16>, HWND)>,
    pressed: Option<i32>,
    dark: bool,
    dpi: u32,
    normal: HFONT,
    bold: HFONT,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static CLASS_DONE: Cell<bool> = const { Cell::new(false) };
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Text without its ending zero.
fn text(w: &[u16]) -> &[u16] {
    &w[..w.len().saturating_sub(1)]
}

fn colours(dark: bool) -> &'static Colours {
    if dark { &DARK } else { &LIGHT }
}

/// Whether Windows is set to dark for apps.
fn system_is_dark() -> bool {
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value = wide("AppsUseLightTheme");
    let mut light: u32 = 1;
    let mut size = 4u32;
    // SAFETY: valid strings and a 4-byte buffer for a DWORD.
    let ok = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), (&raw mut light).cast(), &mut size)
    } == 0;
    ok && light == 0
}

fn scale(v: i32, dpi: u32) -> i32 {
    v * dpi as i32 / 96
}

/// Windows' message font at Windows 11's text size (14 px), and a
/// semibold one for the title (18 px).
fn fonts(dpi: u32) -> (HFONT, HFONT) {
    // SAFETY: ncm is sized as SystemParametersInfoForDpi expects.
    let mut ncm: NONCLIENTMETRICSW = unsafe { std::mem::zeroed() };
    ncm.cbSize = size_of::<NONCLIENTMETRICSW>() as u32;
    // SAFETY: as above.
    unsafe { SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, ncm.cbSize, (&raw mut ncm).cast(), 0, dpi) };
    let mut normal: LOGFONTW = ncm.lfMessageFont;
    normal.lfHeight = -scale(14, dpi);
    let mut bold = normal;
    bold.lfHeight = -scale(18, dpi);
    bold.lfWeight = FW_SEMIBOLD as i32;
    // SAFETY: valid LOGFONTW values.
    unsafe { (CreateFontIndirectW(&normal), CreateFontIndirectW(&bold)) }
}

fn show(icon: Icon, title: &str, detail: &str, buttons: &[(i32, String)]) -> Option<i32> {
    // A message without a question has a Close button.
    let close = [(0, t("action-close"))];
    let message_only = buttons.is_empty();
    let buttons = if message_only { &close[..] } else { buttons };
    let class = wide("IqalSetupDialog");
    let rtl = iqal_i18n::is_rtl();
    let dark = system_is_dark();
    // SAFETY: plain structures; the class name outlives registration.
    let hwnd = unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        if !CLASS_DONE.with(|c| c.replace(true)) {
            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                // The setup's icon, resource 1 (see build.rs).
                hIcon: windows_sys::Win32::UI::WindowsAndMessaging::LoadIconW(instance, 1 as _),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);
        }
        let window_title = wide(&t("setup-window-title"));
        CreateWindowExW(
            WS_EX_DLGMODALFRAME | if rtl { WS_EX_LAYOUTRTL } else { 0 },
            class.as_ptr(),
            window_title.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return None;
    }
    // SAFETY: our window.
    let dpi = match unsafe { GetDpiForWindow(hwnd) } {
        0 => 96,
        d => d,
    };
    let c = colours(dark);
    let dark_flag: i32 = i32::from(dark);
    // SAFETY: our window; a 4-byte BOOL and 4-byte COLORREFs.
    unsafe {
        DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE as u32, (&raw const dark_flag).cast(), 4);
        DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR as u32, (&raw const c.page).cast(), 4);
        DwmSetWindowAttribute(hwnd, DWMWA_TEXT_COLOR as u32, (&raw const c.text).cast(), 4);
        DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR as u32, (&raw const c.line).cast(), 4);
    }
    let (normal, bold) = fonts(dpi);
    let mut made = Vec::new();
    for (i, (id, label)) in buttons.iter().enumerate() {
        let label = wide(label);
        // SAFETY: our window; a NUL-terminated class name and text.
        let button = unsafe {
            CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                label.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW as u32,
                0,
                0,
                0,
                0,
                hwnd,
                (FIRST_BUTTON as usize + i) as _,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            )
        };
        // SAFETY: our button and font.
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(button, WM_SETFONT, normal as usize, 0) };
        made.push((*id, label, button));
    }
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            icon,
            title: wide(title),
            detail: wide(detail),
            buttons: made,
            pressed: None,
            dark,
            dpi,
            normal,
            bold,
        })
    });
    layout(hwnd);
    // SAFETY: our window; standard message loop. IsDialogMessage gives Tab
    // between the buttons and Enter and Space on the focused one.
    unsafe {
        // Focus boxes hidden until the keyboard is used (WM_CHANGEUISTATE,
        // UIS_INITIALIZE, UISF_HIDEFOCUS | UISF_HIDEACCEL).
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(hwnd, 0x0127, 3 | 3 << 16, 0);
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        if let Some(last) = STATE.with(|s| s.borrow().as_ref().and_then(|s| s.buttons.last().map(|b| b.2))) {
            SetFocus(last);
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            // Esc closes, as Cancel would.
            if msg.message == WM_KEYDOWN && msg.wParam == 0x1B {
                DestroyWindow(hwnd);
                continue;
            }
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
    let state = STATE.with(|s| s.borrow_mut().take());
    let pressed = state.as_ref().and_then(|s| s.pressed);
    if let Some(s) = state {
        // SAFETY: fonts we made; their window is gone.
        unsafe {
            DeleteObject(s.normal);
            DeleteObject(s.bold);
        }
    }
    if message_only { None } else { pressed }
}

/// Where everything goes, for a window `width` wide: (title, detail,
/// buttons' top, total height).
fn measure(s: &State, width: i32) -> (RECT, RECT, i32, i32) {
    let d = s.dpi;
    let m = scale(MARGIN, d);
    let left = m + scale(ICON + GAP, d);
    let right = width - m;
    // SAFETY: a screen DC, released again; our fonts.
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        let old = SelectObject(hdc, s.bold);
        let mut title = RECT { left, top: m, right, bottom: m };
        DrawTextW(hdc, s.title.as_ptr(), text(&s.title).len() as i32, &mut title, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
        title.right = right;
        // At least as tall as the icon, the title centred beside it.
        let title_height = (title.bottom - title.top).max(scale(ICON, d));
        title.bottom = m + title_height;
        SelectObject(hdc, s.normal);
        let mut detail = RECT { left, top: title.bottom + scale(8, d), right, bottom: title.bottom + scale(8, d) };
        if text(&s.detail).is_empty() {
            detail.bottom = title.bottom;
        } else {
            DrawTextW(hdc, s.detail.as_ptr(), text(&s.detail).len() as i32, &mut detail, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
            detail.right = right;
        }
        SelectObject(hdc, old);
        ReleaseDC(std::ptr::null_mut(), hdc);
        let buttons_top = detail.bottom + scale(24, d);
        (title, detail, buttons_top, buttons_top + scale(BUTTON_HEIGHT, d) + m)
    }
}

/// Sizes the window for its text and places the buttons (right-aligned,
/// the main one last), centred on the screen.
fn layout(hwnd: HWND) {
    let Some((dpi, height, buttons)) = STATE.with(|s| {
        let s = s.borrow();
        let s = s.as_ref()?;
        let width = scale(WIDTH, s.dpi);
        let (_, _, top, height) = measure(s, width);
        // Each button as wide as its text, at least 96.
        let mut x = width - scale(MARGIN, s.dpi);
        let mut placed = Vec::new();
        // SAFETY: a screen DC, released again; our font.
        unsafe {
            let hdc = GetDC(std::ptr::null_mut());
            let old = SelectObject(hdc, s.normal);
            for (_, label, button) in s.buttons.iter().rev() {
                let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                DrawTextW(hdc, label.as_ptr(), text(label).len() as i32, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
                let w = (r.right + scale(32, s.dpi)).max(scale(96, s.dpi));
                x -= w;
                placed.push((*button, x, top, w, scale(BUTTON_HEIGHT, s.dpi)));
                x -= scale(8, s.dpi);
            }
            SelectObject(hdc, old);
            ReleaseDC(std::ptr::null_mut(), hdc);
        }
        Some((s.dpi, height, placed))
    }) else {
        return;
    };
    // SAFETY: our window and its buttons.
    unsafe {
        let mut frame = RECT { left: 0, top: 0, right: scale(WIDTH, dpi), bottom: height };
        AdjustWindowRectExForDpi(&mut frame, WS_POPUP | WS_CAPTION | WS_SYSMENU, 0, WS_EX_DLGMODALFRAME, dpi);
        let (w, h) = (frame.right - frame.left, frame.bottom - frame.top);
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        SetWindowPos(hwnd, std::ptr::null_mut(), (sw - w) / 2, (sh - h) / 3, w, h, SWP_NOZORDER);
        for (button, x, y, w, h) in buttons {
            SetWindowPos(button, std::ptr::null_mut(), x, y, w, h, SWP_NOZORDER);
        }
    }
}

/// Draws with `paint` at SMOOTH times the size into a memory picture, then
/// shrinks it into `target`: GDI shapes have jagged edges, shrunk ones are
/// smooth.
fn smooth(hdc: HDC, target: RECT, back: COLORREF, paint: impl FnOnce(HDC, i32)) {
    let (w, h) = (target.right - target.left, target.bottom - target.top);
    if w <= 0 || h <= 0 {
        return;
    }
    let (bw, bh) = (w * SMOOTH, h * SMOOTH);
    // SAFETY: a memory DC and bitmap made, used and freed here.
    unsafe {
        let mem = CreateCompatibleDC(hdc);
        let bitmap = CreateCompatibleBitmap(hdc, bw, bh);
        let old = SelectObject(mem, bitmap);
        let back_brush = CreateSolidBrush(back);
        FillRect(mem, &RECT { left: 0, top: 0, right: bw, bottom: bh }, back_brush);
        DeleteObject(back_brush);
        paint(mem, SMOOTH);
        SetStretchBltMode(hdc, HALFTONE);
        SetBrushOrgEx(hdc, 0, 0, std::ptr::null_mut());
        StretchBlt(hdc, target.left, target.top, w, h, mem, 0, 0, bw, bh, SRCCOPY);
        SelectObject(mem, old);
        DeleteObject(bitmap);
        DeleteDC(mem);
    }
}

/// One shape filled with `fill` and outlined with `edge`.
fn shape(dc: HDC, fill: COLORREF, edge: COLORREF, width: i32, draw: impl FnOnce(HDC) -> i32) {
    // SAFETY: drawing into our memory DC; the pen and brush are freed here.
    unsafe {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, width, edge);
        let (old_brush, old_pen) = (SelectObject(dc, brush), SelectObject(dc, pen));
        draw(dc);
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        DeleteObject(brush);
        DeleteObject(pen);
    }
}

fn paint(hwnd: HWND) {
    // SAFETY: a PAINTSTRUCT for BeginPaint to fill.
    let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    // SAFETY: our window, inside WM_PAINT.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    STATE.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else { return };
        let c = colours(s.dark);
        let mut client = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: our window; r is an out value.
        unsafe { GetClientRect(hwnd, &mut client) };
        let (mut title, mut detail, _, _) = measure(s, client.right);
        let d = s.dpi;
        let m = scale(MARGIN, d);
        let side = scale(ICON, d);
        let icon_rect = RECT { left: m, top: m, right: m + side, bottom: m + side };
        let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
        // SAFETY: drawing into our window's DC with our fonts.
        unsafe {
            let page = CreateSolidBrush(c.page);
            FillRect(hdc, &client, page);
            DeleteObject(page);
            match s.icon {
                Icon::Iqal => {
                    // The setup's own icon, at the size wanted.
                    let icon = LoadImageW(GetModuleHandleW(std::ptr::null()), 1 as _, IMAGE_ICON, side, side, LR_DEFAULTCOLOR) as HICON;
                    if !icon.is_null() {
                        DrawIconEx(hdc, m, m, icon, side, side, 0, std::ptr::null_mut(), DI_NORMAL);
                        windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon(icon);
                    }
                }
                Icon::Warning | Icon::Error => {
                    // A round amber or red icon with "!" in it.
                    let fill = if matches!(s.icon, Icon::Warning) { c.warn } else { c.error };
                    smooth(hdc, icon_rect, c.page, |dc, k| {
                        shape(dc, fill, fill, 1, |dc| Ellipse(dc, 0, 0, side * k, side * k));
                    });
                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetTextColor(hdc, c.on_sign);
                    let old = SelectObject(hdc, s.bold);
                    let mark = [b'!' as u16];
                    let mut r = icon_rect;
                    DrawTextW(hdc, mark.as_ptr(), 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_NOPREFIX);
                    SelectObject(hdc, old);
                }
            }
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, c.text);
            let old = SelectObject(hdc, s.bold);
            // The title centred beside the icon when it is one line.
            let mut one = title;
            DrawTextW(hdc, s.title.as_ptr(), text(&s.title).len() as i32, &mut one, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
            let h = one.bottom - one.top;
            title.top += ((title.bottom - title.top) - h).max(0) / 2;
            DrawTextW(hdc, s.title.as_ptr(), text(&s.title).len() as i32, &mut title, DT_WORDBREAK | DT_NOPREFIX | rtl);
            SelectObject(hdc, s.normal);
            SetTextColor(hdc, c.grey);
            DrawTextW(hdc, s.detail.as_ptr(), text(&s.detail).len() as i32, &mut detail, DT_WORDBREAK | DT_NOPREFIX | rtl);
            SelectObject(hdc, old);
        }
    });
    // SAFETY: matches BeginPaint.
    unsafe { EndPaint(hwnd, &ps) };
}

/// A rounded button: the last one filled with the accent (the main action),
/// the others plain; darker while pressed.
fn draw_button(item: &DRAWITEMSTRUCT) {
    STATE.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else { return };
        let c = colours(s.dark);
        let Some(index) = s.buttons.iter().position(|b| b.2 == item.hwndItem) else { return };
        let main = index + 1 == s.buttons.len();
        let pressed = item.itemState & ODS_SELECTED != 0;
        let (fill, edge, ink) = if main {
            (if pressed { c.accent_down } else { c.accent }, c.accent, c.on_accent)
        } else {
            (if pressed { c.button_down } else { c.button }, c.line, c.text)
        };
        let rc = item.rcItem;
        let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
        let r = scale(RADIUS, s.dpi);
        smooth(item.hDC, rc, c.page, |dc, k| {
            // SAFETY: drawing into the memory DC `smooth` gives.
            shape(dc, fill, edge, k, |dc| unsafe { RoundRect(dc, 0, 0, w * k, h * k, 2 * r * k, 2 * r * k) });
        });
        let label = &s.buttons[index].1;
        let mut r = rc;
        let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
        // SAFETY: drawing into the button's DC with our font.
        unsafe {
            SetBkMode(item.hDC, TRANSPARENT as i32);
            SetTextColor(item.hDC, ink);
            let old = SelectObject(item.hDC, s.normal);
            DrawTextW(item.hDC, label.as_ptr(), text(label).len() as i32, &mut r, DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_NOPREFIX | rtl);
            SelectObject(item.hDC, old);
            // Only once the keyboard is used (Windows then clears ODS_NOFOCUSRECT).
            if item.itemState & ODS_FOCUS != 0 && item.itemState & ODS_NOFOCUSRECT == 0 {
                let inset = scale(3, s.dpi);
                let focus = RECT { left: rc.left + inset, top: rc.top + inset, right: rc.right - inset, bottom: rc.bottom - inset };
                DrawFocusRect(item.hDC, &focus);
            }
        }
    });
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_DRAWITEM => {
            // SAFETY: for WM_DRAWITEM, lparam points to a DRAWITEMSTRUCT.
            draw_button(unsafe { &*(lparam as *const DRAWITEMSTRUCT) });
            1
        }
        WM_CTLCOLORBTN => {
            // Behind the rounded corners: the page colour.
            let dark = STATE.with(|s| s.borrow().as_ref().is_some_and(|s| s.dark));
            thread_local! {
                static BRUSHES: Cell<[isize; 2]> = const { Cell::new([0, 0]) };
            }
            let mut brushes = BRUSHES.with(Cell::get);
            let i = usize::from(dark);
            if brushes[i] == 0 {
                // SAFETY: a plain colour, kept while the setup runs.
                brushes[i] = unsafe { CreateSolidBrush(colours(dark).page) } as isize;
                BRUSHES.with(|b| b.set(brushes));
            }
            brushes[i]
        }
        WM_COMMAND => {
            let mut index = (wparam & 0xffff) as u16;
            if index == 1 {
                // Enter (IDOK, from IsDialogMessage): the focused button.
                // SAFETY: plain questions about our windows.
                index = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetDlgCtrlID(windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus()) } as u16;
            }
            if index >= FIRST_BUTTON {
                let id = STATE.with(|s| s.borrow().as_ref().and_then(|s| s.buttons.get((index - FIRST_BUTTON) as usize).map(|b| b.0)));
                STATE.with(|s| {
                    if let Some(s) = s.borrow_mut().as_mut() {
                        s.pressed = id;
                    }
                });
                // SAFETY: our window.
                unsafe { DestroyWindow(hwnd) };
            }
            0
        }
        WM_CLOSE => {
            // SAFETY: our window.
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            // SAFETY: ends this dialog's message loop.
            unsafe { PostQuitMessage(0) };
            0
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
