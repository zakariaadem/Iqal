//! The editor for one block: name and switches on top, then tabs, and a
//! save bar while there are unsaved changes.
//!
//! Controls that already show their new value change the draft with
//! `ui::edit`; changes that show or hide parts of the editor use
//! `ui::modify`, which rebuilds it.

use std::rc::Rc;

use iqal_app::draft::ListKey;
use iqal_core::ListMode;
use iqal_core::import::parse_rules_text;
use iqal_i18n::{t, ta};
use iqal_ipc::{Request, Response};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::Controls::{
    BST_CHECKED, TCIF_TEXT, TCITEMW, TCM_GETCURSEL, TCM_INSERTITEMW, TCM_SETCURSEL, TCN_SELCHANGE,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, CB_ADDSTRING, CB_GETCURSEL,
    CB_SETCURSEL, CBN_SELCHANGE, CBS_DROPDOWNLIST, EN_CHANGE, ES_AUTOHSCROLL, SWP_NOACTIVATE, SWP_NOZORDER,
    SetWindowPos, WS_CLIPSIBLINGS, WS_EX_CLIENTEDGE, WS_TABSTOP, WS_VSCROLL,
};

use crate::panel::{self, Item, Panel, Width, button_width};
use crate::rules;
use crate::ui::{self, View};
use crate::{dialog, win};

/// Static texts centred vertically (single line), without & shortcuts.
const SS_LABEL: u32 = 0x2000 | 0x0200;

pub struct Editor {
    pub root: Panel,
    save_cells: Vec<HWND>,
}

impl Editor {
    /// The save bar's controls, for showing and hiding.
    pub fn save_bar(&self) -> (Panel, Vec<HWND>) {
        (self.root, self.save_cells.clone())
    }
}

