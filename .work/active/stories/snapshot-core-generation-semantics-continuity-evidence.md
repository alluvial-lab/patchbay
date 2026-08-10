---
id: snapshot-core-generation-semantics-continuity-evidence
kind: story
stage: implementing
tags: [verification, protocol, foundation]
parent: snapshot-core-generation-semantics
depends_on: [snapshot-core-generation-semantics-snapshot-compatibility]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Verify continuity semantics and roll the foundation forward

## Checkpoint

Prove the persisted epoch survives a real process/storage reopen, prove mismatched derived state is rejected without losing log authority, align the existing formal-model vocabulary, and replace the contradictory foundation assertions in place.

## Design element

- Extend `core/tests/rusqlite_storage.rs` and `core/tests/audit_records.rs` for insert-once/concurrent candidates, per-domain isolation, invalid bounds, reopen persistence, and v3→v4 migration preservation.
- Extend `server/src/state.rs` tests and `server/tests/grpc_smoke.rs` with a file-backed checkpoint restart. Write a valid current `SessionSnapshot`, reopen/rebuild, and require the same epoch. Replace the payload at the same valid LSN with a missing/different generation and require `LoadSnapshot` to return repaired current materialization carrying the persisted epoch.
- Extend `contracts/vectors/snapshot-reconciliation.json` and `server/tests/conformance_vectors.rs` to seed a deterministic epoch through `CoreGenerationStore` and constrain/assert both `SessionSnapshot.core_generation` and `ResourceSnapshot.core_generation`. This strengthens field carriage in the existing promoted example; it does not promote `SnapshotCrossDomainRejected`.
- Align `specs/seed/snapshot_recovery.qnt` comments/initialization with one nonzero durable continuity epoch preserved by `crash`/`restart`; keep generation-mismatch rejection and existing draft promotion metadata honest.
- Roll `docs/PROTOCOL.md`, `docs/GLOSSARY.md`, `docs/VERIFICATION.md`, and `docs/ARCHITECTURE.md` forward in place: committed durable epoch, exact domain/epoch/LSN compatibility, replay fallback, and reserved restore/fork/HA rollover/process-fencing seams. Remove “assigned on every restart” and “currently unset/reserved” assertions without historical migration prose.

## Acceptance evidence

- [ ] Tests kill overwriting the persisted generation on restart and ignoring an embedded generation mismatch.
- [ ] The existing promoted snapshot-reconciliation runner reports both generation fields while retaining its current property classification.
- [ ] Quint parses/typechecks with ordinary restart preserving the epoch; model/vector traceability checks remain green and no new formal tier is claimed.
- [ ] Foundation docs agree on the durable-continuity meaning and do not claim checkpoint, clone, restore, HA, or process-fencing guarantees that are not implemented.
- [ ] Full workspace tests, clippy, formatting, model checks, and vector checks are green.

## Review and ordering

Depends on `snapshot-core-generation-semantics-snapshot-compatibility`. Because this story is tagged `[verification]`, it uses the project deep story-review lane and attacks the two load-bearing mutations before advancing to `done`. The integrated parent still requires the caller-selected `thorough` feature review.
