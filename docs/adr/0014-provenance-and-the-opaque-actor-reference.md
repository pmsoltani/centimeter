---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0014: Provenance on the record, and the opaque actor reference

## Context

An audit trail is only valuable if authorship cannot be dropped, altered, or separated from the record. However, user management is an external, churn-heavy domain that the core must not know about ([ADR-0017](0017-extension-attachment-model.md)). Furthermore, the core must remain pure and deterministic, lacking access to ambient clocks for timestamps.

## Decision

**The core directly stores provenance data (`created_at`, `created_by`), utilizing an opaque actor reference for user identity.**

- **Opaque Actor Reference:** Core stores `created_by` (a TypeID string, [ADR-0011](0011-record-ids-uuidv7-typeid.md)) directly on the transaction ([ADR-0008](0008-core-transaction-model.md)) and includes it in the cryptographic hash. Core never parses it, resolves it, or depends on the user type existing.
- **The Exception Justification:** This is a narrow, deliberate exception to the tiny core model ([ADR-0003](0003-keep-the-core-small.md)). If authorship lived in an extension's side table, deleting that table would erase the audit trail without breaking the hash chain. Trustworthiness dictates it must be inside the hash.
- **Injected Clock:** `created_at` requires an injected clock passed to the builder (like the injected ID generator).
- **GDPR Erasure:** Because the ledger holds only the ID and the extension holds the identity, a person can be erased by deleting the mapping in the extension. The ledger remains perfectly balanced and cryptographically intact, simply becoming unattributable.

## Consequences

- **Good:** Authorship attribution is as durable and tamper-evident as the financial amounts.
- **Good:** Core remains entirely ignorant of user management, and GDPR erasure works without breaking historical immutability.
- **Good:** The injected clock allows the entire engine to be deterministically replayed for testing.
- **Bad:** Core cannot cryptographically verify that the `created_by` prefix is registered or that the target user actually exists.

### Confirmation

`centimeter-core` contains no `User` type and no dependencies resolving one. A test builds the same transaction twice with a fixed clock and ID generator and asserts byte-identical hashes; another deletes the actor's row in the extension and asserts the chain still verifies.
