---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0006: Balance in one functional currency, fixed at the ledger level

> **A note on vocabulary.** `centimeter`'s type system has only `Commodity`. There is no `Currency` type; a currency is simply a commodity that happens to be legal tender (conventional, not enforced). This ADR nevertheless says "currency" throughout, because _functional currency_, and _presentation currency_ are the terms of art in IAS 21 and ASC 830.

## Context

Three distinct currency roles exist in accounting:

| Role                                   | Accounting term           | Model field                  | Scope                              |
| -------------------------------------- | ------------------------- | ---------------------------- | ---------------------------------- |
| the one currency the ledger balance in | **functional currency**   | `value.commodity`            | the **ledger/entity**, exactly one |
| the currency a leg is denominated in   | **posting currency**      | `amount.commodity`           | per posting or document            |
| the currency you report in             | **presentation currency** | a reporting-layer projection | per report                         |

Conflating these is a common modeling error. We need to establish which currency (commodity) the `∑ value = 0` invariant applies to and at what scope it is fixed.

## Decision

Balance in **one functional currency**, fixed at the **ledger/entity level**.

- **The Rule:** The `value.commodity` of every posting must equal the ledger's designated functional currency.
- **Standards Alignment:** This matches IAS 21 and ASC 830 directly. Multi-currency groups require multiple ledgers and a consolidation layer outside the core.

## Consequences

- **Good:** The balancing invariant reduces to a single scalar equation (`∑ value = 0`), which is extremely cheap to enforce anywhere, including SQL ([ADR-0019](0019-storage-layer-constraints-and-sqlite.md)). Valuation cannot drift between entries.
- **Bad:** Forces a primary currency choice on personal-finance users who might prefer genuinely independent dual-currency ledgers.

### Confirmation

The `Ledger` type holds exactly one functional currency. The posting builder statically rejects any posting where `value.commodity` diverges from it.
