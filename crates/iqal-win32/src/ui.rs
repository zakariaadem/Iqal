//! The main window: the block list on one side, the selected block on the
//! other, and the service's status at the bottom. Every few seconds it asks
//! the service whether anything changed and redraws if so.
//!
//! Window procedures can be re-entered (a control may draw itself while we
//! send it a message), so the app state is only ever borrowed briefly and
//! never while sending messages.

use std::cell::RefCell;

use iqal_app::service::Service;
use iqal_core::SetId;
use iqal_i18n::{t, ta};
use iqal_ipc::{SetInfo, Status};
use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DT_END_ELLIPSIS,
    DT_NOPREFIX, DT_RTLREADING, DT_SINGLELINE, DT_WORDBREAK, DrawTextW, EndPaint, FillRect, GetSysColor,
    GetSysColorBrush, HDC, HFONT, InvalidateRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ICC_BAR_CLASSES, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, ODS_SELECTED,
    SBARS_SIZEGRIP,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, IDC_ARROW,
    IsDialogMessageW, KillTimer, LB_ADDSTRING, LB_GETCURSEL, LB_RESETCONTENT, LB_SETCURSEL, LB_SETITEMHEIGHT,
    LBN_SELCHANGE, LBS_NOINTEGRALHEIGHT, LBS_NOTIFY, LBS_OWNERDRAWFIXED, LoadCursorW, MINMAXINFO, MSG,
    PostQuitMessage, RegisterClassExW, SW_RESTORE, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER,
    SetForegroundWindow, SetTimer, SetWindowPos, ShowWindow, TranslateMessage, WM_COMMAND, WM_DESTROY,
    WM_DPICHANGED, WM_DRAWITEM, WM_GETMINMAXINFO, WM_PAINT, WM_SIZE, WM_TIMER, WNDCLASSEXW, WS_CHILD,
    WS_CLIPCHILDREN, WS_EX_CLIENTEDGE, WS_EX_LAYOUTRTL, WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

use crate::win::{self, Fonts, scale};

const CLASS: &str = "IqalMainWindow";
const ID_LIST: u16 = 100;
const ID_STATUS: u16 = 101;
const TIMER_REFRESH: usize = 1;
const REFRESH_MS: u32 = 3000;
/// Sizes in pixels at 96 DPI.
const LIST_WIDTH: i32 = 280;
const MARGIN: i32 = 12;
const ROW_HEIGHT: i32 = 52;
const START_SIZE: (i32, i32) = (980, 640);
const MIN_SIZE: (i32, i32) = (640, 420);

