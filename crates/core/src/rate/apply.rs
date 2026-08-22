//! Applies a rate to derive the missing side of `amount * rate = value`.
//!
//! This is the only place in core where rounding happens. Elsewhere, the engine
//! stays exact by construction: [`Quantity`] rejects values that do not fit its
//! commodity's scale instead of silently trimming them. Here, a product or
//! quotient must fit the target commodity's scale, so rounding is sometimes
//! unavoidable. For example, if `USD` is defined to have two decimal places, we
//! cannot have `53.649 USD` and we round it to `53.65 USD`.
//!
//! The two directions are deliberately not symmetric:
//!
//! - **Multiplication should be exact.** A product needs at most the sum of its
//!   operands' scales. As long as that sum does not exceed 28, nothing is lost
//!   before we round to the target scale. [`Rate::apply_amount`] checks this
//!   and reports the residue introduced by that rounding.
//! - **Division usually is not.** For example, `100.00 / 0.8567` cannot be
//!   represented exactly as a decimal, and so there is no equivalent exactness
//!   check to make.

use rust_decimal::{Decimal, RoundingStrategy};

use crate::{Commodity, Quantity, Rate, RateError};

impl Rate {
    /// How core rounds when applying `rate`.
    ///
    /// Half-up away from zero, so `round(-x) == -round(x)`: a reversal cancels
    /// its original exactly, as this mode of rounding is an odd function. Note
    /// that this is not a parameter, since a ledger whose lines round with
    /// different strategies cannot be audited.
    const ROUNDING_STRATEGY: RoundingStrategy = RoundingStrategy::MidpointAwayFromZero;

    /// Derives a value from an amount: `value = round(amount * rate)`.
    ///
    /// Returns the rounded value & the signed residue `amount * rate - value`.
    /// The residue is the part of the exact product that does not fit the quote
    /// commodity's scale.
    ///
    /// The caller is responsible for ensuring that `amount` is in this rate's
    /// base commodity and `quote` is its quote commodity. Those invariants are
    /// not checked here.
    ///
    /// # Errors
    /// - [`RateError::Overflow`] if the multiplication overflows.
    /// - [`RateError::InexactProduct`] if the product needs more than 28
    ///   significant digits. `rust_decimal` otherwise silently drops precision.
    /// - [`RateError::Quantity`] if the rounded value does not fit `quote`.
    pub(crate) fn apply_amount(
        &self,
        amount: Quantity,
        quote: &Commodity,
    ) -> Result<(Quantity, Decimal), RateError> {
        let (a, r) = (amount.number(), self.number());

        // Trailing zeros are formatting, not precision, and they consume the
        // scale available for the product. For example, multiplying `4.000...0`
        // (28 decimal places) by `1.50` (2 decimal places) is exactly `6`, even
        // though the raw operands scales add up to 30. Normalize the operands
        // before multiplying so an exact product doesn't look less precise just
        // because the inputs were written with lots of trailing zeros.
        let (a, r) = (a.normalize(), r.normalize());
        let product = a.checked_mul(r).ok_or(RateError::Overflow { left: a, right: r })?;
        Self::ensure_exact_product(a, r, product)?;

        let scale = u32::from(quote.scale());
        let rounded = product.round_dp_with_strategy(scale, Self::ROUNDING_STRATEGY);
        let value = Quantity::try_new(rounded, quote)?;
        Ok((value, product - rounded)) // The residue is in value commodity's unit
    }

    /// Derives an amount from a value: `amount = round(value / rate)`.
    ///
    /// This function does not return a residue. The residue would be in the
    /// base commodity, but only the functional value side has a zero-sum
    /// invariant that can require a rounding residue to be carried forward.
    /// Also, the amount derived here is a purely calculated number: if the
    /// caller had an amount from a document, it would supply that amount
    /// instead. There is therefore nothing external for this derived amount to
    /// reconcile against.
    ///
    /// Unlike [`apply_amount`](Self::apply_amount), there is no exactness
    /// guarantee to enforce. A quotient such as `100.00 / 0.8567` is generally
    /// inexact before it is rounded to the base commodity's scale.
    ///
    /// The caller is responsible for ensuring that `value` is in this rate's
    /// quote commodity and `base` is its base commodity. Those invariants are
    /// not checked here.
    ///
    /// # Errors
    /// - [`RateError::Overflow`] if the rate is zero or the division overflows.
    /// - [`RateError::Quantity`] if the rounded amount does not fit `base`.
    pub(crate) fn apply_value(
        &self,
        value: Quantity,
        base: &Commodity,
    ) -> Result<Quantity, RateError> {
        let (v, r) = (value.number(), self.number());
        let quotient = v.checked_div(r).ok_or(RateError::Overflow { left: v, right: r })?;

        let scale = u32::from(base.scale());
        let rounded = quotient.round_dp_with_strategy(scale, Self::ROUNDING_STRATEGY);
        Quantity::try_new(rounded, base).map_err(RateError::from)
    }

