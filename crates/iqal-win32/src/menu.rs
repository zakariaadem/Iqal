//! The menu and the dialogs reached from it, and creating a new block.

use iqal_app::describe::fmt_time;
use iqal_app::settings::GuiSettings;
use iqal_core::{BlockSet, SetId};
use iqal_i18n::{LANGUAGES, t, ta};
use iqal_ipc::{BlockEvent, EventKind, Request, Response};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_GRAYTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DT_END_ELLIPSIS, DT_NOPREFIX, DT_RTLREADING, DT_SINGLELINE,
    DrawTextW, FillRect, GetSysColor, GetSysColorBrush, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;
use windows_sys::Win32::UI::Controls::Dialogs::{GetSaveFileNameW, OFN_EXPLORER, OFN_OVERWRITEPROMPT, OPENFILENAMEW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, BN_CLICKED, BS_DEFPUSHBUTTON, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBN_SELCHANGE,
    CBS_DROPDOWNLIST, CreatePopupMenu, DestroyMenu, ES_AUTOHSCROLL, GetWindowRect, LB_SETCOUNT, LB_SETITEMHEIGHT,
    LBS_NODATA, LBS_NOINTEGRALHEIGHT, LBS_OWNERDRAWFIXED, MF_STRING, TPM_LAYOUTRTL, TPM_RETURNCMD, TrackPopupMenu,
    WS_EX_CLIENTEDGE, WS_TABSTOP, WS_VSCROLL,
};

use crate::editor::open_file;
use crate::modal::{self, Closer};
use crate::panel::{self, Item, Panel, Width, button_width};
use crate::win::{self, scale};
use crate::{dialog, ui};

const EM_SETCUEBANNER: u32 = 0x1501;
const SS_LABEL: u32 = 0x2000 | 0x0200;

/// Shows the menu under `button` and runs the chosen entry.
pub fn show(owner: HWND, button: HWND) {
    let entries: [(&str, fn()); 6] = [
        ("menu-import", import_blocks),
        ("menu-export-all", export_all),
        ("menu-activity", activity),
        ("menu-stats", crate::stats::open),
        ("menu-settings", settings),
        ("menu-about", about),
    ];
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: a menu we create and destroy; our button and window.
    let chosen = unsafe {
        let menu = CreatePopupMenu();
        for (i, (key, _)) in entries.iter().enumerate() {
            let text = win::wide(&t(key));
            AppendMenuW(menu, MF_STRING, i + 1, text.as_ptr());
        }
        GetWindowRect(button, &mut r);
        let rtl = if iqal_i18n::is_rtl() { TPM_LAYOUTRTL } else { 0 };
        let x = if iqal_i18n::is_rtl() { r.right } else { r.left };
        let chosen = TrackPopupMenu(menu, TPM_RETURNCMD | rtl, x, r.bottom, 0, owner, std::ptr::null());
        DestroyMenu(menu);
        chosen
    };
    if let Some((_, action)) = usize::try_from(chosen).ok().and_then(|c| c.checked_sub(1)).and_then(|i| entries.get(i)) {
        action();
    }
}

/// Asks for a name and creates an empty block.
pub fn new_block() {
    let owner = ui::main_window();
    modal::run(owner, &t("action-new-block"), (420, 200), |page, closer| {
        let label = page.control("STATIC", &t("new-block-name"), SS_LABEL, 0);
        let name = page.control("EDIT", "", WS_TABSTOP | ES_AUTOHSCROLL as u32, WS_EX_CLIENTEDGE);
        let cue = win::wide(&t("new-block-placeholder"));
        win::send(name, EM_SETCUEBANNER, 1, cue.as_ptr() as isize);
        let (spacer, cancel, create) = buttons(page, "action-create");
        page.set_items(vec![
            Item::Full { hwnd: label, height: 22 },
            Item::Full { hwnd: name, height: panel::ROW },
            Item::Space(8),
            Item::Row {
                cells: vec![(spacer, Width::Flex), (cancel, button_width(&t("action-cancel"))), (create, button_width(&t("action-create")))],
                height: panel::ROW,
            },
        ]);
        // SAFETY: our edit box.
        unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(name) };
        close_on(cancel, &closer);
        panel::on(create, move |code| {
            if code != BN_CLICKED {
                return;
            }
            let text = win::text(name).trim().to_string();
            if text.is_empty() {
                return;
            }
            let set = BlockSet::new(text);
            let id = set.id;
            match ui::call(Request::UpsertSet { set: Box::new(set) }) {
                Ok(_) => {
                    closer.close();
                    ui::refresh(true);
                    ui::select(id);
                }
                Err(e) => dialog::error(closer.hwnd(), &e),
            }
        });
    });
}

