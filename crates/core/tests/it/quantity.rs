//! Quantities as a consumer meets them: resolve a commodity, then do arithmetic.

use centimeter_core::{CommodityId, CommodityRegistry, Decimal, Error, Quantity, QuantityError};

use crate::fixtures::registry;

#[test]
fn quantities_add_and_subtract_within_a_commodity() {
    let (reg, usd, _jpy) = registry();
    let dollar = reg.get(usd).expect("USD is registered");

    let a = Quantity::try_new(Decimal::new(1500, 2), dollar).expect("15.00 is representable");
    let b = Quantity::try_new(Decimal::new(50, 2), dollar).expect("0.50 is representable");

    let sum = a.checked_add(b).expect("same commodity, no overflow");
    assert_eq!(sum.number(), Decimal::new(1550, 2));
    assert_eq!(sum.commodity(), usd);

    let diff = b.checked_sub(a).expect("same commodity, no overflow");
    assert_eq!(diff.number(), Decimal::new(-1450, 2));

    // Scale is the commodity's, on every result.
    assert_eq!(sum.number().scale(), 2);
    assert_eq!(diff.number().scale(), 2);
}

#[test]
fn a_quantity_takes_its_scale_from_its_commodity_not_its_input() {
    let (reg, usd, jpy) = registry();

    // The same written number lands at a different scale per commodity.
    let dollars = Quantity::try_new(Decimal::new(5, 0), reg.get(usd).expect("USD")).expect("5 USD");
    let yen = Quantity::try_new(Decimal::new(5, 0), reg.get(jpy).expect("JPY")).expect("5 JPY");

    assert_eq!(dollars.number().scale(), 2);
    assert_eq!(yen.number().scale(), 0);
    assert_eq!(dollars.number(), yen.number(), "both are five, spelled differently");
}

#[test]
fn quantities_of_different_commodities_refuse_to_combine() {
    let (reg, usd, jpy) = registry();
    let dollars = Quantity::try_new(Decimal::ONE, reg.get(usd).expect("USD")).expect("1 USD");
    let yen = Quantity::try_new(Decimal::ONE, reg.get(jpy).expect("JPY")).expect("1 JPY");

    let err = dollars.checked_add(yen).expect_err("USD and JPY must not combine");
    assert!(
        matches!(err, QuantityError::CommodityMismatch { left, right } if left == usd && right == jpy),
        "got {err}"
    );
    assert!(dollars.checked_sub(yen).is_err(), "subtraction must refuse it too");
}

/// The spelling of a number is adjusted to the commodity; its value never is.
#[test]
fn an_over_precise_number_is_refused_while_a_padded_one_is_accepted() {
    let (reg, usd, _jpy) = registry();
    let dollar = reg.get(usd).expect("USD is registered");

    // 1.500 is 1.50 written differently, so it is accepted and stored as 1.50.
    let padded = Quantity::try_new(Decimal::new(1500, 3), dollar).expect("1.500 is 1.50");
    assert_eq!(padded.number(), Decimal::new(150, 2));
    assert_eq!(padded.number().scale(), 2);

    // 1.555 would have to lose a significant digit, so it is refused outright
    // rather than rounded to 1.56 (ADR-0004).
    let err = Quantity::try_new(Decimal::new(1555, 3), dollar).expect_err("1.555 is not 1.56");
    assert!(matches!(err, QuantityError::ScaleTooLarge { scale: 2, .. }), "got {err}");
}

#[test]
fn a_zero_quantity_carries_its_commodity_scale() {
    let (reg, usd, jpy) = registry();

    let no_dollars = Quantity::zero(reg.get(usd).expect("USD"));
    let no_yen = Quantity::zero(reg.get(jpy).expect("JPY"));

    assert!(no_dollars.is_zero() && no_yen.is_zero());
    assert_eq!(no_dollars.number().scale(), 2);
    assert_eq!(no_yen.number().scale(), 0);

    // Negating zero must not produce a "-0.00" for a report to render.
    assert!(!(-no_dollars).number().is_sign_negative());
}

/// The composing root (ADR-0022) lets a caller mix commodity and quantity
/// failures in one `Result` without writing a conversion.
#[test]
fn quantity_errors_bubble_into_the_root_error() {
    fn add_across(reg: &CommodityRegistry, a: CommodityId, b: CommodityId) -> Result<(), Error> {
        let left = Quantity::try_new(Decimal::ONE, reg.get(a).expect("registered"))?;
        let right = Quantity::try_new(Decimal::ONE, reg.get(b).expect("registered"))?;
        left.checked_add(right)?;
        Ok(())
    }

    let (reg, usd, jpy) = registry();
    let err = add_across(&reg, usd, jpy).expect_err("USD and JPY must not combine");
    assert!(matches!(err, Error::Quantity(QuantityError::CommodityMismatch { .. })), "got {err}");

    // `#[error(transparent)]` means the root reads exactly like the domain error.
    let domain = QuantityError::CommodityMismatch { left: usd, right: jpy };
    assert_eq!(err.to_string(), domain.to_string());

    assert!(add_across(&reg, usd, usd).is_ok(), "one commodity must combine fine");
}
