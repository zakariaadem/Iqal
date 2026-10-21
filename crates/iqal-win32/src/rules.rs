//! Editing a rule list: a box to type or paste new rules with an Add
//! button, next to the saved list, which can be searched and from which
//! selected rules can be removed. The rules of chosen pre-made lists are
//! shown after the block's own, greyed and marked with the list's name;
//! they cannot be removed one by one (the list is switched off instead).
//!
//! The list draws only the rows on screen (a "no data" list box), so lists
//! with hundreds of thousands of websites stay quick.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use iqal_app::draft::ListKey;
use iqal_i18n::{t, ta};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DT_END_ELLIPSIS, DT_NOPREFIX,
    DT_RTLREADING, DT_SINGLELINE, DT_VCENTER, DrawTextW, FillRect, InvalidateRect,
    SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_SELECTED};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, EN_CHANGE, ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_MULTILINE, ES_WANTRETURN, LB_GETSELCOUNT,
    LB_GETSELITEMS, LB_SETITEMHEIGHT, LBS_EXTENDEDSEL, LBS_NODATA, LBS_NOINTEGRALHEIGHT, LBS_NOTIFY,
    LBS_OWNERDRAWFIXED, WS_EX_CLIENTEDGE, WS_TABSTOP, WS_VSCROLL,
};

use crate::panel::{self, Item, Panel, button_width};
use crate::ui;
use crate::win::{self, scale};

const LB_SETCOUNT: u32 = 0x01A7;
const EM_SETCUEBANNER: u32 = 0x1501;
/// Heights at 96 DPI.
const BOX_HEIGHT: i32 = 180;
const LINE_HEIGHT: i32 = 24;

struct State {
    /// The block's own rules, as edited.
    own: Vec<String>,
    /// The chosen pre-made lists: (name, websites).
    lists: Vec<(String, Rc<Vec<String>>)>,
    /// Their websites shown, each once: (list, website) indexes into `lists`
    /// (no copies: a list can have tens of thousands).
    listed: Vec<(u32, u32)>,
    /// What the search box asks for, lower case.
    filter: String,
    /// The rows shown: indexes into `own`, then into `listed` (after
    /// `own.len()`).
    shown: Vec<usize>,
}

impl State {
    fn refilter(&mut self) {
        let matches = |s: &str| self.filter.is_empty() || s.to_lowercase().contains(&self.filter);
        let own = self.own.iter().enumerate().filter(|(_, r)| matches(r)).map(|(i, _)| i);
        let listed = self
            .listed
            .iter()
            .enumerate()
            .filter(|(_, (l, r))| matches(&self.lists[*l as usize].1[*r as usize]))
            .map(|(i, _)| self.own.len() + i);
        self.shown = own.chain(listed).collect();
    }
}

pub struct RuleEditor {
    pub items: Vec<Item>,
    /// Adds rules to the list; returns how many were new.
    pub add: Rc<dyn Fn(Vec<String>) -> usize>,
}

