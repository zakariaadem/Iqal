//! The editor for one block: name and switches on top, then tabs, and a
//! save bar while there are unsaved changes.
//!
//! Controls that already show their new value use `Ui::edit`; changes that
//! show or hide parts of the editor use `Ui::modify`, which redraws it.

use std::rc::Rc;

use iqal_core::import::parse_rules_text;
use iqal_core::partial;
use iqal_core::schedule::{DateRange, Day, Schedule, WeeklyRange};
use iqal_core::{Allowance, BlockSet, ListMode, UserScope};
use iqal_i18n::{t, ta};
use iqal_ipc::{Request, Response, SetInfo};
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::ui::{summary, ListKey, Ui};
use crate::rules::rule_editor;
use crate::widgets::{describe_schedule, day_name, heading, hbox, label, note, page, rule_list, setting_row, vbox};
use crate::{appspicker, dialogs, lockui};

pub fn build(ui: &Rc<Ui>, info: &SetInfo) -> gtk::Widget {
    let root = vbox(0);
    root.append(&header(ui, info));
    if !ui.is_admin() {
        let n = note(&t("status-not-admin"));
        n.set_margin_start(18);
        n.set_margin_bottom(6);
        root.append(&n);
    }
    if info.locked {
        let banner = gtk::Label::new(Some(&t("locked-banner")));
        banner.add_css_class("warning");
        banner.set_wrap(true);
        banner.set_margin_start(18);
        banner.set_margin_end(18);
        banner.set_margin_bottom(6);
        banner.set_xalign(0.0);
        root.append(&banner);
    }
    let tabs = gtk::Notebook::new();
    tabs.set_vexpand(true);
    let pages: [(&str, gtk::Widget); 5] = [
        ("tab-websites", websites(ui, &info.set)),
        ("tab-apps", apps(ui, &info.set)),
        ("tab-users", users(ui, &info.set)),
        ("tab-schedule", schedule(ui, info)),
        ("tab-lock", lockui::page(ui, info)),
    ];
    for (key, widget) in pages {
        tabs.append_page(&widget, Some(&gtk::Label::new(Some(&t(key)))));
    }
    root.append(&tabs);
    let bar = save_bar(ui);
    root.append(&bar);
    ui.editor_built(&tabs, &bar);
    root.upcast()
}

/// Shown while there are unsaved changes.
fn save_bar(ui: &Rc<Ui>) -> gtk::Revealer {
    let bar = gtk::ActionBar::new();
    let text = gtk::Label::new(Some(&t("unsaved-changes")));
    text.add_css_class("heading");
    bar.pack_start(&text);
    let save = gtk::Button::with_label(&t("action-save"));
    save.add_css_class("suggested-action");
    save.set_tooltip_text(Some("Ctrl+S"));
    let weak = ui.weak();
    save.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            glib::idle_add_local_once(move || {
                ui.save();
            });
        }
    });
    let discard = gtk::Button::with_label(&t("action-discard"));
    let weak = ui.weak();
    discard.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            glib::idle_add_local_once(move || ui.discard());
        }
    });
    bar.pack_end(&save);
    bar.pack_end(&discard);
    gtk::Revealer::builder().child(&bar).transition_type(gtk::RevealerTransitionType::SlideUp).build()
}

