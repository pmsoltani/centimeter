//! Civil calendar conversions, from days since the epoch to a year-month-day
//! triple and back.
//!
//! The calendar is the proleptic Gregorian one, as prescribed by ISO 8601. The
//! algorithms are Howard Hinnant's, donated to the public domain and documented
//! at <https://howardhinnant.github.io/date_algorithms.html>.
//!
//! The trick behind them is a single line: the year is relabeled to start on
//! March 1st, which moves the leap day to the end of the year. Month lengths
//! from March then follow a strict 5-month, 153-day cycle, so the day of the
//! year has a closed form and needs no lookup table, and the calendar repeats
//! exactly every 400 years with no remainder.
//!
//! The functions are unchecked: they assume their conditions are met and will
//! produce the wrong answer rather than an error if those are violated, so
//! validation must be performed by the caller.

/// Days in one 400-year Gregorian cycle: `400 * 365 + 97` leap days.
const DAYS_PER_ERA: i32 = 146_097;

/// Days from `0000-03-01`, where the March-based year begins, to the
/// `1970-01-01` epoch. Shifts the algorithm's origin onto the stored one.
const EPOCH_SHIFT: i32 = 719_468;

/// Number of days since 1970-01-01 (negative for dates before 1970-01-01).
///
/// Conditions:
/// - y-m-d represents a date in the civil (Gregorian) calendar
/// - m is in `[1, 12]`
/// - d is in `[1, last_day_of_month(y, m)]`
pub(super) const fn days_from_civil(y: i32, m: i32, d: i32) -> i32 {
    // January and February belong to the previous March-based year.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400); // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 }; // [0, 11], March-based month
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * DAYS_PER_ERA + doe - EPOCH_SHIFT
}

/// Returns (year, month, day) triple in civil calendar
///
/// Conditions:
/// - z is a number of days since 1970-01-01, in
///   `[i32::MIN + EPOCH_SHIFT, i32::MAX - EPOCH_SHIFT]`
pub(super) const fn civil_from_days(z: i32) -> (i32, i32, i32) {
    let z = z + EPOCH_SHIFT;
    let era = z.div_euclid(DAYS_PER_ERA);
    let doe = z.rem_euclid(DAYS_PER_ERA); // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11], March-based month
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (y + (m <= 2) as i32, m, d)
}

/// Returns true if y is a leap year in the civil calendar.
pub(super) const fn is_leap(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// Returns the number of days in the month m of year y, in the range [28, 31].
///
/// Conditions:
/// - m is in [1, 12]
pub(super) const fn last_day_of_month(y: i32, m: i32) -> i32 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Roughly 3.65 million iterations, under a second unoptimised.
    #[test]
    fn test_every_supported_day_round_trips_in_both_directions() {
        let mut z = days_from_civil(1, 1, 1);
        for y in 1..=9999 {
            for m in 1..=12 {
                for d in 1..=last_day_of_month(y, m) {
                    assert_eq!(days_from_civil(y, m, d), z, "{y:04}-{m:02}-{d:02} is not day {z}");
                    assert_eq!(
                        civil_from_days(z),
                        (y, m, d),
                        "day {z} is not {y:04}-{m:02}-{d:02}"
                    );
                    z += 1;
                }
            }
        }
        assert_eq!(z - 1, days_from_civil(9999, 12, 31), "the sweep skipped or repeated a day");
    }

    /// `div_euclid` stands in for the branch Hinnant needs in C++, where
    /// integer division truncates toward zero instead of flooring. The two must
    /// agree everywhere, and only negative inputs can tell them apart.
    #[test]
    fn test_div_euclid_matches_the_reference_branch() {
        for y in -5_000..=10_000i32 {
            let reference = (if y >= 0 { y } else { y - 399 }) / 400;
            assert_eq!(y.div_euclid(400), reference, "era differs at y={y}");
            assert_eq!(y.rem_euclid(400), y - reference * 400, "yoe differs at y={y}");
        }
        for z in (-2_000_000..=2_000_000).step_by(7) {
            let reference = (if z >= 0 { z } else { z - 146_096 }) / DAYS_PER_ERA;
            assert_eq!(z.div_euclid(DAYS_PER_ERA), reference, "era differs at z={z}");
        }
    }

    /// The epoch itself, and the day either side of it, which is where an
    /// off-by-one in `EPOCH_SHIFT` would show up first.
    #[test]
    fn test_the_epoch_is_where_it_should_be() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(days_from_civil(2026, 8, 22), 20_687);
    }

    /// Divisible by four, except centuries, except multiples of four hundred.
    #[test]
    fn test_the_leap_rule_is_the_gregorian_one() {
        for y in [1600, 2000, 2024, 2400] {
            assert!(is_leap(y), "{y} should be a leap year");
            assert_eq!(last_day_of_month(y, 2), 29);
        }
        for y in [1700, 1800, 1900, 2023, 2100] {
            assert!(!is_leap(y), "{y} should not be a leap year");
            assert_eq!(last_day_of_month(y, 2), 28);
        }
    }

    #[test]
    fn test_month_lengths_sum_to_the_year() {
        let common: Vec<i32> = (1..=12).map(|m| last_day_of_month(2023, m)).collect();
        assert_eq!(common, [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]);
        assert_eq!(common.iter().sum::<i32>(), 365);
        assert_eq!((1..=12).map(|m| last_day_of_month(2024, m)).sum::<i32>(), 366);
    }
}