/// An editor for one list. `lists` are the chosen pre-made lists (name,
/// websites); `extra` are more buttons shown next to Add.
pub fn editor(
    page: Panel,
    key: ListKey,
    add_title: &str,
    list_title: &str,
    examples: &str,
    lists: &[(String, Rc<Vec<String>>)],
    extra: &[HWND],
) -> RuleEditor {
    let own: Vec<String> =
        ui::list_text(key).lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
    let mut seen: HashSet<&str> = own.iter().map(String::as_str).collect();
    let listed: Vec<(u32, u32)> = lists
        .iter()
        .enumerate()
        .flat_map(|(l, (_, rules))| rules.iter().enumerate().map(move |(r, rule)| (l as u32, r as u32, rule)))
        .filter(|&(_, _, rule)| seen.insert(rule.as_str()))
        .map(|(l, r, _)| (l, r))
        .collect();
    let state = Rc::new(RefCell::new(State { own, lists: lists.to_vec(), listed, filter: String::new(), shown: Vec::new() }));
    state.borrow_mut().refilter();

    // Left: adding.
    let add_heading = page.heading(add_title);
    let input = page.control(
        "EDIT",
        "",
        WS_TABSTOP | WS_VSCROLL | (ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN | ES_AUTOHSCROLL) as u32,
        WS_EX_CLIENTEDGE,
    );
    let hint = page.note(&ta("list-empty-hint", &[("examples", examples.to_string().into())]));
    let problem = page.control("STATIC", "", 0x2000, 0);
    let add_button = page.control("BUTTON", &t("action-add"), WS_TABSTOP, 0);

    // Right: the list.
    let list_heading = page.heading(list_title);
    let search = page.control("EDIT", "", WS_TABSTOP | ES_AUTOHSCROLL as u32, WS_EX_CLIENTEDGE);
    let cue = win::wide(&t("search"));
    win::send(search, EM_SETCUEBANNER, 1, cue.as_ptr() as isize);
    let list = page.control(
        "LISTBOX",
        "",
        WS_TABSTOP | WS_VSCROLL | (LBS_OWNERDRAWFIXED | LBS_NODATA | LBS_EXTENDEDSEL | LBS_NOINTEGRALHEIGHT | LBS_NOTIFY) as u32,
        WS_EX_CLIENTEDGE,
    );
    win::send(list, LB_SETITEMHEIGHT, 0, scale(LINE_HEIGHT, panel::dpi()) as isize);
    win::send(list, LB_SETCOUNT, state.borrow().shown.len(), 0);
    let remove = page.control("BUTTON", &t("action-remove"), WS_TABSTOP, 0);

    // Shows the current rows again.
    let reshow = {
        let state = state.clone();
        move || {
            let count = state.borrow().shown.len();
            win::send(list, LB_SETCOUNT, count, 0);
            // SAFETY: our list.
            unsafe { InvalidateRect(list, std::ptr::null(), 1) };
        }
    };

    let add: Rc<dyn Fn(Vec<String>) -> usize> = {
        let (state, reshow) = (state.clone(), reshow.clone());
        Rc::new(move |lines: Vec<String>| {
            let (added, text) = {
                let mut st = state.borrow_mut();
                let mut added = 0;
                for line in lines {
                    if !st.own.contains(&line) {
                        st.own.push(line);
                        added += 1;
                    }
                }
                st.refilter();
                (added, st.own.join("\n"))
            };
            ui::set_list_text(key, text);
            reshow();
            added
        })
    };

    {
        let add = add.clone();
        panel::on(add_button, move |code| {
            if code != BN_CLICKED {
                return;
            }
            let text = win::text(input);
            let mut good = Vec::new();
            let mut bad: Vec<(String, String)> = Vec::new();
            for line in text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                match key.check(line) {
                    Ok(()) => good.push(line.to_string()),
                    Err(why) => bad.push((line.to_string(), why)),
                }
            }
            add(good);
            // Rules that cannot be used stay in the box, for fixing.
            let left: Vec<&str> = bad.iter().map(|(l, _)| l.as_str()).collect();
            win::set_text(input, &left.join("\r\n"));
            let message = match bad.first() {
                None => String::new(),
                Some((line, why)) => {
                    let mut m = ta("rule-invalid", &[("rule", line.clone().into()), ("problem", why.clone().into())]);
                    if bad.len() > 1 {
                        m.push(' ');
                        m.push_str(&ta("lines-invalid-more", &[("count", (bad.len() as i64 - 1).into())]));
                    }
                    m
                }
            };
            win::set_text(problem, &message);
            page.set_hidden(problem, message.is_empty());
        });
    }
    {
        let (state, reshow) = (state.clone(), reshow.clone());
        panel::on(search, move |code| {
            if code == EN_CHANGE as u32 {
                let filter = win::text(search).trim().to_lowercase();
                {
                    let mut st = state.borrow_mut();
                    st.filter = filter;
                    st.refilter();
                }
                reshow();
            }
        });
    }
    {
        let (state, reshow) = (state.clone(), reshow.clone());
        panel::on(remove, move |code| {
            if code != BN_CLICKED {
                return;
            }
            let count = win::send(list, LB_GETSELCOUNT, 0, 0).max(0) as usize;
            let mut rows = vec![0i32; count];
            win::send(list, LB_GETSELITEMS, count, rows.as_mut_ptr() as isize);
            let text = {
                let mut st = state.borrow_mut();
                // Only the block's own rules; pre-made lists are switched off
                // as a whole.
                let chosen: HashSet<usize> =
                    rows.iter().filter_map(|&r| st.shown.get(r as usize).copied()).filter(|&i| i < st.own.len()).collect();
                if chosen.is_empty() {
                    return;
                }
                let kept: Vec<String> =
                    st.own.iter().enumerate().filter(|(i, _)| !chosen.contains(i)).map(|(_, r)| r.clone()).collect();
                st.own = kept;
                st.refilter();
                st.own.join("\n")
            };
            ui::set_list_text(key, text);
            reshow();
        });
    }
    {
        let state = state.clone();
        panel::on_draw(list, move |item| draw(&state.borrow(), item));
    }

    page.set_hidden(problem, true);
    let mut left = vec![
        Item::Full { hwnd: add_heading, height: 22 },
        Item::Full { hwnd: input, height: BOX_HEIGHT },
        Item::Text(problem),
        Item::Row { cells: vec![(add_button, button_width(&t("action-add")))], height: panel::ROW },
    ];
    // More ways to add on a row of their own: next to Add they would not fit.
    // Two to a row, so they fit a narrow column.
    for pair in extra.chunks(2) {
        left.push(Item::Row { cells: pair.iter().map(|&b| (b, button_width(&win::text(b)))).collect(), height: panel::ROW });
    }
    left.push(Item::Text(hint));
    let items = vec![Item::Columns {
        left,
        right: vec![
            Item::Full { hwnd: list_heading, height: 22 },
            Item::Full { hwnd: search, height: panel::ROW },
            Item::Full { hwnd: list, height: BOX_HEIGHT - panel::ROW - 8 },
            Item::Row { cells: vec![(remove, button_width(&t("action-remove")))], height: panel::ROW },
        ],
    }];
    RuleEditor { items, add }
}

