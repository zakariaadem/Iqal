//! iqal: the Iqal graphical app. It edits blocks through the Iqal
//! service; all enforcement (and refusing to weaken locked blocks) happens
//! in the service.

mod appspicker;
mod dialogs;
mod editor;
mod lockui;
mod messages;
mod rules;
mod stats;
mod theme;
mod ui;
mod widgets;

use gtk::glib;
use gtk::prelude::*;
use iqal_app::{service, settings};

const APP_ID: &str = "org.iqal.Iqal";

fn main() -> glib::ExitCode {
    let saved = settings::GuiSettings::load();
    let language = saved.language.as_deref().unwrap_or_else(|| iqal_i18n::system_language());
    iqal_i18n::init(language);

    // One window per service: a test service (IQAL_SOCKET) gets its own.
    let app_id = if std::env::var_os("IQAL_SOCKET").is_some() { format!("{APP_ID}.Test") } else { APP_ID.to_string() };
    let app = gtk::Application::builder().application_id(app_id).build();
    app.connect_activate(|app| {
        // Arabic and other right-to-left languages mirror the whole layout.
        gtk::Widget::set_default_direction(if iqal_i18n::is_rtl() {
            gtk::TextDirection::Rtl
        } else {
            gtk::TextDirection::Ltr
        });
        if let Some(w) = app.active_window() {
            w.present();
            return;
        }
        theme::apply();
        ui::Ui::start(app);
    });
    app.run_with_args::<&str>(&[])
}

