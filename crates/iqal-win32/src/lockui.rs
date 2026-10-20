//! The Lock tab: current lock, locking, unlocking.

use std::rc::Rc;

use iqal_app::describe::{describe_lock, fmt_time};
use iqal_core::lock::random_text;
use iqal_core::schedule::Schedule;
use iqal_core::{GeoArea, Lock, LockKind, PasswordHash, SetId};
use iqal_i18n::{t, ta};
use iqal_ipc::{ChallengeInfo, Request, Response, UnlockInfo};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, SYSTEMTIME, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateFontW, DeleteObject, FW_NORMAL};
use windows_sys::Win32::UI::Controls::{BST_CHECKED, DTM_GETSYSTEMTIME, DTS_SHORTDATEFORMAT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_DEFPUSHBUTTON, CB_ADDSTRING,
    CB_GETCURSEL, CB_SETCURSEL, CBN_SELCHANGE, CBS_DROPDOWNLIST, ES_AUTOHSCROLL, ES_NUMBER, ES_PASSWORD, WM_CONTEXTMENU,
    WM_PASTE, WM_SETFONT, WS_EX_CLIENTEDGE, WS_GROUP, WS_TABSTOP, WS_VSCROLL,
};

use crate::editor::{picked_minutes, time_picker, weekly_picker};
use crate::modal::{self, Closer};
use crate::panel::{self, Item, Panel, Width, button_width};
use crate::ui::{self, View};
use crate::{dialog, win};

const SS_LABEL: u32 = 0x2000 | 0x0200;
const EM_SETCUEBANNER: u32 = 0x1501;

const KINDS: [&str; 7] = [
    "lock-kind-duration",
    "lock-kind-until",
    "lock-kind-password",
    "lock-kind-random",
    "lock-kind-restart",
    "lock-kind-schedule",
    "lock-kind-location",
];

