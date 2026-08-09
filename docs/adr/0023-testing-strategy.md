---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0023: Testing strategy: inline units, one integration binary, property tests

## Context

Because the entire value proposition of `centimeter` is that the books are mathematically correct, the test suite is a core product component, not a support activity. We need a testing strategy that verifies private mathematical invariants, tests the public API ergonomically, and proves algebraic laws without exploding compile and link times.

## Decision

**A multi-tiered testing strategy emphasizing algebraic properties and strict integration boundaries.**

- **Unit Tests (Private Invariants):** Inline `#[cfg(test)] mod tests` placed directly beside the code. This is the only way to access and test private internals (like rounding and builder states) without exposing them.
- **Integration Tests (Public API):** Exactly **ONE** test binary per crate (`tests/it/main.rs` with `mod` declarations). Multiple top-level files in `tests/` trigger redundant linking steps, which severely inflates compilation time.
- **Property Tests:** First-class usage of `proptest` to prove algebraic laws (e.g., any accepted transaction commits with a net value of exactly zero, quantity arithmetic perfectly preserves scale, and a derived `value` always equals `round(amount * rate)` at the value commodity's scale ([ADR-0005](0005-commodity-bearing-postings.md))).
- **Parity Tests:** Where an invariant is enforced twice, a test proves the two agree. The SQLite suite asserts the database rejects exactly what the builder rejects ([ADR-0019](0019-storage-layer-constraints-and-sqlite.md)).
- **Fuzzing:** `cargo-fuzz` targets are isolated in a workspace-excluded `fuzz/` directory to avoid forcing nightly Rust on standard developers ([ADR-0020](0020-cargo-workspace-structure.md)).
- **Strict CI Gates:** Commits must pass `cargo test --workspace`, `cargo clippy --workspace --all-targets` (to catch test-code panics), and `cargo fmt --all --check`.

## Consequences

- **Good:** Private invariants are tested where they live, while public behavior is tested exactly as a consumer experiences it.
- **Good:** The single integration binary rule keeps link times flat as the project scales.
- **Bad:** Inline unit tests inflate source file length, adding a visual readability cost.

### Confirmation

Each crate's `tests/` directory contains exactly one top-level file, `it/main.rs`. CI runs the three gate commands on every commit.
