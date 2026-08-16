//! Errors for the account domain.

use super::{AccountId, AccountType};

/// Errors related to accounts.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AccountError {
    /// A root account cannot be reparented.
    #[error("account {id} is a root account and cannot be reparented")]
    CannotReparentRoot {
        /// The provided account ID.
        id: AccountId,
    },

    /// Reparenting an account would create a cycle.
    #[error("reparenting account {id} to parent {parent_id} would create a cycle")]
    CycleDetected {
        /// The provided account ID.
        id: AccountId,
        /// The provided parent account ID.
        parent_id: AccountId,
    },

    /// An account with ID already exists.
    #[error("an account with ID {id} already exists")]
    DuplicateId {
        /// The provided account ID.
        id: AccountId,
    },

    /// An account with name already exists under the same parent.
    #[error("sibling account {id} already has the name '{name}'")]
    DuplicateName {
        /// The id of the sibling already holding the name.
        id: AccountId,
        /// The provided account name.
        name: String,
    },

    /// The root accounts must be distinct.
    #[error("root accounts must be distinct: {id} is given as both {first} and {second}")]
    DuplicateRoot {
        /// The provided account ID that is duplicated.
        id: AccountId,
        /// The first account type.
        first: AccountType,
        /// The second account type.
        second: AccountType,
    },

    /// The maximum depth of the account hierarchy has been exceeded.
    #[error("maximum depth of account hierarchy, {max}, has been exceeded for account {id}")]
    MaxDepthExceeded {
        /// The provided account ID.
        id: AccountId,
        /// The maximum depth allowed for the account hierarchy.
        max: usize,
    },

    /// An account name has invalid characters.
    #[error("account name has invalid characters, got '{}' at index {index}", got.escape_debug())]
    NameBadChar {
        /// The account name that has invalid characters.
        got: String,
        /// The index of the invalid character.
        index: usize,
    },

    /// An account name is empty.
    #[error("account name must not be empty")]
    NameEmpty,

    /// An account name is too long.
    #[error("account name must be at most {max} characters, got {got}")]
    NameTooLong {
        /// The maximum length allowed for an account name.
        max: usize,
        /// The length of the provided account name.
        got: usize,
    },

    /// An account with ID is not a root account.
    #[error("account with ID {id} is not a root account")]
    NotARoot {
        /// The provided account ID.
        id: AccountId,
    },

    /// An account with ID was not found.
    #[error("an account with ID {id} was not found")]
    NotFound {
        /// The provided account ID.
        id: AccountId,
    },

    /// An account names a parent that is not in the chart.
    #[error("account {id} names parent {parent_id}, which is not in the chart")]
    Orphaned {
        /// The account holding the dangling parent id.
        id: AccountId,
        /// The parent id that resolves to nothing.
        parent_id: AccountId,
    },

    /// A parent account with ID was not found.
    #[error("a parent account with ID {parent_id} was not found")]
    ParentNotFound {
        /// The provided parent account ID.
        parent_id: AccountId,
    },

    /// An account cannot be reparented to a parent of a different type.
    #[error("account {id} of type {first} cannot be reparented to {parent_id} of type {second}")]
    ReparentTypeMismatch {
        /// The provided account ID.
        id: AccountId,
        /// The element of `id`.
        first: AccountType,
        /// The provided parent account ID.
        parent_id: AccountId,
        /// The element of `parent_id`.
        second: AccountType,
    },
}