fn header(ui: &Rc<Ui>, info: &SetInfo) -> gtk::Box {
    let bar = hbox(12);
    crate::widgets::set_margins(&bar, 14);
    let name = gtk::Entry::builder().text(&info.set.name).hexpand(true).build();
    name.add_css_class("title-3");
    let weak = ui.weak();
    name.connect_changed(move |e| {
        let text = e.text().trim().to_string();
        if let (Some(ui), false) = (weak.upgrade(), text.is_empty()) {
            ui.edit(|s| s.name = text);
        }
    });
    bar.append(&name);

    let counts = note(&summary(info));
    counts.set_wrap(false);
    bar.append(&counts);

    let enabled = gtk::Switch::builder().active(info.set.enabled).valign(gtk::Align::Center).build();
    enabled.set_tooltip_text(Some(&t("block-enabled")));
    let weak = ui.weak();
    enabled.connect_state_set(move |_, on| {
        if let Some(ui) = weak.upgrade() {
            ui.edit(|s| s.enabled = on);
        }
        glib::Propagation::Proceed
    });
    bar.append(&enabled);

    let export = gtk::Button::from_icon_name("document-save-symbolic");
    export.set_tooltip_text(Some(&t("action-export-block")));
    let weak = ui.weak();
    let id = info.set.id;
    export.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.after_unsaved(move |ui| dialogs::export(ui, Some(vec![id])));
        }
    });
    bar.append(&export);

    let delete = gtk::Button::from_icon_name("user-trash-symbolic");
    delete.set_tooltip_text(Some(&t("action-delete-block")));
    delete.set_sensitive(!info.locked);
    let weak = ui.weak();
    let name_text = info.set.name.clone();
    delete.connect_clicked(move |_| {
        let Some(ui) = weak.upgrade() else { return };
        let weak = ui.weak();
        ui.confirm(
            &ta("confirm-delete", &[("name", name_text.clone().into())]),
            &t("confirm-delete-detail"),
            &t("action-delete"),
            move || {
                if let Some(ui) = weak.upgrade() {
                    if let Err(e) = ui.service.remove(id) {
                        ui.error(&e);
                    }
                    ui.refresh(true);
                }
            },
        );
    });
    bar.append(&delete);
    bar
}

fn mode_dropdown(ui: &Rc<Ui>, current: ListMode, labels: [String; 2], apply: fn(&mut BlockSet, ListMode)) -> gtk::DropDown {
    let strings: Vec<&str> = labels.iter().map(String::as_str).collect();
    let dd = gtk::DropDown::from_strings(&strings);
    dd.set_selected(if current == ListMode::AllowOnly { 1 } else { 0 });
    let weak = ui.weak();
    dd.connect_selected_notify(move |d| {
        let mode = if d.selected() == 1 { ListMode::AllowOnly } else { ListMode::Block };
        if mode != current
            && let Some(ui) = weak.upgrade()
        {
            // The list titles change with the mode: redraw.
            glib::idle_add_local_once(move || {
                ui.modify(|s| {
                    apply(s, mode);
                    Ok(())
                })
            });
        }
    });
    dd
}

/// A switch that changes the draft; `redraw` when it shows or hides parts.
fn switch(ui: &Rc<Ui>, on: bool, redraw: bool, apply: impl Fn(&mut BlockSet, bool) + Clone + 'static) -> gtk::Switch {
    let sw = gtk::Switch::builder().active(on).build();
    let weak = ui.weak();
    sw.connect_state_set(move |_, on| {
        let Some(ui) = weak.upgrade() else { return glib::Propagation::Proceed };
        let apply = apply.clone();
        if redraw {
            glib::idle_add_local_once(move || {
                ui.modify(|s| {
                    apply(s, on);
                    Ok(())
                })
            });
            glib::Propagation::Stop
        } else {
            ui.edit(|s| apply(s, on));
            glib::Propagation::Proceed
        }
    });
    sw
}

/// A check button that adds or removes `id` in a list of the draft.
fn list_check(ui: &Rc<Ui>, text: &str, active: bool, id: String, list: fn(&mut BlockSet) -> &mut Vec<String>) -> gtk::CheckButton {
    let check = gtk::CheckButton::with_label(text);
    check.set_active(active);
    let weak = ui.weak();
    check.connect_toggled(move |c| {
        let on = c.is_active();
        if let Some(ui) = weak.upgrade() {
            ui.edit(|s| {
                let l = list(s);
                l.retain(|x| *x != id);
                if on {
                    l.push(id.clone());
                }
            });
        }
    });
    check
}

/// "Block list" or "Allow list", at the start of the tab.
fn mode_row(dropdown: &gtk::DropDown) -> gtk::DropDown {
    dropdown.set_halign(gtk::Align::Start);
    dropdown.clone()
}

