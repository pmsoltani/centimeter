//! Ownership and lookup for a ledger's commodities.
//!
//! Every commodity a ledger can post in lives in one and only
//! [`CommodityRegistry`]. Because the registry is the only way to build a
//! [`Commodity`], holding one is proof that it was validated and that its `id`
//! and `code` are unique among its peers, so a posting can reference a
//! [`CommodityId`] without re-checking anything.

use std::collections::HashMap;

use super::{Commodity, CommodityCode, CommodityError, CommodityId};

/// The set of commodities a ledger can post in.
///
/// Adding enforces two uniqueness rules: no two commodities share an `id`, and
/// no two share a [`code`](Commodity::code). Either lookup therefore returns
/// at most one commodity.
///
/// The registry carries no id of its own. It is always reached through the
/// `Ledger` that owns it, so there is nothing to address it by.
///
/// # Examples
///
/// ```
/// # use centimeter_core::{Commodity, CommodityError, CommodityId, CommodityRegistry};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut registry = CommodityRegistry::new();
/// let usd = CommodityId::from_uuid(uuid::Uuid::now_v7())?;
/// registry.add(usd, "USD", "US Dollar", 2)?;
///
/// assert_eq!(registry.get_by_code("USD").map(Commodity::id), Some(usd));
///
/// // The code is taken, whoever asks for it next.
/// let other = CommodityId::from_uuid(uuid::Uuid::now_v7())?;
/// assert!(matches!(
///     registry.add(other, "USD", "United States Dollar", 2),
///     Err(CommodityError::DuplicateCode { .. })
/// ));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct CommodityRegistry {
    commodities: HashMap<CommodityId, Commodity>,
    // TODO: consider adding a separate index code->id if performance needs it.
}

impl CommodityRegistry {
    /// Creates a new, empty commodity registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a commodity to the registry.
    ///
    /// NOTE: Once a commodity is added, it cannot be removed!
    /// NOTE: Once a commodity is added, its `code` & `scale` cannot be changed!
    ///
    /// # Errors
    /// Checks run in this order:
    /// 1. `code` is malformed; see the `Code*` variants of [`CommodityError`].
    /// 2. [`DuplicateId`](CommodityError::DuplicateId) if `id` is taken.
    /// 3. [`DuplicateCode`](CommodityError::DuplicateCode) if `code` is taken,
    ///    naming the commodity already holding it.
    /// 4. `name` or `scale` is invalid; see [`Commodity`].
    pub fn add(
        &mut self,
        id: CommodityId,
        code: &str,
        name: &str,
        scale: u8,
    ) -> Result<(), CommodityError> {
        let code = CommodityCode::try_new(code)?;
        if self.commodities.contains_key(&id) {
            return Err(CommodityError::DuplicateId { id });
        }
        if let Some(existing) = self.get_by_code(code.as_str()) {
            return Err(CommodityError::DuplicateCode {
                code: code.as_str().to_string(),
                id: existing.id(),
            });
        }
        let commodity = Commodity::try_new(id, code, name, scale)?;
        self.commodities.insert(commodity.id(), commodity);
        Ok(())
    }

    /// Gets a commodity by its ID.
    #[must_use]
    pub fn get(&self, id: CommodityId) -> Option<&Commodity> {
        self.commodities.get(&id)
    }

    /// Gets a commodity by its code.
    ///
    /// The match is exact: `"usd"` will not find `"USD"`.
    #[must_use]
    pub fn get_by_code(&self, code: &str) -> Option<&Commodity> {
        self.commodities.values().find(|c| c.code() == code)
    }

    /// Updates the display name of a commodity.
    ///
    /// # Errors
    /// - [`NotFound`](CommodityError::NotFound) if no commodity has `id`.
    /// - A name error if `name` is empty, too long, or has a control character.
    pub fn rename(&mut self, id: CommodityId, name: &str) -> Result<(), CommodityError> {
        let commodity = self.commodities.get_mut(&id).ok_or(CommodityError::NotFound { id })?;
        commodity.set_name(name)
    }

