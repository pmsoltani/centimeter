---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0020: Cargo virtual workspace structure and crate naming

## Context

To ensure the core truly depends on nothing ([ADR-0003](0003-keep-the-core-small.md)), the package dependency graph must physically enforce the boundary. We need a monorepo to allow atomic cross-cutting refactors during early development, but routine developer commands (like `cargo test --workspace`) must not require contributors to install Python, Node, or WASM toolchains.

## Decision

**A Cargo virtual workspace, organized as a monorepo of separately installable packages.**

- **Workspace Layout:**
  - `crates/`: Libraries (e.g., `crates/core`, `crates/sqlite`).
  - `apps/`: Composition roots (the only crates that know the full extension set).
  - `bindings/` and `fuzz/`: Standalone packages.
- **Workspace Exclusions:** `bindings/*` (Python/Node/C) and `fuzz/` are explicitly excluded from the main workspace members list and have their own lockfiles. This prevents FFI toolchain pollution during standard `cargo test` runs ([ADR-0023](0023-testing-strategy.md)).
- **The Facade Pattern (`centimeter`):** Consumers never depend on `centimeter-core` directly. They depend on the `centimeter` facade crate, which re-exports the core and extensions behind weak-dependency forwarding features (e.g., `sqlite = ["dep:centimeter-sqlite"]`).
- **Dependency Rule:** `[workspace.dependencies]` is the single version source. The core manifest is physically prevented from importing extensions.
- **Centralized Lints:** Floating-point math is banned workspace-wide via `[workspace.lints]` ([ADR-0004](0004-exact-decimal-numerics.md)). Member crates must explicitly opt-in via `[lints] workspace = true`.

## Consequences

- **Good:** The closed-core architecture is structurally verified by Cargo on every compilation.
- **Good:** Developer loops stay clean of nightly and FFI build requirements.
- **Bad:** Adding a new crate requires a strict checklist to avoid silently dropping lint inheritances or messing up lockstep versioning.

### Confirmation

`cargo tree -p centimeter-core` shows no internal dependency, so the boundary is checked on every build rather than by review. Every member crate's manifest contains `[lints] workspace = true`.