fn websites(ui: &Rc<Ui>, set: &BlockSet) -> gtk::Widget {
    let (scroll, body) = page();
    let mode = mode_dropdown(ui, set.web.mode, [t("web-mode-block"), t("web-mode-allow-only")], |s, m| s.web.mode = m);
    body.append(&mode_row(&mode));

    let allow_only = set.web.mode == ListMode::AllowOnly;
    let examples = t("web-placeholder");
    if allow_only {
        // An allow list stands on its own.
        body.append(&rule_editor(ui, ListKey::WebAllow, &t("add-websites"), &t("web-allowed"), &examples).widget);
        body.append(&note(&t("web-syntax")));
    } else {
        let blocked = rule_editor(ui, ListKey::WebBlock, &t("add-websites"), &t("web-blocked"), &examples);
        let import = gtk::Button::with_label(&t("action-import-text"));
        let (weak, add) = (ui.weak(), blocked.add.clone());
        import.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                import_text(&ui, add.clone());
            }
        });
        blocked.actions.append(&import);
        body.append(&blocked.widget);
        body.append(&note(&t("web-syntax")));
        body.append(&rule_editor(ui, ListKey::WebAllow, &t("add-exceptions"), &t("web-exceptions"), &examples).widget);
    }

    let safe = switch(ui, set.web.safe_search, false, |s, on| s.web.safe_search = on);
    body.append(&setting_row(&t("web-safesearch"), &safe));

    // Pre-made lists are block lists: not used with an allow list.
    let catalog = match ui.service.call(Request::Catalog) {
        Ok(Response::Catalog { lists }) if !allow_only => lists,
        _ => Vec::new(),
    };
    if !catalog.is_empty() {
        body.append(&heading(&t("web-lists")));
        body.append(&note(&t("web-lists-detail")));
    }
    for list in catalog {
        let text = format!("{}  ·  {}", t(&format!("list-{}", list.id)), ta("list-count", &[("count", (list.rules as i64).into())]));
        let active = set.web.lists.contains(&list.id);
        body.append(&list_check(ui, &text, active, list.id, |s| &mut s.web.lists));
    }

    body.append(&heading(&t("web-partial")));
    body.append(&note(&t("web-partial-detail")));
    for rule in partial::BUILTIN {
        let active = set.web.partial.iter().any(|p| p == rule.id);
        body.append(&list_check(ui, &t(&format!("partial-{}", rule.id)), active, rule.id.to_string(), |s| &mut s.web.partial));
    }
    scroll.upcast()
}

/// Adds the rules of a text file (a list, or a hosts file) to the blocked
/// websites.
fn import_text(ui: &Rc<Ui>, add: Rc<dyn Fn(Vec<String>) -> usize>) {
    let dialog = gtk::FileDialog::builder().title(t("action-import-text")).modal(true).build();
    let weak = ui.weak();
    dialog.open(Some(&ui.window), None::<&gio::Cancellable>, move |res| {
        let (Some(ui), Ok(file)) = (weak.upgrade(), res) else { return };
        let Some(path) = file.path() else { return };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => return ui.error(&e.to_string()),
        };
        let parsed = parse_rules_text(&text);
        let skipped = parsed.errors.len();
        let lines: Vec<String> = parsed.rules.iter().map(ToString::to_string).collect();
        let added = add(lines);
        ui.info(
            &t("import-done"),
            &ta("import-result", &[("added", (added as i64).into()), ("skipped", (skipped as i64).into())]),
        );
    });
}

fn apps(ui: &Rc<Ui>, set: &BlockSet) -> gtk::Widget {
    let (scroll, body) = page();
    let mode = mode_dropdown(ui, set.apps.mode, [t("apps-mode-block"), t("apps-mode-allow-only")], |s, m| s.apps.mode = m);
    body.append(&mode_row(&mode));

    let examples = t("apps-placeholder");
    // An allow list stands on its own; a block list has exceptions.
    let lists: &[(ListKey, &str, &str)] = match set.apps.mode {
        ListMode::AllowOnly => &[(ListKey::AppAllow, "add-apps", "apps-allowed")],
        ListMode::Block => &[(ListKey::AppBlock, "add-apps", "apps-blocked"), (ListKey::AppAllow, "add-exceptions", "apps-exceptions")],
    };
    for &(key, add_title, title) in lists {
        let editor = rule_editor(ui, key, &t(add_title), &t(title), &examples);
        let installed = gtk::Button::with_label(&t("action-add-installed-app"));
        let (weak, add) = (ui.weak(), editor.add.clone());
        installed.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                let add = add.clone();
                appspicker::pick(&ui, move |exe| {
                    add(vec![exe]);
                });
            }
        });
        let file = gtk::Button::with_label(&t("action-add-program-file"));
        let (weak, add) = (ui.weak(), editor.add.clone());
        file.connect_clicked(move |_| {
            let Some(ui) = weak.upgrade() else { return };
            let dialog = gtk::FileDialog::builder().title(t("action-add-program-file")).modal(true).build();
            let add = add.clone();
            dialog.open(Some(&ui.window), None::<&gio::Cancellable>, move |res| {
                if let Ok(f) = res
                    && let Some(p) = f.path()
                {
                    add(vec![format!("path:{}", p.display())]);
                }
            });
        });
        // Under Add: the two other ways to add an app.
        let more = vbox(6);
        more.set_halign(gtk::Align::Start);
        more.append(&installed);
        more.append(&file);
        let column = editor.actions.parent().and_downcast::<gtk::Box>();
        if let Some(column) = column {
            column.append(&more);
        }
        body.append(&editor.widget);
    }
    body.append(&note(&t("apps-syntax")));
    scroll.upcast()
}

