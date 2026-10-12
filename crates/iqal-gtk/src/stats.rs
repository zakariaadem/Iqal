//! The Statistics window: summary cards, blocks per day, and ranked lists
//! with each number next to its name.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use iqal_core::schedule::Day;
use iqal_i18n::{t, ta};
use iqal_ipc::{DayStats, Request, Response, StatsReport};
use gtk::prelude::*;

use crate::ui::Ui;
use crate::widgets::{day_name, hbox, heading, label, note, set_margins, vbox};

const PERIODS: [(u32, &str); 4] = [(1, "stats-today"), (7, "stats-7"), (30, "stats-30"), (90, "stats-90")];
/// Rows shown in a list before "Show all".
const SHORT_LIST: usize = 5;
/// The longest bar in a list, in pixels.
const BAR_WIDTH: f64 = 200.0;
/// The tallest bar in the chart, in pixels.
const CHART_HEIGHT: f64 = 120.0;
const ALLOWANCE_ICON: &str = "alarm-symbolic";

pub fn statistics(ui: &Rc<Ui>) {
    let win = gtk::Window::builder()
        .transient_for(&ui.window)
        .modal(true)
        .title(t("menu-stats"))
        .default_width(820)
        .default_height(760)
        .build();
    let outer = vbox(16);
    set_margins(&outer, 20);

    // Period: one click, not a menu.
    let switcher = hbox(0);
    switcher.add_css_class("linked");
    switcher.set_halign(gtk::Align::Start);
    let content = vbox(22);
    let names = Rc::new(crate::appspicker::display_names());
    let fill = {
        let (weak, content) = (ui.weak(), content.clone());
        Rc::new(move |days: u32| {
            let Some(ui) = weak.upgrade() else { return };
            while let Some(c) = content.first_child() {
                content.remove(&c);
            }
            match ui.service.call(Request::Stats { days }) {
                Ok(Response::Stats { stats }) => build(&ui, &content, &stats, days, &names),
                Ok(_) => ui.error(&t("error-unexpected")),
                Err(e) => ui.error(&e),
            }
        })
    };
    let mut first: Option<gtk::ToggleButton> = None;
    for (days, key) in PERIODS {
        let b = gtk::ToggleButton::with_label(&t(key));
        if let Some(f) = &first {
            b.set_group(Some(f));
        }
        let fill = fill.clone();
        b.connect_toggled(move |b| {
            if b.is_active() {
                fill(days);
            }
        });
        switcher.append(&b);
        first.get_or_insert(b);
    }
    outer.append(&switcher);
    outer.append(&content);
    let footer = hbox(6);
    footer.append(&gtk::Image::from_icon_name("changes-prevent-symbolic"));
    footer.append(&note(&t("stats-private")));
    footer.add_css_class("dim-label");
    outer.append(&footer);

    let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&outer).build();
    win.set_child(Some(&scroll));
    // Last 7 days first.
    if let Some(b) = switcher.first_child().and_then(|c| c.next_sibling()).and_downcast::<gtk::ToggleButton>() {
        b.set_active(true);
    }
    win.present();
}

fn build(ui: &Rc<Ui>, content: &gtk::Box, stats: &StatsReport, days: u32, names: &HashMap<String, String>) {
    let total: u32 = stats.days.iter().map(|d| d.blocked).sum();
    if total == 0 && stats.allowance_by_block.is_empty() {
        let empty = vbox(10);
        empty.set_margin_top(60);
        empty.set_margin_bottom(60);
        let icon = gtk::Image::from_icon_name("security-high-symbolic");
        icon.set_pixel_size(56);
        icon.add_css_class("dim-label");
        empty.append(&icon);
        let text = note(&t("stats-empty"));
        text.set_halign(gtk::Align::Center);
        empty.append(&text);
        content.append(&empty);
        return;
    }

    content.append(&cards(stats, total, days, names));

    if days > 1 {
        let chart_box = vbox(8);
        chart_box.append(&heading(&t("stats-chart")));
        chart_box.append(&chart(&stats.days));
        content.append(&chart_box);
    }

    // Two columns side by side when there is room.
    let lists = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(2)
        .min_children_per_line(1)
        .homogeneous(true)
        .column_spacing(28)
        .row_spacing(22)
        .build();
    let top: Vec<Row> = stats
        .top
        .iter()
        .map(|t| Row {
            icon: if t.web { "web-browser-symbolic" } else { "application-x-executable-symbolic" },
            name: if t.web { t.name.clone() } else { names.get(&t.name).cloned().unwrap_or_else(|| t.name.clone()) },
            detail: (!t.web && names.contains_key(&t.name)).then(|| t.name.clone()),
            value: t.count,
            text: t.count.to_string(),
            limit: None,
        })
        .collect();
    if !top.is_empty() {
        lists.insert(&ranked(&t("stats-top"), top), -1);
    }
    let right = vbox(22);
    if !stats.by_block.is_empty() {
        let rows = stats
            .by_block
            .iter()
            .map(|(name, n)| Row {
                icon: "security-high-symbolic",
                name: name.clone(),
                detail: None,
                value: *n,
                text: n.to_string(),
                limit: None,
            })
            .collect();
        right.append(&ranked(&t("stats-by-block"), rows));
    }
    if !stats.allowance_by_block.is_empty() {
        right.append(&allowances(ui, stats, days));
    }
    if right.first_child().is_some() {
        lists.insert(&right, -1);
    }
    content.append(&lists);
}

