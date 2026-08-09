---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0013: Rates as a distinct type with history and provenance

> **A note on vocabulary.** In this ADR, we use the term **rate** to mean a general conversion factor between two distinct commodities. A **price** is a special case of a rate where one of the commodities is a currency (typically the ledger's functional currency).

## Context

Because `rate` bridges the `amount` and `value` in a posting ([ADR-0005](0005-commodity-bearing-postings.md)), we must decide if it is just another `Quantity` with a special unit or a separate type entirely. Rates obey different laws than standard quantities. We also must ensure that historical rates are perfectly reproducible from the transaction itself, while still supporting general market valuation and reporting.

## Decision

**`Rate` is a distinct type, stored on the posting as provenance, supplemented by a separate rate history table.**

- **The `Rate` Type:** An enum with two cases: `Identity` (1:1, same commodity) and `Conversion` (distinct commodities).
- **Mathematical Laws:** A rate holds two commodities (base and quote), is _not_ scale-constrained (unlike a `Quantity`, [ADR-0004](0004-exact-decimal-numerics.md)), and its arithmetic is not closed (adding two rates is meaningless).
- **Zero and Negative Rates:** Explicitly permitted to support valid economic realities (e.g., negative oil prices, fully depreciated asset transfers).
- **Storage and History:** The rate actually used is stored directly on the posting as _provenance_. A separate, dated rate history table handles general market valuation.

## Consequences

- **Good:** The type system actively prevents operations that are mathematically invalid for rates.
- **Good:** Zero and negative rates prevent the engine from rejecting valid, albeit unusual, economic events.
- **Good:** Historical rates cannot be retroactively broken because the effective rate is sealed on the posting.
- **Bad:** Users must learn the difference between the `Rate` and `Quantity` types.

### Confirmation

`Rate` exposes no `Add`, `Sub`, or `Neg` traits. Zero and negative rates are explicitly tested and accepted.
