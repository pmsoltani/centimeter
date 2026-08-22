//! Postings are the individual lines that make up a transaction.
//!
//! Each posting connects a quantity to an account and represents the triple
//! `amount * rate = value`. The `amount` is expressed in the posting's
//! commodity, while `value` is always expressed in the ledger's functional
//! commodity. The `rate` describes the conversion between the two. Debits are
//! positive and credits are negative, so a transaction balances when the values
//! of all its postings sum to zero.
//!
//! A caller may provide any supported combination of these fields; the engine
//! derives the missing values. [`PostingValuation`] defines which combinations
//! are valid. Once calculated, the `(amount, value)` pair becomes the posting's
//! source of truth. Any rate reported later describes how that value was
//! obtained rather than being used to recalculate or validate it.
//!
//! Postings are immutable, so the only way to correct them is by creating a new
//! posting rather than an edit. Whether a replacement posting can be made at
//! all is the enclosing transaction's decision: a draft transaction allows it,
//! while a posted transaction is immutable.

mod error;
mod valuation;

use rust_decimal::Decimal;

use crate::{
    AccountId, CommodityId, CommodityRegistry, Id, IdPrefix, Identifiable, Quantity, Rate,
};
pub use error::PostingError;
pub use valuation::PostingValuation;
use valuation::Resolved;

/// A single line in a transaction: a signed amount in an account, with a value
/// in the ledger's functional commodity.
///
/// The `(amount, value)` pair is fixed when the posting is created. The
/// `stated_rate` and the rate returned by `derived_rate` are kept only as
/// provenance; they are never used to recompute either amount or value. This
/// means a stored posting remains unchanged even if the rate calculation logic
/// changes later.
///
/// A posting is immutable once built. It exposes no way to change any member
/// and has only the constructor [`try_new`](Self::try_new), so correcting one
/// means building a replacement rather than editing this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    id: PostingId,
    account: AccountId,
    amount: Quantity,
    stated_rate: Option<Rate>,
    value: Quantity,
    value_residue: Option<Decimal>,
}

impl Identifiable for Posting {
    const PREFIX: IdPrefix = IdPrefix::new("pst");
}

/// The id of a [`Posting`], rendered as `pst_<suffix>`.
pub type PostingId = Id<Posting>;

