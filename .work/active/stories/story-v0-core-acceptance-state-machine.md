---
id: story-v0-core-acceptance-state-machine
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: []
created: 2026-07-12
updated: 2026-07-12
gate_origin: null
release_binding: null
---

# Story: Command state machine and transition validation

## Scope

Implement the `CommandState` transition adjacency (the SSOT for allowed transitions, derived from `docs/PROTOCOL.md`) and the `apply_transition` function that enforces it. This is the pure state-machine core — no I/O, no ports. It is the single source of truth that both the acceptance pipeline (live) and the replay fold (recovery) call.

## Units

- `core/src/acceptance/state.rs` — `CommandRecord`, `is_terminal()`
- `core/src/acceptance/transitions.rs` — `allowed_transition()`, `apply_transition()`

## Key properties

- **TerminalFinality** (promoted): terminal states reject transitions out.
- **NoAcceptedToCompleted** (promoted): accepted→completed is not in the adjacency.
- **Corruption detection** (Fail Fast): a transition whose `from_state` doesn't match the current state is `CorruptLog`.

## Acceptance criteria

- [ ] `allowed_transition` matches the protocol table for all 9 states × 9 states.
- [ ] `apply_transition` rejects transitions out of terminal states (TerminalFinality).
- [ ] `apply_transition` rejects accepted→completed (NoAcceptedToCompleted).
- [ ] `apply_transition` rejects from_state mismatches (corruption detection).
- [ ] `is_terminal()` correctly identifies the 6 terminal states (completed/rejected/failed/expired/cancelled/superseded).

## Design reference

See `feature-v0-core-acceptance.md` § "Implementation Units" → "Unit 1".
