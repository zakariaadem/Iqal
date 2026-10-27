//! Dialogs reached from the menu and the block list.

use std::rc::Rc;

use iqal_core::{BlockSet, SetId};
use iqal_i18n::{t, ta, LANGUAGES};
use iqal_ipc::{EventKind, Request, Response};
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::settings::GuiSettings;
use crate::ui::Ui;
use crate::widgets::{fmt_time, hbox, heading, label, note, set_margins, setting_row, vbox};

fn small_window(ui: &Ui, title: &str, width: i32) -> (gtk::Window, gtk::Box) {
    let win = gtk::Window::builder().transient_for(&ui.window).modal(true).title(title).default_width(width).build();
    let body = vbox(12);
    set_margins(&body, 18);
    win.set_child(Some(&body));
    (win, body)
}

pub fn new_block(ui: &Rc<Ui>) {
    let (win, body) = small_window(ui, &t("action-new-block"), 380);
    body.append(&label(&t("new-block-name")));
    let entry = gtk::Entry::builder().placeholder_text(t("new-block-placeholder")).build();
    body.append(&entry);
    let buttons = hbox(6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label(&t("action-cancel"));
    let create = crate::widgets::icon_button(&t("action-create"), "list-add-symbolic", "main");
    win.set_default_widget(Some(&create));
    entry.set_activates_default(true);
    buttons.append(&cancel);
    buttons.append(&create);
    body.append(&buttons);
    {
        let win = win.clone();
        cancel.connect_clicked(move |_| win.close());
    }
    let submit = {
        let (weak, win, entry) = (ui.weak(), win.clone(), entry.clone());
        move || {
            let name = entry.text().trim().to_string();
            let Some(ui) = weak.upgrade() else { return };
            if name.is_empty() {
                return;
            }
            let set = BlockSet::new(name);
            let id = set.id;
            match ui.service.save(set) {
                Ok(()) => {
                    win.close();
                    ui.refresh(false);
                    ui.select(Some(id));
                }
                Err(e) => ui.error(&e),
            }
        }
    };
    create.connect_clicked(move |_| submit());
    win.present();
}

pub fn export_all(ui: &Rc<Ui>) {
    export(ui, None);
}

pub fn export(ui: &Rc<Ui>, ids: Option<Vec<SetId>>) {
    let json = match ui.service.call(Request::Export { ids }) {
        Ok(Response::Exported { json }) => json,
        Ok(_) => return ui.error(&t("error-unexpected")),
        Err(e) => return ui.error(&e),
    };
    let dialog = gtk::FileDialog::builder().title(t("menu-export-all")).initial_name("iqal-blocks.iqal").modal(true).build();
    let weak = ui.weak();
    dialog.save(Some(&ui.window), None::<&gio::Cancellable>, move |res| {
        let (Some(ui), Ok(file)) = (weak.upgrade(), res) else { return };
        let Some(path) = file.path() else { return };
        match std::fs::write(&path, &json) {
            Ok(()) => ui.info(&t("export-done"), &path.display().to_string()),
            Err(e) => ui.error(&e.to_string()),
        }
    });
}

pub fn import_blocks(ui: &Rc<Ui>) {
    let dialog = gtk::FileDialog::builder().title(t("menu-import")).modal(true).build();
    let weak = ui.weak();
    dialog.open(Some(&ui.window), None::<&gio::Cancellable>, move |res| {
        let (Some(ui), Ok(file)) = (weak.upgrade(), res) else { return };
        let Some(path) = file.path() else { return };
        let json = match std::fs::read_to_string(&path) {
            Ok(j) => j,
            Err(e) => return ui.error(&e.to_string()),
        };
        match ui.service.call(Request::Import { json }) {
            Ok(Response::Imported { added, replaced, rejected }) => {
                let mut lines = Vec::new();
                lines.extend(added.iter().map(|n| ta("import-added", &[("name", n.clone().into())])));
                lines.extend(replaced.iter().map(|n| ta("import-replaced", &[("name", n.clone().into())])));
                lines.extend(rejected.iter().map(|(n, why)| ta("import-rejected", &[("name", n.clone().into()), ("why", why.clone().into())])));
                ui.info(&t("import-done"), &lines.join("\n"));
                ui.refresh(true);
            }
            Ok(_) => ui.error(&t("error-unexpected")),
            Err(e) => ui.error(&e),
        }
    });
}

pub fn activity(ui: &Rc<Ui>) {
    let (win, body) = small_window(ui, &t("menu-activity"), 640);
    win.set_default_height(520);
    let events = match ui.service.call(Request::RecentEvents) {
        Ok(Response::Events { events }) => events,
        Ok(_) => Vec::new(),
        Err(e) => return ui.error(&e),
    };
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    if events.is_empty() {
        list.append(&note(&t("activity-empty")));
    }
    for e in events.iter().rev() {
        let row = vbox(2);
        set_margins(&row, 6);
        let what = t(match e.kind {
            EventKind::AppStart => "event-app-start",
            EventKind::AppKilled => "event-app-killed",
            EventKind::WindowClosed => "event-window-closed",
            EventKind::SiteBlocked => "event-site-blocked",
            EventKind::PageBlocked => "event-page-blocked",
            EventKind::AllowanceLow => "event-allowance-low",
            EventKind::AllowanceUsed => "event-allowance-used",
        });
        let top = label(&format!("{what}: {}", e.target));
        top.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        row.append(&top);
        row.append(&note(&format!("{} · {} · {} · {}", fmt_time(e.time), e.user, e.set_name, e.reason)));
        list.append(&row);
    }
    list.add_css_class("blocks");
    let scroll = gtk::ScrolledWindow::builder().vexpand(true).child(&list).build();
    body.append(&scroll);
    body.append(&close_row(&win));
    win.present();
}

pub fn settings(ui: &Rc<Ui>) {
    let (win, body) = small_window(ui, &t("menu-settings"), 560);
    body.append(&heading(&t("settings-language")));
    let mut names = vec![t("settings-language-system")];
    names.extend(LANGUAGES.iter().map(|(_, n, _)| n.to_string()));
    let dd = gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
    dd.set_halign(gtk::Align::Start);
    let saved = GuiSettings::load();
    let pos = saved.language.as_deref().and_then(|c| LANGUAGES.iter().position(|(code, _, _)| *code == c)).map_or(0, |i| i + 1);
    dd.set_selected(pos as u32);
    let weak = ui.weak();
    dd.connect_selected_notify(move |d| {
        let mut s = GuiSettings::load();
        s.language = match d.selected() {
            0 => None,
            i => LANGUAGES.get(i as usize - 1).map(|(c, _, _)| c.to_string()),
        };
        s.save();
        if let Some(ui) = weak.upgrade() {
            ui.info(&t("settings-language"), &t("settings-language-restart"));
        }
    });
    body.append(&dd);

    // Light or dark, at once.
    body.append(&heading(&t("settings-theme")));
    const THEMES: [Option<&str>; 3] = [None, Some("light"), Some("dark")];
    let themes = [t("settings-theme-desktop"), t("settings-theme-light"), t("settings-theme-dark")];
    let appearance = gtk::DropDown::from_strings(&themes.iter().map(String::as_str).collect::<Vec<_>>());
    appearance.set_halign(gtk::Align::Start);
    let pos = THEMES.iter().position(|th| *th == saved.theme.as_deref()).unwrap_or(0);
    appearance.set_selected(pos as u32);
    appearance.connect_selected_notify(|d| {
        let mut s = GuiSettings::load();
        s.theme = THEMES.get(d.selected() as usize).copied().flatten().map(String::from);
        s.save();
        crate::theme::apply();
    });
    body.append(&appearance);

    // Linux only: while locked, no administrator rights for the blocked.
    let status = ui.status();
    let hardened = gtk::Switch::builder().active(status.as_ref().is_some_and(|s| s.hardened)).build();
    let weak = ui.weak();
    hardened.connect_state_set(move |sw, on| {
        let Some(ui) = weak.upgrade() else { return glib::Propagation::Stop };
        match ui.service.call(Request::SetHardened { on }) {
            Ok(_) => sw.set_state(on),
            Err(e) => ui.error(&e),
        }
        ui.refresh(false);
        glib::Propagation::Stop
    });
    let row = setting_row(&t("settings-hardened"), &hardened);
    row.set_margin_top(6);
    body.append(&row);
    body.append(&note(&t("settings-hardened-detail")));

    if let Some(s) = status {
        body.append(&heading(&t("settings-status")));
        body.append(&note(&ta("status-service", &[("version", s.version.clone().into())])));
        body.append(&note(&ta("status-time", &[("source", iqal_app::describe::time_source(&s.clock).into())])));
        body.append(&note(&ta("status-location", &[("state", s.location.clone().into())])));
        let ext = if s.extensions.is_empty() { t("status-no-extensions") } else { s.extensions.join(", ") };
        body.append(&note(&ta("status-extensions", &[("list", ext.into())])));
        if !s.admin {
            body.append(&note(&t("status-not-admin")));
        }
    }
    body.append(&close_row(&win));
    win.present();
}

/// A Close button at the bottom right, the window's default button.
fn close_row(win: &gtk::Window) -> gtk::Box {
    let row = hbox(6);
    row.set_halign(gtk::Align::End);
    row.set_margin_top(8);
    let close = gtk::Button::with_label(&t("action-close"));
    let w = win.downgrade();
    close.connect_clicked(move |_| {
        if let Some(w) = w.upgrade() {
            w.close();
        }
    });
    win.set_default_widget(Some(&close));
    row.append(&close);
    row
}

pub fn about(ui: &Rc<Ui>) {
    let detail = format!(
        "{}\n\n{} {}\nGNU General Public License 3.0\nhttps://github.com/zakariaadem/Iqal",
        t("app-description"),
        t("app-name"),
        env!("CARGO_PKG_VERSION"),
    );
    ui.info(&t("app-name"), &detail);
}
