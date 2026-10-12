//! Small reusable widgets and text helpers.

use al_dir_core::schedule::{Day, Schedule};
use al_dir_core::{GeoArea, Lock, LockKind};
use al_dir_i18n::{t, ta};
use gtk::glib;
use gtk::prelude::*;

pub fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, spacing)
}

pub fn hbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, spacing)
}

/// A page body inside a scrolled window, with comfortable margins.
pub fn page() -> (gtk::ScrolledWindow, gtk::Box) {
    let body = vbox(14);
    set_margins(&body, 18);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&body)
        .build();
    (scroll, body)
}

pub fn heading(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("heading");
    l.set_xalign(0.0);
    l
}

pub fn note(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("dim-label");
    l.set_xalign(0.0);
    l.set_wrap(true);
    l
}

pub fn label(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    l.set_wrap(true);
    l
}

/// A list of rules with a remove button each, and an entry to add more.
pub fn rule_list(
    title: &str,
    rules: Vec<String>,
    placeholder: &str,
    on_add: impl Fn(String) + 'static,
    on_remove: impl Fn(String) + 'static,
) -> gtk::Box {
    let b = vbox(6);
    b.append(&heading(title));
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    let on_remove = std::rc::Rc::new(on_remove);
    if rules.is_empty() {
        let row = hbox(6);
        row.append(&note(&t("list-empty")));
        set_margins(&row, 8);
        list.append(&row);
    }
    for rule in rules {
        let row = hbox(6);
        set_margins(&row, 4);
        let text = gtk::Label::new(Some(&rule));
        text.set_xalign(0.0);
        text.set_hexpand(true);
        text.set_selectable(true);
        text.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let remove = gtk::Button::from_icon_name("list-remove-symbolic");
        remove.add_css_class("flat");
        remove.set_tooltip_text(Some(&t("action-remove")));
        let cb = on_remove.clone();
        remove.connect_clicked(move |_| cb(rule.clone()));
        row.append(&text);
        row.append(&remove);
        list.append(&row);
    }
    b.append(&list);

    let add_row = hbox(6);
    let entry = gtk::Entry::builder().placeholder_text(placeholder).hexpand(true).build();
    let add = gtk::Button::with_label(&t("action-add"));
    let on_add = std::rc::Rc::new(on_add);
    let submit = {
        let entry = entry.clone();
        let on_add = on_add.clone();
        move || {
            let text = entry.text().trim().to_string();
            if !text.is_empty() {
                entry.set_text("");
                on_add(text);
            }
        }
    };
    let s1 = submit.clone();
    entry.connect_activate(move |_| s1());
    add.connect_clicked(move |_| submit());
    add_row.append(&entry);
    add_row.append(&add);
    b.append(&add_row);
    b
}

/// A text box of rules, one per line. Lines `check` rejects are underlined
/// and the first one is explained below the box, with the number of rules.
pub fn rule_text(
    title: &str,
    text: &str,
    examples: &str,
    check: impl Fn(&str) -> Result<(), String> + 'static,
    on_change: impl Fn(String) + 'static,
) -> gtk::Box {
    use std::cell::Cell;
    use std::rc::Rc;

    let b = vbox(6);
    b.append(&heading(title));
    let buffer = gtk::TextBuffer::new(None);
    buffer.set_text(text);
    let invalid = gtk::TextTag::builder().name("invalid").underline(gtk::pango::Underline::Error).build();
    buffer.tag_table().add(&invalid);
    let view = gtk::TextView::builder()
        .buffer(&buffer)
        .monospace(true)
        .accepts_tab(false)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(10)
        .right_margin(10)
        .build();
    // Addresses and program names read left to right in every language.
    view.set_direction(gtk::TextDirection::Ltr);
    let scroll = gtk::ScrolledWindow::builder()
        .child(&view)
        .min_content_height(120)
        .max_content_height(360)
        .propagate_natural_height(true)
        .build();
    scroll.add_css_class("frame");
    b.append(&scroll);
    let status = note("");
    b.append(&status);
    let problem_label = label("");
    problem_label.add_css_class("error");
    problem_label.set_visible(false);
    b.append(&problem_label);

    let examples = examples.to_string();
    let validate = move |buffer: &gtk::TextBuffer| {
        buffer.remove_tag_by_name("invalid", &buffer.start_iter(), &buffer.end_iter());
        let (mut count, mut bad, mut first) = (0i64, 0i64, None);
        for i in 0..buffer.line_count() {
            let Some(start) = buffer.iter_at_line(i) else { continue };
            let mut end = start;
            if !end.ends_line() {
                end.forward_to_line_end();
            }
            let line = buffer.text(&start, &end, false);
            let rule = line.trim();
            if rule.is_empty() {
                continue;
            }
            match check(rule) {
                Ok(()) => count += 1,
                Err(problem) => {
                    buffer.apply_tag(&invalid, &start, &end);
                    bad += 1;
                    first.get_or_insert((i + 1, rule.to_string(), problem));
                }
            }
        }
        status.set_text(&if count == 0 && bad == 0 {
            ta("list-empty-hint", &[("examples", examples.clone().into())])
        } else {
            ta("rules-count", &[("count", count.into())])
        });
        match first {
            Some((line, rule, problem)) => {
                let mut text =
                    ta("line-invalid", &[("line", (line as i64).into()), ("rule", rule.into()), ("problem", problem.into())]);
                if bad > 1 {
                    text.push(' ');
                    text.push_str(&ta("lines-invalid-more", &[("count", (bad - 1).into())]));
                }
                problem_label.set_text(&text);
                problem_label.set_visible(true);
            }
            None => problem_label.set_visible(false),
        }
    };
    validate(&buffer);

    // Checking waits for a pause in typing, so long lists stay quick.
    let pending: Rc<Cell<Option<glib::SourceId>>> = Rc::default();
    let validate = Rc::new(validate);
    buffer.connect_changed(move |buffer| {
        on_change(buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string());
        if let Some(id) = pending.take() {
            id.remove();
        }
        let (buffer, validate, slot) = (buffer.clone(), validate.clone(), pending.clone());
        pending.set(Some(glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
            slot.set(None);
            validate(&buffer);
        })));
    });
    b
}

