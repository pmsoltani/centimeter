---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0008: The core transaction model: one balanced-entry type, signed amounts

## Context

The fundamental shape of the core economic record must be defined. This includes whether different business events (invoices, bills, payments, etc.) require distinct tables and how a posting's direction (debit vs. credit) is physically represented.

## Decision

The core uses **one balanced-entry type** with **signed amounts**.

- **One Unified Type:** A single `Transaction` holds a date and a list of postings, becoming immutable once posted ([ADR-0012](0012-draft-posted-lifecycle-append-only.md)). It is the unit of atomicity and the anchor for provenance ([ADR-0014](0014-provenance-and-the-opaque-actor-reference.md)). Invoices and bills are external layers that _emit_ transactions; the core does not know the word "invoice" ([ADR-0003](0003-keep-the-core-small.md)).
- **Signed Amounts:** Postings store a signed `Quantity`, not separate `debit` and `credit` columns.
- **Sign Convention:** Debits are positive, credits are negative.
- **Presentation Only:** Debit and credit columns are strictly presentation and input concerns. They are converted at the edges (accepted in forms, derived in reports) but never stored.
- **Description Field:** The inclusion of a `description` field in the core transaction model is not decided in this ADR.
- **No Duration on Transactions:** All postings in the transaction occur on a single economic date. Time-spanning events (like quarterly rent) are modeled as multiple transactions moving through accrual/deferral clearing accounts.

## Consequences

- **Good:** A single table means no synchronization drift between parallel document types.
- **Good:** Signed amounts compose naturally with multi-commodity arithmetic, keeping the `∑ value = 0` check a plain sum. It makes invalid states (e.g., both debit and credit columns populated) unrepresentable.
- **Bad:** Document-specific fields have nowhere to live in core and must utilize the extension model ([ADR-0017](0017-extension-attachment-model.md)).
- **Bad:** Raw database rows lack explicit dr/cr columns, which accountants expect (mitigated by reporting views).

### Confirmation

The core exposes no `Invoice` or `Bill` type. No posting field is named `debit` or `credit`. A reporting-layer function converts signed postings to dr/cr pairs, and round-trip property tests ensure they map identically.
