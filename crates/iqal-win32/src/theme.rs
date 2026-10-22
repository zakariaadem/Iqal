//! Iqal's look: ivory and deep red in light mode, charcoal with ivory in
//! dark mode. Iqal draws its buttons, check boxes, radio buttons, switches,
//! drop-down lists, tabs and status bar itself (rounded, with small icons
//! from Windows' icon font), answers the other controls' colour questions (WM_CTLCOLOR*), and
//! in dark mode gives them Windows' dark control themes where those exist
//! (scroll bars, text boxes, drop-down lists).
//!
//! Colours can be asked for by Windows' system colour numbers
//! (COLOR_WINDOW, COLOR_WINDOWTEXT…): each has its place in Iqal's
//! palette. Drawn shapes are kept and copied, so drawing stays as quick as
//! Windows' own.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use iqal_app::settings::GuiSettings;
use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, COLOR_BTNFACE, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT,
    COLOR_HOTLIGHT, COLOR_WINDOW, COLOR_WINDOWTEXT, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, CreatePen,
    CreateSolidBrush, DIB_RGB_COLORS, DT_END_ELLIPSIS, DT_NOPREFIX, DT_RTLREADING, DT_SINGLELINE, DT_VCENTER, DeleteDC,
    DeleteObject, DrawFocusRect, DrawTextW, Ellipse, EndPaint, FillRect, FrameRect, GdiFlush, GetSysColor, GetSysColorBrush,
    GetTextExtentPoint32W, HALFTONE, HBITMAP, HBRUSH, HDC, HFONT, PAINTSTRUCT, PS_SOLID, Polyline, RDW_ALLCHILDREN,
    RDW_ERASE, RDW_FRAME, RDW_INVALIDATE, RedrawWindow, RoundRect, SRCCOPY, SelectObject, SetBkMode, SetBrushOrgEx,
    SetStretchBltMode, SetTextColor, StretchBlt, TRANSPARENT, GetClipBox, MapWindowPoints, DT_CENTER, InvalidateRect, CreateBitmap, GetDC, ReleaseDC, GetTextMetricsW, TEXTMETRICW, GetWindowDC,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetFocus, IsWindowEnabled, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::UI::Controls::{
    BST_CHECKED, CDDS_PREPAINT, CDIS_DISABLED, CDIS_FOCUS, CDIS_HOT, CDIS_SELECTED, CDIS_SHOWKEYBOARDCUES, CDRF_DODEFAULT,
    CDRF_SKIPDEFAULT, NMCUSTOMDRAW, SetWindowTheme, TCIF_TEXT, TCITEMW, TCM_GETCURSEL, TCM_GETITEMCOUNT, TCM_GETITEMRECT,
    TCM_GETITEMW, TCM_SETIMAGELIST, WM_MOUSELEAVE, DRAWITEMSTRUCT, COMBOBOXINFO, GetComboBoxInfo, HIMAGELIST, ILC_COLOR32, ImageList_Add, ImageList_Create,
};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_CHECKBOX, BS_DEFPUSHBUTTON, BS_PUSHBUTTON, BS_PUSHLIKE,
    BS_RADIOBUTTON, BS_TYPEMASK, ES_MULTILINE, EnumChildWindows, EnumThreadWindows, GCW_ATOM, GWL_STYLE, GetClassWord,
    GetWindowLongW,
    GetWindowRect, WM_CTLCOLORBTN, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_ERASEBKGND, WM_GETFONT,
    GW_CHILD, GW_HWNDNEXT, GetWindow, IsWindowVisible,
    WM_NCDESTROY, WM_PAINT, CB_GETCURSEL, CB_GETDROPPEDSTATE, CB_GETLBTEXT, CB_GETLBTEXTLEN, WM_ENABLE, WM_KILLFOCUS,
    WM_MOUSEMOVE, WM_SETFOCUS, CB_SETITEMHEIGHT, WM_NCPAINT,
};

use crate::win;

/// Sizes at 96 DPI.
const BOX: i32 = 16;
const SWITCH: (i32, i32) = (40, 20);
const GAP: i32 = 8;
/// Text box margins (EM_SETMARGINS: left and right).
const EM_SETMARGINS: u32 = 0xD3;
const EC_LEFTMARGIN: u32 = 1;
const EC_RIGHTMARGIN: u32 = 2;
/// Corners of buttons.
const RADIUS: i32 = 6;
/// Room an icon takes in a button (icon and gap).
const ICON_ROOM: i32 = 22;
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
    forget_shapes();
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

/// Iqal's colours: ivory and deep red (light), charcoal with ivory (dark).
pub struct Palette {
    /// The window's background.
    pub page: COLORREF,
    /// Lists.
    pub list: COLORREF,
    /// Text boxes.
    pub field: COLORREF,
    pub text: COLORREF,
    /// Explanations and other quiet text.
    pub grey: COLORREF,
    /// Main buttons, switches that are on, ticked boxes, the chosen tab.
    pub accent: COLORREF,
    pub on_accent: COLORREF,
    pub accent_hot: COLORREF,
    pub accent_down: COLORREF,
    /// Outlines.
    pub line: COLORREF,
    /// The chosen row of a list.
    pub chosen: COLORREF,
    pub chosen_text: COLORREF,
    /// Other buttons.
    pub button: COLORREF,
    pub button_hot: COLORREF,
    pub button_down: COLORREF,
    pub link: COLORREF,
    /// The stripe on the edge of the chosen block in the block list.
    pub stripe: COLORREF,
    /// The "Locked" label: sand with gold.
    pub badge: COLORREF,
    pub badge_text: COLORREF,
    pub badge_line: COLORREF,
    /// Warnings (amber) and errors.
    pub warn: COLORREF,
    pub error: COLORREF,
    /// The mark on a message's round icon.
    pub on_sign: COLORREF,
}

const LIGHT_COLOURS: Palette = Palette {
    page: rgb(0xF6, 0xF1, 0xE7),
    list: rgb(0xFF, 0xFC, 0xF6),
    field: rgb(0xFF, 0xFF, 0xFF),
    text: rgb(0x3A, 0x1F, 0x18),
    grey: rgb(0x7A, 0x5F, 0x55),
    accent: rgb(0x6B, 0x1F, 0x2A),
    on_accent: rgb(0xF6, 0xF1, 0xE7),
    accent_hot: rgb(0x80, 0x2B, 0x37),
    accent_down: rgb(0x56, 0x18, 0x21),
    line: rgb(0xD9, 0xCB, 0xB5),
    chosen: rgb(0xF3, 0xE1, 0xDE),
    chosen_text: rgb(0x6B, 0x1F, 0x2A),
    button: rgb(0xFF, 0xFC, 0xF6),
    button_hot: rgb(0xF1, 0xE6, 0xD6),
    button_down: rgb(0xE6, 0xD6, 0xC0),
    link: rgb(0x6B, 0x1F, 0x2A),
    stripe: rgb(0x6B, 0x1F, 0x2A),
    badge: rgb(0xEF, 0xE3, 0xCC),
    badge_text: rgb(0x6B, 0x4E, 0x1F),
    badge_line: rgb(0xC9, 0xA2, 0x4A),
    warn: rgb(0xA8, 0x62, 0x0A),
    error: rgb(0x6B, 0x1F, 0x2A),
    on_sign: rgb(0xF6, 0xF1, 0xE7),
};

