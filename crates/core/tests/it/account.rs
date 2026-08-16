//! The chart of accounts as a consumer meets it: five roots in, a typed tree
//! out.

use centimeter_core::{
    Account, AccountError, AccountId, AccountType, ChartOfAccounts, Error, RootSpec, RootsSpec,
};

use crate::fixtures::{chart, new_id};

/// The names of an account's children, sorted. The chart yields children in an
/// arbitrary order, so nothing outside may depend on it.
fn child_names(coa: &ChartOfAccounts, id: AccountId) -> Vec<&str> {
    let mut names: Vec<&str> =
        coa.children_of(id).expect("the parent must exist").map(Account::name).collect();
    names.sort_unstable();
    names
}

#[test]
fn a_chart_is_built_from_five_named_roots() {
    let coa = chart();
    let roots = coa.roots();

    let expected = [
        (roots.asset(), "Assets", AccountType::Asset),
        (roots.liability(), "Liabilities", AccountType::Liability),
        (roots.equity(), "Equity", AccountType::Equity),
        (roots.income(), "Income", AccountType::Income),
        (roots.expense(), "Expenses", AccountType::Expense),
    ];
    for (id, name, element) in expected {
        let account = coa.get(id).expect("a seeded root must be readable");
        assert_eq!(account.name(), name);
        assert_eq!(account.parent_id(), None, "a root has no parent");
        assert!(account.is_root());
        assert_eq!(coa.type_of(id).expect("a root has an element"), element);
    }
}

/// Roots are ordinary accounts wearing fixed roles. The element travels with
/// the slot, never with the name, so a French ledger is the same structure.
#[test]
fn roots_may_be_named_anything() {
    let asset = new_id();
    let coa = ChartOfAccounts::try_new(RootsSpec {
        asset: RootSpec { id: asset, name: "Actif" },
        liability: RootSpec { id: new_id(), name: "Passif" },
        equity: RootSpec { id: new_id(), name: "Capitaux propres" },
        income: RootSpec { id: new_id(), name: "Produits" },
        expense: RootSpec { id: new_id(), name: "Charges" },
    })
    .expect("names are free");

    assert_eq!(coa.get(asset).expect("seeded").name(), "Actif");
    assert_eq!(coa.type_of(asset).expect("still an asset root"), AccountType::Asset);
}

/// The headline claim of ADR-0009, asserted from outside: an account's element
/// comes from its root, however deep it sits, and no account carries one.
#[test]
fn an_account_inherits_its_element_from_its_root() {
    let mut coa = chart();
    let expense = coa.roots().expense();

    let (operating, travel, flights) = (new_id(), new_id(), new_id());
    coa.add(operating, "Operating", expense).expect("Operating must add");
    coa.add(travel, "Travel", operating).expect("Travel must add");
    coa.add(flights, "Flights", travel).expect("Flights must add");

    for id in [operating, travel, flights] {
        assert_eq!(coa.type_of(id).expect("every account has an element"), AccountType::Expense);
    }
    assert_eq!(coa.root_of(flights).expect("every account has a root").id(), expense);
    assert_eq!(coa.level_of(flights).expect("three levels down"), 3);
    assert_eq!(
        coa.path_of(flights).expect("a path to the root"),
        ["Expenses", "Operating", "Travel", "Flights"]
    );
}

#[test]
fn children_are_the_direct_ones_only() {
    let mut coa = chart();
    let asset = coa.roots().asset();

    let (current, savings, bank) = (new_id(), new_id(), new_id());
    coa.add(current, "Current", asset).expect("Current must add");
    coa.add(savings, "Savings", asset).expect("Savings must add");
    coa.add(bank, "Bank", current).expect("Bank must add");

    assert_eq!(child_names(&coa, asset), ["Current", "Savings"], "Bank is a grandchild");
    assert_eq!(child_names(&coa, current), ["Bank"]);
    assert!(child_names(&coa, bank).is_empty(), "a leaf has no children");
}

/// Names are unique per parent, not per chart. "Bank" under Assets and "Bank"
/// under Liabilities are two accounts that happen to share a label.
#[test]
fn sibling_names_are_unique_but_cousins_may_share() {
    let mut coa = chart();
    let (asset, liability) = (coa.roots().asset(), coa.roots().liability());

    let first = new_id();
    coa.add(first, "Bank", asset).expect("the first Bank must add");
    coa.add(new_id(), "Bank", liability).expect("a different parent is a different sibling set");

    let err = coa.add(new_id(), "Bank", asset).expect_err("a second Bank under Assets");
    assert!(
        matches!(err, AccountError::DuplicateName { id, .. } if id == first),
        "the error must name the account already holding it, got {err}"
    );
}

