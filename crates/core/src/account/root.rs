//! The five root accounts and the specification used to create them.
//!
//! Every account belongs to one of five roots, and its root determines its
//! [element](super::AccountType). Root accounts can have any name, so "Assets"
//! and "Actif" are equivalent as far as the chart is concerned.
//!
//! [`RootsSpec`] describes the roots when bootstrapping a
//! [`ChartOfAccounts`](super::ChartOfAccounts). [`RootAccounts`] keeps track
//! of the five root ids after the chart has been created.

use super::{AccountError, AccountId, AccountType};

/// The ids of the five root accounts, one per [element](AccountType).
///
/// Only [`ChartOfAccounts::try_new`](super::ChartOfAccounts::try_new) can
/// build one, so holding a `RootAccounts` is proof that those five accounts
/// exist in a chart and are all distinct.
#[derive(Debug, Clone, Copy)]
pub struct RootAccounts {
    asset: AccountId,
    liability: AccountId,
    equity: AccountId,
    income: AccountId,
    expense: AccountId,
}

impl RootAccounts {
    /// Records the five root ids, ensuring they are all distinct.
    ///
    /// # Errors
    /// - [`DuplicateRoot`](AccountError::DuplicateRoot) if one id is given for
    ///   two elements.
    pub(super) fn try_new(
        asset: AccountId,
        liability: AccountId,
        equity: AccountId,
        income: AccountId,
        expense: AccountId,
    ) -> Result<Self, AccountError> {
        // Sorting five `Copy` ids and scanning adjacent pairs costs no
        // allocation, and unlike a `HashSet` it can name both colliding slots.
        let mut roots = [
            (AccountType::Asset, asset),
            (AccountType::Liability, liability),
            (AccountType::Equity, equity),
            (AccountType::Income, income),
            (AccountType::Expense, expense),
        ];
        roots.sort_unstable_by_key(|(_, id)| *id);
        for window in roots.windows(2) {
            if window[0].1 == window[1].1 {
                return Err(AccountError::DuplicateRoot {
                    id: window[0].1,
                    first: window[0].0,
                    second: window[1].0,
                });
            }
        }
        Ok(Self { asset, liability, equity, income, expense })
    }

    /// Returns the element of a **root** account.
    ///
    /// This answers only about the five roots. To ask about any account, use
    /// [`ChartOfAccounts::type_of`](super::ChartOfAccounts::type_of), which
    /// walks to the root first.
    ///
    /// # Errors
    /// - [`NotARoot`](AccountError::NotARoot) if `id` is not one of the five.
    pub fn type_of(self, id: AccountId) -> Result<AccountType, AccountError> {
        match id {
            _ if id == self.asset => Ok(AccountType::Asset),
            _ if id == self.liability => Ok(AccountType::Liability),
            _ if id == self.equity => Ok(AccountType::Equity),
            _ if id == self.income => Ok(AccountType::Income),
            _ if id == self.expense => Ok(AccountType::Expense),
            _ => Err(AccountError::NotARoot { id }),
        }
    }

    /// Returns the id of the asset root.
    #[must_use]
    pub fn asset(self) -> AccountId {
        self.asset
    }

    /// Returns the id of the liability root.
    #[must_use]
    pub fn liability(self) -> AccountId {
        self.liability
    }

    /// Returns the id of the equity root.
    #[must_use]
    pub fn equity(self) -> AccountId {
        self.equity
    }

    /// Returns the id of the income root.
    #[must_use]
    pub fn income(self) -> AccountId {
        self.income
    }

    /// Returns the id of the expense root.
    #[must_use]
    pub fn expense(self) -> AccountId {
        self.expense
    }
}

/// One root account's id and display name, as supplied by the caller.
#[derive(Debug, Clone, Copy)]
pub struct RootSpec<'a> {
    /// The id of the root account.
    pub id: AccountId,
    /// The display name of the root account, e.g., `"Assets"` or `"Actif"`.
    pub name: &'a str,
}