const DARK_COLOURS: Palette = Palette {
    page: rgb(0x1A, 0x1A, 0x1A),
    list: rgb(0x24, 0x24, 0x24),
    field: rgb(0x20, 0x20, 0x20),
    text: rgb(0xF6, 0xF1, 0xE7),
    grey: rgb(0xB5, 0xAE, 0xA2),
    accent: rgb(0xF6, 0xF1, 0xE7),
    on_accent: rgb(0x6B, 0x1F, 0x2A),
    accent_hot: rgb(0xFF, 0xFC, 0xF6),
    accent_down: rgb(0xDE, 0xD5, 0xC6),
    line: rgb(0x38, 0x38, 0x38),
    chosen: rgb(0x3A, 0x22, 0x26),
    chosen_text: rgb(0xF6, 0xF1, 0xE7),
    button: rgb(0x24, 0x24, 0x24),
    button_hot: rgb(0x2E, 0x2E, 0x2E),
    button_down: rgb(0x38, 0x38, 0x38),
    link: rgb(0xE8, 0xB4, 0xBA),
    stripe: rgb(0xB8, 0x4A, 0x5A),
    badge: rgb(0x3B, 0x30, 0x20),
    badge_text: rgb(0xE8, 0xC9, 0x8A),
    badge_line: rgb(0x8A, 0x6D, 0x2E),
    warn: rgb(0xE8, 0xA8, 0x4C),
    error: rgb(0xE0, 0x6C, 0x7A),
    on_sign: rgb(0x1A, 0x1A, 0x1A),
};

pub fn palette() -> &'static Palette {
    if is_dark() { &DARK_COLOURS } else { &LIGHT_COLOURS }
}

/// The background of lists (not a system colour).
pub const LIST: i32 = -2;
/// Outlines (not a system colour).
const LINE: i32 = -3;
/// The accent (not a system colour).
const ACCENT: i32 = -4;
/// The chosen block's stripe (not a system colour).
pub const STRIPE: i32 = -5;
/// Warnings (not a system colour).
const WARN: i32 = -6;

/// A system colour's counterpart in Iqal's colours.
pub fn color(index: i32) -> COLORREF {
    let p = palette();
    match index {
        FIELD => p.field,
        LIST => p.list,
        LINE => p.line,
        ACCENT => p.accent,
        STRIPE => p.stripe,
        WARN => p.warn,
        i if i == COLOR_WINDOW || i == COLOR_BTNFACE => p.page,
        i if i == COLOR_WINDOWTEXT => p.text,
        i if i == COLOR_HIGHLIGHTTEXT => p.chosen_text,
        i if i == COLOR_GRAYTEXT => p.grey,
        i if i == COLOR_HIGHLIGHT => p.chosen,
        i if i == COLOR_HOTLIGHT => p.link,
        // SAFETY: no pointers involved.
        i => unsafe { GetSysColor(i) },
    }
}

/// A brush of `color(index)`, kept until the mode changes.
pub fn brush(index: i32) -> HBRUSH {
    BRUSHES.with(|b| {
        // SAFETY: a plain colour.
        *b.borrow_mut().entry(index).or_insert_with(|| unsafe { CreateSolidBrush(color(index)) })
    })
}

/// The accent: switches that are on, ticked boxes.
fn accent() -> COLORREF {
    palette().accent
}

/// Fills a window's background (WM_ERASEBKGND).
pub fn erase(hwnd: HWND, hdc: HDC) -> LRESULT {
    let r = win::client_rect(hwnd);
    // SAFETY: the DC Windows gave us for erasing.
    unsafe { FillRect(hdc, &r, brush(COLOR_WINDOW)) };
    draw_outlines(hwnd, hdc);
    1
}

thread_local! {
    /// Text boxes and lists whose outline their window draws (see `outline`).
    /// (Text boxes: true.)
    static OUTLINED: RefCell<HashMap<isize, bool>> = RefCell::new(HashMap::new());
}

/// Text boxes and lists get a thin outline in Iqal's colour instead of
/// Windows' sunken edge: they are made without the edge (see
/// `win::child`), and the window they are in draws the line just around
/// them with its background. So they do no drawing of their own (Windows
/// redraws a control's edge every time it moves, which made scrolling a
/// little slower).
pub fn outline(hwnd: HWND, edit: bool) {
    if edit {
        // Room between the text and the line.
        let m = win::scale(6, win::dpi(hwnd)) as usize;
        win::send(hwnd, EM_SETMARGINS, (EC_LEFTMARGIN | EC_RIGHTMARGIN) as usize, (m | m << 16) as isize);
    }
    OUTLINED.with(|o| o.borrow_mut().insert(hwnd as isize, edit));
}

/// Whether this control's outline is drawn by its window.
pub fn outlined(hwnd: HWND) -> bool {
    OUTLINED.with(|o| o.borrow().contains_key(&(hwnd as isize)))
}

/// Whether this is a text box whose outline its window draws.
fn outlined_edit(hwnd: HWND) -> bool {
    OUTLINED.with(|o| o.borrow().get(&(hwnd as isize)).copied()) == Some(true)
}

/// Where a control's outline goes in its window: one pixel around it.
fn outline_rect(parent: HWND, child: HWND) -> RECT {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: our windows; the rectangle is an out value.
    unsafe {
        GetWindowRect(child, &mut r);
        MapWindowPoints(std::ptr::null_mut(), parent, (&raw mut r).cast(), 2);
    }
    // In right-to-left windows the mapped corners come out swapped.
    let (left, right) = (r.left.min(r.right), r.left.max(r.right));
    RECT { left: left - 1, top: r.top - 1, right: right + 1, bottom: r.bottom + 1 }
}

/// Draws the outlines of the window's outlined children that are in the
/// part being painted.
fn draw_outlines(parent: HWND, hdc: HDC) {
    if OUTLINED.with(|o| o.borrow().is_empty()) {
        return;
    }
    let mut clip = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: walking our window's children; rectangles are out values; the
    // brushes are kept.
    unsafe {
        GetClipBox(hdc, &mut clip);
        let focus = GetFocus();
        let mut child = GetWindow(parent, GW_CHILD);
        while !child.is_null() {
            if outlined(child) && IsWindowVisible(child) != 0 {
                let frame = outline_rect(parent, child);
                // The text box being typed in: in the accent, with a
                // thicker line below (as Windows 11 marks it).
                let focused = child == focus && outlined_edit(child);
                let bottom = frame.bottom + i32::from(focused);
                if frame.left < clip.right && frame.right > clip.left && frame.top < clip.bottom && bottom > clip.top {
                    if focused {
                        FrameRect(hdc, &frame, brush(ACCENT));
                        FillRect(hdc, &RECT { top: frame.bottom, bottom, ..frame }, brush(ACCENT));
                    } else {
                        FrameRect(hdc, &frame, brush(LINE));
                    }
                }
            }
            child = GetWindow(child, GW_HWNDNEXT);
        }
    }
}

