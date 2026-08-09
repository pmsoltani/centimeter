---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0019: Storage layer constraints, and SQLite

## Context

Checking the balancing invariant exclusively at the application layer leaves the system one buggy write path away from corruption. We need an embedded, zero-configuration database to meet our embed-everywhere goals, and it must support strict cross-row constraints to physically block unbalanced or corrupted records from being persisted.

## Decision

**SQLite is the first and default backend, and it enforces core mathematical invariants directly in the database.**

- **Embedded SQLite (`centimeter-sqlite`):** The ledger is a single, zero-configuration file that users own, can copy, and can open with any tool.
- **Row-Local `CHECK` Constraints:**
  - _Zero Consistency:_ `amount = 0` strictly implies `value = 0`. (Note: because zero-rates are permitted ([ADR-0013](0013-rates-as-a-distinct-type-with-history.md)), the reverse is not always true).
  - _Dimensional Consistency:_ Rate commodities must exactly match the `amount` and `value` commodities ([ADR-0005](0005-commodity-bearing-postings.md)).
  - _TypeID Shape:_ Enforced via `GLOB` patterns on every ID column ([ADR-0011](0011-record-ids-uuidv7-typeid.md)).
- **Strict Non-Nullability:** Because the core does not permit "free legs" ([ADR-0005](0005-commodity-bearing-postings.md)), every posting has a fully resolved `amount` and `value` at entry. Both columns are `NOT NULL` in the database, closing the loophole where `NULL` values could bypass SQLite `CHECK` constraints.
- **Cross-Row Balance & Append-Only Triggers:** SQLite cannot defer triggers to the end of a transaction. Therefore, the `SUM(value) = 0` cross-row balance check is enforced via a trigger acting strictly on the draft-to-posted state transition (`UPDATE ... state -> 'posted'`). Companion triggers physically forbid `INSERT`, `UPDATE`, and `DELETE` on already-posted transactions, mechanizing the append-only promise.

## Consequences

- **Good:** The database survives buggy or malicious write paths (e.g., someone typing `INSERT` manually via the `sqlite3` CLI).
- **Good:** Zero operational burden for end users.
- **Bad:** Bulk imports cannot inject posted rows directly; they must insert drafts and transition them to trigger the balance check.

## Confirmation

The `centimeter-sqlite` test suite runs against in-memory SQLite and proves the database rejects exactly what the application builder rejects. The `amount` and `value` columns are explicitly defined as `NOT NULL`.
