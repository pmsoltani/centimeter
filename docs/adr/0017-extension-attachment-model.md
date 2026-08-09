---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0017: The extension attachment model via stable ID side relations

## Context

To keep the core small and closed ([ADR-0003](0003-keep-the-core-small.md)), there must be a seamless way to attach domain data (parties, attachments, cost centers, etc.) without the core knowing those concepts exist. Adding a field like `posting.party` directly to the core schema destroys the dependency boundary.

## Decision

**Do not add fields to core records. Extensions own their own side-relations keyed by the core record's stable ID.**

- **The Primary Mechanism:** Extensions own their own storage tables keyed by the core ID (e.g., a `posting_party` table storing `posting_id` and `party_id`). This relies entirely on stable IDs, discussed in [ADR-0011](0011-record-ids-uuidv7-typeid.md).
- **The Fallback (Ad-Hoc Annotations):** An open `metadata` bag (JSON/Map) on core records is permitted for untyped, ad-hoc string annotations. It sits outside the integrity hash, so annotating a posted record does not break the chain.
- **Rejected (Dynamic Schema Extension):** We strictly reject core registration hooks that allow extensions to physically add columns to core tables. A closed core must remain statically closed.
- **The Registry:** Core provides a generic, extension-agnostic attachment registry. Extensions register lazy resolvers by name at startup (e.g., `registry.register("party", party_store.party_of)`).
- **Dependency Rule:** Extensions may depend on the core and other extensions. The core does not depend on any extensions, enforced by Cargo ([ADR-0020](0020-cargo-workspace-structure.md)).

## Consequences

- **Good:** Core physically cannot import an extension; the closed boundary is enforced by Cargo's dependency graph.
- **Good:** Independent extensions cannot conflict in storage because they share the ID _key_, not the _table_.
- **Good:** Lazy resolution means that core balancing never triggers an extension's SQL join.
- **Bad:** Attached data requires a join rather than a direct field read. If read performance becomes an issue, extensions must build their own denormalized read models.

### Confirmation

`crates/core/Cargo.toml` lists no internal dependencies, so no core record can name an extension type.