impl Posting {
    /// Builds a posting, calculating any part of the `amount * rate = value`
    /// triple that the caller did not provide.
    ///
    /// `functional` is the ledger's functional commodity. Every posting's value
    /// is denominated in it, which lets balancing reduce to a scalar sum.
    /// `registry` provides the scale used when deriving a missing quantity, so
    /// both the functional commodity and the rate's base commodity must be
    /// registered.
    ///
    /// Core never generates posting IDs; id is supplied by the caller.
    ///
    /// # Examples
    ///
    /// ```
    /// # use centimeter_core::{
    /// #     AccountId, CommodityId, CommodityRegistry, Decimal, Posting, PostingId,
    /// #     PostingValuation, Quantity, Rate,
    /// # };
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut registry = CommodityRegistry::new();
    /// let usd_id = CommodityId::from_uuid(uuid::Uuid::now_v7())?;
    /// let jpy_id = CommodityId::from_uuid(uuid::Uuid::now_v7())?;
    /// registry.add(usd_id, "USD", "US Dollar", 2)?;
    /// registry.add(jpy_id, "JPY", "Japanese Yen", 0)?;
    ///
    /// let jpy = registry.get(jpy_id).expect("registered");
    /// // 10,000 JPY booked in a USD ledger at 0.0065437 USD/JPY.
    /// let amount = Quantity::try_new(Decimal::new(10_000, 0), jpy)?;
    /// let stated_rate = Rate::try_new(Decimal::new(65_437, 7), usd_id, jpy_id)?;
    ///
    /// let posting = Posting::try_new(
    ///     PostingId::from_uuid(uuid::Uuid::now_v7())?,
    ///     PostingValuation::AmountAtRate { amount, stated_rate },
    ///     AccountId::from_uuid(uuid::Uuid::now_v7())?,
    ///     usd_id,
    ///     &registry,
    /// )?;
    ///
    /// // The exact product is 65.437, so USD stores 65.44 and reports the residue.
    /// assert_eq!(posting.value().number(), Decimal::new(6_544, 2));
    /// assert_eq!(posting.value_residue(), Some(Decimal::new(-3, 3)));
    ///
    /// // The amount is untouched, and the stored pair implies its own rate.
    /// assert_eq!(posting.amount().number(), Decimal::new(10_000, 0));
    /// assert_eq!(posting.derived_rate().expect("two commodities").number(), Decimal::new(6_544, 6));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// - [`FunctionalMismatch`](PostingError::FunctionalMismatch) if a member
    ///   that must be denominated in the functional commodity is not.
    /// - [`BaseMismatch`](PostingError::BaseMismatch) if the amount is not
    ///   denominated in the rate's base commodity.
    /// - [`QuoteMismatch`](PostingError::QuoteMismatch) if the value is not
    ///   denominated in the rate's quote commodity.
    /// - [`AmountValueMismatch`](PostingError::AmountValueMismatch) if amount
    ///   and value use the same commodity but differ, implying a conversion
    ///   from a commodity to itself at a rate other than one.
    /// - [`ZeroRateWithValue`](PostingError::ZeroRateWithValue) if a value is
    ///   given with a zero rate, which cannot be inverted to derive an amount.
    /// - [`UnknownCommodity`](PostingError::UnknownCommodity) if `functional`
    ///   or the rate's base commodity is not registered in registry.
    /// - [`Rate`](PostingError::Rate) if applying the rate overflows, loses
    ///   precision, or the result does not fit its commodity's scale.
    pub fn try_new(
        id: PostingId,
        valuation: PostingValuation,
        account: AccountId,
        functional: CommodityId,
        registry: &CommodityRegistry,
    ) -> Result<Self, PostingError> {
        let functional =
            registry.get(functional).ok_or(PostingError::UnknownCommodity { id: functional })?;
        let Resolved { amount, stated_rate, value, value_residue } =
            valuation.resolve(functional, registry)?;
        Ok(Self { id, account, amount, stated_rate, value, value_residue })
    }

    /// The id of this posting.
    #[must_use]
    pub fn id(&self) -> PostingId {
        self.id
    }

    /// The account this posting is for.
    #[must_use]
    pub fn account(&self) -> AccountId {
        self.account
    }

    /// The amount of the posting.
    #[must_use]
    pub fn amount(&self) -> Quantity {
        self.amount
    }

    /// The stated rate of the posting.
    #[must_use]
    pub fn stated_rate(&self) -> Option<Rate> {
        self.stated_rate
    }

    /// The value of the posting.
    #[must_use]
    pub fn value(&self) -> Quantity {
        self.value
    }

    /// The signed part of the exact product that was too small for the
    /// functional commodity's scale: `value + value_residue == amount * rate`.
    ///
    /// The residue is signed rather than discarded. For example, rounding
    /// `28.556` to `28.56` creates `-0.004`, while rounding `28.553` to `28.55`
    /// leaves `+0.003`. In other words, the sign indicates whether the stored
    /// value is above or below the exact product.
    ///
    /// Returns `None` when the value was not derived and there is no residue to
    /// report. Only [`PostingValuation::AmountAtRate`] rounds on the functional
    /// side. A derived amount, as happens in [`PostingValuation::ValueAtRate`],
    /// does not report a residue because that rounding occurs in the posting's
    /// own commodity; the functional side is what matters for the ledger's
    /// zero-sum invariant.
    ///
    /// `Some(0)` is distinct from `None`: it means the product fit the
    /// functional commodity's scale exactly.
    #[must_use]
    pub fn value_residue(&self) -> Option<Decimal> {
        self.value_residue
    }