/// A top-level window: its title bar and edge in Iqal's colours (Windows
/// 11; older Windows keep their own, dark in dark mode).
pub fn window(hwnd: HWND) {
    let dark: i32 = i32::from(is_dark());
    let p = palette();
    // SAFETY: our window; a 4-byte BOOL and 4-byte COLORREFs.
    unsafe {
        DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE as u32, (&raw const dark).cast(), 4);
        DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR as u32, (&raw const p.page).cast(), 4);
        DwmSetWindowAttribute(hwnd, DWMWA_TEXT_COLOR as u32, (&raw const p.text).cast(), 4);
        DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR as u32, (&raw const p.line).cast(), 4);
    }
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
    let part = match class.as_str() {
        "systabcontrol32" => Some(Part::Tabs),
        "msctls_statusbar32" => Some(Part::Status),
        "sysdatetimepick32" => Some(Part::Picker),
        "msctls_updown32" => Some(Part::Arrows),
        "combobox" => Some(Part::Dropdown),
        _ => None,
    };
    if let Some(part) = part {
        subclass(hwnd, part);
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

/// Whether a control is a button, by its class's number (no text to
/// compare: this is asked every time a button is drawn).
fn is_button(hwnd: HWND) -> bool {
    thread_local! {
        static ATOM: Cell<u16> = const { Cell::new(0) };
    }
    // SAFETY: a live control.
    let atom = unsafe { GetClassWord(hwnd, GCW_ATOM) };
    let known = ATOM.with(Cell::get);
    if known != 0 {
        return atom == known;
    }
    let button = class_name(hwnd).eq_ignore_ascii_case("button");
    if button {
        ATOM.with(|a| a.set(atom));
    }
    button
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

/// How a text stands out.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Normal,
    /// Notes and explanations.
    Grey,
    /// Warnings.
    Warning,
}

/// Answers a control's colour question (WM_CTLCOLOR*), or `None` to leave
/// it to Windows. `tone`: how a text control's text stands out.
pub fn ctl_color(msg: u32, hdc: HDC, tone: Tone) -> Option<LRESULT> {
    let text_index = match tone {
        Tone::Normal => COLOR_WINDOWTEXT,
        Tone::Grey => COLOR_GRAYTEXT,
        Tone::Warning => WARN,
    };
    let (text, back) = match msg {
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => (text_index, COLOR_WINDOW),
        WM_CTLCOLOREDIT => (COLOR_WINDOWTEXT, FIELD),
        WM_CTLCOLORLISTBOX => (COLOR_WINDOWTEXT, LIST),
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
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum Kind {
    Check,
    Radio,
    Switch,
}

/// Small icons for buttons, from Windows' icon font.
#[derive(Clone, Copy)]
#[repr(u16)]
pub enum Icon {
    Add = 0xE710,
    Menu = 0xE700,
    Lock = 0xE72E,
    Unlock = 0xE785,
    Export = 0xE896,
    Delete = 0xE74D,
    Import = 0xE8B5,
    Remove = 0xE711,
    Save = 0xE74E,
    Discard = 0xE7A7,
    Apps = 0xE71D,
    File = 0xE8A5,
    Window = 0xE8A7,
    Globe = 0xE774,
    People = 0xE716,
    Clock = 0xE823,
    ChevronDown = 0xE70D,
    Check = 0xE73E,
    Power = 0xE7E8,
}

/// How a button stands out.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Plain,
    /// The main action: filled with the accent colour.
    Main,
    /// One to think twice about (Delete): quieter text.
    Quiet,
}

thread_local! {
    /// Check boxes shown as on/off switches.
    static SWITCHES: RefCell<std::collections::HashSet<isize>> = RefCell::new(std::collections::HashSet::new());
    /// Buttons with an icon or another look than plain.
    static BUTTONS: RefCell<HashMap<isize, (Option<Icon>, Look)>> = RefCell::new(HashMap::new());
    /// Rows of tabs with an icon on each tab.
    static TAB_ICONS: RefCell<HashMap<isize, &'static [Icon]>> = RefCell::new(HashMap::new());
    /// The drop-down list under the mouse.
    static HOT_DROPDOWN: Cell<isize> = const { Cell::new(0) };
}

/// Shows this check box as an on/off switch.
pub fn make_switch(hwnd: HWND) {
    SWITCHES.with(|s| s.borrow_mut().insert(hwnd as isize));
}

/// Gives a push button an icon and a look.
pub fn style_button(hwnd: HWND, icon: Option<Icon>, look: Look) {
    BUTTONS.with(|b| b.borrow_mut().insert(hwnd as isize, (icon, look)));
}

/// Whether a button has an icon.
pub fn has_icon(hwnd: HWND) -> bool {
    BUTTONS.with(|b| b.borrow().get(&(hwnd as isize)).is_some_and(|(icon, _)| icon.is_some()))
}

pub fn forget(hwnd: HWND) {
    OUTLINED.with(|o| o.borrow_mut().remove(&(hwnd as isize)));
    SWITCHES.with(|s| s.borrow_mut().remove(&(hwnd as isize)));
    BUTTONS.with(|b| b.borrow_mut().remove(&(hwnd as isize)));
    TAB_ICONS.with(|t| t.borrow_mut().remove(&(hwnd as isize)));
    DROPDOWNS.with(|d| d.borrow_mut().remove(&(hwnd as isize)));
    if HOT_DROPDOWN.with(Cell::get) == hwnd as isize {
        HOT_DROPDOWN.with(|h| h.set(0));
    }
}

/// Draws buttons, check boxes, radio buttons and switches in Iqal's look.
/// For WM_NOTIFY's NM_CUSTOMDRAW from a button; `None` when it is not
/// ours to draw.
pub fn custom_draw(draw: &NMCUSTOMDRAW) -> Option<LRESULT> {
    let hwnd = draw.hdr.hwndFrom;
    if !is_button(hwnd) {
        return None;
    }
    if draw.dwDrawStage != CDDS_PREPAINT {
        return Some(CDRF_DODEFAULT as LRESULT);
    }
    // SAFETY: a live control.
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    let kind = match style & BS_TYPEMASK as u32 {
        _ if SWITCHES.with(|s| s.borrow().contains(&(hwnd as isize))) => Some(Kind::Switch),
        _ if style & BS_PUSHLIKE as u32 != 0 => None,
        t if t == BS_AUTOCHECKBOX as u32 || t == BS_CHECKBOX as u32 => Some(Kind::Check),
        t if t == BS_AUTORADIOBUTTON as u32 || t == BS_RADIOBUTTON as u32 => Some(Kind::Radio),
        t if t == BS_PUSHBUTTON as u32 || t == BS_DEFPUSHBUTTON as u32 => None,
        _ => return None,
    };
    let on = win::send(hwnd, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
    match kind {
        Some(kind) => paint_button(hwnd, draw.hdc, draw.rc, kind, on, draw.uItemState),
        None => paint_push(hwnd, draw.hdc, draw.rc, on, draw.uItemState),
    }
    Some(CDRF_SKIPDEFAULT as LRESULT)
}

/// A push button (or a check box that looks like one, ticked: `on`):
/// rounded, with its icon and text, lighter under the mouse and darker
/// pressed.
fn paint_push(hwnd: HWND, hdc: HDC, rc: RECT, on: bool, state: u32) {
    let p = palette();
    let dpi = win::dpi(hwnd);
    let (icon, look) = BUTTONS.with(|b| b.borrow().get(&(hwnd as isize)).copied()).unwrap_or((None, Look::Plain));
    let disabled = state & CDIS_DISABLED != 0;
    let pressed = state & CDIS_SELECTED != 0;
    let hot = state & CDIS_HOT != 0;
    let (fill, edge, ink) = match (look, disabled) {
        (_, true) => (p.page, p.line, p.grey),
        (Look::Main, false) => (if pressed { p.accent_down } else if hot { p.accent_hot } else { p.accent }, p.accent, p.on_accent),
        _ if on => (if pressed { p.button_down } else { p.chosen }, p.accent, p.chosen_text),
        (look, false) => (
            if pressed { p.button_down } else if hot { p.button_hot } else { p.button },
            p.line,
            if look == Look::Quiet { p.grey } else { p.text },
        ),
    };
    let radius = win::scale(RADIUS, dpi);
    rounded_box(hdc, rc, p.page, fill, edge, radius, true);
    // The icon and the text, centred together.
    let Some(fonts) = crate::panel::fonts() else { return };
    let mut buf = [0u16; 256];
    let text = label(hwnd, &mut buf);
    let glyph = icon.map(|i| [i as u16]);
    let gap = win::scale(6, dpi);
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    // SAFETY: drawing into the button's DC with our fonts; the extents are
    // out values.
    unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, ink);
        let mut size = SIZE { cx: 0, cy: 0 };
        let mut icon_size = SIZE { cx: 0, cy: 0 };
        let old = SelectObject(hdc, fonts.normal);
        GetTextExtentPoint32W(hdc, text.as_ptr(), text.len() as i32, &mut size);
        if let Some(g) = &glyph {
            SelectObject(hdc, fonts.icons);
            GetTextExtentPoint32W(hdc, g.as_ptr(), 1, &mut icon_size);
        }
        let total = size.cx + if glyph.is_some() { icon_size.cx + gap } else { 0 };
        let mut x = rc.left + ((rc.right - rc.left) - total).max(0) / 2;
        if let Some(g) = &glyph {
            let mut r = RECT { left: x, right: x + icon_size.cx, ..rc };
            DrawTextW(hdc, g.as_ptr(), 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
            x += icon_size.cx + gap;
            SelectObject(hdc, fonts.normal);
        }
        let mut r = RECT { left: x, right: rc.right.min(x + size.cx + 1), ..rc };
        let ellipsis = if x + size.cx <= rc.right { 0 } else { DT_END_ELLIPSIS };
        DrawTextW(hdc, text.as_ptr(), text.len() as i32, &mut r, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | ellipsis | rtl);
        SelectObject(hdc, old);
        if state & CDIS_FOCUS != 0 && state & CDIS_SHOWKEYBOARDCUES != 0 {
            let inset = win::scale(3, dpi);
            let focus = RECT { left: rc.left + inset, top: rc.top + inset, right: rc.right - inset, bottom: rc.bottom - inset };
            DrawFocusRect(hdc, &focus);
        }
    }
}

/// A box with rounded corners and a one-pixel outline, of any size: its
/// smooth corners come from one small drawn square (kept, see
/// `cached_shape`), the rest are plain fills. So a button of a new size
/// costs no new drawing.
fn rounded_box(hdc: HDC, rc: RECT, back: COLORREF, fill: COLORREF, edge: COLORREF, radius: i32, keep: bool) {
    let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
    if w <= 0 || h <= 0 {
        return;
    }
    let r = radius.min(w / 2).min(h / 2).max(1);
    let side = 2 * r + 2;
    let square = RECT { left: 0, top: 0, right: side, bottom: side };
    let corners = cached_bitmap(hdc, square, back, (0, fill, edge, r as u32), |dc, k| {
        // SAFETY: drawing into the memory DC `cached_bitmap` gives.
        shape(dc, fill, edge, k, |dc| unsafe { RoundRect(dc, 0, 0, side * k, side * k, 2 * r * k, 2 * r * k) });
    });
    if !keep {
        // Drawn now and then (drop-down lists, labels): put together in
        // place, no picture of its own to keep.
        box_parts(hdc, rc, r, side, corners, fill, edge);
        return;
    }
    // The whole box is kept too: the same buttons are drawn again and again
    // (scrolling, the mouse passing over them), and then it is one copy.
    let whole = kept_bitmap(hdc, w, h, back, (2, fill, edge, r as u32), |out| {
        box_parts(out, RECT { left: 0, top: 0, right: w, bottom: h }, r, side, corners, fill, edge);
    });
    // SAFETY: a memory DC made and freed here; the picture stays kept.
    unsafe {
        let mem = CreateCompatibleDC(hdc);
        let old = SelectObject(mem, whole);
        BitBlt(hdc, rc.left, rc.top, w, h, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old);
        DeleteDC(mem);
    }
}

/// A rounded box into `rc` of `out`: its four corners from `corners` (a
/// drawn square, see `rounded_box`), the rest plain fills.
fn box_parts(out: HDC, rc: RECT, r: i32, side: i32, corners: HBITMAP, fill: COLORREF, edge: COLORREF) {
    let (x0, y0, w, h) = (rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top);
    let at = |left: i32, top: i32, right: i32, bottom: i32| RECT { left: x0 + left, top: y0 + top, right: x0 + right, bottom: y0 + bottom };
    // SAFETY: drawing into `out`; a memory DC and brushes made and freed here.
    unsafe {
        let mem = CreateCompatibleDC(out);
        let old = SelectObject(mem, corners);
        // The four corners.
        for (x, y) in [(0, 0), (w - r, 0), (0, h - r), (w - r, h - r)] {
            let (sx, sy) = (if x == 0 { 0 } else { side - r }, if y == 0 { 0 } else { side - r });
            BitBlt(out, x0 + x, y0 + y, r, r, mem, sx, sy, SRCCOPY);
        }
        SelectObject(mem, old);
        DeleteDC(mem);
        // The middle, then the straight parts of the outline.
        let fill_brush = CreateSolidBrush(fill);
        let edge_brush = CreateSolidBrush(edge);
        FillRect(out, &at(r, 0, w - r, h), fill_brush);
        FillRect(out, &at(0, r, w, h - r), fill_brush);
        FillRect(out, &at(r, 0, w - r, 1), edge_brush);
        FillRect(out, &at(r, h - 1, w - r, h), edge_brush);
        FillRect(out, &at(0, r, 1, h - r), edge_brush);
        FillRect(out, &at(w - 1, r, w, h - r), edge_brush);
        DeleteObject(fill_brush);
        DeleteObject(edge_brush);
    }
}

/// A small label with slightly rounded corners (the gold "Active" and
/// "Locked" labels in the block list): an icon and a text, starting at
/// `left` and centred on `middle`, on the background `back`. Returns where
/// it ends.
pub fn badge(hdc: HDC, fonts: &win::Fonts, left: i32, middle: i32, back: COLORREF, icon: Icon, text: &[u16], dpi: u32) -> i32 {
    let p = palette();
    let s = |v: i32| win::scale(v, dpi);
    let glyph = [icon as u16];
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    // SAFETY: drawing into the DC we were given with our fonts; the extents
    // are out values.
    unsafe {
        let mut size = SIZE { cx: 0, cy: 0 };
        let mut icon_size = SIZE { cx: 0, cy: 0 };
        let old = SelectObject(hdc, fonts.normal);
        GetTextExtentPoint32W(hdc, text.as_ptr(), text.len() as i32, &mut size);
        SelectObject(hdc, fonts.icons);
        GetTextExtentPoint32W(hdc, glyph.as_ptr(), 1, &mut icon_size);
        let (pad, gap, h) = (s(8), s(4), s(20));
        let top = middle - h / 2;
        let rc = RECT { left, top, right: left + pad + icon_size.cx + gap + size.cx + pad, bottom: top + h };
        rounded_box(hdc, rc, back, p.badge, p.badge_line, s(4), false);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, p.badge_text);
        let mut r = RECT { left: rc.left + pad, right: rc.left + pad + icon_size.cx, ..rc };
        DrawTextW(hdc, glyph.as_ptr(), 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
        SelectObject(hdc, fonts.normal);
        let mut r = RECT { left: r.right + gap, right: rc.right - pad + s(2), ..rc };
        DrawTextW(hdc, text.as_ptr(), text.len() as i32, &mut r, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | rtl);
        SelectObject(hdc, old);
        rc.right
    }
}

/// A control's text, read into `buf` (no allocation: this runs for every
/// button drawn).
fn label(hwnd: HWND, buf: &mut [u16; 256]) -> &[u16] {
    // SAFETY: the buffer's length is passed.
    let n = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    &buf[..n.max(0) as usize]
}

/// Whether `text` fits in `width` in the DC's font (only then is the
/// slower "…" shortening asked for).
fn text_fits(hdc: HDC, text: &[u16], width: i32) -> bool {
    let mut size = SIZE { cx: 0, cy: 0 };
    // SAFETY: a valid text and out value.
    unsafe { GetTextExtentPoint32W(hdc, text.as_ptr(), text.len() as i32, &mut size) };
    size.cx <= width
}

/// The width a button needs for its text (and icon), in pixels at 96 DPI.
pub fn button_width(text: &str, icon: bool) -> i32 {
    crate::panel::text_width(text) * 96 / crate::panel::dpi().max(1) as i32 + 32 + if icon { ICON_ROOM } else { 0 }
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
    let p = palette();
    let fg = if disabled { p.grey } else { p.text };
    let border = if disabled { p.line } else { p.grey };
    let fill = if disabled { p.line } else { p.accent };
    let back = p.page;
    let on_mark = p.on_accent;
    let field = p.field;
    let key = (1 + kind as u32 * 8 + u32::from(on) * 4 + u32::from(hot) * 2 + u32::from(disabled), fill, border, field);
    cached_shape(hdc, glyph, back, key, |dc, k| {
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
                    shape(dc, field, border, line, |dc| unsafe { RoundRect(dc, 0, 0, gw, gh, r, r) });
                }
            }
            Kind::Radio => {
                if on {
                    shape(dc, fill, fill, line, |dc| unsafe { Ellipse(dc, 0, 0, gw, gh) });
                    let d = gw * 2 / 5;
                    let at = (gw - d) / 2;
                    shape(dc, on_mark, on_mark, 1, |dc| unsafe { Ellipse(dc, at, at, at + d, at + d) });
                } else {
                    shape(dc, field, border, line, |dc| unsafe { Ellipse(dc, 0, 0, gw, gh) });
                }
            }
        }
    });
    // The label.
    let mut buf = [0u16; 256];
    let wide = label(hwnd, &mut buf);
    let mut r = RECT { left: glyph.right + s(GAP), ..rc };
    let font = win::send(hwnd, WM_GETFONT, 0, 0) as HFONT;
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    // SAFETY: drawing into the control's DC.
    unsafe {
        let old = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, fg);
        let fits = text_fits(hdc, wide, r.right - r.left);
        let ellipsis = if fits { 0 } else { DT_END_ELLIPSIS };
        DrawTextW(hdc, wide.as_ptr(), wide.len() as i32, &mut r, DT_SINGLELINE | DT_VCENTER | ellipsis | DT_NOPREFIX | rtl);
        if state & CDIS_FOCUS != 0 && state & CDIS_SHOWKEYBOARDCUES != 0 {
            let mut measured = r;
            DrawTextW(
                hdc,
                wide.as_ptr(),
                wide.len() as i32,
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

thread_local! {
    /// Shapes already drawn, by size and look: drawing one smoothly takes a
    /// little work, copying it again almost none.
    static SHAPES: RefCell<HashMap<(i32, i32, u32, (u32, u32, u32, u32)), HBITMAP>> = RefCell::new(HashMap::new());
}

/// Most shapes kept; beyond that the oldest set is dropped (sizes change
/// when windows are resized).
const SHAPES_KEPT: usize = 400;

/// Draws a shape into `target` of `hdc` on the background `back`: the first
/// time with `paint` (smoothly, see `smooth`), later as a copy. `key` says
/// what the shape looks like apart from its size.
fn cached_shape(hdc: HDC, target: RECT, back: COLORREF, key: (u32, u32, u32, u32), paint: impl FnOnce(HDC, i32)) {
    let (w, h) = (target.right - target.left, target.bottom - target.top);
    if w <= 0 || h <= 0 {
        return;
    }
    let bitmap = cached_bitmap(hdc, target, back, key, paint);
    // SAFETY: a memory DC made and freed here; the bitmap stays in SHAPES.
    unsafe {
        let mem = CreateCompatibleDC(hdc);
        let old = SelectObject(mem, bitmap);
        BitBlt(hdc, target.left, target.top, w, h, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old);
        DeleteDC(mem);
    }
}

/// The kept picture of a shape the size of `target` (drawn the first time).
fn cached_bitmap(hdc: HDC, target: RECT, back: COLORREF, key: (u32, u32, u32, u32), paint: impl FnOnce(HDC, i32)) -> HBITMAP {
    let (w, h) = (target.right - target.left, target.bottom - target.top);
    kept_bitmap(hdc, w, h, back, key, |mem| smooth(mem, RECT { left: 0, top: 0, right: w, bottom: h }, back, paint))
}

/// The kept picture of size `w`×`h` for `key`, made with `build` (drawing
/// into a memory DC at its top left) the first time.
fn kept_bitmap(hdc: HDC, w: i32, h: i32, back: COLORREF, key: (u32, u32, u32, u32), build: impl FnOnce(HDC)) -> HBITMAP {
    let full = (w, h, back, key);
    if let Some(bitmap) = SHAPES.with(|s| s.borrow().get(&full).copied()) {
        return bitmap;
    }
    // SAFETY: a memory DC made and freed here; the bitmap is kept in SHAPES.
    unsafe {
        let mem = CreateCompatibleDC(hdc);
        let bitmap = CreateCompatibleBitmap(hdc, w, h);
        let old = SelectObject(mem, bitmap);
        build(mem);
        SelectObject(mem, old);
        DeleteDC(mem);
        SHAPES.with(|s| {
            let mut s = s.borrow_mut();
            if s.len() >= SHAPES_KEPT {
                for (_, b) in s.drain() {
                    DeleteObject(b);
                }
            }
            s.insert(full, bitmap);
        });
        bitmap
    }
}

fn forget_shapes() {
    SHAPES.with(|s| {
        for (_, b) in s.borrow_mut().drain() {
            // SAFETY: bitmaps we made and no DC holds.
            unsafe { DeleteObject(b) };
        }
    });
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

/// Kinds of control Iqal draws parts of, remembered with each one when it
/// is made (so no message has to ask a control what it is).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum Part {
    Tabs = 1,
    Status,
    Picker,
    Arrows,
    Dropdown,
    /// The list a drop-down list opens.
    DropList,
}

thread_local! {
    /// Drop-down lists, which Iqal draws (while closed).
    static DROPDOWNS: RefCell<std::collections::HashSet<isize>> = RefCell::new(std::collections::HashSet::new());
}

/// A control told its window something (WM_COMMAND `code`) that changes
/// how Iqal draws it: a text box got or lost the keyboard, or a drop-down
/// list opened, closed or shows another choice.
pub fn notified(parent: HWND, child: HWND, code: u32) {
    const EN_SETFOCUS: u32 = 0x100;
    const EN_KILLFOCUS: u32 = 0x200;
    const CBN_SELCHANGE: u32 = 1;
    const CBN_DROPDOWN: u32 = 7;
    const CBN_CLOSEUP: u32 = 8;
    match code {
        EN_SETFOCUS | EN_KILLFOCUS if outlined_edit(child) => {
            let frame = outline_rect(parent, child);
            // With the thicker line below.
            let ring = RECT { bottom: frame.bottom + 1, ..frame };
            // SAFETY: our window; only the outline is redrawn.
            unsafe { InvalidateRect(parent, &ring, 1) };
        }
        CBN_SELCHANGE | CBN_DROPDOWN | CBN_CLOSEUP if DROPDOWNS.with(|d| d.borrow().contains(&(child as isize))) => {
            // SAFETY: our control.
            unsafe { InvalidateRect(child, std::ptr::null(), 0) };
        }
        _ => {}
    }
}

/// Gives each tab of a row of tabs an icon (before the tabs are added; each
/// is added with picture 0). Windows makes room on a tab for a picture from
/// the tabs' picture list: it is given a blank one as wide as an icon and some
/// more space between tabs (one pixel tall: only its width counts), and the
/// icon is drawn in that room (see `paint_tabs`).
pub fn tab_icons(hwnd: HWND, icons: &'static [Icon]) {
    thread_local! {
        /// The blank picture list, made once (per DPI).
        static BLANK: Cell<(u32, HIMAGELIST)> = const { Cell::new((0, 0)) };
    }
    TAB_ICONS.with(|t| t.borrow_mut().insert(hwnd as isize, icons));
    let dpi = win::dpi(hwnd);
    let (known, mut list) = BLANK.with(Cell::get);
    if known != dpi {
        let side = win::scale(ICON_ROOM + 8, dpi);
        // SAFETY: a picture list with one blank picture, kept for the
        // program's life (one more only if the scaling changes).
        unsafe {
            list = ImageList_Create(side, 1, ILC_COLOR32, 1, 0);
            let blank = CreateBitmap(side, 1, 1, 32, std::ptr::null());
            ImageList_Add(list, blank, std::ptr::null_mut());
            DeleteObject(blank);
        }
        BLANK.with(|b| b.set((dpi, list)));
    }
    win::send(hwnd, TCM_SETIMAGELIST, 0, list);
}

fn subclass(hwnd: HWND, part: Part) {
    if part == Part::Dropdown {
        DROPDOWNS.with(|d| d.borrow_mut().insert(hwnd as isize));
        // Its list gets an outline in Iqal's colour (not Windows' blue).
        // SAFETY: plain data for GetComboBoxInfo to fill.
        let mut info: COMBOBOXINFO = unsafe { std::mem::zeroed() };
        info.cbSize = size_of::<COMBOBOXINFO>() as u32;
        // SAFETY: our control; a sized COMBOBOXINFO.
        if unsafe { GetComboBoxInfo(hwnd, &mut info) } != 0 && !info.hwndList.is_null() {
            subclass(info.hwndList, Part::DropList);
        }
    }
    // SAFETY: our control; the procedure stays valid for the program's life.
    // Subclassing again with the same id only replaces the data.
    unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), 1, part as usize) };
}

