# centimeter glossary

**Living document.** Plain-language definitions of the vocabulary used across the ADRs and the code, for readers who learned accounting by doing rather than in a classroom.

Terms recurring in the ADRs are defined here. An ADR should link rather than re-explain.

## The structural pieces

- **Ledger:** The whole set of records for one _reporting entity_: its chart of accounts, its posting stream, its functional currency, its lock dates.
- **Reporting entity:** The accounting sense of "entity", the unit whose financial position the records describe (a company, a person). Maps to exactly one `Ledger`.
- **Chart of accounts:** The tree of accounts belonging to one ledger, holding the five fixed roots.
- **Account:** A bucket that value sits in or flows through (e.g., a bank account, an expense line). Organized in a tree, taking its type from its root. Accounts are not restricted to a single commodity and may hold multiple commodities simultaneously.
- **Account type / element:** The role an account plays: **Asset**, **Liability**, **Equity**, **Income**, or **Expense**. Closed at five by the IFRS Conceptual Framework. Fixes how an account reports and its normal balance direction.
- **Commodity:** A unit of measure of value (e.g., GBP, USD, AAPL shares, hours). Has its own `scale`. There is no separate `Currency` type.
- **Posting:** One line of a transaction, being an `amount` of a `commodity` in an `account`.
- **Transaction:** A group of postings that together balance to zero. Atomic in time: happens on one date, never over a range.

## Making it balance

- **Double-entry:** Every transaction moves value _from_ somewhere _to_ somewhere, so its postings sum to zero.
- **Debit / credit:** The traditional names for a posting's two directions. Stored as a signed amount (**debits positive, credits negative**) and converted at the edges (input forms and reports).
- **Normal balance:** The side an account's balance usually falls on. Assets and Expenses normally fall on the **debit** side, while Liabilities, Equity, and Income normally fall on the **credit** side.
- **Accounting equation:** Assets = Liabilities + Equity. This defines the arithmetic of the five roots.
- **Balance:** The net of all postings in an account, per commodity. **Always derived**, never stored authoritatively.
- **Trial balance:** A report listing every account with its total debits and credits at a point in time. The expensive query to design against because it touches every account.
- **Nominal vs. real accounts:** Income/Expense are _nominal_ (zeroed at year-end). Asset/Liability/Equity are _real_ (carry forward).
- **Imbalance account:** An anti-pattern this project rejects (e.g., absorbing errors into `Imbalance-USD`). centimeter strictly rejects unbalanced entries.

## The value of a posting

- **amount / rate / value:** The core triple (`amount * rate = value`). `amount` is in the posting's specified commodity, `value` is in the ledger's functional currency, and `rate` bridges them. The authoritative pair `(amount, value)` is stored; `rate` is kept only as provenance and is never a check. The equation is exact **dimensionally** but holds only **to the value commodity's scale** numerically.
- **Quantity:** "A number with a commodity" (`1.50 USD`), the shape shared by `amount` and `value`.
- **Rate:** A general ratio between any two commodities, with a **base** (converted from) and a **quote** (converted to). Unconstrained by scale and permitted to be zero or negative.
- **Rate direction:** A rate is written `quote/base`, read as a fraction: `JPY/USD = 150` is 150 JPY per USD. Market FX quoting reverses the naming order, so `EUR/USD` on a trading screen means USD per EUR, with EUR as the base. The two notations look identical and name the commodities in opposite orders, which is the likeliest way to enter a rate backwards.
- **Price:** A special case of a rate where one of the commodities is a currency (typically the ledger's functional currency).
- **Scale:** The number of decimal places an _amount_ in a given commodity may have. Binds stored amounts and values only.
- **Functional currency:** The one currency a ledger balances in, fixed at the ledger level.
- **Posting currency:** The currency a given leg is denominated in.
- **Presentation currency**: The currency that a _report_ is denominated in, which may differ from the ledger's functional currency.
- **Derive-and-freeze:** Computing a derived member of the triple **once** at entry and storing the rounded result permanently to prevent downstream drift.
- **Rounding line:** An explicit visible posting absorbing a rounding residue.

## Lifecycle and integrity

- **Draft vs posted:** A draft is freely editable; _posting_ finalizes it, making it append-only. The transition is one-way.
- **Reversal / adjustment:** The correct way to fix a _posted_ entry. An adjustment posts the delta; a reversal posts the mirror entry and then the correct one. The original is untouched.
- **Economic date vs entry time:** The **economic date** is when the event happened (used for lock dates and sorting). **Entry time** is when the row was written.
- **Reconciliation:** Comparing the ledger against an outside authority (e.g., a bank statement). Implemented strictly as an **extension**, not core, as it consists of mutable per-posting metadata.
- **Balance assertion:** A user-authored, verifiable claim written into the ledger (e.g., "account X on date D equals N"). Kept in **core** because it actively verifies the core's own balancing invariant.
- **Checkpoint:** An **engine-generated** stored balance at a date, used for read performance.
- **Lock date:** A date before which no posted transaction may be created or altered. Required to make checkpoints provably valid.
- **Hash chain:** An optional tamper-evidence seal linking posted entries. Covers only economic fields, allowing mutable metadata (like reconciliation flags) to change.
- **Provenance:** Authorship records (`created_by`, `created_at`). Stored directly on the transaction and sealed inside the hash chain. Core uses an opaque TypeID string and never knows the user's identity; an extension resolves it.

## Documents and attachments

- **Source document:** The accounting sense: real-world evidence (a paper invoice or a receipt).
- **Business document:** The software sense: a record type (like an Invoice or Bill) that **emits** balanced transactions.
- **Attachment:** A file stapled to a transaction. An extension-owned side relation, freely mutable.

## Extensibility

- **Core:** The closed engine (model + balancing invariant). Depends on nothing.
- **Extension:** A separately installable package that attaches data to the core records without the core knowing it exists.
- **Attachment model:** The rule that an extension holds a **side relation keyed by a core record's stable ID**, rather than adding a field to a core record.
- **Composition root:** The application or CLI that knows the full set of extensions and wires them together.
- **Resolver registry:** The extension-agnostic place where extensions register lazy lookups by name.

## Identity

- **`Id<T>`:** The code-level typed identifier. Generic over a marker type, making `Id<Account>` and `Id<Posting>` compile-time distinct.
- **`Identifiable`:** The trait a type implements to have an `Id`, supplying its `const PREFIX`.
- **UUIDv7:** RFC 9562 timestamp-based identifier providing B-tree locality.
- **TypeID:** The wire/storage format (`<prefix>_<26-char Crockford base32 UUIDv7>`). Stored as `TEXT` all the way down so database rows are self-describing.