    /// Returns the rate implied by the stored `(amount, value)` pair.
    ///
    /// This is different from [`stated_rate`](Self::stated_rate) when the value
    /// was rounded to the functional commodity's scale. For example, a posting
    /// stated at `0.85673 GBP/EUR` might store `(100.00 EUR, 85.67 GBP)`, which
    /// implies a rate of `0.8567`. The stored `(amount, value)` pair is all
    /// that matters; both rates describe how that pair was obtained rather than
    /// acting as validation checks.
    ///
    /// Returns `None` when:
    /// - the amount is zero and the commodities differ, so rate cannot be
    ///   inferred. If the amount and value use the same commodity, the rate is
    ///   `Identity` by definition.
    /// - the quotient is too large for a `Decimal`.
    #[must_use]
    pub fn derived_rate(&self) -> Option<Rate> {
        let (quote, base) = (self.value.commodity(), self.amount.commodity());
        if quote == base {
            return Some(Rate::Identity(quote));
        }
        let number = self.value.number().checked_div(self.amount.number())?;
        Rate::try_new(number, quote, base).ok()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    use crate::test_support::{id, qty, rate, registry, usd_jpy};

    /// Builds a posting from `valuation`, balancing in `USD`.
    ///
    /// The ids stand in for the caller, since core mints none: 20 for the
    /// posting, 10 for the account it lands in.
    fn posting(
        valuation: PostingValuation,
        registry: &CommodityRegistry,
    ) -> Result<Posting, PostingError> {
        let (usd, _) = usd_jpy(registry);
        Posting::try_new(id(20), valuation, id(10), usd.id(), registry)
    }

    #[test]
    fn test_accessors_return_what_was_built() {
        let registry = registry();
        let quantity = qty(Decimal::new(1_500, 2), "USD", &registry);
        let built = posting(PostingValuation::Functional(quantity), &registry).unwrap();

        assert_eq!(built.id(), id(20));
        assert_eq!(built.account(), id(10));
        assert_eq!(built.amount(), quantity);
        assert_eq!(built.value(), quantity);
        assert_eq!(built.stated_rate(), None);
        assert_eq!(built.value_residue(), None);
    }

    /// The functional commodity is resolved before anything else, so an
    /// unregistered one is refused whatever the valuation says.
    #[test]
    fn test_rejects_an_unregistered_functional_commodity() {
        let registry = registry();
        let unregistered = id(99);
        let valuation = PostingValuation::Functional(qty(Decimal::new(1_500, 2), "USD", &registry));

        let err = Posting::try_new(id(20), valuation, id(10), unregistered, &registry).unwrap_err();
        assert!(
            matches!(err, PostingError::UnknownCommodity { id } if id == unregistered),
            "got {err}"
        );
    }

    // The two rates, and how they differ.

    #[test]
    fn test_derived_rate_and_stated_rate_legitimately_disagree() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        // 100 JPY at 0.85673 USD/JPY is 85.673, which USD stores as 85.67.
        let stated = rate(85_673, 5, usd, jpy);
        let valuation = PostingValuation::AmountAtRate {
            amount: qty(Decimal::new(100, 0), "JPY", &registry),
            stated_rate: stated,
        };
        let built = posting(valuation, &registry).unwrap();

        // The stored pair implies 85.67 / 100 = 0.8567, not what was stated.
        let derived = built.derived_rate().unwrap();
        assert_eq!(derived.number(), Decimal::new(8_567, 4));
        assert_ne!(derived.number(), stated.number());
        assert_eq!(derived.quote(), usd.id());
        assert_eq!(derived.base(), jpy.id());
    }

    #[test]
    fn test_derived_rate_is_identity_for_a_single_commodity_posting() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let valuation = PostingValuation::Functional(qty(Decimal::new(1_500, 2), "USD", &registry));

