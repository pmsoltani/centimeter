//! Accounts: the buckets value sits in or flows through.
//!
//! An account is a node in one tree per ledger, the
//! [`ChartOfAccounts`]. Every account descends from exactly one of five roots,
//! and that root is what fixes the account's [`AccountType`]. Nothing else
//! does: the element is not a field, so an account cannot contradict its
//! parent about what it is.
//!
//! An account is not restricted to one commodity. A brokerage account holding
//! USD, AAPL and TSLA is one account with three balances, not three accounts.
//! There is no `balance` field either: a balance is derived by folding the
//! posting stream, never stored.
//!
//! The chart owns every account, and is the only thing that can build or
//! change one. That is what lets a mutation be checked against its siblings
//! and its ancestors, neither of which an `Account` can see by itself.

use crate::{Id, IdPrefix, Identifiable, Text, TextProblem, TextSpec};

mod chart;
mod element;
mod error;
mod root;

pub use chart::ChartOfAccounts;
pub use element::AccountType;
pub use error::AccountError;
pub use root::{RootAccounts, RootSpec, RootsSpec};

/// One account in the [`ChartOfAccounts`].
///
/// Identity is the [`id`](Self::id) alone. Two values with the same id are the
/// same account however their names differ, because a rename is a correction
/// to a label and not the birth of a new account. Postings reference the id
/// for the life of the ledger, which is what makes that the right rule.
///
/// Values of this type are produced only by the chart, so holding one is proof
/// that its name was validated and that it sits under a parent that exists.
///
/// # Examples
///
/// ```
/// # use centimeter_core::{AccountId, ChartOfAccounts, RootSpec, RootsSpec};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let new_id = || AccountId::from_uuid(uuid::Uuid::now_v7()).unwrap();
/// # let (asset, liability, equity, income, expense) =
/// #     (new_id(), new_id(), new_id(), new_id(), new_id());
/// let mut chart = ChartOfAccounts::try_new(RootsSpec {
///     asset: RootSpec { id: asset, name: "Assets" },
///     liability: RootSpec { id: liability, name: "Liabilities" },
///     equity: RootSpec { id: equity, name: "Equity" },
///     income: RootSpec { id: income, name: "Income" },
///     expense: RootSpec { id: expense, name: "Expenses" },
/// })?;
///
/// let bank = new_id();
/// chart.add(bank, "Bank", asset)?;
///
/// let account = chart.get(bank).expect("just added");
/// assert_eq!(account.name(), "Bank");
/// assert_eq!(account.parent_id(), Some(asset));
/// assert!(!account.is_root());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Account {
    /// The unique id for this account.
    id: AccountId,
    /// The display name, unique among the account's siblings.
    name: AccountName,
    /// The parent account, or `None` for exactly the five roots.
    parent_id: Option<AccountId>,
}

impl Identifiable for Account {
    const PREFIX: IdPrefix = IdPrefix::new("acc");
}

/// The id of an [`Account`], rendered as `acc_<suffix>`.
pub type AccountId = Id<Account>;

struct AccountNameSpec;
impl TextSpec for AccountNameSpec {
    type Error = AccountError;
    fn map_error(problem: TextProblem) -> Self::Error {
        match problem {
            TextProblem::TooShort { .. } => AccountError::NameEmpty,
            TextProblem::TooLong { max, got } => AccountError::NameTooLong { max, got },
            TextProblem::BadChar { character, index } => {
                AccountError::NameBadChar { character, index }
            }
        }
    }
}
type AccountName = Text<AccountNameSpec>;

impl Account {
    /// The maximum length of an account name (counted in chars).
    pub const MAX_NAME_LENGTH: usize = AccountNameSpec::MAX_LENGTH;

    /// Builds an account from parts the chart has already validated.
    ///
    /// Infallible by design: an `AccountName` cannot exist without having been
    /// validated, and the chart is the only caller, so it has already ensured
    /// that the parent exists and that no sibling has the name.
    #[must_use]
    fn new(id: AccountId, name: AccountName, parent_id: Option<AccountId>) -> Self {
        Self { id, name, parent_id }
    }

    /// Returns the account's id.
    #[must_use]
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// Returns the account's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the id of the account's parent, or `None` if it is a root.
    #[must_use]
    pub fn parent_id(&self) -> Option<AccountId> {
        self.parent_id
    }

    /// Returns true if the account is one of the five roots.
    ///
    /// Answered structurally, by having no parent. That agrees with
    /// [`RootAccounts`] because only chart construction ever builds a
    /// parentless account: an account has no parent if and only if it is one
    /// of the five roots.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Replaces the display name with one the chart has already validated and
    /// checked for uniqueness among siblings.
    fn set_name(&mut self, name: AccountName) {
        self.name = name;
    }

