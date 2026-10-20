//! Panels: child windows that hold controls, arrange them in rows (and
//! two-column sections), scroll when their content is taller than they
//! are, and pass control events to closures.
//!
//! Raw Win32 has no layout: a panel keeps a list of [`Item`]s and places
//! every control again whenever its size, scroll position or content
//! changes. Controls send their events (WM_COMMAND, WM_NOTIFY) to their
//! parent window; a panel looks up the closure registered for the sending
//! control. Closures run while the panel's state is not borrowed, so they
//! may change or rebuild the window freely (rebuilding the panel that sent
//! the event must go through `ui::rebuild_later`).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_GRAYTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DT_CALCRECT, DT_NOPREFIX, DT_WORDBREAK, DrawTextW, GetDC, GetSysColor,
    GetSysColorBrush, GetTextExtentPoint32W, HDC, HFONT, ReleaseDC, SelectObject, SetBkColor, SetTextColor,
};
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, NMHDR, SetScrollInfo};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetScrollInfo, GetWindowTextLengthW, GetWindowTextW, IDC_ARROW, LoadCursorW, RegisterClassExW, SB_LINEDOWN,
    SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBTRACK, SB_VERT, SCROLLINFO, SIF_ALL, SIF_PAGE, SIF_POS, SIF_RANGE,
    SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos, ShowWindow, SW_HIDE, SW_SHOW, WM_COMMAND,
    WM_CTLCOLORBTN, WM_CTLCOLORSTATIC, WM_DRAWITEM, WM_MOUSEWHEEL, WM_GETFONT, WM_NCDESTROY, WM_NOTIFY, WM_SIZE, WM_VSCROLL,
    WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_EX_CONTROLPARENT, WS_VISIBLE, WS_VSCROLL,
};

use crate::win::{self, Fonts, scale};

const CLASS: &str = "IqalPanel";
const WHEEL_STEP: i32 = 48;
/// Height of a one-line control (text box, button), at 96 DPI.
pub const ROW: i32 = 30;
/// Height of a drop-down list's window while open, at 96 DPI.
const DROPDOWN_OPEN: i32 = 240;

/// How wide a control in a row is.
#[derive(Clone, Copy)]
pub enum Width {
    /// Fixed, in pixels at 96 DPI.
    Dip(i32),
    /// Shares what is left of the row.
    Flex,
}

/// One entry of a panel's layout. Heights are in pixels at 96 DPI.
pub enum Item {
    /// A control across the whole width.
    Full { hwnd: HWND, height: i32 },
    /// A static text across the whole width, as tall as its wrapped text.
    Text(HWND),
    /// Controls side by side.
    Row { cells: Vec<(HWND, Width)>, height: i32 },
    /// Two columns of equal width, each laid out like a panel.
    Columns { left: Vec<Item>, right: Vec<Item> },
    /// A drop-down list (combo box): its window also covers the list when
    /// open, so it is made taller than the room it takes.
    Dropdown { hwnd: HWND, width: Width },
    /// Empty space.
    Space(i32),
    /// For panels that do not scroll: a control taking the height left over.
    Fill(HWND),
}

type Handler = Rc<dyn Fn(u32)>;
type Drawer = Rc<dyn Fn(&DRAWITEMSTRUCT)>;

struct PanelState {
    items: Vec<Item>,
    scrolls: bool,
    offset: i32,
    /// Controls whose layout space is taken but that are not shown.
    hidden: HashSet<isize>,
    /// Height of the content at the last layout, in device pixels.
    content: i32,
}

thread_local! {
    static PANELS: RefCell<HashMap<isize, PanelState>> = RefCell::new(HashMap::new());
    static HANDLERS: RefCell<HashMap<isize, Handler>> = RefCell::new(HashMap::new());
    static DRAWERS: RefCell<HashMap<isize, Drawer>> = RefCell::new(HashMap::new());
    /// Static texts shown greyed (notes and explanations).
    static NOTES: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
    /// The fonts and DPI of the main window, for new controls.
    static STYLE: RefCell<Option<(Rc<Fonts>, u32)>> = const { RefCell::new(None) };
}