    /// Checks that multiplying the two operands did not lose any precision.
    ///
    /// `rust_decimal` limits results to 28 decimal places. If the product needs
    /// more, it silently reduces the precision using half-to-even rounding;
    /// with a large enough excess, the result can even collapse to zero! Since
    /// neither case is reported as an error, we detect the lost precision by
    /// checking the product's scale against the sum of the operands' scales.
    ///
    /// A zero operand is exempt: multiplication returns `Decimal::ZERO` at
    /// scale 0 without calculating the product, and a zero rate is valid.
    ///
    /// # Errors
    /// - [`RateError::InexactProduct`] if the product has lost precision.
    fn ensure_exact_product(
        left: Decimal,
        right: Decimal,
        product: Decimal,
    ) -> Result<(), RateError> {
        if left.is_zero() || right.is_zero() {
            return Ok(());
        }
        if product.scale() != left.scale() + right.scale() {
            return Err(RateError::InexactProduct { left, right });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    use crate::{
        QuantityError,
        test_support::{qty, rate, registry, usd_jpy},
    };

    // The rounding mode.

    /// Pins the strategy against the crate default, so a `rust_decimal` upgrade
    /// cannot move it silently.
    #[test]
    fn test_the_rounding_strategy_is_applied_not_inherited() {
        let midpoint = Decimal::new(5, 3); // 0.005
        assert_eq!(midpoint.round_dp_with_strategy(2, Rate::ROUNDING_STRATEGY), Decimal::new(1, 2));
        // The crate default disagrees, so the strategy cannot be coming from it.
        assert_eq!(midpoint.round_dp(2), Decimal::ZERO);
    }

    #[test]
    fn test_apply_amount_rounds_midpoints_away_from_zero() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let rate = rate(5, 3, usd, jpy); // 0.005 USD/JPY

        // +1 JPY -> +0.005 -> 0.01, and -1 JPY -> -0.005 -> -0.01. Half-even
        // would give 0.00 for both, so this pins the mode through the real API.
        let (value, residue) = rate.apply_amount(qty(Decimal::ONE, "JPY", &registry), usd).unwrap();
        assert_eq!(value.number(), Decimal::new(1, 2));
        assert_eq!(residue, Decimal::new(-5, 3));

        let (value, residue) =
            rate.apply_amount(qty(-Decimal::ONE, "JPY", &registry), usd).unwrap();
        assert_eq!(value.number(), Decimal::new(-1, 2));
        assert_eq!(residue, Decimal::new(5, 3));
    }

    // Deriving a value.

    #[test]
    fn test_apply_amount_rounds_to_the_quote_scale() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let rate = rate(7_139, 3, usd, jpy); // 7.139 USD/JPY

        let (value, residue) =
            rate.apply_amount(qty(Decimal::new(4, 0), "JPY", &registry), usd).unwrap();
        assert_eq!(value.number(), Decimal::new(2_856, 2)); // 28.556 -> 28.56 USD
        // Rounding up records value the product did not contain, so the residue
        // is negative.
        assert_eq!(residue, Decimal::new(-4, 3));
    }

    #[test]
    fn test_apply_amount_reports_a_zero_residue_when_the_product_fits() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let rate = rate(150, 2, usd, jpy); // 1.50 USD/JPY

