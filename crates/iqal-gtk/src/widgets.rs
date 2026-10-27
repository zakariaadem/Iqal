//! Small reusable widgets and text helpers.

use gtk::prelude::*;

pub use iqal_app::describe::{day_name, describe_lock, fmt_time};

pub fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, spacing)
}

pub fn hbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, spacing)
}

/// A page body inside a scrolled window, with comfortable margins.
pub fn page() -> (gtk::ScrolledWindow, gtk::Box) {
    let body = vbox(14);
    set_margins(&body, 18);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&body)
        .build();
    (scroll, body)
}

pub fn heading(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("heading");
    l.set_xalign(0.0);
    l
}

pub fn note(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("dim-label");
    l.set_xalign(0.0);
    l.set_wrap(true);
    // Long texts wrap instead of widening the window (they still use all
    // the width the window has).
    l.set_max_width_chars(60);
    l
}

pub fn label(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    l.set_wrap(true);
    l.set_max_width_chars(60);
    l
}


pub fn set_margins(w: &impl IsA<gtk::Widget>, m: i32) {
    w.set_margin_top(m);
    w.set_margin_bottom(m);
    w.set_margin_start(m);
    w.set_margin_end(m);
}

/// A row with a label on one side and a control on the other.
pub fn setting_row(text: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = hbox(12);
    let l = label(text);
    l.set_hexpand(true);
    row.append(&l);
    control.set_valign(gtk::Align::Center);
    row.append(control);
    row
}

/// A button with a small icon and a text, as on Windows. `look`: "main"
/// (filled with the accent), "quiet" (grey text, the icon in deep red, for
/// Delete) or "" (plain).
pub fn icon_button(text: &str, icon: &str, look: &str) -> gtk::Button {
    let button = gtk::Button::new();
    let row = hbox(6);
    row.set_halign(gtk::Align::Center);
    let image = gtk::Image::from_icon_name(icon);
    if look == "quiet" {
        image.add_css_class("danger");
    }
    row.append(&image);
    row.append(&gtk::Label::new(Some(text)));
    button.set_child(Some(&row));
    if !look.is_empty() {
        button.add_css_class(look);
    }
    button
}

/// A small gold label with an icon (the block list's "Active", "Locked").
pub fn badge(icon: &str, text: &str) -> gtk::Box {
    let b = hbox(4);
    b.add_css_class("badge");
    b.append(&gtk::Image::from_icon_name(icon));
    b.append(&gtk::Label::new(Some(text)));
    b
}

/// The mouse wheel over a box that cannot scroll further that way scrolls
/// the page around it instead (otherwise the turn would be lost).
pub fn pass_wheel(inner: &gtk::ScrolledWindow) {
    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = inner.downgrade();
    scroll.connect_scroll(move |_, _, dy| {
        let Some(inner) = weak.upgrade() else { return gtk::glib::Propagation::Proceed };
        let adj = inner.vadjustment();
        let can_scroll = if dy < 0.0 { adj.value() > adj.lower() } else { adj.value() < adj.upper() - adj.page_size() };
        if can_scroll {
            return gtk::glib::Propagation::Proceed;
        }
        let Some(outer) = inner.parent().and_then(|p| p.ancestor(gtk::ScrolledWindow::static_type())).and_downcast::<gtk::ScrolledWindow>()
        else {
            return gtk::glib::Propagation::Proceed;
        };
        let page = outer.vadjustment();
        page.set_value(page.value() + dy * page.step_increment().max(48.0));
        gtk::glib::Propagation::Stop
    });
    inner.add_controller(scroll);
}
