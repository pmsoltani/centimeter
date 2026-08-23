---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0004: Exact decimal numerics with per-commodity precision

## Context

Every decision in `centimeter` rests on numeric representation. If the representation is not exact, balancing requires an arbitrary tolerance epsilon. Furthermore, different commodities (JPY, USD, BTC) have different natural precisions, and rounding residues must remain visibly auditable rather than silently absorbed.

## Decision

We will use **exact scaled decimals** via the `rust_decimal` crate, treating precision as a property of the commodity. Binary floating-point numbers are strictly forbidden.

- **The `Quantity` Type:** An amount is explicitly represented as `(number: Decimal, commodity: Commodity)`.
- **Scale belongs to the commodity:** `Commodity` holds a `scale`, capped at `rust_decimal`'s hard limit of 28 decimal places. Note that scale (decimal places) is different from precision (significant digits). If a commodity is declared at scale 28, it cannot represent numbers above roughly 7.9 due to the 96-bit mantissa limit.
- **Zero tolerance:** Tolerance is purely a presentation-layer concern. The engine stays mathematically exact. Any rounding residue must be recorded as an explicit posting, never hidden; where that residue goes is decided in [ADR-0005](0005-commodity-bearing-postings.md).
- **No Floats:** The never-floats rule is enforced at the compiler level across the workspace using `float_arithmetic = "deny"` and `lossy_float_literal = "deny"`.
- **Cash-rounding:** Cash-rounding increments (e.g., nearest-0.05 tender rules) are deferred as a separate presentation concern, not a scale.

## Consequences

- **Good:** Balancing is a genuine equality test with no tolerance epsilons.
- **Good:** The compiler outright rejects float arithmetic, ensuring the rule never decays into mere discipline.
- **Bad:** `rust_decimal` limits us to a 96-bit mantissa (about 28 digits), meaning extreme-magnitude mixed-scale additions could theoretically drop precision.

## Confirmation

`cargo clippy` fails on any float arithmetic. Property tests verify that quantity arithmetic rigorously preserves per-commodity scale and catches overflows safely.
