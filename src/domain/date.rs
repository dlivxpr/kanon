//! Floating local wall-clock date and time values.
//!
//! Kanon interprets Scheduled and Due date/times as *floating* local
//! wall-clock values in the device's current time zone: they are stored
//! and compared as plain `YYYY-MM-DD` / `HH:MM` text, with no UTC offset
//! conversion when the time zone changes.

use std::fmt;

/// A local calendar date (`YYYY-MM-DD`), without any time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

/// A local wall-clock time (`HH:MM`), without any time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalTime {
    pub hour: u8,
    pub minute: u8,
}

/// The local date and time together: what a [`Clock`](crate::Clock) reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalDateTime {
    pub date: LocalDate,
    pub time: LocalTime,
}

impl LocalDate {
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// Parses `YYYY-MM-DD`; rejects months outside 1..=12 and days outside
    /// 1..=31 (calendar validation such as February 30th is the caller's).
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let digits = |range: std::ops::Range<usize>| -> Option<i32> {
            let part = bytes.get(range)?;
            if part.iter().all(u8::is_ascii_digit) {
                std::str::from_utf8(part).ok()?.parse().ok()
            } else {
                None
            }
        };
        if bytes.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
            return None;
        }
        let year = digits(0..4)?;
        let month = digits(5..7)? as u8;
        let day = digits(8..10)? as u8;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub fn days_from_civil(self) -> i64 {
        let y = if self.month <= 2 { self.year - 1 } else { self.year } as i64;
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let month = self.month as i64;
        let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    /// The date `days` away from this one.
    pub fn add_days(self, days: i64) -> Self {
        let z = self.days_from_civil() + days;
        // Howard Hinnant's `civil_from_days`.
        let z = z + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
        Self { year: if month <= 2 { y + 1 } else { y } as i32, month, day }
    }

    /// 0 = Monday, 6 = Sunday.
    pub fn weekday(self) -> u8 {
        ((self.days_from_civil() + 3).rem_euclid(7)) as u8
    }

    /// `周一` … `周日`.
    pub fn weekday_label(self) -> &'static str {
        ["周一", "周二", "周三", "周四", "周五", "周六", "周日"][self.weekday() as usize]
    }

    /// `2026年11月20日 周五`.
    pub fn long_label(self) -> String {
        format!("{}年{}月{}日 {}", self.year, self.month, self.day, self.weekday_label())
    }
}

impl fmt::Display for LocalDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl LocalTime {
    pub const fn new(hour: u8, minute: u8) -> Self {
        Self { hour, minute }
    }

    /// Parses `HH:MM`; rejects hours outside 0..=23 and minutes outside 0..=59.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let digits = |range: std::ops::Range<usize>| -> Option<u8> {
            let part = bytes.get(range)?;
            if part.iter().all(u8::is_ascii_digit) {
                std::str::from_utf8(part).ok()?.parse().ok()
            } else {
                None
            }
        };
        if bytes.len() != 5 || bytes.get(2) != Some(&b':') {
            return None;
        }
        let hour = digits(0..2)?;
        let minute = digits(3..5)?;
        if hour > 23 || minute > 59 {
            return None;
        }
        Some(Self { hour, minute })
    }
}

impl fmt::Display for LocalTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl LocalDateTime {
    pub const fn new(date: LocalDate, time: LocalTime) -> Self {
        Self { date, time }
    }

    /// Parses `YYYY-MM-DD HH:MM`.
    pub fn parse(text: &str) -> Option<Self> {
        let (date, time) = text.split_once(' ')?;
        Some(Self { date: LocalDate::parse(date)?, time: LocalTime::parse(time)? })
    }
}

impl fmt::Display for LocalDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.date, self.time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_dates() {
        let date = LocalDate::parse("2026-11-20").unwrap();
        assert_eq!(date.to_string(), "2026-11-20");
        assert_eq!(date.year, 2026);
        assert_eq!(date.month, 11);
        assert_eq!(date.day, 20);
        assert!(LocalDate::parse("2026-13-01").is_none());
        assert!(LocalDate::parse("2026-11-32").is_none());
        assert!(LocalDate::parse("20-11-2026").is_none());
        assert!(LocalDate::parse("2026/11/20").is_none());
    }

    #[test]
    fn round_trips_days() {
        let start = LocalDate::parse("1970-01-01").unwrap();
        assert_eq!(start.add_days(0), start);
        assert_eq!(start.add_days(1).to_string(), "1970-01-02");
        assert_eq!(start.add_days(-1).to_string(), "1969-12-31");
        let leap = LocalDate::parse("2024-02-28").unwrap().add_days(1);
        assert_eq!(leap.to_string(), "2024-02-29");
        let year = LocalDate::parse("2025-12-31").unwrap().add_days(1);
        assert_eq!(year.to_string(), "2026-01-01");
    }

    #[test]
    fn computes_weekdays() {
        // 2026-11-20 is a Friday.
        assert_eq!(LocalDate::parse("2026-11-20").unwrap().weekday_label(), "周五");
        assert_eq!(LocalDate::parse("2026-11-16").unwrap().weekday_label(), "周一");
        assert_eq!(LocalDate::parse("2026-11-22").unwrap().weekday_label(), "周日");
    }

    #[test]
    fn orders_dates_and_times() {
        assert!(LocalDate::parse("2026-11-20").unwrap() < LocalDate::parse("2026-11-21").unwrap());
        assert!(LocalTime::parse("09:30").unwrap() < LocalTime::parse("09:31").unwrap());
        let a = LocalDateTime::parse("2026-11-20 09:30").unwrap();
        let b = LocalDateTime::parse("2026-11-20 09:31").unwrap();
        assert!(a < b);
        assert!(LocalDateTime::parse("2026-11-20 23:59").unwrap() < LocalDateTime::parse("2026-11-21 00:00").unwrap());
    }

    #[test]
    fn parses_times_strictly() {
        assert_eq!(LocalTime::parse("09:30").unwrap().to_string(), "09:30");
        assert!(LocalTime::parse("24:00").is_none());
        assert!(LocalTime::parse("09:60").is_none());
        assert!(LocalTime::parse("930").is_none());
    }
}
