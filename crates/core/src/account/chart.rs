//! The account tree and the rules that keep it valid.
//!
//! This is the mutating side of [`ChartOfAccounts`]: creating the chart, adding
//! accounts, renaming them, and moving them. All tree invariants are enforced
//! here, so this is the place to look when checking how the chart can change.
//!
//! The tree maintains these invariants:
//!
//! - Exactly five accounts have no parent. These are the accounts in
//!   [`RootAccounts`].
//! - Every other account has a parent that exists. This means every account
//!   belongs to exactly one root and therefore has exactly one element.
//! - Siblings cannot have the same name. The five root accounts are siblings
//!   too.
//! - An account cannot change its element or become its own ancestor.
//! - Accounts are never removed.

use std::collections::HashMap;

use super::{Account, AccountError, AccountId, AccountType, RootAccounts, RootsSpec};

mod query;

/// The tree of every account in one ledger.
///
/// The chart is the sole owner and constructor of an [`Account`], so holding
/// one is proof it was validated and placed.
///
/// There is no way to remove an account. A posting references an account for
/// the life of the ledger, and the chart cannot prove an account is
/// unreferenced, so the only operation is `add`.
///
/// # Examples
///
/// ```
/// # use centimeter_core::{AccountId, AccountType, ChartOfAccounts, RootSpec, RootsSpec};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let new_id = || AccountId::from_uuid(uuid::Uuid::now_v7()).unwrap();
/// # let (asset, liability, equity, income, expense) =
/// #     (new_id(), new_id(), new_id(), new_id(), new_id());
/// // Core mints no ids and names no roots, so the caller supplies both.
/// let mut chart = ChartOfAccounts::try_new(RootsSpec {
///     asset: RootSpec { id: asset, name: "Assets" },
///     liability: RootSpec { id: liability, name: "Liabilities" },
///     equity: RootSpec { id: equity, name: "Equity" },
///     income: RootSpec { id: income, name: "Income" },
///     expense: RootSpec { id: expense, name: "Expenses" },
/// })?;
///
/// let (current, bank) = (new_id(), new_id());
/// chart.add(current, "Current", asset)?;
/// chart.add(bank, "Bank", current)?;
///
/// // The element comes from the root, not from a field on the account.
/// assert_eq!(chart.type_of(bank)?, AccountType::Asset);
/// assert_eq!(chart.path_of(bank)?, ["Assets", "Current", "Bank"]);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ChartOfAccounts {
    accounts: HashMap<AccountId, Account>,
    roots: RootAccounts,
}

impl ChartOfAccounts {
    /// Maximum number of ancestors a tree operation will traverse.
    ///
    /// This is not a limit on the depth of a valid chart. Cycles are prevented
    /// by [`reparent`](Self::reparent), so a valid chart should never reach
    /// this limit. It acts as a safety check in case the tree is corrupted or
    /// that invariant is ever violated.
    const MAX_TREE_DEPTH: usize = 50;

    /// Estimated number of levels in a typical account path.
    /// Used to pre-allocate memory and prevent vector reallocation.
    const EXPECTED_PATH_LENGTH: usize = 5;

    /// Builds a chart from its five roots.
    ///
    /// The roots are ordinary accounts, distinguished only by having no parent
    /// and by being listed in [`RootAccounts`]. They may be named anything.
    ///
    /// # Errors
    /// - [`DuplicateRoot`](AccountError::DuplicateRoot) if one id is given for
    ///   two elements.
    /// - [`DuplicateName`](AccountError::DuplicateName) if two roots share a
    ///   name.
    /// - A name error if any root name is empty, too long, or has a control
    ///   character. See [`Account`].
    pub fn try_new(spec: RootsSpec) -> Result<Self, AccountError> {
        let roots = RootAccounts::try_new(
            spec.asset.id,
            spec.liability.id,
            spec.equity.id,
            spec.income.id,
            spec.expense.id,
        )?;
        let mut coa = Self { accounts: HashMap::new(), roots };
        for root in [spec.asset, spec.liability, spec.equity, spec.income, spec.expense] {
            coa.insert(root.id, root.name, None)?;
        }
        Ok(coa)
    }

