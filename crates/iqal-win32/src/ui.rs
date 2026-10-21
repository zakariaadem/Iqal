//! The main window: the block list on one side, the editor for the selected
//! block on the other, and the service's status at the bottom. Every few
//! seconds it asks the service whether anything changed and redraws if so.
//!
//! Edits go to a draft of the selected block and reach the service only
//! when the person presses Save (or Ctrl+S); Discard throws the draft away.
//!
//! Window procedures can be re-entered (a control may draw itself while we
//! send it a message), so the app state is only ever borrowed briefly and
//! never while sending messages. The editor is rebuilt through a posted
//! message ([`rebuild_later`]), never from inside one of its own controls'
//! events.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use iqal_app::draft::{Draft, ListKey};
use iqal_app::service::Service;
use iqal_core::{BlockSet, SetId};
use iqal_i18n::{t, ta};
use iqal_ipc::{ListSummary, Request, Response, SetInfo, Status};
use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOWTEXT, DT_END_ELLIPSIS,
    DT_NOPREFIX, DT_RTLREADING, DT_SINGLELINE, DT_WORDBREAK, DrawTextW, EndPaint, FillRect,
    HDC, HFONT, InvalidateRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ICC_BAR_CLASSES, ICC_DATE_CLASSES, ICC_STANDARD_CLASSES, ICC_TAB_CLASSES, INITCOMMONCONTROLSEX,
    InitCommonControlsEx, ODS_SELECTED, SBARS_SIZEGRIP,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, SetFocus, VK_CONTROL};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW, GetMessageW,
    GetWindowRect, IDC_ARROW, IsDialogMessageW, KillTimer, LB_ADDSTRING, LB_GETCURSEL, LB_RESETCONTENT, LB_SETCURSEL,
    LB_SETITEMHEIGHT, LBN_SELCHANGE, LBS_NOINTEGRALHEIGHT, LBS_NOTIFY, LBS_OWNERDRAWFIXED, LoadCursorW, MINMAXINFO,
    MSG, PostMessageW, PostQuitMessage, RegisterClassExW, SW_RESTORE, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, HWND_TOP, SetForegroundWindow, SetTimer, SetWindowPos, ShowWindow, TranslateMessage, WM_APP, WM_CLOSE,
    WM_COMMAND, WM_CTLCOLORBTN, WM_NOTIFY, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_DRAWITEM, WM_ERASEBKGND, WM_SETTINGCHANGE, WM_GETMINMAXINFO, WM_KEYDOWN, WM_PAINT, WM_SIZE, WM_TIMER,
    BN_CLICKED, WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_EX_CLIENTEDGE, WS_EX_LAYOUTRTL, WS_OVERLAPPEDWINDOW, WS_TABSTOP,
    WS_VISIBLE, WS_VSCROLL,
};

use crate::dialog;
use crate::menu;
use crate::editor::{self, Editor};
use crate::panel;
use crate::theme;
use crate::timing;
use crate::win::{self, Fonts, scale};

const CLASS: &str = "IqalMainWindow";
const ID_LIST: u16 = 100;
const ID_STATUS: u16 = 101;
const ID_NEW: u16 = 102;
const ID_MENU: u16 = 103;
const TIMER_REFRESH: usize = 1;
const REFRESH_MS: u32 = 3000;
/// Rebuild the editor (posted, so it never happens inside a control's event).
const WM_REBUILD: u32 = WM_APP + 1;
/// Sizes in pixels at 96 DPI.
const LIST_WIDTH: i32 = 280;
const MARGIN: i32 = 12;
const ROW_HEIGHT: i32 = 52;
const START_SIZE: (i32, i32) = (1060, 700);
const MIN_SIZE: (i32, i32) = (760, 480);

/// Button ids of the "save the changes?" question.
const ANSWER_SAVE: i32 = 100;
const ANSWER_DISCARD: i32 = 101;
const ANSWER_CANCEL: i32 = 102;

