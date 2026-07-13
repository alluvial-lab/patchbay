---
id: story-v0-core-acceptance-state-machine
kind: story
stage: review
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: []
created: 2026-07-12
updated: 2026-07-13
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

## Implementation notes

- Files changed: `core/src/acceptance/mod.rs`, `core/src/acceptance/state.rs`, `core/src/acceptance/transitions.rs`, `core/src/lib.rs`, and `core/tests/acceptance_state_machine.rs`.
- Implemented the protocol-derived 18-edge adjacency, fail-fast transition folding, terminal metadata capture, command-record construction, and public acceptance-module exports.
- Tests added: 8 integration tests, including the exhaustive 9×9 adjacency oracle and coverage for all 18 allowed transitions; all 60 `patchbay-core` tests pass.
- Discrepancies from design: generated prost enum fields are `i32`, so transition application validates and converts them before mutation; Rust's orphan rule prevents an inherent `impl OperationState`, so the ergonomic method is provided by the re-exported `OperationStateExt` trait; generated `Operation` implements `PartialEq` but not `Eq`, so `CommandRecord` derives the strongest supported equality trait. The accepted-event LSN is used in constructor corruption diagnostics but is not duplicated in `CommandRecord`, matching the designed field set.
- Verification: build, full package tests, clippy for all targets, and rustfmt check pass with `CARGO_HOME=/tmp/cargo-home`.
- Adjacent issues parked: none.