    /// Adds an account under an existing parent.
    ///
    /// # Errors
    /// Checks run in this order:
    /// 1. [`ParentNotFound`](AccountError::ParentNotFound) if `parent_id`
    ///    does not exist.
    /// 2. [`DuplicateId`](AccountError::DuplicateId) if `id` is taken.
    /// 3. A name error if `name` is empty, too long, or has a control
    ///    character. See [`Account`].
    /// 4. [`DuplicateName`](AccountError::DuplicateName) if a sibling already
    ///    holds `name`, naming the sibling that does.
    pub fn add(
        &mut self,
        id: AccountId,
        name: &str,
        parent_id: AccountId,
    ) -> Result<(), AccountError> {
        self.get(parent_id).ok_or(AccountError::ParentNotFound { parent_id })?;
        self.insert(id, name, Some(parent_id))
    }

    /// Changes an account's display name.
    ///
    /// The name is the only part of an account allowed to change. The account
    /// keeps its id and position in the tree, so renaming it does not affect
    /// existing postings or reports.
    ///
    /// # Errors
    /// - [`NotFound`](AccountError::NotFound) if no account has `id`.
    /// - A name error if `name` is empty, too long, or contains a control
    ///   character. See [`Account`].
    /// - [`DuplicateName`](AccountError::DuplicateName) if a sibling already
    ///   has the given name. Renaming an account to the name it already has is
    ///   not a collision with itself.
    pub fn rename(&mut self, id: AccountId, name: &str) -> Result<(), AccountError> {
        // Copy the parent out before taking the mutable borrow below: the two
        // cannot both be live, and `Option<AccountId>` is `Copy`.
        let parent_id = self.get(id).ok_or(AccountError::NotFound { id })?.parent_id();
        // Validate before comparing, so "  Bank  " cannot pass the uniqueness
        // check and then trim into a duplicate.
        let name = Account::validate_name(name)?;
        self.check_name(Some(id), &name, parent_id)?;
        let account = self.accounts.get_mut(&id).ok_or(AccountError::NotFound { id })?;
        account.set_name(name);
        Ok(())
    }

    /// Moves an account and all its children under a new parent.
    ///
    /// Reparenting reorganizes presentation within one element. It cannot
    /// change an account's element.
    ///
    /// # Errors
    /// Checks run in this order:
    /// 1. [`NotFound`](AccountError::NotFound) if no account has `id`.
    /// 2. [`CannotReparentRoot`](AccountError::CannotReparentRoot) if `id` is
    ///    a root. The five roots are fixed for the life of the ledger.
    /// 3. [`ParentNotFound`](AccountError::ParentNotFound) if `parent_id`
    ///    does not exist.
    /// 4. [`ReparentTypeMismatch`](AccountError::ReparentTypeMismatch) if the
    ///    new parent has a different element.
    /// 5. [`CycleDetected`](AccountError::CycleDetected) if `parent_id` is
    ///    `id` itself or one of its descendants.
    /// 6. [`DuplicateName`](AccountError::DuplicateName) if a child of the new
    ///    parent already holds this account's name.
    pub fn reparent(&mut self, id: AccountId, parent_id: AccountId) -> Result<(), AccountError> {
        if self.is_root(id)? {
            return Err(AccountError::CannotReparentRoot { id });
        }
        self.get(parent_id).ok_or(AccountError::ParentNotFound { parent_id })?;
        let (first, second) = (self.type_of(id)?, self.type_of(parent_id)?);
        if first != second {
            return Err(AccountError::ReparentTypeMismatch { id, parent_id, first, second });
        }
        if self.makes_cycle(id, parent_id)? {
            return Err(AccountError::CycleDetected { id, parent_id });
        }
        // The name has to clear the destination's children.
        let name = self.get(id).ok_or(AccountError::NotFound { id })?.name();
        self.check_name(Some(id), name, Some(parent_id))?;
        let account = self.accounts.get_mut(&id).ok_or(AccountError::NotFound { id })?;
        account.set_parent_id(parent_id);
        Ok(())
    }

