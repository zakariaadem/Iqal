//! The Statistics window: summary cards, blocks per day, and ranked lists
//! with each number next to its name. The view below the period buttons is
//! drawn by hand (Windows' controls have no cards or charts).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use iqal_app::describe::day_name;
use iqal_core::schedule::Day;
use iqal_i18n::{t, ta};
use iqal_ipc::{DayStats, Request, Response, StatsReport};
use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, COLOR_GRAYTEXT, COLOR_HOTLIGHT, COLOR_WINDOW, COLOR_WINDOWTEXT, CreateSolidBrush, DT_CALCRECT,
    DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RTLREADING, DT_SINGLELINE, DT_VCENTER,
    DeleteObject, DrawTextW, EndPaint, FillRect, HDC, HFONT, InvalidateRect,
    PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_SETCHECK, BN_CLICKED, BS_AUTORADIOBUTTON, BS_PUSHLIKE, CreateWindowExW, DefWindowProcW, IDC_HAND,
    LoadCursorW, RegisterClassExW, SetCursor, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WNDCLASSEXW, WS_CHILD,
    WS_GROUP, WS_TABSTOP, WS_VISIBLE,
};

use crate::modal;
use crate::panel::{self, Item, width_of};
use crate::win::{self, scale};
use crate::{dialog, ui};

const CLASS: &str = "IqalStatsView";
const PERIODS: [(u32, &str); 4] = [(1, "stats-today"), (7, "stats-7"), (30, "stats-30"), (90, "stats-90")];
/// Rows shown in a list before "Show all".
const SHORT_LIST: usize = 5;
/// Sizes in pixels at 96 DPI.
const WIDTH: i32 = 820;
const CARD_HEIGHT: i32 = 92;
const CHART_HEIGHT: i32 = 120;
const ROW: i32 = 26;
const GAP: i32 = 16;
const GREEN: COLORREF = rgb(0x1D, 0x9E, 0x75);
const AMBER: COLORREF = rgb(0xBA, 0x75, 0x17);

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

/// A line in a ranked list.
struct Row {
    name: String,
    value: u32,
    text: String,
    /// For allowances: the bar shows `value` out of this.
    limit: Option<u32>,
    allowance: bool,
}

struct List {
    title: String,
    rows: Vec<Row>,
    open: bool,
}

/// What the view shows, and where its "Show all" links are.
struct View {
    stats: Option<StatsReport>,
    days: u32,
    lists: Vec<List>,
    links: Vec<(RECT, usize)>,
}

thread_local! {
    static VIEW: RefCell<Option<View>> = const { RefCell::new(None) };
    /// Lays the window out again for the view's current content.
    static RELAYOUT: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
}

pub fn open() {
    let owner = ui::main_window();
    modal::run(owner, &t("menu-stats"), (WIDTH, 760), |page, closer| {
        register_class();
        let mut cells = Vec::new();
        let mut buttons = Vec::new();
        for (i, (days, key)) in PERIODS.iter().enumerate() {
            let group = if i == 0 { WS_GROUP } else { 0 };
            let b = page.control("BUTTON", &t(key), WS_TABSTOP | group | (BS_AUTORADIOBUTTON | BS_PUSHLIKE) as u32, 0);
            cells.push((b, width_of(b)));
            buttons.push((b, *days));
        }
        let class = win::wide(CLASS);
        // SAFETY: our registered class; the panel is a live window.
        let view = unsafe {
            CreateWindowExW(0, class.as_ptr(), std::ptr::null(), WS_CHILD | WS_VISIBLE, 0, 0, 0, 0, page.hwnd, std::ptr::null_mut(), win::instance(), std::ptr::null())
        };
        VIEW.with(|v| *v.borrow_mut() = Some(View { stats: None, days: 7, lists: Vec::new(), links: Vec::new() }));
        let note = page.note(&t("stats-private"));
        let relayout: Rc<dyn Fn()> = {
            let closer = closer.clone();
            Rc::new(move || {
                let height = content_height();
                page.set_items(vec![
                    Item::Row { cells: cells.clone(), height: panel::ROW },
                    Item::Full { hwnd: view, height },
                    Item::Text(note),
                ]);
                closer.fit();
                // SAFETY: our view.
                unsafe { InvalidateRect(view, std::ptr::null(), 1) };
            })
        };
        RELAYOUT.with(|r| *r.borrow_mut() = Some(relayout.clone()));
        let fill = move |days: u32| {
            match ui::call(Request::Stats { days }) {
                Ok(Response::Stats { stats }) => set_stats(*stats, days),
                Ok(_) => dialog::error(closer.hwnd(), &t("error-unexpected")),
                Err(e) => dialog::error(closer.hwnd(), &e),
            }
            relayout();
        };
        let fill = Rc::new(fill);
        for &(b, days) in &buttons {
            let fill = fill.clone();
            panel::on(b, move |code| {
                if code == BN_CLICKED {
                    fill(days);
                }
            });
        }
        // The last 7 days first.
        win::send(buttons[1].0, BM_SETCHECK, 1, 0);
        fill(7);
    });
    VIEW.with(|v| *v.borrow_mut() = None);
    RELAYOUT.with(|r| *r.borrow_mut() = None);
}

