---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani]
---

# ADR-0021: License under MIT OR Apache-2.0

## Context

`centimeter` is built to be a library embedded everywhere: native, WASM, Python, Node, and C. The chosen license directly dictates whether adoption is legally possible for given consumers. We must accommodate corporate users demanding explicit patent grants while also supporting open-source ecosystems that demand GPL compatibility.

## Decision

**Dual-licensed under MIT OR Apache-2.0.** Consumers may choose the license that fits their legal requirements.

- **Apache-2.0:** Contributes an explicit patent grant, which corporate legal reviews typically require.
- **MIT:** Keeps GPLv2 compatibility, which Apache-2.0 lacks.
- **Permissive over Copyleft:** A copyleft license (like GPL/AGPL) is explicitly rejected. The core value proposition relies on being embedded deeply into other software (including proprietary software); copyleft would fundamentally break this goal.

## Consequences

- **Good:** Standard SPDX strings mean most consumers skip legal review entirely.
- **Good:** Both patent-grant and GPLv2-compatible audiences are served.
- **Bad:** A competitor could build a proprietary product on this work without contributing back. This is an accepted cost of maximizing adoption.

### Confirmation

Both `LICENSE-MIT` and `LICENSE-APACHE` exist at the repository root, and no member crate overrides `license` in its own `[package]` table. A published crate's metadata reads `MIT OR Apache-2.0`.
