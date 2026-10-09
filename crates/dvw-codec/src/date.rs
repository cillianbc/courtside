//! Calendar dates without a date-time dependency.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => return None,
        };
        (1..=days)
            .contains(&day)
            .then_some(Date { year, month, day })
    }

    /// From a spreadsheet date serial (day 0 = 1899-12-30), as DataVolley
    /// writes on the second `[3MATCH]` line.
    pub fn from_serial(serial: i64) -> Option<Self> {
        const UNIX_EPOCH_SERIAL: i64 = 25_569;
        Self::from_unix_days(serial - UNIX_EPOCH_SERIAL)
    }

    /// Days since 1970-01-01 to a date (Hinnant's `civil_from_days`).
    fn from_unix_days(days: i64) -> Option<Self> {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        Date::new(
            i32::try_from(year).ok()?,
            u8::try_from(month).ok()?,
            u8::try_from(day).ok()?,
        )
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates() {
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(2023, 2, 29).is_none());
        assert!(Date::new(2026, 13, 1).is_none());
        assert!(Date::new(2026, 4, 31).is_none());
    }

    #[test]
    fn serials_seen_in_corpus() {
        assert_eq!(Date::from_serial(46_234), Date::new(2026, 7, 31));
        assert_eq!(Date::from_serial(45_317), Date::new(2024, 1, 26));
        assert_eq!(Date::from_serial(25_569), Date::new(1970, 1, 1));
        assert_eq!(Date::from_serial(0), Date::new(1899, 12, 30));
    }
}
