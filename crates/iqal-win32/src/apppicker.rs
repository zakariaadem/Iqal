//! Picking installed apps from the Start menu: its shortcuts, by the names
//! people know, with their icons. Each app becomes the rule that blocks its
//! program.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use iqal_i18n::t;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DT_END_ELLIPSIS, DT_NOPREFIX,
    DT_RTLREADING, DT_SINGLELINE, DT_VCENTER, DrawTextW, FillRect, GetSysColor, GetSysColorBrush, InvalidateRect,
    SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_SELECTED};
use windows_sys::Win32::UI::Shell::{
    FOLDERID_CommonPrograms, FOLDERID_Programs, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW,
    SHGetKnownFolderPath,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, BS_DEFPUSHBUTTON, DI_NOMIRROR, DI_NORMAL, DestroyIcon, DrawIconEx, EN_CHANGE, ES_AUTOHSCROLL, HICON, LB_GETSELCOUNT,
    LB_GETSELITEMS, LB_SETITEMHEIGHT, LBN_DBLCLK, LBS_EXTENDEDSEL, LBS_NODATA, LBS_NOINTEGRALHEIGHT, LBS_NOTIFY,
    LBS_OWNERDRAWFIXED, WS_EX_CLIENTEDGE, WS_TABSTOP, WS_VSCROLL,
};
use windows_sys::core::{GUID, HRESULT};

use crate::modal;
use crate::panel::{self, Item, Width, button_width};
use crate::win::{self, scale};

const LB_SETCOUNT: u32 = 0x01A7;
const EM_SETCUEBANNER: u32 = 0x1501;
const ROW_HEIGHT: i32 = 32;
const ICON_SIZE: i32 = 24;

/// Programs whose name says nothing about the app (launchers, updaters):
/// blocked by their path instead, or every app's "update" would be blocked.
const GENERIC: &[&str] = &["update", "updater", "launcher", "launch", "start", "run", "app", "main", "client", "game"];

struct App {
    /// The shortcut's name, as the Start menu shows it.
    name: String,
    /// The shortcut, for its icon.
    shortcut: PathBuf,
    rule: String,
}

/// The rule for a shortcut's program and arguments.
fn rule_for(target: &str, args: &str) -> Option<String> {
    let path = Path::new(target);
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
        return None;
    }
    let stem = path.file_stem()?.to_str()?.to_lowercase();
    // Uninstallers are not apps.
    if stem.starts_with("unins") || stem.contains("uninstall") {
        return None;
    }
    // Apps installed with Squirrel (Discord, Teams…) start through
    // Update.exe --processStart App.exe.
    let mut words = args.split_whitespace().map(|w| w.trim_matches('"'));
    if words.any(|w| w.eq_ignore_ascii_case("--processStart"))
        && let Some(app) = words.next()
    {
        let app = app.strip_suffix(".exe").or_else(|| app.strip_suffix(".EXE")).unwrap_or(app);
        return Some(format!("name:{}", app.to_lowercase()));
    }
    if GENERIC.contains(&stem.as_str()) {
        return Some(format!("path:{target}"));
    }
    Some(stem)
}

/// The Start menu's folders: for everyone, and for this account.
fn start_menu_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    for id in [&FOLDERID_CommonPrograms, &FOLDERID_Programs] {
        let mut path: *mut u16 = std::ptr::null_mut();
        // SAFETY: Windows allocates the path; we copy it and free it.
        unsafe {
            if SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut path) == 0 && !path.is_null() {
                let len = (0..).take_while(|&i| *path.add(i) != 0).count();
                folders.push(PathBuf::from(String::from_utf16_lossy(std::slice::from_raw_parts(path, len))));
            }
            CoTaskMemFree(path as *const c_void);
        }
    }
    folders
}

fn shortcuts(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            shortcuts(&path, out);
        } else if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")) {
            out.push(path);
        }
    }
}

// The two COM interfaces needed to read a shortcut, declared by hand
// (windows-sys has no COM interfaces). Only the methods used are typed.
#[repr(C)]
struct ShellLinkVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_path: unsafe extern "system" fn(*mut c_void, *mut u16, i32, *mut c_void, u32) -> HRESULT,
    get_id_list: usize,
    set_id_list: usize,
    get_description: usize,
    set_description: usize,
    get_working_directory: usize,
    set_working_directory: usize,
    get_arguments: unsafe extern "system" fn(*mut c_void, *mut u16, i32) -> HRESULT,
}

#[repr(C)]
struct PersistFileVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_class_id: usize,
    is_dirty: usize,
    load: unsafe extern "system" fn(*mut c_void, *const u16, u32) -> HRESULT,
}

const CLSID_SHELL_LINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);
const IID_SHELL_LINK_W: GUID = GUID::from_u128(0x000214f9_0000_0000_c000_000000000046);
const IID_PERSIST_FILE: GUID = GUID::from_u128(0x0000010b_0000_0000_c000_000000000046);

