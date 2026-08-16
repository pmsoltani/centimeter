//! The five elements an account belongs to.
//!
//! The set of elements is fixed at five by the IFRS Conceptual Framework.
//! `centimeter` treats these as part of the accounting model rather than
//! something that can be configured.
//!
//! Some national account plans appear to use more categories, such as
//! France's eight-class PCG. Those categories are a presentation and coding
//! scheme built on top of these five elements.
//!
//! Accounts do not store their element directly. Instead, an account inherits
//! its element from its root. Because every account has exactly one root,
//! every account has exactly one element.

use std::fmt;

/// The fundamental accounting classification of an account.
///
/// The type determines how an account is reported and its normal balance:
/// assets and expenses normally have debit balances, while liabilities,
/// equity and income normally have credit balances.
///
/// Assets, liabilities and equity are *real* (or permanent) accounts: their
/// balances carry forward from one period to the next. They are connected by
/// the accounting equation `Assets = Liabilities + Equity`.
///
/// Income and expenses are *nominal* (or temporary) accounts: their balances
/// are closed into equity at the end of each accounting period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    /// An economic resource the entity controls, expected to provide benefits.
    Asset,
    /// A current obligation to transfer economic value to another entity.
    Liability,
    /// The remaining interest in assets after all obligations are subtracted.
    Equity,
    /// An increase in economic benefit other than a contribution from owners.
    Income,
    /// A decrease in economic benefit other than a distribution to owners.
    Expense,
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asset => write!(f, "Asset"),
            Self::Liability => write!(f, "Liability"),
            Self::Equity => write!(f, "Equity"),
            Self::Income => write!(f, "Income"),
            Self::Expense => write!(f, "Expense"),
        }
    }
}

impl AccountType {
    /// Every element, in financial-statement order.
    ///
    /// Rust enums carry no runtime list of their variants, so anything wanting
    /// to iterate the five needs this. The order is the domain's: the balance
    /// sheet runs Asset, Liability, Equity, and the income statement runs
    /// Income, Expense.
    ///
    /// Hand-maintained, and the compiler will not notice if it goes stale. The
    /// exhaustive matches in [`Display`](fmt::Display) and
    /// [`RootAccounts::type_of`](crate::RootAccounts::type_of) are what fail
    /// first if an element is ever added.
    pub const ALL: [Self; 5] =
        [Self::Asset, Self::Liability, Self::Equity, Self::Income, Self::Expense];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_names_each_element() {
        let rendered: Vec<String> = AccountType::ALL.iter().map(ToString::to_string).collect();
        assert_eq!(rendered, ["Asset", "Liability", "Equity", "Income", "Expense"]);
    }

    /// [`AccountType::ALL`] is hand-written, so assert it against a match the
    /// compiler checks for exhaustiveness: a sixth element stops compiling here
    /// before it can silently go missing from the constant.
    #[test]
    fn test_all_covers_every_element_in_statement_order() {
        fn position(element: AccountType) -> usize {
            match element {
                AccountType::Asset => 0,
                AccountType::Liability => 1,
                AccountType::Equity => 2,
                AccountType::Income => 3,
                AccountType::Expense => 4,
            }
        }

        assert_eq!(AccountType::ALL.len(), 5);
        for (index, element) in AccountType::ALL.into_iter().enumerate() {
            assert_eq!(position(element), index, "{element} is out of statement order");
        }
    }

    #[test]
    fn test_elements_are_distinct() {
        for (index, element) in AccountType::ALL.into_iter().enumerate() {
            for (other_index, other) in AccountType::ALL.into_iter().enumerate() {
                assert_eq!(
                    element == other,
                    index == other_index,
                    "{element} and {other} compare wrongly"
                );
            }
        }
    }

    #[test]
    fn test_element_is_copy() {
        let element = AccountType::Equity;
        let copy = element;
        assert_eq!(element, copy);
    }
}
