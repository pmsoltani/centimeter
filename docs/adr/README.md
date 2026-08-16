# Architecture Decision Records

This directory is centimeter's decision record. Each file captures one architectural decision: why it was made, what else was considered, and what had to be given up for it. The format is [MADR](https://adr.github.io/madr/); the conventions are set out in [ADR-0001](0001-record-architecture-decisions.md) and the blank template is [`adr-template.md`](adr-template.md).

## Index

| #    | Title                                                                                                        | Status   | Date       | Summary                                                                                  |
| ---- | ------------------------------------------------------------------------------------------------------------ | -------- | ---------- | ---------------------------------------------------------------------------------------- |
| 0001 | [Record architecture decisions as MADR ADRs](0001-record-architecture-decisions.md)                          | ACCEPTED | 2026-08-09 | Decisions live one per file in MADR format; superseded ADRs are never rewritten.         |
| 0002 | [Rust for the core engine](0002-rust-for-the-core.md)                                                        | ACCEPTED | 2026-08-09 | Invariants in the type system, no performance ceiling, one core embeds everywhere.       |
| 0003 | [Keep the core small: what stays out](0003-keep-the-core-small.md)                                           | ACCEPTED | 2026-08-09 | Invoicing, tax, reporting and import are layers. The rule for deciding new cases.        |
| 0004 | [Exact decimal numerics with per-commodity precision](0004-exact-decimal-numerics.md)                        | ACCEPTED | 2026-08-09 | `rust_decimal`, scale on the commodity, never floats, no engine epsilon.                 |
| 0005 | [Commodity-bearing postings and the triple](0005-commodity-bearing-postings.md)                              | ACCEPTED | 2026-08-09 | Exact dimensionally, to scale numerically; residue goes to an explicit rounding line.    |
| 0006 | [Balance in one functional currency](0006-one-functional-currency-per-ledger.md)                             | ACCEPTED | 2026-08-09 | `∑ value = 0` is one scalar equation, fixed at the ledger level.                         |
| 0007 | [Enforce balance at one commit boundary](0007-enforce-balance-at-one-commit-boundary.md)                     | ACCEPTED | 2026-08-09 | A staging builder checks before persisting; reject, never auto-repair.                   |
| 0008 | [The core transaction model](0008-core-transaction-model.md)                                                 | ACCEPTED | 2026-08-09 | One balanced-entry type, signed amounts; dr/cr converted at both edges, never stored.    |
| 0009 | [A typed account tree with five fixed roots](0009-typed-account-tree-five-roots.md)                          | ACCEPTED | 2026-08-16 | Five IFRS elements, closed; the chart holds the type; an account never changes it.       |
| 0010 | [Multi-commodity accounts with per-commodity balances](0010-multi-commodity-accounts.md)                     | ACCEPTED | 2026-08-09 | No `commodity` field on `Account`; a balance query returns a set, not a scalar.          |
| 0011 | [Record IDs: UUIDv7 inside, TypeID on the wire](0011-record-ids-uuidv7-typeid.md)                            | ACCEPTED | 2026-08-09 | `txn_01h455...` stored as TEXT; typed IDs; injectable generation; prefix rules.          |
| 0012 | [Draft-to-posted lifecycle, correction by appending](0012-draft-posted-lifecycle-append-only.md)             | ACCEPTED | 2026-08-09 | Drafts edit freely; posted entries are corrected by reversal or adjustment.              |
| 0013 | [Rates are a distinct type, with price history](0013-rates-as-a-distinct-type-with-history.md)               | ACCEPTED | 2026-08-09 | Two commodities, not scale-constrained, arithmetic not closed; zero/negative allowed.    |
| 0014 | [Provenance and the opaque actor reference](0014-provenance-and-the-opaque-actor-reference.md)               | ACCEPTED | 2026-08-09 | `created_by` is an opaque TypeID inside the hash; the injected clock.                    |
| 0015 | [Balances are derived from postings](0015-balances-are-derived-from-postings.md)                             | ACCEPTED | 2026-08-09 | The stream is the sole authority; no balance field; close-anchored checkpoints deferred. |
| 0016 | [Assertions in core, reconciliation as an extension](0016-assertions-in-core-reconciliation-as-extension.md) | ACCEPTED | 2026-08-09 | Participation in an invariant, not usefulness for diagnosis, decides core membership.    |
| 0017 | [The extension attachment model](0017-extension-attachment-model.md)                                         | ACCEPTED | 2026-08-09 | Side relations keyed by stable ID, plus a lazy resolver registry. Core stays closed.     |
| 0018 | [Parties: one unified type with roles](0018-parties-unified-type-with-roles.md)                              | ACCEPTED | 2026-08-09 | Roles as data, not disjoint types; details snapshotted onto documents.                   |
| 0019 | [Storage layer constraints, and SQLite](0019-storage-layer-constraints-and-sqlite.md)                        | ACCEPTED | 2026-08-09 | Row CHECKs and GLOBs; balance the draft-to-posted transition.                            |
| 0020 | [Cargo virtual workspace structure](0020-cargo-workspace-structure.md)                                       | ACCEPTED | 2026-08-09 | The tree, the facade, growth triggers, and the silent-failure traps.                     |
| 0021 | [License under MIT OR Apache-2.0](0021-dual-license-mit-apache.md)                                           | ACCEPTED | 2026-08-09 | Patent grant from Apache, GPLv2 compatibility from MIT; permissive is deliberate.        |
| 0022 | [Errors as values: per-domain enums](0022-errors-as-values-per-domain-enums.md)                              | ACCEPTED | 2026-08-09 | Domain enums plus a thin transparent root; `Result` versus `panic!` layering.            |
| 0023 | [Testing strategy](0023-testing-strategy.md)                                                                 | ACCEPTED | 2026-08-16 | Inline units, one integration binary per crate, property tests, the gates.               |
| 0024 | [Commodity identity, and the registry](0024-commodity-identity-and-registry.md)                              | ACCEPTED | 2026-08-11 | Identity is the id; the registry is the only constructor; code and scale are frozen.     |

## Reading order for newcomers

These are not all equally important. The first six steps are the ledger model, and they are the ones worth reading closely.

1. **How the record works**: ADR-0001.
2. **What the core is and is not**: ADR-0002 (Rust), ADR-0003 (what stays out).
3. **The numeric foundation**: ADR-0004. Everything downstream depends on it.
4. **The central model decision**: ADR-0005, the commodity-bearing posting, and the `amount * rate = value` triple. Most of the rest follows from this one.
5. **Balancing**: ADR-0006 (which currency), ADR-0007 (where it is enforced), and ADR-0008 (one entry type, signed amounts).
6. **Structure and integrity**: ADR-0009 and ADR-0010 (accounts), ADR-0011 (identity), ADR-0024 (commodities), ADR-0012 (lifecycle), ADR-0013 (rates), ADR-0014 (provenance), ADR-0015 (balances are derived), and ADR-0016 (assertions).
7. **Extensibility**: ADR-0017 (the attachment model), and ADR-0018 (parties, the first extension, and the proof that the model works).
8. **Engineering convention**: ADR-0019 (storage), ADR-0020 (workspace), ADR-0021 (licensing), ADR-0022 (errors), ADR-0023 (testing).

## Living documents

These change as a routine consequence of doing the work, so they are deliberately **not** ADRs. An ADR records a decision and its reasoning; a registry, a schema, or a file listing belongs in a living document and is linked from the ADR that sets its rules.

- [../glossary.md](../glossary.md)
- [../id-prefixes.md](../id-prefixes.md)