fn set_stats(stats: StatsReport, days: u32) {
    let limits: HashMap<String, u32> = ui::blocks()
        .into_iter()
        .filter_map(|i| Some((i.set.name.clone(), i.set.allowance?.daily_secs / 60)))
        .collect();
    let mut lists = Vec::new();
    if !stats.top.is_empty() {
        let rows = stats
            .top
            .iter()
            .map(|t| Row { name: t.name.clone(), value: t.count, text: t.count.to_string(), limit: None, allowance: false })
            .collect();
        lists.push(List { title: t("stats-top"), rows, open: false });
    }
    if !stats.by_block.is_empty() {
        let rows = stats
            .by_block
            .iter()
            .map(|(name, n)| Row { name: name.clone(), value: *n, text: n.to_string(), limit: None, allowance: false })
            .collect();
        lists.push(List { title: t("stats-by-block"), rows, open: false });
    }
    if !stats.allowance_by_block.is_empty() {
        // "21 of 30 min" for today, the total for longer periods.
        let rows = stats
            .allowance_by_block
            .iter()
            .map(|(name, secs)| {
                let used = secs / 60;
                let limit = limits.get(name).copied().filter(|_| days == 1);
                let text = match limit {
                    Some(total) => ta("stats-allowance-of", &[("used", (used as i64).into()), ("total", (total as i64).into())]),
                    None => ta("stats-minutes", &[("minutes", (used as i64).into())]),
                };
                Row { name: name.clone(), value: used, text, limit, allowance: true }
            })
            .collect();
        lists.push(List { title: t("stats-allowance"), rows, open: false });
    }
    VIEW.with(|v| {
        if let Some(v) = v.borrow_mut().as_mut() {
            v.stats = Some(stats);
            v.days = days;
            v.lists = lists;
        }
    });
}

/// The view's height for its content, in pixels at 96 DPI.
fn content_height() -> i32 {
    VIEW.with(|v| {
        let v = v.borrow();
        let Some(v) = v.as_ref() else { return 0 };
        let Some(stats) = &v.stats else { return 0 };
        if is_empty(stats) {
            return 160;
        }
        let mut h = CARD_HEIGHT + GAP;
        if v.days > 1 {
            h += 24 + CHART_HEIGHT + 40 + GAP;
        }
        let list_height = |l: &List| {
            let shown = if l.open { l.rows.len() } else { l.rows.len().min(SHORT_LIST) };
            let link = if l.rows.len() > SHORT_LIST { ROW } else { 0 };
            26 + shown as i32 * ROW + link + GAP
        };
        // Left: the first list; right: the others.
        let left = v.lists.first().map_or(0, list_height);
        let right: i32 = v.lists.iter().skip(1).map(list_height).sum();
        h + left.max(right)
    })
}

fn is_empty(stats: &StatsReport) -> bool {
    stats.days.iter().all(|d| d.blocked == 0) && stats.allowance_by_block.is_empty()
}

fn blocks(count: u32) -> String {
    ta("stats-blocks", &[("count", (count as i64).into())])
}

