//! Messages and questions in Iqal's own small window (the same on Windows):
//! a round icon, a short title, an explanation, and buttons, the last one
//! the main action.

use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use iqal_i18n::t;

use crate::widgets::{hbox, label, set_margins, vbox};

/// What a message is about, for its round icon.
#[derive(Clone, Copy)]
pub enum Sign {
    Info,
    Warning,
    Error,
}

/// Shows a message over `parent`. `buttons`: their texts, the last one the
/// main action (none: a Close button). `answer` gets the button pressed,
/// or `None` when the window was closed.
pub fn show(
    parent: &impl IsA<gtk::Window>,
    sign: Option<Sign>,
    title: &str,
    detail: &str,
    buttons: &[String],
    answer: impl Fn(Option<usize>) + 'static,
) {
    let win = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(t("app-name"))
        .resizable(false)
        .default_width(460)
        .build();
    let body = vbox(12);
    set_margins(&body, 20);
    let top = hbox(12);
    if let Some(sign) = sign {
        let (mark, class) = match sign {
            Sign::Info => ("i", "info"),
            Sign::Warning => ("!", "warning"),
            Sign::Error => ("!", "error"),
        };
        let icon = gtk::Label::new(Some(mark));
        icon.add_css_class("sign");
        icon.add_css_class(class);
        icon.set_valign(gtk::Align::Center);
        top.append(&icon);
    }
    let heading = label(title);
    heading.add_css_class("heading");
    heading.set_valign(gtk::Align::Center);
    heading.set_hexpand(true);
    top.append(&heading);
    body.append(&top);
    if !detail.is_empty() {
        body.append(&label(detail));
    }
    let close = [t("action-close")];
    let message_only = buttons.is_empty();
    let buttons = if message_only { &close[..] } else { buttons };
    let row = hbox(8);
    row.set_halign(gtk::Align::End);
    row.set_margin_top(8);
    let answer = Rc::new(answer);
    let answered = Rc::new(std::cell::Cell::new(false));
    for (i, text) in buttons.iter().enumerate() {
        let b = gtk::Button::with_label(text);
        let last = i + 1 == buttons.len();
        if last {
            b.add_css_class("main");
            win.set_default_widget(Some(&b));
        }
        let (win, answer, answered) = (win.clone(), answer.clone(), answered.clone());
        b.connect_clicked(move |_| {
            answered.set(true);
            win.close();
            if !message_only {
                answer(Some(i));
            }
        });
        row.append(&b);
        if last {
            let b = b.clone();
            glib::idle_add_local_once(move || {
                b.grab_focus();
            });
        }
    }
    body.append(&row);
    win.set_child(Some(&body));
    // Esc closes, as Cancel.
    let keys = gtk::ShortcutController::new();
    let w = win.downgrade();
    keys.add_shortcut(gtk::Shortcut::new(
        gtk::ShortcutTrigger::parse_string("Escape"),
        Some(gtk::CallbackAction::new(move |_, _| {
            if let Some(w) = w.upgrade() {
                w.close();
            }
            glib::Propagation::Stop
        })),
    ));
    win.add_controller(keys);
    win.connect_close_request(move |_| {
        if !answered.replace(true) {
            answer(None);
        }
        glib::Propagation::Proceed
    });
    win.present();
}
