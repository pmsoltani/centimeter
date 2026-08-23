//! Timestamps as a consumer meets them: provenance the caller's clock supplies.

use std::time::{SystemTime, UNIX_EPOCH};

use centimeter_core::{Date, Error, Timestamp, TimestampError};

/// Milliseconds in one day, which is all the arithmetic a consumer needs to
/// relate an instant to the day it falls on.
const DAY: i64 = 86_400_000;

/// Core never reads the clock, so a consumer has to inject one.
#[test]
fn the_caller_supplies_the_clock() {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the host clock must be at or after 1970")
        .as_millis();
    let millis = i64::try_from(since_epoch).expect("milliseconds fit an i64 for another aeon");

    let now = Timestamp::try_new(millis).expect("the present is in range");
    assert_eq!(now.millis(), millis);
    assert!(Timestamp::MIN < now && now < Timestamp::MAX);
}

/// The two types must agree on what "supported" means: every instant a
/// timestamp can hold maps to a day a `Date` can hold, with nothing over at
/// either end.
#[test]
fn the_timestamp_range_covers_exactly_the_days_the_date_range_covers() {
    let day_of = |instant: Timestamp| {
        let days = instant.millis().div_euclid(DAY);
        Date::try_from_days(i32::try_from(days).expect("a supported day fits an i32"))
            .expect("a supported instant must fall on a supported day")
    };

    assert_eq!(day_of(Timestamp::MIN), Date::MIN);
    assert_eq!(day_of(Timestamp::MAX), Date::MAX);

    for millis in [Timestamp::MIN.millis() - 1, Timestamp::MAX.millis() + 1] {
        let err = Timestamp::try_new(millis).expect_err("outside years 1-9999");
        assert!(matches!(err, TimestampError::OutOfRange { got } if got == millis), "got {err}");
    }
}

/// Period and provenance are independent axes. A back-dated entry belongs to an
/// earlier period than one written before it, so the two orderings genuinely
/// disagree, and core pins no relation between them.
#[test]
fn an_audit_trail_orders_by_instant_while_a_ledger_orders_by_date() {
    let day = |text: &str| text.parse::<Date>().expect("a real date");
    let midnight_on = |text: &str| {
        Timestamp::try_new(i64::from(day(text).days()) * DAY).expect("a supported instant")
    };

    // The second entry is back-dated: an earlier period, written months later.
    let mut entries = [
        (day("2026-03-31"), midnight_on("2026-03-31")),
        (day("2026-01-15"), midnight_on("2026-05-28")),
    ];

    entries.sort_by_key(|&(_, recorded)| recorded);
    assert_eq!(entries.map(|(date, _)| date.to_string()), ["2026-03-31", "2026-01-15"]);

    entries.sort_by_key(|&(date, _)| date);
    assert_eq!(entries.map(|(date, _)| date.to_string()), ["2026-01-15", "2026-03-31"]);
}

/// A caller can bubble a date failure with `?` alongside other domain errors.
#[test]
fn timestamp_errors_bubble_into_the_root_error() {
    fn record_at(millis: i64) -> Result<Timestamp, Error> {
        Ok(Timestamp::try_new(millis)?)
    }

    // Microseconds passed where milliseconds were expected, which is the
    // practical value of the upper bound.
    let micros = 1_787_000_000_000_000;
    let err = record_at(micros).expect_err("the year 58000 is not a recording time");
    assert!(matches!(err, Error::Timestamp(TimestampError::OutOfRange { .. })), "got {err}");

    // `#[error(transparent)]` means the root reads exactly like the domain error.
    let domain = TimestampError::OutOfRange { got: micros };
    assert_eq!(err.to_string(), domain.to_string());

    assert!(record_at(micros / 1_000).is_ok(), "the same instant in milliseconds is fine");
}
