//! Errors for the quantity domain.

use super::CommodityId;
use rust_decimal::Decimal;

/// Errors related to quantities.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QuantityError {
    /// The combination of two quantities with different commodities is invalid.
    #[error("arithmetic requires matching commodities: left={left}, right={right}")]
    CommodityMismatch {
        /// The left quantity's commodity ID.
        left: CommodityId,
        /// The right quantity's commodity ID.
        right: CommodityId,
    },

    /// The arithmetic resulted in a value that needed more than 28 significant digits.
    #[error("arithmetic resulted in an inexact value: left={left}, right={right}")]
    Inexact {
        /// The left operand of the operation.
        left: Decimal,
        /// The right operand of the operation.
        right: Decimal,
    },

    /// A quantity number is too large for the commodity's scale.
    #[error("number {number} is too large for the scale {scale} of commodity '{code}'")]
    NumberTooLarge {
        /// The commodity code associated with the quantity.
        code: String,
        /// The commodity scale associated with the quantity.
        scale: u8,
        /// The provided number.
        number: Decimal,
    },

    /// An error indicating that an arithmetic operation overflowed.
    #[error("arithmetic overflow: left={left}, right={right}")]
    Overflow {
        /// The left operand of the operation.
        left: Decimal,
        /// The right operand of the operation.
        right: Decimal,
    },

    /// A quantity scale exceeded its commodity's scale.
    #[error("scale of quantity {number} exceeds scale {scale} of commodity '{code}'")]
    ScaleTooLarge {
        /// The commodity code associated with the quantity.
        code: String,
        /// The commodity scale associated with the quantity.
        scale: u8,
        /// The provided quantity's number.
        number: Decimal,
    },
}
