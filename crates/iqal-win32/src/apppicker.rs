//! Picking installed apps from Windows' list of apps (what "All apps" in
//! the Start menu shows, Store apps included), by the names people know,
//! with their icons. Each app becomes the rule that blocks its program.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::Path;
use std::rc::Rc;

use iqal_i18n::t;
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DT_END_ELLIPSIS, DT_NOPREFIX,
    DT_RTLREADING, DT_SINGLELINE, DT_VCENTER, DrawTextW, FillRect, InvalidateRect,
    SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_SELECTED};
use windows_sys::Win32::UI::Shell::Common::ITEMIDLIST;
use windows_sys::Win32::UI::Shell::{
    BHID_EnumItems, ILFree, SHCreateItemFromParsingName, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_PIDL,
    SHGetFileInfoW, SHGetIDListFromObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, BS_DEFPUSHBUTTON, DI_NOMIRROR, DI_NORMAL, DestroyIcon, DrawIconEx, EN_CHANGE, ES_AUTOHSCROLL, EnumWindows, GW_OWNER,
    GWL_EXSTYLE, GetWindow, GetWindowLongW, GetWindowThreadProcessId, HICON, IsWindowVisible, LB_ADDSTRING, LB_GETCURSEL, LB_GETSELCOUNT,
    LB_GETSELITEMS, LB_SETITEMHEIGHT, LBN_DBLCLK, LBN_SELCHANGE, LBS_EXTENDEDSEL, LBS_NODATA, LBS_NOINTEGRALHEIGHT, LBS_NOTIFY,
    LBS_OWNERDRAWFIXED, WS_EX_CLIENTEDGE, WS_EX_TOOLWINDOW, WS_TABSTOP, WS_VSCROLL,
};
use windows_sys::core::{GUID, HRESULT};

use crate::{dialog, modal};
use crate::panel::{self, Item, Width, button_width};
use crate::win::{self, scale};

const LB_SETCOUNT: u32 = 0x01A7;
const EM_SETCUEBANNER: u32 = 0x1501;
const ROW_HEIGHT: i32 = 32;
const ICON_SIZE: i32 = 24;

/// Programs whose name says nothing about the app (launchers, updaters):
/// blocked by their path instead, or every app's "update" would be blocked.
const GENERIC: &[&str] = &["update", "updater", "setup", "install", "installer", "launcher", "launch", "start", "run", "app", "main", "client", "game"];

struct App {
    /// The app's name, as the Start menu shows it.
    name: String,
    icon: Pidl,
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

// The COM interfaces needed to list the apps, declared by hand (windows-sys
// has no COM interfaces). Only the methods used are typed.
#[repr(C)]
struct ShellItemVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    bind_to_handler: unsafe extern "system" fn(*mut c_void, *mut c_void, *const GUID, *const GUID, *mut *mut c_void) -> HRESULT,
    get_parent: usize,
    get_display_name: unsafe extern "system" fn(*mut c_void, i32, *mut *mut u16) -> HRESULT,
    get_attributes: usize,
    compare: usize,
    // IShellItem2
    get_property_store: usize,
    get_property_store_with_create_object: usize,
    get_property_store_for_keys: usize,
    get_property_description_list: usize,
    update: usize,
    get_property: usize,
    get_clsid: usize,
    get_file_time: usize,
    get_int32: usize,
    get_string: unsafe extern "system" fn(*mut c_void, *const PropertyKey, *mut *mut u16) -> HRESULT,
}

#[repr(C)]
struct EnumShellItemsVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    next: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void, *mut u32) -> HRESULT,
}

#[repr(C)]
struct PropertyKey {
    fmtid: GUID,
    pid: u32,
}

const IID_SHELL_ITEM: GUID = GUID::from_u128(0x43826d1e_e718_42ee_bc55_a1e261c37bfe);
const IID_SHELL_ITEM2: GUID = GUID::from_u128(0x7e9fb0d3_919f_4307_ab2e_9b1860310c93);
const IID_ENUM_SHELL_ITEMS: GUID = GUID::from_u128(0x70629033_e363_4a28_a567_0db78006e6d7);
const PKEY_LINK_TARGET: PropertyKey = PropertyKey { fmtid: GUID::from_u128(0xb9b4b3fc_2b51_4a42_b5d8_324146afcf25), pid: 2 };
const PKEY_LINK_ARGUMENTS: PropertyKey = PropertyKey { fmtid: GUID::from_u128(0x436f2667_14e2_4feb_b30a_146c53b5b674), pid: 100 };
const PKEY_PACKAGE_INSTALL_PATH: PropertyKey =
    PropertyKey { fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3), pid: 15 };
