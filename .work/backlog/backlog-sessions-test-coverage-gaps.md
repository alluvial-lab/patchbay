---
id: backlog-sessions-test-coverage-gaps
kind: feature
stage: backlog
tags: [protocol, testing, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Backlog: Sessions feature test coverage gaps

## Source
Found during deep review of `feature-v0-core-sessions` (Phase 1 completeness, cross-model openai-codex/gpt-5.6-sol). Multiple non-blocking coverage gaps.

## Findings

1. **Replay corruption cases untested.** `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`, but `sessions_replay_resolver.rs` tests only successful replay + resolver cases. The header claims corruption rejection that it does not exercise. Add a fake `Storage` returning out-of-order and cross-domain `RecordedEvent`s, assert `CorruptLog`.

2. **Acceptance↔Sessions integration test missing.** `acceptance_pipeline.rs` still uses `TestTargetResolver`; no test submits through `acceptance::submit` using a real `SessionRegistry`. The feature's primary integration seam (sessions implements the port acceptance calls) is compile-checked but not exercised end-to-end. Add an integration test: register/replay a session, submit an operation through `acceptance::submit`, verify live success + tombstoned/unknown rejection through the real resolver.

3. **Malformed-event replay tests missing.** `registry.rs` has many fail-fast corruption branches (missing/empty domains, domain mismatch, missing mutation, missing identity fields, missing initial state, unknown enum values, non-increasing bumps, unknown bump sources, mismatched from-states). Tests cover happy paths + disallowed adjacency only. Add table-driven malformed-event tests covering each validation family, asserting `CorruptRecord` vs `CorruptLog` classification.

4. **Resolver boundary validation under-tested.** Tests cover Q3 behavioral cases but not missing/empty identity fields or incompatible `TargetScopeKind`. Acceptance rejects only unknown/unspecified kinds, so an `Actor` or other non-runtime scope carrying session fields could reach and resolve. Require `TargetScopeKind::RuntimeSession`, validate non-empty fields, add malformed-scope tests.

5. **Proptest identity isolation.** `any_session_report_sequence` fixes every report to one adapter/scope/runtime-session despite the design calling for sequences across "one or more sessions." Properties don't test cross-identity isolation. Generate multiple adapters/scopes/runtime-IDs; assert per-identity monotonicity, tombstone retention, absence of cross-session interference. (Partially overlaps with the tombstone-key blocker B4 — once B4 is fixed, the proptest should cover it.)

## Priority
Not blocking for correctness (the code paths exist and work), but the feature's fail-fast guarantees are unverified. Address alongside or after the blocker fixes. The integration test (#2) is the highest-value item — it exercises the seam that justifies the feature.
