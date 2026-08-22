//! Errors for the rate domain.

use rust_decimal::Decimal;

use crate::{CommodityId, QuantityError};

/// Errors related to rates.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RateError {
    /// An identity rate must have a multiplier of exactly 1.0.
    #[error("invalid identity rate: expected 1.0, got {got}")]
    BadIdentityRate {
        /// The invalid number that was provided.
        got: Decimal,
    },

    /// A product needed more than 28 significant digits.
    #[error("multiplication resulted in an inexact product: left={left}, right={right}")]
    InexactProduct {
        /// The left operand.
        left: Decimal,
        /// The right operand.
        right: Decimal,
    },

    /// Rate application overflowed (`amount * rate` or `value / rate`).
    #[error("applying the rate overflowed: left={left}, right={right}")]
    Overflow {
        /// The left operand.
        left: Decimal,
        /// The right operand.
        right: Decimal,
    },

    /// A derived amount or value did not fit the commodity it is denominated in.
    #[error(transparent)]
    Quantity(#[from] QuantityError),

    /// The two commodities provided to a distinct pair were identical.
    #[error("expected two distinct commodities, but got the same commodity '{got}' for both")]
    SameCommodity {
        /// The invalid commodity that was provided for both the quote and base.
        got: CommodityId,
    },
}
