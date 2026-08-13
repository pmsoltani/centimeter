//! Quantity: a number denominated in a commodity.
//!
//! A [`Quantity`] is the shape shared by a posting's `amount` and its `value`:
//! an exact decimal paired with the commodity it is measured in (for example,
//! "1.50 USD" or "2.2 kg"). Every balance is ultimately a sum of quantities.
//!
//! The number is held at exactly the commodity's scale, fixed at construction.
//! That turns the scale into a property of the value rather than something to
//! look up, eliminating the need for repeated commodity lookups.
//!
//! Nothing here is rounded. If a number has more decimal places than its
//! commodity allows, we reject it rather than trim it. Likewise, if an
//! operation produces a result that doesn't fit the commodity's scale, it will
//! also be rejected. What happens to any residue is a posting-level decision,
//! not something a quantity should decide on its own.

mod error;

use rust_decimal::Decimal;

use crate::{Commodity, CommodityId};

pub use error::QuantityError;

/// An exact decimal amount denominated in one commodity.
///
/// The number is stored at exactly the commodity's scale: [`try_new`] rescales
/// it on the way in, and every operation preserves it. So `number().scale()`
/// is always the commodity's scale, and further operations do not need to
/// resolve the commodity again.
///
/// A quantity is `Copy` and only stores the [`CommodityId`], not the commodity
/// itself. To resolve it, use the registry when you need the code or name.
///
/// # Examples
///
/// ```
/// # use centimeter_core::{CommodityId, CommodityRegistry, Decimal, Quantity};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut registry = CommodityRegistry::new();
/// let id = CommodityId::from_uuid(uuid::Uuid::now_v7())?;
/// registry.add(id, "USD", "US Dollar", 2)?;
/// let usd = registry.get(id).expect("just registered");
///
/// // 1.5 is stored as 1.50: the commodity's scale, not the input's.
/// let a = Quantity::try_new(Decimal::new(15, 1), usd)?;
/// assert_eq!(a.number().scale(), 2);
///
/// let b = Quantity::try_new(Decimal::new(50, 2), usd)?;
/// assert_eq!(a.checked_add(b)?.number(), Decimal::new(200, 2));
///
/// // More decimals than USD allows is refused, never rounded away.
/// assert!(Quantity::try_new(Decimal::new(1555, 3), usd).is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantity {
    number: Decimal,
    commodity: CommodityId,
}

impl Quantity {
    /// Creates a new quantity with the given number and commodity ID.
    ///
    /// # Errors
    /// - [`QuantityError::ScaleTooLarge`] if the number's significant decimal
    ///   places exceeds the commodity's scale.
    /// - [`QuantityError::NumberTooLarge`] if the number is too large to be
    ///   represented with the commodity's scale.
    pub fn try_new(number: Decimal, commodity: &Commodity) -> Result<Self, QuantityError> {
        Ok(Self { number: Self::validate_number(number, commodity)?, commodity: commodity.id() })
    }

    /// Returns the number of the quantity.
    #[must_use]
    pub fn number(self) -> Decimal {
        self.number
    }

    /// Returns the commodity ID of the quantity.
    #[must_use]
    pub fn commodity(self) -> CommodityId {
        self.commodity
    }

    /// Returns a new 'zero' quantity with the provided commodity, at its scale.
    ///
    /// Unlike [`try_new`](Self::try_new) this cannot fail: rescaling zero
    /// never grows the mantissa, so it fits at any scale.
    #[must_use]
    pub fn zero(commodity: &Commodity) -> Self {
        let mut zero = Decimal::ZERO;
        zero.rescale(u32::from(commodity.scale()));
        Self { number: zero, commodity: commodity.id() }
    }

