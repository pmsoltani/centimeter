//! Errors for the posting domain.

use rust_decimal::Decimal;

use crate::{CommodityId, RateError};

/// Errors related to postings.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PostingError {
    /// The amount and value have the same commodity but different numbers.
    #[error(
        "amount {amount} and value {value} are in the same commodity but differ, \
         which would convert a commodity to itself at a rate other than one"
    )]
    AmountValueMismatch {
        /// The amount's number.
        amount: Decimal,
        /// The value's number.
        value: Decimal,
    },

    /// The amount's commodity is not the rate's base commodity.
    #[error("amount is in {got}, but the rate's base is {expected}")]
    BaseMismatch {
        /// The amount's commodity.
        got: CommodityId,
        /// The rate's base commodity.
        expected: CommodityId,
    },

    /// A member that must be in the functional commodity is not.
    #[error("expected the functional commodity {expected}, got {got}")]
    FunctionalMismatch {
        /// The provided commodity.
        got: CommodityId,
        /// The functional commodity.
        expected: CommodityId,
    },

    /// The value's commodity is not the rate's quote commodity.
    #[error("value is in {got}, but the rate's quote is {expected}")]
    QuoteMismatch {
        /// The value's commodity.
        got: CommodityId,
        /// The rate's quote commodity.
        expected: CommodityId,
    },

    /// Applying the rate failed.
    #[error(transparent)]
    Rate(#[from] RateError),

    /// A commodity is not in the registry.
    #[error("commodity {id} is not registered")]
    UnknownCommodity {
        /// The provided commodity id.
        id: CommodityId,
    },

    /// The rate was zero when a value was provided.
    #[error("cannot derive an amount from a value at a zero rate; supply the amount instead")]
    ZeroRateWithValue,
}
