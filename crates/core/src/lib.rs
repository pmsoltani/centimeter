//! The double-entry accounting engine at the heart of centimeter.

mod account;
mod commodity;
mod error;
mod id;
mod posting;
mod quantity;
mod rate;
mod text;

#[cfg(test)]
mod test_support;

pub use rust_decimal::Decimal;

pub use account::{
    Account, AccountError, AccountId, AccountType, ChartOfAccounts, RootAccounts, RootSpec,
    RootsSpec,
};
pub use commodity::{Commodity, CommodityError, CommodityId, CommodityRegistry};
pub use error::Error;
pub use id::{Id, IdError, IdPrefix, Identifiable};
pub use posting::{Posting, PostingError, PostingId, PostingValuation};
pub use quantity::{Quantity, QuantityError};
pub use rate::{CommodityPair, Rate, RateError};
use text::{Text, TextProblem, TextSpec};
