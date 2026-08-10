---
id: recovery-checkpoint-writer-session-recovery-state
kind: story
stage: implementing
tags: [protocol, storage]
parent: recovery-checkpoint-writer
depends_on: [snapshot-core-generation-semantics, replay-integrity-prefix-discipline, session-registry-replay-domain-soundness, adapter-report-source-ordering]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Complete session recovery checkpoint

## Checkpoint

Make the existing private typed/versioned session slot a complete, compact recovery seed for the authority-domain-bound `SessionRegistry`, then use one validator-aware checkpoint-plus-tail recovery path for every production session-registry rebuild.

## Design element

- Add generated `StoredSessionCheckpoint { snapshot, tombstones }` and `SessionCheckpointTombstone` messages. Bump the private outer envelope to format 2; reject format 1/raw/cross-type bytes without a dual reader.
- Add `SessionRegistry::from_checkpoint(...)`, `tombstones()`, and covered-prefix metadata. Restore live records, source cursors, retained generation tombstones, lockdown clamp, and revisions only after domain/identity/state/uniqueness/range validation.
- A checkpoint-seeded registry rejects every direct event at or below its covered prefix rather than accepting on LSN alone. It retains exact payload-redelivery semantics for post-checkpoint tail events.
- Add `recover_session_registry(storage, domain, generation) -> RecoveredSessionRegistry`; route `ProjectionState` and `AdapterControlServiceImpl` session rebuilds through it. The aggregate/server sibling projections still replay from LSN 0.
- Materialize the public session snapshot and private tombstone payload at one applied prefix. `LoadSnapshot` unwraps only the public `SessionSnapshot`.

## Acceptance evidence

- [ ] Format-2 checkpoint plus tail equals a fresh full-log session rebuild across live records, source cursors, tombstones, lockdown state, and revisions.
- [ ] Control and adapter production session rebuilds apply only post-anchor tail events when compatible state exists.
- [ ] Every framing/anchor/semantic mutation falls back to full replay without hiding a sibling projection's earlier events.
- [ ] Checkpoint size does not contain the full covered event-equality ledger, and covered-prefix direct re-feed fails closed.

## Ordering constraints

This is the first sibling checkpoint, but the parent feature remains blocked on `replay-integrity-prefix-discipline`, `session-registry-replay-domain-soundness`, and `adapter-report-source-ordering`. Re-read those landed interfaces before implementation. This story blocks `recovery-checkpoint-writer-scheduling-runtime`.
