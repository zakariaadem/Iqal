//! The editor for one block: name and switches on top, then tabs, and a
//! save bar while there are unsaved changes.
//!
//! Controls that already show their new value use `Ui::edit`; changes that
//! show or hide parts of the editor use `Ui::modify`, which redraws it.

use std::rc::Rc;

use iqal_core::import::parse_rules_text;
use iqal_core::schedule::{Day, WeeklyRange};
use iqal_core::{Allowance, BlockSet, ListMode, UserScope};
use iqal_i18n::{t, ta};
use iqal_ipc::{Request, Response, SetInfo};
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::ui::{ListKey, Ui};
use crate::rules::{rule_editor, rule_editor_with_lists};
use crate::widgets::{day_name, heading, hbox, icon_button, label, note, page, vbox};
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
        let banner = note(&t("locked-banner"));
        banner.set_margin_start(18);
        banner.set_margin_end(18);
        banner.set_margin_bottom(6);
        root.append(&banner);
    }
    // Tabs with an icon and a word (the daily allowance is on the Lock tab).
    let tabs = gtk::Notebook::new();
    tabs.set_vexpand(true);
    tabs.set_margin_start(6);
    tabs.set_margin_end(12);
    let pages: [(&str, &str, gtk::Widget); 4] = [
        ("tab-websites", "iqal-globe-symbolic", websites(ui, &info.set)),
        ("tab-apps", "view-list-symbolic", apps(ui, &info.set)),
        ("tab-users", "system-users-symbolic", users(ui, &info.set)),
        ("tab-lock", "changes-prevent-symbolic", lockui::page(ui, info)),
    ];
    for (key, icon, widget) in pages {
        let face = hbox(6);
        face.append(&gtk::Image::from_icon_name(icon));
        face.append(&gtk::Label::new(Some(&t(key))));
        tabs.append_page(&widget, Some(&face));
    }
    root.append(&tabs);
    let (bar, text, save) = save_bar(ui);
    root.append(&bar);
    ui.editor_built(&tabs, &bar, &text, &save);
    root.upcast()
}

/// Shown while there are unsaved changes: what is unsaved, Discard, Save.
fn save_bar(ui: &Rc<Ui>) -> (gtk::Revealer, gtk::Label, gtk::Button) {
    let bar = hbox(8);
    bar.add_css_class("savebar");
    bar.set_margin_start(18);
    bar.set_margin_end(12);
    let text = gtk::Label::new(Some(&t("unsaved-changes")));
    text.set_xalign(0.0);
    text.set_hexpand(true);
    text.set_wrap(true);
    bar.append(&text);
    let discard = icon_button(&t("action-discard"), "edit-undo-symbolic", "");
    let weak = ui.weak();
    discard.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            glib::idle_add_local_once(move || ui.discard());
        }
    });
    let save = icon_button(&t("action-save"), "media-floppy-symbolic", "main");
    save.set_tooltip_text(Some("Ctrl+S"));
    let weak = ui.weak();
    save.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            glib::idle_add_local_once(move || {
                ui.save();
            });
        }
    });
    bar.append(&discard);
    bar.append(&save);
    let revealer = gtk::Revealer::builder().child(&bar).transition_type(gtk::RevealerTransitionType::None).build();
    (revealer, text, save)
}

