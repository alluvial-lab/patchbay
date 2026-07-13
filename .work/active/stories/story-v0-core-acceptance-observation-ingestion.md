---
id: story-v0-core-acceptance-observation-ingestion
kind: story
stage: review
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: [story-v0-core-acceptance-state-machine, story-v0-core-acceptance-pipeline]
created: 2026-07-12
updated: 2026-07-13
gate_origin: null
release_binding: null
---

# Story: Observation ingestion and command-state reflection

## Scope

Implement `ingest_observation` — the adapter→core ingress. Durably records the `OBSERVATION` event, then derives a candidate command transition (if the Observation implies one) and emits a `COMMAND_TRANSITION` event. Late terminal candidates (command already terminal) are recorded as `stale_event` audit Observations — NOT transition events. The streaming/subscription layer is out of scope (protocol-seam).

## Units

- `core/src/acceptance/observation.rs` — `ingest_observation()`, `derive_transition()`

## Key properties

- **First-durable-terminal-wins** (stated-normative): the first terminal `COMMAND_TRANSITION` in LSN order wins; later candidates are `stale_event`.
- **TerminalFinality** (promoted): late candidates do not mutate terminal state (they're audit-only).

## Acceptance criteria

- [ ] `ingest_observation` durably records the `OBSERVATION` event for all `ObservationKind`s.
- [ ] A `result` Observation with no failure emits a `completed` transition.
- [ ] A `result` Observation with a failure emits the appropriate terminal transition (`failed`/`execution_outcome_unknown`).
- [ ] A `status` Observation emits a `running` transition (if not already running).
- [ ] A late terminal candidate (command already terminal) is recorded as `stale_event`, NOT a transition event.
- [ ] Non-transition Observations (event, delta) record without emitting a transition.
- [ ] The streaming/subscription/cursor/fan-out layer is NOT implemented here (reserved for protocol-seam).

## Design reference

See `feature-v0-core-acceptance.md` § "Implementation Units" → "Unit 3".

## Implementation notes

- Files changed: `core/src/acceptance/observation.rs`, `core/src/acceptance/mod.rs`, `core/tests/acceptance_observation.rs`.
- Tests added: 12 integration tests covering every ObservationKind, completed/failed/ambiguous result mappings, running and duplicate-running status, terminal stale candidates, non-transition observations, malformed domain rejection, and unknown-command evidence retention.
- Integration: introduced the read-only `CommandStateLookup` port for the replay-owned `CommandIndex`; transition events remain the durable authority and live index application remains the index/composition layer's responsibility.
- Fail-fast behavior: malformed authority domains are rejected before append; unknown commands, malformed transition correlations, and disallowed lifecycle transitions are rejected after preserving the raw Observation as durable evidence.
- Discrepancies from design: `committed_at` remains unset because this domain ingress has no clock port and must not fabricate durable-commit time. Repeated `running` status Observations are recorded without emitting duplicate transitions. The story's test suggestion allowed `accepted → running`, but the completed canonical state machine forbids that edge; ingestion preserves such an Observation and rejects the derived transition rather than writing a corrupt log event. `delivered → running` is covered.
- Dispatch: direct-read only; the integration surface was bounded to the storage port, generated contracts, and acceptance state machine.
- Verification: `cargo build -p patchbay-core`, `cargo test -p patchbay-core`, `cargo clippy -p patchbay-core --all-targets -- -D warnings`, and `cargo fmt -p patchbay-core -- --check` pass with `CARGO_HOME=/tmp/cargo-home`.
- Adjacent issues parked: none.
