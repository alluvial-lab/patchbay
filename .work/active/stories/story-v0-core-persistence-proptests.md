---
id: story-v0-core-persistence-proptests
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-persistence
depends_on: [story-v0-core-persistence-recovery]
created: 2026-07-11
updated: 2026-07-11
gate_origin: null
release_binding: null
---

# Story: Property tests for storage invariants

## Scope

Write proptest-based property tests validating the storage invariants: gap-free LSN, idempotent replay, crash recovery loses no committed event, snapshot prefix consistency, stale/wrong-domain snapshot rejection.

## Units

- `core/tests/storage_proptest.rs` — proptest suite

## Key properties

- **Gap-free LSN** — committed LSNs are contiguous (1, 2, 3, ...). This is the empirical validation of the bare-`INTEGER PRIMARY KEY`-as-LSN choice: it tests the guarantee rather than assuming it.
- **Idempotent replay** — replaying the same committed prefix produces identical state (`IdempotentLogReplay`, stated-normative).
- **Crash recovery** — all committed events survive a reopen (`CrashNoAcceptedLost`, stated-normative).
- **Snapshot prefix consistency** — a snapshot at LSN N + replay of N+1..M equals replay from 0 (`SnapshotConsistentPrefix`, stated-normative).
- **Stale snapshot rejection** — a snapshot with LSN < current state is rejected (Fail Fast).
- **Wrong-domain snapshot rejection** — a snapshot from a different authority domain is rejected (Fail Fast).

## Acceptance criteria

- [ ] `committed_lsns_are_gap_free_and_monotonic` passes for 100 generated event sequences.
- [ ] `replay_is_idempotent` passes.
- [ ] `crash_recovery_loses_no_committed_event` passes.
- [ ] `snapshot_prefix_consistent` passes.
- [ ] Stale and wrong-domain snapshot rejections pass.
- [ ] All proptests shrink to minimal failures on mutation (verified by injecting a deliberate bug — e.g., a `+1` in the LSN assignment — and confirming the test catches it with a minimal counterexample).

## Design reference

See `feature-v0-core-persistence.md` § "Implementation Units" → "Unit 4" for the proptest shapes.