/// The five roots a [`ChartOfAccounts`](super::ChartOfAccounts) is built from.
#[derive(Debug, Clone, Copy)]
pub struct RootsSpec<'a> {
    /// The asset root.
    pub asset: RootSpec<'a>,
    /// The liability root.
    pub liability: RootSpec<'a>,
    /// The equity root.
    pub equity: RootSpec<'a>,
    /// The income root.
    pub income: RootSpec<'a>,
    /// The expense root.
    pub expense: RootSpec<'a>,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    use crate::account::Account;
    use crate::test_support::id;

    /// The five roots under ids 1 to 5, in element order.
    fn roots() -> RootAccounts {
        RootAccounts::try_new(id(1), id(2), id(3), id(4), id(5)).expect("five distinct ids")
    }

    #[test]
    fn test_accessors_return_what_was_given() {
        let roots = roots();
        assert_eq!(roots.asset(), id(1));
        assert_eq!(roots.liability(), id(2));
        assert_eq!(roots.equity(), id(3));
        assert_eq!(roots.income(), id(4));
        assert_eq!(roots.expense(), id(5));
    }

    #[test]
    fn test_type_of_resolves_each_element() {
        let roots = roots();
        let pairs = [
            (roots.asset(), AccountType::Asset),
            (roots.liability(), AccountType::Liability),
            (roots.equity(), AccountType::Equity),
            (roots.income(), AccountType::Income),
            (roots.expense(), AccountType::Expense),
        ];
        for (id, element) in pairs {
            assert_eq!(roots.type_of(id).expect("a root must resolve"), element);
        }
    }

    #[test]
    fn test_type_of_refuses_a_stranger() {
        let stranger = id(99);
        let err = roots().type_of(stranger).expect_err("id 99 is not a root");
        assert!(matches!(err, AccountError::NotARoot { id } if id == stranger), "got {err}");
    }

    /// The error has to name the repeated id and *both* slots that claimed it,
    /// otherwise the caller only learns that something, somewhere, collided.
    #[test]
    fn test_duplicate_roots_name_the_collision() {
        let dup = id(1);
        let err = RootAccounts::try_new(dup, dup, id(3), id(4), id(5))
            .expect_err("asset and liability share an id");
        let AccountError::DuplicateRoot { id: got, first, second } = err else {
            panic!("expected DuplicateRoot, got {err}");
        };
        assert_eq!(got, dup);
        // Order within the pair is not specified, only that both slots appear.
        let named = [first, second];
        assert!(named.contains(&AccountType::Asset), "got {named:?}");
        assert!(named.contains(&AccountType::Liability), "got {named:?}");
    }

    #[test]
    fn test_a_duplicate_in_any_pair_of_slots_is_refused() {
        // Every pair of the five slots, with that pair sharing one id.
        for (first, second) in [(0, 1), (0, 4), (1, 2), (2, 3), (3, 4), (1, 4)] {
            let mut ids = [id::<Account>(1), id(2), id(3), id(4), id(5)];
            ids[second] = ids[first];
            let built = RootAccounts::try_new(ids[0], ids[1], ids[2], ids[3], ids[4]);
            assert!(
                matches!(built, Err(AccountError::DuplicateRoot { .. })),
                "slots {first} and {second} sharing an id must be refused"
            );
        }
    }

    /// The message interpolates [`AccountType`]'s `Display`, not its `Debug`.
    #[test]
    fn test_duplicate_root_message_names_the_id_and_both_elements() {
        let dup = id::<Account>(1);
        let err = RootAccounts::try_new(dup, dup, id(3), id(4), id(5)).unwrap_err();

        let message = err.to_string();
        assert!(message.contains(&dup.to_string()), "got {message}");
        assert!(message.contains("Asset"), "got {message}");
        assert!(message.contains("Liability"), "got {message}");
    }

    #[test]
    fn test_roots_are_copy() {
        let roots = roots();
        let copy = roots;
        assert_eq!(roots.asset(), copy.asset());
    }

    proptest! {
        /// Five distinct ids always build, whatever they are, and each slot
        /// reads back the id it was given.
        #[test]
        fn prop_five_distinct_ids_always_build(seed: u32) {
            // Distinct by construction: five consecutive seeds cannot collide.
            let base = u64::from(seed) * 5;
            let ids: [AccountId; 5] = std::array::from_fn(|i| id(base + i as u64));
            let roots = RootAccounts::try_new(ids[0], ids[1], ids[2], ids[3], ids[4]);
            let roots = roots.expect("five distinct ids must build");

            prop_assert_eq!(roots.asset(), ids[0]);
            prop_assert_eq!(roots.liability(), ids[1]);
            prop_assert_eq!(roots.equity(), ids[2]);
            prop_assert_eq!(roots.income(), ids[3]);
            prop_assert_eq!(roots.expense(), ids[4]);
        }

        /// Exactly the five seeded ids resolve to an element, and nothing else
        /// does, however the stranger is chosen.
        #[test]
        fn prop_only_the_five_resolve(stranger in 6u64..10_000) {
            let roots = roots();
            for (index, element) in AccountType::ALL.into_iter().enumerate() {
                let root_id = id(index as u64 + 1);
                prop_assert_eq!(roots.type_of(root_id).ok(), Some(element));
            }
            prop_assert!(roots.type_of(id(stranger)).is_err(), "id {stranger} is not a root");
        }
    }
}
