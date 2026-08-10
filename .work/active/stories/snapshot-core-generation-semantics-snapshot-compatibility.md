---
id: snapshot-core-generation-semantics-snapshot-compatibility
kind: story
stage: done
tags: [protocol]
parent: snapshot-core-generation-semantics
depends_on: [snapshot-core-generation-semantics-durable-epoch]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Carry and validate the snapshot continuity anchor

## Checkpoint

Use the persisted epoch in every materialized snapshot and enforce one exact compatibility rule for the current durable session-checkpoint slot. A prior process's checkpoint is accepted after restart only because the new process loads the same authority-domain epoch.

## Design element

- Add `server/src/snapshot.rs` and export the module from `server/src/lib.rs`:

```rust
pub fn decode_compatible_session_checkpoint(
    stored: &StoredSnapshot,
    expected_domain: &AuthorityDomainId,
    expected_core_generation: &Generation,
) -> Result<SessionSnapshot, SessionCheckpointRejection>;
```

- The helper fails closed on decode failure; missing/wrong stored or embedded domain; missing/zero/wrong embedded generation; missing stored/embedded LSN; or embedded LSN unequal to the storage row. It validates identity only—freshness remains the caller's LSN decision.
- In `server/src/state.rs`, set `core_generation: Some(self.core_generation.clone())` for both session and resource materialization.
- In `server/src/service.rs`, validate every stored session checkpoint with the helper before applying the existing freshness rule. An incompatible or stale derived checkpoint is a cache miss: return a freshly materialized current session snapshot if log reconstruction is healthy. Resource reads remain on-demand and never decode the session slot.
- Retain the current session-only durable namespace. Do not add a generic checkpoint framework, another wire field, a process counter, or a predecessor-generation exception.

## Acceptance evidence

- [ ] Session and resource snapshots from one `ProjectionState` carry the same persisted nonzero epoch and exact authority domain/current LSN.
- [ ] A current session checkpoint survives drop/reopen/rebuild of the same database and can be returned after restart.
- [ ] Wrong/missing/zero generation, wrong/missing domain, corrupt bytes, and embedded/stored LSN disagreement cannot become authoritative and repair to current materialization.
- [ ] Generation equality does not make an older checkpoint fresh.
- [ ] Resource reads remain discriminated from the session snapshot slot.

## Ordering

Depends on `snapshot-core-generation-semantics-durable-epoch`. Blocks `snapshot-core-generation-semantics-continuity-evidence`, which verifies the real restart and mismatch paths and rolls the foundation forward.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for exact recovery compatibility semantics).
- Review weight: `thorough` (explicit operator selection); child checkpoint advanced directly to done after verification per delegated endpoint instructions.
- Files changed: `server/src/{snapshot,lib,state,service}.rs`, `server/tests/grpc_smoke.rs`.
- Tests added: pure domain/generation/LSN/decode rejection tests, shared session/resource carriage test, and a file-backed restart RPC test covering compatible reuse, stale fallback, and missing/different-generation repair.
- Simplification: replaced inline ad hoc checkpoint identity checks with one reusable decoder; freshness remains one separate caller comparison and the current session-only namespace remains unchanged.
- Discrepancies from design: the file-backed restart/mismatch test scheduled in Unit 3 was pulled into this checkpoint so cross-incarnation compatibility was verified before advancing; `SessionCheckpointRejection` implements `Error` directly because the server crate does not depend on `thiserror`, avoiding an out-of-scope manifest change.
- Adjacent issues parked: none.
- Verification: `cargo check --workspace`; `cargo test -p patchbay-core-server snapshot::tests --lib`; `cargo test -p patchbay-core-server state::tests --lib`; `cargo test -p patchbay-core-server --test grpc_smoke core_generation`.
