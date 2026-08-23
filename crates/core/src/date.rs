//! The civil date that fixes a transaction's accounting period.
//!
//! A [`Date`] has no time of day and no timezone. It shows which period an
//! entry belongs to.

use std::{fmt, ops, str};

mod civil;
mod error;

use civil::{civil_from_days, days_from_civil, last_day_of_month};
pub use error::DateError;

/// A civil calendar date: a year, a month and a day.
///
/// Held as a count of days since 1970-01-01, enabling integer operations for
/// date arithmetic. Every value in the supported range is a valid date, so a
/// `Date` instance needs no further validation.
///
/// Renders and parses as ISO 8601 `YYYY-MM-DD`.
///
/// # Examples
///
/// ```
/// # use centimeter_core::Date;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let date = Date::try_new(2026, 8, 22)?;
/// assert_eq!(date.to_string(), "2026-08-22");
/// assert_eq!(date.to_ymd(), (2026, 8, 22));
///
/// // The stored form is a day count, which is what makes ordering cheap.
/// assert_eq!(date.days(), 20_687);
/// assert!(Date::try_new(2026, 8, 21)? < date);
///
/// // Parsing is strict, and only real dates exist.
/// assert_eq!("2026-08-22".parse::<Date>()?, date);
/// assert!("2026-8-22".parse::<Date>().is_err());
/// assert!(Date::try_new(2023, 2, 29).is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(i32);

impl Date {
    /// The first supported year. Together with [`Self::MAX_YEAR`] this keeps
    /// every rendered year four digits wide which makes it compatible with
    /// ISO 8601, and enables text sorting to match chronological order.
    const MIN_YEAR: i32 = 1;

    /// The last supported year. See [`Self::MIN_YEAR`].
    const MAX_YEAR: i32 = 9999;

    /// Day count of `0001-01-01`, derived so the two ranges cannot disagree.
    pub const MIN: Self = Self(days_from_civil(Self::MIN_YEAR, 1, 1));

    /// Day count of `9999-12-31`, derived so the two ranges cannot disagree.
    pub const MAX: Self = Self(days_from_civil(Self::MAX_YEAR, 12, 31));

    /// Creates a new date from the given year, month, and day.
    ///
    /// # Errors
    /// - [`DateError::YearOutOfRange`] if the year is not in [1, 9999]
    /// - [`DateError::MonthOutOfRange`] if the month is not in [1, 12]
    /// - [`DateError::DayOutOfRange`] if the day is not in
    ///   [1, `last_day_of_month(year, month)`]
    pub fn try_new(year: i32, month: i32, day: i32) -> Result<Self, DateError> {
        if !(Self::MIN_YEAR..=Self::MAX_YEAR).contains(&year) {
            return Err(DateError::YearOutOfRange { got: year });
        }
        if !(1..=12).contains(&month) {
            return Err(DateError::MonthOutOfRange { got: month });
        }
        let last_day = last_day_of_month(year, month);
        if !(1..=last_day).contains(&day) {
            return Err(DateError::DayOutOfRange { year, month, day, last_day });
        }
        Self::try_from_days(days_from_civil(year, month, day))
    }

    /// Creates a new date from the given number of days since 1970-01-01.
    ///
    /// # Errors
    /// Returns [`DateError::DaysOutOfRange`] if the number of days is not in
    /// [`Self::MIN`, `Self::MAX`]
    pub fn try_from_days(days: i32) -> Result<Self, DateError> {
        if !(Self::MIN.0..=Self::MAX.0).contains(&days) {
            return Err(DateError::DaysOutOfRange { got: days });
        }
        Ok(Self(days))
    }

    /// Returns the year, month, and day of the date.
    #[must_use]
    pub const fn to_ymd(self) -> (i32, i32, i32) {
        civil_from_days(self.0)
    }

    /// Returns the number of days since (1970-01-01), negative for dates before epoch.
    #[must_use]
    pub const fn days(self) -> i32 {
        self.0
    }

    /// Returns the year of the date.
    #[must_use]
    pub const fn year(self) -> i32 {
        self.to_ymd().0
    }

    /// Returns the month of the date.
    #[must_use]
    pub const fn month(self) -> i32 {
        self.to_ymd().1
    }

    /// Returns the day of the date.
    #[must_use]
    pub const fn day(self) -> i32 {
        self.to_ymd().2
    }
}

impl fmt::Display for Date {
    /// Returns the date as ISO 8601 format `YYYY-MM-DD`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = self.to_ymd();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

impl str::FromStr for Date {
    type Err = DateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || DateError::InvalidFormat { got: s.to_string() };
        let bytes = s.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(invalid());
        }