    /// Returns true if the registry has no commodities.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commodities.is_empty()
    }

    /// Returns the number of commodities in the registry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commodities.len()
    }

    /// Returns an iterator over the commodities in the registry.
    pub fn iter(&self) -> impl Iterator<Item = &Commodity> {
        self.commodities.values()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

    use crate::test_support::id;

    /// Asserts the registry holds `code` under `id` with the given name/scale.
    fn assert_registered(reg: &CommodityRegistry, id: CommodityId, code: &str, scale: u8) {
        let by_id = reg.get(id).unwrap_or_else(|| panic!("{id} should be registered"));
        assert_eq!(by_id.code(), code);
        assert_eq!(by_id.scale(), scale);

        let by_code = reg.get_by_code(code).unwrap_or_else(|| panic!("{code} should be findable"));
        assert_eq!(by_code.id(), id, "the two lookups must agree");
    }

    /// A registry holding one commodity, `USD` at scale 2, and its id.
    fn registry_with_usd() -> (CommodityRegistry, CommodityId) {
        let mut reg = CommodityRegistry::new();
        let usd = id(1);
        reg.add(usd, "USD", "US Dollar", 2).expect("the fixture must register");
        (reg, usd)
    }

    /// A registry holding `count` commodities, coded `C0`..`C{count-1}`.
    fn registry_of(count: usize) -> CommodityRegistry {
        let mut reg = CommodityRegistry::new();
        for i in 0..count {
            reg.add(id(i as u64), &format!("C{i}"), &format!("Commodity {i}"), 2)
                .expect("the fixture must register");
        }
        reg
    }

    #[test]
    fn test_registry_starts_empty() {
        let reg = CommodityRegistry::new();
        assert!(reg.get(id(0)).is_none());
        assert!(reg.get_by_code("USD").is_none());
        // `new` and `default` must agree.
        let default = CommodityRegistry::default();
        assert!(default.get_by_code("USD").is_none());
    }

    #[test]
    fn test_add_then_look_up() {
        let mut reg = CommodityRegistry::new();
        let (usd, jpy, btc) = (id(1), id(2), id(3));

        reg.add(usd, "USD", "US Dollar", 2).unwrap();
        reg.add(jpy, "JPY", "Japanese Yen", 0).unwrap();
        reg.add(btc, "BTC", "Bitcoin", 8).unwrap();

        assert_registered(&reg, usd, "USD", 2);
        assert_registered(&reg, jpy, "JPY", 0);
        assert_registered(&reg, btc, "BTC", 8);
    }

    #[test]
    fn test_lookup_misses_return_none() {
        let (reg, _usd) = registry_with_usd();

        assert!(reg.get(id(2)).is_none(), "an unregistered id must not resolve");
        assert!(reg.get_by_code("EUR").is_none(), "an unregistered code must not resolve");
    }

    #[test]
    fn test_add_rejects_duplicate_id() {
        let (mut reg, usd) = registry_with_usd();

        // Same id, entirely different commodity.
        assert!(matches!(
            reg.add(usd, "EUR", "Euro", 2),
            Err(CommodityError::DuplicateId { id }) if id == usd
        ));
        // The first registration is untouched.
        assert_registered(&reg, usd, "USD", 2);
        assert!(reg.get_by_code("EUR").is_none(), "the rejected add must not have landed");
    }

    #[test]
    fn test_add_rejects_duplicate_code() {
        let (mut reg, first) = registry_with_usd();
        let second = id(2);

        // The error points at the commodity already holding the code.
        assert!(matches!(
            reg.add(second, "USD", "United States Dollar", 2),
            Err(CommodityError::DuplicateCode { ref code, id }) if code == "USD" && id == first
        ));
        assert!(reg.get(second).is_none(), "the rejected add must not have landed");
    }

    #[test]
    fn test_duplicate_code_detection_sees_through_whitespace() {
        let (mut reg, _usd) = registry_with_usd();

        // Codes are trimmed before the uniqueness check, so padding cannot
        // sneak a second commodity past it.
        assert!(matches!(
            reg.add(id(2), "  USD  ", "US Dollar", 2),
            Err(CommodityError::DuplicateCode { .. })
        ));
    }

    /// Codes are compared byte-for-byte, so `usd` and `USD` are distinct.
    #[test]
    fn test_codes_are_case_sensitive() {
        let (mut reg, usd) = registry_with_usd();
        reg.add(id(2), "usd", "US Dollar", 2).unwrap();

        assert_eq!(reg.get_by_code("USD").unwrap().id(), usd);
        assert_eq!(reg.get_by_code("usd").unwrap().id(), id(2));
    }

    /// `add` is the only door into a `Commodity`, so every field's validation
    /// error has to survive the trip out through it.
    #[test]
    fn test_add_propagates_validation_errors() {
        let mut reg = CommodityRegistry::new();

        let err = reg.add(id(1), "", "US Dollar", 2).unwrap_err();
        assert!(matches!(err, CommodityError::CodeEmpty), "got {err}");

        let err = reg.add(id(2), "-USD", "US Dollar", 2).unwrap_err();
        assert!(matches!(err, CommodityError::CodeBadFirstChar { .. }), "got {err}");

        let err = reg.add(id(3), "US D", "US Dollar", 2).unwrap_err();
        assert!(matches!(err, CommodityError::CodeBadChar { .. }), "got {err}");

        let err = reg.add(id(4), "USD", "  ", 2).unwrap_err();
        assert!(matches!(err, CommodityError::NameEmpty), "got {err}");

        let err = reg.add(id(5), "USD", "US Dollar", 29).unwrap_err();
        assert!(matches!(err, CommodityError::ScaleTooLarge { .. }), "got {err}");
    }

    /// A rejected `add` must leave nothing behind, including when the failure
    /// happens after the code has already been validated.
    #[test]
    fn test_rejected_add_leaves_no_partial_state() {
        let mut reg = CommodityRegistry::new();
        let ghost = id(1);

        // Valid code, invalid scale: the failure lands inside `Commodity::try_new`.
        assert!(reg.add(ghost, "USD", "US Dollar", 29).is_err());
        assert!(reg.get(ghost).is_none(), "a failed add must not insert by id");
        assert!(reg.get_by_code("USD").is_none(), "a failed add must not claim the code");
        assert!(reg.is_empty(), "a failed add must not leave any commodities behind");

        // The code is still free afterwards.
        reg.add(ghost, "USD", "US Dollar", 2).unwrap();
        assert_registered(&reg, ghost, "USD", 2);
    }

    #[test]
    fn test_rename() {
        let (mut reg, usd) = registry_with_usd();
        reg.rename(usd, "United States Dollar").unwrap();

        let renamed = reg.get(usd).unwrap();
        assert_eq!(renamed.name(), "United States Dollar");
        assert_eq!(renamed.code(), "USD", "rename must not change code");
        assert_eq!(renamed.scale(), 2, "rename must not change scale");
        assert_eq!(reg.get_by_code("USD").unwrap().name(), "United States Dollar");
    }

    #[test]
    fn test_rename_rejects_unknown_id() {
        let (mut reg, _usd) = registry_with_usd();
        let missing_id = id(2);
        let err = reg.rename(missing_id, "United States Dollar").unwrap_err();
        assert!(matches!(err, CommodityError::NotFound { id } if id == missing_id));
    }

    #[test]
    fn test_rename_leaves_the_old_name_on_invalid_input() {
        let (mut reg, usd) = registry_with_usd();
        let err = reg.rename(usd, "").unwrap_err();
        assert!(matches!(err, CommodityError::NameEmpty));
        assert_eq!(reg.get(usd).unwrap().name(), "US Dollar");
    }

    #[test]
    fn test_len_and_is_empty_track_adds() {
        let mut reg = CommodityRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);

        reg.add(id(1), "USD", "US Dollar", 2).unwrap();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);

        reg.add(id(2), "EUR", "Euro", 2).unwrap();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn test_iter_is_empty_for_a_new_registry() {
        let reg = CommodityRegistry::new();
        assert_eq!(reg.iter().count(), 0);
    }

    #[test]
    fn test_iter_yields_every_commodity_exactly_once() {
        let mut reg = CommodityRegistry::new();
        let commodities = [
            (id(1), "USD", "US Dollar", 2),
            (id(2), "EUR", "Euro", 2),
            (id(3), "JPY", "Japanese Yen", 0),
        ];
        for (id, code, name, scale) in commodities {
            reg.add(id, code, name, scale).unwrap();
        }

        let mut seen = HashSet::new();
        for commodity in reg.iter() {
            let id = commodity.id();
            assert!(seen.insert(id), "commodity {id} was yielded twice");
        }
        assert_eq!(seen.len(), commodities.len(), "not all commodities were yielded");
    }

    #[test]
    fn prop_iter_agrees_with_len_and_get() {
        proptest!(|(count in 0usize..24)| {
            let reg = registry_of(count);
            prop_assert_eq!(reg.iter().count(), reg.len(), "iter count must match len");
            for commodity in reg.iter() {
                let id = commodity.id();
                prop_assert!(reg.get(id).is_some(), "iterated {id} must be retrievable by id");
                prop_assert!(
                    reg.get_by_code(commodity.code()).is_some(),
                    "iterated {id} must be retrievable by code"
                );
            }
        });
    }

    proptest! {
        /// Distinct ids with distinct codes always coexist, and every one of
        /// them stays retrievable by both keys.
        #[test]
        fn prop_distinct_commodities_all_register(count in 1usize..24) {
            let mut reg = CommodityRegistry::new();
            for i in 0..count {
                let added = reg.add(id(i as u64), &format!("C{i}"), &format!("Commodity {i}"), 2);
                prop_assert!(added.is_ok(), "commodity {i} should register, got {added:?}");
            }
            for i in 0..count {
                let by_id = reg.get(id(i as u64));
                prop_assert!(by_id.is_some(), "commodity {i} should be retrievable by id");
                prop_assert_eq!(by_id.unwrap().code(), format!("C{i}"));
                let by_code = reg.get_by_code(&format!("C{i}"));
                prop_assert!(by_code.is_some(), "commodity {i} should be retrievable by code");
            }
        }

        /// Re-adding an already registered commodity is always refused, however
        /// the second attempt differs from the first.
        #[test]
        fn prop_re_adding_always_fails(
            same_id: bool,
            same_code: bool,
            // Leading letter required: an all-whitespace name trims to empty
            // and would be refused for a reason this test is not about.
            name in "[A-Za-z][A-Za-z ]{0,19}",
            scale in 0u8..=28,
        ) {
            let (first, second) = (id(1), id(2));
            let mut reg = CommodityRegistry::new();
            reg.add(first, "USD", "US Dollar", 2).unwrap();

            let re_id = if same_id { first } else { second };
            let re_code = if same_code { "USD" } else { "EUR" };
            let re_added = reg.add(re_id, re_code, &name, scale);

            if same_id || same_code {
                prop_assert!(
                    re_added.is_err(),
                    "re-adding (same_id: {same_id}, same_code: {same_code}) must fail"
                );
                // The original survives untouched either way.
                let original = reg.get(first).unwrap();
                prop_assert_eq!(original.code(), "USD");
                prop_assert_eq!(original.name(), "US Dollar");
                prop_assert_eq!(original.scale(), 2);
            } else {
                prop_assert!(re_added.is_ok(), "a fresh id and code must register, got {re_added:?}");
            }
        }
    }
}
