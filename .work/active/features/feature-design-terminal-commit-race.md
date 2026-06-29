---
id: feature-design-terminal-commit-race
kind: feature
stage: drafting
tags: [protocol, verification]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Design: command terminal-commit race resolution

The "first durable terminal commit wins" race rule is currently committed v0 behavior in `docs/PROTOCOL.md` (cancellation/expiration/supersession race semantics), but it was decided inside a prose consolidation feature (`feature-command-state-ssot`) without a design pass over the alternatives. This feature reopens it as a deliberate design decision.

## What is under design review

The rule as currently committed:

> First durable terminal commit wins. The core assigns a total order to accepted state-transition events in the durable event log; the earliest committed valid terminal transition becomes authoritative. If two terminal candidates are truly concurrent before persistence, models may treat the winner as nondeterministic, but implementations must persist one total order and expose the chosen terminal state consistently in snapshots and conformance traces. Later conflicting events are audit/reconciliation events, not state rewrites.

## Alternatives to evaluate

- **First durable terminal commit wins** (current) — simplest; relies on LSN total order; late events are audit-only.
- **Last durable commit wins** — allows later events to override; simpler reconciliation but can rewrite history the operator saw.
- **Priority-ordered resolution** — e.g. operator cancellation always wins over adapter completion, or vice versa; more predictable per-stakeholder but encodes priority policy in the core.
- **Explicit conflict surface** — surface concurrent terminal candidates to the operator as a distinct state rather than silently resolving.
- **Hybrid** — first-commit-wins for most cases, priority override for specific command kinds (e.g. safety-critical cancellation).

## Design questions to resolve

- Which failure modes actually produce truly concurrent terminal candidates in v0's single-writer model? (If none, the rule is theoretical and the choice is low-stakes.)
- Does the rule interact correctly with idempotent retry — i.e. can a retry land after a terminal commit and create a false "later event"?
- How does this interact with revocation policy on already-accepted commands?
- Does the formal model need to expose the nondeterministic case, or is v0's single-writer guarantee enough to make it deterministic in practice?
- Does the choice affect the generated contract or conformance vectors materially?

## Relationship to committed docs

The rule is currently committed in `docs/PROTOCOL.md` and referenced in `docs/VERIFICATION.md` (operator intent delivery, idempotent retry). A design pass either ratifies the rule as-is (and the note is removed) or revises it (and the docs roll forward). The rule stays as committed v0 behavior until the design pass concludes.

## Acceptance criteria

- The race resolution rule is a deliberate design choice, not a prose artifact.
- The chosen rule is documented with its rationale and the alternatives considered.
- `docs/VERIFICATION.md` model obligations are consistent with the chosen rule.
- Conformance vectors for the terminal-commit race are identified (even if deferred for implementation).
