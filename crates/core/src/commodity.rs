//! Commodities: the units centimeter measures value in.
//!
//! A commodity is anything an amount can be denominated in: a currency, a
//! security, a crypto asset, or a countable unit like an hour or kWh. There is
//! no separate `Currency` type: a currency is simply a commodity that happens
//! to be legal tender. That keeps single-currency, multi-currency, and
//! non-currency bookkeeping on one unified code path.
//!
//! Every commodity carries a [`scale`](Commodity::scale), the number of
//! decimal places an amount denominated in it may have. Scale belongs to the
//! commodity rather than to the number, which is why `JPY` is scale 0, `USD`
//! is scale 2, and `BTC` is scale 8.
//!
//! A [`CommodityRegistry`] owns every commodity a ledger can post in, and is
//! the only way to create one.

use crate::{Id, IdPrefix, Identifiable};

mod code;
mod error;
mod registry;

use code::CommodityCode;
pub use error::CommodityError;
pub use registry::CommodityRegistry;

/// A unit of measure of value: a currency, a security, or any countable
/// quantity.
///
/// A commodity is addressed two ways: by its [`id`](Self::id), which postings
/// and balances reference, and by its [`code`](Self::code), the natural key a
/// user reads and types. Both are unique within a [`CommodityRegistry`].
///
/// Values of this type are produced only by [`CommodityRegistry::add`], so
/// holding one is proof that its fields have already been validated.
///
/// # Examples
///
/// ```
/// # use centimeter_core::{CommodityId, CommodityRegistry};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut registry = CommodityRegistry::new();
/// // Core never mints ids, so the caller supplies a UUIDv7.
/// let id = CommodityId::from_uuid(uuid::Uuid::now_v7())?;
/// registry.add(id, "JPY", "Japanese Yen", 0)?;
///
/// let yen = registry.get(id).expect("just registered");
/// assert_eq!(yen.code(), "JPY");
/// assert_eq!(yen.scale(), 0);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Commodity {
    /// The unique ID for this commodity.
    id: CommodityId,
    /// The commodity's natural key (e.g. `"USD"`, `"AAPL"`, `"BTC"`, `"HOUR"`).
    code: CommodityCode,
    /// The human-readable display name (e.g. `"US Dollar"`, `"Apple Inc."`,
    /// `"Bitcoin"`, `"Hour"`).
    name: String,
    /// The number of decimal places for quantities denominated in this
    /// commodity. The smallest representable increment is `10^-scale`:
    /// 0 for JPY, 2 for USD, 8 for BTC.
    /// At most [`Commodity::MAX_SCALE`].
    scale: u8,
}

impl Identifiable for Commodity {
    const PREFIX: IdPrefix = IdPrefix::new("cmo");
}

/// The id of a [`Commodity`], rendered as `cmo_<suffix>`.
pub type CommodityId = Id<Commodity>;

impl PartialEq for Commodity {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Commodity {}

impl std::hash::Hash for Commodity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Commodity {
    /// The maximum length of a commodity name (counted in chars).
    pub const MAX_NAME_LENGTH: usize = 256;

    /// The upper bound for [`Commodity::scale`], fixed by `rust_decimal`'s
    /// limit of 28 decimal places.
    ///
    /// Scale is not precision: a commodity declared at 28 cannot represent
    /// numbers much above 7.9, because the mantissa is only 96 bits.
    pub const MAX_SCALE: u8 = 28;

    fn validate_name(name: &str) -> Result<String, CommodityError> {
        let name = name.trim();
        let len = name.chars().count();
        if name.is_empty() {
            return Err(CommodityError::NameEmpty);
        }
        if Self::MAX_NAME_LENGTH < len {
            return Err(CommodityError::NameTooLong { max: Self::MAX_NAME_LENGTH, got: len });
        }
        if let Some((index, _)) = name.char_indices().find(|(_, c)| c.is_control()) {
            return Err(CommodityError::NameBadChar { got: name.to_string(), index });
        }

        Ok(name.to_string())
    }

    fn validate_scale(scale: u8) -> Result<u8, CommodityError> {
        if Self::MAX_SCALE < scale {
            return Err(CommodityError::ScaleTooLarge { max: Self::MAX_SCALE, got: scale });
        }
        Ok(scale)
    }

    /// Creates a new commodity from `code`, `name` and `scale`.
    ///
    /// The code arrives pre-validated because the registry has to check it for
    /// uniqueness before it is willing to build a commodity at all.
    ///
    /// `name` is trimmed first, so surrounding whitespace is discarded rather
    /// than rejected.
    ///
    /// # Errors
    /// - [`NameEmpty`](CommodityError::NameEmpty) if `name` is empty or only
    ///   whitespace.
    /// - [`NameTooLong`](CommodityError::NameTooLong) if `name` exceeds
    ///   [`MAX_NAME_LENGTH`](Self::MAX_NAME_LENGTH) characters.
    /// - [`NameBadChar`](CommodityError::NameBadChar) if `name` contains a
    ///   control character.
    /// - [`ScaleTooLarge`](CommodityError::ScaleTooLarge) if `scale` exceeds
    ///   [`MAX_SCALE`](Self::MAX_SCALE).
    fn try_new(
        id: CommodityId,
        code: CommodityCode,
        name: &str,
        scale: u8,
    ) -> Result<Self, CommodityError> {
        let name = Self::validate_name(name)?;
        let scale = Self::validate_scale(scale)?;
        Ok(Self { id, code, name, scale })
    }