/// The Lock tab.
pub fn page(page: Panel, view: &View) {
    let info = &view.info;
    let set = &info.set;
    let id = set.id;
    let mut items = Vec::new();
    match &set.lock {
        None => {
            items.push(Item::Full { hwnd: page.heading(&t("lock-none")), height: 22 });
            items.push(Item::Text(page.note(&t("lock-none-detail"))));
        }
        Some(lock) => {
            let state = if info.locked { "lock-active" } else { "lock-inactive" };
            items.push(Item::Full { hwnd: page.heading(&t(state)), height: 22 });
            items.push(Item::Text(page.control("STATIC", &describe_lock(lock), 0x2000, 0)));
            if let Some(at) = set.pending_unlock_at {
                items.push(Item::Text(page.control("STATIC", &ta("lock-pending", &[("time", fmt_time(at).into())]), 0x2000, 0)));
                let cancel = page.control("BUTTON", &t("action-cancel-unlock"), WS_TABSTOP, 0);
                panel::on(cancel, move |code| {
                    if code == BN_CLICKED {
                        if let Err(e) = ui::call(Request::CancelUnlock { id }) {
                            dialog::error(ui::main_window(), &e);
                        }
                        ui::refresh(true);
                    }
                });
                items.push(Item::Row { cells: vec![(cancel, button_width(&t("action-cancel-unlock")))], height: panel::ROW });
            }
            if let LockKind::Location { .. } = lock.kind {
                if info.place_known {
                    items.push(Item::Text(page.note(&t("location-known"))));
                } else {
                    // Only needed when Iqal cannot recognise the place by
                    // itself (it was not there when the lock was made).
                    items.push(Item::Text(page.note(&t("location-learn-detail"))));
                    let learn = page.control("BUTTON", &t("action-learn-location"), WS_TABSTOP, 0);
                    panel::on(learn, move |code| {
                        if code == BN_CLICKED {
                            learn_place(ui::main_window(), id);
                            ui::refresh(true);
                        }
                    });
                    items.push(Item::Row { cells: vec![(learn, button_width(&t("action-learn-location")))], height: panel::ROW });
                }
            }
            if info.locked {
                let unlock = page.control("BUTTON", &t("action-unlock"), WS_TABSTOP, 0);
                panel::on(unlock, move |code| {
                    if code == BN_CLICKED {
                        start_unlock(id);
                    }
                });
                items.push(Item::Row { cells: vec![(unlock, button_width(&t("action-unlock")))], height: panel::ROW });
            } else {
                let remove = page.control("BUTTON", &t("action-remove-lock"), WS_TABSTOP, 0);
                panel::on(remove, |code| {
                    if code == BN_CLICKED {
                        ui::commit(ui::main_window(), |s| s.lock = None);
                    }
                });
                items.push(Item::Row { cells: vec![(remove, button_width(&t("action-remove-lock")))], height: panel::ROW });
            }
        }
    }
    if !info.locked {
        let text = t(if set.lock.is_some() { "action-change-lock" } else { "action-lock" });
        let lock = page.control("BUTTON", &text, WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
        panel::on(lock, |code| {
            if code == BN_CLICKED {
                lock_dialog();
            }
        });
        items.push(Item::Space(8));
        items.push(Item::Row { cells: vec![(lock, button_width(&text))], height: panel::ROW });
    }
    page.set_items(items);
}

/// Remembers the Wi-Fi networks around the computer for the block's
/// location lock, and says how many.
fn learn_place(owner: HWND, id: SetId) {
    match ui::call(Request::LearnLocation { id }) {
        Ok(Response::Learned { networks }) => dialog::message(
            owner,
            dialog::Icon::Info,
            &t("location-learned"),
            &ta("location-learned-detail", &[("count", (networks as i64).into())]),
        ),
        Ok(_) => dialog::error(owner, &t("error-unexpected")),
        Err(e) => dialog::error(owner, &format!("{}\n{e}", t("location-learn-failed"))),
    }
}

fn number_box(page: Panel, value: u32) -> HWND {
    page.control("EDIT", &value.to_string(), WS_TABSTOP | (ES_NUMBER | ES_AUTOHSCROLL) as u32, WS_EX_CLIENTEDGE)
}

fn number(hwnd: HWND) -> u32 {
    win::text(hwnd).trim().parse().unwrap_or(0)
}

fn label(page: Panel, key: &str) -> (HWND, Width) {
    let text = t(key);
    (page.control("STATIC", &text, SS_LABEL, 0), Width::Dip(panel::text_width(&text) * 96 / panel::dpi().max(1) as i32 + 8))
}

fn checked(hwnd: HWND) -> bool {
    win::send(hwnd, BM_GETCHECK, 0, 0) == BST_CHECKED as isize
}

/// Choosing a lock: what kind, its details, and confirming.
pub fn lock_dialog() {
    let owner = ui::main_window();
    modal::run(owner, &t("lock-dialog-title"), (580, 560), |page, closer| build_lock_dialog(page, closer));
}

fn build_lock_dialog(page: Panel, closer: Closer) {
    let kind = page.control("COMBOBOX", "", WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32, 0);
    for key in KINDS {
        let text = win::wide(&t(key));
        win::send(kind, CB_ADDSTRING, 0, text.as_ptr() as isize);
    }
    win::send(kind, CB_SETCURSEL, 0, 0);
    let mut items = vec![Item::Dropdown { hwnd: kind, width: Width::Flex }];
    // The controls of each kind, shown only while it is chosen.
    let mut sections: Vec<Vec<HWND>> = vec![Vec::new(); KINDS.len()];

    // For a while.
    let (days, hours, minutes) = (number_box(page, 0), number_box(page, 2), number_box(page, 0));
    let (dl, hl, ml) = (label(page, "unit-days"), label(page, "unit-hours"), label(page, "unit-minutes"));
    sections[0].extend([days, dl.0, hours, hl.0, minutes, ml.0]);
    items.push(Item::Row {
        cells: vec![(days, Width::Dip(70)), dl, (hours, Width::Dip(70)), hl, (minutes, Width::Dip(70)), ml],
        height: panel::ROW,
    });

    // Until a date.
    let date = page.control("SysDateTimePick32", "", WS_TABSTOP | DTS_SHORTDATEFORMAT, 0);
    let at = label(page, "time-at");
    let time = time_picker(page, 18 * 60);
    sections[1].extend([date, at.0, time]);
    items.push(Item::Row { cells: vec![(date, Width::Dip(180)), at, (time, Width::Dip(90))], height: panel::ROW });

    // With a password.
    let p1 = page.control("EDIT", "", WS_TABSTOP | (ES_PASSWORD | ES_AUTOHSCROLL) as u32, WS_EX_CLIENTEDGE);
    let p2 = page.control("EDIT", "", WS_TABSTOP | (ES_PASSWORD | ES_AUTOHSCROLL) as u32, WS_EX_CLIENTEDGE);
    for (edit, key) in [(p1, "password"), (p2, "password-repeat")] {
        let cue = win::wide(&t(key));
        win::send(edit, EM_SETCUEBANNER, 1, cue.as_ptr() as isize);
    }
    let generate = page.control("BUTTON", &t("password-generate"), WS_TABSTOP | BS_AUTOCHECKBOX as u32, 0);
    panel::on(generate, move |code| {
        if code == BN_CLICKED {
            // Iqal makes the password: nothing to type.
            let typed = i32::from(!checked(generate));
            // SAFETY: our edit boxes.
            unsafe {
                EnableWindow(p1, typed);
                EnableWindow(p2, typed);
            }
        }
    });
    let generate_note = page.note(&t("password-generate-detail"));
    sections[2].extend([p1, p2, generate, generate_note]);
    items.push(Item::Full { hwnd: p1, height: panel::ROW });
    items.push(Item::Full { hwnd: p2, height: panel::ROW });
    items.push(Item::Full { hwnd: generate, height: 26 });
    items.push(Item::Text(generate_note));

    // By typing random text.
    let length_label = label(page, "random-length");
    let length = number_box(page, 200);
    sections[3].extend([length_label.0, length]);
    items.push(Item::Row { cells: vec![length_label, (length, Width::Dip(90))], height: panel::ROW });

    // Until the computer restarts.
    let restart_note = page.note(&t("lock-restart-detail"));
    sections[4].push(restart_note);
    items.push(Item::Text(restart_note));

    // During certain times.
    let (weekly, picked) = weekly_picker(page);
    for item in &weekly {
        if let Item::Row { cells, .. } = item {
            sections[5].extend(cells.iter().map(|(h, _)| *h));
        }
    }
    items.extend(weekly);

    // While at a place: a name, and whether the computer is there now.
    // No answer is chosen in advance: a wrong "yes" would teach Iqal the
    // networks of another place.
    let name_label = label(page, "location-name");
    let place = page.control("EDIT", "", WS_TABSTOP | ES_AUTOHSCROLL as u32, WS_EX_CLIENTEDGE);
    let cue = win::wide(&t("location-name-placeholder"));
    win::send(place, EM_SETCUEBANNER, 1, cue.as_ptr() as isize);
    let place_note = page.note(&t("lock-location-detail"));
    let question = page.heading(&t("location-here-question"));
    let yes = page.control("BUTTON", &t("location-here-yes"), WS_TABSTOP | WS_GROUP | BS_AUTORADIOBUTTON as u32, 0);
    let no = page.control("BUTTON", &t("location-here-no"), WS_TABSTOP | BS_AUTORADIOBUTTON as u32, 0);
    let here_note = page.note(&t("location-here-detail"));
    sections[6].extend([name_label.0, place, place_note, question, yes, no, here_note]);
    items.push(Item::Row { cells: vec![name_label, (place, Width::Flex)], height: panel::ROW });
    items.push(Item::Text(place_note));
    items.push(Item::Full { hwnd: question, height: 22 });
    items.push(Item::Full { hwnd: yes, height: 26 });
    items.push(Item::Full { hwnd: no, height: 26 });
    items.push(Item::Text(here_note));

    // A wait, only for locks opened with a password or a text.
    let delay_label = label(page, "unlock-delay");
    let delay = number_box(page, 0);
    let delay_unit = label(page, "unit-minutes");
    let delay_note = page.note(&t("unlock-delay-detail"));
    let delay_controls = [delay_label.0, delay, delay_unit.0, delay_note];
    items.push(Item::Space(8));
    items.push(Item::Row { cells: vec![delay_label, (delay, Width::Dip(80)), delay_unit], height: panel::ROW });
    items.push(Item::Text(delay_note));

    let spacer = page.control("STATIC", "", 0, 0);
    let cancel = page.control("BUTTON", &t("action-cancel"), WS_TABSTOP, 0);
    let ok = page.control("BUTTON", &t("action-lock"), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
    items.push(Item::Space(8));
    items.push(Item::Row {
        cells: vec![(spacer, Width::Flex), (cancel, button_width(&t("action-cancel"))), (ok, button_width(&t("action-lock")))],
        height: panel::ROW,
    });
    page.set_items(items);

    let sections = Rc::new(sections);
    let show = {
        let sections = sections.clone();
        move |chosen: usize| {
            for (i, controls) in sections.iter().enumerate() {
                for &c in controls {
                    page.set_hidden(c, i != chosen);
                }
            }
            for c in delay_controls {
                page.set_hidden(c, !matches!(chosen, 2 | 3));
            }
        }
    };
    show(0);
    {
        let closer = closer.clone();
        panel::on(kind, move |code| {
            if code == CBN_SELCHANGE {
                show(win::send(kind, CB_GETCURSEL, 0, 0).max(0) as usize);
                closer.fit();
            }
        });
    }
    {
        let closer = closer.clone();
        panel::on(cancel, move |code| {
            if code == BN_CLICKED {
                closer.close();
            }
        });
    }
    panel::on(ok, move |code| {
        if code != BN_CLICKED {
            return;
        }
        let owner = closer.hwnd();
        let fail = |key: &str| dialog::message(owner, dialog::Icon::Warning, &t(key), "");
        let now = jiff::Timestamp::now().as_second();
        let chosen = win::send(kind, CB_GETCURSEL, 0, 0).max(0) as usize;
        let mut shown_password = None;
        let lock_kind = match chosen {
            0 => {
                let secs = number(days) as i64 * 86_400 + number(hours) as i64 * 3600 + number(minutes) as i64 * 60;
                if secs <= 0 {
                    return fail("lock-duration-zero");
                }
                LockKind::Until { until: now + secs }
            }
            1 => {
                // SAFETY: plain data, filled in by the date picker.
                let mut d: SYSTEMTIME = unsafe { std::mem::zeroed() };
                win::send(date, DTM_GETSYSTEMTIME, 0, (&raw mut d) as isize);
                let m = picked_minutes(time);
                let dt = jiff::civil::date(d.wYear as i16, d.wMonth as i8, d.wDay as i8).at((m / 60) as i8, (m % 60) as i8, 0, 0);
                match dt.to_zoned(jiff::tz::TimeZone::system()) {
                    Ok(z) if z.timestamp().as_second() > now => LockKind::Until { until: z.timestamp().as_second() },
                    _ => return fail("lock-until-past"),
                }
            }
            2 => {
                let password = if checked(generate) {
                    match random_text(16) {
                        Ok(p) => {
                            shown_password = Some(p.clone());
                            p
                        }
                        Err(e) => return dialog::error(owner, &e.to_string()),
                    }
                } else {
                    let (a, b) = (win::text(p1), win::text(p2));
                    if a.is_empty() || a != b {
                        return fail("password-mismatch");
                    }
                    a
                };
                match PasswordHash::new(&password) {
                    Ok(hash) => LockKind::Password { hash },
                    Err(e) => return dialog::error(owner, &e.to_string()),
                }
            }
            3 => LockKind::RandomText { length: number(length).clamp(20, 5000) as u16 },
            4 => LockKind::UntilRestart { boot_id: String::new() },
            5 => match picked() {
                Some(w) => LockKind::Schedule { schedule: Schedule { weekly: vec![w], dates: vec![] } },
                None => return fail("schedule-no-days"),
            },
            _ => {
                let area = GeoArea::Named { name: win::text(place).trim().to_string() };
                if area.validate().is_err() {
                    return fail("location-name");
                }
                if !checked(yes) && !checked(no) {
                    return fail("location-here-unanswered");
                }
                LockKind::Location { area }
            }
        };
        let learn = matches!(lock_kind, LockKind::Location { .. }) && checked(yes);
        let timer = matches!(lock_kind, LockKind::Until { .. });
        let unlock_delay_secs = if matches!(chosen, 2 | 3) { number(delay).min(24 * 60) * 60 } else { 0 };
        let lock = Lock { kind: lock_kind, unlock_delay_secs };
        let mut detail = ta("lock-confirm-detail", &[("lock", describe_lock(&lock).into())]);
        if timer {
            detail.push(' ');
            detail.push_str(&t("lock-confirm-timer"));
        }
        if ui::has_unsaved_changes() {
            detail.push_str("\n\n");
            detail.push_str(&t("lock-saves-changes"));
        }
        let answer = dialog::ask(
            owner,
            Some(dialog::Icon::Warning),
            &t("lock-confirm"),
            &detail,
            &[(1, t("action-cancel")), (2, t("action-lock"))],
        );
        if answer != Some(2) {
            return;
        }
        let mut id = None;
        if !ui::commit(owner, |s| {
            id = Some(s.id);
            s.lock = Some(lock);
        }) {
            return;
        }
        closer.close();
        if let (true, Some(id)) = (learn, id) {
            learn_place(ui::main_window(), id);
            ui::refresh(true);
        }
        if let Some(p) = &shown_password {
            dialog::message(
                ui::main_window(),
                dialog::Icon::Info,
                &t("password-generated"),
                &ta("password-generated-detail", &[("password", p.clone().into())]),
            );
        }
    });
}

/// Asks the service how the lock opens early, and asks for the answer.
pub fn start_unlock(id: SetId) {
    let owner = ui::main_window();
    let challenge = match ui::call(Request::UnlockStart { id }) {
        Ok(Response::Challenge { challenge }) => challenge,
        Ok(_) => return dialog::error(owner, &t("error-unexpected")),
        Err(e) => return dialog::error(owner, &e),
    };
    match challenge {
        ChallengeInfo::NotLocked => ui::refresh(true),
        ChallengeInfo::NotPossible => {
            dialog::message(owner, dialog::Icon::Info, &t("unlock-not-possible"), &t("unlock-not-possible-detail"))
        }
        ChallengeInfo::Password => answer_dialog(id, None),
        ChallengeInfo::RandomText { text } => answer_dialog(id, Some(text)),
    }
}

/// Asks for the password, or for the random text to be typed (pasting is
/// blocked; the text cannot be selected or copied).
fn answer_dialog(id: SetId, text: Option<String>) {
    let owner = ui::main_window();
    let dpi = panel::dpi();
    // A fixed-width font makes the random text easier to copy by eye.
    let mono = win::wide("Consolas");
    // SAFETY: a font for this dialog, deleted after it closed.
    let font = unsafe {
        CreateFontW(-win::scale(16, dpi), 0, 0, 0, FW_NORMAL as i32, 0, 0, 0, 0, 0, 0, 0, 0, mono.as_ptr())
    };
    let is_text = text.is_some();
    modal::run(owner, &t("unlock-title"), (620, if is_text { 520 } else { 220 }), |page, closer| {
        let mut items = Vec::new();
        let entry = match &text {
            Some(text) => {
                items.push(Item::Text(page.control("STATIC", &t("unlock-type-text"), 0x2000, 0)));
                let shown = page.control("STATIC", text, 0x2000, 0);
                win::send(shown, WM_SETFONT, font as usize, 1);
                items.push(Item::Text(shown));
                let e = page.control("EDIT", "", WS_TABSTOP | ES_AUTOHSCROLL as u32, WS_EX_CLIENTEDGE);
                win::send(e, WM_SETFONT, font as usize, 1);
                // SAFETY: our edit box; the procedure lives as long as the program.
                unsafe { SetWindowSubclass(e, Some(no_paste), 1, 0) };
                e
            }
            None => {
                items.push(Item::Text(page.control("STATIC", &t("unlock-enter-password"), 0x2000, 0)));
                page.control("EDIT", "", WS_TABSTOP | (ES_PASSWORD | ES_AUTOHSCROLL) as u32, WS_EX_CLIENTEDGE)
            }
        };
        items.push(Item::Full { hwnd: entry, height: panel::ROW });
        let spacer = page.control("STATIC", "", 0, 0);
        let cancel = page.control("BUTTON", &t("action-cancel"), WS_TABSTOP, 0);
        let ok = page.control("BUTTON", &t("action-unlock"), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
        items.push(Item::Space(8));
        items.push(Item::Row {
            cells: vec![(spacer, Width::Flex), (cancel, button_width(&t("action-cancel"))), (ok, button_width(&t("action-unlock")))],
            height: panel::ROW,
        });
        page.set_items(items);
        // SAFETY: our edit box.
        unsafe { SetFocus(entry) };
        {
            let closer = closer.clone();
            panel::on(cancel, move |code| {
                if code == BN_CLICKED {
                    closer.close();
                }
            });
        }
        panel::on(ok, move |code| {
            if code != BN_CLICKED {
                return;
            }
            let owner = closer.hwnd();
            let answer = win::text(entry);
            match ui::call(Request::UnlockAnswer { id, answer }) {
                Ok(Response::Unlock { outcome }) => {
                    closer.close();
                    let main = ui::main_window();
                    match outcome {
                        UnlockInfo::Unlocked => dialog::message(main, dialog::Icon::Info, &t("unlocked"), &t("unlocked-detail")),
                        UnlockInfo::Pending { at } => dialog::message(
                            main,
                            dialog::Icon::Info,
                            &t("unlock-pending"),
                            &ta("lock-pending", &[("time", fmt_time(at).into())]),
                        ),
                    }
                    ui::refresh(true);
                }
                Ok(_) => dialog::error(owner, &t("error-unexpected")),
                // Say which: the password, or the text.
                Err(e) if e == t("error-wrong-answer") => {
                    dialog::error(owner, &t(if is_text { "error-wrong-text" } else { "error-wrong-password" }))
                }
                Err(e) => dialog::error(owner, &e),
            }
        });
    });
    // SAFETY: the dialog that used the font is gone.
    unsafe { DeleteObject(font) };
}

/// Keeps text from being pasted into the random-text box (Ctrl+V,
/// Shift+Insert, or the right-click menu).
unsafe extern "system" fn no_paste(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    match msg {
        WM_PASTE | WM_CONTEXTMENU => 0,
        // SAFETY: everything else goes on to the edit box.
        _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
    }
}
