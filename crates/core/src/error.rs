//! The crate's composing root error.

use crate::{AccountError, CommodityError, IdError, PostingError, QuantityError, RateError};

/// Any error the core can produce.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The error is related to accounts.
    #[error(transparent)]
    Account(#[from] AccountError),

    /// The error is related to commodities.
    #[error(transparent)]
    Commodity(#[from] CommodityError),

    /// The error is related to record ids.
    #[error(transparent)]
    Id(#[from] IdError),

    /// The error is related to postings.
    #[error(transparent)]
    Posting(#[from] PostingError),

    /// The error is related to quantities.
    #[error(transparent)]
    Quantity(#[from] QuantityError),

    /// The error is related to rates.
    #[error(transparent)]
    Rate(#[from] RateError),
}