/// A shortcut's program and arguments, as Windows reads them.
fn read_shortcut(lnk: &Path) -> Option<(String, String)> {
    let file = win::wide(&lnk.to_string_lossy());
    // SAFETY: COM calls through the interfaces' tables; every object made
    // here is released here, and the buffers outlive the calls.
    unsafe {
        let mut link: *mut c_void = std::ptr::null_mut();
        if CoCreateInstance(&CLSID_SHELL_LINK, std::ptr::null_mut(), CLSCTX_INPROC_SERVER, &IID_SHELL_LINK_W, &mut link) != 0 {
            return None;
        }
        let link_vtbl = &**(link as *mut *const ShellLinkVtbl);
        let mut persist: *mut c_void = std::ptr::null_mut();
        let mut result = None;
        if (link_vtbl.query_interface)(link, &IID_PERSIST_FILE, &mut persist) == 0 {
            let persist_vtbl = &**(persist as *mut *const PersistFileVtbl);
            if (persist_vtbl.load)(persist, file.as_ptr(), 0 /* STGM_READ */) == 0 {
                let mut target = vec![0u16; 1024];
                let mut args = vec![0u16; 1024];
                if (link_vtbl.get_path)(link, target.as_mut_ptr(), target.len() as i32, std::ptr::null_mut(), 0) == 0 {
                    (link_vtbl.get_arguments)(link, args.as_mut_ptr(), args.len() as i32);
                    let text = |b: &[u16]| String::from_utf16_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]);
                    result = Some((text(&target), text(&args)));
                }
            }
            (persist_vtbl.release)(persist);
        }
        (link_vtbl.release)(link);
        result.filter(|(target, _)| !target.is_empty())
    }
}

