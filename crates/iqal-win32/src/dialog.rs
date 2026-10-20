//! Message boxes, through Windows' task dialog: a short title, an
//! explanation, and buttons named in the app's language.

use iqal_i18n::t;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDCBF_OK_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_POSITION_RELATIVE_TO_WINDOW,
    TDF_RTL_LAYOUT,
    TaskDialogIndirect,
};

use crate::win::{self, wide};

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
