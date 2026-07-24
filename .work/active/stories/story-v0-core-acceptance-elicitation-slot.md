---
id: story-v0-core-acceptance-elicitation-slot
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: [story-v0-core-acceptance-replay]
created: 2026-07-12
updated: 2026-07-13
gate_origin: null
release_binding: v0.1.0
---

# Story: Elicitation-slot terminalization (A2 decoupled layer)

## Scope

Implement the Elicitation-slot layer as an independent event-log consumer (design decision Q4 = A2). Acceptance accepts `approval-response`/`elicitation-response` Operations as plain operations; this layer tails the log, sees response terminal `COMMAND_TRANSITION` events (via `correlations` → `ElicitationId`), and terminalizes the Elicitation slot with its own events. First-answer-wins is structural: the first response terminal event in LSN order wins the slot; later responses are `stale`.

## Units

- `core/src/acceptance/elicitation.rs` — `ElicitationSlotLayer`, `observe()`, slot state machine

## Key properties

- **ElicitationFirstAnswerWins** (stated-normative): first response terminal event in LSN order wins; later responses are `stale`.
- **ElicitationPendingFinality** (stated-normative): once terminal, the slot doesn't mutate.
- **Decoupling** (Ports & Adapters): acceptance knows nothing about `ElicitationState`.

## Acceptance criteria

- [ ] The slot layer terminalizes an Elicitation slot when a response Operation reaches terminal.
- [ ] First-answer-wins: the first response terminal event wins; later responses are `stale`.
- [ ] The slot layer is decoupled from acceptance (no direct call; observes the log).
- [ ] The slot layer reconstructs slot state from replay.
- [ ] The slot layer handles re-processing (idempotent observe — re-observing a terminal event is a no-op).

## Design reference

See `feature-v0-core-acceptance.md` § "Implementation Units" → "Unit 5".

## Implementation notes

- Files changed: `core/src/acceptance/elicitation.rs`, `core/src/acceptance/mod.rs`, `core/tests/acceptance_elicitation.rs`.
- Tests added: six integration tests covering response-terminal slot closure, first-answer-wins by LSN, event-consumer decoupling, live/replay equivalence, idempotent re-observation, non-terminal response handling, and generated initial-state use.
- Discrepancies from design: the finalized story's pure-consumer shape supersedes the parent sketch's generic storage-owning struct; `ElicitationSlotLayer` owns only its projection and `rebuild_slots_from_log` receives the storage port. Per the v0.1.0 brief, every correlated terminal response maps to `answered`; contract-aware explicit-denial mapping to `declined` remains a later refinement rather than failure-code guessing in this layer.
- Implementation discovery: direct-read only; the existing `CommandIndex` and `rebuild_from_log` folds fully established the integration pattern. Duplicate opening and already-terminal response events are idempotent no-ops, so replay cannot reset a terminal winner.
- Adjacent issues parked: none.
