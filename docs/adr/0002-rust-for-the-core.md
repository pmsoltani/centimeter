---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0002: Rust for the core engine

## Context

`centimeter` is a double-entry accounting engine intended to be embedded across multiple environments: as a native library, in a browser via WASM, from Python, from Node, and behind a C ABI. The core is small and correctness-critical (handling the model, balancing, exact numerics, validation, hashing, and assertions). We need a language that can enforce strict mathematical invariants while reaching all target platforms without requiring duplicate implementations.

## Decision

The core engine will be written in **Rust**.

Rust's sum types, exhaustive matching, and ownership model map perfectly onto accounting states, making illegal states unrepresentable rather than just unlikely (e.g., an unbalanced transaction can be typed differently than a balanced one). Rust also targets native, WASM, Python (via PyO3), Node (via napi-rs), and C ABIs from a single codebase.

**Scope Boundary:** All I/O, application logic, and UI churn must remain completely outside the core. Rust is expensive to write in these areas, and frontend languages are better suited for them. Extensions may be written in other languages across the FFI/WASM boundary.

## Consequences

- **Good:** The compiler enforces correctness invariants that other languages would leave to test coverage or discipline.
- **Good:** Reaches all binding targets without needing parallel implementations.
- **Bad:** Slower initial development speed and steeper learning curve compared to scripting languages.

## Confirmation

The core crate's manifest lists no I/O, async, or storage dependencies. `centimeter-core` compiles for `wasm32-unknown-unknown` without ambient-entropy configuration.