/// Sets the fonts and DPI new controls are made with.
pub fn set_style(fonts: Rc<Fonts>, dpi: u32) {
    STYLE.with(|s| *s.borrow_mut() = Some((fonts, dpi)));
}

pub fn dpi() -> u32 {
    STYLE.with(|s| s.borrow().as_ref().map_or(96, |(_, d)| *d))
}

pub fn fonts() -> Option<Rc<Fonts>> {
    STYLE.with(|s| s.borrow().as_ref().map(|(f, _)| f.clone()))
}

#[derive(Clone, Copy)]
pub struct Panel {
    pub hwnd: HWND,
}

impl Panel {
    /// A new panel inside `parent`. `scrolls`: content taller than the
    /// panel can be scrolled.
    pub fn new(parent: HWND, scrolls: bool) -> Self {
        register_class();
        let class = win::wide(CLASS);
        let style = WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | if scrolls { WS_VSCROLL } else { 0 };
        // SAFETY: registered class; parent is a live window.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_CONTROLPARENT,
                class.as_ptr(),
                std::ptr::null(),
                style,
                0,
                0,
                0,
                0,
                parent,
                std::ptr::null_mut(),
                win::instance(),
                std::ptr::null(),
            )
        };
        PANELS.with(|p| {
            p.borrow_mut().insert(hwnd as isize, PanelState { items: Vec::new(), scrolls, offset: 0, hidden: HashSet::new(), content: 0 })
        });
        Self { hwnd }
    }

    /// A control in this panel, in the window's font.
    pub fn control(&self, class: &str, text: &str, style: u32, ex_style: u32) -> HWND {
        let hwnd = win::child(self.hwnd, class, text, WS_CHILD | WS_VISIBLE | style, ex_style, 0);
        if let Some(fonts) = fonts() {
            fonts.apply(hwnd);
        }
        hwnd
    }

    /// A greyed explanation text.
    pub fn note(&self, text: &str) -> HWND {
        let hwnd = self.control("STATIC", text, 0x2000 /* SS_NOPREFIX */, 0);
        NOTES.with(|n| n.borrow_mut().insert(hwnd as isize));
        hwnd
    }

    /// A text in the bold font (headings).
    pub fn heading(&self, text: &str) -> HWND {
        let hwnd = self.control("STATIC", text, 0x2000, 0);
        if let Some(fonts) = fonts() {
            win::send(hwnd, windows_sys::Win32::UI::WindowsAndMessaging::WM_SETFONT, fonts.bold as usize, 1);
        }
        hwnd
    }

    pub fn set_items(&self, items: Vec<Item>) {
        with_state(self.hwnd, |s| s.items = items);
        self.relayout();
    }

    /// Takes a control out of the layout (or puts it back).
    pub fn set_hidden(&self, hwnd: HWND, hidden: bool) {
        with_state(self.hwnd, |s| {
            if hidden {
                s.hidden.insert(hwnd as isize);
            } else {
                s.hidden.remove(&(hwnd as isize));
            }
        });
        // SAFETY: a live control.
        unsafe { ShowWindow(hwnd, if hidden { SW_HIDE } else { SW_SHOW }) };
        self.relayout();
    }

    pub fn relayout(&self) {
        layout(self.hwnd);
    }

    /// How tall the content is, in device pixels (as last laid out).
    pub fn content_height(&self) -> i32 {
        with_state(self.hwnd, |s| s.content).unwrap_or(0)
    }
}

/// Runs `f` when the control sends an event (its notification code).
pub fn on(control: HWND, f: impl Fn(u32) + 'static) {
    HANDLERS.with(|h| h.borrow_mut().insert(control as isize, Rc::new(f)));
}

