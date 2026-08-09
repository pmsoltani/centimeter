---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0003: Keep the core small: what stays out

## Context

Accounting software ecosystems sit at two extremes: libraries with tiny cores that eject everything else and ERPs that bake invoicing, taxes, and reporting directly into the product. Because `centimeter` is a library first, a bloated core would force every consumer to pay for unused features in compile time, API surface, and semver commitments. Additionally, churn heavily concentrates in areas like tax rules and import formats.

## Decision

**Keep the core tiny.** The core consists exclusively of the model and the balancing engine: `ledger`, `transactions`, `postings`, `accounts`, `commodities`, `quantities`, `rates`, `identity`, the draft-to-posted lifecycle, and balance assertions.

**The Rule:** If a feature does not participate in the balancing invariant, it is a layer above the core.

Explicitly outside the core:

- **Invoicing, bills, and payments:** Documents that emit balanced transactions.
- **Tax computation:** Taxes produce postings; they are not core primitives.
- **Reporting and query:** Read models over the immutable stream.
- **Import (OFX/QIF/CSV/Plaid):** Endless per-institution format parsing.
- **Parties:** A unified type with roles, layered above the core ([ADR-0018](0018-parties-unified-type-with-roles.md)).
- **Attachments:** Side relations keyed by `TransactionId` (e.g., scanned receipts).
- **Reconciliation status:** Mutable per-posting metadata that participates in no invariant ([ADR-0016](0016-assertions-in-core-reconciliation-as-extension.md)).

**The Exceptions:** Here we make two deliberate exceptions to our strict core boundaries:

1. **Balance Assertions:** These remain in the core because they verify the core's own balancing invariant. A verification mechanism living outside the thing it verifies is ineffective; see [ADR-0016](0016-assertions-in-core-reconciliation-as-extension.md).
2. **Provenance:** While user management is an external domain, the core must store an opaque actor reference directly on the transaction. An audit trail is only trustworthy if the author's ID is sealed permanently inside the core's cryptographic hash chain; see [ADR-0014](0014-provenance-and-the-opaque-actor-reference.md).

## Consequences

- **Good:** The semver surface remains small and stable.
- **Good:** Highly dynamic features can be versioned independently of the engine.
- **Good:** The core remains small enough to be exhaustively fuzzed and property-tested, while still supporting a mathematically rigorous audit trail.
- **Bad:** Consumers will need facade crates and composition roots to build a fully usable app ([ADR-0020](0020-cargo-workspace-structure.md)).

### Confirmation

`crates/core/Cargo.toml` lists no internal dependencies and no I/O crates.
