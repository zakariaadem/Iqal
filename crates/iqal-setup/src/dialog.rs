//! The setup's questions and messages, through Windows' task dialog.

use iqal_i18n::t;
use windows_sys::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDCBF_OK_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_RTL_LAYOUT,
    TDF_USE_HICON_MAIN, TaskDialogIndirect,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::LoadIconW;

#[derive(Clone, Copy)]
pub enum Icon {
    /// Iqal's own icon.
    Iqal,
    Warning,
    Error,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn message(icon: Icon, title: &str, detail: &str) {
    show(icon, title, detail, &[]);
}

/// Returns the id of the button pressed, or `None` when closed. The last
/// button is the default.
pub fn ask(icon: Icon, title: &str, detail: &str, buttons: &[(i32, String)]) -> Option<i32> {
    show(icon, title, detail, buttons)
}

fn show(icon: Icon, title: &str, detail: &str, buttons: &[(i32, String)]) -> Option<i32> {
    let window_title = wide(&t("setup-window-title"));
    let (title, detail) = (wide(title), wide(detail));
    let texts: Vec<Vec<u16>> = buttons.iter().map(|(_, text)| wide(text)).collect();
    let list: Vec<TASKDIALOG_BUTTON> = buttons
        .iter()
        .zip(&texts)
        .map(|((id, _), text)| TASKDIALOG_BUTTON { nButtonID: *id, pszButtonText: text.as_ptr() })
        .collect();
    // SAFETY: plain data; every pointer set below outlives the call.
    let mut config: TASKDIALOGCONFIG = unsafe { std::mem::zeroed() };
    config.cbSize = size_of::<TASKDIALOGCONFIG>() as u32;
    config.dwFlags = TDF_ALLOW_DIALOG_CANCELLATION | if iqal_i18n::is_rtl() { TDF_RTL_LAYOUT } else { 0 };
    config.pszWindowTitle = window_title.as_ptr();
    // Stock icons: TD_WARNING_ICON, TD_ERROR_ICON.
    let stock = |id: isize| id as u16 as usize as *const u16;
    match icon {
        Icon::Iqal => {
            config.dwFlags |= TDF_USE_HICON_MAIN;
            // SAFETY: resource 1 of this program (see build.rs).
            config.Anonymous1.hMainIcon = unsafe { LoadIconW(GetModuleHandleW(std::ptr::null()), 1 as _) };
        }
        Icon::Warning => config.Anonymous1.pszMainIcon = stock(-1),
        Icon::Error => config.Anonymous1.pszMainIcon = stock(-2),
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
