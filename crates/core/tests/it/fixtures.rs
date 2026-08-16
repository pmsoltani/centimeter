//! Setup shared by the integration modules.
//!
//! These stand in for the caller: core mints no ids and seeds no commodities
//! or accounts, so every consumer has to do this much before it can record
//! anything.

use centimeter_core::{
    ChartOfAccounts, CommodityId, CommodityRegistry, Id, Identifiable, RootSpec, RootsSpec,
};
use uuid::{NoContext, Timestamp, Uuid};

/// Mints a fresh id the way a caller would.
pub(crate) fn new_id<T: Identifiable>() -> Id<T> {
    let ts = Timestamp::now(NoContext);
    Id::from_uuid(Uuid::new_v7(ts)).expect("new_v7 must produce a v7 uuid")
}

/// A registry holding USD at scale 2 and JPY at scale 0, with their ids.
///
/// Two scales rather than one, so a test can tell "the commodity's scale" from
/// "two decimal places" by accident.
pub(crate) fn registry() -> (CommodityRegistry, CommodityId, CommodityId) {
    let mut registry = CommodityRegistry::new();
    let (usd, jpy) = (new_id(), new_id());
    registry.add(usd, "USD", "US Dollar", 2).expect("USD must register");
    registry.add(jpy, "JPY", "Japanese Yen", 0).expect("JPY must register");
    (registry, usd, jpy)
}

/// A chart holding only its five roots.
///
/// The root ids are not returned: a consumer reaches them through
/// [`ChartOfAccounts::roots`], and a test that does the same proves the
/// accessor works.
pub(crate) fn chart() -> ChartOfAccounts {
    ChartOfAccounts::try_new(RootsSpec {
        asset: RootSpec { id: new_id(), name: "Assets" },
        liability: RootSpec { id: new_id(), name: "Liabilities" },
        equity: RootSpec { id: new_id(), name: "Equity" },
        income: RootSpec { id: new_id(), name: "Income" },
        expense: RootSpec { id: new_id(), name: "Expenses" },
    })
    .expect("the five roots must build a chart")
}
