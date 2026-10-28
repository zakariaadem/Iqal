//! The Lock tab: current lock, locking, unlocking.

use std::rc::Rc;

use iqal_core::lock::random_text;
use iqal_core::schedule::Schedule;
use iqal_core::{GeoArea, Lock, LockKind, PasswordHash};
use iqal_i18n::{t, ta};
use iqal_ipc::{ChallengeInfo, Request, Response, SetInfo, UnlockInfo};
use gtk::glib;
use gtk::prelude::*;

use crate::editor::weekly_editor;
use crate::ui::Ui;
use crate::widgets::{describe_lock, fmt_time, heading, hbox, label, note, opens_by_itself, set_margins, vbox};
use crate::widgets::page as page_body;

pub fn page(ui: &Rc<Ui>, info: &SetInfo) -> gtk::Widget {
    let (scroll, body) = page_body();
    let set = &info.set;
    match &set.lock {
        None => {
            body.append(&heading(&t("lock-none")));
            body.append(&note(&t("lock-none-detail")));
        }
        Some(lock) => {
            body.append(&heading(&t(if info.locked { "lock-active" } else { "lock-inactive" })));
            body.append(&label(&describe_lock(lock)));
            if let Some(at) = set.pending_unlock_at {
                body.append(&label(&ta("lock-pending", &[("time", fmt_time(at).into())])));
                let cancel = gtk::Button::with_label(&t("action-cancel-unlock"));
                cancel.set_halign(gtk::Align::Start);
                let weak = ui.weak();
                let id = set.id;
                cancel.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        if let Err(e) = ui.service.call(Request::CancelUnlock { id }) {
                            ui.error(&e);
                        }
                        ui.refresh(true);
                    }
                });
                body.append(&cancel);
            }
            if let LockKind::Location { .. } = lock.kind {
                // Until Iqal recognises the place: "I am here now". After
                // that, the same button adds where the computer is now
                // (another place, or the same one, recognised better).
                let (detail, action) = if info.place_known {
                    ("location-learn-more-detail", "action-learn-more")
                } else {
                    ("location-learn-detail", "action-learn-location")
                };
                body.append(&note(&t(detail)));
                let learn = gtk::Button::with_label(&t(action));
                learn.set_halign(gtk::Align::Start);
                let weak = ui.weak();
                let id = set.id;
                learn.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        learn_place(&ui, id);
                        ui.refresh(true);
                    }
                });
                body.append(&learn);
            }
            if info.locked {
                let opens = opens_by_itself(lock);
                // An unknown place stays locked everywhere (said above).
                let unknown_place = matches!(lock.kind, LockKind::Location { .. }) && !info.place_known;
                if let (Some(text), false) = (&opens, unknown_place) {
                    body.append(&note(text));
                }
                let unlock = gtk::Button::with_label(&t("action-unlock"));
                // Only a password or a random text opens a lock early.
                unlock.set_sensitive(opens.is_none());
                unlock.set_halign(gtk::Align::Start);
                let weak = ui.weak();
                let id = set.id;
                unlock.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        start_unlock(&ui, id);
                    }
                });
                body.append(&unlock);
            } else {
                let remove = gtk::Button::with_label(&t("action-remove-lock"));
                remove.set_halign(gtk::Align::Start);
                let weak = ui.weak();
                remove.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        ui.commit(|s| s.lock = None);
                    }
                });
                body.append(&remove);
            }
        }
    }
    if !info.locked {
        let text = t(if set.lock.is_some() { "action-change-lock" } else { "action-lock" });
        let lock = crate::widgets::icon_button(&text, "changes-prevent-symbolic", "main");
        lock.set_halign(gtk::Align::Start);
        let weak = ui.weak();
        lock.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                lock_dialog(&ui);
            }
        });
        body.append(&lock);
    }
    // The daily allowance (and an old time schedule, if the block has one).
    body.append(&crate::editor::allowance(ui, info));
    scroll.upcast()
}

/// Remembers the Wi-Fi networks around the computer for the block's
/// location lock, and says how many.
fn learn_place(ui: &Rc<Ui>, id: iqal_core::SetId) {
    match ui.service.call(Request::LearnLocation { id }) {
        Ok(Response::Learned { networks }) => {
            ui.info(&t("location-learned"), &ta("location-learned-detail", &[("count", (networks as i64).into())]))
        }
        Ok(_) => ui.error(&t("error-unexpected")),
        Err(e) => ui.error(&format!("{}\n{e}", t("location-learn-failed"))),
    }
}