/// A filler, Cancel, and the main button (the default, for Enter).
fn buttons(page: Panel, main: &str) -> (HWND, HWND, HWND) {
    let spacer = page.control("STATIC", "", 0, 0);
    let cancel = page.control("BUTTON", &t("action-cancel"), WS_TABSTOP, 0);
    let ok = page.control("BUTTON", &t(main), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
    (spacer, cancel, ok)
}

fn close_on(button: HWND, closer: &Closer) {
    let closer = closer.clone();
    panel::on(button, move |code| {
        if code == BN_CLICKED {
            closer.close();
        }
    });
}

fn export_all() {
    export(None);
}

/// Saves blocks (`None`: all of them) to an `.iqal` file.
pub fn export(ids: Option<Vec<SetId>>) {
    let owner = ui::main_window();
    let json = match ui::call(Request::Export { ids }) {
        Ok(Response::Exported { json }) => json,
        Ok(_) => return dialog::error(owner, &t("error-unexpected")),
        Err(e) => return dialog::error(owner, &e),
    };
    let Some(path) = save_file(owner, &t("menu-export-all"), "iqal-blocks.iqal") else { return };
    match std::fs::write(&path, json) {
        Ok(()) => dialog::message(owner, dialog::Icon::Info, &t("export-done"), &path),
        Err(e) => dialog::error(owner, &e.to_string()),
    }
}

/// Adds or replaces blocks from an `.iqal` file (locked blocks are never
/// weakened: the service refuses those).
fn import_blocks() {
    let owner = ui::main_window();
    let Some(path) = open_file(owner, &t("menu-import")) else { return };
    let json = match std::fs::read_to_string(&path) {
        Ok(j) => j,
        Err(e) => return dialog::error(owner, &e.to_string()),
    };
    match ui::call(Request::Import { json }) {
        Ok(Response::Imported { added, replaced, rejected }) => {
            let mut lines = Vec::new();
            lines.extend(added.iter().map(|n| ta("import-added", &[("name", n.clone().into())])));
            lines.extend(replaced.iter().map(|n| ta("import-replaced", &[("name", n.clone().into())])));
            lines.extend(
                rejected.iter().map(|(n, why)| ta("import-rejected", &[("name", n.clone().into()), ("why", why.clone().into())])),
            );
            dialog::message(owner, dialog::Icon::Info, &t("import-done"), &lines.join("\n"));
            ui::refresh(true);
        }
        Ok(_) => dialog::error(owner, &t("error-unexpected")),
        Err(e) => dialog::error(owner, &e),
    }
}

/// What was blocked recently, newest first.
fn activity() {
    let owner = ui::main_window();
    let mut events = match ui::call(Request::RecentEvents) {
        Ok(Response::Events { events }) => events,
        Ok(_) => Vec::new(),
        Err(e) => return dialog::error(owner, &e),
    };
    events.reverse();
    modal::run(owner, &t("menu-activity"), (680, 540), move |page, closer| {
        let mut items = Vec::new();
        if events.is_empty() {
            items.push(Item::Text(page.note(&t("activity-empty"))));
        } else {
            let list = page.control(
                "LISTBOX",
                "",
                WS_TABSTOP | WS_VSCROLL | (LBS_OWNERDRAWFIXED | LBS_NODATA | LBS_NOINTEGRALHEIGHT) as u32,
                WS_EX_CLIENTEDGE,
            );
            win::send(list, LB_SETITEMHEIGHT, 0, scale(46, panel::dpi()) as isize);
            win::send(list, LB_SETCOUNT, events.len(), 0);
            let shown = events.clone();
            panel::on_draw(list, move |item| draw_event(&shown, item));
            items.push(Item::Full { hwnd: list, height: 420 });
        }
        let spacer = page.control("STATIC", "", 0, 0);
        let close = page.control("BUTTON", &t("action-close"), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
        close_on(close, &closer);
        items.push(Item::Row { cells: vec![(spacer, Width::Flex), (close, button_width(&t("action-close")))], height: panel::ROW });
        page.set_items(items);
    });
}

/// One event: what and which, then when, who, block and why (greyed).
fn draw_event(events: &[BlockEvent], item: &DRAWITEMSTRUCT) {
    let Some(e) = events.get(item.itemID as usize) else { return };
    let Some(fonts) = panel::fonts() else { return };
    let what = t(match e.kind {
        EventKind::AppStart => "event-app-start",
        EventKind::AppKilled => "event-app-killed",
        EventKind::WindowClosed => "event-window-closed",
        EventKind::SiteBlocked => "event-site-blocked",
        EventKind::PageBlocked => "event-page-blocked",
        EventKind::AllowanceLow => "event-allowance-low",
        EventKind::AllowanceUsed => "event-allowance-used",
    });
    let lines = [
        (format!("{what}: {}", e.target), COLOR_WINDOWTEXT),
        (format!("{} · {} · {} · {}", fmt_time(e.time), e.user, e.set_name, e.reason), COLOR_GRAYTEXT),
    ];
    let pad = scale(8, panel::dpi());
    let half = (item.rcItem.bottom - item.rcItem.top) / 2;
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    // SAFETY: drawing into the list's DC during WM_DRAWITEM.
    unsafe {
        FillRect(item.hDC, &item.rcItem, GetSysColorBrush(COLOR_WINDOW as i32));
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SelectObject(item.hDC, fonts.normal);
        for (i, (text, colour)) in lines.iter().enumerate() {
            let top = item.rcItem.top + pad / 2 + i as i32 * (half - pad / 2);
            let mut r = RECT { left: item.rcItem.left + pad, right: item.rcItem.right - pad, top, bottom: top + half };
            SetTextColor(item.hDC, GetSysColor(*colour as i32));
            // Ends in a zero: DrawTextW reads through the pointer even for an empty text.
            let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
            DrawTextW(item.hDC, text.as_ptr(), text.len() as i32 - 1, &mut r, DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX | rtl);
        }
    }
}

/// The app's language, and what the service reports about itself.
fn settings() {
    let owner = ui::main_window();
    let status = ui::status();
    modal::run(owner, &t("menu-settings"), (560, 460), move |page, closer| {
        let mut items = vec![Item::Full { hwnd: page.heading(&t("settings-language")), height: 22 }];
        let language = page.control("COMBOBOX", "", WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32, 0);
        let mut names = vec![t("settings-language-system")];
        names.extend(LANGUAGES.iter().map(|(_, n, _)| n.to_string()));
        for name in &names {
            let text = win::wide(name);
            win::send(language, CB_ADDSTRING, 0, text.as_ptr() as isize);
        }
        let saved = GuiSettings::load();
        let pos = saved.language.as_deref().and_then(|c| LANGUAGES.iter().position(|(code, _, _)| *code == c)).map_or(0, |i| i + 1);
        win::send(language, CB_SETCURSEL, pos, 0);
        {
            let owner = closer.hwnd();
            panel::on(language, move |code| {
                if code != CBN_SELCHANGE {
                    return;
                }
                let mut s = GuiSettings::load();
                s.language = match win::send(language, CB_GETCURSEL, 0, 0) {
                    i if i <= 0 => None,
                    i => LANGUAGES.get(i as usize - 1).map(|(c, _, _)| c.to_string()),
                };
                s.save();
                dialog::message(owner, dialog::Icon::Info, &t("settings-language"), &t("settings-language-restart"));
            });
        }
        items.push(Item::Dropdown { hwnd: language, width: Width::Dip(260) });

        if let Some(s) = &status {
            items.push(Item::Space(12));
            items.push(Item::Full { hwnd: page.heading(&t("settings-status")), height: 22 });
            let ext = if s.extensions.is_empty() { t("status-no-extensions") } else { s.extensions.join(", ") };
            let mut lines = vec![
                ta("status-service", &[("version", s.version.clone().into())]),
                ta("status-time", &[("source", s.clock.clone().into())]),
                ta("status-location", &[("state", s.location.clone().into())]),
                ta("status-extensions", &[("list", ext.into())]),
            ];
            if !s.admin {
                lines.push(t("status-not-admin"));
            }
            for line in lines {
                items.push(Item::Text(page.note(&line)));
            }
        }
        let spacer = page.control("STATIC", "", 0, 0);
        let close = page.control("BUTTON", &t("action-close"), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
        close_on(close, &closer);
        items.push(Item::Space(8));
        items.push(Item::Row { cells: vec![(spacer, Width::Flex), (close, button_width(&t("action-close")))], height: panel::ROW });
        page.set_items(items);
    });
}

fn about() {
    let detail = format!(
        "{}\n\n{} {}\nGNU General Public License 3.0\nhttps://github.com/zakariaadem/Iqal",
        t("app-description"),
        t("app-name"),
        env!("CARGO_PKG_VERSION"),
    );
    dialog::message(ui::main_window(), dialog::Icon::Info, &t("app-name"), &detail);
}

/// Asks where to save a file.
fn save_file(owner: HWND, title: &str, suggested: &str) -> Option<String> {
    let mut file: Vec<u16> = suggested.encode_utf16().collect();
    file.resize(1024, 0);
    let filter: Vec<u16> = "Iqal (*.iqal)\0*.iqal\0\0".encode_utf16().collect();
    let extension = win::wide("iqal");
    let title = win::wide(title);
    // SAFETY: plain data; the buffers outlive the call.
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = file.as_mut_ptr();
    ofn.nMaxFile = file.len() as u32;
    ofn.lpstrTitle = title.as_ptr();
    ofn.lpstrDefExt = extension.as_ptr();
    ofn.Flags = OFN_EXPLORER | OFN_OVERWRITEPROMPT;
    // SAFETY: ofn is filled in as GetSaveFileNameW requires.
    if unsafe { GetSaveFileNameW(&mut ofn) } == 0 {
        return None;
    }
    let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
    Some(String::from_utf16_lossy(&file[..len]))
}
