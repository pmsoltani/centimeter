---
status: ACCEPTED
date: 2026-08-11
decision-makers: [pmsoltani, Claude]
---

# ADR-0024: Commodity identity, and the registry that issues it

## Context

`Commodity` is the foundational type that every other ledger record depends on. As such, it is important to decide how to compare two commodities for equality. One approach is to check equality over all fields, but this means that by changing the content (e.g., the name from "US Dollar" to "United States Dollar"), two independently built `USD` commodities would become unequal.

Furthermore, there is another design gap: nothing prevented a ledger from holding two conflicting definitions of `USD` simultaneously. We need to strictly define what makes commodities unique and strictly control who is allowed to create and keep track of them.

## Decision

**A commodity is identified by its ID alone, and only a registry may issue one.**

- **Identity is the ID:** `PartialEq`, `Eq`, and `Hash` are defined on [`CommodityId`](0011-record-ids-uuidv7-typeid.md) only. Every other field is a property of the commodity, not part of its identity.
- **The Registry is the Sole Constructor:** `Commodity` has no public constructor. `CommodityRegistry::add` validates the code, name, and scale, enforces uniqueness, and is the sole producer. Holding a `Commodity` instance is therefore proof that it is the ledger's only validated record for that `id` and `code`.
- **Dual Uniqueness:** No two commodities in a registry may share an ID, and no two may share a code. Codes are compared exactly and are case-sensitive (e.g., `USD` and `usd` are technically distinct, as are `mW` and `MW` for energy units).
- **One Registry per Ledger:** The registry is owned by the `Ledger`. It ships empty; the core attaches no meaning to codes and seeds no ISO 4217 currency table, pushing localization to layers above.
- **Frozen Code and Scale:** `scale` defines the immutable bounds of representable amounts, and `code` is the natural key for reporting. Neither may change after registration; a rescale or redenomination requires minting a completely new commodity. The `name`, being purely for display, may be corrected in-place.
- **Append-Only (No Removal):** A posted transaction references a commodity for the life of the ledger, and the core cannot cheaply prove a commodity is unreferenced. Therefore, `add` is the only way in, and there is no way out.
- **Reference by ID, Resolve via Registry:** Records store a `CommodityId`. Resolving it happens strictly at the boundaries (on load or posting construction) so the core never holds a stale copy of a commodity's scale or code.

## Consequences

- **Good:** A ledger cannot hold two conflicting records for one code, making the foundational failure mode unreachable.
- **Good:** `Rate` can now reliably decide `Identity` against `Conversion` by doing a cheap `base == quote` check on 16-byte UUIDs without touching the heap.
- **Good:** A stored amount is never retroactively reinterpreted, as its governing scale is strictly frozen.
- **Bad:** Correcting a mistyped code or scale requires registering a replacement and posting reversal transactions for anything written under the original.
- **Bad:** Anything rendering a commodity (including error messages) now needs the registry in hand to resolve the `CommodityId` string into a human-readable code.

### Confirmation

`Commodity::try_new` is private, making `CommodityRegistry::add` the only producer. A compile-fail test asserts the constructor is unreachable from outside the crate. Unit tests assert that two commodities sharing code, name, and scale but differing in ID compare unequal, and the registry exposes no `&mut self` methods other than `add` and a name correction.