/// One summary card: a label, a large number, and a line under it.
fn card(title: &str, value: &str, sub: &str) -> gtk::Box {
    let c = vbox(2);
    c.add_css_class("stat-card");
    let l = note(title);
    l.set_wrap(false);
    c.append(&l);
    let v = gtk::Label::new(Some(value));
    v.add_css_class("stat-value");
    v.set_xalign(0.0);
    v.set_ellipsize(gtk::pango::EllipsizeMode::End);
    c.append(&v);
    let s = note(sub);
    s.add_css_class("caption");
    c.append(&s);
    c
}

fn cards(stats: &StatsReport, total: u32, days: u32, names: &HashMap<String, String>) -> gtk::FlowBox {
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(4)
        .min_children_per_line(2)
        .homogeneous(true)
        .column_spacing(12)
        .row_spacing(12)
        .build();
    let before = match stats.previous {
        Some(n) => ta("stats-before", &[("days", (days as i64).into()), ("count", (n as i64).into())]),
        None => String::new(),
    };
    flow.insert(&card(&t("stats-card-total"), &total.to_string(), &before), -1);
    flow.insert(
        &card(
            &t("stats-card-websites"),
            &stats.websites.to_string(),
            &ta("stats-distinct-sites", &[("count", (stats.distinct_websites as i64).into())]),
        ),
        -1,
    );
    flow.insert(
        &card(
            &t("stats-card-apps"),
            &stats.apps.to_string(),
            &ta("stats-distinct-apps", &[("count", (stats.distinct_apps as i64).into())]),
        ),
        -1,
    );
    if days == 1 {
        // Today: the busiest day is today, so show the top item instead.
        if let Some(top) = stats.top.first() {
            let name = if top.web { top.name.clone() } else { names.get(&top.name).cloned().unwrap_or_else(|| top.name.clone()) };
            flow.insert(&card(&t("stats-card-top"), &name, &blocks(top.count)), -1);
        }
    } else if let Some(busiest) = stats.days.iter().filter(|d| d.blocked > 0).max_by_key(|d| (d.blocked, d.date.clone())) {
        let (weekday, date) = describe_day(busiest);
        flow.insert(&card(&t("stats-card-busiest"), &weekday, &format!("{date} · {}", blocks(busiest.blocked))), -1);
    }
    flow
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

/// Bars for each day, the count on top (when there is room) and the day
/// under it. Hovering a day shows its date and count. Right-to-left
/// languages mirror it like the rest of the window.
fn chart(days: &[DayStats]) -> gtk::Grid {
    let grid = gtk::Grid::builder().column_homogeneous(true).column_spacing(if days.len() > 40 { 1 } else { 4 }).build();
    let max = days.iter().map(|d| d.blocked).max().unwrap_or(0).max(1) as f64;
    let bar_width = match days.len() {
        0..=10 => 34,
        11..=40 => 12,
        _ => 5,
    };
    let last = days.len().saturating_sub(1);
    for (i, d) in days.iter().enumerate() {
        let (weekday, date) = describe_day(d);
        let column = vbox(3);
        column.set_valign(gtk::Align::End);
        column.set_tooltip_text(Some(&format!("{weekday} {date}: {}", blocks(d.blocked))));
        if days.len() <= 14 {
            let value = gtk::Label::new(Some(&d.blocked.to_string()));
            value.add_css_class("caption");
            if d.blocked == 0 {
                value.add_css_class("dim-label");
            }
            column.append(&value);
        }
        let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bar.set_halign(gtk::Align::Center);
        let height = if d.blocked == 0 { 2 } else { ((d.blocked as f64 / max) * CHART_HEIGHT).round().max(3.0) as i32 };
        bar.set_size_request(bar_width, height);
        bar.add_css_class(if d.blocked == 0 { "stat-bar-empty" } else { "stat-bar" });
        column.append(&bar);
        grid.attach(&column, i as i32, 0, 1, 1);
        // A week: each day's name and date. Longer: a date every few days,
        // spread over the following columns so the bars stay narrow.
        let every = match days.len() {
            0..=10 => 1,
            11..=40 => 7,
            _ => 15,
        };
        if i % every == 0 {
            let text = if every == 1 { format!("{weekday}\n{date}") } else { date };
            let day = gtk::Label::new(Some(&text));
            day.add_css_class("caption");
            day.add_css_class(if i == last { "stat-today" } else { "dim-label" });
            if every == 1 {
                day.set_justify(gtk::Justification::Center);
            } else {
                day.set_xalign(0.0);
            }
            grid.attach(&day, i as i32, 1, every.min(days.len() - i) as i32, 1);
        }
    }
    grid
}

/// A line in a ranked list.
struct Row {
    icon: &'static str,
    name: String,
    /// Shown on hover (the program name behind an app's menu name).
    detail: Option<String>,
    value: u32,
    text: String,
    /// For allowances: the bar shows `value` out of this.
    limit: Option<u32>,
}

/// A titled list: icon, name, then a bar with the number right after it.
/// Long lists show the first few with a "Show all" button.
fn ranked(title: &str, rows: Vec<Row>) -> gtk::Box {
    let b = vbox(6);
    b.append(&heading(title));
    let grid = gtk::Grid::builder().column_spacing(10).row_spacing(6).build();
    let max = rows.iter().map(|r| r.limit.unwrap_or(r.value)).max().unwrap_or(0).max(1) as f64;
    let mut extra = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let icon = gtk::Image::from_icon_name(r.icon);
        icon.add_css_class("dim-label");
        let name = label(&r.name);
        name.set_wrap(false);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name.set_width_chars(16);
        name.set_max_width_chars(22);
        name.set_tooltip_text(Some(r.detail.as_deref().unwrap_or(&r.name)));
        let bar_row = hbox(8);
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        bar.set_valign(gtk::Align::Center);
        bar.set_size_request(((r.value as f64 / max) * BAR_WIDTH).round().max(3.0) as i32, 10);
        bar.add_css_class(if r.icon == ALLOWANCE_ICON { "stat-bar-allowance" } else { "stat-bar" });
        bar_row.append(&bar);
        let count = gtk::Label::new(Some(&r.text));
        count.add_css_class("heading");
        bar_row.append(&count);
        let row = i as i32;
        grid.attach(&icon, 0, row, 1, 1);
        grid.attach(&name, 1, row, 1, 1);
        grid.attach(&bar_row, 2, row, 1, 1);
        if i >= SHORT_LIST {
            for w in [icon.upcast::<gtk::Widget>(), name.upcast(), bar_row.upcast()] {
                w.set_visible(false);
                extra.push(w);
            }
        }
    }
    b.append(&grid);
    if !extra.is_empty() {
        let more = gtk::Button::with_label(&ta("stats-show-all", &[("count", (rows.len() as i64).into())]));
        more.add_css_class("flat");
        more.set_halign(gtk::Align::Start);
        let open = Rc::new(Cell::new(false));
        let total = rows.len() as i64;
        more.connect_clicked(move |m| {
            let now = !open.get();
            open.set(now);
            for w in &extra {
                w.set_visible(now);
            }
            m.set_label(&if now { t("stats-show-fewer") } else { ta("stats-show-all", &[("count", total.into())]) });
        });
        b.append(&more);
    }
    b
}

/// Time used from each block's daily allowance: "21 of 30 min" for today,
/// the total for longer periods.
fn allowances(ui: &Rc<Ui>, stats: &StatsReport, days: u32) -> gtk::Box {
    let limits: HashMap<String, u32> = ui
        .blocks()
        .into_iter()
        .filter_map(|i| Some((i.set.name.clone(), i.set.allowance?.daily_secs / 60)))
        .collect();
    let rows = stats
        .allowance_by_block
        .iter()
        .map(|(name, secs)| {
            let used = secs / 60;
            let limit = limits.get(name).copied().filter(|_| days == 1);
            Row {
                icon: ALLOWANCE_ICON,
                name: name.clone(),
                detail: None,
                value: used,
                text: match limit {
                    Some(total) => ta("stats-allowance-of", &[("used", (used as i64).into()), ("total", (total as i64).into())]),
                    None => ta("stats-minutes", &[("minutes", (used as i64).into())]),
                },
                limit,
            }
        })
        .collect();
    ranked(&t("stats-allowance"), rows)
}
