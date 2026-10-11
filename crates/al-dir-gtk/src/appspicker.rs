//! Picking an installed app from the application menu entries.

use std::path::Path;
use std::rc::Rc;

use al_dir_i18n::t;
use gtk::prelude::*;

use crate::ui::Ui;
use crate::widgets::{hbox, set_margins, vbox};

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

fn installed_apps() -> Vec<App> {
    let lang = al_dir_i18n::current();
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

pub fn pick(ui: &Rc<Ui>, on_pick: impl Fn(String) + 'static) {
    let win = gtk::Window::builder()
        .transient_for(&ui.window)
        .modal(true)
        .title(t("action-add-installed-app"))
        .default_width(460)
        .default_height(560)
        .build();
    let body = vbox(8);
    set_margins(&body, 12);
    let search = gtk::SearchEntry::new();
    body.append(&search);
    let list = gtk::ListBox::new();
    let apps = Rc::new(installed_apps());
    for app in apps.iter() {
        let row = hbox(10);
        set_margins(&row, 6);
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
    body.append(&scroll);
    win.set_child(Some(&body));
    let win2 = win.clone();
    list.connect_row_activated(move |_, row| {
        if let Some(app) = apps.get(row.index() as usize) {
            on_pick(app.rule.clone());
            win2.close();
        }
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