    /// Returns an error if another child of `parent_id` already has `name`.
    ///
    /// `parent_id` is `None` for roots, making those five accounts siblings.
    /// `id` is the account being renamed or moved in place, which must not be
    /// treated as colliding with itself.
    ///
    /// `name` must already have been through [`Account::validate_name`], since
    /// stored names are trimmed.
    fn check_name(
        &self,
        id: Option<AccountId>,
        name: &str,
        parent_id: Option<AccountId>,
    ) -> Result<(), AccountError> {
        let clash = self
            .accounts
            .values()
            .find(|a| a.parent_id() == parent_id && Some(a.id()) != id && a.name() == name);
        match clash {
            Some(a) => Err(AccountError::DuplicateName { id: a.id(), name: name.to_string() }),
            None => Ok(()),
        }
    }

    /// Validates and inserts one account. The only path into `self.accounts`.
    fn insert(
        &mut self,
        id: AccountId,
        name: &str,
        parent_id: Option<AccountId>,
    ) -> Result<(), AccountError> {
        if self.accounts.contains_key(&id) {
            return Err(AccountError::DuplicateId { id });
        }
        let name = Account::validate_name(name)?;
        self.check_name(None, &name, parent_id)?;
        self.accounts.insert(id, Account::new(id, name, parent_id));
        Ok(())
    }