const SIGDN_NORMALDISPLAY: i32 = 0;
const SIGDN_PARENTRELATIVEPARSING: i32 = 0x80018001u32 as i32;

/// Programs that start other things (folders, consoles, control panels):
/// an app that is one of these with arguments is not that program.
const HOSTS: &[&str] = &["explorer", "mmc", "control", "rundll32", "cmd", "powershell", "pwsh", "wscript", "cscript"];

/// Takes a string Windows allocated with CoTaskMemAlloc.
///
/// # Safety
/// `p` is null or a NUL-terminated string from CoTaskMemAlloc.
unsafe fn take_string(p: *mut u16) -> Option<String> {
    if p.is_null() {
        return None;
    }
    // SAFETY: as promised by the caller.
    unsafe {
        let len = (0..).take_while(|&i| *p.add(i) != 0).count();
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, len));
        CoTaskMemFree(p as *const c_void);
        Some(s)
    }
}

/// The program a Store app (packaged app) runs, from its package's
/// manifest: `<Application Id="App" Executable="Spotify.exe" …>`.
fn packaged_program(install_path: &str, app_id: &str) -> Option<String> {
    let manifest = std::fs::read_to_string(Path::new(install_path).join("AppxManifest.xml")).ok()?;
    manifest
        .split("<Application ")
        .skip(1)
        .map(|tag| &tag[..tag.find('>').unwrap_or(tag.len())])
        .find(|tag| attribute(tag, "Id").is_some_and(|id| id.eq_ignore_ascii_case(app_id)))
        .and_then(|tag| attribute(tag, "Executable"))
        .filter(|exe| !exe.contains('$'))
        .map(|exe| format!(r"{install_path}\{exe}"))
}

/// The value of `name="…"` in the text of an XML tag.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(at) = rest.find(name) {
        let before_ok = at == 0 || rest[..at].ends_with(char::is_whitespace);
        let after = rest[at + name.len()..].trim_start();
        if before_ok && let Some(value) = after.strip_prefix('=') {
            let value = value.trim_start();
            let quote = value.chars().next()?;
            if quote == '"' || quote == '\'' {
                return value[1..].split(quote).next();
            }
        }
        rest = &rest[at + name.len()..];
    }
    None
}

/// The rule for one entry of Windows' list of apps, if it is an app Iqal
/// can block (not a web link, document or settings page).
fn rule_for_entry(target: Option<&str>, args: &str, package: Option<(&str, &str)>) -> Option<String> {
    if let Some((install_path, app_id)) = package {
        return rule_for(&packaged_program(install_path, app_id)?, "");
    }
    let target = target?;
    let stem = Path::new(target).file_stem()?.to_str()?.to_lowercase();
    if HOSTS.contains(&stem.as_str()) && !args.trim().is_empty() {
        return None;
    }
    // File Explorer and Iqal itself are never blocked.
    if stem == "explorer" || stem.starts_with("iqal") {
        return None;
    }
    rule_for(target, args)
}

