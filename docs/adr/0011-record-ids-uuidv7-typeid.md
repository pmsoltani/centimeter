---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0011: Record IDs: UUIDv7 inside, TypeID format on the wire

## Context

Every core record needs a stable identifier. Identity is the critical seam for the extension attachment model ([ADR-0017](0017-extension-attachment-model.md)), side tables, reversal links, and hash chains. The ID scheme cannot change after shipping. It must support offline generation, sortability, and readability by external tools.

## Decision

**IDs use UUIDv7 internally, represented as TypeID strings on the wire and in storage.**

- **Inner bits (UUIDv7):** RFC 9562 (48-bit millisecond Unix timestamp + 74 random bits). This provides near-append B-tree locality and creation-order sorting. (Autoincrement integers are forbidden, as they prevent offline merging).
- **Wire and Storage Format (TypeID):** `<prefix>_<26-char Crockford base32>`, e.g., `txn_01h455vb4pex5vsknk084sn02q`. ID columns in SQLite are `TEXT`, not `BLOB`.
- **Typed IDs in Rust:** `Id<T>` is generic over a marker type implementing `Identifiable`, which supplies a `const PREFIX`. Passing an `AccountId` where a `PostingId` is expected is a compile-time error.
- **Injectable Generation:** Core stays pure. An `IdGen` is handed to the builder so core requires no ambient clock or entropy, making it deterministic and WASM-compatible.
- **Prefix Rules:** Short, lowercase, unique (ideally, but not strictly, 3 characters). First-party records use bare prefixes; third-party extensions must use a vendor-prefixed namespace (e.g., `acme_widget`).

## Consequences

- **Good:** IDs survive offline creation, syncing, and round-trips flawlessly.
- **Good:** Raw database rows and log lines are instantly self-describing, aiding auditability.
- **Good:** Compile-time prevention of record type confusion.
- **Bad:** `TEXT` keys are wider (~30 bytes vs. 16 bytes), replicating across all foreign keys and indexes. This is an accepted tradeoff for observability.

### Confirmation

Round-trip property tests assert `parse(render(id)) == id` for every registered prefix and that a TypeID string with the wrong prefix fails to parse into `Id<T>`. A compile-fail test proves `AccountId` cannot be passed where `PostingId` is expected. `centimeter-core` builds for `wasm32-unknown-unknown`, which is only possible because no ID is generated from ambient entropy.
