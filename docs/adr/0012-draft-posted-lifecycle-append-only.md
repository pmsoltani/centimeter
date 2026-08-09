---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0012: Draft-to-posted lifecycle, and correction by appending

## Context

An audit trail requires that history not be rewritten. However, treating a ledger as purely immutable from creation makes fixing simple entry-time typos overly cumbersome. We need a system that offers the ease of a spreadsheet for personal users while providing statutory tamper evidence for corporate users.

## Decision

**A draft-to-posted state machine, where drafts are freely mutable and posted entries are corrected exclusively by appending.**

- **The Lifecycle:** A _draft_ transaction is freely editable and deletable. _Posting_ the transaction finalizes it, passing the balancing checks ([ADR-0007](0007-enforce-balance-at-one-commit-boundary.md)) and making it strictly append-only.
- **Corrections:** A posted entry is never edited. It is corrected via an _adjustment_ (posting the delta) or a _full reversal_ (posting a mirror entry, then the correct entry).
- **The `reverts` Link:** The correcting transaction stores a `reverts` link pointing to the original ID. The original record is never mutated to point forward.
- **Append, Never Insert:** Backdated entries are appended to the stream with a new ID and sorted by _economic date_ at query time. Storage never reorders rows.
- **Lock Dates:** Lock dates seal a period against new or altered posted transactions, evaluated against the economic date. (The exact semantics of day-of sealing are not settled here).
- **Tamper Evidence:** An optional, per-ledger runtime hash chain (SHA-256/512) with a gapless sequence. Integrity hashes cover only economic fields, allowing mutable metadata (like reconciliation marks) to change without breaking the chain.

## Consequences

- **Good:** The common case (typos) is fixed easily in draft, while the rare case (post-finalization correction) is rigorously audited.
- **Good:** The system scales seamlessly. Personal users leave hashes/locks off for a lightweight experience; corporate users turn them on for statutory compliance.
- **Bad:** A netted view ("what the books say now") and a full view ("everything ever posted") diverge, requiring reports to be explicit about which they display.

### Confirmation

The storage layer forbids `UPDATE` and `DELETE` on a posted transaction outright ([ADR-0019](0019-storage-layer-constraints-and-sqlite.md)), so append-only holds even against a handwritten `sqlite3` statement. A test mutates a field covered by the hash and asserts the chain breaks.
