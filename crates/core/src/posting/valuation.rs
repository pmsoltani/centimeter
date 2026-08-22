//! What a caller provides for a posting's `amount * rate = value` triple.
//!
//! A caller usually has only two of these values, and the engine can derive
//! the third from any sufficient combination. [`PostingValuation`] represents
//! those valid combinations. This also means invalid or ambiguous inputs can't
//! be represented:
//! - A lone quantity that is not denominated in the functional commodity will
//!   not be allowed, since there is no rate to infer.
//! - All three values together aren't accepted either, since the rate in that
//!   case would only serve as a check against `round(amount * rate)`, which is
//!   unreliable due to limited precision.

use rust_decimal::Decimal;

use crate::{Commodity, CommodityId, CommodityRegistry, PostingError, Quantity, Rate};

/// The complete triple, after [`PostingValuation::resolve`] has filled in
/// whatever the caller left out.
#[derive(Debug)]
pub(super) struct Resolved {
    /// The amount, in the posting's own commodity.
    pub amount: Quantity,
    /// The caller's supplied rate, if any.
    pub stated_rate: Option<Rate>,
    /// The balancing member, always in the functional commodity.
    pub value: Quantity,
    /// What the functional commodity's scale could not hold when `value` was
    /// derived, or `None` when it was not derived. See
    /// [`Posting::value_residue`](super::Posting::value_residue).
    pub value_residue: Option<Decimal>,
}

/// What the caller supplies for a posting's `amount * rate = value` triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostingValuation {
    /// No rate is stated; `amount == value` and the implied rate is `Identity`.
    Functional(Quantity),
    /// Both sides given with no stated rate; one can be derived from the pair.
    AmountAndValue {
        /// The amount of the posting.
        amount: Quantity,
        /// The value of the posting.
        value: Quantity,
    },
    /// The `value` is derived from `round(amount * rate)`.
    ///
    /// The only variant that reports a residue, because it is the only one that
    /// rounds on the functional side.
    AmountAtRate {
        /// The amount of the posting.
        amount: Quantity,
        /// The rate to apply to the amount to derive the value.
        stated_rate: Rate,
    },
    /// The `amount` is derived from `round(value / rate)`.
    ValueAtRate {
        /// The value of the posting.
        value: Quantity,
        /// The rate to apply to the value to derive the amount.
        stated_rate: Rate,
    },
}

impl PostingValuation {
    /// Fills in whichever members of the triple the caller did not supply.
    ///
    /// # Errors
    /// See [`Posting::try_new`](super::Posting::try_new), which surfaces these
    /// errors unchanged.
    pub(super) fn resolve(
        self,
        functional: &Commodity,
        registry: &CommodityRegistry,
    ) -> Result<Resolved, PostingError> {
        match self {
            PostingValuation::Functional(qty) => {
                Self::ensure_functional(qty.commodity(), functional.id())?;
                let (stated_rate, value_residue) = (None, None);
                Ok(Resolved { amount: qty, stated_rate, value: qty, value_residue })
            }
            PostingValuation::AmountAndValue { amount, value } => {
                Self::ensure_functional(value.commodity(), functional.id())?;
                Self::ensure_no_self_conversion(amount, value)?;
                let (stated_rate, value_residue) = (None, None);
                Ok(Resolved { amount, stated_rate, value, value_residue })
            }
            PostingValuation::AmountAtRate { amount, stated_rate } => {
                Self::ensure_functional(stated_rate.quote(), functional.id())?;
                Self::ensure_base(amount, stated_rate)?;
                let (value, residue) = stated_rate.apply_amount(amount, functional)?;
                let (stated_rate, value_residue) = (Some(stated_rate), Some(residue));
                Ok(Resolved { amount, stated_rate, value, value_residue })
            }
            PostingValuation::ValueAtRate { value, stated_rate } => {
                Self::ensure_functional(stated_rate.quote(), functional.id())?;
                Self::ensure_quote(value, stated_rate)?;
                if stated_rate.number().is_zero() {
                    return Err(PostingError::ZeroRateWithValue);
                }
                let base = registry
                    .get(stated_rate.base())
                    .ok_or(PostingError::UnknownCommodity { id: stated_rate.base() })?;
                // No residue will be reported here.
                let amount = stated_rate.apply_value(value, base)?;
                let (stated_rate, value_residue) = (Some(stated_rate), None);
                Ok(Resolved { amount, stated_rate, value, value_residue })
            }
        }
    }

