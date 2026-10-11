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
        ui::Ui::start(app);
    });
    app.run_with_args::<&str>(&[])
}