struct App {
    hwnd: HWND,
    list: HWND,
    status: HWND,
    dpi: u32,
    fonts: Fonts,
    service: Service,
    sets: Vec<SetInfo>,
    /// The service's change counter when `sets` was loaded.
    generation: Option<u64>,
    selected: Option<SetId>,
    /// Shown instead of the blocks (title, then a newline and details).
    error: Option<String>,
    info: Option<Box<Status>>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` with the app state, if the window exists. Never send window
/// messages from inside `f`.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|app| app.borrow_mut().as_mut().map(f))
}

pub fn run() {
    let class = win::wide(CLASS);
    // Iqal is already open: bring that window to the front instead.
    // SAFETY: NUL-terminated class name.
    let existing = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
    if !existing.is_null() {
        // SAFETY: a window of ours in another process.
        unsafe {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
        }
        return;
    }
    let controls = INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_STANDARD_CLASSES | ICC_BAR_CLASSES,
    };
    // SAFETY: plain structures; the class name outlives registration.
    let hwnd = unsafe {
        InitCommonControlsEx(&controls);
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
        // Arabic and other right-to-left languages mirror the whole layout.
        let ex_style = if iqal_i18n::is_rtl() { WS_EX_LAYOUTRTL } else { 0 };
        let title = win::wide(&t("app-name"));
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            win::instance(),
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return;
    }
    build(hwnd);
    refresh(true);
    // SAFETY: our window; standard message loop. IsDialogMessage gives
    // keyboard navigation (Tab between controls).
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

/// Creates the child controls and sizes the window for its monitor.
fn build(hwnd: HWND) {
    let dpi = win::dpi(hwnd);
    let fonts = Fonts::new(dpi);
    let list = win::child(
        hwnd,
        "LISTBOX",
        "",
        WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_TABSTOP | (LBS_OWNERDRAWFIXED | LBS_NOTIFY | LBS_NOINTEGRALHEIGHT) as u32,
        WS_EX_CLIENTEDGE,
        ID_LIST,
    );
    let status = win::child(hwnd, "msctls_statusbar32", "", WS_CHILD | WS_VISIBLE | SBARS_SIZEGRIP, 0, ID_STATUS);
    fonts.apply(list);
    fonts.apply(status);
    win::send(list, LB_SETITEMHEIGHT, 0, scale(ROW_HEIGHT, dpi) as isize);
    APP.with(|app| {
        *app.borrow_mut() = Some(App {
            hwnd,
            list,
            status,
            dpi,
            fonts,
            service: Service::new(),
            sets: Vec::new(),
            generation: None,
            selected: None,
            error: None,
            info: None,
        })
    });
    // SAFETY: our window.
    unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            scale(START_SIZE.0, dpi),
            scale(START_SIZE.1, dpi),
            SWP_NOZORDER | SWP_NOACTIVATE | 0x0002, // SWP_NOMOVE
        );
        SetTimer(hwnd, TIMER_REFRESH, REFRESH_MS, None);
        SetFocus(list);
    }
}

/// Asks the service what changed; reloads everything after a change, or
/// when `full`.
fn refresh(full: bool) {
    let loaded = with_app(|app| {
        let states = app.service.states();
        let changed = match &states {
            Ok((generation, _)) => full || app.generation != Some(*generation) || app.error.is_some(),
            Err(_) => app.error.is_none() || full,
        };
        if !changed {
            return false;
        }
        match states.and_then(|(generation, _)| Ok((generation, app.service.sets()?, app.service.status()?))) {
            Ok((generation, sets, info)) => {
                app.generation = Some(generation);
                if app.selected.is_none_or(|id| !sets.iter().any(|s| s.set.id == id)) {
                    app.selected = sets.first().map(|s| s.set.id);
                }
                app.sets = sets;
                app.info = Some(info);
                app.error = None;
            }
            Err(e) => {
                app.generation = None;
                app.sets.clear();
                app.info = None;
                app.error = Some(e);
            }
        }
        true
    });
    if loaded == Some(true) {
        redraw();
    }
}

/// Refills the list and status bar from the app state, and repaints.
fn redraw() {
    let Some((hwnd, list, status, count, selected, status_text)) = with_app(|app| {
        let selected = app.sets.iter().position(|s| Some(s.set.id) == app.selected);
        (app.hwnd, app.list, app.status, app.sets.len(), selected, status_text(app))
    }) else {
        return;
    };
    win::send(list, LB_RESETCONTENT, 0, 0);
    for i in 0..count {
        win::send(list, LB_ADDSTRING, 0, i as isize);
    }
    win::send(list, LB_SETCURSEL, selected.map_or(usize::MAX, |i| i), 0);
    win::set_text(status, &status_text);
    // SAFETY: our window.
    unsafe { InvalidateRect(hwnd, std::ptr::null(), 1) };
}

fn status_text(app: &App) -> String {
    let Some(info) = &app.info else { return String::new() };
    let mut parts = vec![
        ta("status-service", &[("version", info.version.clone().into())]),
        ta("status-time", &[("source", info.clock.clone().into())]),
    ];
    if !info.admin {
        parts.push(t("status-not-admin"));
    }
    parts.join("   ·   ")
}

fn state_text(info: &SetInfo) -> String {
    let state = if !info.set.enabled {
        t("state-off")
    } else if info.active {
        t("state-active")
    } else {
        t("state-scheduled")
    };
    if info.locked { format!("{state} · {}", t("state-locked")) } else { state }
}

/// Positions the controls for the window's size.
fn layout(hwnd: HWND) {
    let Some((list, status, dpi)) = with_app(|app| (app.list, app.status, app.dpi)) else { return };
    // The status bar sizes itself when told the window changed size.
    win::send(status, WM_SIZE, 0, 0);
    let client = win::client_rect(hwnd);
    let status_height = {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: r is a valid out pointer.
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(status, &mut r) };
        r.bottom - r.top
    };
    let m = scale(MARGIN, dpi);
    // SAFETY: our child window.
    unsafe {
        SetWindowPos(
            list,
            std::ptr::null_mut(),
            m,
            m,
            scale(LIST_WIDTH, dpi),
            (client.bottom - status_height - 2 * m).max(0),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

/// Where the selected block is shown: right of the list.
fn content_rect(hwnd: HWND, dpi: u32) -> RECT {
    let client = win::client_rect(hwnd);
    let m = scale(MARGIN, dpi);
    RECT {
        left: m + scale(LIST_WIDTH, dpi) + 2 * m,
        top: m * 2,
        right: client.right - 2 * m,
        bottom: client.bottom - m * 4,
    }
}

fn color(index: i32) -> COLORREF {
    // SAFETY: no pointers involved.
    unsafe { GetSysColor(index) }
}

fn draw_text(hdc: HDC, font: HFONT, text: &str, rect: &mut RECT, format: u32) -> i32 {
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    let text: Vec<u16> = text.encode_utf16().collect();
    // SAFETY: hdc is being painted; text and rect are valid.
    unsafe {
        SelectObject(hdc, font);
        DrawTextW(hdc, text.as_ptr(), text.len() as i32, rect, format | DT_NOPREFIX | rtl)
    }
}

/// Paints the selected block (or an empty state or error) right of the list.
fn paint(hwnd: HWND) {
    // SAFETY: a PAINTSTRUCT for BeginPaint to fill.
    let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    // SAFETY: our window, inside WM_PAINT.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    with_app(|app| {
        let (title, lines): (String, Vec<String>) = match (&app.error, app.sets.iter().find(|s| Some(s.set.id) == app.selected)) {
            (Some(e), _) => {
                let (title, detail) = e.split_once('\n').unwrap_or((e, ""));
                (title.to_string(), vec![detail.to_string()])
            }
            (None, Some(info)) => (info.set.name.clone(), vec![state_text(info), summary(info)]),
            (None, None) => (t("empty-title"), vec![t("empty-detail")]),
        };
        let mut r = content_rect(hwnd, app.dpi);
        // SAFETY: painting into hdc.
        unsafe {
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, color(COLOR_WINDOWTEXT as i32));
        }
        let height = draw_text(hdc, app.fonts.title, &title, &mut r.clone(), DT_WORDBREAK);
        r.top += height + scale(8, app.dpi);
        // SAFETY: as above.
        unsafe { SetTextColor(hdc, color(COLOR_GRAYTEXT as i32)) };
        for line in lines.iter().filter(|l| !l.is_empty()) {
            let height = draw_text(hdc, app.fonts.normal, line, &mut r.clone(), DT_WORDBREAK);
            r.top += height + scale(4, app.dpi);
        }
    });
    // SAFETY: matches BeginPaint.
    unsafe { EndPaint(hwnd, &ps) };
}

/// One row of the block list: the name, and below it the state and how
/// many rules the block has.
fn draw_row(item: &DRAWITEMSTRUCT) {
    with_app(|app| {
        let Some(info) = app.sets.get(item.itemID as usize) else { return };
        let selected = item.itemState & ODS_SELECTED != 0;
        let (back, fore, dim) = if selected {
            (COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_HIGHLIGHTTEXT)
        } else {
            (COLOR_WINDOW, COLOR_WINDOWTEXT, COLOR_GRAYTEXT)
        };
        let hdc = item.hDC;
        let pad = scale(8, app.dpi);
        let mut r = item.rcItem;
        // SAFETY: drawing into the list's DC during WM_DRAWITEM.
        unsafe {
            FillRect(hdc, &r, GetSysColorBrush(back as i32));
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, color(fore as i32));
        }
        r.left += pad;
        r.right -= pad;
        r.top += pad / 2;
        let half = (r.bottom - r.top) / 2;
        let mut top = RECT { bottom: r.top + half, ..r };
        draw_text(hdc, app.fonts.bold, &info.set.name, &mut top, DT_SINGLELINE | DT_END_ELLIPSIS);
        let mut bottom = RECT { top: r.top + half, ..r };
        // SAFETY: as above.
        unsafe { SetTextColor(hdc, color(dim as i32)) };
        let line = format!("{} · {}", state_text(info), summary(info));
        draw_text(hdc, app.fonts.normal, &line, &mut bottom, DT_SINGLELINE | DT_END_ELLIPSIS);
    });
}

/// How many rules are in use (an allow list does not use the blocked ones).
fn summary(info: &SetInfo) -> String {
    let s = &info.set;
    let used = |mode, block: usize, allow: usize| match mode {
        iqal_core::ListMode::Block => block + allow,
        iqal_core::ListMode::AllowOnly => allow,
    } as i64;
    ta(
        "block-summary",
        &[
            ("web", used(s.web.mode, s.web.block.len(), s.web.allow.len()).into()),
            ("apps", used(s.apps.mode, s.apps.block.len(), s.apps.allow.len()).into()),
        ],
    )
}

fn select_from_list() {
    let Some(list) = with_app(|app| app.list) else { return };
    let index = win::send(list, LB_GETCURSEL, 0, 0);
    let Some(hwnd) = with_app(|app| {
        app.selected = usize::try_from(index).ok().and_then(|i| app.sets.get(i)).map(|s| s.set.id);
        app.hwnd
    }) else {
        return;
    };
    // SAFETY: our window.
    unsafe { InvalidateRect(hwnd, std::ptr::null(), 1) };
}

/// The window moved to a monitor with another scaling: new fonts and
/// sizes, and Windows' suggested window rectangle.
fn dpi_changed(hwnd: HWND, dpi: u32, suggested: &RECT) {
    let fonts = Fonts::new(dpi);
    let Some((list, status)) = with_app(|app| (app.list, app.status)) else { return };
    fonts.apply(list);
    fonts.apply(status);
    win::send(list, LB_SETITEMHEIGHT, 0, scale(ROW_HEIGHT, dpi) as isize);
    // The old fonts are dropped only now that no control uses them.
    with_app(|app| {
        app.dpi = dpi;
        app.fonts = fonts;
    });
    // SAFETY: our window; Windows' suggested rectangle.
    unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            suggested.left,
            suggested.top,
            suggested.right - suggested.left,
            suggested.bottom - suggested.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            layout(hwnd);
            0
        }
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_DRAWITEM => {
            // SAFETY: for WM_DRAWITEM, lparam points to a DRAWITEMSTRUCT.
            let item = unsafe { &*(lparam as *const DRAWITEMSTRUCT) };
            if item.CtlID == ID_LIST as u32 {
                draw_row(item);
            }
            1
        }
        WM_COMMAND => {
            if win::loword(wparam) == ID_LIST && win::hiword(wparam) == LBN_SELCHANGE as u16 {
                select_from_list();
            }
            0
        }
        WM_TIMER if wparam == TIMER_REFRESH => {
            refresh(false);
            0
        }
        WM_GETMINMAXINFO => {
            let dpi = win::dpi(hwnd);
            // SAFETY: for WM_GETMINMAXINFO, lparam points to a MINMAXINFO.
            let info = unsafe { &mut *(lparam as *mut MINMAXINFO) };
            info.ptMinTrackSize.x = scale(MIN_SIZE.0, dpi);
            info.ptMinTrackSize.y = scale(MIN_SIZE.1, dpi);
            0
        }
        WM_DPICHANGED => {
            // SAFETY: for WM_DPICHANGED, lparam points to the suggested RECT.
            let suggested = unsafe { &*(lparam as *const RECT) };
            dpi_changed(hwnd, win::hiword(wparam) as u32, suggested);
            0
        }
        WM_DESTROY => {
            // SAFETY: our window and timer.
            unsafe {
                KillTimer(hwnd, TIMER_REFRESH);
                PostQuitMessage(0);
            }
            0
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