    /// Checks if `id` is an ancestor of `parent_id`. If it is, moving `id`
    /// under `parent_id` would create a circular loop.
    ///
    /// # Errors
    /// - [`MaxDepthExceeded`](AccountError::MaxDepthExceeded) if the walk
    ///   above `parent_id` runs past [`MAX_TREE_DEPTH`](Self::MAX_TREE_DEPTH),
    ///   which means the tree is already corrupt.
    fn makes_cycle(&self, id: AccountId, parent_id: AccountId) -> Result<bool, AccountError> {
        let mut current = Some(parent_id);
        let mut depth = 0;
        while let Some(current_id) = current {
            if current_id == id {
                return Ok(true);
            }
            depth += 1;
            if depth > Self::MAX_TREE_DEPTH {
                return Err(AccountError::MaxDepthExceeded {
                    id: parent_id,
                    max: Self::MAX_TREE_DEPTH,
                });
            }
            current = self.get(current_id).and_then(Account::parent_id);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    use crate::account::RootSpec;
    use crate::test_support::{chart, id};

    /// Asserts the chart holds `id` under `parent`, with the given name.
    fn assert_placed(coa: &ChartOfAccounts, id: AccountId, name: &str, parent: Option<AccountId>) {
        let account = coa.get(id).unwrap_or_else(|| panic!("{id} should be in the chart"));
        assert_eq!(account.name(), name);
        assert_eq!(account.parent_id(), parent);
    }

    /// The names of an account's children, sorted, since the order a chart
    /// yields children in is arbitrary.
    fn child_names(coa: &ChartOfAccounts, id: AccountId) -> Vec<String> {
        let mut names: Vec<String> =
            coa.children_of(id).expect("parent must exist").map(|a| a.name().to_string()).collect();
        names.sort();
        names
    }

    // Construction

    #[test]
    fn test_try_new_seeds_five_roots() {
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
            assert_placed(&coa, id, name, None);
            assert!(coa.is_root(id).expect("a root must be in the chart"));
            assert_eq!(coa.type_of(id).expect("a root has an element"), element);
        }
    }

    #[test]
    fn test_try_new_rejects_duplicate_root_ids() {
        let dup = id(1);
        let built = ChartOfAccounts::try_new(RootsSpec {
            asset: RootSpec { id: dup, name: "Assets" },
            liability: RootSpec { id: dup, name: "Liabilities" },
            equity: RootSpec { id: id(3), name: "Equity" },
            income: RootSpec { id: id(4), name: "Income" },
            expense: RootSpec { id: id(5), name: "Expenses" },
        });
        assert!(matches!(built, Err(AccountError::DuplicateRoot { id, .. }) if id == dup));
    }

    /// The roots are siblings of one another, so the sibling-name rule has to
    /// apply to them too, or the constructor would build a state `rename`
    /// refuses.
    #[test]
    fn test_try_new_rejects_duplicate_root_names() {
        let built = ChartOfAccounts::try_new(RootsSpec {
            asset: RootSpec { id: id(1), name: "Accounts" },
            liability: RootSpec { id: id(2), name: "Accounts" },
            equity: RootSpec { id: id(3), name: "Equity" },
            income: RootSpec { id: id(4), name: "Income" },
            expense: RootSpec { id: id(5), name: "Expenses" },
        });
        assert!(
            matches!(built, Err(AccountError::DuplicateName { ref name, .. }) if name == "Accounts"),
            "got {built:?}"
        );
    }

    #[test]
    fn test_try_new_propagates_name_errors() {
        let built = ChartOfAccounts::try_new(RootsSpec {
            asset: RootSpec { id: id(1), name: "   " },
            liability: RootSpec { id: id(2), name: "Liabilities" },
            equity: RootSpec { id: id(3), name: "Equity" },
            income: RootSpec { id: id(4), name: "Income" },
            expense: RootSpec { id: id(5), name: "Expenses" },
        });
        assert!(matches!(built, Err(AccountError::NameEmpty)), "got {built:?}");
    }

    // add

    #[test]
    fn test_add_places_an_account_under_its_parent() {
        let mut coa = chart();
        let asset = coa.roots().asset();

        coa.add(id(10), "Current", asset).unwrap();
        coa.add(id(11), "Bank", id(10)).unwrap();

        assert_placed(&coa, id(10), "Current", Some(asset));
        assert_placed(&coa, id(11), "Bank", Some(id(10)));
        assert_eq!(child_names(&coa, asset), ["Current"]);
        assert_eq!(child_names(&coa, id(10)), ["Bank"]);
    }

    #[test]
    fn test_add_trims_the_name() {
        let mut coa = chart();
        coa.add(id(10), "  Bank \n", coa.roots().asset()).unwrap();
        assert_eq!(coa.get(id(10)).unwrap().name(), "Bank");
    }

    #[test]
    fn test_add_rejects_a_taken_id() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Bank", asset).unwrap();

        let err = coa.add(id(10), "Cash", asset).unwrap_err();
        assert!(matches!(err, AccountError::DuplicateId { id: got } if got == id(10)), "got {err}");
        // A root's id is taken too, even though it was never added by `add`.
        let err = coa.add(asset, "Assets Again", asset).unwrap_err();
        assert!(matches!(err, AccountError::DuplicateId { .. }), "got {err}");
    }

    #[test]
    fn test_add_rejects_a_missing_parent() {
        let mut coa = chart();
        let ghost = id(99);
        let err = coa.add(id(10), "Bank", ghost).unwrap_err();
        assert!(
            matches!(err, AccountError::ParentNotFound { parent_id } if parent_id == ghost),
            "got {err}"
        );
        assert!(coa.get(id(10)).is_none(), "the rejected add must not have landed");
    }

    /// The rule is per parent, not per chart: "Bank" under Assets and "Bank"
    /// under Liabilities are different accounts with the same label.
    #[test]
    fn test_add_scopes_name_uniqueness_to_siblings() {
        let mut coa = chart();
        let (asset, liability) = (coa.roots().asset(), coa.roots().liability());

        coa.add(id(10), "Bank", asset).unwrap();
        coa.add(id(11), "Bank", liability).expect("a different parent is a different sibling set");

        let err = coa.add(id(12), "Bank", asset).unwrap_err();
        let AccountError::DuplicateName { id: holder, name } = err else {
            panic!("expected DuplicateName, got {err}");
        };
        assert_eq!(holder, id(10), "the error must name the account holding the name");
        assert_eq!(name, "Bank");
        assert!(coa.get(id(12)).is_none(), "the rejected add must not have landed");
    }