/// Builds the editor for the selected block in `area` of the main window.
pub fn build(main: HWND, area: RECT) -> Option<Editor> {
    let view = ui::view()?;
    let root = Panel::new(main, false);
    // SAFETY: our new panel.
    unsafe {
        SetWindowPos(
            root.hwnd,
            std::ptr::null_mut(),
            area.left,
            area.top,
            area.right - area.left,
            area.bottom - area.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
    let mut items = vec![header(root, &view)];
    if view.info.locked {
        items.push(Item::Text(root.note(&t("locked-banner"))));
    }

    let tabs = root.control("SysTabControl32", "", WS_TABSTOP | WS_CLIPSIBLINGS, 0);
    for (i, key) in ["tab-websites", "tab-apps", "tab-users", "tab-schedule", "tab-lock"].iter().enumerate() {
        let mut text = win::wide(&t(key));
        // SAFETY: plain data; the text outlives the message.
        let mut item: TCITEMW = unsafe { std::mem::zeroed() };
        item.mask = TCIF_TEXT;
        item.pszText = text.as_mut_ptr();
        win::send(tabs, TCM_INSERTITEMW, i, (&raw const item) as isize);
    }
    win::send(tabs, TCM_SETCURSEL, view.tab, 0);
    panel::on(tabs, move |code| {
        if code == TCN_SELCHANGE {
            ui::set_tab(win::send(tabs, TCM_GETCURSEL, 0, 0).max(0) as usize);
        }
    });
    items.push(Item::Full { hwnd: tabs, height: panel::ROW });

    let page = Panel::new(root.hwnd, true);
    match view.tab {
        0 => websites(page, &view),
        _ => page.set_items(vec![Item::Text(page.note(&t("list-empty")))]),
    }
    items.push(Item::Fill(page.hwnd));

    // The save bar, shown while there are unsaved changes.
    let unsaved = root.control("STATIC", &t("unsaved-changes"), SS_LABEL, 0);
    let discard = root.control("BUTTON", &t("action-discard"), WS_TABSTOP, 0);
    let save = root.control("BUTTON", &t("action-save"), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
    panel::on(discard, |code| {
        if code == BN_CLICKED {
            ui::discard();
        }
    });
    panel::on(save, |code| {
        if code == BN_CLICKED {
            ui::save();
        }
    });
    items.push(Item::Row {
        cells: vec![
            (unsaved, Width::Flex),
            (discard, button_width(&t("action-discard"))),
            (save, button_width(&t("action-save"))),
        ],
        height: panel::ROW,
    });
    root.set_items(items);
    let save_cells = vec![unsaved, discard, save];
    for &cell in &save_cells {
        root.set_hidden(cell, !view.dirty);
    }
    Some(Editor { root, save_cells })
}

/// Name, on or off, delete.
fn header(root: Panel, view: &View) -> Item {
    let name = root.control("EDIT", &view.set.name, WS_TABSTOP | ES_AUTOHSCROLL as u32, WS_EX_CLIENTEDGE);
    if let Some(fonts) = panel::fonts() {
        win::send(name, windows_sys::Win32::UI::WindowsAndMessaging::WM_SETFONT, fonts.bold as usize, 1);
    }
    panel::on(name, move |code| {
        if code == EN_CHANGE as u32 {
            let text = win::text(name).trim().to_string();
            if !text.is_empty() {
                ui::edit(|s| s.name = text);
            }
        }
    });
    let enabled = check_box(root, &t("block-enabled"), view.set.enabled, |s, on| s.enabled = on);
    let delete = root.control("BUTTON", &t("action-delete-block"), WS_TABSTOP, 0);
    // SAFETY: our button.
    unsafe { EnableWindow(delete, i32::from(!view.info.locked)) };
    panel::on(delete, |code| {
        if code == BN_CLICKED {
            ui::delete_selected();
        }
    });
    Item::Row {
        cells: vec![
            (name, Width::Flex),
            (enabled, Width::Dip(panel::text_width(&t("block-enabled")) * 96 / panel::dpi().max(1) as i32 + 32)),
            (delete, button_width(&t("action-delete-block"))),
        ],
        height: panel::ROW,
    }
}

/// A check box that changes the draft (the box shows the new value itself).
fn check_box(panel: Panel, text: &str, on: bool, apply: fn(&mut iqal_core::BlockSet, bool)) -> HWND {
    let check = panel.control("BUTTON", text, WS_TABSTOP | BS_AUTOCHECKBOX as u32, 0);
    win::send(check, BM_SETCHECK, if on { BST_CHECKED as usize } else { 0 }, 0);
    panel::on(check, move |code| {
        if code == BN_CLICKED {
            let on = win::send(check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
            ui::edit(|s| apply(s, on));
        }
    });
    check
}

/// "Block list" or "Allow list".
fn mode_dropdown(page: Panel, current: ListMode, labels: [String; 2], apply: fn(&mut iqal_core::BlockSet, ListMode)) -> Item {
    let combo = page.control("COMBOBOX", "", WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32, 0);
    for label in &labels {
        let text = win::wide(label);
        win::send(combo, CB_ADDSTRING, 0, text.as_ptr() as isize);
    }
    win::send(combo, CB_SETCURSEL, usize::from(current == ListMode::AllowOnly), 0);
    panel::on(combo, move |code| {
        if code == CBN_SELCHANGE {
            let mode = if win::send(combo, CB_GETCURSEL, 0, 0) == 1 { ListMode::AllowOnly } else { ListMode::Block };
            if mode != current {
                // The lists shown depend on the mode: rebuild.
                ui::modify(|s| apply(s, mode));
            }
        }
    });
    let widest = labels.iter().map(|l| panel::text_width(l)).max().unwrap_or(0) * 96 / panel::dpi().max(1) as i32;
    Item::Dropdown { hwnd: combo, width: Width::Dip(widest + 48) }
}

fn websites(page: Panel, view: &View) {
    let set = &view.set;
    let mut items =
        vec![mode_dropdown(page, set.web.mode, [t("web-mode-block"), t("web-mode-allow-only")], |s, m| s.web.mode = m)];
    let allow_only = set.web.mode == ListMode::AllowOnly;
    // Pre-made lists are block lists: not used with an allow list.
    let catalog = match ui::call(Request::Catalog) {
        Ok(Response::Catalog { lists }) if !allow_only => lists,
        _ => Vec::new(),
    };
    let examples = t("web-placeholder");
    if allow_only {
        // An allow list stands on its own.
        items.extend(rules::editor(page, ListKey::WebAllow, &t("add-websites"), &t("web-allowed"), &examples, &[], &[]).items);
        items.push(Item::Text(page.note(&t("web-syntax"))));
    } else {
        // The chosen pre-made lists show in the blocked websites.
        let chosen: Vec<(String, Rc<Vec<String>>)> = catalog
            .iter()
            .filter(|l| set.web.lists.contains(&l.id))
            .map(|l| (t(&format!("list-{}", l.id)), ui::list_rules(l)))
            .collect();
        let import = page.control("BUTTON", &t("action-import-text"), WS_TABSTOP, 0);
        let blocked = rules::editor(page, ListKey::WebBlock, &t("add-websites"), &t("web-blocked"), &examples, &chosen, &[import]);
        let add = blocked.add.clone();
        panel::on(import, move |code| {
            if code == BN_CLICKED {
                import_text(add.clone());
            }
        });
        items.extend(blocked.items);
        items.push(Item::Text(page.note(&t("web-syntax"))));
        items.extend(
            rules::editor(page, ListKey::WebAllow, &t("add-exceptions"), &t("web-exceptions"), &examples, &[], &[]).items,
        );
    }

    let safe = check_box(page, &t("web-safesearch"), set.web.safe_search, |s, on| s.web.safe_search = on);
    items.push(Item::Full { hwnd: safe, height: panel::ROW });

    if !catalog.is_empty() {
        items.push(Item::Space(8));
        items.push(Item::Full { hwnd: page.heading(&t("web-lists")), height: 22 });
        items.push(Item::Text(page.note(&t("web-lists-detail"))));
    }
    for list in catalog {
        let text = format!(
            "{}  ·  {}",
            t(&format!("list-{}", list.id)),
            ta("list-count", &[("count", (list.rules as i64).into())])
        );
        let check = page.control("BUTTON", &text, WS_TABSTOP | BS_AUTOCHECKBOX as u32, 0);
        let on = set.web.lists.contains(&list.id);
        win::send(check, BM_SETCHECK, if on { BST_CHECKED as usize } else { 0 }, 0);
        let id = list.id.clone();
        panel::on(check, move |code| {
            if code == BN_CLICKED {
                let on = win::send(check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
                let id = id.clone();
                // The list's websites show in the blocked websites: rebuild.
                ui::modify(move |s| {
                    s.web.lists.retain(|x| *x != id);
                    if on {
                        s.web.lists.push(id);
                    }
                });
            }
        });
        items.push(Item::Full { hwnd: check, height: 26 });
    }
    page.set_items(items);
}

/// Adds the rules of a text file (a list, or a hosts file) to the blocked
/// websites.
fn import_text(add: Rc<dyn Fn(Vec<String>) -> usize>) {
    let owner = ui::main_window();
    let Some(path) = open_file(owner, &t("action-import-text")) else { return };
    let text = match std::fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(e) => return dialog::error(owner, &e.to_string()),
    };
    let parsed = parse_rules_text(&text);
    let skipped = parsed.errors.len();
    let added = add(parsed.rules.iter().map(ToString::to_string).collect());
    dialog::message(
        owner,
        dialog::Icon::Info,
        &t("import-done"),
        &ta("import-result", &[("added", (added as i64).into()), ("skipped", (skipped as i64).into())]),
    );
}

/// Asks for a file to open.
pub fn open_file(owner: HWND, title: &str) -> Option<String> {
    let mut file = vec![0u16; 1024];
    let filter: Vec<u16> = "*.*\0*.*\0\0".encode_utf16().collect();
    let title = win::wide(title);
    // SAFETY: plain data; the buffers outlive the call.
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = file.as_mut_ptr();
    ofn.nMaxFile = file.len() as u32;
    ofn.lpstrTitle = title.as_ptr();
    ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;
    // SAFETY: ofn is filled in as GetOpenFileNameW requires.
    if unsafe { GetOpenFileNameW(&mut ofn) } == 0 {
        return None;
    }
    let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
    Some(String::from_utf16_lossy(&file[..len]))
}
