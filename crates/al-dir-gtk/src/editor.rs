//! The editor for one block: name and switches on top, then tabs.

use std::rc::Rc;

use al_dir_core::import::parse_rules_text;
use al_dir_core::partial;
use al_dir_core::schedule::{DateRange, Day, Schedule, WeeklyRange};
use al_dir_core::{Allowance, AppRule, BlockSet, ListMode, UserScope, WebRule};
use al_dir_i18n::{t, ta};
use al_dir_ipc::SetInfo;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::ui::{summary, Ui};
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
        ("tab-schedule", schedule(ui, &info.set)),
        ("tab-lock", lockui::page(ui, info)),
    ];
    for (key, widget) in pages {
        tabs.append_page(&widget, Some(&gtk::Label::new(Some(&t(key)))));
    }
    // Developer aid (screenshots, testing): open a given tab.
    if let Some(n) = std::env::var("AL_DIR_GUI_TAB").ok().and_then(|v| v.parse::<u32>().ok()) {
        tabs.set_current_page(Some(n));
    }
    root.append(&tabs);
    root.upcast()
}

fn header(ui: &Rc<Ui>, info: &SetInfo) -> gtk::Box {
    let bar = hbox(12);
    crate::widgets::set_margins(&bar, 14);
    let name = gtk::Entry::builder().text(&info.set.name).hexpand(true).build();
    name.add_css_class("title-3");
    let weak = ui.weak();
    name.connect_activate(move |e| {
        let text = e.text().trim().to_string();
        if let (Some(ui), false) = (weak.upgrade(), text.is_empty()) {
            ui.modify(|s| {
                s.name = text;
                Ok(())
            });
        }
    });
    let weak = ui.weak();
    name.connect_has_focus_notify(move |e| {
        let text = e.text().trim().to_string();
        if let (Some(ui), false) = (weak.upgrade(), e.has_focus())
            && !text.is_empty()
            && ui.current().is_some_and(|c| c.set.name != text)
        {
            ui.modify(|s| {
                s.name = text;
                Ok(())
            });
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
            glib::idle_add_local_once(move || {
                ui.modify(|s| {
                    s.enabled = on;
                    Ok(())
                })
            });
        }
        glib::Propagation::Stop
    });
    bar.append(&enabled);

    let export = gtk::Button::from_icon_name("document-save-symbolic");
    export.set_tooltip_text(Some(&t("action-export-block")));
    let weak = ui.weak();
    let id = info.set.id;
    export.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            dialogs::export(&ui, Some(vec![id]));
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

fn invalid(input: &str, err: impl std::fmt::Display) -> String {
    ta("rule-invalid", &[("rule", input.to_string().into()), ("problem", err.to_string().into())])
}

fn websites(ui: &Rc<Ui>, set: &BlockSet) -> gtk::Widget {
    let (scroll, body) = page();
    let mode = mode_dropdown(
        ui,
        set.web.mode,
        [t("web-mode-block"), t("web-mode-allow-only")],
        |s, m| s.web.mode = m,
    );
    body.append(&setting_row(&t("web-mode"), &mode));

    let weak = ui.weak();
    let weak2 = ui.weak();
    body.append(&rule_list(
        &t(if set.web.mode == ListMode::AllowOnly { "web-also-blocked" } else { "web-blocked" }),
        set.web.block.iter().map(ToString::to_string).collect(),
        &t("web-placeholder"),
        move |text| {
            if let Some(ui) = weak.upgrade() {
                ui.modify(|s| {
                    let r = WebRule::parse(&text).map_err(|e| invalid(&text, e))?;
                    if !s.web.block.contains(&r) {
                        s.web.block.push(r);
                    }
                    Ok(())
                });
            }
        },
        move |text| {
            if let Some(ui) = weak2.upgrade() {
                ui.modify(|s| {
                    s.web.block.retain(|r| r.to_string() != text);
                    Ok(())
                });
            }
        },
    ));
    body.append(&note(&t("web-syntax")));

    let import = gtk::Button::with_label(&t("action-import-text"));
    import.set_halign(gtk::Align::Start);
    let weak = ui.weak();
    import.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            import_text(&ui);
        }
    });
    body.append(&import);

    let weak = ui.weak();
    let weak2 = ui.weak();
    body.append(&rule_list(
        &t(if set.web.mode == ListMode::AllowOnly { "web-allowed" } else { "web-exceptions" }),
        set.web.allow.iter().map(ToString::to_string).collect(),
        &t("web-placeholder"),
        move |text| {
            if let Some(ui) = weak.upgrade() {
                ui.modify(|s| {
                    let r = WebRule::parse(&text).map_err(|e| invalid(&text, e))?;
                    if !s.web.allow.contains(&r) {
                        s.web.allow.push(r);
                    }
                    Ok(())
                });
            }
        },
        move |text| {
            if let Some(ui) = weak2.upgrade() {
                ui.modify(|s| {
                    s.web.allow.retain(|r| r.to_string() != text);
                    Ok(())
                });
            }
        },
    ));

    let safe = gtk::Switch::builder().active(set.web.safe_search).build();
    let weak = ui.weak();
    safe.connect_state_set(move |_, on| {
        if let Some(ui) = weak.upgrade() {
            glib::idle_add_local_once(move || {
                ui.modify(|s| {
                    s.web.safe_search = on;
                    Ok(())
                })
            });
        }
        glib::Propagation::Stop
    });
    body.append(&setting_row(&t("web-safesearch"), &safe));

    body.append(&heading(&t("web-partial")));
    body.append(&note(&t("web-partial-detail")));
    for rule in partial::BUILTIN {
        let check = gtk::CheckButton::with_label(&t(&format!("partial-{}", rule.id)));
        check.set_active(set.web.partial.iter().any(|p| p == rule.id));
        let weak = ui.weak();
        let id = rule.id.to_string();
        check.connect_toggled(move |c| {
            let on = c.is_active();
            let id = id.clone();
            if let Some(ui) = weak.upgrade() {
                glib::idle_add_local_once(move || {
                    ui.modify(|s| {
                        s.web.partial.retain(|p| *p != id);
                        if on {
                            s.web.partial.push(id);
                        }
                        Ok(())
                    })
                });
            }
        });
        body.append(&check);
    }
    scroll.upcast()
}