    /// Whitespace must not smuggle a duplicate past the check and then trim
    /// into one.
    #[test]
    fn test_add_sees_through_whitespace() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Bank", asset).unwrap();

        let err = coa.add(id(11), "  Bank  ", asset).unwrap_err();
        assert!(matches!(err, AccountError::DuplicateName { .. }), "got {err}");
    }

    #[test]
    fn test_add_propagates_name_errors() {
        let mut coa = chart();
        let asset = coa.roots().asset();

        assert!(matches!(coa.add(id(10), "", asset), Err(AccountError::NameEmpty)));
        assert!(matches!(
            coa.add(id(11), "a\rb", asset),
            Err(AccountError::NameBadChar { index: 1, .. })
        ));
        let over = "a".repeat(Account::MAX_NAME_LENGTH + 1);
        assert!(matches!(coa.add(id(12), &over, asset), Err(AccountError::NameTooLong { .. })));
    }

    // rename

    #[test]
    fn test_rename_replaces_the_name_and_nothing_else() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Bank", asset).unwrap();

        coa.rename(id(10), "  Current Account ").unwrap();
        assert_placed(&coa, id(10), "Current Account", Some(asset));
    }

    #[test]
    fn test_rename_allows_an_account_its_own_name() {
        let mut coa = chart();
        coa.add(id(10), "Bank", coa.roots().asset()).unwrap();
        coa.rename(id(10), "Bank").expect("an account cannot collide with itself");
        assert_eq!(coa.get(id(10)).unwrap().name(), "Bank");
    }

    #[test]
    fn test_rename_refuses_a_sibling_name() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Bank", asset).unwrap();
        coa.add(id(11), "Cash", asset).unwrap();

        for attempt in ["Bank", "  Bank  "] {
            let err = coa.rename(id(11), attempt).unwrap_err();
            assert!(
                matches!(err, AccountError::DuplicateName { id: holder, .. } if holder == id(10)),
                "{attempt:?} should collide, got {err}"
            );
        }
        assert_eq!(coa.get(id(11)).unwrap().name(), "Cash", "the old name must survive");
    }

    /// The five roots share a `None` parent, so they are siblings and the rule
    /// binds them as well.
    #[test]
    fn test_rename_treats_the_roots_as_siblings() {
        let mut coa = chart();
        let err = coa.rename(coa.roots().asset(), "Equity").unwrap_err();
        assert!(
            matches!(err, AccountError::DuplicateName { id: holder, .. }
                if holder == coa.roots().equity()),
            "got {err}"
        );
        // A root may still be renamed to anything free.
        coa.rename(coa.roots().asset(), "Actif").expect("roots are ordinary accounts otherwise");
        assert_eq!(coa.get(coa.roots().asset()).unwrap().name(), "Actif");
    }

    #[test]
    fn test_rename_rejects_an_unknown_id() {
        let mut coa = chart();
        let ghost = id(99);
        let err = coa.rename(ghost, "Assets").unwrap_err();
        assert!(
            matches!(err, AccountError::NotFound { id } if id == ghost),
            "a missing account must not be mistaken for a root, got {err}"
        );
    }

    #[test]
    fn test_rename_leaves_the_old_name_on_invalid_input() {
        let mut coa = chart();
        coa.add(id(10), "Bank", coa.roots().asset()).unwrap();
        assert!(matches!(coa.rename(id(10), "  "), Err(AccountError::NameEmpty)));
        assert_eq!(coa.get(id(10)).unwrap().name(), "Bank");
    }

    // reparent

    #[test]
    fn test_reparent_moves_an_account_and_its_subtree() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Current", asset).unwrap();
        coa.add(id(11), "Savings", asset).unwrap();
        coa.add(id(12), "Bank", id(10)).unwrap();
        coa.add(id(13), "Card", id(12)).unwrap();

        coa.reparent(id(12), id(11)).unwrap();

        assert_eq!(coa.get(id(12)).unwrap().parent_id(), Some(id(11)));
        // The subtree travels with it: `Card` never moved, but its path did.
        assert_eq!(coa.get(id(13)).unwrap().parent_id(), Some(id(12)));
        assert_eq!(coa.path_of(id(13)).unwrap(), ["Assets", "Savings", "Bank", "Card"]);
        assert_eq!(child_names(&coa, id(10)), Vec::<String>::new());
    }

    #[test]
    fn test_reparent_refuses_to_move_a_root() {
        let mut coa = chart();
        let (asset, liability) = (coa.roots().asset(), coa.roots().liability());
        let err = coa.reparent(asset, liability).unwrap_err();
        assert!(matches!(err, AccountError::CannotReparentRoot { id } if id == asset), "got {err}");
        assert!(coa.get(asset).unwrap().is_root(), "the root must still be a root");
    }

    #[test]
    fn test_reparent_rejects_unknown_ids() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Bank", asset).unwrap();
        let ghost = id(99);

        assert!(matches!(coa.reparent(ghost, asset), Err(AccountError::NotFound { .. })));
        let err = coa.reparent(id(10), ghost).unwrap_err();
        assert!(
            matches!(err, AccountError::ParentNotFound { parent_id } if parent_id == ghost),
            "got {err}"
        );
        assert_eq!(coa.get(id(10)).unwrap().parent_id(), Some(asset), "nothing may have moved");
    }

    /// Moving an account across elements would restate every report it has
    /// appeared in, so it is refused rather than silently allowed.
    #[test]
    fn test_reparent_refuses_to_change_element() {
        let mut coa = chart();
        let (asset, expense) = (coa.roots().asset(), coa.roots().expense());
        coa.add(id(10), "Bank", asset).unwrap();

        let err = coa.reparent(id(10), expense).unwrap_err();
        let AccountError::ReparentTypeMismatch { first, second, .. } = err else {
            panic!("expected ReparentTypeMismatch, got {err}");
        };
        assert_eq!(first, AccountType::Asset);
        assert_eq!(second, AccountType::Expense);
        assert_eq!(coa.type_of(id(10)).unwrap(), AccountType::Asset);
    }

    #[test]
    fn test_reparent_refuses_a_cycle() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Current", asset).unwrap();
        coa.add(id(11), "Bank", id(10)).unwrap();
        coa.add(id(12), "Card", id(11)).unwrap();

        // Under itself, under a child, and under a grandchild.
        for parent in [id(10), id(11), id(12)] {
            let err = coa.reparent(id(10), parent).unwrap_err();
            assert!(
                matches!(err, AccountError::CycleDetected { id: got, parent_id }
                    if got == id(10) && parent_id == parent),
                "moving under {parent} should be refused, got {err}"
            );
        }
        assert_eq!(coa.get(id(10)).unwrap().parent_id(), Some(asset));
    }

    /// The destination's children are the sibling set that matters, not the
    /// ones the account is leaving behind.
    #[test]
    fn test_reparent_refuses_a_name_clash_at_the_destination() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Current", asset).unwrap();
        coa.add(id(11), "Savings", asset).unwrap();
        coa.add(id(12), "Bank", id(10)).unwrap();
        coa.add(id(13), "Bank", id(11)).unwrap();

        let err = coa.reparent(id(12), id(11)).unwrap_err();
        assert!(
            matches!(err, AccountError::DuplicateName { id: holder, .. } if holder == id(13)),
            "got {err}"
        );
        assert_eq!(coa.get(id(12)).unwrap().parent_id(), Some(id(10)), "nothing may have moved");
        assert_eq!(child_names(&coa, id(11)), ["Bank"]);
    }

    /// Reparenting an account to the parent it already has is a no-op, not a
    /// self-collision.
    #[test]
    fn test_reparent_to_the_same_parent_is_allowed() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Bank", asset).unwrap();
        coa.reparent(id(10), asset).expect("a no-op move must be accepted");
        assert_eq!(coa.get(id(10)).unwrap().parent_id(), Some(asset));
    }

    // Private helpers

    #[test]
    fn test_makes_cycle_sees_every_ancestor() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Current", asset).unwrap();
        coa.add(id(11), "Bank", id(10)).unwrap();
        coa.add(id(12), "Sibling", asset).unwrap();

        assert!(coa.makes_cycle(id(10), id(10)).unwrap(), "an account is its own ancestor");
        assert!(coa.makes_cycle(id(10), id(11)).unwrap(), "a descendant closes the loop");
        assert!(!coa.makes_cycle(id(10), id(12)).unwrap(), "a cousin does not");
        assert!(!coa.makes_cycle(id(10), asset).unwrap(), "its current parent does not");
    }

    #[test]
    fn test_check_name_excludes_only_the_named_account() {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Bank", asset).unwrap();

        assert!(coa.check_name(None, "Bank", Some(asset)).is_err(), "a new account collides");
        assert!(coa.check_name(Some(id(10)), "Bank", Some(asset)).is_ok(), "it is itself");
        assert!(coa.check_name(None, "Bank", Some(coa.roots().equity())).is_ok(), "other parent");
        assert!(coa.check_name(None, "Assets", None).is_err(), "roots are siblings");
    }

    proptest! {
        /// However deep an account sits, it reports the element of the root it
        /// descends from, and it descends from exactly one. The element is
        /// known here independently, from the slot the root was seeded into.
        #[test]
        fn prop_every_account_inherits_its_root_element(depth in 1usize..8) {
            let mut coa = chart();
            let roots = coa.roots();
            let cases = [
                (roots.asset(), AccountType::Asset),
                (roots.liability(), AccountType::Liability),
                (roots.equity(), AccountType::Equity),
                (roots.income(), AccountType::Income),
                (roots.expense(), AccountType::Expense),
            ];

            // One chain per element, so every level of every element is covered.
            for (slot, (root, element)) in cases.into_iter().enumerate() {
                let mut parent = root;
                for level in 0..depth {
                    let child = id((10 + slot * 100 + level) as u64);
                    coa.add(child, &format!("Level {level}"), parent)
                        .expect("each add must succeed");

                    prop_assert_eq!(coa.type_of(child).unwrap(), element);
                    prop_assert_eq!(coa.root_of(child).unwrap().id(), root);
                    prop_assert!(coa.root_of(child).unwrap().is_root());
                    prop_assert_eq!(coa.level_of(child).unwrap(), level + 1);
                    parent = child;
                }
            }
        }

        /// A chain of `depth` accounts under a root reports exactly that depth,
        /// and its path has one more entry than its level.
        #[test]
        fn prop_level_and_path_agree(depth in 1usize..20) {
            let mut coa = chart();
            let mut parent = coa.roots().expense();
            for i in 0..depth {
                let child = id(10 + i as u64);
                coa.add(child, &format!("Level {i}"), parent).expect("each add must succeed");
                parent = child;
            }

            let deepest = id(10 + depth as u64 - 1);
            let path = coa.path_of(deepest).unwrap();
            prop_assert_eq!(coa.level_of(deepest).unwrap(), depth);
            prop_assert_eq!(path.len(), depth + 1);
            prop_assert_eq!(&path[0], "Expenses");
        }

        /// No accepted sequence of adds and renames leaves two siblings sharing
        /// a name, whatever order the operations arrive in.
        #[test]
        fn prop_siblings_never_share_a_name(
            ops in prop::collection::vec(("[A-C]", 0usize..12), 1..12),
        ) {
            let mut coa = chart();
            let asset = coa.roots().asset();
            for (i, (name, target)) in ops.iter().enumerate() {
                // Deliberately colliding names: most of these must be refused.
                let _ = coa.add(id(10 + i as u64), name, asset);
                let _ = coa.rename(id(10 + *target as u64), name);
            }

            let mut seen: Vec<&str> = coa.children_of(asset).unwrap().map(Account::name).collect();
            let total = seen.len();
            prop_assert!(total >= 1, "the first add always lands, so this cannot pass vacuously");
            seen.sort_unstable();
            seen.dedup();
            prop_assert_eq!(seen.len(), total, "two siblings share a name");
        }
    }
}
