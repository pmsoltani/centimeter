//! The crate's composing root error.

use crate::{CommodityError, IdError, QuantityError, RateError};

/// Any error the core can produce.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The error is related to commodities.
    #[error(transparent)]
    Commodity(#[from] CommodityError),

    /// The error is related to record ids.
    #[error(transparent)]
    Id(#[from] IdError),

    /// The error is related to quantities.
    #[error(transparent)]
    Quantity(#[from] QuantityError),

    /// The error is related to rates.
    #[error(transparent)]
    Rate(#[from] RateError),
}
