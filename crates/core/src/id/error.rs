//! Errors for the `id` module.

use uuid::Uuid;

/// Errors related to record ids.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IdError {
    /// The UUID provided is not a version 7 UUID.
    #[error("uuid is not version 7: '{got}' has version {version}")]
    UuidNotV7 {
        /// The provided UUID.
        got: Uuid,
        /// The provided UUID's version.
        version: usize,
    },

    /// A record id string is not in the correct format.
    #[error(
        "record id string is not in the correct format, '<prefix>_<suffix>', got '{}'",
        got.escape_debug()
    )]
    BadFormat {
        /// The provided ID.
        got: String,
    },

    /// A record id string has an invalid prefix.
    #[error(
        "record id string has an invalid prefix, got '{}', expected '{expected}'",
        got.escape_debug()
    )]
    PrefixMismatch {
        /// The provided ID.
        got: String,
        /// The expected prefix for the record type.
        expected: &'static str,
    },

    /// The record id suffix is not 26 characters long.
    #[error(
        "record id suffix is not 26 characters long, got '{}' with length {len}",
        got.escape_debug()
    )]
    SuffixBadLength {
        /// The provided ID's suffix.
        got: String,
        /// The provided ID's suffix length.
        len: usize,
    },

    /// The record id suffix contains an invalid character.
    #[error("record id suffix contains an invalid character, got '{got}' at index {index}")]
    SuffixBadChar {
        /// The provided ID's suffix.
        got: String,
        /// The index of the invalid character.
        index: usize,
    },

    /// The record id suffix is too large to be a valid `UUIDv7`.
    #[error(
        "record id suffix is too large for UUIDv7 (first char must be <= 7), got '{}'",
        got.escape_ascii()
    )]
    SuffixOverflow {
        /// The provided ID's suffix.
        got: u8,
    },
}
