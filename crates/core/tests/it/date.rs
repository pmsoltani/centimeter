//! Dates as a consumer meets them: the key that fixes an accounting period.

use std::collections::BTreeMap;

use centimeter_core::{Date, DateError, Error};

#[test]
fn a_date_is_built_from_a_calendar_triple_and_reads_back_as_one() {
    let date = Date::try_new(2026, 8, 22).expect("2026-08-22 is a real date");

    // Reading four fields off one value also proves `Date` is `Copy` here.
    assert_eq!(date.to_ymd(), (2026, 8, 22));
    assert_eq!((date.year(), date.month(), date.day()), (2026, 8, 22));

    // What core renders, it can parse, and it is the only form it accepts.
    assert_eq!(date.to_string(), "2026-08-22");
    assert_eq!("2026-08-22".parse::<Date>().expect("the rendering must parse back"), date);
    assert!("22/08/2026".parse::<Date>().is_err(), "a locale format is not a date");
}

/// The supported range is public, but the day count behind it is not, so a
/// consumer can only reach the edges through `days`.
#[test]
fn the_range_bounds_are_reachable_and_are_the_first_and_last_supported_days() {
    assert_eq!(Date::MIN.to_ymd(), (1, 1, 1));
    assert_eq!(Date::MAX.to_ymd(), (9999, 12, 31));
    assert!(Date::MIN < Date::MAX);

    assert_eq!(Date::try_from_days(Date::MIN.days()).expect("MIN is in range"), Date::MIN);
    assert_eq!(Date::try_from_days(Date::MAX.days()).expect("MAX is in range"), Date::MAX);

    for days in [Date::MIN.days() - 1, Date::MAX.days() + 1] {
        let err = Date::try_from_days(days).expect_err("outside years 1-9999");
        assert!(matches!(err, DateError::DaysOutOfRange { got } if got == days), "got {err}");
    }
}

/// A consumer must be able to sort by date and bucket on it.
#[test]
fn dates_sort_chronologically_and_bucket_as_a_period_key() {
    let text = ["2027-01-01", "2026-12-31", "2026-01-01", "2026-12-31"];
    let mut dates: Vec<Date> = text.iter().map(|t| t.parse().expect("a real date")).collect();
    dates.sort_unstable();

    let rendered: Vec<String> = dates.iter().map(Date::to_string).collect();
    assert_eq!(rendered, ["2026-01-01", "2026-12-31", "2026-12-31", "2027-01-01"]);

    // Bucketing entries by period: equal dates land in one bucket, and the map
    // hands the buckets back already in chronological order.
    let mut periods: BTreeMap<Date, usize> = BTreeMap::new();
    for date in dates {
        *periods.entry(date).or_default() += 1;
    }
    assert_eq!(periods.len(), 3, "the repeated date must be one bucket");
    assert_eq!(periods.values().copied().collect::<Vec<_>>(), [1, 2, 1]);
}

/// A caller can bubble a date failure with `?` alongside other domain errors.
#[test]
fn date_errors_bubble_into_the_root_error() {
    fn parse_period(text: &str) -> Result<Date, Error> {
        Ok(text.parse::<Date>()?)
    }

    let err = parse_period("2023-02-29").expect_err("2023 is not a leap year");
    assert!(matches!(err, Error::Date(DateError::DayOutOfRange { .. })), "got {err}");

    // `#[error(transparent)]` means the root reads exactly like the domain error.
    let domain = DateError::DayOutOfRange { year: 2023, month: 2, day: 29, last_day: 28 };
    assert_eq!(err.to_string(), domain.to_string());

    assert!(parse_period("2024-02-29").is_ok(), "2024 is a leap year");
}