/// Every app in Windows' list of apps ("All apps" in the Start menu):
/// programs with shortcuts, Windows' own tools and Store apps.
fn installed_apps() -> Vec<App> {
    // SAFETY: paired with CoUninitialize below when it succeeds.
    let com = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) } >= 0;
    let mut apps: Vec<App> = Vec::new();
    let folder = win::wide("shell:AppsFolder");
    // SAFETY: COM calls through the interfaces' tables; every object made
    // here is released here (or kept as an owned PIDL), and strings Windows
    // returns are freed by `take_string`.
    unsafe {
        let mut root: *mut c_void = std::ptr::null_mut();
        if SHCreateItemFromParsingName(folder.as_ptr(), std::ptr::null_mut(), &IID_SHELL_ITEM, &mut root) == 0 {
            let root_vtbl = &**(root as *mut *const ShellItemVtbl);
            let mut items: *mut c_void = std::ptr::null_mut();
            if (root_vtbl.bind_to_handler)(root, std::ptr::null_mut(), &BHID_EnumItems, &IID_ENUM_SHELL_ITEMS, &mut items) == 0 {
                let items_vtbl = &**(items as *mut *const EnumShellItemsVtbl);
                loop {
                    let mut item: *mut c_void = std::ptr::null_mut();
                    let mut fetched = 0u32;
                    if (items_vtbl.next)(items, 1, &mut item, &mut fetched) != 0 || fetched == 0 || item.is_null() {
                        break;
                    }
                    let vtbl = &**(item as *mut *const ShellItemVtbl);
                    let mut item2: *mut c_void = std::ptr::null_mut();
                    if (vtbl.query_interface)(item, &IID_SHELL_ITEM2, &mut item2) == 0 {
                        let string = |key: &PropertyKey| {
                            let mut p = std::ptr::null_mut();
                            if (vtbl.get_string)(item2, key, &mut p) == 0 { take_string(p) } else { None }
                        };
                        let display = |kind| {
                            let mut p = std::ptr::null_mut();
                            if (vtbl.get_display_name)(item, kind, &mut p) == 0 { take_string(p) } else { None }
                        };
                        let name = display(SIGDN_NORMALDISPLAY).unwrap_or_default();
                        let id = display(SIGDN_PARENTRELATIVEPARSING).unwrap_or_default();
                        let target = string(&PKEY_LINK_TARGET);
                        let args = string(&PKEY_LINK_ARGUMENTS).unwrap_or_default();
                        let install = string(&PKEY_PACKAGE_INSTALL_PATH);
                        let package = install.as_deref().zip(id.split_once('!').map(|(_, app)| app));
                        if !name.is_empty()
                            && !name.to_lowercase().contains("uninstall")
                            && let Some(rule) = rule_for_entry(target.as_deref(), &args, package)
                        {
                            match apps.iter_mut().find(|a| a.rule == rule) {
                                // Several entries start one program (a
                                // console's shortcuts): keep the plainest.
                                Some(a) if a.name.len() > name.len() => {
                                    a.name = name;
                                    a.icon = Pidl::of(item);
                                }
                                Some(_) => {}
                                None => apps.push(App { name, icon: Pidl::of(item), rule }),
                            }
                        }
                        (vtbl.release)(item2);
                    }
                    (vtbl.release)(item);
                }
                (items_vtbl.release)(items);
            }
            (root_vtbl.release)(root);
        }
    }
    if com {
        // SAFETY: matches the successful CoInitializeEx.
        unsafe { CoUninitialize() };
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// An app's place in the shell, for its icon. Freed when dropped.
struct Pidl(*mut ITEMIDLIST);

impl Pidl {
    /// # Safety
    /// `item` is a live IShellItem.
    unsafe fn of(item: *mut c_void) -> Pidl {
        let mut pidl = std::ptr::null_mut();
        // SAFETY: as promised by the caller; Windows allocates the list.
        unsafe { SHGetIDListFromObject(item, &mut pidl) };
        Pidl(pidl)
    }
}

impl Drop for Pidl {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: a list from SHGetIDListFromObject, freed once.
            unsafe { ILFree(self.0) };
        }
    }
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

/// The titles of the windows open now on this desktop, as the taskbar
/// shows them (not hidden, tool or Iqal's own windows), each once.
fn open_window_titles() -> Vec<String> {
    unsafe extern "system" fn each(hwnd: HWND, data: LPARAM) -> i32 {
        // SAFETY: `data` is the Vec passed below, alive during EnumWindows.
        let titles = unsafe { &mut *(data as *mut Vec<String>) };
        // SAFETY: plain queries about a window; out values are ours.
        unsafe {
            let mut cloaked = 0u32;
            DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as u32, (&raw mut cloaked).cast(), 4);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if IsWindowVisible(hwnd) == 0
                || cloaked != 0
                || !GetWindow(hwnd, GW_OWNER).is_null()
                || GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW != 0
                || pid == GetCurrentProcessId()
            {
                return 1;
            }
        }
        let title = win::text(hwnd).trim().to_string();
        if !title.is_empty() && title != "Program Manager" && !titles.contains(&title) {
            titles.push(title);
        }
        1
    }
    let mut titles: Vec<String> = Vec::new();
    // SAFETY: the callback only reads windows and adds to `titles`.
    unsafe { EnumWindows(Some(each), (&raw mut titles) as LPARAM) };
    titles
}

