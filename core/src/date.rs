//! Calendar dates (no time, no timezone) and year-month periods.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    pub year: i32,
    pub month: u32,
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Option<Date> {
        let valid = (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month);
        valid.then_some(Date { year, month, day })
    }

    /// Parses `YYYY-MM-DD`.
    pub fn parse_iso(s: &str) -> Option<Date> {
        let mut parts = s.trim().split('-');
        let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || y.len() != 4 {
            return None;
        }
        Date::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?)
    }

    /// Parses `D/M/YYYY` (the format spreadsheets in Laos usually produce).
    pub fn parse_dmy(s: &str) -> Option<Date> {
        let mut parts = s.trim().split('/');
        let (d, m, y) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || y.len() != 4 {
            return None;
        }
        Date::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?)
    }

    pub fn prev_day(self) -> Date {
        if self.day > 1 {
            return Date { day: self.day - 1, ..self };
        }
        let ym = self.year_month().prev();
        Date { year: ym.year, month: ym.month, day: days_in_month(ym.year, ym.month) }
    }

    pub fn year_month(self) -> YearMonth {
        YearMonth { year: self.year, month: self.month }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl YearMonth {
    pub fn prev(self) -> YearMonth {
        if self.month == 1 {
            YearMonth { year: self.year - 1, month: 12 }
        } else {
            YearMonth { month: self.month - 1, ..self }
        }
    }

    pub fn next(self) -> YearMonth {
        if self.month == 12 {
            YearMonth { year: self.year + 1, month: 1 }
        } else {
            YearMonth { month: self.month + 1, ..self }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> Date {
        Date::new(y, m, day).unwrap()
    }

    #[test]
    fn leap_years_follow_gregorian_rules() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2026));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn days_in_month_handles_february() {
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2028, 2), 29);
        assert_eq!(days_in_month(2026, 11), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn new_rejects_impossible_dates() {
        assert!(Date::new(2026, 2, 29).is_none());
        assert!(Date::new(2026, 13, 1).is_none());
        assert!(Date::new(2026, 0, 1).is_none());
        assert!(Date::new(2026, 4, 0).is_none());
        assert!(Date::new(2028, 2, 29).is_some());
    }

    #[test]
    fn parse_iso_reads_padded_dates() {
        assert_eq!(Date::parse_iso("2026-11-01"), Some(d(2026, 11, 1)));
        assert_eq!(Date::parse_iso(" 2026-11-01 "), Some(d(2026, 11, 1)));
    }

    #[test]
    fn parse_iso_rejects_garbage() {
        assert_eq!(Date::parse_iso(""), None);
        assert_eq!(Date::parse_iso("2026-02-30"), None);
        assert_eq!(Date::parse_iso("11/01/2026"), None);
        assert_eq!(Date::parse_iso("2026-11"), None);
    }

    #[test]
    fn parse_dmy_reads_day_first() {
        assert_eq!(Date::parse_dmy("1/11/2026"), Some(d(2026, 11, 1)));
        assert_eq!(Date::parse_dmy("31/12/2025"), Some(d(2025, 12, 31)));
        assert_eq!(Date::parse_dmy("31/11/2025"), None);
        assert_eq!(Date::parse_dmy("2026-11-01"), None);
    }

    #[test]
    fn display_is_zero_padded_iso() {
        assert_eq!(d(2026, 1, 5).to_string(), "2026-01-05");
    }

    #[test]
    fn prev_day_crosses_month_and_year() {
        assert_eq!(d(2026, 11, 11).prev_day(), d(2026, 11, 10));
        assert_eq!(d(2026, 3, 1).prev_day(), d(2026, 2, 28));
        assert_eq!(d(2026, 1, 1).prev_day(), d(2025, 12, 31));
    }

    #[test]
    fn dates_order_chronologically() {
        assert!(d(2025, 12, 31) < d(2026, 1, 1));
        assert!(d(2026, 2, 10) < d(2026, 10, 2));
    }

    #[test]
    fn year_month_steps_across_years() {
        let jan = YearMonth { year: 2026, month: 1 };
        let dec = YearMonth { year: 2025, month: 12 };
        assert_eq!(jan.prev(), dec);
        assert_eq!(dec.next(), jan);
        assert_eq!(d(2026, 11, 11).year_month(), YearMonth { year: 2026, month: 11 });
    }
}