    /// Returns true if the quantity is zero.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.number.is_zero()
    }

    /// Confirms both quantities share a commodity.
    ///
    /// Matching ids also mean matching scales, as `CommodityRegistry` freezes a
    /// commodity's scale and allows only one commodity per id.
    fn ensure_same_commodity(self, other: Self) -> Result<(), QuantityError> {
        if self.commodity != other.commodity {
            return Err(QuantityError::CommodityMismatch {
                left: self.commodity,
                right: other.commodity,
            });
        }
        Ok(())
    }

    /// Returns `number` at exactly the `commodity`'s scale.
    ///
    /// The value is never changed. Only trailing zeros are removed and missing
    /// decimal places are padded as required. If `number` has a significant
    /// digit beyond the commodity's scale, it is rejected rather than rounded.
    ///
    /// # Errors
    /// - [`ScaleTooLarge`](QuantityError::ScaleTooLarge) if `number` has more
    ///   significant decimal places than the commodity allows.
    /// - [`NumberTooLarge`](QuantityError::NumberTooLarge) if `number` is too big
    ///   to have that many decimal places.
    fn validate_number(number: Decimal, commodity: &Commodity) -> Result<Decimal, QuantityError> {
        let (code, scale) = (commodity.code(), commodity.scale());

        // Trailing zeros don't affect the value: `1.500` is the same as `1.50`.
        // Remove them before checking whether the number exceeds the allowed scale.
        let mut normalized = number.normalize();
        if u32::from(scale) < normalized.scale() {
            return Err(QuantityError::ScaleTooLarge { code: code.into(), scale, number });
        }

        // `Decimal::rescale()` works on a best-effort basis, so if the number
        // is too large for the commodity's scale, it will silently be rounded
        // to the maximum scale possible for that number. To avoid surprises, we
        // check the scale of the number after rescaling.
        normalized.rescale(u32::from(scale));
        if normalized.scale() != u32::from(scale) {
            return Err(QuantityError::NumberTooLarge { code: code.into(), scale, number });
        }
        Ok(normalized)
    }

    /// Adds two quantities together, returning a new quantity.
    ///
    /// # Errors
    /// - [`QuantityError::CommodityMismatch`] if the commodities do not match.
    /// - [`QuantityError::Overflow`] if the addition overflows.
    /// - [`QuantityError::Inexact`] if the resulting number has more than 28
    ///   significant digits.
    pub fn checked_add(self, other: Self) -> Result<Self, QuantityError> {
        self.ensure_same_commodity(other)?;
        let result = self
            .number
            .checked_add(other.number)
            .ok_or(QuantityError::Overflow { left: self.number, right: other.number })?;
        if result.scale() != self.number.scale() {
            return Err(QuantityError::Inexact { left: self.number, right: other.number });
        }
        Ok(Self { number: result, commodity: self.commodity })
    }

    /// Subtracts one quantity from another, returning a new quantity.
    ///
    /// # Errors
    /// - [`QuantityError::CommodityMismatch`] if the commodities do not match.
    /// - [`QuantityError::Overflow`] if the subtraction overflows.
    /// - [`QuantityError::Inexact`] if the resulting number has more than 28
    ///   significant digits.
    pub fn checked_sub(self, other: Self) -> Result<Self, QuantityError> {
        self.ensure_same_commodity(other)?;
        let result = self
            .number
            .checked_sub(other.number)
            .ok_or(QuantityError::Overflow { left: self.number, right: other.number })?;
        if result.scale() != self.number.scale() {
            return Err(QuantityError::Inexact { left: self.number, right: other.number });
        }
        Ok(Self { number: result, commodity: self.commodity })
    }
}

impl std::ops::Neg for Quantity {
    type Output = Self;

