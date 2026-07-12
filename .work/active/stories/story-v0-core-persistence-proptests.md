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
- **Crash recovery** — all committed events survive a reopen (`CrashNoAcceptedLost`, stated-normative). Note: "reopen" is not a process-level crash; it does not prove `synchronous=FULL` durability against power loss. It proves committed events are visible after the storage handle is dropped and a new one opens the same DB file.
- **Snapshot bounds replay** — a snapshot at LSN N bounds replay to events > N, and the tail events are byte-identical to the corresponding events in a full replay. This is the storage-layer portion of `SnapshotConsistentPrefix`. The snapshot *payload content* (that it reflects events 1..=N) is a caller obligation per `port.rs`; the storage layer treats the payload as opaque bytes.
- **Fail Fast** — invalid-LSN snapshot rejected; cross-domain snapshot isolation.
- **Reserved error variants** — `SnapshotStale` and `SnapshotWrongDomain` are reserved for future snapshot-reconciliation operations; the current API shape cannot trigger them. They are the non-foreclosure seam and get tests when the operations that can trigger them are implemented.

## Acceptance criteria

- [x] `committed_lsns_are_gap_free_and_monotonic` passes for 100 generated event sequences. (Also reads back full events to verify payload preservation, not just LSNs.)
- [x] `replay_is_idempotent` passes. (`replay_deterministic_for_unchanged_contents` — renamed: the honest storage-layer claim is determinism for unchanged contents, not unconditional idempotency. `IdempotentLogReplay` end-to-end depends on the domain layer's deterministic `apply`, not built yet.)
- [x] `crash_recovery_loses_no_committed_event` passes. (`crash_recovery_preserves_full_events` — compares full events including payloads, not just LSNs. A payload-corruption mutant is caught by a dedicated mutation test.)
- [x] `snapshot_prefix_consistent` passes. (`snapshot_bounds_tail` — renamed to the honest claim: the snapshot bounds replay to the correct tail, and the tail matches the corresponding suffix of a full replay. It does NOT prove the snapshot payload reflects events 1..=N, which is a caller obligation on opaque bytes the storage layer cannot verify.)
- [x] Stale and wrong-domain snapshot rejections pass. **REVISED (review finding):** these cannot be tested against the current API — `write_snapshot` takes the domain as a parameter (so wrong-domain submission is impossible) and accepts any valid LSN (so staleness is a reconciliation-time concern, not a write-time rejection). Tested instead: `write_snapshot_rejects_invalid_lsn` (Fail Fast on a nonexistent LSN) and `snapshot_isolated_per_authority_domain` (a snapshot for domain A does not surface for domain B). `SnapshotStale`/`SnapshotWrongDomain` are reserved error variants — the non-foreclosure seam — and get tests when the operations that can trigger them are implemented.
- [x] All proptests catch injected bugs (mutation discipline). Three mutation tests prove non-vacuity: `gap_free_catches_injected_lsn_bug` (+1 LSN fault), `crash_recovery_catches_payload_corruption` (constant-payload fault, caught by full-event comparison), `dedup_catches_injected_double_apply` (always-append fault). Each asserts the property FAILS on the buggy store. Proptest shrinking is automatic when a generated case fails; these tests prove the properties catch the named bugs, which is the precondition for shrinking to matter.

## Design reference

See `feature-v0-core-persistence.md` § "Implementation Units" → "Unit 4" for the proptest shapes.

## Implementation notes

- **Suite:** `core/tests/storage_proptest.rs` — 18 tests: 15 proptests (100 cases each) + 3 mutation-discipline integration tests.
- **Properties covered:**
  - Gap-free LSN (single domain, full-event readback) + cross-domain monotonicity with tuple-identity check
  - Deterministic replay for unchanged contents (no snapshot + with snapshot)
  - Crash recovery preserves full events (payload comparison, not just LSNs)
  - Snapshot bounds tail to correct suffix (renamed from prefix-consistency)
  - Snapshot at log head → empty tail
  - Fail Fast: invalid snapshot LSN rejected; cross-domain snapshot isolation
  - `BoundaryDedup`: retry returns existing (no double-apply); different targets don't dedup; differing payload conflicts AND persists nothing; dedup appends remain gap-free (with readback count oracle); concurrent same-key submissions produce exactly one append; dedup keys survive restart
- **Deep review (round 1, two-phase):** Phase 1 (completeness) and Phase 2 (adversarial) both flagged the same core issues. Fixes applied:
  1. **Full-event comparisons, not LSN-only.** `crash_recovery` and `committed_lsns` now read back and compare full `RecordedEvent` payloads. Added `crash_recovery_catches_payload_corruption` mutation test (constant-payload fault) to prove the comparison is non-vacuous.
  2. **Snapshot property renamed and scoped honestly.** `snapshot_plus_tail_equals_full_replay` → `snapshot_bounds_tail`. The old test wrote an arbitrary marker `[0xFF]` and compared only tail suffixes — proving cursor selection, not prefix consistency. The storage layer cannot prove prefix consistency (the snapshot payload is opaque bytes; the caller materializes it per `port.rs`). The renamed test asserts what the storage layer CAN prove: the snapshot bounds replay to the correct tail, and the tail matches the full replay's suffix.
  3. **Stale/wrong-domain criterion revised.** The current `write_snapshot`/`load_latest_snapshot` API cannot trigger `SnapshotStale` or `SnapshotWrongDomain` — the domain is a call parameter, and staleness is reconciliation-time. Marked the criterion honestly as tested-via-isolation, with the error variants documented as the reserved seam.
  4. **Concurrent dedup test added.** `append_dedup_concurrent_same_key_no_double_apply` launches 2-8 tasks with the same key simultaneously and asserts exactly one `Appended`. This is the concurrent-acceptance-handler case the port's `appliedKeys` claim is grounded in — sequential tests cannot catch a check-then-yield-then-append race.
  5. **Cross-restart dedup test added.** `append_dedup_survives_restart` verifies idempotency keys are durable — a backend keeping keys only in memory would fail.
  6. **`dedup_appends_remain_gap_free` readback oracle added.** The old test checked only `lsns.windows(2)`, which an always-Duplicate mutant (empty vector) would pass. Now also asserts the log count equals the appended count.
  7. **Conflict test strengthened.** `append_dedup_conflict_on_differing_payload` → `append_dedup_conflict_rejects_and_persists_nothing`: now asserts no conflicting event was persisted and the surviving event is the original.
  8. **Cross-domain test renamed and tuple-identity added.** `per_domain_lsns_are_contiguous...` → `per_domain_lsns_monotonic...` (honest name: monotonic, not contiguous, under the global-rowid design). Now also verifies every read-back event carries the correct `authority_domain_id`, not just a bare LSN.
- **Honest naming:** `replay_is_idempotent` → `replay_deterministic_for_unchanged_contents`. Two `recover()` calls can differ if writes happen between them; the storage-layer claim is determinism for unchanged contents.
- **`per_domain_lsns_monotonic_under_cross_domain_writes`:** the global rowid is shared across domains, so per-domain LSNs interleave (domain A gets [1,3], domain B gets [2]). The honest property is strict monotonicity + correct count + tuple identity. v0.1.0 has one domain so per-domain contiguity holds trivially in production; multi-domain gap-free is a reserved seam.
- **Mechanical notes:** Each proptest case spins up its own `open_in_memory()` store (isolation). Tokio runtime is constructed per test (`Runtime::new`) since `#[tokio::test]` and `proptest!` don't compose directly. `indexed_payload(i)` encodes the index across two bytes to survive `u8` overflow. `prop_assume!` guards degenerate inputs. Concurrent test uses `Arc` + `tokio::spawn`.
- **Build env:** `CARGO_HOME=/tmp/cargo-home` required (read-only `~/.cargo` cache in the sandbox). Clippy clean, `rustfmt` clean.
- **Test count:** 52 total across `patchbay-core` (18 proptests + 18 rusqlite + 9 recovery + 7 port smoke), all green.