        let built = posting(valuation, &registry).unwrap();
        assert_eq!(built.derived_rate(), Some(Rate::Identity(usd.id())));
    }

    /// One commodity converts to itself at one even when there is nothing to
    /// divide, so a zero posting still reports `Identity` rather than `None`.
    #[test]
    fn test_derived_rate_is_identity_for_a_zero_functional_posting() {
        let registry = registry();
        let (usd, _) = usd_jpy(&registry);
        let valuation = PostingValuation::Functional(qty(Decimal::ZERO, "USD", &registry));

        let built = posting(valuation, &registry).unwrap();
        assert_eq!(built.derived_rate(), Some(Rate::Identity(usd.id())));
    }

    /// Across two commodities a zero amount leaves `0 / 0`, which no rate can
    /// describe.
    #[test]
    fn test_derived_rate_is_none_when_a_zero_amount_spans_commodities() {
        let registry = registry();
        let (usd, jpy) = usd_jpy(&registry);
        let valuation = PostingValuation::AmountAtRate {
            amount: qty(Decimal::ZERO, "JPY", &registry),
            stated_rate: rate(85_673, 5, usd, jpy),
        };

        let built = posting(valuation, &registry).unwrap();
        assert!(built.value().is_zero());
        assert!(built.derived_rate().is_none());
    }

    // proptests

    proptest! {
        /// Negating a posting's amount negates its value and its residue, so a
        /// reversal cancels its original to the penny. This is the property the
        /// whole choice of rounding mode rests on.
        #[test]
        fn prop_negating_the_amount_negates_the_value_and_the_residue(
            mantissa in -1_000_000_000_000i64..=1_000_000_000_000,
            rate_mantissa in -1_000_000_000i64..=1_000_000_000,
        ) {
            let registry = registry();
            let (usd, jpy) = usd_jpy(&registry);
            let stated_rate = rate(rate_mantissa, 4, usd, jpy);
            let build = |m: i64| posting(
                PostingValuation::AmountAtRate {
                    amount: qty(Decimal::new(m, 0), "JPY", &registry),
                    stated_rate,
                },
                &registry,
            );

            let expect = "the ranges keep the product in range";
            let original = build(mantissa).expect(expect);
            let reversal = build(-mantissa).expect(expect);

            prop_assert_eq!(reversal.value(), -original.value());
            prop_assert_eq!(reversal.value_residue().unwrap(), -original.value_residue().unwrap());
            prop_assert!(original.value().checked_add(reversal.value()).unwrap().is_zero());
        }

        /// Given both sides, nothing is derived and so nothing is rounded: the
        /// pair is stored exactly as supplied.
        #[test]
        fn prop_amount_and_value_round_trips_its_inputs(
            amount_mantissa: i64,
            value_mantissa: i64,
        ) {
            let registry = registry();
            let amount = qty(Decimal::new(amount_mantissa, 0), "JPY", &registry);
            let value = qty(Decimal::new(value_mantissa, 2), "USD", &registry);

            let built =
                posting(PostingValuation::AmountAndValue { amount, value }, &registry).unwrap();
            prop_assert_eq!(built.amount(), amount);
            prop_assert_eq!(built.value(), value);
            prop_assert_eq!(built.stated_rate(), None);
            prop_assert_eq!(built.value_residue(), None);
        }

        /// A derived value always lands at exactly the functional commodity's
        /// scale, whatever the amount and rate were spelled at.
        #[test]
        fn prop_a_derived_value_is_always_at_the_functional_scale(
            mantissa in -1_000_000_000_000i64..=1_000_000_000_000,
            rate_mantissa in -1_000_000_000i64..=1_000_000_000,
        ) {
            let registry = registry();
            let (usd, jpy) = usd_jpy(&registry);
            let valuation = PostingValuation::AmountAtRate {
                amount: qty(Decimal::new(mantissa, 0), "JPY", &registry),
                stated_rate: rate(rate_mantissa, 4, usd, jpy),
            };

            let built =
                posting(valuation, &registry).expect("the ranges keep the product in range");
            prop_assert_eq!(built.value().commodity(), usd.id());
            prop_assert_eq!(built.value().number().scale(), u32::from(usd.scale()));
        }
    }
}