        let parse = |range: ops::Range<usize>| {
            let part = &s[range];
            if !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid());
            }
            part.parse::<i32>().map_err(|_| invalid())
        };
        Self::try_new(parse(0..4)?, parse(5..7)?, parse(8..10)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Construction

    #[test]
    fn test_try_new_accepts_the_range_boundaries() {
        let first = Date::try_new(1, 1, 1).unwrap();
        let last = Date::try_new(9999, 12, 31).unwrap();
        assert_eq!(first.days(), Date::MIN.0);
        assert_eq!(last.days(), Date::MAX.0);
        assert!(first < last);
    }

    /// A five-digit or negative year would break the fixed width that ISO 8601
    /// text ordering depends on.
    #[test]
    fn test_try_new_rejects_a_year_outside_the_supported_range() {
        for year in [i32::MIN, -1, 0, 10_000, i32::MAX] {
            let err = Date::try_new(year, 1, 1).unwrap_err();
            assert!(matches!(err, DateError::YearOutOfRange { got } if got == year), "got {err}");
        }
    }

    #[test]
    fn test_try_new_rejects_a_month_outside_the_calendar() {
        for month in [i32::MIN, -1, 0, 13, i32::MAX] {
            let err = Date::try_new(2026, month, 1).unwrap_err();
            assert!(matches!(err, DateError::MonthOutOfRange { got } if got == month), "got {err}");
        }
    }

    /// A day is only wrong relative to its month, so the error shows both, and
    /// also the correct length of that month.
    #[test]
    fn test_try_new_rejects_a_day_the_month_does_not_have() {
        let err = Date::try_new(2023, 2, 29).unwrap_err();
        assert!(
            matches!(err, DateError::DayOutOfRange { year: 2023, month: 2, day: 29, last_day: 28 }),
            "got {err}"
        );
        assert!(Date::try_new(2024, 2, 29).is_ok(), "2024 is a leap year");

        for (month, day) in [(2, 30), (4, 31), (6, 31), (9, 31), (11, 31), (1, 32), (1, 0)] {
            assert!(Date::try_new(2024, month, day).is_err(), "2024-{month}-{day} should fail");
        }
    }

    #[test]
    fn test_try_from_days_rejects_days_beyond_the_supported_range() {
        assert!(Date::try_from_days(Date::MIN.0).is_ok());
        assert!(Date::try_from_days(Date::MAX.0).is_ok());
        for days in [Date::MIN.0 - 1, Date::MAX.0 + 1, i32::MIN, i32::MAX] {
            let err = Date::try_from_days(days).unwrap_err();
            assert!(matches!(err, DateError::DaysOutOfRange { got } if got == days), "got {err}");
        }
    }

    // Accessors and rendering

    #[test]
    fn test_accessors_agree_with_to_ymd() {
        let date = Date::try_new(2026, 8, 22).unwrap();
        assert_eq!(date.to_ymd(), (2026, 8, 22));
        assert_eq!((date.year(), date.month(), date.day()), (2026, 8, 22));
        assert_eq!(date.days(), 20_687);
    }

    #[test]
    fn test_display_pads_every_field() {
        assert_eq!(Date::try_new(1, 1, 1).unwrap().to_string(), "0001-01-01");
        assert_eq!(Date::try_new(999, 9, 9).unwrap().to_string(), "0999-09-09");
        assert_eq!(Date::try_new(9999, 12, 31).unwrap().to_string(), "9999-12-31");
    }

    // Parsing

    #[test]
    fn test_from_str_accepts_strict_iso_8601() {
        for text in ["2026-08-22", "0001-01-01", "9999-12-31", "2024-02-29", "1969-12-31"] {
            let date = text.parse::<Date>().expect(text);
            assert_eq!(date.to_string(), text, "{text} did not round trip");
        }
    }

    /// Anything that is not exactly ten bytes of `YYYY-MM-DD` is refused.
    #[test]
    fn test_from_str_rejects_anything_else() {
        for text in [
            "",
            "2026-8-22",
            "20260822",
            "2026/08/22",
            "2026x08x22",
            "2026-08-22 ",
            " 2026-08-2",
            "+123-01-01", // `i32::from_str` would accept the sign
            "-123-01-01",
            "2026-+1-01",
            "2026-08-+1",
            "202\u{e9}08-22", // ten bytes, but not ten characters
            "20\u{e9}-08-22",
        ] {
            let err = text.parse::<Date>().unwrap_err();
            assert!(
                matches!(&err, DateError::InvalidFormat { got } if got == text),
                "{text:?} gave {err}"
            );
        }
    }

    /// A malformed string and an impossible date are different errors.
    #[test]
    fn test_from_str_separates_a_bad_shape_from_a_bad_date() {
        assert!(matches!(
            "0000-01-01".parse::<Date>().unwrap_err(),
            DateError::YearOutOfRange { got: 0 }
        ));
        assert!(matches!(
            "2026-13-01".parse::<Date>().unwrap_err(),
            DateError::MonthOutOfRange { got: 13 }
        ));
        assert!(matches!(
            "2023-02-29".parse::<Date>().unwrap_err(),
            DateError::DayOutOfRange { day: 29, last_day: 28, .. }
        ));
    }

    /// Sorting the rendered text sorts the dates. Sampled here with a stride
    /// that is coprime with 7, 30, 365 and 400, so it lands on every weekday,
    /// month length and leap rule.
    #[test]
    fn test_text_order_matches_date_order() {
        let mut previous: Option<(String, Date)> = None;
        for days in (Date::MIN.0..=Date::MAX.0).step_by(1_009) {
            let date = Date::try_from_days(days).unwrap();
            let text = date.to_string();
            assert_eq!(text.len(), 10, "{text} is not ten bytes wide");
            assert_eq!(text.parse::<Date>().unwrap(), date, "{text} did not round trip");
            if let Some((previous_text, previous_date)) = &previous {
                assert!(previous_text.as_str() < text.as_str(), "{previous_text} !< {text}");
                assert!(*previous_date < date);
            }
            previous = Some((text, date));
        }
    }

    /// The unsampled version of the test above. Allocates a `String` per date,
    /// which costs several seconds unoptimised, so it is not part of the
    /// default test suite. Run: `cargo test -- --ignored`.
    #[test]
    #[ignore = "3.65 million allocations; the sampled test covers the same property"]
    fn test_every_date_renders_and_reparses() {
        for days in Date::MIN.0..=Date::MAX.0 {
            let date = Date::try_from_days(days).unwrap();
            let text = date.to_string();
            assert_eq!(text.len(), 10, "{text} is not ten bytes wide");
            assert_eq!(text.parse::<Date>().unwrap(), date, "{text} did not round trip");
        }
    }
}
