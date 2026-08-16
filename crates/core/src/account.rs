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

use crate::{Id, IdPrefix, Identifiable};

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
    name: String,
    /// The parent account, or `None` for exactly the five roots.
    parent_id: Option<AccountId>,
}

impl Identifiable for Account {
    const PREFIX: IdPrefix = IdPrefix::new("acc");
}

/// The id of an [`Account`], rendered as `acc_<suffix>`.
pub type AccountId = Id<Account>;

impl Account {
    // TODO: Consider refactoring the `name` fields of `Account` and `Commodity`
    // into a newtype, making the code more DRY.

    /// The maximum length of an account name (counted in chars).
    pub const MAX_NAME_LENGTH: usize = 256;

    /// Builds an account from parts the chart has already validated.
    ///
    /// Infallible by design. The chart has to run
    /// [`validate_name`](Self::validate_name) before it can check the result
    /// for uniqueness among siblings, so re-validating here would be doing the
    /// same work twice and calling it safety.
    #[must_use]
    fn new(id: AccountId, name: String, parent_id: Option<AccountId>) -> Self {
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
        &self.name
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

    /// Trims `name` and returns it, or says why it is unusable.
    ///
    /// Surrounding whitespace is discarded rather than rejected, so the
    /// returned string is what the account will store and what a uniqueness
    /// check must compare against.
    ///
    /// # Errors
    /// - [`NameEmpty`](AccountError::NameEmpty) if `name` is empty or only
    ///   whitespace.
    /// - [`NameTooLong`](AccountError::NameTooLong) if `name` exceeds
    ///   [`MAX_NAME_LENGTH`](Self::MAX_NAME_LENGTH) characters.
    /// - [`NameBadChar`](AccountError::NameBadChar) if `name` contains a
    ///   control character.
    fn validate_name(name: &str) -> Result<String, AccountError> {
        let name = name.trim();
        let len = name.chars().count();
        if name.is_empty() {
            return Err(AccountError::NameEmpty);
        }
        if Self::MAX_NAME_LENGTH < len {
            return Err(AccountError::NameTooLong { max: Self::MAX_NAME_LENGTH, got: len });
        }
        if let Some((index, _)) = name.char_indices().find(|(_, c)| c.is_control()) {
            return Err(AccountError::NameBadChar { got: name.to_string(), index });
        }
        Ok(name.to_string())
    }

    /// Replaces the display name with one the chart has already validated and
    /// checked for uniqueness among siblings.
    fn set_name(&mut self, name: String) {
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
        let validated = Account::validate_name(name)
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
        let root = Account::new(id(1), "Assets".to_string(), None);
        assert!(root.is_root());
        assert_eq!(root.parent_id(), None);
    }

    #[test]
    fn test_name_is_trimmed() {
        assert_eq!(build("  Bank \n").name(), "Bank");
    }

    #[test]
    fn test_name_rejects_empty() {
        for name in ["", "   ", "\t\n"] {
            assert!(
                matches!(Account::validate_name(name), Err(AccountError::NameEmpty)),
                "{name:?} should be rejected as empty"
            );
        }
    }

    #[test]
    fn test_name_length_is_measured_in_chars() {
        // Exactly at the limit is fine, one over is not.
        let at_limit = "a".repeat(Account::MAX_NAME_LENGTH);
        assert_eq!(build(&at_limit).name().chars().count(), Account::MAX_NAME_LENGTH);

        let over = "a".repeat(Account::MAX_NAME_LENGTH + 1);
        assert!(matches!(
            Account::validate_name(&over),
            Err(AccountError::NameTooLong { max: Account::MAX_NAME_LENGTH, got: 257 })
        ));

        // Chars, not bytes: 256 two-byte chars is 512 bytes and still accepted.
        let multibyte = "é".repeat(Account::MAX_NAME_LENGTH);
        assert_eq!(multibyte.len(), 2 * Account::MAX_NAME_LENGTH);
        assert!(Account::validate_name(&multibyte).is_ok());
    }

    #[test]
    fn test_name_rejects_control_chars() {
        for (name, index) in [("Ba\u{7}nk", 2), ("a\rb", 1), ("x\u{1b}[2J", 1)] {
            assert!(
                matches!(
                    Account::validate_name(name),
                    Err(AccountError::NameBadChar { index: at, .. }) if at == index
                ),
                "{name:?} should be rejected at index {index}"
            );
        }
    }

    #[test]
    fn test_name_allows_non_control_unicode() {
        // Names are display strings: accents, CJK and symbols must survive.
        for name in ["Bank", "Café ☕ Petty Cash", "現金", "Expenses (2026)", "£ float"] {
            assert_eq!(build(name).name(), name);
        }
    }

    #[test]
    fn test_name_error_escapes_untrusted_input() {
        let err = Account::validate_name("Ba\u{1b}[2Jnk").unwrap_err();
        let message = err.to_string();
        assert!(!message.contains('\u{1b}'), "message leaked a raw escape: {message:?}");
    }

    /// A rename corrects a label. It does not produce a different account, so
    /// equality has to ignore every field but the id.
    #[test]
    fn test_identity_is_by_id() {
        let bank = build("Bank");
        let renamed = Account::new(bank.id(), "Current Account".to_string(), None);
        assert_eq!(bank, renamed, "same id must mean the same account");

        let other = Account::new(id(1), "Bank".to_string(), Some(id(9)));
        assert_ne!(bank, other, "different ids must mean different accounts");
    }

    #[test]
    fn test_hash_is_by_id() {
        let bank = build("Bank");
        // Same id, every other field different: still the same account.
        let alias = Account::new(id(0), "Current Account".to_string(), None);
        let other = Account::new(id(1), "Bank".to_string(), Some(id(9)));

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
        account.set_name("Current Account".to_string());
        assert_eq!(account.name(), "Current Account");
        assert_eq!(account.id(), id(0));
        assert_eq!(account.parent_id(), Some(id(9)));
    }

    /// Reparenting takes an id rather than an `Option`, so the type refuses to
    /// express "make this account a sixth root".
    #[test]
    fn test_set_parent_id_never_produces_a_root() {
        let mut root = Account::new(id(1), "Assets".to_string(), None);
        assert!(root.is_root());
        root.set_parent_id(id(2));
        assert_eq!(root.parent_id(), Some(id(2)));
        assert!(!root.is_root());
    }

    proptest! {
        /// Validation never panics, and acceptance implies the documented shape.
        #[test]
        fn prop_validation_is_total(name: String) {
            if let Ok(validated) = Account::validate_name(&name) {
                prop_assert_eq!(&validated, name.trim());
                prop_assert!(!validated.is_empty());
                prop_assert!(validated.chars().count() <= Account::MAX_NAME_LENGTH);
                prop_assert!(!validated.chars().any(char::is_control));
            }
        }

        /// Validation is idempotent: feeding an accepted name back in returns
        /// it unchanged. The chart relies on this when it compares a validated
        /// name against names already stored.
        #[test]
        fn prop_validation_is_idempotent(name: String) {
            if let Ok(once) = Account::validate_name(&name) {
                let twice = Account::validate_name(&once).ok();
                prop_assert_eq!(twice, Some(once));
            }
        }

        /// Equality tracks the id and nothing else, whatever the other fields.
        #[test]
        fn prop_equality_follows_the_id2(
            seed: u32,
            same: bool,
            left_name in "[A-Za-z][A-Za-z ]{0,19}",
            right_name in "[A-Za-z][A-Za-z ]{0,19}",
        ) {
            let other = if same { seed } else { seed.wrapping_add(1) };
            let left = Account::new(id(u64::from(seed)), left_name, None);
            let right = Account::new(id(u64::from(other)), right_name, Some(id(9)));
            prop_assert_eq!(left == right, same);
        }
    }
}
