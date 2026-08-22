//! Postings as a consumer meets them: a partial triple in, a complete one out.
//!
//! The two cases that matter here are ledger-level facts no single posting can
//! state on its own: that symmetric legs cancel exactly, and that a many-to-one
//! split does not.

use centimeter_core::{
    CommodityId, CommodityRegistry, Decimal, Error, Posting, PostingError, PostingId,
    PostingValuation, Quantity, Rate,
};

use crate::fixtures::{new_id, registry};

/// The shared registry plus `EUR` at scale 2, with the ids of both.
///
/// `JPY` cannot show any of this: at scale 0 it has no sub-unit for a residue to
/// live below, so the interesting cases need a second two-decimal commodity that
/// is not the one being balanced in.
fn registry_with_eur() -> (CommodityRegistry, CommodityId, CommodityId) {
    let (mut registry, usd, _jpy) = registry();
    let eur = new_id();
    registry.add(eur, "EUR", "Euro", 2).expect("EUR must register");
    (registry, usd, eur)
}

/// Values an `amount` of `EUR` at `rate` into the `USD` functional commodity.
fn leg(amount: Decimal, rate: Rate, registry: &CommodityRegistry, usd: CommodityId) -> Posting {
    let eur = registry.get_by_code("EUR").expect("EUR is registered");
    let amount = Quantity::try_new(amount, eur).expect("EUR holds two decimals");
    Posting::try_new(
        new_id(),
        PostingValuation::AmountAtRate { amount, stated_rate: rate },
        new_id(),
        usd,
        registry,
    )
    .expect("a EUR amount at a USD/EUR rate is valuable in USD")
}

/// Sums the values of every posting, the way a balance check would.
fn total_value(postings: &[Posting]) -> Quantity {
    postings
        .iter()
        .map(Posting::value)
        .reduce(|acc, value| acc.checked_add(value).expect("one commodity, no overflow"))
        .expect("at least one posting")
}

/// Sums the residues of every posting that reports one.
fn total_residue(postings: &[Posting]) -> Decimal {
    postings.iter().filter_map(Posting::value_residue).sum()
}

/// The ordinary two-leg transaction, and the reason it never needs a rounding
/// line: rounding is symmetric about zero, so a leg and its exact negation
/// discard exactly opposite residues.
#[test]
fn postings_of_a_two_leg_transaction_cancel_exactly() {
    let (registry, usd, eur) = registry_with_eur();
    // 33.33 EUR at 0.8567 USD/EUR is exactly 28.553811 USD, which USD cannot hold.
    let rate = Rate::try_new(Decimal::new(8_567, 4), usd, eur).expect("distinct commodities");

    let legs = [
        leg(Decimal::new(3_333, 2), rate, &registry, usd),
        leg(Decimal::new(-3_333, 2), rate, &registry, usd),
    ];

    assert_eq!(legs[0].value().number(), Decimal::new(2_855, 2));
    assert_eq!(legs[1].value().number(), Decimal::new(-2_855, 2));

    // Each leg drops the same amount, in opposite directions.
    assert_eq!(legs[0].value_residue(), Some(Decimal::new(3_811, 6)));
    assert_eq!(legs[1].value_residue(), Some(Decimal::new(-3_811, 6)));

    assert!(total_value(&legs).is_zero(), "the transaction must balance");
    assert!(total_residue(&legs).is_zero(), "the residues must cancel too");
}

/// A many-to-one split is where the residue becomes visible, because rounding
/// is odd but not additive: these amounts sum to zero exactly and their values
/// do not. Core refuses to invent a balancing leg, so the caller has to post
/// the penny itself; this test is the executable form of that requirement.
///
/// The figures are Microsoft's own worked example for Dynamics 365 F&O, with
/// `EUR` and `USD` standing in for its currencies.
#[test]
fn a_many_to_one_split_leaves_a_visible_residue() {
    let (registry, usd, eur) = registry_with_eur();
    let rate = Rate::try_new(Decimal::new(15, 1), usd, eur).expect("distinct commodities");

    let legs = [
        leg(Decimal::new(333, 2), rate, &registry, usd), // 4.995 -> 5.00
        leg(Decimal::new(333, 2), rate, &registry, usd), // 4.995 -> 5.00
        leg(Decimal::new(334, 2), rate, &registry, usd), // 5.010 -> 5.01
        leg(Decimal::new(-1_000, 2), rate, &registry, usd), // -15.00 exactly
    ];

    // The amounts balance in EUR, to the cent.
    let amounts = legs
        .iter()
        .map(Posting::amount)
        .reduce(|acc, amount| acc.checked_add(amount).expect("one commodity"))
        .expect("four legs");
    assert!(amounts.is_zero(), "the EUR amounts must balance exactly");

    // The values do not: rounding up twice manufactured a cent.
    let value = total_value(&legs);
    assert_eq!(value.number(), Decimal::new(1, 2));
    assert_eq!(value.commodity(), usd);

    // And the residues account for the whole of that discrepancy.
    assert_eq!(total_residue(&legs), Decimal::new(-1, 2));
    assert_eq!(value.number() + total_residue(&legs), Decimal::ZERO);
}