/// ("Mon", "12/10") for a day's statistics.
fn describe_day(d: &DayStats) -> (String, String) {
    match d.date.parse::<jiff::civil::Date>() {
        Ok(date) => {
            let day = match date.weekday() {
                jiff::civil::Weekday::Monday => Day::Mon,
                jiff::civil::Weekday::Tuesday => Day::Tue,
                jiff::civil::Weekday::Wednesday => Day::Wed,
                jiff::civil::Weekday::Thursday => Day::Thu,
                jiff::civil::Weekday::Friday => Day::Fri,
                jiff::civil::Weekday::Saturday => Day::Sat,
                jiff::civil::Weekday::Sunday => Day::Sun,
            };
            (day_name(day), format!("{}/{}", date.day(), date.month()))
        }
        Err(_) => (d.date.clone(), String::new()),
    }
}

/// Mixes two colours: `part` (out of 100) of `b` into `a`.
fn mix(a: COLORREF, b: COLORREF, part: u32) -> COLORREF {
    let channel = |shift: u32| {
        let (x, y) = ((a >> shift) & 0xff, (b >> shift) & 0xff);
        ((x * (100 - part) + y * part) / 100) << shift
    };
    channel(0) | channel(8) | channel(16)
}

struct Painter {
    hdc: HDC,
    dpi: u32,
}

impl Painter {
    fn s(&self, dip: i32) -> i32 {
        scale(dip, self.dpi)
    }

    fn fill(&self, r: &RECT, colour: COLORREF) {
        // SAFETY: a brush made and deleted here; painting into hdc.
        unsafe {
            let brush = CreateSolidBrush(colour);
            FillRect(self.hdc, r, brush);
            DeleteObject(brush);
        }
    }

    /// Draws text in `r`; returns its width.
    fn text(&self, font: HFONT, colour: COLORREF, text: &str, r: RECT, format: u32) -> i32 {
        let rtl = if iqal_i18n::is_rtl() { DT_RTLREADING } else { 0 };
        // Ends in a zero: DrawTextW reads through the pointer even for an empty text.
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let len = wide.len() as i32 - 1;
        let mut measure = r;
        // SAFETY: painting into hdc with valid text and rectangles.
        unsafe {
            SelectObject(self.hdc, font);
            SetTextColor(self.hdc, colour);
            DrawTextW(self.hdc, wide.as_ptr(), len, &mut measure, format | DT_NOPREFIX | rtl | DT_CALCRECT);
            let mut r = r;
            DrawTextW(self.hdc, wide.as_ptr(), len, &mut r, format | DT_NOPREFIX | rtl);
        }
        measure.right - measure.left
    }
}

