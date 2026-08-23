---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0016: Balance assertions in core, reconciliation status as an extension

## Context

Both balance assertions and reconciliation marks help users localize discrepancies by narrowing the search window. Because they serve similar diagnostic purposes, it is tempting to bundle them together. However, evaluating them against our strict core boundary rule ([ADR-0003](0003-keep-the-core-small.md)) reveals they are structurally opposites: one is an immutable mathematical claim that verifies the core's own balancing invariant, while the other is mutable human metadata. We must correctly separate them to preserve the integrity of the append-only record.

## Decision

**Balance assertions remain in the core. Reconciliation status moves to an extension.**

- **Balance Assertions (Core):**
  - An assertion (e.g., _account X on date D equals N_) actively verifies the core's own balancing invariant and must run at the same commit boundary ([ADR-0007](0007-enforce-balance-at-one-commit-boundary.md)).
  - It acts as an immutable, dated, append-only directive record included in the cryptographic hash chain.
  - _Recompute Rule:_ Assertions must recompute balances from the start of the ledger (or a proven anchor) and never trust an unverified checkpoint ([ADR-0015](0015-balances-are-derived-from-postings.md)).
  - _Absence Rule:_ An assertion against a non-existent account is an error, preventing a missing ledger segment from silently passing a zero-balance assertion.
- **Reconciliation Status (Extension):**
  - Marking a posting as "cleared" or "reconciled" against a bank statement is mutable, per-posting metadata.
  - It participates in no core invariant.
  - It is implemented via the standard extension mechanism ([ADR-0017](0017-extension-attachment-model.md)): a side relation keyed by `PostingId` owned by a separate `reconciliation` package.

## Consequences

- **Good:** The core's posted record becomes uniformly immutable, requiring no caveats for status flags.
- **Good:** Relegating reconciliation to an extension preserves the integrity hash chain (which only covers economic fields).
- **Bad:** Users who want standard reconciliation must explicitly add an extension crate.

## Confirmation

`centimeter-core` contains `assertions.rs` but no mutable status field on a posted posting.
