//! Rates in centimeter.
//!
//! A rate represents the ratio between two commodities (e.g., how much USD is
//! required to buy one GBP). It forms the foundation of multi-currency
//! bookkeeping, satisfying the equation: `amount * rate = value`.
//!
//! # A commodity converts to itself at exactly 1
//!
//! This is a statement about units, not about market prices. If two things
//! exchange at anything other than 1:1, they are different commodities, even
//! when they have similar names. For example, cash dollars and deposit dollars
//! may trade at different prices under capital controls; gas at different
//! delivery points may have different rates; and restricted shares may trade
//! differently from freely tradable shares. In each case, the two things are
//! distinct commodities, and [`Rate::Conversion`] already covers them.
//!
//! Discounting may look like an exception, but it is not. $100 today is worth
//! more than a hundred dollars next year, but that is a valuation across time,
//! not a difference in units. A discount factor is dimensionless, while a rate
//! has the unit `quote/base`. Treating a discount factor as a rate would
//! therefore undermine the meaning of `amount * rate = value`.
//!
//! Accounting reflects the same distinction: the discounted amount is recorded,
//! and the difference is recognized as finance cost. That is a matter of
//! postings, not a currency conversion.

mod error;

use rust_decimal::Decimal;

use crate::CommodityId;

pub use error::RateError;

/// An ordered pair of distinct commodities used to represent a conversion rate.
///
/// The pair is written as `quote/base`: the quote commodity is the numerator
/// and the base commodity is the denominator. For example, `JPY/USD = 150`
/// means 1 USD is worth 150 JPY.
///
/// Direction matters: `JPY/USD` and `USD/JPY` are distinct pairs, compare
/// unequal, and produce different hashes.
///
/// The quote and base commodities must be different. This invariant prevents
/// [`Rate::Conversion`] from representing an identity conversion, which is
/// instead represented by [`Rate::Identity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommodityPair {
    quote: CommodityId,
    base: CommodityId,
}

impl CommodityPair {
    /// Creates a pair from two distinct commodities.
    ///
    /// # Errors
    /// - [`SameCommodity`](RateError::SameCommodity) if `quote` and `base` are
    ///   the same commodity.
    pub fn try_new(quote: CommodityId, base: CommodityId) -> Result<Self, RateError> {
        if quote == base {
            return Err(RateError::SameCommodity { got: quote });
        }
        Ok(Self { quote, base })
    }

    /// Returns the quote (numerator) commodity.
    #[must_use]
    pub fn quote(self) -> CommodityId {
        self.quote
    }

    /// Returns the base (denominator) commodity.
    #[must_use]
    pub fn base(self) -> CommodityId {
        self.base
    }
}

/// A ratio used to convert a quantity of one commodity into another.
///
/// The conversion is expressed as:
///
/// `amount_in_base * rate = value_in_quote`
///
/// where the rate is written as `quote/base`. For example, `1.25 USD/GBP` means
/// that 1 GBP is worth 1.25 USD.
///
/// **Note:** This notation differs from conventional FX market notation, where
/// the currency pair is written as `base/quote`. Here, `quote/base` explicitly
/// describes the units of the rate: quote commodity per unit of base commodity.
///
/// There are two structurally distinct kinds of rate:
///
/// - [`Identity`](Self::Identity) represents a commodity converted to itself.
///   It is always exactly one.
/// - [`Conversion`](Self::Conversion) represents a conversion between two
///   distinct commodities and carries both the numeric rate and its
///   [`CommodityPair`](crate::CommodityPair). A conversion whose number happens
///   to be one is still a conversion: replacing it with an identity rate would
///   lose the commodities that the rate relates.
///
/// Unlike [`Quantity`](crate::Quantity), a rate is not constrained by the
/// scale of either commodity. It is a ratio, not an amount, so a rate such as
/// `0.85671234 GBP/EUR` is valid even if GBP is represented with two decimal
/// places.
///
/// Rates may also be zero or negative. Zero can represent a transfer with no
/// value, while negative rates can occur in real markets, such as the negative
/// oil prices seen in April 2020.
///
/// Rates do not support arithmetic. Adding or subtracting rates, or negating
/// one, has no generally meaningful interpretation, so `Add`, `Sub`, and `Neg`
/// are deliberately not implemented.
///
/// # Examples
///
/// ```
/// # use centimeter_core::{CommodityId, Decimal, Rate};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let usd = CommodityId::from_uuid(uuid::Uuid::now_v7())?;
/// let gbp = CommodityId::from_uuid(uuid::Uuid::now_v7())?;
///
/// // 1.25 USD per GBP: quote over base.
/// // No registry is needed: a rate carries its commodity ids.
/// let rate = Rate::try_new(Decimal::new(125, 2), usd, gbp)?;
/// assert_eq!(rate.quote(), usd);
/// assert_eq!(rate.base(), gbp);
///
/// // The same commodity has an identity rate of exactly one.
/// assert_eq!(Rate::try_new(Decimal::ONE, usd, usd)?, Rate::Identity(usd));
///
/// // A non-one rate between the same commodity is invalid.
/// assert!(Rate::try_new(Decimal::new(2, 0), usd, usd).is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rate {
    /// A rate showing 1:1 conversion of the same commodity (e.g., 1 USD per USD).
    Identity(CommodityId),
    /// A conversion rate between two distinct commodities (e.g., 1.25 USD per GBP).
    Conversion {
        /// The numeric multiplier of the rate (e.g., 1.25).
        number: Decimal,
        /// The pair of distinct commodities involved in the conversion.
        pair: CommodityPair,
    },
}