unsafe extern "system" fn subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _: usize, data: usize) -> LRESULT {
    let part = match data {
        1 => Part::Tabs,
        2 => Part::Status,
        3 => Part::Picker,
        4 => Part::Arrows,
        5 => Part::Dropdown,
        _ => Part::DropList,
    };
    match (part, msg) {
        (Part::DropList, WM_NCPAINT) => {
            // Windows' edge first (and any scroll bar), then the outline over it.
            // SAFETY: default handling, then drawing into our list's whole
            // window, released again.
            unsafe {
                let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(hwnd, &mut r);
                let hdc = GetWindowDC(hwnd);
                FrameRect(hdc, &RECT { left: 0, top: 0, right: r.right - r.left, bottom: r.bottom - r.top }, brush(LINE));
                ReleaseDC(hwnd, hdc);
                return result;
            }
        }
        (Part::Dropdown, WM_ERASEBKGND) => return 1,
        (Part::Dropdown, WM_PAINT) => {
            // SAFETY: a PAINTSTRUCT for BeginPaint to fill.
            let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
            // SAFETY: our control, inside WM_PAINT.
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            paint_dropdown(hwnd, hdc);
            // SAFETY: matches BeginPaint.
            unsafe { EndPaint(hwnd, &ps) };
            return 0;
        }
        (Part::Dropdown, WM_MOUSEMOVE) if HOT_DROPDOWN.with(Cell::get) != hwnd as isize => {
            // Lighter under the mouse, until it leaves.
            let before = HOT_DROPDOWN.with(|h| h.replace(hwnd as isize));
            let mut track = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            // SAFETY: our controls; a filled-in TRACKMOUSEEVENT.
            unsafe {
                if before != 0 {
                    InvalidateRect(before as HWND, std::ptr::null(), 0);
                }
                TrackMouseEvent(&mut track);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        (Part::Dropdown, WM_MOUSELEAVE) if HOT_DROPDOWN.with(Cell::get) == hwnd as isize => {
            HOT_DROPDOWN.with(|h| h.set(0));
            // SAFETY: our control.
            unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
        }
        (Part::Dropdown, WM_SETFOCUS | WM_KILLFOCUS | WM_ENABLE) => {
            // SAFETY: default handling, then drawn again (outline colour).
            unsafe {
                let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                InvalidateRect(hwnd, std::ptr::null(), 0);
                return result;
            }
        }
        (Part::Picker | Part::Arrows, WM_ERASEBKGND) if is_dark() => return 1,
        (Part::Picker | Part::Arrows, WM_PAINT) if is_dark() => {
            if part == Part::Picker {
                // The picker makes its arrows when first sized: catch them.
                // SAFETY: a live control; the callback only subclasses its children.
                unsafe { EnumChildWindows(hwnd, Some(each_control), 0) };
            }
            paint_darkened(hwnd);
            return 0;
        }
        (Part::Tabs | Part::Status, WM_ERASEBKGND) => return 1,
        (Part::Tabs | Part::Status, WM_PAINT) => {
            // SAFETY: a PAINTSTRUCT for BeginPaint to fill.
            let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
            // SAFETY: our control, inside WM_PAINT.
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            if part == Part::Tabs {
                paint_tabs(hwnd, hdc);
            } else {
                paint_status(hwnd, hdc);
            }
            // SAFETY: matches BeginPaint.
            unsafe { EndPaint(hwnd, &ps) };
            return 0;
        }
        (_, WM_NCDESTROY) => forget(hwnd),
        _ => {}
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

/// A closed drop-down list, like a plain button: rounded, the choice and a
/// small arrow, lighter under the mouse, outlined in the accent while it
/// has the keyboard.
fn paint_dropdown(hwnd: HWND, hdc: HDC) {
    let rc = win::client_rect(hwnd);
    let dpi = win::dpi(hwnd);
    let s = |v: i32| win::scale(v, dpi);
    let p = palette();
    // SAFETY: plain questions about our control.
    let (enabled, focused) = unsafe { (IsWindowEnabled(hwnd) != 0, GetFocus() == hwnd) };
    let dropped = win::send(hwnd, CB_GETDROPPEDSTATE, 0, 0) != 0;
    let hot = HOT_DROPDOWN.with(Cell::get) == hwnd as isize;
    let fill = match () {
        _ if !enabled => p.page,
        _ if dropped => p.button_down,
        _ if hot => p.button_hot,
        _ => p.button,
    };
    let edge = if focused && enabled { p.accent } else { p.line };
    rounded_box(hdc, rc, p.page, fill, edge, s(RADIUS), false);
    // The choice, read without allocating.
    let mut buf = [0u16; 256];
    let index = win::send(hwnd, CB_GETCURSEL, 0, 0);
    let fits = index >= 0 && (0..buf.len() as isize).contains(&win::send(hwnd, CB_GETLBTEXTLEN, index as usize, 0));
    let n = if fits { win::send(hwnd, CB_GETLBTEXT, index as usize, buf.as_mut_ptr() as isize).max(0) as usize } else { 0 };
    let text = &buf[..n];
    let font = win::send(hwnd, WM_GETFONT, 0, 0) as HFONT;
    let (pad, arrow) = (s(10), s(16));
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    // SAFETY: drawing into the control's DC with its font and ours.
    unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, if enabled { p.text } else { p.grey });
        let old = SelectObject(hdc, font);
        let mut r = RECT { left: rc.left + pad, right: rc.right - pad - arrow - s(4), ..rc };
        let ellipsis = if text_fits(hdc, text, r.right - r.left) { 0 } else { DT_END_ELLIPSIS };
        DrawTextW(hdc, text.as_ptr(), text.len() as i32, &mut r, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | ellipsis | rtl);
        if let Some(fonts) = crate::panel::fonts() {
            SelectObject(hdc, fonts.icons);
            SetTextColor(hdc, p.grey);
            let glyph = [Icon::ChevronDown as u16];
            let mut r = RECT { left: rc.right - pad - arrow, right: rc.right - pad, ..rc };
            DrawTextW(hdc, glyph.as_ptr(), 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_NOPREFIX);
        }
        SelectObject(hdc, old);
    }
}

/// Makes a drop-down list (made owner-drawn, see `win::child`) as tall as
/// Windows makes one with this font: Iqal draws its choices in its own
/// colours, but its size stays the same.
pub fn size_dropdown(hwnd: HWND, font: HFONT) {
    // SAFETY: the control's DC, released again; the metrics are an out value.
    let height = unsafe {
        let hdc = GetDC(hwnd);
        let old = SelectObject(hdc, font);
        let mut tm: TEXTMETRICW = std::mem::zeroed();
        GetTextMetricsW(hdc, &mut tm);
        SelectObject(hdc, old);
        ReleaseDC(hwnd, hdc);
        tm.tmHeight
    };
    // The closed box (-1) and the choices, as Windows sizes them.
    win::send(hwnd, CB_SETITEMHEIGHT, usize::MAX, (height + 2) as isize);
    win::send(hwnd, CB_SETITEMHEIGHT, 0, height as isize);
}

/// A choice in an open drop-down list: the one under the mouse in the
/// chosen-row colour.
pub fn draw_dropdown_item(item: &DRAWITEMSTRUCT) {
    const ODT_COMBOBOX: u32 = 3;
    const ODS_COMBOBOXEDIT: u32 = 0x1000;
    // The closed box is drawn in `paint_dropdown`.
    if item.CtlType != ODT_COMBOBOX || item.itemState & ODS_COMBOBOXEDIT != 0 {
        return;
    }
    let p = palette();
    let selected = item.itemState & 1 /* ODS_SELECTED */ != 0;
    let combo = item.hwndItem;
    let mut buf = [0u16; 256];
    let index = item.itemID as usize;
    let fits = item.itemID != u32::MAX && (0..buf.len() as isize).contains(&win::send(combo, CB_GETLBTEXTLEN, index, 0));
    let n = if fits { win::send(combo, CB_GETLBTEXT, index, buf.as_mut_ptr() as isize).max(0) as usize } else { 0 };
    let font = win::send(combo, WM_GETFONT, 0, 0) as HFONT;
    let pad = win::scale(10, win::dpi(combo));
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    let mut r = RECT { left: item.rcItem.left + pad, right: item.rcItem.right - pad, ..item.rcItem };
    // SAFETY: drawing into the list's DC during WM_DRAWITEM; kept brushes.
    unsafe {
        FillRect(item.hDC, &item.rcItem, brush(if selected { COLOR_HIGHLIGHT } else { LIST }));
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, if selected { p.chosen_text } else { p.text });
        let old = SelectObject(item.hDC, font);
        DrawTextW(item.hDC, buf.as_ptr(), n as i32, &mut r, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS | rtl);
        SelectObject(item.hDC, old);
    }
}

/// What a message is about, for its round icon.
#[derive(Clone, Copy)]
pub enum Sign {
    Info,
    Warning,
    Error,
}

/// A message's round icon in `rc`: a circle (accent, amber or red) with an
/// "i" or "!" in it.
pub fn sign(hdc: HDC, rc: RECT, sign: Sign, mark_font: HFONT) {
    let p = palette();
    let (fill, mark) = match sign {
        Sign::Info => (p.accent, 'i'),
        Sign::Warning => (p.warn, '!'),
        Sign::Error => (p.error, '!'),
    };
    let d = (rc.right - rc.left).min(rc.bottom - rc.top);
    let (x, y) = (rc.left + (rc.right - rc.left - d) / 2, rc.top + (rc.bottom - rc.top - d) / 2);
    let circle = RECT { left: x, top: y, right: x + d, bottom: y + d };
    cached_shape(hdc, circle, p.page, (20, fill, 0, 0), |dc, k| {
        // SAFETY: drawing into the memory DC `cached_shape` gives.
        shape(dc, fill, fill, 1, |dc| unsafe { Ellipse(dc, 0, 0, d * k, d * k) });
    });
    let text = [mark as u16];
    let mut r = circle;
    // SAFETY: drawing into the DC we were given.
    unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, if matches!(sign, Sign::Info) { p.on_accent } else { p.on_sign });
        let old = SelectObject(hdc, mark_font);
        DrawTextW(hdc, text.as_ptr(), 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_NOPREFIX);
        SelectObject(hdc, old);
    }
}

