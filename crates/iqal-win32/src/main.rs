//! iqal-win32: the Iqal graphical app for Windows, drawn with Windows' own
//! controls. It edits blocks through the Iqal service; all enforcement
//! (and refusing to weaken locked blocks) happens in the service.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod dialog;
#[cfg(windows)]
mod editor;
#[cfg(windows)]
mod lockui;
#[cfg(windows)]
mod menu;
#[cfg(windows)]
mod modal;
#[cfg(windows)]
mod panel;
#[cfg(windows)]
mod rules;
#[cfg(windows)]
mod stats;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win;

#[cfg(windows)]
fn main() {
    let saved = iqal_app::settings::GuiSettings::load();
    let language = saved.language.as_deref().unwrap_or_else(|| iqal_i18n::system_language());
    // GDI shows the direction isolates around inserted values as boxes.
    iqal_i18n::init_with(language, false);
    ui::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("iqal-win32 is the Windows app; on Linux use the GTK app (iqal)");
}
