---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0018: Parties as a unified extension type with roles

## Context

Accounting systems must track counterparties (customers, vendors, employees). We must decide if these are distinct structural types or one type with roles. Furthermore, we must determine if parties belong in the core and how to handle historical data when a party's address or name changes.

## Decision

**One unified party type with roles, strictly as an extension to the core.**

- **Layered as an Extension:** Parties do not participate in the core `∑ value = 0` invariant, so by [ADR-0003](0003-keep-the-core-small.md)'s rule they sit outside the core. They attach to postings via the extension model ([ADR-0017](0017-extension-attachment-model.md)).
- **Roles as Data:** "Customer," "Vendor," and "Employee" are data properties (roles) on a single Party record, not distinct struct types. This prevents the classic drift issue where a company that is both a vendor and a customer requires two desynchronized records.
- **Snapshotting for Documents:** When a business document (like an invoice) is issued, the party's details (name, tax ID, address) are snapshotted onto it. A party record can be freely updated later without rewriting historical document truth.

## Consequences

- **Good:** Dual-role counterparties exist as single, coherent records.
- **Good:** Writing this as the first official extension actively proves that the closed-core attachment model ([ADR-0017](0017-extension-attachment-model.md)) works.
- **Good:** Snapshotting preserves strict historical and statutory accuracy for reissued invoices.
- **Bad:** Because roles are data, role-specific required fields (e.g., requiring a tax ID for vendors but not employees) must be validated in application logic rather than the compiler's type system.

### Confirmation

`centimeter-core` contains no type named `Party` and no field referencing one. A test updates a party's registered address and asserts a previously issued document still renders the old one.
