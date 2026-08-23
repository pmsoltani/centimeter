---
status: ACCEPTED
date: 2026-08-16
decision-makers: [pmsoltani, Claude]
---

# ADR-0009: A typed account tree with exactly five fixed roots

## Context

Accounts need structure. Systems differ on whether accounts should be strictly typed and how many root elements exist. Challenges like France's PCG 8-class system, treating Cost of Goods Sold as a root, or tracking statistical quantities (hours/liters) often push designs toward arbitrary root counts or untyped naming conventions.

## Decision

**Five fixed roots, typed, with the chart of accounts holding the type rather than each individual account.**

- **Exactly Five Roots:** The tree is constrained to Asset, Liability, Equity, Income, and Expense, as defined by the IFRS Conceptual Framework.
- **Root Element Typing:** These five names are the elements (types), not the account names themselves. The actual root accounts can be named anything, but each one must be typed to one of these five elements.
- **The `RootAccounts` Pattern:** The type is not a field on the `Account` struct (which would admit invalid states, like a child contradicting its parent). Instead, the chart holds a `RootAccounts` value containing exactly the five root IDs. An account derives its type by walking up to its root.
- **An Account's Element Is Fixed at Creation:** Accounts can be reorganized within their own element (root account), but never moved to another element. Changing elements would alter how past reports are interpreted.
- **Dismissing the Challenges:**
  - _PCG 8-class system:_ This is a statutory coding layer, not structural elements. It is handled via an optional free-text `code` field.
  - _Cost of Goods Sold:_ This is an Expense. Wanting it adjacent to Revenue is a presentation requirement, not a structural one.
  - _Statistical quantities:_ Handled seamlessly as non-currency commodities ([ADR-0005](0005-commodity-bearing-postings.md)), requiring no special "statistical" root.
- **Deferred:** Off-balance-sheet items (contingent liabilities, custody assets) are deferred until a concrete use case appears.

## Consequences

- **Good:** Statement derivation, sign conventions, and nominal/real closing logic all have one authoritative, structural source.
- **Good:** The five-roots rule is unrepresentable when violated, rather than needing repeated validation.
- **Bad:** Deriving an account's type requires a tree walk to the root, though this is cheap at standard chart sizes.
- **Bad:** A chart cannot be reorganized across elements at all, so a misfiled account costs a replacement and a reversal rather than a drag in a UI.

## Confirmation

`Account` has no field naming an element, and `RootAccounts` holds exactly five IDs, one per element, so a sixth root cannot be constructed. A test asserts that an account's element is resolved by walking to its root; another ensures that a reparent whose destination belongs to a different root is refused.
