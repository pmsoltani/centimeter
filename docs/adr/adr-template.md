---
status: PROPOSED
date: { YYYY-MM-DD, when the decision was last updated }
decision-makers: [pmsoltani, Claude]
---

# ADR-NNNN: {Short title naming the problem and the chosen solution}

## Context

{Two or three sentences explaining the problem. What forced this decision? What are the driving factors? What breaks if it is not made? Keep it concise and directly outline the tension.}

## Decision

**{State the chosen option clearly and boldly}**.

{Detail the concrete rules, implementation specifics, and architectural boundaries. Use bullet points for readability. If a rejected alternative provides critical context, mention _why_ it was rejected here briefly, rather than listing out all considered options.}

## Consequences

- **Good:** {A desired quality this improves.}
- **Bad:** {A real cost, accepted knowingly. If nothing goes here, look harder.}
- **Neutral:** {A consequence that is neither, but which a reader would otherwise wonder about.}

## Confirmation

{How someone checks that the code actually complies. e.g., A specific compile-time property, a `cargo` command, or an exact test assertion. Not "review carefully".}

<!-- Conventions for this project, from ADR-0001. Delete this comment before saving.

- Filename `NNNN-kebab-case-title.md`, sequential, never reused.
- `status` is one of PROPOSED, ACCEPTED, REJECTED, DEPRECATED, or SUPERSEDED BY ADR-NNNN.
- Keep it concise: Strip out conversational fluff and bloated alternative-weighing of choices. Focus on Context, Decision, and Consequences.
- A superseded ADR is never rewritten. Change its status, point at the successor, and leave the original reasoning intact.
- A registry, a schema, or a file listing belongs in a living document, not here.
- Add a row to README.md's index upon creation.
-->
