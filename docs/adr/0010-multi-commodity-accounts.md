---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0010: Multi-commodity accounts with per-commodity balances

## Context

The account tree in [ADR-0009](0009-typed-account-tree-five-roots.md) types an account by its root element but does not say anything about commodity, which has left open the question of whether an account is restricted to a single commodity or can hold several at once. Restricting an account to a single commodity creates modeling friction for real-world cases like multi-currency bank accounts, crypto wallets, and brokerage accounts (e.g., holding USD, AAPL, and TSLA in one "Brokerage" account).

## Decision

**Accounts can hold multiple commodities. An `Account` record does not enforce or define a single `commodity` restriction.**

- **No Account-Level Constraint:** The `Account` struct will not have a `commodity` field. Any commodity can be posted to any account, provided the overall transaction balances exactly in the ledger's functional currency ([ADR-0006](0006-one-functional-currency-per-ledger.md)).
- **Per-Commodity Balances:** As established in [ADR-0015](0015-balances-are-derived-from-postings.md), a balance is a fact about `(account, commodity, point_in_time)`. Querying a single account will naturally yield a list of balances (e.g., a vector of `Quantity`), one for each commodity it currently holds.

## Consequences

- **Good:** Perfectly models real-world multi-asset accounts (brokerages, crypto wallets) without forcing the user to create dozens of artificial, single-asset sub-accounts.
- **Good:** Simplifies the `Account` struct by removing an unnecessary constraint field.
- **Bad:** Application UIs, trial balances, and reports must be strictly designed to handle and display an account's balance as a list of multiple commodities, rather than a single scalar value.
- **Bad:** The posting builder's entailment logic must rely entirely on the posting input rather than structural account definitions.

### Confirmation

`Account` has no `commodity` field and no method returning one. A test posts two different commodities to the same account and asserts the account query returns two balances rather than an error.
