---
id: story-v0-core-persistence-proptests
kind: story
stage: review
tags: [protocol, verification, foundation]
parent: feature-v0-core-persistence
depends_on: [story-v0-core-persistence-recovery]
created: 2026-07-11
updated: 2026-07-12
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

- [x] `committed_lsns_are_gap_free_and_monotonic` passes for 100 generated event sequences.
- [x] `replay_is_idempotent` passes. (`replay_deterministic_for_unchanged_contents` — renamed to reflect the honest claim: deterministic for unchanged contents, not unconditionally idempotent, per the recovery story's review.)
- [x] `crash_recovery_loses_no_committed_event` passes.
- [x] `snapshot_prefix_consistent` passes. (`snapshot_plus_tail_equals_full_replay`.)
- [x] Stale and wrong-domain snapshot rejections pass. (`write_snapshot_rejects_invalid_lsn`, `snapshot_isolated_per_authority_domain`.)
- [x] All proptests shrink to minimal failures on mutation (verified by injecting a deliberate bug — `gap_free_catches_injected_lsn_bug` wraps the store with a `+1` LSN fault; `dedup_catches_injected_double_apply` wraps it with an always-append dedup fault; both assert the property *fails* on the buggy store, proving non-vacuity).

## Design reference

See `feature-v0-core-persistence.md` § "Implementation Units" → "Unit 4" for the proptest shapes.

## Implementation notes

- **Suite:** `core/tests/storage_proptest.rs` — 16 tests: 14 proptests (100 cases each) + 2 mutation-discipline integration tests.
- **Properties covered:**
  - Gap-free LSN (single domain + cross-domain monotonicity)
  - Deterministic replay for unchanged contents (no snapshot + with snapshot)
  - Crash recovery loses no committed event
  - Snapshot prefix consistency (snapshot + tail == full replay)
  - Snapshot at log head → empty tail
  - Fail Fast: invalid snapshot LSN rejected; cross-domain snapshot isolation
  - `BoundaryDedup`: retry returns existing (no double-apply); different targets don't dedup; differing payload conflicts; dedup appends remain gap-free
- **Mutation discipline (the acceptance criterion that makes the suite honest):** Two `#[test]`s wrap `RusqliteStorage` in a thin adapter that injects the named fault (`OffByOneLsnStorage` adds `+1` to appended LSNs; `DoubleApplyStorage` ignores the dedup key and always appends) and assert the property *fails* on the buggy store. This is the genuine proof that the proptests are not vacuous — a suite that passes on a buggy implementation is not testing the property. (The story named `gap_free_catches_injected_lsn_bug` and a double-apply catch; both land as real tests.)
- **Honest naming:** `replay_is_idempotent` was renamed to `replay_deterministic_for_unchanged_contents`. Two `recover()` calls can differ if writes happen between them; the honest storage-layer claim is determinism for unchanged contents, not unconditional idempotency. This aligns with the recovery story's review catch (the `IdempotentLogReplay` obligation is stated-normative and depends on the domain layer's deterministic `apply`, which isn't built yet).
- **`per_domain_lsns_are_contiguous_under_cross_domain_writes`:** the global rowid is shared across domains, so per-domain LSNs are NOT contiguous-from-1 (they interleave). The honest property is strict monotonicity within a domain + correct count, asserted here. This documents that the `(authority_domain_id, LSN)` tuple — not the bare LSN — is the canonical `EventId`, matching the federation-seam design.
- **Mechanical notes:** Each proptest case spins up its own `open_in_memory()` store (isolation). Tokio runtime is constructed per test (`Runtime::new`) since `#[tokio::test]` and `proptest!` don't compose directly. `indexed_payload(i)` encodes the index across two bytes to survive `u8` overflow in larger sequences. `prop_assume!` guards the different-targets/differing-payload tests so proptest skips degenerate inputs rather than failing on them.
- **Build env:** `CARGO_HOME=/tmp/cargo-home` required (read-only `~/.cargo` cache in the sandbox), per the session note. Clippy clean, `rustfmt` clean.
- **Test count:** 50 total across `patchbay-core` (16 proptests + 18 rusqlite + 9 recovery + 7 port smoke), all green.
