---
status: ACCEPTED
date: 2026-08-22
decision-makers: [pmsoltani, Claude]
---

# ADR-0025: Rounding half-up away from zero, for derivation only

## Context

[ADR-0004](0004-exact-decimal-numerics.md) and [ADR-0005](0005-commodity-bearing-postings.md) dictate that derived values use `round(amount * rate)`, but omit _how_ to round a trailing 5. The default cannot safely be inherited from our underlying dependencies (e.g., `rust_decimal`), as its internal methods contradict one another (`.rescale()` uses half-away-from-zero; `.round_dp()` uses half-even). We must explicitly define the engine's rounding behavior to ensure reproducibility, statutory compliance, and mathematical symmetry during transaction reversals.

## Decision

**Core rounds half-up away from zero (`MidpointAwayFromZero`), set by a private constant, and used when deriving a posting's amount or value.**

- **Symmetric half-up:** Standard commercial and tax accounting usually expect half-up rounding, unlike the half-even (Banker's rounding) common in engineering (IEEE 754). Symmetry about zero (`round(-x) == -round(x)`, i.e., being an odd function) is non-negotiable; standard programmatic half-up (rounding towards positive infinity) is rejected as it causes reversals [ADR-0012](0012-draft-posted-lifecycle-append-only.md) to leave phantom balances.
- **One private constant:** The rounding mode is never exposed as a configuration, a function parameter, or a field on `Commodity`. A ledger with inconsistent internal rounding cannot be audited.
- **Scope is derivation only:** The core only applies this rounding when applying a `rate` to the posting's `amount` or `value` to calculate the other one. Rounding for reports, invoices, or specific tax filings is a presentation concern [ADR-0003](0003-keep-the-core-small.md) and left to extensions, which may implement their own localized rules.
- **Explicit naming:** Core code must explicitly call `round_dp_with_strategy`. Relying on `round_dp` is forbidden as it uses half-even by default.
- **Only the `value` side reports a residue:** rounding `(amount * rate)` returns the signed remainder as a bare `Decimal`, instead of throwing it away. A derived amount returns none as its rounding falls on the posting's own commodity, which does not need to balance out to zero.

## Consequences

- **Good:** An accountant recomputing a line by hand gets the engine's exact answer, aligning with standard commercial expectations.
- **Good:** A reversal exactly cancels its original; no residue accumulates from corrections.
- **Bad:** Half-up rounding carries a known, systematic upward bias across large datasets, accepted knowingly: any accumulated drift lands on an explicit rounding line [ADR-0005](0005-commodity-bearing-postings.md) where it is visible as a balance, rather than hidden in the math.
- **Neutral:** Institutions requiring half-even for specific regulatory filings must implement that rounding in their reporting layer.

## Confirmation

Property tests assert `round(-x) == -round(x)` over arbitrary values and scales, and `value + discarded == amount * rate` exactly. A pinning test proves the strategy is applied correctly and ignores the crate default (e.g., `round_dp_with_strategy(0.005, 2) == 0.01`, not `0.00`). The constant is private to the module that rounds, so that the only public route to rounding is the posting constructor.
