//! When a record was written, distinct from when the economic event happened.
//!
//! A [`Timestamp`] is an instant on the UTC timeline. It is provenance: it
//! shows when the transaction was recorded. It is not a substitute for a
//! [`Date`](crate::Date), which shows when the economic event happened.
//!
//! Core never uses the clock. The timestamp must be supplied by the caller.

mod error;

use crate::Date;

pub use error::TimestampError;

/// An instant on the UTC timeline, in milliseconds since 1970-01-01T00:00:00Z.
///
/// A timestamp should not replace a [`Date`](crate::Date), and the timestamp
/// embedded in a record's id is not a substitute for a `Timestamp`: an offline
/// device mints ids from its own clock, which is why neither the id nor its
/// embedded instant can serve as an audit time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    /// The earliest supported timestamp, `0001-01-01T00:00:00.000Z`.
    pub const MIN: Self = Self(Date::MIN.days() as i64 * 86_400_000);

    /// The latest supported timestamp, `9999-12-31T23:59:59.999Z`.
    pub const MAX: Self = Self(((Date::MAX.days() as i64) + 1) * 86_400_000 - 1);

    /// Creates a new timestamp from a number of milliseconds since epoch.
    ///
    /// # Errors
    /// Returns [`TimestampError::OutOfRange`] if the milliseconds are outside
    /// the supported range of years 1-9999.
    pub const fn try_new(millis: i64) -> Result<Self, TimestampError> {
        if millis < Self::MIN.0 || Self::MAX.0 < millis {
            return Err(TimestampError::OutOfRange { got: millis });
        }
        Ok(Self(millis))
    }

    /// Returns the number of milliseconds since 1970-01-01T00:00:00.000Z.
    #[must_use]
    pub const fn millis(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bounds are derived from [`Date`], so what needs pinning is the
    /// derivation itself: that `MIN` is midnight on the first supported day and
    /// `MAX` is the last millisecond of the last one. The literals are the
    /// hand-checked answers.
    #[test]
    fn test_the_bounds_are_the_first_and_last_instant_of_the_supported_years() {
        assert_eq!(Timestamp::MIN.millis(), -62_135_596_800_000);
        assert_eq!(Timestamp::MAX.millis(), 253_402_300_799_999);

        // MIN is the start of Date::MIN, and MAX one millisecond before the day
        // after Date::MAX, so the two ranges cover exactly the same instants.
        let day = 86_400_000i64;
        assert_eq!(Timestamp::MIN.millis(), i64::from(Date::MIN.days()) * day);
        assert_eq!(Timestamp::MAX.millis() + 1, (i64::from(Date::MAX.days()) + 1) * day);
        assert_eq!(Timestamp::MAX.millis() - Timestamp::MIN.millis() + 1, 3_652_059 * day);
    }

    #[test]
    fn test_try_new_accepts_the_bounds_and_the_epoch() {
        for millis in [Timestamp::MIN.millis(), -1, 0, 1, Timestamp::MAX.millis()] {
            let built = Timestamp::try_new(millis).expect("in range");
            assert_eq!(built.millis(), millis);
        }
    }

    #[test]
    fn test_try_new_rejects_anything_outside_the_bounds() {
        for millis in [
            Timestamp::MIN.millis() - 1,
            Timestamp::MAX.millis() + 1,
            i64::MIN,
            i64::MAX,
            // A microsecond count passed where milliseconds were expected lands
            // around the year 55000, which is the practical value of the bound.
            1_756_000_000_000_000,
        ] {
            let err = Timestamp::try_new(millis).unwrap_err();
            assert!(
                matches!(err, TimestampError::OutOfRange { got } if got == millis),
                "got {err}"
            );
        }
    }

    /// Ordering is the integer's, so a later instant is a greater timestamp.
    #[test]
    fn test_ordering_follows_the_instant() {
        let epoch = Timestamp::try_new(0).unwrap();
        let later = Timestamp::try_new(1).unwrap();
        assert!(Timestamp::MIN < epoch);
        assert!(epoch < later);
        assert!(later < Timestamp::MAX);
        assert_eq!(Timestamp::MIN.min(Timestamp::MAX), Timestamp::MIN);
    }

    /// `try_new` is a `const fn`, so a timestamp can be built at compile time.
    /// This fails to compile rather than fails to pass if that ever changes.
    #[test]
    fn test_a_timestamp_can_be_built_in_a_const() {
        const EPOCH: Timestamp = match Timestamp::try_new(0) {
            Ok(timestamp) => timestamp,
            Err(_) => panic!("the epoch is in range"),
        };
        assert_eq!(EPOCH.millis(), 0);
    }
}
