---
id: story-elicitation-response-projection-wiring
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-elicitation-response-contract
depends_on: [story-elicitation-response-proto-messages, story-elicitation-response-core-validation]
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-19
---

# Story: ElicitationSlotLayer extension + server projection wiring

Checkpoint for `feature-v0-elicitation-response-contract` Unit 3. Makes the
validation path live end-to-end: the core's contract lookup reads a real
projection reconciled under the submit gate.

## Deliverable

1. Extend `ElicitationRecord` (in `core/src/acceptance/elicitation.rs`) to store
   the `response_contract` and `expected_responder_actor` from the opening
   event. `observe_elicitation` already decodes the full `Elicitation` message;
   it now also populates those two fields into the record.
2. Add `LockedElicitationContractLookup` to `server/src/state.rs`, mirroring
   the existing `LockedCommandStateLookup` wrapper pattern. It holds a
   `Mutex<ElicitationSlotLayer>` and implements `ElicitationContractLookup`
   (the port added in Unit 2).
3. Add an `elicitation_slots: LockedElicitationContractLookup` field to
   `ProjectionState`; fold every event through it in both `rebuild` and
   `catch_up` (alongside the existing three projections).
4. Pass `self.state.elicitation_contract_lookup()` into `acceptance::submit`
   in `server/src/service.rs::submit`.

Full signatures in the feature body Unit 3.

## Acceptance evidence

- [ ] `ElicitationRecord` stores `contract: Option<ResponseContract>` +
      `expected_responder_actor: Option<ActorId>`; `observe_elicitation`
      populates them from the opening `Elicitation` event.
- [ ] `LockedElicitationContractLookup` implements `ElicitationContractLookup`
      (serves `ActiveElicitation { contract, is_terminal }` from the slot).
- [ ] `ProjectionState` holds the new projection; `rebuild` + `catch_up` fold
      events into it alongside the existing three projections.
- [ ] `service.rs::submit` passes the new lookup into `acceptance::submit`.
- [ ] **Fold-lag invariant (property test):** after `catch_up` folds the
      Elicitation opening event, `active_contract` returns the real contract;
      before it, returns `None`. This is the race-free reasoning in D6.
- [ ] Existing `acceptance_elicitation.rs` first-answer-wins / terminal-race
      tests still pass unchanged after the `ElicitationRecord` field additions.
- [ ] `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` pass.

## Notes

- **Fold-lag race-free reasoning.** `submit` holds `submit_guard` for the whole
  validate→append→catch-up sequence (it already does). `catch_up` folds all
  events with `LSN > last_applied` into every projection *before* the
  validation read, so the contract lookup sees every event durably committed
  before this submit — including the Elicitation opening event the response
  targets. The contract lookup never reads a stale projection relative to the
  durable log. Same invariant the existing `state_lookup` relies on for
  deduplicated-retry state lookup.
- The `ElicitationSlotLayer` is event-log-driven and owns no storage;
  extending it to hold the contract keeps that property. The server-side
  projection is a cache of the durable Elicitation event, never authority.
- `is_terminal_state` already exists in `core/src/acceptance/state.rs`.
- The `Locked*` wrappers each hold a `Mutex` and release before the next port
  is called (existing "no nested projection locks" discipline). The new
  wrapper follows the same rule.
- A response submitted referencing an Elicitation whose opening event is not
  yet in the projection rejects as `validation_failed` (unknown elicitation) —
  this is correct Fail-Fast behavior, not a bug.

## Implementation notes

Extended `ElicitationRecord` with the opening response contract and expected
responder actor. Added the locked contract lookup projection, folded it during
rebuild and catch-up, and passed it through the server submit path. The fold-lag
invariant test confirms lookup returns `None` before the opening event is
observed and the real non-terminal contract afterward.

`ResponseContract` is retained as generated `PartialEq` rather than `Eq`
through the projection because protobuf timestamp-containing fields are not
`Eq`; this is the same mechanical constraint recorded by the core-validation
story.

Verification: `cargo test --workspace` and `cargo clippy --workspace
--all-targets -- -D warnings` pass.