        let (value, residue) =
            rate.apply_amount(qty(Decimal::new(2, 0), "JPY", &registry), usd).unwrap();
        assert_eq!(value.number(), Decimal::new(300, 2)); // 3.00 USD
        assert_eq!(residue, Decimal::ZERO);
    }

    /// An identity rate can never discard anything: the amount is already at
    /// the commodity's scale, and multiplying by one does not move it.
    #[test]
    fn test_identity_rate_discards_nothing() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let rate = Rate::Identity(usd.id());

        let (value, residue) =
            rate.apply_amount(qty(Decimal::new(1_500, 2), "USD", &registry), usd).unwrap();
        assert_eq!(value.number(), Decimal::new(1_500, 2));
        assert_eq!(residue, Decimal::ZERO);
    }

    // Precision guards.

    /// A product needing more than 28 places loses digits silently, so it has
    /// to be refused rather than rounded.
    #[test]
    fn test_apply_amount_detects_an_inexact_product() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let hyp = registry.get_by_code("HYP").expect("the fixture registry holds HYP");
        // 28 significant decimals against a scale-1 rate needs 29 places.
        let amount = qty(
            Decimal::from_str_exact("0.1234567890123456789012345678").unwrap(),
            "HYP",
            &registry,
        );

        let err = rate(5, 1, usd, hyp).apply_amount(amount, usd).unwrap_err();
        assert!(matches!(err, RateError::InexactProduct { .. }), "got {err}");
    }

    /// Trailing zeros must not be mistaken for precision. `4.000…0 * 1.50` is
    /// exactly 6, even though the raw operand scales sum to 30, so normalizing
    /// the operands first is what keeps a scale-28 commodity usable at all.
    #[test]
    fn test_apply_amount_accepts_an_exact_product_padded_with_trailing_zeros() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let hyp = registry.get_by_code("HYP").expect("the fixture registry holds HYP");
        let amount = qty(Decimal::new(4, 0), "HYP", &registry); // 4.000…0, scale 28

        let (value, residue) = rate(150, 2, usd, hyp).apply_amount(amount, usd).unwrap();
        assert_eq!(value.number(), Decimal::new(600, 2)); // 6.00 USD
        assert!(residue.is_zero());
    }

    /// A zero operand is exempt from the precision guard: multiplication
    /// short-circuits without computing anything, and a zero rate is valid.
    /// Without the exemption, a high-scale commodity could not take one.
    #[test]
    fn test_apply_amount_exempts_a_zero_operand() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let hyp = registry.get_by_code("HYP").expect("the fixture registry holds HYP");

        let rate_one_five = rate(150, 2, usd, hyp);
        let (value, residue) =
            rate_one_five.apply_amount(qty(Decimal::ZERO, "HYP", &registry), usd).unwrap();
        assert_eq!(value.number(), Decimal::ZERO);
        assert_eq!(residue, Decimal::ZERO);

        // 28 *significant* decimals, so normalizing cannot rescue this one: only
        // the zero-operand exemption lets it through.
        let amount = qty(
            Decimal::from_str_exact("0.1234567890123456789012345678").unwrap(),
            "HYP",
            &registry,
        );
        let (value, residue) = rate(0, 0, usd, hyp).apply_amount(amount, usd).unwrap();
        assert_eq!(value.number(), Decimal::ZERO);
        assert_eq!(residue, Decimal::ZERO);
    }

    #[test]
    fn test_apply_amount_detects_overflow() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        // `Decimal::MAX` needs a scale-0 commodity: it spends all its digits
        // before the point, so it could not be rescaled to USD's two places.
        let amount = qty(Decimal::MAX, "JPY", &registry);

        let err = rate(2, 0, usd, jpy).apply_amount(amount, usd).unwrap_err();
        assert!(
            matches!(err, RateError::Overflow { left, right }
                if left == Decimal::MAX && right == Decimal::new(2, 0)),
            "got {err}"
        );
    }

    // Deriving an amount.

    #[test]
    fn test_apply_value_rounds_to_the_base_scale() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let rate = rate(8_000, 4, usd, jpy); // 0.8000 USD/JPY

        let amount = rate.apply_value(qty(Decimal::new(50_000, 2), "USD", &registry), jpy).unwrap();
        assert_eq!(amount.number(), Decimal::new(625, 0)); // 625 JPY
        assert_eq!(amount.number().scale(), 0);
    }

    /// A product can be exact, small, and still not fit: at scale 28 a
    /// commodity has no digits left for anything above roughly 7.9, so the
    /// failure comes from `Quantity` rather than from the multiplication.
    #[test]
    fn test_apply_amount_rejects_a_product_the_quote_cannot_hold() {
        let registry = registry();
        let (_usd, jpy) = usd_jpy(&registry);
        let hyp = registry.get_by_code("HYP").expect("the fixture registry holds HYP");

        let amount = qty(Decimal::new(3, 0), "JPY", &registry);
        let err = rate(4, 0, hyp, jpy).apply_amount(amount, hyp).unwrap_err();
        assert!(
            matches!(err, RateError::Quantity(QuantityError::NumberTooLarge { scale: 28, .. })),
            "got {err}"
        );
    }

    /// Division by zero has no answer, so a zero rate cannot be inverted.
    #[test]
    fn test_apply_value_rejects_a_zero_rate() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let rate = rate(0, 0, usd, jpy);

        let err =
            rate.apply_value(qty(Decimal::new(50_000, 2), "USD", &registry), jpy).unwrap_err();
        assert!(matches!(err, RateError::Overflow { .. }), "got {err}");
    }

    // proptests
    //
    // The amount and rate mantissas are bounded so that every generated product
    // stays inside `Decimal`'s range. An unbounded `i64` mantissa against an
    // unbounded rate overflows the multiplication, which leaves a property that
    // skips failures asserting nothing at all. Overflow and the
    // precision guard are covered by the unit tests above.
    proptest! {
        /// `ROUNDING_STRATEGY` is an odd function, which is what lets a reversal
        /// cancel its original exactly.
        #[test]
        fn prop_the_rounding_strategy_is_odd(mantissa: i64, scale in 0u32..=10, dp in 0u32..=8) {
            let x = Decimal::new(mantissa, scale);
            let pos = x.round_dp_with_strategy(dp, Rate::ROUNDING_STRATEGY);
            let neg = (-x).round_dp_with_strategy(dp, Rate::ROUNDING_STRATEGY);
            prop_assert_eq!(neg, -pos);
        }

        /// The residue accounts for everything the scale could not hold, in both
        /// directions.
        #[test]
        fn prop_value_plus_residue_is_the_exact_product(
            mantissa in -1_000_000_000_000i64..=1_000_000_000_000,
            rate_mantissa in -1_000_000_000i64..=1_000_000_000,
        ) {
            let registry = registry();
            let (usd, jpy) = usd_jpy(&registry);
            // JPY is scale 0 and the rate scale 4, so the product is always exact.
            let amount = qty(Decimal::new(mantissa, 0), "JPY", &registry);
            let rate = rate(rate_mantissa, 4, usd, jpy);

            let (value, residue) =
                rate.apply_amount(amount, usd).expect("the ranges keep the product in range");
            prop_assert_eq!(value.number() + residue, amount.number() * rate.number());
        }

        /// A derived value always lands at exactly the quote commodity's scale,
        /// so no caller has to resolve the commodity again to know it.
        #[test]
        fn prop_a_derived_value_is_always_at_the_quote_scale(
            mantissa in -1_000_000_000_000i64..=1_000_000_000_000,
            rate_mantissa in -1_000_000_000i64..=1_000_000_000,
        ) {
            let registry = registry();
            let (usd, jpy) = usd_jpy(&registry);
            let amount = qty(Decimal::new(mantissa, 0), "JPY", &registry);
            let rate = rate(rate_mantissa, 4, usd, jpy);

            let (value, _residue) =
                rate.apply_amount(amount, usd).expect("the ranges keep the product in range");
            prop_assert_eq!(value.number().scale(), u32::from(usd.scale()));
        }

        /// A derived amount lands at exactly the base commodity's scale too,
        /// even though the rounding itself does not pad: `round_dp` leaves a
        /// short scale alone, and `Quantity` is what fixes it.
        #[test]
        fn prop_a_derived_amount_is_always_at_the_base_scale(
            value_mantissa in -1_000_000_000_000i64..=1_000_000_000_000,
            // Non-zero, since a zero rate cannot be inverted, and bounded below
            // so that dividing by it cannot overflow.
            rate_mantissa in 1i64..=1_000_000_000,
        ) {
            let registry = registry();
            let usd = registry.get_by_code("USD").expect("the fixture registry holds USD");
            let btc = registry.get_by_code("BTC").expect("the fixture registry holds BTC");
            let value = qty(Decimal::new(value_mantissa, 2), "USD", &registry);
            let rate = rate(rate_mantissa, 4, usd, btc);

            let amount =
                rate.apply_value(value, btc).expect("the ranges keep the quotient in range");
            prop_assert_eq!(amount.commodity(), btc.id());
            prop_assert_eq!(amount.number().scale(), u32::from(btc.scale()));
        }

        /// The residue is never large enough to be representable at the quote's
        /// scale: if it were, it belonged in the value.
        #[test]
        fn prop_the_residue_is_always_below_the_quote_scale(
            mantissa in -1_000_000_000_000i64..=1_000_000_000_000,
            rate_mantissa in -1_000_000_000i64..=1_000_000_000,
        ) {
            let registry = registry();
            let (usd, jpy) = usd_jpy(&registry);
            let amount = qty(Decimal::new(mantissa, 0), "JPY", &registry);
            let rate = rate(rate_mantissa, 4, usd, jpy);

            let (_value, residue) =
                rate.apply_amount(amount, usd).expect("the ranges keep the product in range");
            // Half a cent is the most rounding to the nearest cent can move.
            prop_assert!(residue.abs() <= Decimal::new(5, 3), "residue {residue} is too large");
        }
    }
}
