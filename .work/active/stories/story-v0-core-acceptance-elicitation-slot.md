---
id: story-v0-core-acceptance-elicitation-slot
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: [story-v0-core-acceptance-replay]
created: 2026-07-12
updated: 2026-07-12
gate_origin: null
release_binding: null
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