    fn ensure_functional(got: CommodityId, functional: CommodityId) -> Result<(), PostingError> {
        if got != functional {
            return Err(PostingError::FunctionalMismatch { got, expected: functional });
        }
        Ok(())
    }

    fn ensure_base(amount: Quantity, rate: Rate) -> Result<(), PostingError> {
        let (got, expected) = (amount.commodity(), rate.base());
        if got != expected {
            return Err(PostingError::BaseMismatch { got, expected });
        }
        Ok(())
    }

    fn ensure_quote(value: Quantity, rate: Rate) -> Result<(), PostingError> {
        let (got, expected) = (value.commodity(), rate.quote());
        if got != expected {
            return Err(PostingError::QuoteMismatch { got, expected });
        }
        Ok(())
    }

    fn ensure_no_self_conversion(amount: Quantity, value: Quantity) -> Result<(), PostingError> {
        let (got, expected) = (amount.number(), value.number());
        if amount.commodity() == value.commodity() && got != expected {
            return Err(PostingError::AmountValueMismatch { amount: got, value: expected });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        RateError,
        test_support::{id, qty, rate, registry, usd_jpy},
    };

    // Derivation: one test per variant.

    #[test]
    fn test_functional_mirrors_the_quantity_to_both_sides() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let quantity = qty(Decimal::new(1_500, 2), "USD", &registry);

        let resolved = PostingValuation::Functional(quantity).resolve(usd, &registry).unwrap();
        assert_eq!(resolved.amount, quantity);
        assert_eq!(resolved.value, quantity);
        // No rate was stated; `Posting::derived_rate` is where `Identity` shows up.
        assert_eq!(resolved.stated_rate, None);
        assert_eq!(resolved.value_residue, None);
    }

    #[test]
    fn test_amount_and_value_stores_both_sides_untouched() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        // A card statement showing both sides: 120 JPY charged, 94.50 USD billed.
        let valuation = PostingValuation::AmountAndValue {
            amount: qty(Decimal::new(-120, 0), "JPY", &registry),
            value: qty(Decimal::new(-9_450, 2), "USD", &registry),
        };