/// Draws the items of an owner-drawn control.
pub fn on_draw(control: HWND, f: impl Fn(&DRAWITEMSTRUCT) + 'static) {
    DRAWERS.with(|d| d.borrow_mut().insert(control as isize, Rc::new(f)));
}

/// Width of a text in the normal font, in device pixels.
pub fn text_width(text: &str) -> i32 {
    let Some(fonts) = fonts() else { return 0 };
    let text: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE { cx: 0, cy: 0 };
    // SAFETY: a screen DC, released again; valid text and out pointer.
    unsafe {
        let hdc: HDC = GetDC(std::ptr::null_mut());
        let old = SelectObject(hdc, fonts.normal);
        GetTextExtentPoint32W(hdc, text.as_ptr(), text.len() as i32, &mut size);
        SelectObject(hdc, old);
        ReleaseDC(std::ptr::null_mut(), hdc);
    }
    size.cx
}

/// A button width that fits its text, in pixels at 96 DPI.
pub fn button_width(text: &str) -> Width {
    Width::Dip(text_width(text) * 96 / dpi().max(1) as i32 + 32)
}

fn with_state<R>(panel: HWND, f: impl FnOnce(&mut PanelState) -> R) -> Option<R> {
    PANELS.with(|p| p.borrow_mut().get_mut(&(panel as isize)).map(f))
}