    fn neg(self) -> Self::Output {
        // Negating zero sets the sign bit, giving "-0.00", which renders badly.
        if self.number.is_zero() {
            return self;
        }
        Self { number: -self.number, commodity: self.commodity }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rust_decimal::Decimal;

    use super::*;

    use crate::test_support::{qty, registry};

    // Construction and scale normalisation
    #[test]
    fn test_try_new_stores_the_number_at_the_commodity_scale() {
        let registry = registry();
        let usd = registry.get_by_code("USD").unwrap();

        let a = Quantity::try_new(Decimal::new(15, 1), usd).unwrap();
        assert_eq!(a.number().scale(), 2);

        let b = Quantity::try_new(Decimal::new(5, 0), usd).unwrap();
        assert_eq!(b.number().scale(), 2);

        let c = Quantity::try_new(Decimal::new(349, 2), usd).unwrap();
        assert_eq!(c.number().scale(), 2);
    }

    #[test]
    fn test_try_new_rejects_excess_scale() {
        let registry = registry();
        let jpy = registry.get_by_code("JPY").unwrap();
        let btc = registry.get_by_code("BTC").unwrap();

        assert!(matches!(
            Quantity::try_new(Decimal::new(15, 1), jpy),
            Err(QuantityError::ScaleTooLarge { code, scale: 0, number })
            if code == "JPY" && number == Decimal::new(15, 1)
        ));

        assert!(matches!(
            Quantity::try_new(Decimal::new(1_123_456_789, 9), btc),
            Err(QuantityError::ScaleTooLarge { code, scale: 8, number })
            if code == "BTC" && number == Decimal::new(1_123_456_789, 9)
        ));

        let usd = registry.get_by_code("USD").unwrap();
        let err = Quantity::try_new(Decimal::new(1555, 3), usd).unwrap_err();
        assert!(matches!(err, QuantityError::ScaleTooLarge { scale: 2, .. }), "got {err}");
    }

    #[test]
    fn test_try_new_ignores_trailing_zeros() {
        let registry = registry();
        let usd = registry.get_by_code("USD").unwrap();
        let jpy = registry.get_by_code("JPY").unwrap();

        let padded = Quantity::try_new(Decimal::new(1500, 3), usd).unwrap();
        assert_eq!(padded.number(), Decimal::new(150, 2));
        assert_eq!(padded.number().scale(), 2);

        let yen = Quantity::try_new(Decimal::new(5000, 3), jpy).unwrap();
        assert_eq!(yen.number(), Decimal::new(5, 0));
        assert_eq!(yen.number().scale(), 0);

        let zero = Quantity::try_new(Decimal::new(0, 6), usd).unwrap();
        assert!(zero.is_zero());
        assert_eq!(zero.number().scale(), 2);
    }

    #[test]
    fn test_try_new_normalizes_a_negative_zero() {
        let registry = registry();
        let usd = registry.get_by_code("USD").unwrap();

        let built = Quantity::try_new(-Decimal::ZERO, usd).unwrap();
        assert!(built.is_zero());
        assert!(!built.number().is_sign_negative(), "stored a negative zero: {}", built.number());
    }

    #[test]
    fn test_try_new_handles_scale_zero_and_high_scale() {
        let registry = registry();
        let jpy = registry.get_by_code("JPY").unwrap();
        let btc = registry.get_by_code("BTC").unwrap();

        let five_jpy = Quantity::try_new(Decimal::new(5, 0), jpy).unwrap();
        assert_eq!(five_jpy.number().scale(), 0);

        assert!(matches!(
            Quantity::try_new(Decimal::new(55, 1), jpy),
            Err(QuantityError::ScaleTooLarge { code, scale: 0, number })
            if code == "JPY" && number == Decimal::new(55, 1)
        ));

        let one_satoshi = Quantity::try_new(Decimal::new(1, 8), btc).unwrap();
        assert_eq!(one_satoshi.number().scale(), 8);
    }

    #[test]
    fn test_try_new_rejects_a_number_too_large_for_the_scale() {
        let registry = registry();
        let hyp = registry.get_by_code("HYP").unwrap();

        let too_large = Decimal::new(8, 0);
        assert!(matches!(
            Quantity::try_new(too_large, hyp),
            Err(QuantityError::NumberTooLarge { code, scale: 28, number })
            if code == "HYP" && number == Decimal::new(8, 0)
        ));
    }

    #[test]
    fn test_try_new_accepts_negative_numbers() {
        let registry = registry();
        let usd = registry.get_by_code("USD").unwrap();

        let a = Quantity::try_new(Decimal::new(-15, 1), usd).unwrap();
        assert_eq!(a.number().scale(), 2);

        let b = Quantity::try_new(Decimal::new(-5, 0), usd).unwrap();
        assert_eq!(b.number().scale(), 2);

        let c = Quantity::try_new(Decimal::new(-349, 2), usd).unwrap();
        assert_eq!(c.number().scale(), 2);
    }

    // Zero
    #[test]
    fn test_zero_is_at_the_commodity_scale() {
        let registry = registry();
        let usd = registry.get_by_code("USD").unwrap();
        let jpy = registry.get_by_code("JPY").unwrap();
        let zero_usd = Quantity::zero(usd);
        let zero_jpy = Quantity::zero(jpy);
        assert_eq!(zero_usd.number().scale(), 2);
        assert_eq!(zero_jpy.number().scale(), 0);
    }

    #[test]
    fn test_is_zero_tracks_the_number() {
        let registry = registry();
        let usd = registry.get_by_code("USD").unwrap();
        let zero_usd = Quantity::zero(usd);
        assert!(zero_usd.is_zero());

        let nonzero_usd = Quantity::try_new(Decimal::new(1, 2), usd).unwrap();
        assert!(!nonzero_usd.is_zero());
    }

    // Negation
    #[test]
    fn test_neg_flips_sign_and_preserves_scale_and_commodity() {
        let registry = registry();

        let a = qty(Decimal::new(15, 1), "USD", &registry);
        let neg_a = -a;
        assert_eq!(neg_a.number(), Decimal::new(-15, 1));
        assert_eq!(neg_a.number().scale(), 2);
        assert_eq!(neg_a.commodity(), a.commodity());
    }

    #[test]
    fn test_neg_preserves_zero_sign() {
        let registry = registry();
        let usd = registry.get_by_code("USD").unwrap();
        let zero_usd = Quantity::zero(usd);
        let neg_zero_usd = -zero_usd;
        assert_eq!(neg_zero_usd.number(), Decimal::ZERO);
        assert!(neg_zero_usd.number().is_sign_positive());
        assert_eq!(neg_zero_usd.number().scale(), 2);
        assert_eq!(neg_zero_usd.commodity(), zero_usd.commodity());
    }

    // Arithmetic
    #[test]
    fn test_add_and_sub() {
        let registry = registry();
        let a = qty(Decimal::new(150, 1), "USD", &registry); // 15.0 USD
        let b = qty(Decimal::new(50, 2), "USD", &registry); // 0.50 USD

        let sum = a.checked_add(b).unwrap();
        assert_eq!(sum.number(), Decimal::new(1550, 2)); // 15.50 USD

        let diff = a.checked_sub(b).unwrap();
        assert_eq!(diff.number(), Decimal::new(1450, 2)); // 14.50 USD
    }

    #[test]
    fn test_arithmetic_preserves_scale() {
        let registry = registry();
        let a = qty(Decimal::new(150, 1), "USD", &registry); // 15.0 USD
        let b = qty(Decimal::new(50, 2), "USD", &registry); // 0.50 USD

        let sum = a.checked_add(b).unwrap();
        assert_eq!(sum.number().scale(), 2);

        let diff = a.checked_sub(b).unwrap();
        assert_eq!(diff.number().scale(), 2);
    }

    #[test]
    fn test_add_rejects_commodity_mismatch() {
        let registry = registry();
        let usd = registry.get_by_code("USD").unwrap();
        let jpy = registry.get_by_code("JPY").unwrap();
        let a = qty(Decimal::new(150, 1), "USD", &registry); // 15.0 USD
        let b = qty(Decimal::new(50, 0), "JPY", &registry); // 50 JPY

        assert!(matches!(
            a.checked_add(b),
            Err(QuantityError::CommodityMismatch { left, right })
            if left == usd.id() && right == jpy.id()
        ));
    }

    #[test]
    fn test_add_detects_overflow() {
        // Overflow needs a scale-0 commodity: `Decimal::MAX` spends all 29 of
        // its digits before the decimal point, so it cannot be rescaled to
        // USD's 2 places at all and would fail construction instead.
        let registry = registry();
        let a = qty(Decimal::MAX, "JPY", &registry);
        let b = qty(Decimal::ONE, "JPY", &registry);

        assert!(matches!(
            a.checked_add(b),
            Err(QuantityError::Overflow { left, right })
            if left == Decimal::MAX && right == Decimal::ONE
        ));
    }

    #[test]
    fn test_sub_detects_overflow() {
        let registry = registry();
        let a = qty(Decimal::MIN, "JPY", &registry);
        let b = qty(Decimal::ONE, "JPY", &registry);

        assert!(matches!(
            a.checked_sub(b),
            Err(QuantityError::Overflow { left, right })
            if left == Decimal::MIN && right == Decimal::ONE
        ));
    }

    #[test]
    fn test_add_detects_inexact() {
        let registry = registry();
        let valid1 = qty(Decimal::new(4, 0), "HYP", &registry); // 4.0000000000000000000000000000 HYP
        let valid2 = qty(Decimal::new(3, 0), "HYP", &registry); // 3.0000000000000000000000000000 HYP

        assert_eq!(valid1.checked_add(valid2).unwrap().number(), Decimal::new(7, 0));

        assert!(matches!(
            valid1.checked_add(valid1),
            Err(QuantityError::Inexact { left, right })
            if left == Decimal::new(4, 0) && right == Decimal::new(4, 0)
        ));
    }

    // proptests
    //
    // An `i64` mantissa at a fixed scale always fits a `Decimal`, so `try_new`
    // never fails here and the laws below are exercised on real values rather
    // than on rejections. Overflow and `Inexact` are covered by the unit tests
    // above, which need magnitudes an `i64` cannot reach.
    proptest! {
        /// A quantity always lands at exactly its commodity's scale.
        #[test]
        fn prop_try_new_preserves_the_commodity_scale(
            mantissa: i64,
            code in prop::sample::select(vec!["USD", "JPY", "BTC", "HYP"]),
        ) {
            let registry = registry();
            let commodity = registry.get_by_code(code).unwrap();
            let scale = u32::from(commodity.scale());

            let built = Quantity::try_new(Decimal::new(mantissa, scale), commodity).unwrap();
            prop_assert_eq!(built.number().scale(), scale);
        }

        /// A number needing more decimals than its commodity allows is always
        /// refused, never quietly rounded to fit (ADR-0004).
        #[test]
        fn prop_try_new_never_rounds(units in 1i64..100_000, last in 1i64..=9) {
            let registry = registry();
            let usd = registry.get_by_code("USD").unwrap();

            // A non-zero third decimal, so the excess digit is significant.
            let number = Decimal::new(units * 10 + last, 3);
            let built = Quantity::try_new(number, usd);
            prop_assert!(
                matches!(built, Err(QuantityError::ScaleTooLarge { .. })),
                "{number} should be refused by a scale-2 commodity, got {built:?}"
            );
        }

        /// Trailing zeros are spelling, not precision: padding a number with
        /// them changes nothing about the quantity it builds.
        #[test]
        fn prop_try_new_ignores_trailing_zeros(
            mantissa in -1_000_000_000i64..1_000_000_000,
            extra in 1u32..=4,
        ) {
            let registry = registry();
            let usd = registry.get_by_code("USD").unwrap();

            let plain = Quantity::try_new(Decimal::new(mantissa, 2), usd).unwrap();
            let padded = Decimal::new(mantissa * 10i64.pow(extra), 2 + extra);
            let padded = Quantity::try_new(padded, usd).unwrap();

            prop_assert_eq!(padded, plain);
            // `Quantity`'s `PartialEq` ignores scale, so assert it separately.
            prop_assert_eq!(padded.number().scale(), plain.number().scale());
        }

        /// Arithmetic preserves scale exactly (ADR-0023).
        #[test]
        fn prop_arithmetic_never_changes_scale(a: i64, b: i64) {
            let registry = registry();
            let x = qty(Decimal::new(a, 2), "USD", &registry);
            let y = qty(Decimal::new(b, 2), "USD", &registry);

            prop_assert_eq!(x.checked_add(y).unwrap().number().scale(), 2);
            prop_assert_eq!(x.checked_sub(y).unwrap().number().scale(), 2);
            prop_assert_eq!((-x).number().scale(), 2);
        }

        /// Addition commutes.
        #[test]
        fn prop_add_is_commutative(a: i64, b: i64) {
            let registry = registry();
            let x = qty(Decimal::new(a, 2), "USD", &registry);
            let y = qty(Decimal::new(b, 2), "USD", &registry);

            prop_assert_eq!(x.checked_add(y).ok(), y.checked_add(x).ok());
        }

        /// Zero is the additive identity, on both sides.
        #[test]
        fn prop_zero_is_the_additive_identity(a: i64) {
            let registry = registry();
            let usd = registry.get_by_code("USD").unwrap();
            let x = Quantity::try_new(Decimal::new(a, 2), usd).unwrap();
            let zero = Quantity::zero(usd);

            prop_assert_eq!(x.checked_add(zero).unwrap(), x);
            prop_assert_eq!(zero.checked_add(x).unwrap(), x);
            prop_assert_eq!(x.checked_sub(zero).unwrap(), x);
        }

        /// Subtraction agrees with adding the negation.
        #[test]
        fn prop_sub_is_add_of_neg(a: i64, b: i64) {
            let registry = registry();
            let x = qty(Decimal::new(a, 2), "USD", &registry);
            let y = qty(Decimal::new(b, 2), "USD", &registry);

            prop_assert_eq!(x.checked_sub(y).ok(), x.checked_add(-y).ok());
        }

        /// Adding then subtracting the same quantity returns the original.
        #[test]
        fn prop_add_then_sub_round_trips(a: i64, b: i64) {
            let registry = registry();
            let x = qty(Decimal::new(a, 2), "USD", &registry);
            let y = qty(Decimal::new(b, 2), "USD", &registry);

            let back = x.checked_add(y).unwrap().checked_sub(y);
            prop_assert!(back.is_ok(), "an exact round trip must succeed, got {back:?}");
            prop_assert_eq!(back.unwrap(), x);
        }

        /// Negation is its own inverse and leaves the commodity alone.
        #[test]
        fn prop_neg_is_an_involution(a: i64) {
            let registry = registry();
            let x = qty(Decimal::new(a, 2), "USD", &registry);

            prop_assert_eq!(-(-x), x);
            prop_assert_eq!((-x).commodity(), x.commodity());
        }

        /// No operation ever yields a negative zero, which would render as
        /// `-0.00` in a report.
        #[test]
        fn prop_never_produces_a_negative_zero(a: i64, b: i64) {
            let registry = registry();
            let x = qty(Decimal::new(a, 2), "USD", &registry);
            let y = qty(Decimal::new(b, 2), "USD", &registry);

            // `x - x` is the reliable way to reach zero on every case.
            let results = [Ok(-x), x.checked_sub(x), x.checked_add(y), x.checked_sub(y)];
            for value in results.into_iter().flatten() {
                prop_assert!(
                    !value.is_zero() || !value.number().is_sign_negative(),
                    "produced a negative zero: {value:?}"
                );
            }
        }
    }
}