impl Rate {
    /// Creates a new rate from a number and two commodities.
    ///
    /// # Errors
    /// - [`RateError::BadIdentityRate`] if the commodities are identical but
    ///   the provided number is not exactly `1.0`.
    pub fn try_new(
        number: Decimal,
        quote: CommodityId,
        base: CommodityId,
    ) -> Result<Self, RateError> {
        match (quote == base, number) {
            (true, n) if n == Decimal::ONE => Ok(Self::Identity(quote)),
            (true, _) => Err(RateError::BadIdentityRate { got: number }),
            (false, _) => {
                // The pair cannot fail here: this arm is reached only when the
                // commodities are distinct.
                Ok(Self::Conversion { number, pair: CommodityPair::try_new(quote, base)? })
            }
        }
    }

    /// Returns the numeric multiplier of the rate.
    #[must_use]
    pub fn number(self) -> Decimal {
        match self {
            Self::Identity(_) => Decimal::ONE,
            Self::Conversion { number, .. } => number,
        }
    }

    /// Returns the quote (numerator) commodity of the rate.
    #[must_use]
    pub fn quote(self) -> CommodityId {
        match self {
            Self::Identity(commodity) => commodity,
            Self::Conversion { pair, .. } => pair.quote(),
        }
    }

    /// Returns the base (denominator) commodity of the rate.
    #[must_use]
    pub fn base(self) -> CommodityId {
        match self {
            Self::Identity(commodity) => commodity,
            Self::Conversion { pair, .. } => pair.base(),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rust_decimal::Decimal;

    use super::*;

    use crate::test_support::id;

    // CommodityPair

    #[test]
    fn test_pair_keeps_quote_and_base_distinct() {
        let quote = id(0);
        let base = id(1);
        let pair = CommodityPair::try_new(quote, base).unwrap();
        assert_eq!(pair.quote(), quote);
        assert_eq!(pair.base(), base);
    }

    #[test]
    fn test_pair_rejects_the_same_commodity() {
        let commodity = id(0);
        let err = CommodityPair::try_new(commodity, commodity).unwrap_err();
        assert!(matches!(err, RateError::SameCommodity { got } if got == commodity));
    }

    #[test]
    fn test_pair_direction_matters() {
        let quote = id(0);
        let base = id(1);
        let pair1 = CommodityPair::try_new(quote, base).unwrap();
        let pair2 = CommodityPair::try_new(base, quote).unwrap();
        assert_ne!(pair1, pair2);
    }

    #[test]
    fn test_pair_is_copy() {
        let quote = id(0);
        let base = id(1);
        let pair1 = CommodityPair::try_new(quote, base).unwrap();
        let pair2 = pair1;
        assert_eq!(pair1, pair2);
    }

    // Rate construction

    #[test]
    fn test_try_new_builds_a_conversion_for_distinct_commodities() {
        let number = Decimal::new(1, 1); // 0.1
        let quote = id(0);
        let base = id(1);
        let rate = Rate::try_new(number, quote, base).unwrap();
        assert!(matches!(rate, Rate::Conversion { .. }));
        assert_eq!(rate.number(), number);
        assert_eq!(rate.quote(), quote);
        assert_eq!(rate.base(), base);
        assert_ne!(rate.quote(), rate.base());
    }

    #[test]
    fn test_try_new_builds_an_identity_for_one() {
        let number = Decimal::ONE;
        let commodity = id(0);
        let rate = Rate::try_new(number, commodity, commodity).unwrap();
        assert!(matches!(rate, Rate::Identity(_)));
        assert_eq!(rate.number(), number);
        assert_eq!(rate.quote(), commodity);
        assert_eq!(rate.base(), commodity);
    }

    #[test]
    fn test_identity_accepts_any_spelling_of_one() {
        let commodity = id(0);
        for n in [Decimal::ONE, Decimal::new(10, 1), Decimal::new(100, 2)] {
            let rate = Rate::try_new(n, commodity, commodity).unwrap();
            assert!(matches!(rate, Rate::Identity(_)));
            assert_eq!(rate.number(), n);
            assert_eq!(rate.quote(), commodity);
            assert_eq!(rate.base(), commodity);
            assert_eq!(rate.number().scale(), 0);
        }
    }

    #[test]
    fn test_try_new_rejects_a_non_unit_identity() {
        let number = Decimal::new(2, 0); // 2.0
        let commodity = id(0);
        let err = Rate::try_new(number, commodity, commodity).unwrap_err();
        assert!(matches!(err, RateError::BadIdentityRate { got } if got == number));
    }

    #[test]
    fn test_a_unit_conversion_is_not_an_identity() {
        let number = Decimal::ONE;
        let quote = id(0);
        let base = id(1);
        let rate = Rate::try_new(number, quote, base).unwrap();
        assert!(matches!(rate, Rate::Conversion { .. }));
    }

    // Zero, negative, scale

    #[test]
    fn test_zero_and_negative_rates_are_accepted() {
        let quote = id(0);
        let base = id(1);
        for n in [Decimal::ZERO, Decimal::new(-1, 0), Decimal::new(-12345, 3)] {
            let rate = Rate::try_new(n, quote, base).unwrap();
            assert_eq!(rate.number(), n);
            assert_eq!(rate.quote(), quote);
            assert_eq!(rate.base(), base);
        }
    }

    #[test]
    fn test_rates_are_not_scale_constrained() {
        let quote = id(0);
        let base = id(1);
        for n in [Decimal::new(12345, 3), Decimal::new(12345, 5), Decimal::new(12345, 10)] {
            let rate = Rate::try_new(n, quote, base).unwrap();
            assert_eq!(rate.number(), n);
            assert_eq!(rate.quote(), quote);
            assert_eq!(rate.base(), base);
        }
    }

    // Proptests

    proptest! {
        /// Any number and any distinct pair survive construction unchanged.
        #[test]
        fn prop_try_new_round_trips_its_inputs(
            mantissa: i64,
            scale in 0u32..=28,
            seed: u64,
            offset in 1u64..=1_000_000,
        ) {
            let number = Decimal::new(mantissa, scale);
            let (quote, base) = (id(seed), id(seed.wrapping_add(offset)));

            let rate = Rate::try_new(number, quote, base).unwrap();
            prop_assert_eq!(rate.number(), number);
            prop_assert_eq!(rate.quote(), quote);
            prop_assert_eq!(rate.base(), base);
        }

        /// One commodity yields `Identity` exactly when the number is one, and
        /// `BadIdentityRate` otherwise. It never yields a `Conversion`, and
        /// never `SameCommodity`; the match separates that case first.
        #[test]
        fn prop_same_commodity_is_identity_or_error(
            mantissa: i64,
            scale in 0u32..=28,
            seed: u64,
        ) {
            let number = Decimal::new(mantissa, scale);
            let commodity = id(seed);
            let built = Rate::try_new(number, commodity, commodity);

            if number == Decimal::ONE {
                prop_assert_eq!(built.unwrap(), Rate::Identity(commodity));
            } else {
                prop_assert!(
                    matches!(built, Err(RateError::BadIdentityRate { got }) if got == number),
                    "{number} against one commodity should be refused, got {built:?}"
                );
            }
        }

        /// Distinct commodities always convert, whatever the number (including
        /// one), since a unit conversion is not an identity.
        #[test]
        fn prop_distinct_commodities_always_convert(
            mantissa: i64,
            scale in 0u32..=28,
            seed: u64,
            offset in 1u64..=1_000_000,
        ) {
            let number = Decimal::new(mantissa, scale);
            let (quote, base) = (id(seed), id(seed.wrapping_add(offset)));

            let rate = Rate::try_new(number, quote, base).unwrap();
            prop_assert!(matches!(rate, Rate::Conversion { .. }), "got {rate:?}");
            prop_assert_ne!(rate.quote(), rate.base());
        }

        /// Two rates are equal exactly when their number, quote and base all
        /// agree. Guards the derived `PartialEq` against a hand-written one.
        #[test]
        fn prop_rate_equality_matches_its_parts(
            left_mantissa: i64,
            right_mantissa: i64,
            scale in 0u32..=28,
            seed: u64,
            offset in 1u64..=1_000,
            reversed: bool,
        ) {
            let (x, y) = (id(seed), id(seed.wrapping_add(offset)));
            let left = Rate::try_new(Decimal::new(left_mantissa, scale), x, y).unwrap();
            let right_number = Decimal::new(right_mantissa, scale);
            let right = if reversed {
                Rate::try_new(right_number, y, x).unwrap()
            } else {
                Rate::try_new(right_number, x, y).unwrap()
            };

            let same_parts = left.number() == right.number()
                && left.quote() == right.quote()
                && left.base() == right.base();
            prop_assert_eq!(left == right, same_parts);
        }

        /// A pair keeps its direction: reversing it never compares equal, and
        /// the two land as separate entries in a hash set.
        #[test]
        fn prop_pair_direction_is_significant(seed: u64, offset in 1u64..=1_000_000) {
            let (quote, base) = (id(seed), id(seed.wrapping_add(offset)));
            let forward = CommodityPair::try_new(quote, base).unwrap();
            let reverse = CommodityPair::try_new(base, quote).unwrap();

            prop_assert_ne!(forward, reverse);
            let set: std::collections::HashSet<_> = [forward, reverse].into_iter().collect();
            prop_assert_eq!(set.len(), 2, "Hash must agree with Eq on direction");
        }
    }
}
