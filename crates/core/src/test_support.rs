//! Test support utilities for the `core` crate.

use uuid::Uuid;

use crate::{CommodityId, CommodityRegistry, Decimal, Quantity};

/// Mints a distinct [`CommodityId`] from `seed`, reproducibly.
///
/// Core never mints IDs itself ([`Id`] has no `new`), so tests stand in for
/// the caller that normally supplies them. `Uuid::new_v7` is deliberately
/// avoided here: it randomizes 74 of its bits, so the same seed would not
/// yield the same id twice and lookups could not be asserted against it.
pub(crate) fn id(seed: u64) -> CommodityId {
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&seed.to_be_bytes());
    bytes[8..].copy_from_slice(&seed.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0F) | 0x70; // version 7
    bytes[8] = (bytes[8] & 0x3F) | 0x80; // RFC 4122 variant
    CommodityId::from_uuid(Uuid::from_bytes(bytes)).expect("bytes are stamped as v7")
}

/// Returns a commodity registry with a few commodities for testing.
pub(crate) fn registry() -> CommodityRegistry {
    let mut registry = CommodityRegistry::new();
    registry.add(id(0), "USD", "US Dollar", 2).expect("Failed to add USD");
    registry.add(id(1), "JPY", "Japanese Yen", 0).expect("Failed to add JPY");
    registry.add(id(2), "BTC", "Bitcoin", 8).expect("Failed to add BTC");
    registry.add(id(3), "HYP", "Hypothetical", 28).expect("Failed to add HYP");
    registry
}

/// Returns a quantity with the given number and commodity code, using the
/// given registry to look up the commodity.
pub(crate) fn qty(number: Decimal, code: &str, registry: &CommodityRegistry) -> Quantity {
    let commodity = registry.get_by_code(code).expect("Failed to find commodity");
    Quantity::try_new(number, commodity).expect("Failed to create quantity")
}
