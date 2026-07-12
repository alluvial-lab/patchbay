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
- [x] Mutation discipline: proptests are non-vacuous on the properties that carry the highest safety weight. Three mutation tests inject a deliberate bug and assert the property FAILS on the buggy store: `gap_free_catches_injected_lsn_bug` (+1 LSN fault), `crash_recovery_catches_payload_corruption` (constant-payload fault, caught by full-event comparison), `dedup_catches_injected_double_apply` (always-append fault). The remaining proptests are non-vacuous by construction (independent oracles: expected-count from a `HashSet`, exact `1..=N` equality, readback LSN comparison) rather than by dedicated mutation test. Proptest shrinking is automatic when a generated case fails; the mutation tests prove the properties catch the named bugs, which is the precondition for shrinking to matter.

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
  6. **`dedup_appends_remain_gap_free` rewritten.** Round 1 added a readback count oracle, but round-2 adversarial review found it still vacuous: the expected count was derived from the observed `Appended` outcomes, so an always-Duplicate mutant (writes nothing) produced `appended_count == 0` and `events.len() == 0` and passed. Also, the `any_payload()` generator could produce a conflicting reuse (same key+target, different payload) that legitimately returns `IdempotencyConflict` and panicked on `.unwrap()`. Fixed: the strategy now generates only `(key, target)` pairs with a fixed payload; the expected set is computed INDEPENDENTLY (first-seen (key,target) must append, repeats must duplicate) via a `HashSet`. An always-Duplicate mutant fails the first-seen-must-append check immediately.
  7. **Conflict test strengthened.** `append_dedup_conflict_on_differing_payload` → `append_dedup_conflict_rejects_and_persists_nothing`: now asserts no conflicting event was persisted and the surviving event is the original.
  8. **Cross-domain test renamed and tuple-identity added.** `per_domain_lsns_are_contiguous...` → `per_domain_lsns_monotonic...` (honest name: monotonic, not contiguous, under the global-rowid design). Now also verifies every read-back event carries the correct `authority_domain_id`, not just a bare LSN.
- **Honest naming:** `replay_is_idempotent` → `replay_deterministic_for_unchanged_contents`. Two `recover()` calls can differ if writes happen between them; the storage-layer claim is determinism for unchanged contents.
- **`per_domain_lsns_monotonic_under_cross_domain_writes`:** the global rowid is shared across domains, so per-domain LSNs interleave (domain A gets [1,3], domain B gets [2]). The honest property is strict monotonicity + correct count + tuple identity. v0.1.0 has one domain so per-domain contiguity holds trivially in production; multi-domain gap-free is a reserved seam.
- **Mechanical notes:** Each proptest case spins up its own `open_in_memory()` store (isolation). Tokio runtime is constructed per test (`Runtime::new`) since `#[tokio::test]` and `proptest!` don't compose directly. `indexed_payload(i)` encodes the index across two bytes to survive `u8` overflow. `prop_assume!` guards degenerate inputs. Concurrent test uses `Arc` + `tokio::spawn`.
- **Build env:** `CARGO_HOME=/tmp/cargo-home` required (read-only `~/.cargo` cache in the sandbox). Clippy clean, `rustfmt` clean.
- **Test count:** 52 total across `patchbay-core` (18 proptests + 18 rusqlite + 9 recovery + 7 port smoke), all green.

## Deep review (round 2, adversarial re-review)

Round 2 found the round-1 `dedup_appends_remain_gap_free` fix was still vacuous (count oracle derived from observed outcomes) plus two cheaper issues. Fixes applied:

- **`dedup_appends_remain_gap_free` rewritten with an independent oracle.** The expected appended set is now computed from a first-seen `HashSet<(key,target)>` — not from the observed outcomes — so an always-Duplicate mutant (writes nothing, returns Duplicate for all) fails the first-seen-must-append assertion. Fixed payload eliminates the `IdempotencyConflict` panic on generated conflicting reuse.
- **`append_dedup_retry` now verifies the full `EventId` tuple.** Previously checked only the bare LSN; a wrong-domain Duplicate with the right LSN would pass. Now asserts `id == first_id` (full tuple equality).
- **`n_concurrent` range corrected** to `2..=8` (was `2..8`, which generates 2–7).

Round 2 also flagged Important items that are genuine but do not block advancement — they document the honest limits of in-process testing, not test bugs:
- **"Reopen" is not a process-level crash.** The crash-recovery tests drop the handle and reopen the file; they prove committed-event visibility across handle reuse, not `synchronous=FULL` durability against power loss. This is already documented honestly in the story body and the test doc-comments. Process-level durability is a config assertion (`PRAGMA synchronous = FULL`), not a property a proptest can prove without a fault-injection harness that kills the process mid-transaction — out of scope for the storage-layer evidence floor.
- **`snapshot_bounds_tail` uses a shared-oracle comparison** (both DBs use the same append/read/decode). A systematic codec corruption could produce identical wrong bytes on both sides. The start-LSN, tail-length, and monotonicity assertions make the bounds property non-vacuous; a fully independent byte oracle would require a separate decoder, deferred.
- **Concurrent dedup test has no barrier synchronization.** The current single-writer actor makes it deterministic, but a future check-then-act mutant could execute sequentially and escape. The test establishes current observable behavior (exactly one append under concurrent same-key submission); a controlled-overlap race harness is a v0.x hardening, not a v0.1.0 blocker.
- **`write_snapshot_rejects_invalid_lsn` tests only above-head LSNs** (not LSN 0 or in-range cross-domain). LSN 0 is a valid cursor ("from the beginning"), not an invalid snapshot anchor; in-range cross-domain rejection is covered by `snapshot_isolated_per_authority_domain`. The above-head case is the genuine Fail Fast path.
- **`replay_deterministic_with_snapshot` is WEAK** (compares two identical raw reads without asserting snapshot/tail correctness). It is a complement to `replay_deterministic_for_unchanged_contents` (which does assert tail correctness); the snapshot variant exists to cover the snapshot path's determinism, and `snapshot_bounds_tail` covers the snapshot path's correctness. Not a blocker.

The round-2 verdict was "Not converged" solely due to the `dedup_appends_remain_gap_free` vacuity, which is now fixed. The Important items are documented limits, not test defects. The suite is at the honest evidence-floor scope the story claims.

## Deep review (round 3, adversarial re-review)

Round 3 confirmed the round-2 fixes (always-Duplicate mutant now caught; full-EventId equality is real) but found two residual blockers:

- **`dedup_appends_remain_gap_free` did not anchor the sequence at LSN 1.** The `windows(2)` check only verified adjacent differences; a mutant starting at `[2,3,4,...]` would pass. Fixed: now asserts exact equality `expected_lsns == 1..=count`, catching an initial gap.
- **`write_snapshot_rejects_invalid_lsn` covered only above-head LSNs.** Round 3 flagged LSN 0 (never a committed event — rowid starts at 1) and cross-domain LSNs (exists in domain A, not B) as uncovered boundary cases. Fixed: the test now covers all three invalid-anchor cases (above-head, LSN 0, cross-domain) plus a valid-LSN sanity assertion.

Round 3 also flagged a nit on the acceptance-criterion wording ("All proptests catch injected bugs" overclaimed, since only 3 have dedicated mutation tests). Fixed: the criterion now honestly distinguishes the 3 mutation-tested properties from the properties non-vacuous by construction (independent oracles).