#[test]
fn an_account_can_be_renamed_but_keeps_its_identity() {
    let mut coa = chart();
    let bank = new_id();
    coa.add(bank, "Bank", coa.roots().asset()).expect("Bank must add");

    let before = coa.get(bank).expect("just added").clone();
    coa.rename(bank, "Current Account").expect("the name is free");
    let after = coa.get(bank).expect("still there");

    assert_eq!(after.name(), "Current Account");
    assert_eq!(&before, after, "a rename does not produce a different account");
    assert_eq!(after.id(), bank);
}

/// Reparenting reorganizes presentation inside one element. Crossing elements
/// would restate history, so it is refused.
#[test]
fn reparenting_moves_a_subtree_but_never_changes_its_element() {
    let mut coa = chart();
    let (asset, expense) = (coa.roots().asset(), coa.roots().expense());

    let (current, savings, bank) = (new_id(), new_id(), new_id());
    coa.add(current, "Current", asset).expect("Current must add");
    coa.add(savings, "Savings", asset).expect("Savings must add");
    coa.add(bank, "Bank", current).expect("Bank must add");

    coa.reparent(bank, savings).expect("both sides are assets");
    assert_eq!(coa.path_of(bank).expect("a path"), ["Assets", "Savings", "Bank"]);
    assert_eq!(coa.type_of(bank).expect("still an asset"), AccountType::Asset);

    let err = coa.reparent(bank, expense).expect_err("assets are not expenses");
    assert!(matches!(err, AccountError::ReparentTypeMismatch { .. }), "got {err}");
    assert_eq!(coa.type_of(bank).expect("unmoved"), AccountType::Asset);
}

#[test]
fn the_five_roots_are_fixed_for_the_life_of_the_ledger() {
    let mut coa = chart();
    let (asset, liability) = (coa.roots().asset(), coa.roots().liability());

    let err = coa.reparent(asset, liability).expect_err("a root cannot be moved");
    assert!(matches!(err, AccountError::CannotReparentRoot { id } if id == asset), "got {err}");

    // Nor can a sixth root be created: reparenting takes an id, not an option,
    // so there is no way to ask for "no parent".
    assert!(coa.get(asset).expect("still there").is_root());
}

#[test]
fn an_account_cannot_become_its_own_ancestor() {
    let mut coa = chart();
    let (parent, child) = (new_id(), new_id());
    coa.add(parent, "Current", coa.roots().asset()).expect("Current must add");
    coa.add(child, "Bank", parent).expect("Bank must add");

    for destination in [parent, child] {
        let err = coa.reparent(parent, destination).expect_err("that closes a loop");
        assert!(matches!(err, AccountError::CycleDetected { .. }), "got {err}");
    }
}

#[test]
fn an_account_must_be_added_under_a_parent_that_exists() {
    let mut coa = chart();
    let ghost = new_id();

    let err = coa.add(new_id(), "Bank", ghost).expect_err("no such parent");
    assert!(
        matches!(err, AccountError::ParentNotFound { parent_id } if parent_id == ghost),
        "got {err}"
    );
}

#[test]
fn account_ids_are_prefixed_and_round_trip_as_strings() {
    let coa = chart();
    let asset = coa.roots().asset();

    let rendered = asset.to_string();
    assert!(rendered.starts_with("acc_"), "got {rendered}");
    assert_eq!(rendered.parse::<AccountId>().expect("its own form must parse"), asset);
}

/// The composing root lets a caller bubble an account failure with `?`
/// alongside any other domain error.
#[test]
fn account_errors_bubble_into_the_root_error() {
    fn add_twice(coa: &mut ChartOfAccounts, id: AccountId) -> Result<(), Error> {
        let asset = coa.roots().asset();
        coa.add(id, "Bank", asset)?;
        coa.add(id, "Cash", asset)?;
        Ok(())
    }

    let mut coa = chart();
    let bank = new_id();
    let err = add_twice(&mut coa, bank).expect_err("the second add reuses an id");
    assert!(matches!(err, Error::Account(AccountError::DuplicateId { .. })), "got {err}");

    // `#[error(transparent)]` means the root reads exactly like the domain error.
    let domain = AccountError::DuplicateId { id: bank };
    assert_eq!(err.to_string(), domain.to_string());
}
