//! Times, schedules and locks in words, for both apps.

use iqal_core::schedule::{Day, Schedule};
use iqal_core::{GeoArea, Lock, LockKind};
use iqal_i18n::{t, ta};

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
        GeoArea::Named { name } => ta("area-named", &[("name", name.clone().into())]),
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
