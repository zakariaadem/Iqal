//! Picking an installed app from the application menu entries.

use std::path::Path;
use std::rc::Rc;

use iqal_i18n::t;
use gtk::prelude::*;

use crate::ui::Ui;
use crate::widgets::{hbox, icon_button, set_margins, vbox};

struct App {
    name: String,
    icon: Option<String>,
    /// The rule that blocks it.
    rule: String,
}

/// The app rule for a menu entry's Exec line: Flatpak apps by id on the
/// command line, Snap apps by folder, everything else by program name.
fn rule_for(exec: &str) -> Option<String> {
    let tokens: Vec<&str> = exec.split_whitespace().filter(|t| !t.contains('=') && !t.starts_with('%')).collect();
    let mut it = tokens.iter().map(|t| t.trim_matches('"'));
    let mut first = it.next()?;
    if first == "env" {
        first = it.next()?;
    }
    let name = Path::new(first).file_name()?.to_str()?;
    match name {
        "flatpak" => {
            let id = tokens.iter().rev().find(|t| t.contains('.') && !t.starts_with('-'))?;
            Some(format!("cmd:{id}"))
        }
        "snap" => {
            let id = tokens.iter().skip_while(|t| **t != "run").nth(1)?;
            Some(format!("path:/snap/{id}/"))
        }
        _ => Some(format!("name:{name}")),
    }
}

/// Menu names of installed apps by program name ("gnome-calculator" →
/// "Calculator"), for showing programs by the name people know.
pub fn display_names() -> std::collections::HashMap<String, String> {
    installed_apps().into_iter().filter_map(|a| Some((a.rule.strip_prefix("name:")?.to_string(), a.name))).collect()
}

fn installed_apps() -> Vec<App> {
    let lang = iqal_i18n::current();
    let mut dirs: Vec<String> = vec![
        "/usr/share/applications".into(),
        "/usr/local/share/applications".into(),
        "/var/lib/flatpak/exports/share/applications".into(),
        "/var/lib/snapd/desktop/applications".into(),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(format!("{home}/.local/share/applications"));
    }
    let mut apps: Vec<App> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
            let (mut name, mut local_name, mut exec, mut icon, mut hidden, mut in_entry) = (None, None, None, None, false, false);
            for line in text.lines() {
                if line.starts_with('[') {
                    in_entry = line == "[Desktop Entry]";
                    continue;
                }
                if !in_entry {
                    continue;
                }
                let Some((k, v)) = line.split_once('=') else { continue };
                match k.trim() {
                    "Name" => name = Some(v.to_string()),
                    k if k == format!("Name[{lang}]") => local_name = Some(v.to_string()),
                    "Exec" => exec = Some(v.to_string()),
                    "Icon" => icon = Some(v.to_string()),
                    "NoDisplay" | "Hidden" if v.trim() == "true" => hidden = true,
                    _ => {}
                }
            }
            if hidden {
                continue;
            }
            if let (Some(name), Some(rule)) = (local_name.or(name), exec.as_deref().and_then(rule_for))
                && !apps.iter().any(|a| a.rule == rule)
            {
                apps.push(App { name, icon, rule });
            }
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// Shows the installed apps with a tick box each (as on Windows); `on_pick`
/// gets the rules of those ticked.
pub fn pick(ui: &Rc<Ui>, on_pick: impl Fn(Vec<String>) + 'static) {
    let win = gtk::Window::builder()
        .transient_for(&ui.window)
        .modal(true)
        .title(t("action-add-installed-app"))
        .default_width(480)
        .default_height(600)
        .build();
    let body = vbox(8);
    set_margins(&body, 12);
    let search = gtk::SearchEntry::builder().placeholder_text(t("search")).build();
    body.append(&search);
    let list = gtk::ListBox::new();
    list.add_css_class("blocks");
    list.set_selection_mode(gtk::SelectionMode::None);
    let apps = Rc::new(installed_apps());
    let mut checks = Vec::new();
    for app in apps.iter() {
        let row = hbox(10);
        set_margins(&row, 6);
        let check = gtk::CheckButton::new();
        row.append(&check);
        let image = match &app.icon {
            Some(i) if i.starts_with('/') => gtk::Image::from_file(i),
            Some(i) => gtk::Image::from_icon_name(i),
            None => gtk::Image::from_icon_name("application-x-executable"),
        };
        image.set_pixel_size(32);
        row.append(&image);
        let l = gtk::Label::new(Some(&app.name));
        l.set_xalign(0.0);
        row.append(&l);
        list.append(&row);
        checks.push(check);
    }
    let checks = Rc::new(checks);
    // A click anywhere on the row ticks or unticks the app.
    {
        let checks = checks.clone();
        list.set_activate_on_single_click(true);
        list.connect_row_activated(move |_, row| {
            if let Some(c) = checks.get(row.index() as usize) {
                c.set_active(!c.is_active());
            }
        });
    }
    {
        let apps = apps.clone();
        let search2 = search.clone();
        list.set_filter_func(move |row| {
            let q = search2.text().to_lowercase();
            q.is_empty() || apps.get(row.index() as usize).is_some_and(|a| a.name.to_lowercase().contains(&q))
        });
    }
    {
        let list = list.clone();
        search.connect_search_changed(move |_| list.invalidate_filter());
    }
    let scroll = gtk::ScrolledWindow::builder().vexpand(true).child(&list).build();
    scroll.add_css_class("box-frame");
    body.append(&scroll);
    let buttons = hbox(6);
    buttons.set_halign(gtk::Align::End);
    let add = icon_button(&t("action-add"), "list-add-symbolic", "main");
    let cancel = gtk::Button::with_label(&t("action-cancel"));
    buttons.append(&add);
    buttons.append(&cancel);
    body.append(&buttons);
    win.set_child(Some(&body));
    win.set_default_widget(Some(&add));
    {
        let win = win.clone();
        cancel.connect_clicked(move |_| win.close());
    }
    let win2 = win.clone();
    add.connect_clicked(move |_| {
        let rules: Vec<String> =
            checks.iter().zip(apps.iter()).filter(|(c, _)| c.is_active()).map(|(_, a)| a.rule.clone()).collect();
        if rules.is_empty() {
            return;
        }
        win2.close();
        on_pick(rules);
    });
    win.present();
}

/// The titles of the windows open now (X11; Wayland does not tell other
/// programs, and then the list is empty).
fn open_window_titles() -> Vec<String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
    let Ok((conn, screen)) = x11rb::connect(None) else { return Vec::new() };
    let root = conn.setup().roots[screen].root;
    let atom = |name: &[u8]| conn.intern_atom(false, name).ok().and_then(|c| c.reply().ok()).map(|r| r.atom);
    let (Some(list), Some(name), Some(utf8)) = (atom(b"_NET_CLIENT_LIST"), atom(b"_NET_WM_NAME"), atom(b"UTF8_STRING")) else {
        return Vec::new();
    };
    let Some(windows) = conn
        .get_property(false, root, list, AtomEnum::WINDOW, 0, 4096)
        .ok()
        .and_then(|c| c.reply().ok())
        .and_then(|r| r.value32().map(|v| v.collect::<Vec<u32>>()))
    else {
        return Vec::new();
    };
    let mut titles: Vec<String> = windows
        .into_iter()
        .filter_map(|w| conn.get_property(false, w, name, utf8, 0, 1024).ok()?.reply().ok())
        .map(|r| String::from_utf8_lossy(&r.value).trim().to_string())
        .filter(|t| !t.is_empty() && t != &iqal_i18n::t("app-name"))
        .collect();
    titles.sort();
    titles.dedup();
    titles
}

