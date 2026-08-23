---
status: ACCEPTED
date: 2026-08-22
decision-makers: [pmsoltani, Claude]
---

# ADR-0005: Commodity-bearing postings and the `amount * rate = value` triple

## Context

Postings must support multi-currency, securities, and non-currency quantities (like hours or liters) uniformly, without branching into separate subsystems. The conversion rate used must be reproducible directly from the transaction, and the exact balancing invariant must be maintained. However, exact decimals do not make the triple exact. A commodity's scale caps what it can represent, so a product such as `33.33 EUR * 0.8567 = 28.553811 GBP` has no representation in a scale-2 currency. This decision, therefore, has to say precisely which half of the equation is exact and where each kind of remainder goes.

## Decision

**The posting is commodity-bearing** and holds the value triple `amount * rate = value`:

```plaintext
amount: [X]   the posting's specified commodity (USD, hr, BTC, AAPL)
rate  : [Y/X] a compound conversion unit        (USD/hr, USD/BTC; GBP/GBP = 1)
value : [Y]   the balancing commodity           (the ledger's functional currency)
```

- **Dimensional Invariant:** `amount` is in the posting's specified commodity, `rate` is a conversion unit (`[value.commodity / amount.commodity]`), and `value` is in the ledger's functional commodity (better known as the functional currency, [ADR-0006](0006-one-functional-currency-per-ledger.md)). The engine type-checks this dimensionally.
- **Numerical Derivation:** Where the engine derives the value, it stores `round(amount * rate)` at that scale.
- **Sub-unit Residue Is Reported:** The signed remainder `amount * rate - value` is returned alongside a derived value it. The remainder sits below the commodity's scale, so it is a bare `Decimal` and not a `Quantity`, see [ADR-0025](0025-rounding-half-up-derivation-only.md).
- **Storage & Provenance:** The authoritative pair is `(amount, value)`, stored and frozen at entry to prevent drifting. The `rate` is kept only as historical provenance.
- **No Free Legs:** The engine strictly rejects unbalanced entries with the exact residue reported. If a multi-line transaction produces a rounding residue, the caller must supply an explicit rounding line or allocate the remainder. The core will not auto-repair it via a blank "free leg."
- **Derivation Modes:** Users supply one or two members of the triple, and the engine derives the rest. (Examples below assume a GBP ledger, where `value` is always GBP).

| Mode | Given             | Derived  | Example                                                                               |
| :--- | :---------------- | :------- | :------------------------------------------------------------------------------------ |
| 1    | `amount`, `value` | `rate`   | A card statement showing both sides: `-120.00 USD` charged, `-94.50 GBP` billed.      |
| 2    | `amount`, `rate`  | `value`  | `3 HOUR` at `75.144 GBP/HOUR`, so `value = 225.43 GBP`.                               |
| 3    | `rate`, `value`   | `amount` | Spending a pinned budget: `500.00 GBP` at `0.80 GBP/USD` gives `amount = 625.00 USD`. |

- **Entailed Rates:** If `amount.commodity == value.commodity`, the `rate` is structurally provable as Identity. If commodities differ and no rate is supplied, the engine rejects the transaction rather than silently assuming parity.

| Mode | Given      | Condition                             | Derived                             | Example                                            |
| :--- | :--------- | :------------------------------------ | :---------------------------------- | :------------------------------------------------- |
| 4    | `amount`   | `amount.commodity == value.commodity` | `rate = Identity`, `value = amount` | The entire single-currency ledger.                 |
| 5    | `value`    | `amount.commodity == value.commodity` | `rate = Identity`, `amount = value` | Entering directly in functional-currency terms.    |
| n/a  | one member | Commodities **differ**                | **Rejected**                        | Booking 100 EUR against a GBP ledger with no rate. |

- **Rate Direction:** Rates are strictly normalized to the model direction on entry. Inverse rates are applied via direct division, as exact decimal inversion is lossy.

## Consequences

- **Good:** Single-currency, multi-currency, and non-currency quantities all use a single, unified code path.
- **Good:** Every posting's stored triple either reconciles by construction or is authoritative by construction. No leg's three members silently disagree.
- **Bad:** A user entering a pinned multi-line foreign-currency total must supply the rounding line themselves, or use an extension/app-layer to automate it. Core rejects without it.
- **Bad:** Sub-unit rounding is reported but still unrepresentable. An accountant recomputing a single line by hand will find fractions no account can hold.

## Confirmation

Dimensional checks are enforced at the posting constructor. Property tests assert that the stored `value` equals `round(amount * rate)` at scale, that no accepted transaction has a non-zero `value` sum, and that unbalanced sets reject with the exact residue. No posting type has an optional `value` field.
