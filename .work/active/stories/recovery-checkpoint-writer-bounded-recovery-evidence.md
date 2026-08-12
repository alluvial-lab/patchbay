---
id: recovery-checkpoint-writer-bounded-recovery-evidence
kind: story
stage: done
tags: [verification, protocol, foundation]
parent: recovery-checkpoint-writer
depends_on: [recovery-checkpoint-writer-scheduling-runtime]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-10
updated: 2026-08-11
---

# Prove the narrow recovery bound honestly

## Checkpoint

Prove real restart equivalence, crash-safe failure/retry, and both production session consumers; then roll foundation/model wording forward without turning the session-only healthy-policy target into a whole-core or checked-normative claim.

## Design element

- Build a file-backed fixture with generation replacement, lockdown, source-cursor changes, and sibling command/grant/Elicitation/resource/security/operator events. Write a checkpoint with a small test policy, append a short tail, reopen, and compare checkpoint+tail with full session replay.
- Exercise aggregate `ProjectionState` and `AdapterControlServiceImpl`; assert the session recovery helper reports only tail applications while every excluded sibling projection reconstructs facts from before the checkpoint.
- Inject a failed snapshot write and prove the authoritative log/prior row survive, failure observation fires, and the next pass advances.
- Update the existing promoted snapshot-reconciliation runner only for format 2; retain its `SnapshotStaleRejected` property id and assurance classification.
- Update `PROTOCOL`, `ARCHITECTURE`, `VERIFICATION`, and `GLOSSARY` in place: periodic writer implemented, one latest derived row, conditional session-fold target, full sibling replay, failure observability/retry, and reserved composite/per-projection namespaces.
- Relabel the draft command-oriented `snapshot_recovery.qnt` comments as a future abstract whole-core model; compile/check metadata without promoting a property.

## Acceptance evidence

- [x] Both production session registries restart to full-replay-equivalent state by applying only the post-anchor tail.
- [x] Pre-checkpoint sibling facts prove whole-core recovery remains full-log and no documentation claims otherwise.
- [x] Failure/crash evidence proves checkpointing changes cost only, never log order, accepted-state durability, authority, or serving availability.
- [x] Existing promoted vector and draft-model traceability remain green with unchanged assurance tiers.
- [x] Foundation docs use the committed/reserved/rejected vocabulary and state the scheduling qualification exactly.

## Implementation evidence

- `checkpoint::tests::file_restart_recovers_both_session_consumers_and_full_replays_siblings` writes a real file-backed checkpoint, appends a tail, reopens storage, compares both production consumers with full replay, and proves pre-anchor resource/security facts survive sibling full replay.
- Failure evidence preserves the prior checkpoint byte-for-byte and the complete log before retry advances the row. Semantic mutation and latest-only storage regressions are green.
- Foundation docs, `snapshot_recovery.qnt`, and `snapshot-reconciliation.json` retain the narrow session-only/draft assurance posture. Quint compilation, model/vector registries, 22 implementation checks, and 38 mutation kills pass.

## Review and ordering

Depends on `recovery-checkpoint-writer-scheduling-runtime`. This `[verification]` checkpoint follows the project's verification evidence/review policy when implemented; the integrated parent still requires explicit `thorough` feature review and receiver adjudication of findings.
