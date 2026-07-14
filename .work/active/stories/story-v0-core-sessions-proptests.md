---
id: story-v0-core-sessions-proptests
kind: story
stage: review
tags: [protocol, verification, foundation]
parent: feature-v0-core-sessions
depends_on: [story-v0-core-sessions-state-machine, story-v0-core-sessions-registry, story-v0-core-sessions-ingest, story-v0-core-sessions-replay-resolver]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Story: Property tests for session invariants

## Scope

Implement Unit 5 of `feature-v0-core-sessions`: property tests for the promoted `GenerationMonotonic` property and the stated-normative obligations. Mirrors `acceptance_proptest.rs` and `storage_proptest.rs`.

## Units

- `core/tests/sessions_proptest.rs` — proptest strategies, property oracles, mutation tests

## Implementation

See `feature-v0-core-sessions.md` Unit 5 for the property list. Key points:

- Proptest strategies: `any_session_report()`, `any_generation()`, `any_connectivity_state()`, `any_activity_state()`, `any_session_report_sequence()` (a sequence of reports against one or more sessions, including generation bumps, state-axis changes, and stale-generation reports).
- Properties:
  - `generation_never_decreases` — **promoted** (`GenerationMonotonic`); the live session generation never decreases across any sequence of session reports.
  - `equal_generation_is_noop_lower_is_rejected` — stated-normative (strict-supersession action guard).
  - `late_generation_is_inert` — stated-normative (`LateGenerationInert`); reports binding to a tombstoned generation do not mutate the live generation.
  - `relabel_preserves_identity` — stated-normative (labels cannot override identity).
  - `tombstones_retained` — tombstoned generations are retained and queryable after subsequent bumps.
  - `replay_matches_live` — replay determinism.
- Mutation tests (non-vacuity): run the same properties against a buggy registry that allows generation decrease, and assert the property FAILS. Mirrors `acceptance_proptest.rs` mutation adapters (`DoubleApplyStorage`, etc.).
- Test against `RusqliteStorage::open_in_memory()` for the full write→replay round-trip, and against `NoopStorage`/fault-injecting wrappers for targeted property checks.

## Acceptance Criteria

- [ ] `generation_never_decreases` passes against the real registry
- [ ] `generation_never_decreases` FAILS against a mutation that allows decrease (non-vacuous)
- [ ] `equal_generation_is_noop_lower_is_rejected` passes
- [ ] `late_generation_is_inert` passes
- [ ] `relabel_preserves_identity` passes
- [ ] `tombstones_retained` passes
- [ ] `replay_matches_live` passes (replay determinism)

## Notes

- Depends on all four prior stories (needs the full implementation).
- The mutation tests are essential for non-vacuity — the acceptance proptests established this discipline. A property that cannot fail against a buggy implementation is worthless.
- `GenerationMonotonic` is the only PROMOTED property; the others are stated-normative obligations tested as properties but not backed by checked formulas.

## Implementation notes

- Added `core/tests/sessions_proptest.rs` with bounded report/state strategies and durable writer → hot-registry → replay property oracles for generation monotonicity, strict supersession, late-generation inertness, relabel identity, tombstone retention, and replay equivalence.
- Added a `DecreasingGenerationRegistry` mutation whose lower-generation overwrite is rejected by the shared monotonic-generation oracle, proving the promoted property is non-vacuous.
- Verified with `CARGO_HOME=/tmp/cargo-home cargo build -p patchbay-core`, `cargo test -p patchbay-core --test sessions_proptest`, and the complete `cargo test -p patchbay-core` suite.
