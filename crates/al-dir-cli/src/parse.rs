//! Parsing of human-friendly durations, times and schedules.

use al_dir_core::schedule::{Day, WeeklyRange};
use jiff::civil::DateTime;

/// "90s", "45m", "2h30m", "3d", "1w".
pub fn duration_secs(s: &str) -> Result<u64, String> {
    let bad = || format!("\"{s}\" is not a duration (examples: 45m, 2h30m, 3d, 1w)");
    let mut total: u64 = 0;
    let mut num = String::new();
    for c in s.trim().chars() {
        if c.is_ascii_digit() {
            num.push(c);
            continue;
        }
        let unit = match c.to_ascii_lowercase() {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            'd' => 86_400,
            'w' => 604_800,
            _ => return Err(bad()),
        };
        let n: u64 = num.parse().map_err(|_| bad())?;
        total = total.checked_add(n.checked_mul(unit).ok_or_else(bad)?).ok_or_else(bad)?;
        num.clear();
    }
    if !num.is_empty() || total == 0 {
        return Err(bad());
    }
    Ok(total)
}

/// "2026-10-20 18:00", "2026-10-20T18:00", "2026-10-20".
pub fn datetime(s: &str) -> Result<DateTime, String> {
    let s = s.trim();
    let normalized = s.replacen(' ', "T", 1);
    normalized
        .parse::<DateTime>()
        .or_else(|_| format!("{normalized}T00:00").parse::<DateTime>())
        .map_err(|_| format!("\"{s}\" is not a date/time (example: 2026-10-20 18:00)"))
}

/// "HH:MM" as minutes after midnight; "24:00" is allowed as an end time.
fn clock(s: &str) -> Result<u16, String> {
    let bad = || format!("\"{s}\" is not a time (example: 09:30)");
    let (h, m) = s.split_once(':').ok_or_else(bad)?;
    let (h, m): (u16, u16) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
    if m >= 60 || h > 24 || (h == 24 && m != 0) {
        return Err(bad());
    }
    Ok(h * 60 + m)
}

fn day(s: &str) -> Result<Day, String> {
    let i = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
        .iter()
        .position(|d| s.to_ascii_lowercase().starts_with(d))
        .ok_or_else(|| format!("\"{s}\" is not a day (mon, tue, ... sun)"))?;
    Ok(Day::ALL[i])
}

fn days(spec: &str) -> Result<Vec<Day>, String> {
    let mut out = Vec::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let range: Vec<Day> = match part.to_ascii_lowercase().as_str() {
            "daily" | "everyday" | "all" => Day::ALL.to_vec(),
            "weekdays" => Day::ALL[..5].to_vec(),
            "weekends" | "weekend" => Day::ALL[5..].to_vec(),
            _ => match part.split_once('-') {
                Some((a, b)) => {
                    let (a, b) = (day(a)? as usize, day(b)? as usize);
                    (0..7).map(|i| Day::ALL[(a + i) % 7]).take((b + 7 - a) % 7 + 1).collect()
                }
                None => vec![day(part)?],
            },
        };
        for d in range {
            if !out.contains(&d) {
                out.push(d);
            }
        }
    }
    if out.is_empty() {
        return Err("no days given".into());
    }
    Ok(out)
}

/// "mon-fri 09:00-17:00", "weekends 00:00-24:00", "daily 22:00-06:00",
/// "sat,sun 10:00-12:00".
pub fn weekly(spec: &str) -> Result<WeeklyRange, String> {
    let usage = "expected \"<days> <HH:MM-HH:MM>\", e.g. \"mon-fri 09:00-17:00\"";
    let (d, t) = spec.trim().rsplit_once(' ').ok_or(usage)?;
    let (start, end) = t.split_once('-').ok_or(usage)?;
    Ok(WeeklyRange { days: days(d)?, start: clock(start)?, end: clock(end)? })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(duration_secs("45m").unwrap(), 2700);
        assert_eq!(duration_secs("2h30m").unwrap(), 9000);
        assert_eq!(duration_secs("1w2d").unwrap(), 777_600);
        assert!(duration_secs("45").is_err());
        assert!(duration_secs("x").is_err());
        assert!(duration_secs("0m").is_err());
    }

    #[test]
    fn datetimes() {
        assert_eq!(datetime("2026-10-20 18:00").unwrap(), jiff::civil::date(2026, 10, 20).at(18, 0, 0, 0));
        assert_eq!(datetime("2026-10-20").unwrap(), jiff::civil::date(2026, 10, 20).at(0, 0, 0, 0));
        assert!(datetime("tomorrow").is_err());
    }

    #[test]
    fn weekly_specs() {
        let w = weekly("mon-fri 09:00-17:00").unwrap();
        assert_eq!(w.days, Day::ALL[..5].to_vec());
        assert_eq!((w.start, w.end), (540, 1020));
        assert_eq!(weekly("fri-mon 22:00-06:00").unwrap().days, vec![Day::Fri, Day::Sat, Day::Sun, Day::Mon]);
        assert_eq!(weekly("weekends 00:00-24:00").unwrap().end, 1440);
        assert_eq!(weekly("sat,sun,sat 10:00-12:00").unwrap().days, vec![Day::Sat, Day::Sun]);
        assert!(weekly("mon 25:00-26:00").is_err());
        assert!(weekly("funday 10:00-11:00").is_err());
        assert!(weekly("mon").is_err());
    }
}