    /// Updates the display name, validating the replacement first.
    ///
    /// The name is the only commodity field that may change after registration.
    /// `code` & `scale` are frozen, as amounts already recorded depend on them.
    ///
    /// # Errors
    /// Returns an error if `name` is empty, exceeds
    /// [`MAX_NAME_LENGTH`](Self::MAX_NAME_LENGTH) characters, or contains a
    /// control character.
    fn set_name(&mut self, name: &str) -> Result<(), CommodityError> {
        self.name = Self::validate_name(name)?;
        Ok(())
    }

    /// Returns the commodity's id.
    #[must_use]
    pub fn id(&self) -> CommodityId {
        self.id
    }

    /// Returns the commodity's code (e.g. `"USD"`).
    #[must_use]
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// Returns the commodity's human-readable name (e.g. `"US Dollar"`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the commodity's scale: the number of decimal places for
    /// quantities denominated in it.
    #[must_use]
    pub fn scale(&self) -> u8 {
        self.scale
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rust_decimal::Decimal;

    use super::*;

    use crate::test_support::id;

    #[test]
    fn test_id_helper_is_reproducible_and_distinct() {
        // `id` is generic over the record type, and nothing here constrains it.
        assert_eq!(id::<Commodity>(7), id::<Commodity>(7), "the same seed must yield the same id");
        let ids: std::collections::HashSet<_> = (0..64).map(id::<Commodity>).collect();
        assert_eq!(ids.len(), 64, "distinct seeds must yield distinct ids");
    }

    /// A validated code, for tests whose subject is not code validation.
    fn valid_code(code: &str) -> CommodityCode {
        CommodityCode::try_new(code).expect("test code must be valid")
    }

    /// Attempts a `USD` commodity under `id(0)` with the given name and scale.
    fn try_build(name: &str, scale: u8) -> Result<Commodity, CommodityError> {
        Commodity::try_new(id(0), valid_code("USD"), name, scale)
    }

    /// Same as [`try_build`], panicking if the input was rejected.
    fn build(name: &str, scale: u8) -> Commodity {
        try_build(name, scale)
            .unwrap_or_else(|e| panic!("{name:?} at scale {scale} should be accepted, got {e}"))
    }

    #[test]
    fn test_commodity_exposes_its_fields() {
        let cmo_id = id(1);
        let usd = Commodity::try_new(cmo_id, valid_code("USD"), "US Dollar", 2).unwrap();

        assert_eq!(usd.id(), cmo_id);
        assert_eq!(usd.code(), "USD");
        assert_eq!(usd.name(), "US Dollar");
        assert_eq!(usd.scale(), 2);
    }

    #[test]
    fn test_name_is_trimmed() {
        assert_eq!(build("  US Dollar \n", 2).name(), "US Dollar");
    }

    #[test]
    fn test_name_rejects_empty() {
        for name in ["", "   ", "\t\n"] {
            assert!(
                matches!(try_build(name, 2), Err(CommodityError::NameEmpty)),
                "{name:?} should be rejected as empty"
            );
        }
    }

    #[test]
    fn test_name_length_is_measured_in_chars() {
        // Exactly at the limit is fine, one over is not.
        let at_limit = "a".repeat(Commodity::MAX_NAME_LENGTH);
        assert_eq!(build(&at_limit, 2).name().chars().count(), Commodity::MAX_NAME_LENGTH);

        let over = "a".repeat(Commodity::MAX_NAME_LENGTH + 1);
        assert!(matches!(
            try_build(&over, 2),
            Err(CommodityError::NameTooLong { max: Commodity::MAX_NAME_LENGTH, got: 257 })
        ));

        // Chars, not bytes: 256 two-byte chars is 512 bytes and still accepted.
        let multibyte = "é".repeat(Commodity::MAX_NAME_LENGTH);
        assert_eq!(multibyte.len(), 2 * Commodity::MAX_NAME_LENGTH);
        assert!(try_build(&multibyte, 2).is_ok());
    }

    #[test]
    fn test_name_rejects_control_chars() {
        for (name, index) in [("US\u{7}Dollar", 2), ("a\rb", 1), ("x\u{1b}[2J", 1)] {
            assert!(
                matches!(
                    try_build(name, 2),
                    Err(CommodityError::NameBadChar { index: at, .. }) if at == index
                ),
                "{name:?} should be rejected at index {index}"
            );
        }
    }

    #[test]
    fn test_name_allows_non_control_unicode() {
        // Names are display strings: accents, CJK, and symbols must survive.
        for name in ["US Dollar", "Café 🍰 Voucher", "日本円", "Brent Crude (bbl)", "£ sterling"]
        {
            assert_eq!(build(name, 2).name(), name);
        }
    }

    #[test]
    fn test_name_error_escapes_untrusted_input() {
        let err = try_build("US\u{1b}[2JD", 2).unwrap_err();
        let message = err.to_string();
        assert!(!message.contains('\u{1b}'), "message leaked a raw escape: {message:?}");
    }

    #[test]
    fn test_scale_bounds() {
        // JPY, USD, BTC, and the maximum all sit inside the accepted range.
        for scale in [0, 2, 8, Commodity::MAX_SCALE] {
            assert_eq!(build("US Dollar", scale).scale(), scale);
        }

        assert!(matches!(
            try_build("US Dollar", Commodity::MAX_SCALE + 1),
            Err(CommodityError::ScaleTooLarge { max: Commodity::MAX_SCALE, got: 29 })
        ));
    }

    /// [`Commodity::MAX_SCALE`] exists only to mirror `rust_decimal`'s hard
    /// limit, so the two must be proven to agree rather than assumed to.
    #[test]
    fn test_max_scale_matches_rust_decimal() {
        assert_eq!(u32::from(Commodity::MAX_SCALE), Decimal::MAX_SCALE);

        // The limit is inclusive: a decimal at MAX_SCALE is constructible.
        let at_limit = Decimal::try_from_i128_with_scale(1, u32::from(Commodity::MAX_SCALE));
        assert!(at_limit.is_ok(), "scale {} should be representable", Commodity::MAX_SCALE);
        // ...and one past it is not.
        let over = Decimal::try_from_i128_with_scale(1, u32::from(Commodity::MAX_SCALE) + 1);
        assert!(over.is_err(), "scale {} should be rejected", Commodity::MAX_SCALE + 1);
    }

    #[test]
    fn test_identity_is_by_id() {
        let usd1 = build("US Dollar", 2);
        let usd2 = Commodity::try_new(usd1.id(), valid_code("USD"), "US Dollar", 2).unwrap();
        assert_eq!(usd1, usd2, "commodities with the same id must be equal");
        let usd3 = Commodity::try_new(id(1), valid_code("USD"), "US Dollar", 2).unwrap();
        assert_ne!(usd1, usd3, "commodities with different ids must not be equal");
    }

    #[test]
    fn test_hash_is_by_id() {
        let usd = build("US Dollar", 2);
        // Same id, every other field different: still the same commodity.
        let alias =
            Commodity::try_new(id(0), valid_code("usd"), "United States Dollar", 3).unwrap();
        let other = Commodity::try_new(id(1), valid_code("USD"), "US Dollar", 2).unwrap();

        let mut set = std::collections::HashSet::new();
        set.insert(usd);
        set.insert(alias);
        assert_eq!(set.len(), 1, "commodities with the same id must collapse to one entry");
        set.insert(other);
        assert_eq!(set.len(), 2, "commodities with different ids must be distinct entries");
    }

    #[test]
    fn test_set_name_replaces_name() {
        let mut usd = build("US Dollar", 2);
        usd.set_name("United States Dollar").unwrap();
        assert_eq!(usd.name(), "United States Dollar");
        assert_eq!(usd.code(), "USD");
        assert_eq!(usd.scale(), 2);
    }

    #[test]
    fn test_set_name_rejects_invalid() {
        let mut usd = build("US Dollar", 2);
        for name in ["", "   ", "\t\n", "US\u{7}Dollar"] {
            assert!(matches!(
                usd.set_name(name),
                Err(CommodityError::NameEmpty | CommodityError::NameBadChar { .. })
            ));
        }
        assert_eq!(usd.name(), "US Dollar");
    }

    #[test]
    fn test_set_name_trims() {
        let mut usd = build("US Dollar", 2);
        usd.set_name("  United States Dollar \n").unwrap();
        assert_eq!(usd.name(), "United States Dollar");
    }

    proptest! {
        /// Validation never panics, and acceptance implies the documented shape.
        #[test]
        fn prop_validation_is_total(name: String, scale: u8) {
            if let Ok(cmo) = try_build(&name, scale) {
                prop_assert_eq!(cmo.name(), name.trim());
                prop_assert!(!cmo.name().is_empty());
                prop_assert!(cmo.name().chars().count() <= Commodity::MAX_NAME_LENGTH);
                prop_assert!(!cmo.name().chars().any(char::is_control));
                prop_assert!(cmo.scale() <= Commodity::MAX_SCALE);
            }
        }

        /// Every scale in range is stored verbatim; every scale out of it is refused.
        #[test]
        fn prop_scale_is_bounded(scale: u8) {
            let built = try_build("US Dollar", scale);
            if scale <= Commodity::MAX_SCALE {
                prop_assert_eq!(built.unwrap().scale(), scale);
            } else {
                prop_assert!(
                    matches!(built, Err(CommodityError::ScaleTooLarge { .. })),
                    "scale {scale} should be rejected, got {built:?}"
                );
            }
        }
    }
}