/// Name, on or off, lock, export, delete: as on Windows.
fn header(ui: &Rc<Ui>, info: &SetInfo) -> gtk::Box {
    let bar = hbox(10);
    bar.set_margin_start(18);
    bar.set_margin_end(12);
    bar.set_margin_top(12);
    bar.set_margin_bottom(10);
    let name = gtk::Entry::builder().text(&info.set.name).width_chars(18).build();
    name.add_css_class("heading");
    // Enter saves (the save bar's Save is the window's default button).
    name.set_activates_default(true);
    let weak = ui.weak();
    name.connect_changed(move |e| {
        let text = e.text().trim().to_string();
        if let (Some(ui), false) = (weak.upgrade(), text.is_empty()) {
            ui.edit(|s| s.name = text);
        }
    });
    bar.append(&name);

    // On or off: a switch whose text says which.
    let state_text = |on: bool| t(if on { "block-switch-on" } else { "block-switch-off" });
    let enabled = gtk::Switch::builder().active(info.set.enabled).valign(gtk::Align::Center).build();
    let enabled_text = gtk::Label::new(Some(&state_text(info.set.enabled)));
    let weak = ui.weak();
    let label = enabled_text.clone();
    enabled.connect_state_set(move |_, on| {
        label.set_text(&state_text(on));
        if let Some(ui) = weak.upgrade() {
            ui.edit(|s| s.enabled = on);
        }
        glib::Propagation::Proceed
    });
    bar.append(&enabled);
    bar.append(&enabled_text);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    bar.append(&spacer);

    // Lock, or unlock while locked (the same as on the Lock tab).
    let (text, icon) = if info.locked { ("action-unlock", "changes-allow-symbolic") } else { ("action-lock", "changes-prevent-symbolic") };
    let lock = icon_button(&t(text), icon, "main");
    let weak = ui.weak();
    let (locked, id) = (info.locked, info.set.id);
    lock.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            if locked {
                lockui::start_unlock(&ui, id);
            } else {
                lockui::lock_dialog(&ui);
            }
        }
    });
    bar.append(&lock);

    let export = icon_button(&t("action-export-block"), "document-save-as-symbolic", "");
    let weak = ui.weak();
    let id = info.set.id;
    export.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.after_unsaved(move |ui| dialogs::export(ui, Some(vec![id])));
        }
    });
    bar.append(&export);

    let delete = icon_button(&t("action-delete-block"), "user-trash-symbolic", "quiet");
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

/// Shows the pre-made lists with these ids in the blocked websites.
type ShowLists = Rc<dyn Fn(&[String])>;

