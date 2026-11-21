//! The local clock the domain reads.
//!
//! [`Workspace`] accepts any clock provider; production uses
//! [`SystemClock`], tests use [`FixedClock`] for deterministic
//! date and overdue calculations.

use std::sync::Arc;

use super::date::{LocalDateTime, LocalTime};

/// Reads the device's current local date and time.
pub trait Clock: Send + Sync {
    fn now(&self) -> LocalDateTime;
}

/// The real clock: the device's current local wall-clock time.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> LocalDateTime {
        let now = time::OffsetDateTime::now_local()
            .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
        LocalDateTime::new(
            super::date::LocalDate::new(now.year(), now.month() as u8, now.day()),
            LocalTime::new(now.hour(), now.minute()),
        )
    }
}

/// A clock fixed at one instant, for deterministic tests.
pub struct FixedClock {
    now: std::sync::Mutex<LocalDateTime>,
}

impl FixedClock {
    pub fn at(now: LocalDateTime) -> Arc<Self> {
        Arc::new(Self { now: std::sync::Mutex::new(now) })
    }

    /// Moves the fixed instant (a test simulates the next day, and so on).
    pub fn advance_to(&self, now: LocalDateTime) {
        *self.now.lock().expect("fixed clock lock") = now;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> LocalDateTime {
        *self.now.lock().expect("fixed clock lock")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::date::LocalDate;

    #[test]
    fn fixed_clock_holds_and_advances() {
        let clock = FixedClock::at(LocalDateTime::parse("2026-11-20 09:30").unwrap());
        assert_eq!(clock.now().date, LocalDate::parse("2026-11-20").unwrap());
        clock.advance_to(LocalDateTime::parse("2026-11-21 00:00").unwrap());
        assert_eq!(clock.now().date, LocalDate::parse("2026-11-21").unwrap());
    }
}
