//! The ordered pair of commodities that a conversion rate relates.

use crate::{CommodityId, RateError};

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
/// [`Rate::Conversion`](crate::Rate::Conversion) from representing an identity
/// conversion (represented by [`Rate::Identity`](crate::Rate::Identity)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommodityPair {
    quote: CommodityId,
    base: CommodityId,
}

impl CommodityPair {
    /// Creates a pair from two distinct commodities.
    ///
    /// # Errors
    /// - [`SameCommodity`](crate::RateError::SameCommodity) if `quote` and
    ///   `base` are the same commodity.
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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    use crate::test_support::id;

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

    proptest! {
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
