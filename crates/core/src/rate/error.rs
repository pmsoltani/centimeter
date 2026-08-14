//! Errors for the rate domain.

use rust_decimal::Decimal;

use super::CommodityId;

/// Errors related to rates.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RateError {
    /// The two commodities provided to a distinct pair were identical.
    #[error("expected two distinct commodities, but got the same commodity '{got}' for both")]
    SameCommodity {
        /// The invalid commodity that was provided for both the quote and base.
        got: CommodityId,
    },

    /// An identity rate must have a multiplier of exactly 1.0.
    #[error("invalid identity rate: expected 1.0, got {got}")]
    BadIdentityRate {
        /// The invalid number that was provided.
        got: Decimal,
    },
}