/// One row: a rule of the block, or (greyed) of a pre-made list.
fn draw(state: &State, item: &DRAWITEMSTRUCT) {
    let Some(&index) = state.shown.get(item.itemID as usize) else { return };
    let (text, from_list) = match index.checked_sub(state.own.len()) {
        None => (state.own[index].clone(), false),
        Some(i) => {
            let (l, r) = state.listed[i];
            let (name, rules) = &state.lists[l as usize];
            (format!("{}   ·   {}", rules[r as usize], name), true)
        }
    };
    let selected = item.itemState & ODS_SELECTED != 0;
    let (back, fore) = match (selected, from_list) {
        (true, _) => (COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT),
        (false, true) => (COLOR_WINDOW, COLOR_GRAYTEXT),
        (false, false) => (COLOR_WINDOW, COLOR_WINDOWTEXT),
    };
    let Some(fonts) = panel::fonts() else { return };
    let pad = scale(6, panel::dpi());
    let mut r = RECT { left: item.rcItem.left + pad, right: item.rcItem.right - pad, ..item.rcItem };
    let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
    // Ends in a zero: DrawTextW reads through the pointer even for an empty text.
    let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    // SAFETY: drawing into the list's DC during WM_DRAWITEM.
    unsafe {
        FillRect(item.hDC, &item.rcItem, crate::theme::brush(back as i32));
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, crate::theme::color(fore as i32));
        SelectObject(item.hDC, fonts.normal);
        DrawTextW(
            item.hDC,
            text.as_ptr(),
            text.len() as i32 - 1,
            &mut r,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX | rtl,
        );
    }
}
