---
status: ACCEPTED
date: 2026-08-09
decision-makers: [pmsoltani, Claude]
---

# ADR-0001: Record architecture decisions as MADR ADRs

## Context

`centimeter`'s design was initially captured in a single living memo. As the project grows, a single document becomes unwieldy. Decisions can be easily amended in place, causing the historical rationale and rejected alternatives to be lost over time. We need a system that keeps decisions findable, preserves their reasoning, and handles long gaps in development without losing context.

## Decision

We will use markdown files to record architectural decisions, inspired by the [MADR](https://adr.github.io/madr/) format, keeping one file per decision under `docs/adr/`.

### Conventions

- **Filename:** `NNNN-kebab-case-title.md` (four-digit zero-padded, sequential, never reused).
- **Frontmatter:** Include `status`, `date` (YYYY-MM-DD of last update), and `decision-makers`.
- **Status values:** `PROPOSED`, `ACCEPTED`, `REJECTED`, `DEPRECATED`, or `SUPERSEDED BY ADR-NNNN`.
- **Immutability:** Superseded ADRs are never deleted or rewritten. Their status changes with a pointer to the successor, and the original reasoning remains intact. The successor explains the change.
- **Flexibility:** Optional MADR sections are genuinely optional. Omit anything that feels like padding.
- **Index:** [`docs/adr/README.md`](README.md) holds the index (number, title, status, date, and summary) and a recommended reading order for newcomers.

**What does _not_ belong in an ADR:**

- **Registries, schemas, or file listings:** These belong in living documents (e.g., an ID prefix registry). Requiring an ADR amendment for routine clerical work erodes the meaning of "accepted."
- **Open questions:** Track unresolved questions elsewhere, not inside accepted ADRs.

## Consequences

- **Good:** Newcomers can read isolated decisions governing specific code without parsing the entire project history.
- **Good:** The "why" behind a decision outlives the decision itself.
- **Bad:** The process of creating and maintaining ADRs can be time-consuming and may slow down the development process.

## Confirmation

A decision is officially recorded when its ADR file exists and is listed in [`README.md`](README.md)'s index. The index is complete when every `NNNN-*.md` file in this directory has a row and every row points at a file that exists.