fn import_text(ui: &Rc<Ui>) {
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
        let mut added = 0i64;
        ui.modify(|s| {
            for r in parsed.rules {
                if !s.web.block.contains(&r) {
                    s.web.block.push(r);
                    added += 1;
                }
            }
            Ok(())
        });
        ui.info(
            &t("import-done"),
            &ta("import-result", &[("added", added.into()), ("skipped", (skipped as i64).into())]),
        );
    });
}

fn apps(ui: &Rc<Ui>, set: &BlockSet) -> gtk::Widget {
    let (scroll, body) = page();
    let mode = mode_dropdown(
        ui,
        set.apps.mode,
        [t("apps-mode-block"), t("apps-mode-allow-only")],
        |s, m| s.apps.mode = m,
    );
    body.append(&setting_row(&t("apps-mode"), &mode));

    for allow in [false, true] {
        let list: Vec<String> =
            if allow { &set.apps.allow } else { &set.apps.block }.iter().map(ToString::to_string).collect();
        let title = match (allow, set.apps.mode) {
            (false, ListMode::Block) => "apps-blocked",
            (false, ListMode::AllowOnly) => "apps-also-blocked",
            (true, ListMode::Block) => "apps-exceptions",
            (true, ListMode::AllowOnly) => "apps-allowed",
        };
        let weak = ui.weak();
        let weak2 = ui.weak();
        body.append(&rule_list(
            &t(title),
            list,
            &t("apps-placeholder"),
            move |text| {
                if let Some(ui) = weak.upgrade() {
                    add_app_rule(&ui, &text, allow);
                }
            },
            move |text| {
                if let Some(ui) = weak2.upgrade() {
                    ui.modify(|s| {
                        let list = if allow { &mut s.apps.allow } else { &mut s.apps.block };
                        list.retain(|r| r.to_string() != text);
                        Ok(())
                    });
                }
            },
        ));
        let buttons = hbox(6);
        let installed = gtk::Button::with_label(&t("action-add-installed-app"));
        let weak = ui.weak();
        installed.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                let weak = ui.weak();
                appspicker::pick(&ui, move |exe| {
                    if let Some(ui) = weak.upgrade() {
                        add_app_rule(&ui, &exe, allow);
                    }
                });
            }
        });
        let file = gtk::Button::with_label(&t("action-add-program-file"));
        let weak = ui.weak();
        file.connect_clicked(move |_| {
            let Some(ui) = weak.upgrade() else { return };
            let dialog = gtk::FileDialog::builder().title(t("action-add-program-file")).modal(true).build();
            let weak = ui.weak();
            dialog.open(Some(&ui.window), None::<&gio::Cancellable>, move |res| {
                if let (Some(ui), Ok(f)) = (weak.upgrade(), res)
                    && let Some(p) = f.path()
                {
                    add_app_rule(&ui, &format!("path:{}", p.display()), allow);
                }
            });
        });
        buttons.append(&installed);
        buttons.append(&file);
        body.append(&buttons);
    }
    body.append(&note(&t("apps-syntax")));
    scroll.upcast()
}

