//! al-dir: the Al-Dir graphical app. It edits blocks through the Al-Dir
//! service; all enforcement (and refusing to weaken locked blocks) happens
//! in the service.

mod appspicker;
mod dialogs;
mod editor;
mod lockui;
mod mapdialog;
mod service;
mod settings;
mod stats;
mod ui;
mod widgets;

use gtk::glib;
use gtk::prelude::*;

const APP_ID: &str = "org.aldir.AlDir";

fn main() -> glib::ExitCode {
    let saved = settings::GuiSettings::load();
    let language = saved.language.as_deref().unwrap_or_else(|| al_dir_i18n::system_language());
    al_dir_i18n::init(language);

    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        // Arabic and other right-to-left languages mirror the whole layout.
        gtk::Widget::set_default_direction(if al_dir_i18n::is_rtl() {
            gtk::TextDirection::Rtl
        } else {
            gtk::TextDirection::Ltr
        });
        if let Some(w) = app.active_window() {
            w.present();
            return;
        }
        load_css();
        ui::Ui::start(app);
    });
    app.run_with_args::<&str>(&[])
}

/// Styles for the statistics window; colours follow the light or dark theme.
fn load_css() {
    let css = gtk::CssProvider::new();
    css.load_from_string(
        ".stat-card { background-color: alpha(currentColor, 0.06); border-radius: 10px; padding: 12px 14px; }
         .stat-value { font-size: 20pt; font-weight: bold; }
         .stat-bar { background-color: #1D9E75; border-radius: 3px; }
         .stat-bar-empty { background-color: alpha(currentColor, 0.2); border-radius: 1px; }
         .stat-bar-allowance { background-color: #BA7517; border-radius: 3px; }
         .stat-today { font-weight: bold; }",
    );
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}