fn paint(hwnd: HWND) {
    // SAFETY: a PAINTSTRUCT for BeginPaint to fill.
    let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    // SAFETY: our window, inside WM_PAINT.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    let client = win::client_rect(hwnd);
    let p = Painter { hdc, dpi: panel::dpi() };
    let fonts = panel::fonts();
    VIEW.with(|v| {
        let mut v = v.borrow_mut();
        let (Some(v), Some(fonts)) = (v.as_mut(), fonts) else { return };
        // SAFETY: painting into hdc.
        unsafe {
            FillRect(hdc, &client, crate::theme::brush(COLOR_WINDOW as i32));
            SetBkMode(hdc, TRANSPARENT as i32);
        }
        v.links.clear();
        let Some(stats) = &v.stats else { return };
        let (text, grey, back) = (
            crate::theme::color(COLOR_WINDOWTEXT as i32),
            crate::theme::color(COLOR_GRAYTEXT as i32),
            crate::theme::color(COLOR_WINDOW as i32),
        );
        if is_empty(stats) {
            let r = RECT { top: p.s(60), ..client };
            p.text(fonts.normal, grey, &t("stats-empty"), r, DT_CENTER | DT_SINGLELINE);
            return;
        }
        let width = client.right;
        let mut y = 0;

        // Summary cards.
        let total: u32 = stats.days.iter().map(|d| d.blocked).sum();
        let before = match stats.previous {
            Some(n) => ta("stats-before", &[("days", (v.days as i64).into()), ("count", (n as i64).into())]),
            None => String::new(),
        };
        let mut cards = vec![
            (t("stats-card-total"), total.to_string(), before),
            (
                t("stats-card-websites"),
                stats.websites.to_string(),
                ta("stats-distinct-sites", &[("count", (stats.distinct_websites as i64).into())]),
            ),
            (t("stats-card-apps"), stats.apps.to_string(), ta("stats-distinct-apps", &[("count", (stats.distinct_apps as i64).into())])),
        ];
        if v.days == 1 {
            // Today: the busiest day is today, so show the top item instead.
            if let Some(top) = stats.top.first() {
                cards.push((t("stats-card-top"), top.name.clone(), blocks(top.count)));
            }
        } else if let Some(busiest) = stats.days.iter().filter(|d| d.blocked > 0).max_by_key(|d| (d.blocked, d.date.clone())) {
            let (weekday, date) = describe_day(busiest);
            cards.push((t("stats-card-busiest"), weekday, format!("{date} · {}", blocks(busiest.blocked))));
        }
        let gap = p.s(12);
        let card_width = (width - gap * 3) / 4;
        for (i, (title, value, sub)) in cards.iter().enumerate() {
            let x = i as i32 * (card_width + gap);
            let card = RECT { left: x, top: y, right: x + card_width, bottom: y + p.s(CARD_HEIGHT) };
            p.fill(&card, mix(back, text, 6));
            let inner = |top: i32, h: i32| RECT { left: card.left + p.s(12), right: card.right - p.s(12), top: card.top + p.s(top), bottom: card.top + p.s(top + h) };
            p.text(fonts.normal, grey, title, inner(10, 20), DT_SINGLELINE | DT_END_ELLIPSIS);
            p.text(fonts.title, text, value, inner(32, 32), DT_SINGLELINE | DT_END_ELLIPSIS | DT_VCENTER);
            p.text(fonts.normal, grey, sub, inner(66, 20), DT_SINGLELINE | DT_END_ELLIPSIS);
        }
        y += p.s(CARD_HEIGHT + GAP);

        // Blocks per day.
        if v.days > 1 {
            p.text(fonts.bold, text, &t("stats-chart"), RECT { left: 0, right: width, top: y, bottom: y + p.s(22) }, DT_SINGLELINE);
            y += p.s(24);
            let n = stats.days.len().max(1) as i32;
            let column = width / n;
            let bar = match n {
                0..=10 => p.s(34),
                11..=40 => p.s(12),
                _ => p.s(5),
            }
            .min(column - 2);
            let max = stats.days.iter().map(|d| d.blocked).max().unwrap_or(0).max(1) as i32;
            let base = y + p.s(CHART_HEIGHT);
            let every = match n {
                0..=10 => 1,
                11..=40 => 7,
                _ => 15,
            };
            for (i, d) in stats.days.iter().enumerate() {
                let cx = i as i32 * column + column / 2;
                let h = if d.blocked == 0 { p.s(2) } else { (d.blocked as i32 * p.s(CHART_HEIGHT - 20) / max).max(p.s(3)) };
                let r = RECT { left: cx - bar / 2, right: cx + bar / 2, top: base - h, bottom: base };
                p.fill(&r, if d.blocked == 0 { mix(back, text, 20) } else { GREEN });
                if n <= 14 {
                    let label = RECT { left: cx - column / 2, right: cx + column / 2, top: base - h - p.s(20), bottom: base - h };
                    p.text(fonts.normal, if d.blocked == 0 { grey } else { text }, &d.blocked.to_string(), label, DT_CENTER | DT_SINGLELINE);
                }
                if i % every == 0 {
                    let (weekday, date) = describe_day(d);
                    let today = i + 1 == stats.days.len();
                    let font = if today { fonts.bold } else { fonts.normal };
                    let colour = if today { text } else { grey };
                    if every == 1 {
                        let r = RECT { left: cx - column / 2, right: cx + column / 2, top: base + p.s(4), bottom: base + p.s(22) };
                        p.text(font, colour, &weekday, r, DT_CENTER | DT_SINGLELINE);
                        let r = RECT { top: base + p.s(20), bottom: base + p.s(38), ..r };
                        p.text(font, colour, &date, r, DT_CENTER | DT_SINGLELINE);
                    } else {
                        let r = RECT { left: cx - column / 2, right: cx - column / 2 + column * every as i32, top: base + p.s(4), bottom: base + p.s(22) };
                        p.text(font, colour, &date, r, DT_LEFT | DT_SINGLELINE);
                    }
                }
            }
            y = base + p.s(40 + GAP);
        }

        // Ranked lists: the first on the left, the others on the right.
        let column_gap = p.s(28);
        let half = (width - column_gap) / 2;
        let mut links = Vec::new();
        let (mut left_y, mut right_y) = (y, y);
        for (index, list) in v.lists.iter().enumerate() {
            let (x, cy) = if index == 0 { (0, &mut left_y) } else { (half + column_gap, &mut right_y) };
            p.text(fonts.bold, text, &list.title, RECT { left: x, right: x + half, top: *cy, bottom: *cy + p.s(22) }, DT_SINGLELINE);
            *cy += p.s(26);
            let max = list.rows.iter().map(|r| r.limit.unwrap_or(r.value)).max().unwrap_or(0).max(1) as i32;
            let name_width = half * 2 / 5;
            let bar_space = half - name_width - p.s(90);
            let shown = if list.open { list.rows.len() } else { list.rows.len().min(SHORT_LIST) };
            for row in &list.rows[..shown] {
                let name = RECT { left: x, right: x + name_width - p.s(8), top: *cy, bottom: *cy + p.s(ROW) };
                p.text(fonts.normal, text, &row.name, name, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                let bar_width = (row.value as i32 * bar_space / max).max(p.s(3));
                let bar_x = x + name_width;
                let mid = *cy + p.s(ROW) / 2;
                p.fill(
                    &RECT { left: bar_x, right: bar_x + bar_width, top: mid - p.s(5), bottom: mid + p.s(5) },
                    if row.allowance { AMBER } else { GREEN },
                );
                let count = RECT { left: bar_x + bar_width + p.s(8), right: x + half, top: *cy, bottom: *cy + p.s(ROW) };
                p.text(fonts.bold, text, &row.text, count, DT_SINGLELINE | DT_VCENTER);
                *cy += p.s(ROW);
            }
            if list.rows.len() > SHORT_LIST {
                let label = if list.open {
                    t("stats-show-fewer")
                } else {
                    ta("stats-show-all", &[("count", (list.rows.len() as i64).into())])
                };
                let link_colour = crate::theme::color(COLOR_HOTLIGHT as i32);
                let r = RECT { left: x, right: x + half, top: *cy, bottom: *cy + p.s(ROW) };
                let w = p.text(fonts.normal, link_colour, &label, r, DT_SINGLELINE | DT_VCENTER);
                links.push((RECT { right: x + w, ..r }, index));
                *cy += p.s(ROW);
            }
            *cy += p.s(GAP);
        }
        v.links = links;
    });
    // SAFETY: matches BeginPaint.
    unsafe { EndPaint(hwnd, &ps) };
}

/// The "Show all" link under `(x, y)`, if any.
fn link_at(lparam: LPARAM) -> Option<usize> {
    let (x, y) = ((lparam & 0xffff) as i16 as i32, ((lparam >> 16) & 0xffff) as i16 as i32);
    VIEW.with(|v| {
        v.borrow().as_ref().and_then(|v| {
            v.links.iter().find(|(r, _)| x >= r.left && x < r.right && y >= r.top && y < r.bottom).map(|(_, i)| *i)
        })
    })
}

fn register_class() {
    thread_local! { static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    let class = win::wide(CLASS);
    // SAFETY: the class name is copied by Windows during registration.
    unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: win::instance(),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            // Painted in WM_ERASEBKGND, in the app's light or dark colour.
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_MOUSEMOVE => {
            let cursor = if link_at(lparam).is_some() { IDC_HAND } else { windows_sys::Win32::UI::WindowsAndMessaging::IDC_ARROW };
            // SAFETY: a system cursor.
            unsafe { SetCursor(LoadCursorW(std::ptr::null_mut(), cursor)) };
            0
        }
        WM_LBUTTONUP => {
            if let Some(i) = link_at(lparam) {
                // "Show all" / "Show fewer": the view changes height.
                VIEW.with(|v| {
                    if let Some(list) = v.borrow_mut().as_mut().and_then(|v| v.lists.get_mut(i)) {
                        list.open = !list.open;
                    }
                });
                resize_view();
            }
            0
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Gives the view the height of its content after a list opened or closed.
fn resize_view() {
    if let Some(relayout) = RELAYOUT.with(|r| r.borrow().clone()) {
        relayout();
    }
}