fn register_class() {
    thread_local! { static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
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
            hbrBackground: GetSysColorBrush(COLOR_WINDOW),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

/// Places the panel's controls; updates the scroll bar.
fn layout(panel: HWND) {
    let client = win::client_rect(panel);
    let dpi = dpi();
    // Positions are worked out with the state borrowed, and applied after:
    // moving a child panel makes it lay itself out, which needs the state.
    let Some((offset, scrolls, height, moves)) = with_state(panel, |s| {
        let mut moves = Vec::new();
        let m = scale(16, dpi);
        let width = client.right - 2 * m;
        let fill_bottom = client.bottom - m;
        let height = place(&s.items, &s.hidden, m, m - s.offset, width, fill_bottom, dpi, &mut moves) + m + s.offset;
        // Scrolled past the end after the content shrank: go back.
        let max_offset = (height - client.bottom).max(0);
        if s.scrolls && s.offset > max_offset {
            let shift = s.offset - max_offset;
            s.offset = max_offset;
            for mv in &mut moves {
                mv.2 += shift;
            }
        }
        s.content = height;
        (s.offset, s.scrolls, height, moves)
    }) else {
        return;
    };
    for (hwnd, x, y, w, h) in moves {
        // SAFETY: our child controls.
        unsafe { SetWindowPos(hwnd, std::ptr::null_mut(), x, y, w.max(0), h.max(0), SWP_NOZORDER | SWP_NOACTIVATE) };
    }
    if scrolls {
        let info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
            nMin: 0,
            nMax: height,
            nPage: client.bottom.max(0) as u32 + 1,
            nPos: offset,
            nTrackPos: 0,
        };
        // SAFETY: our panel; a filled SCROLLINFO.
        unsafe { SetScrollInfo(panel, SB_VERT, &info, 1) };
    }
}

type Move = (HWND, i32, i32, i32, i32);

/// Lays out `items` from `top` within `width` starting at `x`; returns the
/// bottom. Positions are collected in `moves`.
#[allow(clippy::too_many_arguments)]
fn place(
    items: &[Item],
    hidden: &HashSet<isize>,
    x: i32,
    top: i32,
    width: i32,
    fill_bottom: i32,
    dpi: u32,
    moves: &mut Vec<Move>,
) -> i32 {
    let gap = scale(8, dpi);
    let mut y = top;
    for (index, item) in items.iter().enumerate() {
        match item {
            Item::Full { hwnd, height } => {
                if hidden.contains(&(*hwnd as isize)) {
                    continue;
                }
                let h = scale(*height, dpi);
                moves.push((*hwnd, x, y, width, h));
                y += h + gap;
            }
            Item::Row { cells, height } => {
                let visible: Vec<&(HWND, Width)> = cells.iter().filter(|(h, _)| !hidden.contains(&(*h as isize))).collect();
                if visible.is_empty() {
                    continue;
                }
                let h = scale(*height, dpi);
                let fixed: i32 = visible.iter().map(|(_, w)| if let Width::Dip(d) = w { scale(*d, dpi) } else { 0 }).sum();
                let flexes = visible.iter().filter(|(_, w)| matches!(w, Width::Flex)).count().max(1) as i32;
                let spacing = gap * (visible.len() as i32 - 1);
                let flex = ((width - fixed - spacing) / flexes).max(0);
                let mut cx = x;
                for (hwnd, w) in visible {
                    let cw = match w {
                        Width::Dip(d) => scale(*d, dpi),
                        Width::Flex => flex,
                    };
                    moves.push((*hwnd, cx, y, cw, h));
                    cx += cw + gap;
                }
                y += h + gap;
            }
            Item::Columns { left, right } => {
                let between = scale(20, dpi);
                let half = (width - between) / 2;
                let l = place(left, hidden, x, y, half, fill_bottom, dpi, moves);
                let r = place(right, hidden, x + half + between, y, half, fill_bottom, dpi, moves);
                y = l.max(r) + gap;
            }
            Item::Text(hwnd) => {
                if hidden.contains(&(*hwnd as isize)) {
                    continue;
                }
                let h = text_height(*hwnd, width);
                moves.push((*hwnd, x, y, width, h));
                y += h + gap;
            }
            Item::Dropdown { hwnd, width: w } => {
                let cw = match w {
                    Width::Dip(d) => scale(*d, dpi),
                    Width::Flex => width,
                };
                moves.push((*hwnd, x, y, cw, scale(DROPDOWN_OPEN, dpi)));
                y += scale(ROW, dpi) + gap;
            }
            Item::Space(d) => y += scale(*d, dpi),
            Item::Fill(hwnd) => {
                // Leaves room for what comes after it (a save bar, say).
                let rest = &items[index + 1..];
                let after = if rest.is_empty() { 0 } else { place(rest, hidden, x, 0, width, 0, dpi, &mut Vec::new()) + gap };
                let h = (fill_bottom - y - after).max(0);
                moves.push((*hwnd, x - scale(16, dpi), y, width + 2 * scale(16, dpi), h));
                y += h + gap;
            }
        }
    }
    y - gap
}

fn scroll(panel: HWND, to: i32) {
    with_state(panel, |s| s.offset = to.max(0));
    layout(panel);
}

fn current_offset(panel: HWND) -> i32 {
    let mut info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_ALL,
        nMin: 0,
        nMax: 0,
        nPage: 0,
        nPos: 0,
        nTrackPos: 0,
    };
    // SAFETY: our panel; a sized SCROLLINFO.
    unsafe { GetScrollInfo(panel, SB_VERT, &mut info) };
    info.nPos
}

/// Forgets the controls of a panel that is being destroyed.
fn forget(panel: HWND) {
    let Some(state) = PANELS.with(|p| p.borrow_mut().remove(&(panel as isize))) else { return };
    let mut controls = Vec::new();
    collect(&state.items, &mut controls);
    HANDLERS.with(|h| {
        let mut h = h.borrow_mut();
        for c in &controls {
            h.remove(c);
        }
    });
    DRAWERS.with(|d| {
        let mut d = d.borrow_mut();
        for c in &controls {
            d.remove(c);
        }
    });
    NOTES.with(|n| {
        let mut n = n.borrow_mut();
        for c in &controls {
            n.remove(c);
        }
    });
}

