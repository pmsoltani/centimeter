---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0022: Errors as values: per-domain enums with a thin composing root

## Context

An accounting engine rejects inputs constantly (e.g., unbalanced transactions, mismatched commodities, invalid IDs). Because `centimeter` prioritizes rejection over auto-repair ([ADR-0007](0007-enforce-balance-at-one-commit-boundary.md)), error quality is an important product feature. Errors must be typed data so callers can match and react, and at the same time, a monolithic enum that grows endlessly and forces callers to handle unrelated domains should be avoided.

## Decision

**Errors are returned as values using a two-level enum architecture: per-domain enums composed into a thin root enum.**

- **Per-Domain Enums:** Each module owns its specific errors (e.g., `CommodityError`, `QuantityError`). Variant names drop the domain prefix to avoid stuttering (e.g., `ScaleTooLarge`, not `CommodityScaleTooLarge`).
- **Thin Composing Root:** The crate root composes these using `thiserror` with `#[error(transparent)]` and `#[from]`. This root enum adds no variants of its own but provides a single, convenient type for callers who don't need fine-grained matching.
- **Forward Compatibility:** Every error enum is marked `#[non_exhaustive]` so adding variants is not a breaking change.
- **Strings over Lifetimes:** Error fields use owned `String` rather than `&str` to avoid tying the error's lifetime to the input data.
- **`Result` vs. `panic!`:** `Result` is for invalid inputs a caller might legitimately encounter. `panic!` is reserved exclusively for programmer bugs in the calling code and must be explicitly documented with `# Panics`.

## Consequences

- **Good:** Callers can match precisely on a specific domain error or handle everything uniformly via the root, all without boilerplate conversion code.
- **Good:** `#[error(transparent)]` ensures the composed error reads exactly like the underlying domain error in logs.
- **Bad:** `#[non_exhaustive]` forces callers to use wildcard arms, removing compiler exhaustiveness checks on their end.

## Confirmation

Every error enum is tagged with `#[non_exhaustive]`, and the root enum's variants all have `#[error(transparent)]` and `#[from]`, so it declares no message text of its own. `cargo clippy` is configured to require a `# Panics` section on any public function that can panic ([ADR-0023](0023-testing-strategy.md)).
