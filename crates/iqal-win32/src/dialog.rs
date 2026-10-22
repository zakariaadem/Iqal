//! Message boxes in Iqal's own dialog window (Windows' task dialog has its
//! own look and no dark mode): a round icon, a short title, an explanation,
//! and buttons named in the app's language.

use std::cell::Cell;
use std::rc::Rc;

use iqal_i18n::t;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{BN_CLICKED, BS_DEFPUSHBUTTON, WS_TABSTOP};

use crate::modal;
use crate::panel::{self, Item, Width, width_of};
use crate::theme::{self, Sign};

/// Static control drawn by its window.
const SS_OWNERDRAW: u32 = 0xD;

/// Which icon a message shows.
#[derive(Clone, Copy)]
pub enum Icon {
    Info,
    Warning,
    Error,
}

/// Shows a message with a Close button.
pub fn message(owner: HWND, icon: Icon, title: &str, detail: &str) {
    show(owner, Some(icon), title, detail, &[]);
}

/// Asks a question; returns the id of the button pressed (or `None` when
/// the dialog was closed). Buttons are (id, text); the last is the default.
pub fn ask(owner: HWND, icon: Option<Icon>, title: &str, detail: &str, buttons: &[(i32, String)]) -> Option<i32> {
    show(owner, icon, title, detail, buttons)
}

/// An error from the service: the first line is the title, the rest the
/// explanation.
pub fn error(owner: HWND, message: &str) {
    let (title, detail) = message.split_once('\n').unwrap_or((message, ""));
    show(owner, Some(Icon::Error), title, detail, &[]);
}

fn show(owner: HWND, icon: Option<Icon>, title: &str, detail: &str, buttons: &[(i32, String)]) -> Option<i32> {
    // A message without a question has a Close button.
    let message_only = buttons.is_empty();
    let ok = [(0, t("action-close"))];
    let buttons = if message_only { &ok[..] } else { buttons };
    let pressed = Rc::new(Cell::new(None));
    modal::run(owner, &t("app-name"), (460, 400), |page, closer| {
        let heading = page.heading(title);
        let mut items = Vec::new();
        match icon {
            Some(icon) => {
                let picture = page.control("STATIC", "", SS_OWNERDRAW, 0);
                let sign = match icon {
                    Icon::Warning => Sign::Warning,
                    Icon::Error => Sign::Error,
                    Icon::Info => Sign::Info,
                };
                panel::on_draw(picture, move |item| {
                    if let Some(fonts) = panel::fonts() {
                        theme::sign(item.hDC, item.rcItem, sign, fonts.title);
                    }
                });
                items.push(Item::Row { cells: vec![(picture, Width::Dip(32)), (heading, Width::Flex)], height: 32 });
            }
            None => items.push(Item::Text(heading)),
        }
        if !detail.is_empty() {
            items.push(Item::Text(page.control("STATIC", detail, 0x2000 /* SS_NOPREFIX */, 0)));
        }
        items.push(Item::Space(8));
        let mut row = vec![(page.control("STATIC", "", 0, 0), Width::Flex)];
        for (i, (id, text)) in buttons.iter().enumerate() {
            let last = i + 1 == buttons.len();
            let default = if last { BS_DEFPUSHBUTTON as u32 } else { 0 };
            let button = page.control("BUTTON", text, WS_TABSTOP | default, 0);
            if last {
                theme::style_button(button, None, theme::Look::Main);
            }
            let (id, pressed, closer) = (*id, pressed.clone(), closer.clone());
            panel::on(button, move |code| {
                if code == BN_CLICKED {
                    pressed.set(Some(id));
                    closer.close();
                }
            });
            row.push((button, width_of(button)));
            if last {
                // SAFETY: our button.
                unsafe { SetFocus(button) };
            }
        }
        items.push(Item::Row { cells: row, height: panel::ROW });
        page.set_items(items);
    });
    if message_only { None } else { pressed.get() }
}
