//! Rates as a consumer meets them: ids in, a checked ratio out.

use centimeter_core::{CommodityId, CommodityPair, Decimal, Error, Rate, RateError};

use crate::fixtures::registry;

#[test]
fn rates_are_built_from_registered_commodities() {
    let (reg, usd, jpy) = registry();

    // 150 JPY per USD. Quote is the numerator, base the denominator, so the
    // slash in "JPY/USD" is a division and not the FX pair convention.
    let rate = Rate::try_new(Decimal::new(150, 0), jpy, usd).expect("distinct commodities");

    // Reading three fields off one value also proves `Rate` is `Copy` here.
    assert_eq!(rate.number(), Decimal::new(150, 0));
    assert_eq!(rate.quote(), jpy);
    assert_eq!(rate.base(), usd);

    // A rate carries ids only, so the registry is what turns it back into codes.
    assert_eq!(reg.get(rate.quote()).expect("JPY is registered").code(), "JPY");
    assert_eq!(reg.get(rate.base()).expect("USD is registered").code(), "USD");
}

/// The headline claim of the module, asserted from outside: a commodity
/// converts to itself at exactly one, and at nothing else.
#[test]
fn a_commodity_converts_to_itself_at_exactly_one() {
    let (_reg, usd, _jpy) = registry();

    let rate = Rate::try_new(Decimal::ONE, usd, usd).expect("one is a valid identity");
    assert_eq!(rate, Rate::Identity(usd));
    assert_eq!(rate.number(), Decimal::ONE);
    assert_eq!(rate.quote(), rate.base());

    let err = Rate::try_new(Decimal::new(11, 1), usd, usd).expect_err("1.1 USD per USD");
    assert!(matches!(err, RateError::BadIdentityRate { got } if got == Decimal::new(11, 1)));
}

/// A rate of one between *different* commodities stays a conversion. Collapsing
/// it to `Identity` would throw away which pair it belonged to.
#[test]
fn a_unit_conversion_is_not_an_identity() {
    let (_reg, usd, jpy) = registry();

    let rate = Rate::try_new(Decimal::ONE, jpy, usd).expect("distinct commodities");
    assert!(matches!(rate, Rate::Conversion { .. }), "got {rate:?}");
    assert_ne!(rate, Rate::Identity(usd));
    assert_ne!(rate.quote(), rate.base());
}

/// `Rate::Conversion` hands a `CommodityPair` to whoever matches on it, so a
/// consumer has to be able to name and build one. It was briefly unreachable.
#[test]
fn commodity_pair_is_nameable_by_a_consumer() {
    let (_reg, usd, jpy) = registry();

    let pair: CommodityPair = CommodityPair::try_new(jpy, usd).expect("distinct commodities");
    let rate = Rate::Conversion { number: Decimal::new(150, 0), pair };

    let Rate::Conversion { pair: read_back, .. } = rate else {
        panic!("a conversion must match as one");
    };
    assert_eq!(read_back, pair);
    assert_eq!(read_back.quote(), jpy);
    assert_eq!(read_back.base(), usd);

    // Direction is part of the pair's identity, not a display detail.
    let reversed = CommodityPair::try_new(usd, jpy).expect("distinct commodities");
    assert_ne!(pair, reversed);
}

#[test]
fn a_pair_needs_two_commodities() {
    let (_reg, usd, _jpy) = registry();

    let err = CommodityPair::try_new(usd, usd).expect_err("one commodity is not a pair");
    assert!(matches!(err, RateError::SameCommodity { got } if got == usd), "got {err}");
}

#[test]
fn zero_and_negative_rates_are_accepted() {
    let (_reg, usd, jpy) = registry();

    // Both are real: a fully depreciated transfer, and negative oil prices.
    for number in [Decimal::ZERO, Decimal::new(-125, 2)] {
        let rate = Rate::try_new(number, jpy, usd).expect("rates are not sign constrained");
        assert_eq!(rate.number(), number);
    }
}

/// The composing root lets a caller bubble a rate failure with `?` alongside
/// any other domain error.
#[test]
fn rate_errors_bubble_into_the_root_error() {
    fn quote_against_itself(commodity: CommodityId) -> Result<Rate, Error> {
        Ok(Rate::try_new(Decimal::new(2, 0), commodity, commodity)?)
    }

    let (_reg, usd, _jpy) = registry();
    let err = quote_against_itself(usd).expect_err("2.0 USD per USD is not an identity");
    assert!(matches!(err, Error::Rate(RateError::BadIdentityRate { .. })), "got {err}");

    // `#[error(transparent)]` means the root reads exactly like the domain error.
    let domain = RateError::BadIdentityRate { got: Decimal::new(2, 0) };
    assert_eq!(err.to_string(), domain.to_string());
}
