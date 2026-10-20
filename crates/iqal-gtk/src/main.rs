//! iqal: the Iqal graphical app. It edits blocks through the Iqal
//! service; all enforcement (and refusing to weaken locked blocks) happens
//! in the service.

mod appspicker;
mod dialogs;
mod editor;
mod lockui;
mod mapdialog;
mod rules;
mod stats;
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
    let Some(display) = gtk::gdk::Display::default() else { return };
    gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    // Without a compositor (e.g. Xfce with compositing off) the see-through
    // shadows and round corners of menus and tooltips show as black.
    if !display.is_composited() {
        let flat = gtk::CssProvider::new();
        flat.load_from_string(
            "popover > contents, popover.menu > contents, tooltip, tooltip.background {
                 box-shadow: none; border-radius: 0; margin: 0;
             }
             popover > contents { border: 1px solid alpha(currentColor, 0.15); }",
        );
        gtk::style_context_add_provider_for_display(&display, &flat, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}