fn collect(items: &[Item], out: &mut Vec<isize>) {
    for item in items {
        match item {
            Item::Full { hwnd, .. } | Item::Fill(hwnd) | Item::Text(hwnd) | Item::Dropdown { hwnd, .. } => out.push(*hwnd as isize),
            Item::Row { cells, .. } => out.extend(cells.iter().map(|(h, _)| *h as isize)),
            Item::Columns { left, right } => {
                collect(left, out);
                collect(right, out);
            }
            Item::Space(_) => {}
        }
    }
}

fn handler(control: HWND) -> Option<Handler> {
    HANDLERS.with(|h| h.borrow().get(&(control as isize)).cloned())
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND if lparam != 0 => {
            if let Some(f) = handler(lparam as HWND) {
                f(win::hiword(wparam) as u32);
            }
            0
        }
        WM_NOTIFY => {
            // SAFETY: for WM_NOTIFY, lparam points to an NMHDR.
            let header = unsafe { &*(lparam as *const NMHDR) };
            if let Some(f) = handler(header.hwndFrom) {
                f(header.code);
            }
            0
        }
        WM_DRAWITEM => {
            // SAFETY: for WM_DRAWITEM, lparam points to a DRAWITEMSTRUCT.
            let item = unsafe { &*(lparam as *const DRAWITEMSTRUCT) };
            if let Some(f) = DRAWERS.with(|d| d.borrow().get(&(item.hwndItem as isize)).cloned()) {
                f(item);
            }
            1
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            // Texts, check boxes and radio buttons on the window colour;
            // notes greyed.
            let grey = NOTES.with(|n| n.borrow().contains(&lparam));
            let colour = if grey { COLOR_GRAYTEXT } else { COLOR_WINDOWTEXT };
            // SAFETY: wparam is the control's DC during this message.
            unsafe {
                SetTextColor(wparam as HDC, GetSysColor(colour));
                SetBkColor(wparam as HDC, GetSysColor(COLOR_WINDOW));
                GetSysColorBrush(COLOR_WINDOW) as LRESULT
            }
        }
        WM_SIZE => {
            layout(hwnd);
            0
        }
        WM_VSCROLL => {
            let page = win::client_rect(hwnd).bottom;
            let step = scale(WHEEL_STEP, dpi());
            let now = current_offset(hwnd);
            let to = match win::loword(wparam) as i32 {
                SB_LINEUP => now - step,
                SB_LINEDOWN => now + step,
                SB_PAGEUP => now - page,
                SB_PAGEDOWN => now + page,
                SB_THUMBTRACK | 4 /* SB_THUMBPOSITION */ => win::hiword(wparam) as i32,
                _ => now,
            };
            scroll(hwnd, to);
            0
        }
        WM_MOUSEWHEEL => {
            let scrolls = with_state(hwnd, |s| s.scrolls).unwrap_or(false);
            if !scrolls {
                // SAFETY: default handling passes it on to the parent.
                return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
            }
            let delta = win::hiword(wparam) as i16 as i32;
            let step = scale(WHEEL_STEP, dpi());
            scroll(hwnd, current_offset(hwnd) - delta * step / 120);
            0
        }
        WM_NCDESTROY => {
            forget(hwnd);
            // SAFETY: default handling.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// The height a static text needs at this width, wrapped.
fn text_height(hwnd: HWND, width: i32) -> i32 {
    // SAFETY: a live control; its font is selected into a screen DC only
    // for measuring; the DC is released again.
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        let mut text = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32);
        let font = win::send(hwnd, WM_GETFONT, 0, 0) as HFONT;
        let hdc = GetDC(std::ptr::null_mut());
        let old = SelectObject(hdc, font);
        let mut r = RECT { left: 0, top: 0, right: width.max(1), bottom: 0 };
        DrawTextW(hdc, text.as_ptr(), n, &mut r, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
        SelectObject(hdc, old);
        ReleaseDC(std::ptr::null_mut(), hdc);
        r.bottom - r.top
    }
}