struct App {
    hwnd: HWND,
    list: HWND,
    status: HWND,
    /// "New block" and "Menu", above the list.
    new_button: HWND,
    menu_button: HWND,
    dpi: u32,
    fonts: Rc<Fonts>,
    service: Service,
    sets: Vec<SetInfo>,
    /// The service's change counter when `sets` was loaded.
    generation: Option<u64>,
    selected: Option<SetId>,
    /// Shown instead of the blocks (title, then a newline and details).
    error: Option<String>,
    info: Option<Box<Status>>,
    /// Unsaved changes to the selected block.
    draft: Option<Draft>,
    /// The editor tab shown, kept across rebuilds and blocks.
    tab: usize,
    editor: Option<Editor>,
    /// Rules of pre-made lists, by (list, number of rules).
    list_rules: HashMap<(String, usize), Rc<Vec<String>>>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// A rebuild of the editor is posted and has not happened yet.
    static REBUILD_PENDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the app state, if the window exists. Never send window
/// messages from inside `f`.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|app| app.borrow_mut().as_mut().map(f))
}

pub fn main_window() -> HWND {
    with_app(|app| app.hwnd).unwrap_or(std::ptr::null_mut())
}

pub fn run() {
    let class = win::wide(CLASS);
    // Iqal is already open: bring that window to the front instead. (Not
    // when talking to another service, for testing.)
    // SAFETY: NUL-terminated class name.
    let existing = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
    if !existing.is_null() && std::env::var_os(iqal_ipc::SOCKET_ENV).is_none() {
        // SAFETY: a window of ours in another process.
        unsafe {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
        }
        return;
    }
    let controls = INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_STANDARD_CLASSES | ICC_BAR_CLASSES | ICC_TAB_CLASSES | ICC_DATE_CLASSES,
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
            // Iqal's icon, resource 1 (see build.rs).
            hIcon: windows_sys::Win32::UI::WindowsAndMessaging::LoadIconW(win::instance(), 1 as _),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            // Painted in WM_ERASEBKGND, in the app's light or dark colour.
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
            // Windows makes the small icon from the large one.
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
    theme::window(hwnd);
    build(hwnd);
    refresh(true);
    // SAFETY: our window; standard message loop. IsDialogMessage gives
    // keyboard navigation (Tab between controls).
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            // Ctrl+S saves, wherever the focus is.
            if msg.message == WM_KEYDOWN && msg.wParam == b'S' as usize && GetKeyState(VK_CONTROL as i32) < 0 {
                save();
                continue;
            }
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
    let fonts = Rc::new(Fonts::new(dpi));
    panel::set_style(fonts.clone(), dpi);
    let list = win::child(
        hwnd,
        "LISTBOX",
        "",
        WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_TABSTOP | (LBS_OWNERDRAWFIXED | LBS_NOTIFY | LBS_NOINTEGRALHEIGHT) as u32,
        WS_EX_CLIENTEDGE,
        ID_LIST,
    );
    let status = win::child(hwnd, "msctls_statusbar32", "", WS_CHILD | WS_VISIBLE | SBARS_SIZEGRIP, 0, ID_STATUS);
    let new_button = win::child(hwnd, "BUTTON", &t("action-new-block"), WS_CHILD | WS_VISIBLE | WS_TABSTOP, 0, ID_NEW);
    let menu_button = win::child(hwnd, "BUTTON", &t("menu"), WS_CHILD | WS_VISIBLE | WS_TABSTOP, 0, ID_MENU);
    theme::style_button(new_button, Some(theme::Icon::Add), theme::Look::Main);
    theme::style_button(menu_button, Some(theme::Icon::Menu), theme::Look::Plain);
    for control in [list, status, new_button, menu_button] {
        fonts.apply(control);
    }
    win::send(list, LB_SETITEMHEIGHT, 0, scale(ROW_HEIGHT, dpi) as isize);
    APP.with(|app| {
        *app.borrow_mut() = Some(App {
            hwnd,
            list,
            status,
            new_button,
            menu_button,
            dpi,
            fonts,
            service: Service::new(),
            sets: Vec::new(),
            generation: None,
            selected: None,
            error: None,
            info: None,
            draft: None,
            tab: 0,
            editor: None,
            list_rules: HashMap::new(),
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
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOMOVE,
        );
        SetTimer(hwnd, TIMER_REFRESH, REFRESH_MS, None);
        SetFocus(list);
    }
}

/// Asks the service what changed; reloads everything after a change, or
/// when `full`. The editor is rebuilt for the new data, unless the person
/// is in the middle of editing the block.
pub fn refresh(full: bool) {
    let rebuild = with_app(|app| {
        let states = app.service.states();
        let changed = match &states {
            Ok((generation, _)) => full || app.generation != Some(*generation) || app.error.is_some(),
            Err(_) => app.error.is_none() || full,
        };
        if !changed {
            return None;
        }
        let before = app.selected;
        match states.and_then(|(generation, _)| Ok((generation, app.service.sets()?, app.service.status()?))) {
            Ok((generation, sets, info)) => {
                app.generation = Some(generation);
                if app.selected.is_none_or(|id| !sets.iter().any(|s| s.set.id == id)) {
                    app.selected = sets.first().map(|s| s.set.id);
                    app.draft = None;
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
                app.draft = None;
            }
        }
        // Keep the editor while there are unsaved changes in it.
        Some(before != app.selected || !app.draft.as_ref().is_some_and(Draft::dirty))
    })
    .flatten();
    if let Some(rebuild) = rebuild {
        redraw_list();
        if rebuild {
            rebuild_later();
        }
    }
}

/// Refills the list and status bar from the app state.
fn redraw_list() {
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
        ta("status-time", &[("source", iqal_app::describe::time_source(&info.clock).into())]),
    ];
    if !info.admin {
        parts.push(t("status-not-admin"));
    }
    parts.join("   ·   ")
}

pub fn state_text(info: &SetInfo) -> String {
    let state = if !info.set.enabled {
        t("state-off")
    } else if info.active {
        t("state-active")
    } else {
        t("state-scheduled")
    };
    if info.locked { format!("{state} · {}", t("state-locked")) } else { state }
}

/// Asks for the editor to be rebuilt once the current event is over.
pub fn rebuild_later() {
    // Several requests before it happens make one rebuild.
    if REBUILD_PENDING.with(|p| p.replace(true)) {
        return;
    }
    let hwnd = main_window();
    // SAFETY: our window; a message without pointers.
    unsafe { PostMessageW(hwnd, WM_REBUILD, 0, 0) };
}

/// Replaces the editor with one for the selected block (as drafted).
fn rebuild_editor() {
    let timer = timing::start();
    let Some((hwnd, old, area, show)) = with_app(|app| {
        let show = app.error.is_none() && app.sets.iter().any(|s| Some(s.set.id) == app.selected);
        (app.hwnd, app.editor.take(), editor_rect(app), show)
    }) else {
        return;
    };
    // The new editor is built out of sight and then shown over the old one
    // in one step, so the window never shows a half-built editor or an
    // empty gap between the two.
    let new = if show { editor::build(hwnd, area) } else { None };
    // SAFETY: our panels and everything in them.
    unsafe {
        if let Some(new) = &new {
            SetWindowPos(new.root.hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
        if let Some(old) = old {
            DestroyWindow(old.root.hwnd);
        }
    }
    if timer.is_some() {
        // Measured until painted: the first time, since the program started.
        if let Some(new) = &new {
            // SAFETY: our panel.
            unsafe { windows_sys::Win32::Graphics::Gdi::UpdateWindow(new.root.hwnd) };
        }
        static FIRST: std::sync::Once = std::sync::Once::new();
        let mut first = false;
        FIRST.call_once(|| first = true);
        if first {
            timing::record("open", timing::program_start());
        } else {
            timing::record("tab", timer);
        }
    }
    let placeholder = new.is_none();
    with_app(|app| app.editor = new);
    if placeholder {
        // The placeholder is painted when there is no editor.
        // SAFETY: our window.
        unsafe { InvalidateRect(hwnd, std::ptr::null(), 1) };
    }
}

/// The selected block as the editor shows it: the saved block with the
/// draft's changes.
pub struct View {
    pub info: SetInfo,
    pub set: BlockSet,
    pub dirty: bool,
    pub tab: usize,
}

pub fn view() -> Option<View> {
    with_app(|app| {
        let info = app.sets.iter().find(|s| Some(s.set.id) == app.selected)?.clone();
        let (set, dirty) = match &app.draft {
            Some(d) => (d.set.clone(), d.dirty()),
            None => (info.set.clone(), false),
        };
        Some(View { info, set, dirty, tab: app.tab })
    })
    .flatten()
}

/// The rules of a list as edited (draft) or saved, one per line.
pub fn list_text(key: ListKey) -> String {
    with_app(|app| match &app.draft {
        Some(d) => d.texts[key.index()].clone(),
        None => app.sets.iter().find(|s| Some(s.set.id) == app.selected).map(|s| key.text_of(&s.set)).unwrap_or_default(),
    })
    .unwrap_or_default()
}

fn with_draft<R>(f: impl FnOnce(&mut Draft) -> R) -> Option<R> {
    with_app(|app| {
        if app.draft.is_none() {
            let base = app.sets.iter().find(|s| Some(s.set.id) == app.selected)?.set.clone();
            app.draft = Some(Draft::new(base));
        }
        app.draft.as_mut().map(f)
    })
    .flatten()
}

/// Changes the draft, without rebuilding the editor: for controls that
/// already show the new value.
pub fn edit(change: impl FnOnce(&mut BlockSet)) {
    with_draft(|d| change(&mut d.set));
    update_dirty();
}

/// Changes the draft and rebuilds the editor: for changes that show or hide
/// parts of it.
pub fn modify(change: impl FnOnce(&mut BlockSet)) {
    with_draft(|d| change(&mut d.set));
    update_dirty();
    rebuild_later();
}

/// Records the text of a rule list as edited.
pub fn set_list_text(key: ListKey, text: String) {
    with_draft(|d| d.texts[key.index()] = text);
    update_dirty();
}

pub fn set_tab(tab: usize) {
    with_app(|app| app.tab = tab);
    rebuild_later();
}

fn is_dirty() -> bool {
    with_app(|app| app.draft.as_ref().is_some_and(Draft::dirty)).unwrap_or(false)
}

/// Shows or hides the save bar and the "unsaved" mark in the list.
fn update_dirty() {
    let dirty = is_dirty();
    let Some((list, editor)) = with_app(|app| (app.list, app.editor.as_ref().map(Editor::save_bar))) else { return };
    if let Some((panel, cells)) = editor {
        for cell in cells {
            panel.set_hidden(cell, !dirty);
        }
    }
    // SAFETY: our list.
    unsafe { InvalidateRect(list, std::ptr::null(), 1) };
}

/// Saves the draft. Returns whether everything is saved.
pub fn save() -> bool {
    if !is_dirty() {
        return true;
    }
    let Some(finished) = with_app(|app| app.draft.as_ref().map(Draft::finish)).flatten() else { return true };
    let owner = main_window();
    match finished {
        Ok(set) => match with_app(|app| app.service.save(set)) {
            Some(Ok(())) => {
                with_app(|app| app.draft = None);
                refresh(true);
                rebuild_later();
                true
            }
            Some(Err(e)) => {
                dialog::error(owner, &e);
                false
            }
            None => false,
        },
        Err(problems) => {
            dialog::message(owner, dialog::Icon::Warning, &t("save-has-errors"), &problems);
            false
        }
    }
}

/// Changes the block and saves at once, with any other unsaved edits: used
/// for locks, which take effect when confirmed. If saving fails the change
/// is undone and the other edits are kept. `owner` shows any message.
pub fn commit(owner: HWND, change: impl FnOnce(&mut BlockSet)) -> bool {
    let Some(base) = with_app(|app| {
        let info = app.sets.iter().find(|s| Some(s.set.id) == app.selected)?;
        Some(match &app.draft {
            Some(d) => d.finish(),
            None => Ok(info.set.clone()),
        })
    })
    .flatten() else {
        return false;
    };
    let mut set = match base {
        Ok(set) => set,
        Err(problems) => {
            dialog::message(owner, dialog::Icon::Warning, &t("save-has-errors"), &problems);
            return false;
        }
    };
    change(&mut set);
    match with_app(|app| app.service.save(set)) {
        Some(Ok(())) => {
            with_app(|app| app.draft = None);
            refresh(true);
            rebuild_later();
            true
        }
        Some(Err(e)) => {
            dialog::error(owner, &e);
            false
        }
        None => false,
    }
}

/// The lock of block `id` was opened (which turns the block off): takes it
/// out of unsaved changes too.
pub fn forget_lock(id: SetId) {
    with_app(|app| {
        if let Some(d) = app.draft.as_mut().filter(|d| d.set.id == id) {
            d.set.lock = None;
            d.set.pending_unlock_at = None;
            d.set.enabled = false;
        }
    });
}

pub fn has_unsaved_changes() -> bool {
    is_dirty()
}

pub fn discard() {
    with_app(|app| app.draft = None);
    update_dirty();
    rebuild_later();
}

/// Runs `then` once there are no unsaved changes: at once, or after the
/// person saved or discarded them. Cancelling does nothing.
fn after_unsaved(then: impl FnOnce()) {
    if !is_dirty() {
        return then();
    }
    let name = view().map(|v| v.set.name).unwrap_or_default();
    let answer = dialog::ask(
        main_window(),
        Some(dialog::Icon::Warning),
        &ta("confirm-unsaved", &[("name", name.into())]),
        &t("confirm-unsaved-detail"),
        &[(ANSWER_CANCEL, t("action-cancel")), (ANSWER_DISCARD, t("action-discard")), (ANSWER_SAVE, t("action-save"))],
    );
    match answer {
        Some(ANSWER_DISCARD) => {
            with_app(|app| app.draft = None);
            then();
        }
        Some(ANSWER_SAVE) if save() => then(),
        _ => {}
    }
}

/// Sends a request to the service.
pub fn call(request: Request) -> Result<Response, String> {
    with_app(|app| app.service.call(request)).unwrap_or_else(|| Err(t("error-unexpected")))
}

/// The websites of a pre-made list (asked once per list version).
pub fn list_rules(list: &ListSummary) -> Rc<Vec<String>> {
    let key = (list.id.clone(), list.rules);
    if let Some(rules) = with_app(|app| app.list_rules.get(&key).cloned()).flatten() {
        return rules;
    }
    let rules = match call(Request::ListRules { id: list.id.clone() }) {
        Ok(Response::ListRules { rules }) => Rc::new(rules),
        _ => Rc::new(Vec::new()),
    };
    with_app(|app| app.list_rules.insert(key, rules.clone()));
    rules
}

/// Shows `id` in the editor (the person dealt with unsaved changes first).
pub fn select(id: SetId) {
    with_app(|app| {
        app.selected = Some(id);
        app.draft = None;
    });
    redraw_list();
    rebuild_later();
}

/// All blocks, as last loaded.
pub fn blocks() -> Vec<SetInfo> {
    with_app(|app| app.sets.clone()).unwrap_or_default()
}

/// What the service last reported about itself.
pub fn status() -> Option<Box<Status>> {
    with_app(|app| app.info.clone()).flatten()
}

/// Deletes the selected block (after asking).
pub fn delete_selected() {
    let Some(view) = view() else { return };
    let owner = main_window();
    let answer = dialog::ask(
        owner,
        Some(dialog::Icon::Warning),
        &ta("confirm-delete", &[("name", view.set.name.clone().into())]),
        &t("confirm-delete-detail"),
        &[(ANSWER_CANCEL, t("action-cancel")), (ANSWER_DISCARD, t("action-delete"))],
    );
    if answer != Some(ANSWER_DISCARD) {
        return;
    }
    match call(Request::RemoveSet { id: view.set.id }) {
        Ok(_) => {
            with_app(|app| app.draft = None);
            refresh(true);
        }
        Err(e) => dialog::error(owner, &e),
    }
}

/// Positions the controls for the window's size.
fn layout(hwnd: HWND) {
    let Some((list, status, new_button, menu_button, dpi, editor)) = with_app(|app| {
        (app.list, app.status, app.new_button, app.menu_button, app.dpi, app.editor.as_ref().map(|e| e.root))
    }) else {
        return;
    };
    // The status bar sizes itself when told the window changed size.
    win::send(status, WM_SIZE, 0, 0);
    let client = win::client_rect(hwnd);
    let status_height = status_height(status);
    let m = scale(MARGIN, dpi);
    let row = scale(panel::ROW, dpi);
    let width = scale(LIST_WIDTH, dpi);
    // "New block" and "Menu" share the row above the list.
    let menu_width = scale(theme::button_width(&t("menu"), true), dpi);
    // SAFETY: our child windows.
    unsafe {
        SetWindowPos(new_button, std::ptr::null_mut(), m, m, width - menu_width - m / 2, row, SWP_NOZORDER | SWP_NOACTIVATE);
        SetWindowPos(menu_button, std::ptr::null_mut(), m + width - menu_width, m, menu_width, row, SWP_NOZORDER | SWP_NOACTIVATE);
        SetWindowPos(
            list,
            std::ptr::null_mut(),
            m,
            m + row + m / 2,
            width,
            (client.bottom - status_height - 2 * m - row - m / 2).max(0),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        if let Some(root) = editor {
            let r = with_app(editor_rect).unwrap_or(client);
            SetWindowPos(root.hwnd, std::ptr::null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
        }
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

fn status_height(status: HWND) -> i32 {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: r is a valid out pointer.
    unsafe { GetWindowRect(status, &mut r) };
    r.bottom - r.top
}

/// Where the editor goes: right of the list, above the status bar.
fn editor_rect(app: &mut App) -> RECT {
    let client = win::client_rect(app.hwnd);
    let m = scale(MARGIN, app.dpi);
    RECT {
        left: m + scale(LIST_WIDTH, app.dpi),
        top: 0,
        right: client.right,
        bottom: client.bottom - status_height(app.status),
    }
}

fn color(index: i32) -> COLORREF {
    theme::color(index)
}

fn draw_text(hdc: HDC, font: HFONT, text: &str, rect: &mut RECT, format: u32) -> i32 {
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    // Ends in a zero: DrawTextW reads through the pointer even for an empty text.
    let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    // SAFETY: hdc is being painted; text and rect are valid.
    unsafe {
        SelectObject(hdc, font);
        DrawTextW(hdc, text.as_ptr(), text.len() as i32 - 1, rect, format | DT_NOPREFIX | rtl)
    }
}

/// Paints the placeholder (no blocks, or an error) where the editor goes.
fn paint(hwnd: HWND) {
    // SAFETY: a PAINTSTRUCT for BeginPaint to fill.
    let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    // SAFETY: our window, inside WM_PAINT.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    with_app(|app| {
        if app.editor.is_some() {
            return;
        }
        let (title, detail) = match &app.error {
            Some(e) => {
                let (title, detail) = e.split_once('\n').unwrap_or((e, ""));
                (title.to_string(), detail.to_string())
            }
            None => (t("empty-title"), t("empty-detail")),
        };
        let mut r = editor_rect(app);
        let m = scale(MARGIN * 2, app.dpi);
        r.left += m;
        r.top += m;
        r.right -= m;
        // SAFETY: painting into hdc.
        unsafe {
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, color(COLOR_WINDOWTEXT as i32));
        }
        let height = draw_text(hdc, app.fonts.title, &title, &mut r.clone(), DT_WORDBREAK);
        r.top += height + scale(8, app.dpi);
        // SAFETY: as above.
        unsafe { SetTextColor(hdc, color(COLOR_GRAYTEXT as i32)) };
        draw_text(hdc, app.fonts.normal, &detail, &mut r, DT_WORDBREAK);
    });
    // SAFETY: matches BeginPaint.
    unsafe { EndPaint(hwnd, &ps) };
}

/// One row of the block list: the name (with a mark while it has unsaved
/// changes), and below it the state and how many rules the block has.
fn draw_row(item: &DRAWITEMSTRUCT) {
    with_app(|app| {
        let Some(info) = app.sets.get(item.itemID as usize) else { return };
        let unsaved = Some(info.set.id) == app.selected && app.draft.as_ref().is_some_and(Draft::dirty);
        let selected = item.itemState & ODS_SELECTED != 0;
        let (back, fore, dim) = if selected {
            (COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_HIGHLIGHTTEXT)
        } else {
            (theme::LIST, COLOR_WINDOWTEXT, COLOR_GRAYTEXT)
        };
        let hdc = item.hDC;
        let pad = scale(8, app.dpi);
        let mut r = item.rcItem;
        // SAFETY: drawing into the list's DC during WM_DRAWITEM.
        unsafe {
            FillRect(hdc, &r, theme::brush(back as i32));
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, color(fore as i32));
        }
        r.left += pad;
        r.right -= pad;
        r.top += pad / 2;
        let half = (r.bottom - r.top) / 2;
        let name = if unsaved { format!("{} •", info.set.name) } else { info.set.name.clone() };
        let mut top = RECT { bottom: r.top + half, ..r };
        draw_text(hdc, app.fonts.bold, &name, &mut top, DT_SINGLELINE | DT_END_ELLIPSIS);
        let mut bottom = RECT { top: r.top + half, ..r };
        // SAFETY: as above.
        unsafe { SetTextColor(hdc, color(dim as i32)) };
        let line = format!("{} · {}", state_text(info), summary(info));
        draw_text(hdc, app.fonts.normal, &line, &mut bottom, DT_SINGLELINE | DT_END_ELLIPSIS);
    });
}

/// How many rules are in use (an allow list does not use the blocked ones).
pub fn summary(info: &SetInfo) -> String {
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

/// Another block was chosen in the list: switch to it, after dealing with
/// unsaved changes (or go back to the current one if cancelled).
fn select_from_list() {
    let Some(list) = with_app(|app| app.list) else { return };
    let index = win::send(list, LB_GETCURSEL, 0, 0);
    let Some(chosen) = with_app(|app| usize::try_from(index).ok().and_then(|i| app.sets.get(i)).map(|s| s.set.id)).flatten() else {
        return;
    };
    if with_app(|app| app.selected) == Some(Some(chosen)) {
        return;
    }
    let mut switched = false;
    after_unsaved(|| {
        with_app(|app| {
            app.selected = Some(chosen);
            app.draft = None;
        });
        switched = true;
    });
    if switched {
        redraw_list();
        rebuild_later();
    } else {
        // Cancelled: the list shows the block still being edited.
        redraw_list();
    }
}

/// The window moved to a monitor with another scaling: new fonts and
/// sizes, and Windows' suggested window rectangle.
fn dpi_changed(hwnd: HWND, dpi: u32, suggested: &RECT) {
    let fonts = Rc::new(Fonts::new(dpi));
    let Some(controls) = with_app(|app| [app.list, app.status, app.new_button, app.menu_button]) else { return };
    let list = controls[0];
    for control in controls {
        fonts.apply(control);
    }
    win::send(list, LB_SETITEMHEIGHT, 0, scale(ROW_HEIGHT, dpi) as isize);
    panel::set_style(fonts.clone(), dpi);
    // The old fonts are dropped once the editor made with them is rebuilt.
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
    rebuild_later();
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
        WM_ERASEBKGND => theme::erase(hwnd, wparam as HDC),
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            match theme::ctl_color(msg, wparam as HDC, false) {
                Some(brush) => brush,
                // SAFETY: default handling.
                None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            }
        }
        WM_SETTINGCHANGE => {
            // Windows switched between light and dark: follow, unless the
            // person chose one in Iqal's settings.
            // SAFETY: for WM_SETTINGCHANGE, lparam is null or a NUL-terminated string.
            let area = unsafe { (lparam != 0).then(|| win::from_wide_ptr(lparam as *const u16)) }.unwrap_or_default();
            if area == "ImmersiveColorSet" {
                let was = theme::is_dark();
                theme::init();
                if theme::is_dark() != was {
                    theme::apply_everywhere();
                }
            }
            // SAFETY: default handling.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_NOTIFY => {
            // SAFETY: for WM_NOTIFY, lparam points to an NMHDR.
            let header = unsafe { &*(lparam as *const windows_sys::Win32::UI::Controls::NMHDR) };
            if header.code == windows_sys::Win32::UI::Controls::NM_CUSTOMDRAW {
                // SAFETY: for NM_CUSTOMDRAW from a button, lparam points to an NMCUSTOMDRAW.
                let draw = unsafe { &*(lparam as *const windows_sys::Win32::UI::Controls::NMCUSTOMDRAW) };
                if let Some(result) = theme::custom_draw(draw) {
                    return result;
                }
            }
            // SAFETY: default handling.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
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
            match (win::loword(wparam), win::hiword(wparam) as u32) {
                (ID_LIST, LBN_SELCHANGE) => select_from_list(),
                (ID_NEW, BN_CLICKED) => after_unsaved(menu::new_block),
                (ID_MENU, BN_CLICKED) => {
                    if let Some(button) = with_app(|app| app.menu_button) {
                        menu::show(hwnd, button);
                    }
                }
                _ => {}
            }
            0
        }
        WM_REBUILD => {
            REBUILD_PENDING.with(|p| p.set(false));
            rebuild_editor();
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
            // Wide enough for the editor's top row (and the window's frame).
            let editor = MARGIN + LIST_WIDTH + editor::min_width() + 16;
            info.ptMinTrackSize.x = scale(MIN_SIZE.0.max(editor), dpi);
            info.ptMinTrackSize.y = scale(MIN_SIZE.1, dpi);
            0
        }
        WM_DPICHANGED => {
            // SAFETY: for WM_DPICHANGED, lparam points to the suggested RECT.
            let suggested = unsafe { &*(lparam as *const RECT) };
            dpi_changed(hwnd, win::hiword(wparam) as u32, suggested);
            0
        }
        WM_CLOSE => {
            // SAFETY: our window.
            after_unsaved(|| unsafe {
                DestroyWindow(hwnd);
            });
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