        let resolved = valuation.resolve(usd, &registry).unwrap();
        assert_eq!(resolved.amount.number(), Decimal::new(-120, 0));
        assert_eq!(resolved.amount.commodity(), jpy.id());
        assert_eq!(resolved.value.number(), Decimal::new(-9_450, 2));
        assert_eq!(resolved.value.commodity(), usd.id());
        assert_eq!(resolved.stated_rate, None);
        // Nothing was derived, so nothing was rounded.
        assert_eq!(resolved.value_residue, None);
    }

    #[test]
    fn test_amount_at_rate_derives_the_value() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        // 3 JPY at 75.144 USD/JPY is 225.432, which USD cannot hold.
        let stated_rate = rate(75_144, 3, usd, jpy);
        let valuation = PostingValuation::AmountAtRate {
            amount: qty(Decimal::new(3, 0), "JPY", &registry),
            stated_rate,
        };

        let resolved = valuation.resolve(usd, &registry).unwrap();
        assert_eq!(resolved.amount.number(), Decimal::new(3, 0));
        assert_eq!(resolved.amount.commodity(), jpy.id());
        assert_eq!(resolved.value.number(), Decimal::new(22_543, 2));
        assert_eq!(resolved.value.commodity(), usd.id());
        assert_eq!(resolved.stated_rate, Some(stated_rate));
        assert_eq!(resolved.value_residue, Some(Decimal::new(2, 3)));
    }

    /// The derived amount belongs to the rate's base commodity, not the
    /// functional one. Deriving it into the functional commodity was a real bug.
    #[test]
    fn test_value_at_rate_derives_the_amount_in_the_base_commodity() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        // A pinned functional budget: 500.00 USD at 0.80 USD/JPY is 625 JPY.
        let stated_rate = rate(80, 2, usd, jpy);
        let valuation = PostingValuation::ValueAtRate {
            value: qty(Decimal::new(50_000, 2), "USD", &registry),
            stated_rate,
        };

        let resolved = valuation.resolve(usd, &registry).unwrap();
        assert_eq!(resolved.amount.number(), Decimal::new(625, 0));
        assert_eq!(resolved.amount.commodity(), jpy.id());
        assert_eq!(resolved.stated_rate, Some(stated_rate));
        assert_eq!(resolved.value_residue, None);
    }

    /// A same-commodity pair is legal as long as it implies no conversion.
    #[test]
    fn test_amount_and_value_accepts_an_equal_same_commodity_pair() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let quantity = qty(Decimal::new(10_000, 2), "USD", &registry);
        let valuation = PostingValuation::AmountAndValue { amount: quantity, value: quantity };

        let resolved = valuation.resolve(usd, &registry).unwrap();
        assert_eq!(resolved.amount, quantity);
        assert_eq!(resolved.value, quantity);
    }

    /// A derived *amount* reports no residue even when the division is inexact:
    /// that rounding lands in the posting's own commodity, and only the
    /// functional side carries the zero-sum invariant.
    #[test]
    fn test_value_at_rate_reports_no_residue_even_when_inexact() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        // 100.00 / 0.8567 is 116.7269..., which rounds to 117 at JPY's scale.
        let valuation = PostingValuation::ValueAtRate {
            value: qty(Decimal::new(10_000, 2), "USD", &registry),
            stated_rate: rate(8_567, 4, usd, jpy),
        };

        let resolved = valuation.resolve(usd, &registry).unwrap();
        assert_eq!(resolved.amount.number(), Decimal::new(117, 0));
        assert_eq!(resolved.amount.commodity(), jpy.id());
        assert_eq!(resolved.value_residue, None);
    }

    // Rejections: one test per error variant reachable from `resolve`.

    #[test]
    fn test_functional_rejects_a_non_functional_quantity() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let valuation = PostingValuation::Functional(qty(Decimal::new(100, 2), "JPY", &registry));

        let err = valuation.resolve(usd, &registry).unwrap_err();
        assert!(
            matches!(err, PostingError::FunctionalMismatch { got, expected }
                if got == jpy.id() && expected == usd.id()),
            "got {err}"
        );
    }

    #[test]
    fn test_amount_and_value_rejects_a_non_functional_value() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let valuation = PostingValuation::AmountAndValue {
            amount: qty(Decimal::new(100, 0), "JPY", &registry),
            value: qty(Decimal::new(100, 0), "JPY", &registry),
        };

        let err = valuation.resolve(usd, &registry).unwrap_err();
        assert!(
            matches!(err, PostingError::FunctionalMismatch { got, expected }
                if got == jpy.id() && expected == usd.id()),
            "got {err}"
        );
    }

    #[test]
    fn test_amount_at_rate_rejects_a_non_functional_quote() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        // The rate quotes USD, but the ledger balances in JPY here.
        let valuation = PostingValuation::AmountAtRate {
            amount: qty(Decimal::new(100, 0), "JPY", &registry),
            stated_rate: rate(80, 2, usd, jpy),
        };

        let err = valuation.resolve(jpy, &registry).unwrap_err();
        assert!(
            matches!(err, PostingError::FunctionalMismatch { got, expected }
                if got == usd.id() && expected == jpy.id()),
            "got {err}"
        );
    }

    #[test]
    fn test_amount_at_rate_rejects_an_amount_off_the_rate_base() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let btc = registry.get_by_code("BTC").expect("the fixture registry holds BTC");
        let valuation = PostingValuation::AmountAtRate {
            amount: qty(Decimal::new(1, 0), "BTC", &registry),
            stated_rate: rate(80, 2, usd, jpy),
        };

        let err = valuation.resolve(usd, &registry).unwrap_err();
        assert!(
            matches!(err, PostingError::BaseMismatch { got, expected }
                if got == btc.id() && expected == jpy.id()),
            "got {err}"
        );
    }

    #[test]
    fn test_value_at_rate_rejects_a_value_off_the_rate_quote() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let valuation = PostingValuation::ValueAtRate {
            value: qty(Decimal::new(50, 0), "JPY", &registry),
            stated_rate: rate(80, 2, usd, jpy),
        };

        let err = valuation.resolve(usd, &registry).unwrap_err();
        assert!(
            matches!(err, PostingError::QuoteMismatch { got, expected }
                if got == jpy.id() && expected == usd.id()),
            "got {err}"
        );
    }

    /// Amount and value in one commodity but at different numbers would convert
    /// that commodity to itself at something other than one.
    #[test]
    fn test_amount_and_value_rejects_a_self_conversion() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let valuation = PostingValuation::AmountAndValue {
            amount: qty(Decimal::new(10_000, 2), "USD", &registry),
            value: qty(Decimal::new(9_500, 2), "USD", &registry),
        };

        let err = valuation.resolve(usd, &registry).unwrap_err();
        assert!(
            matches!(err, PostingError::AmountValueMismatch { amount, value }
                if amount == Decimal::new(10_000, 2) && value == Decimal::new(9_500, 2)),
            "got {err}"
        );
    }

    /// A zero rate cannot be inverted, so it is refused before the division
    /// rather than surfacing as a `Rate(Overflow)`.
    #[test]
    fn test_value_at_rate_rejects_a_zero_rate() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let valuation = PostingValuation::ValueAtRate {
            value: qty(Decimal::new(10_000, 2), "USD", &registry),
            stated_rate: rate(0, 0, usd, jpy),
        };

        let err = valuation.resolve(usd, &registry).unwrap_err();
        assert!(matches!(err, PostingError::ZeroRateWithValue), "got {err}");
    }

    /// Only `ValueAtRate` looks the base commodity up, because it is the only
    /// variant that has to build a quantity in it. `AmountAtRate` compares the
    /// base against the amount's commodity instead, and a `Quantity` can only
    /// be built from a registered commodity, so there it reports
    /// `BaseMismatch`.
    #[test]
    fn test_value_at_rate_rejects_an_unregistered_base() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let unregistered = id(99);
        let stated_rate = Rate::try_new(Decimal::new(80, 2), usd.id(), unregistered).unwrap();
        let valuation = PostingValuation::ValueAtRate {
            value: qty(Decimal::new(10_000, 2), "USD", &registry),
            stated_rate,
        };

        let err = valuation.resolve(usd, &registry).unwrap_err();
        assert!(
            matches!(err, PostingError::UnknownCommodity { id } if id == unregistered),
            "got {err}"
        );
    }

    /// A rate failure reaches the caller as `PostingError::Rate` rather than
    /// being flattened into a posting-shaped error.
    #[test]
    fn test_amount_at_rate_propagates_a_rate_error() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let hyp = registry.get_by_code("HYP").expect("the fixture registry holds HYP");
        // HYP is scale 28, so 28 significant decimals against a scale-1 rate
        // needs 29 places and the product silently loses one.
        let amount = qty(
            Decimal::from_str_exact("0.1234567890123456789012345678").unwrap(),
            "HYP",
            &registry,
        );
        let valuation =
            PostingValuation::AmountAtRate { amount, stated_rate: rate(5, 1, usd, hyp) };

        let err = valuation.resolve(usd, &registry).unwrap_err();
        assert!(matches!(err, PostingError::Rate(RateError::InexactProduct { .. })), "got {err}");
    }
}