fn add_app_rule(ui: &Rc<Ui>, text: &str, allow: bool) {
    let text = text.to_string();
    ui.modify(|s| {
        let r = AppRule::parse(&text).map_err(|e| invalid(&text, e))?;
        let list = if allow { &mut s.apps.allow } else { &mut s.apps.block };
        if !list.contains(&r) {
            list.push(r);
        }
        Ok(())
    });
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
                let user = user.clone();
                if let Some(ui) = weak.upgrade() {
                    glib::idle_add_local_once(move || {
                        ui.modify(|s| {
                            let list = match &mut s.users {
                                UserScope::AllExcept(n) | UserScope::Only(n) => n,
                                UserScope::All => return Ok(()),
                            };
                            list.retain(|n| *n != user);
                            if on {
                                list.push(user);
                            }
                            Ok(())
                        })
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

fn schedule(ui: &Rc<Ui>, set: &BlockSet) -> gtk::Widget {
    let (scroll, body) = page();
    let limited = gtk::Switch::builder().active(set.schedule.is_some()).build();
    let weak = ui.weak();
    limited.connect_state_set(move |_, on| {
        if let Some(ui) = weak.upgrade() {
            glib::idle_add_local_once(move || {
                ui.modify(|s| {
                    s.schedule = on.then(Schedule::default);
                    Ok(())
                })
            });
        }
        glib::Propagation::Stop
    });
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
    let allowance = gtk::Switch::builder().active(set.allowance.is_some()).build();
    let weak = ui.weak();
    allowance.connect_state_set(move |_, on| {
        if let Some(ui) = weak.upgrade() {
            glib::idle_add_local_once(move || {
                ui.modify(|s| {
                    s.allowance = on.then_some(Allowance { daily_secs: 30 * 60 });
                    Ok(())
                })
            });
        }
        glib::Propagation::Stop
    });
    body.append(&setting_row(&t("allowance"), &allowance));
    body.append(&note(&t("allowance-detail")));
    if let Some(a) = set.allowance {
        let minutes = gtk::SpinButton::with_range(1.0, 24.0 * 60.0, 5.0);
        minutes.set_value((a.daily_secs / 60) as f64);
        let weak = ui.weak();
        minutes.connect_value_changed(move |m| {
            let v = m.value() as u32;
            if let Some(ui) = weak.upgrade() {
                glib::timeout_add_local_once(std::time::Duration::from_millis(600), move || {
                    ui.modify(|s| {
                        s.allowance = Some(Allowance { daily_secs: v * 60 });
                        Ok(())
                    })
                });
            }
        });
        body.append(&setting_row(&t("allowance-minutes"), &minutes));
    }
    scroll.upcast()
}