    /// Replaces the parent with one the chart has already checked for
    /// existence, element agreement and cycles.
    ///
    /// Takes an `AccountId` rather than an `Option`, so no account can be
    /// promoted to a sixth root by reparenting.
    fn set_parent_id(&mut self, parent_id: AccountId) {
        self.parent_id = Some(parent_id);
    }
}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Account {}

impl std::hash::Hash for Account {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

    use crate::test_support::id;

    /// An account under `id(0)`, with the given name, parented to `id(9)`.
    fn build(name: &str) -> Account {
        let validated = AccountName::try_new(name)
            .unwrap_or_else(|e| panic!("{name:?} should be accepted, got {e}"));
        Account::new(id(0), validated, Some(id(9)))
    }

    #[test]
    fn test_account_exposes_its_fields() {
        let account = build("Bank");
        assert_eq!(account.id(), id(0));
        assert_eq!(account.name(), "Bank");
        assert_eq!(account.parent_id(), Some(id(9)));
        assert!(!account.is_root());
    }

    #[test]
    fn test_a_parentless_account_is_a_root() {
        let name = AccountName::try_new("Assets").unwrap();
        let root = Account::new(id(1), name, None);
        assert!(root.is_root());
        assert_eq!(root.parent_id(), None);
    }

    /// Each `TextProblem` reaches the caller as the matching `AccountError`.
    #[test]
    fn test_name_problems_map_to_account_errors() {
        let over = "a".repeat(AccountNameSpec::MAX_LENGTH + 1);
        assert!(matches!(AccountName::try_new("  "), Err(AccountError::NameEmpty)));
        assert!(matches!(
            AccountName::try_new(&over),
            Err(AccountError::NameTooLong { max: AccountNameSpec::MAX_LENGTH, got: 257 })
        ));
        assert!(matches!(
            AccountName::try_new("Fixed\u{7}Assets"),
            Err(AccountError::NameBadChar { character: '\u{7}', index: 5 })
        ));
    }

    #[test]
    fn test_name_error_escapes_untrusted_input() {
        let err = AccountName::try_new("Ba\u{1b}[2Jnk").unwrap_err();
        let message = err.to_string();
        assert!(!message.contains('\u{1b}'), "message leaked a raw escape: {message:?}");
    }

    /// A rename corrects a label. It does not produce a different account, so
    /// equality has to ignore every field but the id.
    #[test]
    fn test_identity_is_by_id() {
        let bank = build("Bank");
        let new_name = AccountName::try_new("Current Account").unwrap();
        let renamed = Account::new(bank.id(), new_name, None);
        assert_eq!(bank, renamed, "same id must mean the same account");

        let other_name = AccountName::try_new("Bank").unwrap();
        let other = Account::new(id(1), other_name, Some(id(9)));
        assert_ne!(bank, other, "different ids must mean different accounts");
    }

    #[test]
    fn test_hash_is_by_id() {
        let bank = build("Bank");
        // Same id, every other field different: still the same account.
        let new_name = AccountName::try_new("Current Account").unwrap();
        let other_name = AccountName::try_new("Bank").unwrap();

        let alias = Account::new(id(0), new_name, None);
        let other = Account::new(id(1), other_name, Some(id(9)));

        let mut set = HashSet::new();
        set.insert(bank);
        set.insert(alias);
        assert_eq!(set.len(), 1, "accounts with the same id must collapse to one entry");
        set.insert(other);
        assert_eq!(set.len(), 2, "accounts with different ids must be distinct entries");
    }

    #[test]
    fn test_set_name_replaces_the_name_only() {
        let mut account = build("Bank");
        let new_name = AccountName::try_new("Current Account").unwrap();
        account.set_name(new_name);
        assert_eq!(account.name(), "Current Account");
        assert_eq!(account.id(), id(0));
        assert_eq!(account.parent_id(), Some(id(9)));
    }

    /// Reparenting takes an id rather than an `Option`, so the type refuses to
    /// express "make this account a sixth root".
    #[test]
    fn test_set_parent_id_never_produces_a_root() {
        let name = AccountName::try_new("Assets").unwrap();
        let mut root = Account::new(id(1), name, None);
        assert!(root.is_root());
        root.set_parent_id(id(2));
        assert_eq!(root.parent_id(), Some(id(2)));
        assert!(!root.is_root());
    }

    proptest! {
        /// Equality tracks the id and nothing else, whatever the other fields.
        #[test]
        fn prop_equality_follows_the_id(
            seed: u32,
            same: bool,
            left_name in "[A-Za-z][A-Za-z ]{0,19}",
            right_name in "[A-Za-z][A-Za-z ]{0,19}",
        ) {
            let other = if same { seed } else { seed.wrapping_add(1) };
            let left_name = AccountName::try_new(&left_name).unwrap();
            let right_name = AccountName::try_new(&right_name).unwrap();
            let left = Account::new(id(u64::from(seed)), left_name, None);
            let right = Account::new(id(u64::from(other)), right_name, Some(id(9)));
            prop_assert_eq!(left == right, same);
        }
    }
}
