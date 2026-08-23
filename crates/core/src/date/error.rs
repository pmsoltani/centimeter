//! Errors for the date domain.

/// Errors related to dates.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DateError {
    /// The day is out of range for the month and year it was given with.
    #[error("day {day} is out of range for {year:04}-{month:02}, which has {last_day} days")]
    DayOutOfRange {
        /// The provided year.
        year: i32,
        /// The provided month.
        month: i32,
        /// The provided day.
        day: i32,
        /// The last day that month actually has.
        last_day: i32,
    },

    /// The day count falls outside the range of supported dates.
    #[error("{got} days since the epoch is out of range (years 1-9999)")]
    DaysOutOfRange {
        /// The provided day count.
        got: i32,
    },

    /// The date string is not in the expected format.
    #[error("date string '{got}' is not in the expected format YYYY-MM-DD")]
    InvalidFormat {
        /// The provided date string.
        got: String,
    },

    /// The month is not in `1..=12`.
    #[error("month {got} is out of range (1-12)")]
    MonthOutOfRange {
        /// The provided month.
        got: i32,
    },

    /// The year is not in `1..=9999`.
    #[error("year {got} is out of range (1-9999)")]
    YearOutOfRange {
        /// The provided year.
        got: i32,
    },
}
