//! The crate's composing root error.

use crate::{CommodityError, IdError};

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
}
