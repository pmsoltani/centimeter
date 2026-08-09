---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0015: Balances are derived from postings; no stored balance is authoritative

## Context

We must decide if an account's balance is stored or derived state. Pure derivation is safe but hits a performance wall at corporate scales (e.g., millions of postings). Conversely, storing running totals in-place risks silent divergence from the ledger's true economic record.

## Decision

**The append-only posting stream is the sole, permanent authority. Any balance is derived and must be recomputable from postings alone. No stored balance is ever authoritative.**

- **What a Balance Is:** A fact about `(account, commodity, point_in_time)`. Because an account may hold several commodities ([ADR-0010](0010-multi-commodity-accounts.md)), a query over one account returns a set of balances, not a scalar. There is no `balance` field on `Account`.
- **The Deferred Performance Strategy:** To handle massive scale, **period-close checkpoints** will eventually be implemented (e.g., `balance = checkpoint(t0) + SUM(postings since t0)`).
- **Validity tied to Lock Dates:** A checkpoint covering postings up to date `D` is only valid if a Lock Date strictly seals all dates up to and including `D`. Without a lock date, legal backdating would silently invalidate the checkpoint.
- **No Self-Certification:** A checkpoint anchor must _never_ be a user-authored balance assertion ([ADR-0016](0016-assertions-in-core-reconciliation-as-extension.md)). Assertions exist to verify the math; computing balances from them destroys the chain of trust.
- **Rebuild-from-Zero:** Rebuilding the cache from the start of the ledger is a first-class, tested operation. Checkpoints are strictly derived data that can be dropped and rebuilt at any time.

## Consequences

- **Good:** There is exactly one mathematical authority, making silent ledger divergence impossible.
- **Good:** The deferred checkpoint cache requires zero model changes to implement later. The persona that needs mutability (personal) is small enough for pure derivation, while the persona that needs performance (corporate) naturally uses the lock dates required for caching.
- **Bad:** A large corporate ledger will experience slow reports until the checkpoint cache is implemented. A ledger completely refusing to use lock dates will never get the cache speedup.

### Confirmation

`Account` has no `balance` field and no accessor for one. Rebuild-from-zero is a tested operation: a test drops every checkpoint, recomputes from the first posting, and asserts the results match the cached ones exactly.