/// Tabs as Windows 11 draws them: an icon and text, the chosen one bold
/// with an accent line under it.
fn paint_tabs(hwnd: HWND, hdc: HDC) {
    let client = win::client_rect(hwnd);
    let dpi = win::dpi(hwnd);
    let font = win::send(hwnd, WM_GETFONT, 0, 0) as HFONT;
    let fonts = crate::panel::fonts();
    let chosen = win::send(hwnd, TCM_GETCURSEL, 0, 0);
    let count = win::send(hwnd, TCM_GETITEMCOUNT, 0, 0);
    let icons = TAB_ICONS.with(|t| t.borrow().get(&(hwnd as isize)).copied()).unwrap_or(&[]);
    let gap = win::scale(6, dpi);
    // SAFETY: drawing into the tab control's DC; texts and rectangles are
    // valid for each message.
    unsafe {
        FillRect(hdc, &client, brush(COLOR_WINDOW));
        // A thin line under the row of tabs.
        let mut last = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        win::send(hwnd, TCM_GETITEMRECT, 0, (&raw mut last) as isize);
        let base = RECT { left: client.left, top: last.bottom - 1, right: client.right, bottom: last.bottom };
        let line_brush = CreateSolidBrush(palette().line);
        FillRect(hdc, &base, line_brush);
        DeleteObject(line_brush);
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
            // The chosen tab's accent line, as long as its icon and text.
            let underline = |left: i32, right: i32| {
                if selected {
                    let line = RECT { left, top: r.bottom - win::scale(3, dpi), right, bottom: r.bottom };
                    let accent_brush = CreateSolidBrush(accent());
                    FillRect(hdc, &line, accent_brush);
                    DeleteObject(accent_brush);
                }
            };
            let use_font = match (&fonts, selected) {
                (Some(f), true) => f.bold,
                _ => font,
            };
            let old = SelectObject(hdc, use_font);
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, if selected { accent() } else { palette().grey });
            let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
            match (icons.get(i as usize), &fonts) {
                (Some(&icon), Some(f)) => {
                    // The icon and the text, centred together.
                    let glyph = [icon as u16];
                    let mut size = SIZE { cx: 0, cy: 0 };
                    let mut icon_size = SIZE { cx: 0, cy: 0 };
                    GetTextExtentPoint32W(hdc, buf.as_ptr(), len as i32, &mut size);
                    SelectObject(hdc, f.icons);
                    GetTextExtentPoint32W(hdc, glyph.as_ptr(), 1, &mut icon_size);
                    let x = r.left + ((r.right - r.left) - (icon_size.cx + gap + size.cx)).max(0) / 2;
                    underline(x, x + icon_size.cx + gap + size.cx);
                    let mut at = RECT { left: x, right: x + icon_size.cx, ..r };
                    DrawTextW(hdc, glyph.as_ptr(), 1, &mut at, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
                    SelectObject(hdc, use_font);
                    let mut at = RECT { left: at.right + gap, right: r.right.max(at.right + gap + size.cx), ..r };
                    DrawTextW(hdc, buf.as_ptr(), len as i32, &mut at, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | rtl);
                }
                _ => {
                    let mut size = SIZE { cx: 0, cy: 0 };
                    GetTextExtentPoint32W(hdc, buf.as_ptr(), len as i32, &mut size);
                    let x = r.left + ((r.right - r.left) - size.cx).max(0) / 2;
                    underline(x, x + size.cx);
                    DrawTextW(hdc, buf.as_ptr(), len as i32, &mut r, DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_NOPREFIX | rtl);
                }
            }
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
        // A thin line on top, and quiet text.
        let top = RECT { bottom: client.top + 1, ..client };
        let line_brush = CreateSolidBrush(palette().line);
        FillRect(hdc, &top, line_brush);
        DeleteObject(line_brush);
        let old = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, palette().grey);
        let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
        DrawTextW(hdc, wide.as_ptr(), wide.len() as i32 - 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX | rtl);
        SelectObject(hdc, old);
    }
}