/// People's accounts on this computer (uid 1000–60000).
fn system_users() -> Vec<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    passwd
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            let uid: u32 = f.get(2)?.parse().ok()?;
            (1000..=60000).contains(&uid).then(|| f[0].to_string())
        })
        .collect()
}

fn users(ui: &Rc<Ui>, set: &BlockSet) -> gtk::Widget {
    let (scroll, body) = page();
    body.append(&heading(&t("users-applies-to")));
    let (kind, names) = match &set.users {
        UserScope::All => (0, Vec::new()),
        UserScope::AllExcept(n) => (1, n.clone()),
        UserScope::Only(n) => (2, n.clone()),
    };
    let radios = [t("users-all"), t("users-all-except"), t("users-only")].map(|l| gtk::CheckButton::with_label(&l));
    for (i, r) in radios.iter().enumerate() {
        if i > 0 {
            r.set_group(Some(&radios[0]));
        }
        r.set_active(i == kind);
        body.append(r);
    }
    for (i, r) in radios.iter().enumerate() {
        let weak = ui.weak();
        let names = names.clone();
        r.connect_toggled(move |r| {
            if !r.is_active() || i == kind {
                return;
            }
            let names = names.clone();
            if let Some(ui) = weak.upgrade() {
                glib::idle_add_local_once(move || {
                    ui.modify(|s| {
                        s.users = match i {
                            0 => UserScope::All,
                            1 => UserScope::AllExcept(names),
                            _ => UserScope::Only(names),
                        };
                        Ok(())
                    })
                });
            }
        });
    }
    if kind != 0 {
        body.append(&heading(&t(if kind == 1 { "users-except-these" } else { "users-only-these" })));
        let mut all = system_users();
        for n in &names {
            if !all.contains(n) {
                all.push(n.clone());
            }
        }
        for user in all {
            let check = gtk::CheckButton::with_label(&user);
            check.set_active(names.contains(&user));
            let weak = ui.weak();
            check.connect_toggled(move |c| {
                let on = c.is_active();
                if let Some(ui) = weak.upgrade() {
                    ui.edit(|s| {
                        if let UserScope::AllExcept(list) | UserScope::Only(list) = &mut s.users {
                            list.retain(|n| *n != user);
                            if on {
                                list.push(user.clone());
                            }
                        }
                    });
                }
            });
            body.append(&check);
        }
    }
    body.append(&note(&t("users-note")));
    scroll.upcast()
}

/// Day toggles and from/to time controls; returns (container, getter).
pub fn weekly_editor() -> (gtk::Box, impl Fn() -> Option<WeeklyRange>) {
    let b = vbox(8);
    let days_row = hbox(4);
    let toggles: Vec<(Day, gtk::ToggleButton)> = Day::ALL
        .iter()
        .map(|d| {
            let tb = gtk::ToggleButton::with_label(&day_name(*d));
            tb.set_active(!matches!(d, Day::Sat | Day::Sun));
            days_row.append(&tb);
            (*d, tb)
        })
        .collect();
    b.append(&days_row);
    let times = hbox(6);
    let spin = |max: f64, v: f64| {
        let s = gtk::SpinButton::with_range(0.0, max, 1.0);
        s.set_value(v);
        s.set_wrap(true);
        s.set_width_chars(2);
        s
    };
    let (fh, fm, th, tm) = (spin(23.0, 9.0), spin(59.0, 0.0), spin(24.0, 17.0), spin(59.0, 0.0));
    times.append(&label(&t("time-from")));
    times.append(&fh);
    times.append(&gtk::Label::new(Some(":")));
    times.append(&fm);
    times.append(&label(&t("time-to")));
    times.append(&th);
    times.append(&gtk::Label::new(Some(":")));
    times.append(&tm);
    b.append(&times);
    let get = move || {
        let days: Vec<Day> = toggles.iter().filter(|(_, tb)| tb.is_active()).map(|(d, _)| *d).collect();
        if days.is_empty() {
            return None;
        }
        let start = (fh.value() as u16) * 60 + fm.value() as u16;
        let end = ((th.value() as u16) * 60 + tm.value() as u16).min(1440);
        Some(WeeklyRange { days, start, end })
    };
    (b, get)
}

