//! The commodity API as a consumer meets it: the registry is the only door in.

use centimeter_core::{
    Commodity, CommodityError, CommodityId, CommodityRegistry, Error, Identifiable,
};

use crate::fixtures::new_id;

#[test]
fn a_ledger_worth_of_commodities_registers_and_reads_back() {
    let mut reg = CommodityRegistry::new();
    let (gbp, usd, jpy, btc, aapl, hour) =
        (new_id(), new_id(), new_id(), new_id(), new_id(), new_id());

    // A currency, two more currencies at different scales, a crypto asset, a
    // security, and a non-currency unit: one type covers all of them.
    reg.add(gbp, "GBP", "Pound Sterling", 2).unwrap();
    reg.add(usd, "USD", "US Dollar", 2).unwrap();
    reg.add(jpy, "JPY", "Japanese Yen", 0).unwrap();
    reg.add(btc, "BTC", "Bitcoin", 8).unwrap();
    reg.add(aapl, "AAPL", "Apple Inc.", 4).unwrap();
    reg.add(hour, "HOUR", "Hour", 2).unwrap();

    let yen = reg.get(jpy).expect("JPY should be registered");
    assert_eq!(yen.id(), jpy);
    assert_eq!(yen.code(), "JPY");
    assert_eq!(yen.name(), "Japanese Yen");
    assert_eq!(yen.scale(), 0, "a scale-0 currency must survive as scale 0");

    assert_eq!(reg.get_by_code("BTC").expect("BTC should be findable").scale(), 8);
    assert_eq!(reg.get_by_code("AAPL").expect("AAPL should be findable").id(), aapl);
    assert_eq!(reg.get_by_code("HOUR").expect("HOUR should be findable").name(), "Hour");
}

#[test]
fn a_registered_commodity_compares_equal_to_itself() {
    let mut reg = CommodityRegistry::new();
    let usd = new_id();
    reg.add(usd, "USD", "US Dollar", 2).unwrap();

    let a = reg.get(usd).unwrap();
    let b = reg.get_by_code("USD").unwrap();
    assert_eq!(a, b, "both lookups must yield the same commodity");
    assert_eq!(a.clone(), *b, "a clone must equal its source");
}

#[test]
fn duplicates_are_refused_at_the_boundary() {
    let mut reg = CommodityRegistry::new();
    let first = new_id();
    reg.add(first, "USD", "US Dollar", 2).unwrap();

    let by_id = reg.add(first, "EUR", "Euro", 2).unwrap_err();
    assert!(matches!(by_id, CommodityError::DuplicateId { .. }), "got {by_id}");

    let by_code = reg.add(new_id(), "USD", "US Dollar", 2).unwrap_err();
    assert!(matches!(by_code, CommodityError::DuplicateCode { .. }), "got {by_code}");

    // Neither rejection disturbed the registry.
    assert_eq!(reg.get(first).unwrap().name(), "US Dollar");
    assert!(reg.get_by_code("EUR").is_none());
}

#[test]
fn invalid_input_is_reported_as_a_typed_error() {
    let mut reg = CommodityRegistry::new();

    let err = reg.add(new_id(), "US D", "US Dollar", 2).unwrap_err();
    assert!(matches!(err, CommodityError::CodeBadChar { .. }), "got {err}");

    let err = reg.add(new_id(), "USD", "", 2).unwrap_err();
    assert!(matches!(err, CommodityError::NameEmpty), "got {err}");

    let err = reg.add(new_id(), "USD", "US Dollar", 29).unwrap_err();
    assert!(
        matches!(err, CommodityError::ScaleTooLarge { max, got } if max == 28 && got == 29),
        "got {err}"
    );
}

/// The composing root exists so a caller can bubble a domain error up with `?`.
#[test]
fn domain_errors_bubble_into_the_root_error() {
    fn register(reg: &mut CommodityRegistry, scale: u8) -> Result<(), Error> {
        reg.add(new_id(), "USD", "US Dollar", scale)?;
        Ok(())
    }

    let mut reg = CommodityRegistry::new();
    let err = register(&mut reg, 29).expect_err("scale 29 must be rejected");
    assert!(matches!(err, Error::Commodity(CommodityError::ScaleTooLarge { .. })), "got {err}");

    // `#[error(transparent)]` means the root reads exactly like the domain error.
    let domain = CommodityError::ScaleTooLarge { max: Commodity::MAX_SCALE, got: 29 };
    assert_eq!(err.to_string(), domain.to_string());

    assert!(register(&mut reg, 2).is_ok());
}

/// The `cmo` prefix is allocated permanently in `docs/id-prefixes.md`, so a
/// change to it is a breaking change and must fail a test, not just a review.
#[test]
fn commodity_ids_are_prefixed_and_round_trip_as_strings() {
    let id = new_id();
    let as_string = id.to_string();

    assert_eq!(Commodity::PREFIX.as_str(), "cmo");
    assert!(as_string.starts_with("cmo_"), "{as_string} should carry the cmo prefix");
    assert_eq!(as_string.len(), "cmo_".len() + 26);

    let parsed: CommodityId = as_string.parse().expect("a rendered id must parse back");
    assert_eq!(parsed, id);
    assert_eq!(parsed.as_uuid(), id.as_uuid());
}

#[test]
fn an_id_minted_for_another_record_does_not_parse_as_a_commodity_id() {
    let id = new_id();
    let foreign = id.to_string().replace("cmo_", "acc_");

    let err = foreign.parse::<CommodityId>().expect_err("a foreign prefix must not parse");
    assert!(err.to_string().contains("cmo"), "the error should name the expected prefix: {err}");
}