const KINDS: [&str; 7] =
    ["lock-kind-duration", "lock-kind-until", "lock-kind-password", "lock-kind-random", "lock-kind-restart", "lock-kind-schedule", "lock-kind-location"];

fn spin(min: f64, max: f64, value: f64) -> gtk::SpinButton {
    let s = gtk::SpinButton::with_range(min, max, 1.0);
    s.set_value(value);
    s
}

pub fn lock_dialog(ui: &Rc<Ui>) {
    let win = gtk::Window::builder()
        .transient_for(&ui.window)
        .modal(true)
        .title(t("lock-dialog-title"))
        .default_width(520)
        .build();
    let body = vbox(12);
    set_margins(&body, 18);

    let kinds: Vec<String> = KINDS.iter().map(|k| t(k)).collect();
    let kind = gtk::DropDown::from_strings(&kinds.iter().map(String::as_str).collect::<Vec<_>>());
    kind.set_halign(gtk::Align::Start);
    body.append(&kind);
    let stack = gtk::Stack::new();
    stack.set_vhomogeneous(false);
    // Each kind as wide as it needs (not as wide as the widest).
    stack.set_hhomogeneous(false);
    stack.set_interpolate_size(false);

    // Duration
    let dur = hbox(6);
    let (days, hours, minutes) = (spin(0.0, 365.0, 0.0), spin(0.0, 23.0, 2.0), spin(0.0, 59.0, 0.0));
    for (w, key) in [(&days, "unit-days"), (&hours, "unit-hours"), (&minutes, "unit-minutes")] {
        dur.append(w);
        dur.append(&label(&t(key)));
    }
    stack.add_named(&dur, Some("0"));

    // Until
    let until = vbox(6);
    let calendar = gtk::Calendar::new();
    let until_time = hbox(6);
    let (uh, um) = (spin(0.0, 23.0, 18.0), spin(0.0, 59.0, 0.0));
    until_time.append(&label(&t("time-at")));
    until_time.append(&uh);
    until_time.append(&gtk::Label::new(Some(":")));
    until_time.append(&um);
    until.append(&calendar);
    until.append(&until_time);
    stack.add_named(&until, Some("1"));

    // Password
    let pw = vbox(6);
    let p1 = gtk::PasswordEntry::builder().placeholder_text(t("password")).show_peek_icon(true).build();
    let p2 = gtk::PasswordEntry::builder().placeholder_text(t("password-repeat")).show_peek_icon(true).build();
    let generate = gtk::CheckButton::with_label(&t("password-generate"));
    {
        // Iqal makes the password: nothing to type.
        let (p1, p2) = (p1.clone(), p2.clone());
        generate.connect_toggled(move |g| {
            p1.set_sensitive(!g.is_active());
            p2.set_sensitive(!g.is_active());
        });
    }
    pw.append(&p1);
    pw.append(&p2);
    pw.append(&generate);
    pw.append(&note(&t("password-generate-detail")));
    stack.add_named(&pw, Some("2"));

    // Random text
    let random = hbox(6);
    let length = spin(20.0, 5000.0, 200.0);
    random.append(&label(&t("random-length")));
    random.append(&length);
    stack.add_named(&random, Some("3"));

    // Restart
    stack.add_named(&note(&t("lock-restart-detail")), Some("4"));

    // During certain times: on and locked then, off and free to change the
    // rest of the time.
    let sched = vbox(8);
    sched.append(&note(&t("lock-schedule-detail")));
    let (sched_editor, get_weekly) = weekly_editor();
    sched.append(&sched_editor);
    stack.add_named(&sched, Some("5"));

    // While at a place: a name, and whether the computer is there now (as
    // on Windows; Iqal knows the place by its Wi-Fi networks).
    let loc = vbox(6);
    let name_row = hbox(8);
    name_row.append(&label(&t("location-name")));
    let place = gtk::Entry::builder().placeholder_text(t("location-name-placeholder")).hexpand(true).build();
    name_row.append(&place);
    loc.append(&name_row);
    loc.append(&note(&t("lock-location-detail-named")));
    // Iqal learns the place's Wi-Fi networks when the computer is there.
    // No answer is chosen in advance: a wrong "yes" would teach it the
    // networks of another place.
    let question = heading(&t("location-here-question"));
    question.set_margin_top(8);
    loc.append(&question);
    let here_yes = gtk::CheckButton::with_label(&t("location-here-yes"));
    let here_no = gtk::CheckButton::with_label(&t("location-here-no"));
    here_no.set_group(Some(&here_yes));
    loc.append(&here_yes);
    loc.append(&here_no);
    loc.append(&note(&t("location-here-detail")));
    stack.add_named(&loc, Some("6"));
    body.append(&stack);

    // A wait only applies to locks opened with a password or a text.
    let delay_box = vbox(6);
    let delay_row = hbox(6);
    let delay = spin(0.0, 24.0 * 60.0, 0.0);
    delay_row.append(&gtk::Label::new(Some(&t("unlock-delay"))));
    delay_row.append(&delay);
    delay_row.append(&label(&t("unit-minutes")));
    delay_box.append(&delay_row);
    delay_box.append(&note(&t("unlock-delay-detail")));
    delay_box.set_visible(false);
    body.append(&delay_box);

    {
        let (stack, delay_box) = (stack.clone(), delay_box.clone());
        kind.connect_selected_notify(move |k| {
            stack.set_visible_child_name(&k.selected().to_string());
            delay_box.set_visible(matches!(k.selected(), 2 | 3));
        });
    }

    let buttons = hbox(6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label(&t("action-cancel"));
    let ok = crate::widgets::icon_button(&t("action-lock"), "changes-prevent-symbolic", "main");
    buttons.append(&cancel);
    buttons.append(&ok);
    body.append(&buttons);
    win.set_child(Some(&body));
    win.set_default_widget(Some(&ok));

    {
        let win = win.clone();
        cancel.connect_clicked(move |_| win.close());
    }
    let weak = ui.weak();
    let win2 = win.clone();
    ok.connect_clicked(move |_| {
        let Some(ui) = weak.upgrade() else { return };
        let now = jiff::Timestamp::now().as_second();
        let mut shown_password = None;
        let lock_kind = match kind.selected() {
            0 => {
                let secs = days.value() as i64 * 86_400 + hours.value() as i64 * 3600 + minutes.value() as i64 * 60;
                if secs <= 0 {
                    return ui.error(&t("lock-duration-zero"));
                }
                LockKind::Until { until: now + secs }
            }
            1 => {
                let d = calendar.date();
                let dt = jiff::civil::date(d.year() as i16, d.month() as i8, d.day_of_month() as i8)
                    .at(uh.value() as i8, um.value() as i8, 0, 0);
                match dt.to_zoned(jiff::tz::TimeZone::system()) {
                    Ok(z) if z.timestamp().as_second() > now => LockKind::Until { until: z.timestamp().as_second() },
                    _ => return ui.error(&t("lock-until-past")),
                }
            }
            2 => {
                let password = if generate.is_active() {
                    match random_text(16) {
                        Ok(p) => {
                            shown_password = Some(p.clone());
                            p
                        }
                        Err(e) => return ui.error(&e.to_string()),
                    }
                } else {
                    let (a, b) = (p1.text().to_string(), p2.text().to_string());
                    if a.is_empty() || a != b {
                        return ui.error(&t("password-mismatch"));
                    }
                    a
                };
                match PasswordHash::new(&password) {
                    Ok(hash) => LockKind::Password { hash },
                    Err(e) => return ui.error(&e.to_string()),
                }
            }
            3 => LockKind::RandomText { length: length.value() as u16 },
            4 => LockKind::UntilRestart { boot_id: String::new() },
            5 => match get_weekly() {
                Some(w) => LockKind::Schedule { schedule: Schedule { weekly: vec![w], dates: vec![] } },
                None => return ui.error(&t("schedule-no-days")),
            },
            _ => {
                let area = GeoArea::Named { name: place.text().trim().to_string() };
                if area.validate().is_err() {
                    return ui.error(&t("location-name"));
                }
                if !here_yes.is_active() && !here_no.is_active() {
                    return ui.error(&t("location-here-unanswered"));
                }
                LockKind::Location { area }
            }
        };
        let learn = matches!(lock_kind, LockKind::Location { .. }) && here_yes.is_active();
        let timer = matches!(lock_kind, LockKind::Until { .. });
        let unlock_delay_secs = if matches!(kind.selected(), 2 | 3) { (delay.value() as u32) * 60 } else { 0 };
        let lock = Lock { kind: lock_kind, unlock_delay_secs };
        let description = describe_lock(&lock);
        let mut detail = ta("lock-confirm-detail", &[("lock", description.into())]);
        if timer {
            detail.push(' ');
            detail.push_str(&t("lock-confirm-timer"));
        }
        if ui.is_dirty() {
            detail.push_str("\n\n");
            detail.push_str(&t("lock-saves-changes"));
        }
        let weak = ui.weak();
        let win = win2.clone();
        ui.confirm(&t("lock-confirm"), &detail, &t("action-lock"), move || {
            let Some(ui) = weak.upgrade() else { return };
            let lock = lock.clone();
            let mut id = None;
            if !ui.commit(|s| {
                id = Some(s.id);
                // Locking turns the block on: a lock keeps it from being turned off.
                s.enabled = true;
                s.lock = Some(lock)
            }) {
                return;
            }
            win.close();
            if let (true, Some(id)) = (learn, id) {
                learn_place(&ui, id);
                ui.refresh(true);
            }
            if let Some(p) = &shown_password {
                ui.info(&t("password-generated"), &ta("password-generated-detail", &[("password", p.clone().into())]));
            }
        });
    });
    win.present();
}

pub fn start_unlock(ui: &Rc<Ui>, id: iqal_core::SetId) {
    let challenge = match ui.service.call(Request::UnlockStart { id }) {
        Ok(Response::Challenge { challenge }) => challenge,
        Ok(_) => return ui.error(&t("error-unexpected")),
        Err(e) => return ui.error(&e),
    };
    match challenge {
        ChallengeInfo::NotLocked => ui.refresh(true),
        ChallengeInfo::NotPossible => ui.info(&t("unlock-not-possible"), &t("unlock-not-possible-detail")),
        ChallengeInfo::Password => answer_dialog(ui, id, None),
        ChallengeInfo::RandomText { text } => answer_dialog(ui, id, Some(text)),
    }
}

/// Asks for the password, or for the random text to be typed (pasting is
/// disabled and the text cannot be selected or copied).
fn answer_dialog(ui: &Rc<Ui>, id: iqal_core::SetId, text: Option<String>) {
    let win = gtk::Window::builder().transient_for(&ui.window).modal(true).title(t("unlock-title")).default_width(560).build();
    let body = vbox(12);
    set_margins(&body, 18);
    let entry: gtk::Widget = match &text {
        Some(text) => {
            body.append(&label(&t("unlock-type-text")));
            let shown = gtk::Label::new(Some(text));
            shown.set_wrap(true);
            shown.set_wrap_mode(gtk::pango::WrapMode::Char);
            shown.set_selectable(false);
            shown.add_css_class("monospace");
            shown.set_xalign(0.0);
            body.append(&shown);
            let e = gtk::Entry::new();
            e.add_css_class("monospace");
            if let Some(inner) = e.delegate().and_downcast::<gtk::Text>() {
                inner.connect_paste_clipboard(|t| t.stop_signal_emission_by_name("paste-clipboard"));
            }
            e.upcast()
        }
        None => {
            body.append(&label(&t("unlock-enter-password")));
            gtk::PasswordEntry::builder().show_peek_icon(true).build().upcast()
        }
    };
    body.append(&entry);
    let buttons = hbox(6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label(&t("action-cancel"));
    let ok = crate::widgets::icon_button(&t("action-unlock"), "changes-allow-symbolic", "main");
    buttons.append(&cancel);
    buttons.append(&ok);
    body.append(&buttons);
    win.set_child(Some(&body));
    // Enter in the box presses Unlock.
    win.set_default_widget(Some(&ok));
    if let Some(e) = entry.downcast_ref::<gtk::Entry>() {
        e.set_activates_default(true);
    }
    if let Some(e) = entry.downcast_ref::<gtk::PasswordEntry>() {
        e.set_activates_default(true);
    }
    {
        let win = win.clone();
        cancel.connect_clicked(move |_| win.close());
    }
    let is_text = text.is_some();
    let weak = ui.weak();
    let win2 = win.clone();
    ok.connect_clicked(move |_| {
        let Some(ui) = weak.upgrade() else { return };
        let answer = entry.downcast_ref::<gtk::Editable>().map(|e| e.text().to_string()).unwrap_or_default();
        match ui.service.call(Request::UnlockAnswer { id, answer }) {
            Ok(Response::Unlock { outcome }) => {
                win2.close();
                // The main window shows the change at once, behind the message.
                ui.refresh(true);
                match outcome {
                    UnlockInfo::Unlocked => ui.info(&t("unlocked"), &t("unlocked-detail")),
                    UnlockInfo::Pending { at } => ui.info(&t("unlock-pending"), &ta("lock-pending", &[("time", fmt_time(at).into())])),
                }
            }
            Ok(_) => ui.error(&t("error-unexpected")),
            // Say which: the password, or the text.
            Err(e) if e == t("error-wrong-answer") => {
                ui.error(&t(if is_text { "error-wrong-text" } else { "error-wrong-password" }))
            }
            Err(e) => ui.error(&e),
        }
    });
    glib::idle_add_local_once(move || {
        win.present();
    });
}
