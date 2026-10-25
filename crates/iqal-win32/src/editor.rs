//! The editor for one block: name and switches on top, then tabs, and a
//! save bar while there are unsaved changes.
//!
//! Controls that already show their new value change the draft with
//! `ui::edit`; changes that show or hide parts of the editor use
//! `ui::modify`, which rebuilds it.

use std::rc::Rc;

use iqal_app::draft::ListKey;
use iqal_app::describe::day_name;
use iqal_core::schedule::{Day, WeeklyRange};
use iqal_core::{Allowance, ListMode, UserScope};
use iqal_core::import::parse_rules_text;
use iqal_i18n::{t, ta};
use iqal_ipc::{Request, Response};
use windows_sys::Win32::Foundation::{HWND, RECT, SYSTEMTIME};
use windows_sys::Win32::NetworkManagement::NetManagement::{
    FILTER_NORMAL_ACCOUNT, MAX_PREFERRED_LENGTH, NetApiBufferFree, NetUserEnum, UF_ACCOUNTDISABLE, USER_INFO_3,
};
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::Controls::{
    BST_CHECKED, DTM_GETSYSTEMTIME, DTM_SETFORMATW, DTM_SETSYSTEMTIME, DTS_TIMEFORMAT, DTS_UPDOWN, GDT_VALID, TCIF_IMAGE, TCIF_TEXT, TCITEMW, TCM_GETCURSEL, TCM_INSERTITEMW, TCM_SETCURSEL, TCN_SELCHANGE,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_DEFPUSHBUTTON, BS_PUSHLIKE, CB_ADDSTRING, CB_GETCURSEL,
    CB_SETCURSEL, CBN_SELCHANGE, CBS_DROPDOWNLIST, EN_CHANGE, ES_AUTOHSCROLL, ES_NUMBER,
    SWP_NOACTIVATE, SWP_NOZORDER,
    SetWindowPos, WS_CLIPSIBLINGS, WS_EX_CLIENTEDGE, WS_GROUP, WS_TABSTOP, WS_VSCROLL,
};

use crate::panel::{self, Item, Panel, Width, width_of};
use crate::{lockui, rules};
use crate::ui::{self, View};
use crate::{dialog, theme, win};

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
    // Shown by the caller once it is complete.
    let root = Panel::hidden(main, false);
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
    let mut items = header(root, &view);
    if view.info.locked {
        items.push(Item::Text(root.note(&t("locked-banner"))));
    }

    let tabs = root.control("SysTabControl32", "", WS_TABSTOP | WS_CLIPSIBLINGS, 0);
    use theme::Icon;
    theme::tab_icons(tabs, &[Icon::Globe, Icon::Apps, Icon::People, Icon::Lock]);
    for (i, key) in ["tab-websites", "tab-apps", "tab-users", "tab-lock"].iter().enumerate() {
        let mut text = win::wide(&t(key));
        // SAFETY: plain data; the text outlives the message.
        let mut item: TCITEMW = unsafe { std::mem::zeroed() };
        // Picture 0 is blank: it makes room for the icon.
        item.mask = TCIF_TEXT | TCIF_IMAGE;
        item.iImage = 0;
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
        1 => apps(page, &view),
        2 => people(page, &view),
        3 => lockui::page(page, &view),
        _ => page.set_items(vec![Item::Text(page.note(&t("list-empty")))]),
    }
    items.push(Item::Fill(page.hwnd));

    // The save bar, shown while there are unsaved changes.
    let unsaved = root.control("STATIC", &t("unsaved-changes"), SS_LABEL, 0);
    let discard = root.control("BUTTON", &t("action-discard"), WS_TABSTOP, 0);
    theme::style_button(discard, Some(theme::Icon::Discard), theme::Look::Plain);
    let save = root.control("BUTTON", &t("action-save"), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
    theme::style_button(save, Some(theme::Icon::Save), theme::Look::Main);
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
            (discard, width_of(discard)),
            (save, width_of(save)),
        ],
        height: panel::ROW,
    });
    // Hidden before the controls are placed: placing them is the slow part.
    let save_cells = vec![unsaved, discard, save];
    for &cell in &save_cells {
        root.set_hidden(cell, !view.dirty);
    }
    root.set_items(items);
    Some(Editor { root, save_cells })
}