/// "Add a window title…" (as on Windows): words of a title, typed or taken
/// from a window open now; `on_pick` gets the rule.
pub fn pick_title(ui: &Rc<Ui>, on_pick: impl Fn(String) + 'static) {
    let win = gtk::Window::builder()
        .transient_for(&ui.window)
        .modal(true)
        .title(t("action-add-window-title"))
        .default_width(520)
        .default_height(560)
        .build();
    let body = vbox(10);
    set_margins(&body, 14);
    body.append(&crate::widgets::note(&t("window-title-intro")));
    let words = gtk::Entry::builder().placeholder_text(t("window-title-words")).build();
    words.set_activates_default(true);
    body.append(&words);
    body.append(&crate::widgets::heading(&t("window-title-open")));
    let list = gtk::ListBox::new();
    list.add_css_class("blocks");
    let titles = open_window_titles();
    for title in &titles {
        let l = gtk::Label::new(Some(title));
        l.set_xalign(0.0);
        l.set_ellipsize(gtk::pango::EllipsizeMode::End);
        set_margins(&l, 6);
        list.append(&l);
    }
    {
        let words = words.clone();
        list.connect_row_selected(move |_, row| {
            if let Some(title) = row.and_then(|r| titles.get(r.index() as usize)) {
                words.set_text(title);
            }
        });
    }
    let scroll = gtk::ScrolledWindow::builder().vexpand(true).child(&list).build();
    scroll.add_css_class("box-frame");
    body.append(&scroll);
    let buttons = hbox(6);
    buttons.set_halign(gtk::Align::End);
    let add = icon_button(&t("action-add"), "list-add-symbolic", "main");
    let cancel = gtk::Button::with_label(&t("action-cancel"));
    buttons.append(&add);
    buttons.append(&cancel);
    body.append(&buttons);
    win.set_child(Some(&body));
    win.set_default_widget(Some(&add));
    {
        let win = win.clone();
        cancel.connect_clicked(move |_| win.close());
    }
    let (win2, weak) = (win.clone(), ui.weak());
    add.connect_clicked(move |_| {
        let text = words.text().trim().to_string();
        if text.is_empty() {
            if let Some(ui) = weak.upgrade() {
                ui.info(&t("window-title-empty"), "");
            }
            return;
        }
        win2.close();
        on_pick(format!("title:{text}"));
    });
    win.present();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_from_exec_lines() {
        assert_eq!(rule_for("/usr/games/steam %U").as_deref(), Some("name:steam"));
        assert_eq!(rule_for("env GDK_BACKEND=x11 firefox %u").as_deref(), Some("name:firefox"));
        assert_eq!(rule_for("/usr/bin/flatpak run --branch=stable --arch=x86_64 com.discordapp.Discord").as_deref(), Some("cmd:com.discordapp.Discord"));
        assert_eq!(rule_for("/snap/bin/spotify %U").as_deref(), Some("name:spotify"));
        assert_eq!(rule_for("snap run spotify").as_deref(), Some("path:/snap/spotify/"));
    }
}
