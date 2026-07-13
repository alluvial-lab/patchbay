---
id: story-v0-core-acceptance-replay
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: [story-v0-core-acceptance-state-machine, story-v0-core-acceptance-pipeline]
created: 2026-07-12
updated: 2026-07-12
gate_origin: null
release_binding: null
---

# Story: Replay and in-memory index reconstruction

## Scope

Implement the `CommandIndex` (in-memory command record map, the hot lookup path) and the `apply` fold that reconstructs it from the event log on recovery. This is the domain-layer `apply` that the persistence feature's `recover()` hands off to — it completes the `IdempotentLogReplay` stated-normative obligation end-to-end. Includes snapshot checkpointing via the persistence `write_snapshot` port.

## Units

- `core/src/acceptance/index.rs` — `CommandIndex`, `apply()`, `get_command()`
- `core/src/acceptance/replay.rs` — `rebuild_from_log()`, snapshot checkpoint integration

## Key properties

- **IdempotentLogReplay** (stated-normative, end-to-end): same events → same index. The storage layer proved determinism for unchanged contents; this completes it with a deterministic domain `apply`.
- **Corruption detection** (Fail Fast): a `COMMAND_TRANSITION` for an unknown command, or a `from_state` mismatch, is `CorruptLog`.

## Acceptance criteria

- [ ] Replaying `OPERATION` + `COMMAND_TRANSITION` events reconstructs the full command index.
- [ ] Replay is deterministic: same events → same index.
- [ ] Replay rejects a `COMMAND_TRANSITION` for an unknown command (CorruptLog).
- [ ] Replay rejects a transition whose `from_state` mismatches (CorruptLog).
- [ ] The index lookup (`get_command`) is O(1).
- [ ] Snapshot checkpointing bounds replay to the tail (integrates with persistence `write_snapshot`/`load_latest_snapshot`).

## Design reference

See `feature-v0-core-acceptance.md` § "Implementation Units" → "Unit 4".