/// Asks for words of a window title (typed, or taken from a window open
/// now); `on_pick` gets the rule that closes such windows.
pub fn pick_title(owner: HWND, on_pick: impl Fn(String) + 'static) {
    let titles = Rc::new(open_window_titles());
    modal::run(owner, &t("action-add-window-title"), (520, 560), move |page, closer| {
        let intro = page.note(&t("window-title-intro"));
        let words = page.control("EDIT", "", WS_TABSTOP | ES_AUTOHSCROLL as u32, WS_EX_CLIENTEDGE);
        let cue = win::wide(&t("window-title-words"));
        win::send(words, EM_SETCUEBANNER, 1, cue.as_ptr() as isize);
        let open_heading = page.heading(&t("window-title-open"));
        let list = page.control("LISTBOX", "", WS_TABSTOP | WS_VSCROLL | (LBS_NOTIFY | LBS_NOINTEGRALHEIGHT) as u32, WS_EX_CLIENTEDGE);
        for title in titles.iter() {
            let text = win::wide(title);
            win::send(list, LB_ADDSTRING, 0, text.as_ptr() as isize);
        }
        let spacer = page.control("STATIC", "", 0, 0);
        let add = page.control("BUTTON", &t("action-add"), WS_TABSTOP | BS_DEFPUSHBUTTON as u32, 0);
        let cancel = page.control("BUTTON", &t("action-cancel"), WS_TABSTOP, 0);

        // Choosing a window puts its title in the box, to be shortened to
        // the words that matter.
        {
            let titles = titles.clone();
            panel::on(list, move |code| {
                if code == LBN_SELCHANGE
                    && let Some(title) = usize::try_from(win::send(list, LB_GETCURSEL, 0, 0)).ok().and_then(|i| titles.get(i))
                {
                    win::set_text(words, title);
                }
            });
        }
        let on_pick = Rc::new(on_pick);
        {
            let closer = closer.clone();
            panel::on(add, move |code| {
                if code != BN_CLICKED {
                    return;
                }
                let text = win::text(words).trim().to_string();
                if text.is_empty() {
                    dialog::message(closer.hwnd(), dialog::Icon::Warning, &t("window-title-empty"), "");
                    return;
                }
                closer.close();
                on_pick(format!("title:{text}"));
            });
        }
        {
            let closer = closer.clone();
            panel::on(cancel, move |code| {
                if code == BN_CLICKED {
                    closer.close();
                }
            });
        }
        page.set_items(vec![
            Item::Text(intro),
            Item::Full { hwnd: words, height: panel::ROW },
            Item::Space(8),
            Item::Full { hwnd: open_heading, height: 22 },
            Item::Full { hwnd: list, height: 300 },
            Item::Row {
                cells: vec![(spacer, Width::Flex), (add, button_width(&t("action-add"))), (cancel, button_width(&t("action-cancel")))],
                height: panel::ROW,
            },
        ]);
        // SAFETY: our text box.
        unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(words) };
    });
}

/// One row: the app's icon and name.
fn draw(state: &mut State, item: &DRAWITEMSTRUCT) {
    let Some(&index) = state.shown.get(item.itemID as usize) else { return };
    let Some(fonts) = panel::fonts() else { return };
    let icon = *state.icons.0.entry(index).or_insert_with(|| {
        let pidl = state.apps[index].icon.0;
        if pidl.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: a zeroed structure for Windows to fill.
        let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
        // SAFETY: a valid item list (SHGFI_PIDL) and structure.
        unsafe {
            SHGetFileInfoW(pidl as *const u16, 0, &mut info, size_of::<SHFILEINFOW>() as u32, SHGFI_PIDL | SHGFI_ICON | SHGFI_LARGEICON)
        };
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
        FillRect(item.hDC, &item.rcItem, crate::theme::brush(back as i32));
        // Icons keep their own direction in right-to-left windows.
        if !icon.is_null() {
            DrawIconEx(item.hDC, item.rcItem.left + pad, top, icon, size, size, 0, std::ptr::null_mut(), DI_NORMAL | DI_NOMIRROR);
        }
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, crate::theme::color(fore as i32));
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

    #[test]
    fn rules_from_entries() {
        assert_eq!(rule_for_entry(Some(r"C:\Windows\system32\cmd.exe"), "", None).as_deref(), Some("cmd"));
        assert_eq!(rule_for_entry(Some(r"C:\Windows\system32\cmd.exe"), "/k vcvars64.bat", None), None);
        assert_eq!(rule_for_entry(Some(r"C:\Windows\explorer.exe"), "", None), None);
        assert_eq!(rule_for_entry(Some("https://example.com/faq"), "", None), None);
        assert_eq!(rule_for_entry(None, "", None), None);
    }

    /// Shows this PC's apps: `cargo test -p iqal-win32 -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn lists_this_pcs_apps() {
        let apps = installed_apps();
        for a in &apps {
            println!("{:40} {}", a.name, a.rule);
        }
        assert!(!apps.is_empty());
    }

    #[test]
    fn manifest_attributes() {
        let tag = r#"Id="App" Executable="Spotify.exe" EntryPoint='Windows.FullTrustApplication'"#;
        assert_eq!(attribute(tag, "Id"), Some("App"));
        assert_eq!(attribute(tag, "Executable"), Some("Spotify.exe"));
        assert_eq!(attribute(tag, "EntryPoint"), Some("Windows.FullTrustApplication"));
        assert_eq!(attribute(r#"uap:Id="X""#, "Id"), None);
        assert_eq!(attribute(tag, "Missing"), None);
    }
}
