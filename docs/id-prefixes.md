# Record ID prefix registry

**Living document.** Unlike an ADR, this file gains a row whenever a record type is added. While the overall structure is defined in [ADR-0011](adr/0011-record-ids-uuidv7-typeid.md), the _list_ lives here so that adding a record type does not require amending an accepted ADR.

The word "record" is deliberate: the glossary reserves "entity" for its accounting meaning, the reporting entity whose financial records a `Ledger` holds.

## Finalized

Prefixes that exist in code and are now permanent.

| Record    | Prefix |
| --------- | ------ |
| Account   | `acc`  |
| Commodity | `cmo`  |
| Posting   | `pst`  |

## Draft

Proposed, not yet implemented, still changeable.

| Record                | Prefix |
| --------------------- | ------ |
| Attachment            | `att`  |
| Balance assertion     | `asr`  |
| Budget                | `bdg`  |
| Business document     | `doc`  |
| Checkpoint            | `cpt`  |
| Device                | `dev`  |
| Ledger                | `ldg`  |
| Party                 | `pty`  |
| Personal access token | `pat`  |
| Reconciliation mark   | `rec`  |
| Transaction           | `txn`  |
| User                  | `usr`  |

## Related

- [ADR-0011](adr/0011-record-ids-uuidv7-typeid.md): the scheme, the wire format, and the rules.
- [glossary.md](glossary.md): TypeID, UUIDv7, and the `Id` / `Identifiable` distinction.
