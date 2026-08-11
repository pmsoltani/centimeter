//! Errors for the commodity domain.

use super::CommodityId;

/// Errors related to commodities.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommodityError {
    /// A commodity code has invalid characters.
    #[error(
        "commodity code can only have these chars [A-Za-z0-9._-], got '{}' at index {index}",
        got.escape_debug()
    )]
    CodeBadChar {
        /// The commodity code that has invalid characters.
        got: String,
        /// The index of the invalid character.
        index: usize,
    },

    /// A commodity code starts with an invalid character.
    #[error("commodity code can only start with alphanumeric characters, got '{}'", got.escape_debug())]
    CodeBadFirstChar {
        /// The commodity code that starts with an invalid character.
        got: String,
    },

    /// A commodity code ends with an invalid character.
    #[error("commodity code can only end with alphanumeric characters, got '{}'", got.escape_debug())]
    CodeBadLastChar {
        /// The commodity code that ends with an invalid character.
        got: String,
    },

    /// A commodity code is empty.
    #[error("commodity code must not be empty")]
    CodeEmpty,

    /// A commodity code is too long.
    #[error("commodity code must be at most {max} bytes, got {got}")]
    CodeTooLong {
        /// The maximum length allowed for a commodity code.
        max: usize,
        /// The length of the provided commodity code.
        got: usize,
    },

    /// A commodity name has invalid characters.
    #[error("commodity name has invalid characters, got '{}' at index {index}", got.escape_debug())]
    NameBadChar {
        /// The commodity name that has invalid characters.
        got: String,
        /// The index of the invalid character.
        index: usize,
    },

    /// A commodity name is empty.
    #[error("commodity name must not be empty")]
    NameEmpty,

    /// A commodity name is too long.
    #[error("commodity name must be at most {max} characters, got {got}")]
    NameTooLong {
        /// The maximum length allowed for a commodity name.
        max: usize,
        /// The length of the provided commodity name.
        got: usize,
    },

    /// A commodity scale exceeded [`Commodity::MAX_SCALE`](crate::Commodity::MAX_SCALE).
    #[error("commodity scale must be between 0 and {max}, got {got}")]
    ScaleTooLarge {
        /// The maximum scale allowed for a commodity.
        max: u8,
        /// The scale that was provided for the commodity.
        got: u8,
    },

    /// A commodity with the same ID already exists in the registry.
    #[error("a commodity with id '{id}' already exists")]
    DuplicateId {
        /// The provided ID.
        id: CommodityId,
    },

    /// A commodity with the same code already exists in the registry.
    #[error("a commodity with code '{code}' is already registered as '{id}'")]
    DuplicateCode {
        /// The provided code.
        code: String,
        /// The ID of the existing commodity.
        id: CommodityId,
    },
}
