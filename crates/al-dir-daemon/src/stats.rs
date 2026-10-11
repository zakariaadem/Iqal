//! Block statistics, per local day, kept for 90 days in the state
//! directory. Nothing leaves the computer.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use al_dir_ipc::{DayStats, EventKind, StatsReport};
use serde::{Deserialize, Serialize};

const FILE: &str = "stats.json";
const KEEP_DAYS: usize = 90;
/// Distinct targets kept per day (the most blocked ones).
const MAX_TARGETS: usize = 200;

#[derive(Default, Serialize, Deserialize)]
struct Day {
    by_kind: BTreeMap<String, u32>,
    by_block: HashMap<String, u32>,
    targets: HashMap<String, u32>,
    allowance: HashMap<String, u32>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Stats {
    days: BTreeMap<String, Day>,
    #[serde(skip)]
    path: Option<PathBuf>,
    #[serde(skip)]
    dirty: bool,
}

fn kind_key(k: EventKind) -> String {
    serde_json::to_value(k).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

fn kind_from(s: &str) -> Option<EventKind> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

/// Short, groupable name for what was blocked: a host for web addresses,
/// a program name for paths.
pub fn target_label(kind: EventKind, target: &str) -> String {
    match kind {
        EventKind::PageBlocked => {
            url_host(target).unwrap_or(target).trim_start_matches("www.").to_string()
        }
        EventKind::SiteBlocked => target.trim_start_matches("www.").to_string(),
        EventKind::AppStart | EventKind::AppKilled => target.rsplit('/').next().unwrap_or(target).to_string(),
        _ => target.chars().take(60).collect(),
    }
}

fn url_host(url: &str) -> Option<&str> {
    let rest = url.split_once("://")?.1;
    rest.split(['/', '?', '#', ':']).next()
}

impl Stats {
    pub fn load(state_dir: &std::path::Path) -> Self {
        let path = state_dir.join(FILE);
        let mut s: Stats =
            std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        s.path = Some(path);
        s
    }

    fn day(&mut self, date: &str) -> &mut Day {
        self.dirty = true;
        if !self.days.contains_key(date) {
            self.days.insert(date.to_string(), Day::default());
            while self.days.len() > KEEP_DAYS {
                let oldest = self.days.keys().next().cloned().expect("not empty");
                self.days.remove(&oldest);
            }
        }
        self.days.get_mut(date).expect("just inserted")
    }

    pub fn record(&mut self, date: &str, kind: EventKind, block: &str, target: &str) {
        let label = target_label(kind, target);
        let day = self.day(date);
        *day.by_kind.entry(kind_key(kind)).or_default() += 1;
        if !matches!(kind, EventKind::AllowanceLow | EventKind::AllowanceUsed) {
            *day.by_block.entry(block.to_string()).or_default() += 1;
            *day.targets.entry(label).or_default() += 1;
            if day.targets.len() > MAX_TARGETS * 2 {
                let mut v: Vec<(String, u32)> = day.targets.drain().collect();
                v.sort_by_key(|e| std::cmp::Reverse(e.1));
                v.truncate(MAX_TARGETS);
                day.targets = v.into_iter().collect();
            }
        }
    }

    pub fn add_allowance(&mut self, date: &str, block: &str, secs: u32) {
        *self.day(date).allowance.entry(block.to_string()).or_default() += secs;
    }

    pub fn save_if_dirty(&mut self) {
        if !self.dirty {
            return;
        }
        if let (Some(path), Ok(text)) = (&self.path, serde_json::to_string(self)) {
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, path)).is_ok() {
                self.dirty = false;
            }
        }
    }

    /// The last `days` days ending with `today` (oldest first).
    pub fn report(&self, today: jiff::civil::Date, days: u32) -> StatsReport {
        let days = days.clamp(1, KEEP_DAYS as u32);
        let mut report = StatsReport::default();
        let mut by_kind: HashMap<String, u32> = HashMap::new();
        let mut targets: HashMap<String, u32> = HashMap::new();
        let mut blocks: HashMap<String, u32> = HashMap::new();
        let mut allowance: HashMap<String, u32> = HashMap::new();
        for back in (0..days).rev() {
            let date = today.checked_sub(jiff::Span::new().days(back as i64)).unwrap_or(today).to_string();
            let mut ds = DayStats { date: date.clone(), ..DayStats::default() };
            if let Some(d) = self.days.get(&date) {
                for (k, n) in &d.by_kind {
                    *by_kind.entry(k.clone()).or_default() += n;
                    if !matches!(kind_from(k), Some(EventKind::AllowanceLow | EventKind::AllowanceUsed)) {
                        ds.blocked += n;
                    }
                }
                for (k, n) in &d.targets {
                    *targets.entry(k.clone()).or_default() += n;
                }
                for (k, n) in &d.by_block {
                    *blocks.entry(k.clone()).or_default() += n;
                }
                for (k, n) in &d.allowance {
                    *allowance.entry(k.clone()).or_default() += n;
                    ds.allowance_secs += n;
                }
            }
            report.days.push(ds);
        }
        let sorted = |m: HashMap<String, u32>, n: usize| {
            let mut v: Vec<(String, u32)> = m.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            v.truncate(n);
            v
        };
        report.by_kind = sorted(by_kind, 20).into_iter().filter_map(|(k, n)| Some((kind_from(&k)?, n))).collect();
        report.top_targets = sorted(targets, 15);
        report.by_block = sorted(blocks, 20);
        report.allowance_by_block = sorted(allowance, 20);
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn records_and_reports() {
        let mut s = Stats::default();
        s.record("2026-10-11", EventKind::PageBlocked, "Social", "https://www.facebook.com/feed");
        s.record("2026-10-12", EventKind::PageBlocked, "Social", "https://facebook.com/x");
        s.record("2026-10-12", EventKind::AppStart, "Games", "/usr/games/steam");
        s.record("2026-10-12", EventKind::AllowanceUsed, "Games", "Games");
        s.add_allowance("2026-10-12", "Games", 600);
        let r = s.report(date(2026, 10, 12), 7);
        assert_eq!(r.days.len(), 7);
        assert_eq!(r.days.last().unwrap().blocked, 2, "allowance events are not blocks");
        assert_eq!(r.days[5].blocked, 1);
        assert_eq!(r.top_targets[0], ("facebook.com".to_string(), 2));
        assert!(r.top_targets.contains(&("steam".to_string(), 1)));
        assert_eq!(r.allowance_by_block, vec![("Games".to_string(), 600)]);
        assert_eq!(r.days.last().unwrap().allowance_secs, 600);
    }

    #[test]
    fn keeps_ninety_days() {
        let mut s = Stats::default();
        let start = date(2026, 1, 1);
        for i in 0..120 {
            let d = start.checked_add(jiff::Span::new().days(i)).unwrap().to_string();
            s.record(&d, EventKind::SiteBlocked, "b", "x.com");
        }
        assert_eq!(s.days.len(), KEEP_DAYS);
    }
}