/// The three derivation modes a caller reaches for, each from the story that
/// produces it.
#[test]
fn a_caller_may_supply_any_sufficient_part_of_the_triple() {
    let (registry, usd, eur) = registry_with_eur();
    let usd_commodity = registry.get_by_code("USD").expect("USD is registered");
    let eur_commodity = registry.get_by_code("EUR").expect("EUR is registered");

    // Entered directly in functional terms: no conversion, no rate.
    let direct = Quantity::try_new(Decimal::new(1_500, 2), usd_commodity).expect("two decimals");
    let posting =
        Posting::try_new(new_id(), PostingValuation::Functional(direct), new_id(), usd, &registry)
            .expect("a USD quantity in a USD ledger");
    assert_eq!(posting.amount(), posting.value());
    assert_eq!(posting.stated_rate(), None);
    assert_eq!(posting.derived_rate(), Some(Rate::Identity(usd)));

    // A card statement showing both sides, where the bank's own figures rule.
    let posting = Posting::try_new(
        new_id(),
        PostingValuation::AmountAndValue {
            amount: Quantity::try_new(Decimal::new(12_000, 2), eur_commodity).expect("decimals"),
            value: Quantity::try_new(Decimal::new(9_450, 2), usd_commodity).expect("decimals"),
        },
        new_id(),
        usd,
        &registry,
    )
    .expect("both sides given");
    assert_eq!(posting.stated_rate(), None);
    // No rate was stated, but the stored pair still implies one.
    assert_eq!(posting.derived_rate().expect("a rate is implied").number(), Decimal::new(7_875, 4));

    // A pinned functional budget: 500.00 USD at 0.80 USD/EUR buys 625.00 EUR.
    let rate = Rate::try_new(Decimal::new(80, 2), usd, eur).expect("distinct commodities");
    let posting = Posting::try_new(
        new_id(),
        PostingValuation::ValueAtRate {
            value: Quantity::try_new(Decimal::new(50_000, 2), usd_commodity).expect("decimals"),
            stated_rate: rate,
        },
        new_id(),
        usd,
        &registry,
    )
    .expect("a USD value at a USD/EUR rate");
    assert_eq!(posting.amount().number(), Decimal::new(62_500, 2));
    assert_eq!(posting.amount().commodity(), eur);
    // Only a derived *value* reports a residue; this derived an amount.
    assert_eq!(posting.value_residue(), None);
}

/// `Posting::try_new` hands a `PostingValuation` and a `PostingId` back and
/// forth, so a consumer has to be able to name both without reaching inside.
#[test]
fn a_posting_is_nameable_by_a_consumer() {
    let (registry, usd, _eur) = registry_with_eur();
    let usd_commodity = registry.get_by_code("USD").expect("USD is registered");

    let id: PostingId = new_id();
    let valuation: PostingValuation = PostingValuation::Functional(
        Quantity::try_new(Decimal::new(1, 0), usd_commodity).expect("one dollar"),
    );

    let posting = Posting::try_new(id, valuation, new_id(), usd, &registry).expect("a USD posting");
    assert_eq!(posting.id(), id);
    // The id renders with the prefix reserved for postings.
    assert!(posting.id().to_string().starts_with("pst_"), "got {}", posting.id());
}

/// The composing root lets a caller bubble a posting failure with `?` alongside
/// any other domain error.
#[test]
fn posting_errors_bubble_into_the_root_error() {
    fn book_a_foreign_amount_with_no_rate(
        registry: &CommodityRegistry,
        usd: CommodityId,
    ) -> Result<Posting, Error> {
        let eur = registry.get_by_code("EUR").expect("EUR is registered");
        let amount = Quantity::try_new(Decimal::new(10_000, 2), eur)?;
        // A lone EUR quantity cannot be valued in a USD ledger: no rate to infer.
        Ok(Posting::try_new(
            new_id(),
            PostingValuation::Functional(amount),
            new_id(),
            usd,
            registry,
        )?)
    }

    let (registry, usd, eur) = registry_with_eur();
    let err = book_a_foreign_amount_with_no_rate(&registry, usd)
        .expect_err("100.00 EUR is not a USD value");

    assert!(
        matches!(
            err,
            Error::Posting(PostingError::FunctionalMismatch { got, expected })
                if got == eur && expected == usd
        ),
        "got {err}"
    );

    // `#[error(transparent)]` means the root reads exactly like the domain error.
    let domain = PostingError::FunctionalMismatch { got: eur, expected: usd };
    assert_eq!(err.to_string(), domain.to_string());
}