fn installed_apps() -> Vec<App> {
    let mut links = Vec::new();
    for folder in start_menu_folders() {
        shortcuts(&folder, &mut links);
    }
    // SAFETY: paired with CoUninitialize below when it succeeds.
    let com = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) } >= 0;
    let mut apps: Vec<App> = Vec::new();
    for lnk in links {
        let Some(name) = lnk.file_stem().and_then(|s| s.to_str()).map(String::from) else { continue };
        if name.to_lowercase().contains("uninstall") {
            continue;
        }
        let Some(rule) = read_shortcut(&lnk).and_then(|(target, args)| rule_for(&target, &args)) else { continue };
        if !apps.iter().any(|a| a.rule == rule) {
            apps.push(App { name, shortcut: lnk, rule });
        }
    }
    if com {
        // SAFETY: matches the successful CoInitializeEx.
        unsafe { CoUninitialize() };
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// Icons, loaded when their row is first drawn.
struct Icons(HashMap<usize, HICON>);

impl Drop for Icons {
    fn drop(&mut self) {
        for icon in self.0.values().filter(|i| !i.is_null()) {
            // SAFETY: icons we got from SHGetFileInfoW.
            unsafe { DestroyIcon(*icon) };
        }
    }
}

struct State {
    apps: Vec<App>,
    /// The rows shown: indexes into `apps`.
    shown: Vec<usize>,
    icons: Icons,
}

/// Shows the installed apps; `on_pick` gets the rules of those chosen.
pub fn pick(owner: HWND, on_pick: impl Fn(Vec<String>) + 'static) {
    let apps = installed_apps();
    let shown = (0..apps.len()).collect();
    let state = Rc::new(RefCell::new(State { apps, shown, icons: Icons(HashMap::new()) }));
    modal::run(owner, &t("action-add-installed-app"), (480, 580), move |page, closer| {
        let search = page.control("EDIT", "", WS_TABSTOP | ES_AUTOHSCROLL as u32, WS_EX_CLIENTEDGE);
        let cue = win::wide(&t("search"));
        win::send(search, EM_SETCUEBANNER, 1, cue.as_ptr() as isize);
        let list = page.control(
            "LISTBOX",
            "",
            WS_TABSTOP | WS_VSCROLL | (LBS_OWNERDRAWFIXED | LBS_NODATA | LBS_EXTENDEDSEL | LBS_NOINTEGRALHEIGHT | LBS_NOTIFY) as u32,
            WS_EX_CLIENTEDGE,
        );
        win::send(list, LB_SETITEMHEIGHT, 0, scale(ROW_HEIGHT, panel::dpi()) as isize);
        win::send(list, LB_SETCOUNT, state.borrow().shown.len(), 0);
        let spacer = page.control("STATIC", "", 0, 0);
        let add = page.control("BUTTON", &t("action-add"), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
        let cancel = page.control("BUTTON", &t("action-cancel"), WS_TABSTOP, 0);

        {
            let state = state.clone();
            panel::on(search, move |code| {
                if code == EN_CHANGE as u32 {
                    let filter = win::text(search).trim().to_lowercase();
                    let count = {
                        let mut st = state.borrow_mut();
                        st.shown = (0..st.apps.len()).filter(|&i| st.apps[i].name.to_lowercase().contains(&filter)).collect();
                        st.shown.len()
                    };
                    win::send(list, LB_SETCOUNT, count, 0);
                    // SAFETY: our list.
                    unsafe { InvalidateRect(list, std::ptr::null(), 1) };
                }
            });
        }
        let on_pick = Rc::new(on_pick);
        let choose = {
            let (state, closer) = (state.clone(), closer.clone());
            move || {
                let count = win::send(list, LB_GETSELCOUNT, 0, 0).max(0) as usize;
                let mut rows = vec![0i32; count];
                win::send(list, LB_GETSELITEMS, count, rows.as_mut_ptr() as isize);
                let rules: Vec<String> = {
                    let st = state.borrow();
                    rows.iter().filter_map(|&r| st.shown.get(r as usize)).map(|&i| st.apps[i].rule.clone()).collect()
                };
                if rules.is_empty() {
                    return;
                }
                closer.close();
                on_pick(rules);
            }
        };
        let choose = Rc::new(choose);
        {
            let choose = choose.clone();
            panel::on(add, move |code| {
                if code == BN_CLICKED {
                    choose();
                }
            });
        }
        panel::on(list, move |code| {
            if code == LBN_DBLCLK {
                choose();
            }
        });
        {
            let closer = closer.clone();
            panel::on(cancel, move |code| {
                if code == BN_CLICKED {
                    closer.close();
                }
            });
        }
        {
            let state = state.clone();
            panel::on_draw(list, move |item| draw(&mut state.borrow_mut(), item));
        }
        page.set_items(vec![
            Item::Full { hwnd: search, height: panel::ROW },
            Item::Full { hwnd: list, height: 440 },
            Item::Row {
                cells: vec![
                    (spacer, Width::Flex),
                    (add, button_width(&t("action-add"))),
                    (cancel, button_width(&t("action-cancel"))),
                ],
                height: panel::ROW,
            },
        ]);
        // SAFETY: our search box.
        unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(search) };
    });
}

/// One row: the app's icon and name.
fn draw(state: &mut State, item: &DRAWITEMSTRUCT) {
    let Some(&index) = state.shown.get(item.itemID as usize) else { return };
    let Some(fonts) = panel::fonts() else { return };
    let icon = *state.icons.0.entry(index).or_insert_with(|| {
        let path = win::wide(&state.apps[index].shortcut.to_string_lossy());
        // SAFETY: a zeroed structure for Windows to fill.
        let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
        // SAFETY: valid path and structure.
        unsafe { SHGetFileInfoW(path.as_ptr(), 0, &mut info, size_of::<SHFILEINFOW>() as u32, SHGFI_ICON | SHGFI_LARGEICON) };
        info.hIcon
    });
    let selected = item.itemState & ODS_SELECTED != 0;
    let (back, fore) = if selected { (COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT) } else { (COLOR_WINDOW, COLOR_WINDOWTEXT) };
    let dpi = panel::dpi();
    let (pad, size) = (scale(6, dpi), scale(ICON_SIZE, dpi));
    let top = item.rcItem.top + (item.rcItem.bottom - item.rcItem.top - size) / 2;
    let mut r = RECT { left: item.rcItem.left + pad * 2 + size, right: item.rcItem.right - pad, ..item.rcItem };
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    // Ends in a zero: DrawTextW reads through the pointer even for an empty text.
    let text: Vec<u16> = state.apps[index].name.encode_utf16().chain(Some(0)).collect();
    // SAFETY: drawing into the list's DC during WM_DRAWITEM.
    unsafe {
        FillRect(item.hDC, &item.rcItem, GetSysColorBrush(back as i32));
        // Icons keep their own direction in right-to-left windows.
        if !icon.is_null() {
            DrawIconEx(item.hDC, item.rcItem.left + pad, top, icon, size, size, 0, std::ptr::null_mut(), DI_NORMAL | DI_NOMIRROR);
        }
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, GetSysColor(fore as i32));
        SelectObject(item.hDC, fonts.normal);
        DrawTextW(item.hDC, text.as_ptr(), text.len() as i32 - 1, &mut r, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX | rtl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_from_shortcuts() {
        assert_eq!(rule_for(r"C:\Program Files (x86)\Steam\steam.exe", "").as_deref(), Some("steam"));
        assert_eq!(rule_for(r"C:\Users\a\AppData\Local\Discord\Update.exe", "--processStart Discord.exe").as_deref(), Some("name:discord"));
        assert_eq!(rule_for(r"C:\Games\X\launcher.exe", "").as_deref(), Some(r"path:C:\Games\X\launcher.exe"));
        assert_eq!(rule_for(r"C:\Program Files\X\unins000.exe", ""), None);
        assert_eq!(rule_for(r"C:\Program Files\X\readme.txt", ""), None);
    }
}
