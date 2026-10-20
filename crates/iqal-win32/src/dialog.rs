//! Message boxes, through Windows' task dialog (in dark mode, Iqal's own
//! dialog window, as the task dialog is always light): a short title, an
//! explanation, and buttons named in the app's language.

use std::cell::Cell;
use std::rc::Rc;

use iqal_i18n::t;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDCBF_OK_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_POSITION_RELATIVE_TO_WINDOW,
    TDF_RTL_LAYOUT,
    TaskDialogIndirect,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, BS_DEFPUSHBUTTON, IDI_ERROR, IDI_INFORMATION, IDI_WARNING, LoadIconW, STM_SETICON, WS_TABSTOP,
};

use crate::modal;
use crate::panel::{self, Item, Width, button_width};
use crate::win::{self, wide};

/// Static control showing an icon.
const SS_ICON: u32 = 0x3;

/// Which icon a message shows.
#[derive(Clone, Copy)]
pub enum Icon {
    Info,
    Warning,
    Error,
}

impl Icon {
    /// The stock icon resource (TD_*_ICON).
    fn resource(self) -> *const u16 {
        let id: isize = match self {
            Icon::Warning => -1,
            Icon::Error => -2,
            Icon::Info => -3,
        };
        id as u16 as usize as *const u16
    }
}

/// Shows a message with an OK button.
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
    // Windows' task dialog is always light.
    if crate::theme::is_dark() {
        return show_own(owner, icon, title, detail, buttons);
    }
    let window_title = wide(&t("app-name"));
    let (title, detail) = (wide(title), wide(detail));
    let texts: Vec<Vec<u16>> = buttons.iter().map(|(_, text)| wide(text)).collect();
    let list: Vec<TASKDIALOG_BUTTON> = buttons
        .iter()
        .zip(&texts)
        .map(|((id, _), text)| TASKDIALOG_BUTTON { nButtonID: *id, pszButtonText: text.as_ptr() })
        .collect();
    // SAFETY: plain data; every pointer set below outlives the call.
    let mut config: TASKDIALOGCONFIG = unsafe { std::mem::zeroed() };
    config.cbSize = std::mem::size_of::<TASKDIALOGCONFIG>() as u32;
    config.hwndParent = owner;
    config.hInstance = win::instance();
    // Esc and the close button mean "cancel".
    config.dwFlags = TDF_POSITION_RELATIVE_TO_WINDOW
        | TDF_ALLOW_DIALOG_CANCELLATION
        | if iqal_i18n::is_rtl() { TDF_RTL_LAYOUT } else { 0 };
    config.pszWindowTitle = window_title.as_ptr();
    if let Some(icon) = icon {
        config.Anonymous1.pszMainIcon = icon.resource();
    }
    config.pszMainInstruction = title.as_ptr();
    config.pszContent = if detail.len() > 1 { detail.as_ptr() } else { std::ptr::null() };
    if list.is_empty() {
        config.dwCommonButtons = TDCBF_OK_BUTTON;
    } else {
        config.cButtons = list.len() as u32;
        config.pButtons = list.as_ptr();
        config.nDefaultButton = buttons.last().map_or(0, |(id, _)| *id);
    }
    let mut pressed = 0i32;
    // SAFETY: config is filled in as TaskDialogIndirect requires.
    let hr = unsafe { TaskDialogIndirect(&config, &mut pressed, std::ptr::null_mut(), std::ptr::null_mut()) };
    (hr >= 0 && buttons.iter().any(|(id, _)| *id == pressed)).then_some(pressed)
}

/// The same message in Iqal's own dialog window, which follows dark mode.
fn show_own(owner: HWND, icon: Option<Icon>, title: &str, detail: &str, buttons: &[(i32, String)]) -> Option<i32> {
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
                let picture = page.control("STATIC", "", SS_ICON, 0);
                let id = match icon {
                    Icon::Warning => IDI_WARNING,
                    Icon::Error => IDI_ERROR,
                    Icon::Info => IDI_INFORMATION,
                };
                // SAFETY: a stock icon; no freeing needed.
                let handle = unsafe { LoadIconW(std::ptr::null_mut(), id) };
                win::send(picture, STM_SETICON, handle as usize, 0);
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
            let default = if i + 1 == buttons.len() { BS_DEFPUSHBUTTON as u32 } else { 0 };
            let button = page.control("BUTTON", text, WS_TABSTOP | default, 0);
            let (id, pressed, closer) = (*id, pressed.clone(), closer.clone());
            panel::on(button, move |code| {
                if code == BN_CLICKED {
                    pressed.set(Some(id));
                    closer.close();
                }
            });
            row.push((button, button_width(text)));
            if i + 1 == buttons.len() {
                // SAFETY: our button.
                unsafe { SetFocus(button) };
            }
        }
        items.push(Item::Row { cells: row, height: panel::ROW });
        page.set_items(items);
    });
    if message_only { None } else { pressed.get() }
}
