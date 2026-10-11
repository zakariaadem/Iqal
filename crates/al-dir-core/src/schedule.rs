//! Weekly schedules and date ranges.

use jiff::civil::DateTime;
use serde::{Deserialize, Serialize};

/// The current moment, both as an absolute time (for deadlines) and as
/// local wall-clock time (for schedules).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    pub unix: i64,
    pub local: DateTime,
}

impl Now {
    pub fn system() -> Self {
        Self::from_zoned(&jiff::Zoned::now())
    }

    pub fn from_zoned(z: &jiff::Zoned) -> Self {
        Self { unix: z.timestamp().as_second(), local: z.datetime() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Day {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Day {
    pub const ALL: [Day; 7] = [Day::Mon, Day::Tue, Day::Wed, Day::Thu, Day::Fri, Day::Sat, Day::Sun];

    fn of(dt: DateTime) -> Day {
        Day::ALL[dt.weekday().to_monday_zero_offset() as usize]
    }

    fn previous(self) -> Day {
        Day::ALL[(self as usize + 6) % 7]
    }
}

/// A time window repeating on the given days. Times are minutes after
/// midnight. If `end <= start` the window runs past midnight into the next
/// day (`start == end` means a full 24 hours).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyRange {
    pub days: Vec<Day>,
    pub start: u16,
    pub end: u16,
}

/// A one-off local time window `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime,
    pub end: DateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Schedule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weekly: Vec<WeeklyRange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dates: Vec<DateRange>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScheduleError {
    #[error("time {0} is out of range (0..=1440 minutes)")]
    TimeOutOfRange(u16),
    #[error("a weekly range has no days")]
    NoDays,
    #[error("a date range ends before it starts")]
    EmptyDateRange,
}

impl WeeklyRange {
    pub fn is_active(&self, local: DateTime) -> bool {
        let day = Day::of(local);
        let minute = local.hour() as u16 * 60 + local.minute() as u16;
        if self.start < self.end {
            self.days.contains(&day) && (self.start..self.end).contains(&minute)
        } else {
            (self.days.contains(&day) && minute >= self.start)
                || (self.days.contains(&day.previous()) && minute < self.end)
        }
    }
}

impl Schedule {
    pub fn is_active(&self, now: &Now) -> bool {
        self.weekly.iter().any(|r| r.is_active(now.local))
            || self.dates.iter().any(|r| r.start <= now.local && now.local < r.end)
    }

    pub fn validate(&self) -> Result<(), ScheduleError> {
        for r in &self.weekly {
            if r.days.is_empty() {
                return Err(ScheduleError::NoDays);
            }
            for t in [r.start, r.end] {
                if t > 1440 {
                    return Err(ScheduleError::TimeOutOfRange(t));
                }
            }
        }
        if self.dates.iter().any(|r| r.end <= r.start) {
            return Err(ScheduleError::EmptyDateRange);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    // 2026-10-12 is a Monday.
    fn at(day: i8, h: i8, m: i8) -> Now {
        Now { unix: 0, local: date(2026, 10, day).at(h, m, 0, 0) }
    }

    fn weekly(days: &[Day], start: u16, end: u16) -> Schedule {
        Schedule { weekly: vec![WeeklyRange { days: days.to_vec(), start, end }], dates: vec![] }
    }

    #[test]
    fn simple_range() {
        let s = weekly(&[Day::Mon], 9 * 60, 17 * 60);
        assert!(s.is_active(&at(12, 9, 0)));
        assert!(s.is_active(&at(12, 16, 59)));
        assert!(!s.is_active(&at(12, 17, 0)));
        assert!(!s.is_active(&at(12, 8, 59)));
        assert!(!s.is_active(&at(13, 10, 0)), "Tuesday");
    }

    #[test]
    fn overnight_range() {
        let s = weekly(&[Day::Sun], 22 * 60, 6 * 60);
        assert!(s.is_active(&at(11, 23, 0)), "Sunday night");
        assert!(s.is_active(&at(12, 5, 59)), "Monday early morning");
        assert!(!s.is_active(&at(12, 6, 0)));
        assert!(!s.is_active(&at(12, 23, 0)), "Monday night is not scheduled");
    }

    #[test]
    fn full_day() {
        let s = weekly(&[Day::Sat], 0, 0);
        assert!(s.is_active(&at(17, 0, 0)));
        assert!(s.is_active(&at(17, 23, 59)));
        assert!(!s.is_active(&at(18, 0, 0)));
        let s = weekly(&[Day::Sat], 0, 1440);
        assert!(s.is_active(&at(17, 23, 59)));
        assert!(!s.is_active(&at(18, 0, 0)));
    }

    #[test]
    fn date_range() {
        let s = Schedule {
            weekly: vec![],
            dates: vec![DateRange {
                start: date(2026, 10, 12).at(8, 0, 0, 0),
                end: date(2026, 10, 14).at(8, 0, 0, 0),
            }],
        };
        assert!(s.is_active(&at(13, 3, 0)));
        assert!(!s.is_active(&at(14, 8, 0)));
        assert!(!s.is_active(&at(12, 7, 59)));
    }

    #[test]
    fn validation() {
        assert!(weekly(&[Day::Mon], 0, 1440).validate().is_ok());
        assert_eq!(weekly(&[], 0, 60).validate(), Err(ScheduleError::NoDays));
        assert_eq!(weekly(&[Day::Mon], 0, 1441).validate(), Err(ScheduleError::TimeOutOfRange(1441)));
    }

    #[test]
    fn serde_format() {
        let s = weekly(&[Day::Mon, Day::Fri], 540, 1020);
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#"{"weekly":[{"days":["mon","fri"],"start":540,"end":1020}]}"#);
        assert_eq!(serde_json::from_str::<Schedule>(&json).unwrap(), s);
    }
}