pub fn set_margins(w: &impl IsA<gtk::Widget>, m: i32) {
    w.set_margin_top(m);
    w.set_margin_bottom(m);
    w.set_margin_start(m);
    w.set_margin_end(m);
}

/// A row with a label on one side and a control on the other.
pub fn setting_row(text: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = hbox(12);
    let l = label(text);
    l.set_hexpand(true);
    row.append(&l);
    control.set_valign(gtk::Align::Center);
    row.append(control);
    row
}

pub fn fmt_time(unix: i64) -> String {
    jiff::Timestamp::from_second(unix)
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::system()).strftime("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

pub fn fmt_minutes(m: u16) -> String {
    format!("{:02}:{:02}", m / 60, m % 60)
}

pub fn day_name(d: Day) -> String {
    t(match d {
        Day::Mon => "day-mon",
        Day::Tue => "day-tue",
        Day::Wed => "day-wed",
        Day::Thu => "day-thu",
        Day::Fri => "day-fri",
        Day::Sat => "day-sat",
        Day::Sun => "day-sun",
    })
}

pub fn describe_schedule(s: &Schedule) -> Vec<String> {
    let mut out: Vec<String> = s
        .weekly
        .iter()
        .map(|w| {
            let days: Vec<String> = w.days.iter().map(|d| day_name(*d)).collect();
            format!("{}  {}–{}", days.join(", "), fmt_minutes(w.start), fmt_minutes(w.end))
        })
        .collect();
    out.extend(s.dates.iter().map(|d| format!("{} → {}", d.start.strftime("%Y-%m-%d %H:%M"), d.end.strftime("%Y-%m-%d %H:%M"))));
    out
}

pub fn describe_area(area: &GeoArea) -> String {
    match area {
        GeoArea::Circle { lat, lon, radius_m } => ta(
            "area-circle",
            &[("radius", format!("{radius_m:.0}").into()), ("lat", format!("{lat:.4}").into()), ("lon", format!("{lon:.4}").into())],
        ),
        GeoArea::Polygon { points } => ta("area-polygon", &[("count", (points.len() as i64).into())]),
    }
}

pub fn describe_lock(lock: &Lock) -> String {
    let mut s = match &lock.kind {
        LockKind::Until { until } => ta("lock-desc-until", &[("time", fmt_time(*until).into())]),
        LockKind::Schedule { schedule } => {
            ta("lock-desc-schedule", &[("times", describe_schedule(schedule).join("; ").into())])
        }
        LockKind::Password { .. } => t("lock-desc-password"),
        LockKind::RandomText { length } => ta("lock-desc-random", &[("length", (*length as i64).into())]),
        LockKind::UntilRestart { .. } => t("lock-desc-restart"),
        LockKind::Location { area } => ta("lock-desc-location", &[("area", describe_area(area).into())]),
    };
    if lock.unlock_delay_secs > 0 {
        s.push_str(" · ");
        s.push_str(&ta("lock-desc-delay", &[("minutes", ((lock.unlock_delay_secs / 60) as i64).into())]));
    }
    s
}
