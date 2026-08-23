---
status: ACCEPTED
date: 2026-08-23
decision-makers: [pmsoltani, Claude]
---

# ADR-0026: Time in the ledger: dates for periods, instants for provenance

## Context

Every ledger entry must track its economic date (when the event occurred) and its technical provenance (when it was recorded). Relying on a single offset-aware timestamp to represent the economic event makes the accounting period a derived computation rather than a stated fact and forces the system to fabricate fake times for events that have no physical instant (like accruals, depreciation, and opening balances). Furthermore, adopting third-party datetime crates (`chrono`, `jiff`) in the core public API introduces foreign semver to consumers and risks breaking the `wasm32-unknown-unknown` target due to Cargo's additive feature unification.

## Decision

**The core implements separate integer fields for the economic date and the recording instant, with no third-party datetime types in the public API.**

- **`Date` (Economic Period):** The civil date of the transaction and the determinant of its accounting period. Represented as an `i32` holding days since the UNIX epoch (proleptic Gregorian, supporting years 1..=9999). The date parses and renders as `YYYY-MM-DD` and has no timezone and no time of day.
- **`Timestamp` (Provenance/Audit):** The moment the record was created. Represented as an `i64` holding milliseconds since the UNIX epoch (UTC by construction). It is injected by the caller's clock; the core never reads the wall clock.
- **Intraday Sequence:** Entries sharing a `Date` will require another field to be ordered by, whose form is decided separately.
- **Vendored Calendar Math:** The core implements the exact calendar conversion (integer to/from civil dates). Callers wanting formatting, locale, or month arithmetic must reach for a library outside the core.

**Extraneous Precision is Optional:** Because [ADR-0005](0005-commodity-bearing-postings.md) requires the caller to supply the authoritative `(amount, value)` pair, the core never needs an exact instant (moment) to fetch an intraday exchange rate. If an extension (e.g., crypto specific-identification) requires an exact execution instant, it will be added as an optional field (e.g., `occurred_at`) later, but it will never replace the civil `Date` as the period key.

## Consequences

- **Good:** The period key is a stated, unambiguous fact, matching statutory schemas that demand date-only fields.
- **Good:** The core naturally handles accruals, provisions, and opening balances without fabricating a fake time of day.
- **Good:** The public API remains free of foreign semver, and the `wasm32-unknown-unknown` target remains perfectly insulated from downstream feature-unification risks.
- **Bad:** Consumers already holding `chrono`, `jiff`, or `time` types must manually convert them to primitive integers at the core boundary.
- **Neutral:** The core takes on the burden of calendar conversion correctness, mitigating it via exhaustive testing.

## Confirmation

The vendored calendar conversion logic is exhaustively testable: a unit test round-trips every single day from year 1 to 9999 (3.65 million iterations) in a single loop. `Date` implements `Ord`, ensuring period comparisons are evaluated as a single, unambiguous integer comparison.