/// Name, on or off, lock, export, delete.
fn header(root: Panel, view: &View) -> Vec<Item> {
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
    // On or off: a switch whose text says which.
    let state_text = |on: bool| t(if on { "block-switch-on" } else { "block-switch-off" });
    let enabled = root.switch(&state_text(view.set.enabled));
    win::send(enabled, BM_SETCHECK, if view.set.enabled { BST_CHECKED as usize } else { 0 }, 0);
    panel::on(enabled, move |code| {
        if code == BN_CLICKED {
            let on = win::send(enabled, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
            win::set_text(enabled, &state_text(on));
            ui::edit(|s| s.enabled = on);
        }
    });
    let switch_width = [true, false].map(|on| crate::theme::switch_width(&state_text(on))).into_iter().max().unwrap_or(0);
    // Lock, or unlock while locked (the same as on the Lock tab).
    let lock_text = t(if view.info.locked { "action-unlock" } else { "action-lock" });
    let lock = root.control("BUTTON", &lock_text, WS_TABSTOP, 0);
    theme::style_button(lock, Some(if view.info.locked { theme::Icon::Unlock } else { theme::Icon::Lock }), theme::Look::Main);
    let (locked, id) = (view.info.locked, view.set.id);
    panel::on(lock, move |code| {
        if code == BN_CLICKED {
            if locked {
                lockui::start_unlock(id);
            } else {
                lockui::lock_dialog();
            }
        }
    });
    let export = root.control("BUTTON", &t("action-export-block"), WS_TABSTOP, 0);
    theme::style_button(export, Some(theme::Icon::Export), theme::Look::Plain);
    panel::on(export, move |code| {
        if code == BN_CLICKED {
            crate::menu::export(Some(vec![id]));
        }
    });
    let delete = root.control("BUTTON", &t("action-delete-block"), WS_TABSTOP, 0);
    theme::style_button(delete, Some(theme::Icon::Delete), theme::Look::Quiet);
    // SAFETY: our button.
    unsafe { EnableWindow(delete, i32::from(!view.info.locked)) };
    panel::on(delete, |code| {
        if code == BN_CLICKED {
            ui::delete_selected();
        }
    });
    // The window is kept wide enough for this row (see `min_width`), so the
    // name always has room.
    vec![Item::Row {
        cells: vec![
            (name, Width::Flex),
            (enabled, Width::Dip(switch_width)),
            (lock, width_of(lock)),
            (export, width_of(export)),
            (delete, width_of(delete)),
        ],
        height: panel::ROW,
    }]
}

/// The narrowest the editor can be, in pixels at 96 DPI: the top row with
/// room for the name.
pub fn min_width() -> i32 {
    const NAME: i32 = 140;
    const MARGINS: i32 = 2 * 16;
    const GAP: i32 = 8;
    let switch = ["block-switch-on", "block-switch-off"].map(|k| crate::theme::switch_width(&t(k))).into_iter().max().unwrap_or(0);
    let lock = ["action-lock", "action-unlock"].map(|k| crate::theme::button_width(&t(k), true)).into_iter().max().unwrap_or(0);
    let buttons = crate::theme::button_width(&t("action-export-block"), true) + crate::theme::button_width(&t("action-delete-block"), true);
    MARGINS + NAME + switch + lock + buttons + 4 * GAP
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
        theme::style_button(import, Some(theme::Icon::Import), theme::Look::Plain);
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
    items.push(Item::Text(page.note(&t("web-safesearch-detail"))));

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

fn apps(page: Panel, view: &View) {
    let set = &view.set;
    let mut items =
        vec![mode_dropdown(page, set.apps.mode, [t("apps-mode-block"), t("apps-mode-allow-only")], |s, m| s.apps.mode = m)];
    let examples = t("apps-placeholder-windows");
    // An allow list stands on its own; a block list has exceptions.
    let lists: &[(ListKey, &str, &str)] = match set.apps.mode {
        ListMode::AllowOnly => &[(ListKey::AppAllow, "add-apps", "apps-allowed")],
        ListMode::Block => &[(ListKey::AppBlock, "add-apps", "apps-blocked"), (ListKey::AppAllow, "add-exceptions", "apps-exceptions")],
    };
    for &(key, add_title, title) in lists {
        let installed = page.control("BUTTON", &t("action-add-installed-app"), WS_TABSTOP, 0);
        theme::style_button(installed, Some(theme::Icon::Apps), theme::Look::Plain);
        let file = page.control("BUTTON", &t("action-add-program-file"), WS_TABSTOP, 0);
        theme::style_button(file, Some(theme::Icon::File), theme::Look::Plain);
        let window = page.control("BUTTON", &t("action-add-window-title"), WS_TABSTOP, 0);
        theme::style_button(window, Some(theme::Icon::Window), theme::Look::Plain);
        let editor = rules::editor(page, key, &t(add_title), &t(title), &examples, &[], &[installed, file, window]);
        {
            let add = editor.add.clone();
            panel::on(window, move |code| {
                if code == BN_CLICKED {
                    let add = add.clone();
                    crate::apppicker::pick_title(ui::main_window(), move |rule| {
                        add(vec![rule]);
                    });
                }
            });
        }
        {
            let add = editor.add.clone();
            panel::on(installed, move |code| {
                if code == BN_CLICKED {
                    let add = add.clone();
                    crate::apppicker::pick(ui::main_window(), move |rules| {
                        add(rules);
                    });
                }
            });
        }
        let add = editor.add.clone();
        panel::on(file, move |code| {
            if code == BN_CLICKED
                && let Some(path) = open_file(ui::main_window(), &t("action-add-program-file"))
            {
                add(vec![format!("path:{path}")]);
            }
        });
        items.extend(editor.items);
    }
    items.push(Item::Text(page.note(&t("apps-syntax-windows"))));
    page.set_items(items);
}

/// Who the block applies to: everyone, everyone except some accounts, or
/// only some.
fn people(page: Panel, view: &View) {
    let (kind, names) = match &view.set.users {
        UserScope::All => (0, Vec::new()),
        UserScope::AllExcept(n) => (1, n.clone()),
        UserScope::Only(n) => (2, n.clone()),
    };
    let mut items = vec![Item::Full { hwnd: page.heading(&t("users-applies-to")), height: 22 }];
    for (i, key) in ["users-all", "users-all-except", "users-only"].iter().enumerate() {
        // The first radio button starts the group.
        let group = if i == 0 { WS_GROUP } else { 0 };
        let radio = page.control("BUTTON", &t(key), WS_TABSTOP | group | BS_AUTORADIOBUTTON as u32, 0);
        win::send(radio, BM_SETCHECK, if i == kind { BST_CHECKED as usize } else { 0 }, 0);
        let names = names.clone();
        panel::on(radio, move |code| {
            if code == BN_CLICKED && i != kind {
                let names = names.clone();
                // The account list shows or hides: rebuild.
                ui::modify(move |s| {
                    s.users = match i {
                        0 => UserScope::All,
                        1 => UserScope::AllExcept(names),
                        _ => UserScope::Only(names),
                    };
                });
            }
        });
        items.push(Item::Full { hwnd: radio, height: 26 });
    }
    if kind != 0 {
        items.push(Item::Space(8));
        let heading = if kind == 1 { "users-except-these" } else { "users-only-these" };
        items.push(Item::Full { hwnd: page.heading(&t(heading)), height: 22 });
        let mut all = windows_accounts();
        for n in &names {
            if !all.contains(n) {
                all.push(n.clone());
            }
        }
        for user in all {
            let check = page.control("BUTTON", &user, WS_TABSTOP | BS_AUTOCHECKBOX as u32, 0);
            win::send(check, BM_SETCHECK, if names.contains(&user) { BST_CHECKED as usize } else { 0 }, 0);
            panel::on(check, move |code| {
                if code == BN_CLICKED {
                    let on = win::send(check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
                    let user = user.clone();
                    ui::edit(move |s| {
                        if let UserScope::AllExcept(list) | UserScope::Only(list) = &mut s.users {
                            list.retain(|n| *n != user);
                            if on {
                                list.push(user);
                            }
                        }
                    });
                }
            });
            items.push(Item::Full { hwnd: check, height: 26 });
        }
    }
    items.push(Item::Space(8));
    items.push(Item::Text(page.note(&t("users-note"))));
    page.set_items(items);
}

/// When the block is in force (weekly times, one-off periods), and the
/// daily time allowance.
/// The Lock tab's daily allowance: each blocked website and app may be used
/// for some minutes a day. Also an old "only at certain times" schedule, if
/// the block still has one (times are now set with a lock): it can be
/// removed.
pub fn allowance(page: Panel, view: &View) -> Vec<Item> {
    let set = &view.set;
    let mut items = vec![Item::Space(16)];
    if set.schedule.is_some() {
        items.push(Item::Full { hwnd: page.heading(&t("schedule-old")), height: 22 });
        items.push(Item::Text(page.note(&t("schedule-old-detail"))));
        let remove = page.control("BUTTON", &t("schedule-old-remove"), WS_TABSTOP, 0);
        theme::style_button(remove, Some(theme::Icon::Remove), theme::Look::Plain);
        panel::on(remove, |code| {
            if code == BN_CLICKED {
                // Always in force: stricter, so allowed while locked too.
                ui::modify(|s| s.schedule = None);
            }
        });
        items.push(Item::Row { cells: vec![(remove, width_of(remove))], height: panel::ROW });
        items.push(Item::Space(16));
    }
    let allowance = page.control("BUTTON", &t("allowance"), WS_TABSTOP | BS_AUTOCHECKBOX as u32, 0);
    win::send(allowance, BM_SETCHECK, if set.allowance.is_some() { BST_CHECKED as usize } else { 0 }, 0);
    panel::on(allowance, move |code| {
        if code == BN_CLICKED {
            let on = win::send(allowance, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
            ui::modify(move |s| s.allowance = on.then_some(Allowance { daily_secs: 30 * 60 }));
        }
    });
    items.push(Item::Full { hwnd: allowance, height: 26 });
    items.push(Item::Text(page.note(&t("allowance-detail"))));
    if let Some(a) = set.allowance {
        let label = page.control("STATIC", &t("allowance-minutes"), SS_LABEL, 0);
        let minutes =
            page.control("EDIT", &(a.daily_secs / 60).to_string(), WS_TABSTOP | (ES_NUMBER | ES_AUTOHSCROLL) as u32, WS_EX_CLIENTEDGE);
        panel::on(minutes, move |code| {
            if code == EN_CHANGE as u32
                && let Ok(m) = win::text(minutes).trim().parse::<u32>()
                && (1..=24 * 60).contains(&m)
            {
                ui::edit(move |s| s.allowance = Some(Allowance { daily_secs: m * 60 }));
            }
        });
        let width = Width::Dip(panel::text_width(&t("allowance-minutes")) * 96 / panel::dpi().max(1) as i32 + 12);
        items.push(Item::Row { cells: vec![(label, width), (minutes, Width::Dip(80))], height: panel::ROW });
        // Time used today, per website and app (the most used five).
        let total = (a.daily_secs / 60) as i64;
        let used: Vec<String> = view
            .info
            .used_items
            .iter()
            .take(5)
            .map(|(item, secs)| {
                ta("allowance-used-item", &[("item", item.clone().into()), ("used", ((secs / 60) as i64).into()), ("total", total.into())])
            })
            .collect();
        let text = if used.is_empty() { t("allowance-unused") } else { format!("{} {}", t("allowance-used-today-items"), used.join("   ·   ")) };
        items.push(Item::Text(page.control("STATIC", &text, 0x2000, 0)));
    }
    items
}

/// Day buttons (weekdays chosen) and from/to times; returns the rows and a
/// function giving the chosen times (`None` when no day is chosen).
pub fn weekly_picker(page: Panel) -> (Vec<Item>, Rc<dyn Fn() -> Option<WeeklyRange>>) {
    let days: Vec<(Day, HWND)> = Day::ALL
        .iter()
        .map(|&d| {
            let b = page.control("BUTTON", &day_name(d), WS_TABSTOP | (BS_AUTOCHECKBOX | BS_PUSHLIKE) as u32, 0);
            let workday = !matches!(d, Day::Sat | Day::Sun);
            win::send(b, BM_SETCHECK, if workday { BST_CHECKED as usize } else { 0 }, 0);
            (d, b)
        })
        .collect();
    let day_width =
        Day::ALL.iter().map(|&d| panel::text_width(&day_name(d))).max().unwrap_or(0) * 96 / panel::dpi().max(1) as i32 + 24;
    let from_label = page.control("STATIC", &t("time-from"), SS_LABEL, 0);
    let from = time_picker(page, 9 * 60);
    let to_label = page.control("STATIC", &t("time-to"), SS_LABEL, 0);
    let to = time_picker(page, 17 * 60);
    let label_width = |key: &str| Width::Dip(panel::text_width(&t(key)) * 96 / panel::dpi().max(1) as i32 + 8);
    let items = vec![
        Item::Row { cells: days.iter().map(|&(_, b)| (b, Width::Dip(day_width))).collect(), height: panel::ROW },
        Item::Row {
            cells: vec![
                (from_label, label_width("time-from")),
                (from, Width::Dip(90)),
                (to_label, label_width("time-to")),
                (to, Width::Dip(90)),
            ],
            height: panel::ROW,
        },
    ];
    let picked = move || {
        let chosen: Vec<Day> =
            days.iter().filter(|(_, b)| win::send(*b, BM_GETCHECK, 0, 0) == BST_CHECKED as isize).map(|(d, _)| *d).collect();
        if chosen.is_empty() {
            return None;
        }
        let start = picked_minutes(from);
        // 00:00 as the end means midnight at the end of the day.
        let end = match picked_minutes(to) {
            0 => 24 * 60,
            m => m,
        };
        Some(WeeklyRange { days: chosen, start, end })
    };
    (items, Rc::new(picked))
}

/// A time picker (hours and minutes) set to `minutes` after midnight.
pub fn time_picker(page: Panel, minutes: u16) -> HWND {
    let picker = page.control("SysDateTimePick32", "", WS_TABSTOP | (DTS_TIMEFORMAT | DTS_UPDOWN) as u32, 0);
    let format = win::wide("HH':'mm");
    win::send(picker, DTM_SETFORMATW, 0, format.as_ptr() as isize);
    // SAFETY: plain data.
    let mut time: SYSTEMTIME = unsafe { std::mem::zeroed() };
    // SAFETY: as above.
    unsafe { GetLocalTime(&mut time) };
    time.wHour = minutes / 60;
    time.wMinute = minutes % 60;
    time.wSecond = 0;
    time.wMilliseconds = 0;
    win::send(picker, DTM_SETSYSTEMTIME, GDT_VALID as usize, (&raw const time) as isize);
    picker
}

pub fn picked_minutes(picker: HWND) -> u16 {
    // SAFETY: plain data, filled in by the picker.
    let mut time: SYSTEMTIME = unsafe { std::mem::zeroed() };
    win::send(picker, DTM_GETSYSTEMTIME, 0, (&raw mut time) as isize);
    time.wHour * 60 + time.wMinute
}

/// People's accounts on this computer: enabled local accounts with a
/// relative id of 1000 or more (as the service enforces; not the built-in
/// Administrator, Guest and service accounts).
fn windows_accounts() -> Vec<String> {
    let mut out = Vec::new();
    let mut buf: *mut u8 = std::ptr::null_mut();
    let (mut read, mut total, mut resume) = (0u32, 0u32, 0u32);
    // SAFETY: valid out pointers; the buffer is freed below.
    let status = unsafe {
        NetUserEnum(std::ptr::null(), 3, FILTER_NORMAL_ACCOUNT, &mut buf, MAX_PREFERRED_LENGTH, &mut read, &mut total, &mut resume)
    };
    if status != 0 || buf.is_null() {
        return out;
    }
    // SAFETY: NetUserEnum returned `read` USER_INFO_3 entries.
    let users = unsafe { std::slice::from_raw_parts(buf.cast::<USER_INFO_3>(), read as usize) };
    for user in users {
        if user.usri3_user_id < 1000 || user.usri3_flags & UF_ACCOUNTDISABLE != 0 {
            continue;
        }
        // SAFETY: a NUL-terminated name inside the buffer.
        let name = unsafe {
            let len = (0..).take_while(|&i| *user.usri3_name.add(i) != 0).count();
            String::from_utf16_lossy(std::slice::from_raw_parts(user.usri3_name, len))
        };
        out.push(name);
    }
    // SAFETY: allocated by NetUserEnum.
    unsafe { NetApiBufferFree(buf.cast()) };
    out.sort_by_key(|n| n.to_lowercase());
    out
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
