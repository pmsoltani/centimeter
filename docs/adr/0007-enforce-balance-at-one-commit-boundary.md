---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0007: Enforce balance at one commit boundary, and reject rather than repair

## Context

After establishing `∑ value = 0` in [ADR-0006](0006-one-functional-currency-per-ledger.md), we must decide where to enforce it and how to handle failures. Some engines silently absorb imbalances into dedicated "imbalance" accounts, while others reject them at the commit boundary.

## Decision

Enforce balance at **one commit boundary through a staging posting builder**, and **reject** rather than repair.

- **Draft vs. Posted:** An unbalanced _draft_ representation is explicitly allowed, enabling users to freely edit. The _posted_ state is unrepresentable if violated; the posting transition itself executes the check, and it is the only way to reach the posted type.
- **Reject with Typed Errors:** Rejections return a typed error naming the exact mathematical residue. The engine never invents a rounding account or silently absorbs an imbalance to force a commit.
- **Placement:** The invariant check is pushed as close to persistence as feasible to protect the data from buggy application-layer write paths. The storage layer repeats the check as a trigger on the same transition ([ADR-0019](0019-storage-layer-constraints-and-sqlite.md)).

## Consequences

- **Good:** There is exactly one chokepoint in the codebase that decides whether a transaction balances.
- **Good:** A typed error naming the residue is instantly actionable, unlike an auto-generated entry in an `Imbalance-USD` account that must be hunted down later.
- **Bad:** Import pipelines cannot force a "nearly right" transaction into the posted state; they must produce drafts.

### Confirmation

Property tests assert any accepted posting set commits with exact zero, and rejections yield the precise residue. The SQLite suite verifies the database rejects exactly what the builder rejects.
