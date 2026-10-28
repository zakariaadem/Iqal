//! Small reusable widgets and text helpers.

use std::cell::Cell;
use std::rc::Rc;

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
    take_over_wheel(&scroll);
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

/// The mouse wheel on a page: over a box (a list, a text box), the box
/// scrolls while it can, and only once it cannot scroll further that way,
/// the page (never both at once, and the turn is never lost). Decided here,
/// before GTK's own handling: it gives the turn to the page first.
fn take_over_wheel(page: &gtk::ScrolledWindow) {
    // Where the pointer is on the page: wheel turns themselves do not always
    // say (they do not on X11).
    let pointer = Rc::new(Cell::new(None::<(f64, f64)>));
    let motion = gtk::EventControllerMotion::new();
    motion.set_propagation_phase(gtk::PropagationPhase::Capture);
    motion.connect_enter({
        let pointer = pointer.clone();
        move |_, x, y| pointer.set(Some((x, y)))
    });
    motion.connect_motion({
        let pointer = pointer.clone();
        move |_, x, y| pointer.set(Some((x, y)))
    });
    motion.connect_leave({
        let pointer = pointer.clone();
        move |_| pointer.set(None)
    });
    page.add_controller(motion);

    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = page.downgrade();
    scroll.connect_scroll(move |controller, _, dy| {
        let Some(page) = weak.upgrade() else { return gtk::glib::Propagation::Proceed };
        // The box under the pointer, if any.
        let inner = pointer.get().and_then(|(x, y)| {
            let mut w = page.pick(x, y, gtk::PickFlags::DEFAULT)?;
            loop {
                if w == *page.upcast_ref::<gtk::Widget>() {
                    return None;
                }
                if let Some(s) = w.downcast_ref::<gtk::ScrolledWindow>() {
                    return Some(s.clone());
                }
                w = w.parent()?;
            }
        });
        let can_scroll = |adj: &gtk::Adjustment| {
            if dy < 0.0 { adj.value() > adj.lower() } else { adj.value() < adj.upper() - adj.page_size() }
        };
        let adj = match inner.map(|s| s.vadjustment()) {
            Some(adj) if can_scroll(&adj) => adj,
            _ => page.vadjustment(),
        };
        // A touchpad reports pixels, a wheel notches.
        let step = if controller.unit() == gtk::gdk::ScrollUnit::Surface { dy } else { dy * adj.step_increment().max(48.0) };
        adj.set_value(adj.value() + step);
        gtk::glib::Propagation::Stop
    });
    page.add_controller(scroll);
}