/// A check button that adds or removes a pre-made list in the draft; then
/// `shown` puts the chosen lists' websites in the blocked websites.
fn list_check(ui: &Rc<Ui>, text: &str, active: bool, id: String, shown: ShowLists) -> gtk::CheckButton {
    let check = gtk::CheckButton::with_label(text);
    check.set_active(active);
    let weak = ui.weak();
    check.connect_toggled(move |c| {
        let on = c.is_active();
        let Some(ui) = weak.upgrade() else { return };
        let mut chosen = Vec::new();
        ui.edit(|s| {
            s.web.lists.retain(|x| *x != id);
            if on {
                s.web.lists.push(id.clone());
            }
            chosen = s.web.lists.clone();
        });
        shown(&chosen);
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
    // Pre-made lists are block lists: not used with an allow list.
    let catalog = match ui.service.call(Request::Catalog) {
        Ok(Response::Catalog { lists }) if !allow_only => lists,
        _ => Vec::new(),
    };
    let examples = t("web-placeholder");
    // Ticking a pre-made list updates the blocked websites in place.
    let mut shown: ShowLists = Rc::new(|_: &[String]| {});
    if allow_only {
        // An allow list stands on its own.
        body.append(&rule_editor(ui, ListKey::WebAllow, &t("add-websites"), &t("web-allowed"), &examples).widget);
        body.append(&note(&t("web-syntax")));
    } else {
        // The chosen pre-made lists show in the blocked websites.
        let chosen: Vec<(String, Rc<Vec<String>>)> = catalog
            .iter()
            .filter(|l| set.web.lists.contains(&l.id))
            .map(|l| (t(&format!("list-{}", l.id)), ui.list_rules(l)))
            .collect();
        let blocked =
            rule_editor_with_lists(ui, ListKey::WebBlock, &t("add-websites"), &t("web-blocked"), &examples, &chosen);
        let import = gtk::Button::with_label(&t("action-import-text"));
        let (weak, add) = (ui.weak(), blocked.add.clone());
        import.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                import_text(&ui, add.clone());
            }
        });
        blocked.actions.append(&import);
        let (weak, catalog, set_lists) = (ui.weak(), catalog.clone(), blocked.set_lists.clone());
        shown = Rc::new(move |ids: &[String]| {
            let Some(ui) = weak.upgrade() else { return };
            let chosen: Vec<(String, Rc<Vec<String>>)> = catalog
                .iter()
                .filter(|l| ids.contains(&l.id))
                .map(|l| (t(&format!("list-{}", l.id)), ui.list_rules(l)))
                .collect();
            set_lists(&chosen);
        });
        body.append(&blocked.widget);
        body.append(&note(&t("web-syntax")));
        body.append(&rule_editor(ui, ListKey::WebAllow, &t("add-exceptions"), &t("web-exceptions"), &examples).widget);
    }

    // A tick box and what it does, as on Windows.
    let safe = gtk::CheckButton::with_label(&t("web-safesearch"));
    safe.set_active(set.web.safe_search);
    let weak = ui.weak();
    safe.connect_toggled(move |c| {
        let on = c.is_active();
        if let Some(ui) = weak.upgrade() {
            ui.edit(|s| s.web.safe_search = on);
        }
    });
    body.append(&safe);
    body.append(&note(&t("web-safesearch-detail")));

    if !catalog.is_empty() {
        body.append(&heading(&t("web-lists")));
        body.append(&note(&t("web-lists-detail")));
    }
    for list in catalog {
        let text = format!("{}  ·  {}", t(&format!("list-{}", list.id)), ta("list-count", &[("count", (list.rules as i64).into())]));
        let active = set.web.lists.contains(&list.id);
        body.append(&list_check(ui, &text, active, list.id, shown.clone()));
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
                appspicker::pick(&ui, move |rules| {
                    add(rules);
                });
            }
        });
        let title = gtk::Button::with_label(&t("action-add-window-title"));
        let (weak, add) = (ui.weak(), editor.add.clone());
        title.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                let add = add.clone();
                appspicker::pick_title(&ui, move |rule| {
                    add(vec![rule]);
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
        more.append(&title);
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


/// The Lock tab's daily allowance: each blocked website and app may be used
/// for some minutes a day. Also an old "only at certain times" schedule, if
/// the block still has one (times are now set with a lock): it can be
/// removed.
pub fn allowance(ui: &Rc<Ui>, info: &SetInfo) -> gtk::Box {
    let set = &info.set;
    let b = vbox(10);
    b.set_margin_top(16);
    if set.schedule.is_some() {
        b.append(&heading(&t("schedule-old")));
        b.append(&note(&t("schedule-old-detail")));
        let remove = icon_button(&t("schedule-old-remove"), "window-close-symbolic", "");
        remove.set_halign(gtk::Align::Start);
        let weak = ui.weak();
        remove.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                // Always in force: stricter, so allowed while locked too.
                glib::idle_add_local_once(move || {
                    ui.modify(|s| {
                        s.schedule = None;
                        Ok(())
                    })
                });
            }
        });
        b.append(&remove);
    }
    let check = gtk::CheckButton::with_label(&t("allowance"));
    check.set_active(set.allowance.is_some());
    let weak = ui.weak();
    check.connect_toggled(move |c| {
        let on = c.is_active();
        if let Some(ui) = weak.upgrade() {
            glib::idle_add_local_once(move || {
                ui.modify(|s| {
                    s.allowance = on.then_some(Allowance { daily_secs: 30 * 60 });
                    Ok(())
                })
            });
        }
    });
    b.append(&check);
    b.append(&note(&t("allowance-detail")));
    if let Some(a) = set.allowance {
        let minutes = gtk::SpinButton::with_range(1.0, 24.0 * 60.0, 5.0);
        minutes.set_value((a.daily_secs / 60) as f64);
        minutes.set_activates_default(true);
        let weak = ui.weak();
        minutes.connect_value_changed(move |m| {
            let v = m.value() as u32;
            if let Some(ui) = weak.upgrade() {
                ui.edit(|s| s.allowance = Some(Allowance { daily_secs: v * 60 }));
            }
        });
        let row = hbox(12);
        let text = label(&t("allowance-minutes"));
        text.set_wrap(false);
        row.append(&text);
        row.append(&minutes);
        b.append(&row);
        // Time used today, per website and app (the most used five).
        let total = (a.daily_secs / 60) as i64;
        let used: Vec<String> = info
            .used_items
            .iter()
            .take(5)
            .map(|(item, secs)| {
                ta("allowance-used-item", &[("item", item.clone().into()), ("used", ((secs / 60) as i64).into()), ("total", total.into())])
            })
            .collect();
        let text =
            if used.is_empty() { t("allowance-unused") } else { format!("{} {}", t("allowance-used-today-items"), used.join("   ·   ")) };
        b.append(&label(&text));
    }
    b
}