fn schedule(ui: &Rc<Ui>, info: &SetInfo) -> gtk::Widget {
    let set = &info.set;
    let (scroll, body) = page();
    let limited = switch(ui, set.schedule.is_some(), true, |s, on| s.schedule = on.then(Schedule::default));
    body.append(&setting_row(&t("schedule-limited"), &limited));
    body.append(&note(&t(if set.schedule.is_some() { "schedule-limited-detail" } else { "schedule-always" })));

    if let Some(schedule) = &set.schedule {
        let lines = describe_schedule(schedule);
        let weak = ui.weak();
        let weak2 = ui.weak();
        let sched = schedule.clone();
        body.append(&rule_list(
            &t("schedule-times"),
            lines,
            &t("schedule-date-placeholder"),
            move |text| {
                // "2026-10-20 08:00 2026-10-22 18:00": a one-off date range.
                let Some(ui) = weak.upgrade() else { return };
                let parts: Vec<&str> = text.split_whitespace().collect();
                let parse = |d: &str, h: &str| format!("{d}T{h}").parse::<jiff::civil::DateTime>();
                match parts.as_slice() {
                    [d1, h1, d2, h2] => match (parse(d1, h1), parse(d2, h2)) {
                        (Ok(start), Ok(end)) if start < end => ui.modify(|s| {
                            s.schedule.get_or_insert_with(Schedule::default).dates.push(DateRange { start, end });
                            Ok(())
                        }),
                        _ => ui.error(&t("schedule-date-invalid")),
                    },
                    _ => ui.error(&t("schedule-date-invalid")),
                }
            },
            move |text| {
                let Some(ui) = weak2.upgrade() else { return };
                let idx = describe_schedule(&sched).iter().position(|l| *l == text);
                let weekly_len = sched.weekly.len();
                ui.modify(|s| {
                    if let (Some(i), Some(sc)) = (idx, s.schedule.as_mut()) {
                        if i < weekly_len {
                            sc.weekly.remove(i);
                        } else {
                            sc.dates.remove(i - weekly_len);
                        }
                    }
                    Ok(())
                });
            },
        ));
        body.append(&heading(&t("schedule-add-weekly")));
        let (editor, get) = weekly_editor();
        body.append(&editor);
        let add = gtk::Button::with_label(&t("action-add-times"));
        add.set_halign(gtk::Align::Start);
        let weak = ui.weak();
        add.connect_clicked(move |_| {
            let Some(ui) = weak.upgrade() else { return };
            match get() {
                Some(range) => ui.modify(|s| {
                    s.schedule.get_or_insert_with(Schedule::default).weekly.push(range);
                    Ok(())
                }),
                None => ui.error(&t("schedule-no-days")),
            }
        });
        body.append(&add);
    }

    body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let allowance =
        switch(ui, set.allowance.is_some(), true, |s, on| s.allowance = on.then_some(Allowance { daily_secs: 30 * 60 }));
    body.append(&setting_row(&t("allowance"), &allowance));
    body.append(&note(&t("allowance-detail")));
    if let Some(a) = set.allowance {
        let minutes = gtk::SpinButton::with_range(1.0, 24.0 * 60.0, 5.0);
        minutes.set_value((a.daily_secs / 60) as f64);
        let weak = ui.weak();
        minutes.connect_value_changed(move |m| {
            let v = m.value() as u32;
            if let Some(ui) = weak.upgrade() {
                ui.edit(|s| s.allowance = Some(Allowance { daily_secs: v * 60 }));
            }
        });
        body.append(&setting_row(&t("allowance-minutes"), &minutes));
        body.append(&label(&ta(
            "allowance-used-today",
            &[("used", ((info.used_today_secs / 60) as i64).into()), ("total", ((a.daily_secs / 60) as i64).into())],
        )));
    }
    scroll.upcast()
}
