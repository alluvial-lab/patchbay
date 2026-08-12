---
id: snapshot-core-generation-semantics-durable-epoch
kind: story
stage: done
tags: [protocol, foundation]
parent: snapshot-core-generation-semantics
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Persist the authority-domain continuity epoch

## Checkpoint

Establish the durable identity that all later snapshot compatibility checks consume. `core_generation` is a nonzero opaque storage-continuity epoch keyed by authority domain and stable across ordinary process restarts; it is not incremented per process start.

## Design element

- Add the narrow `CoreGenerationStore` port in `core/src/storage/port.rs` and export it from `core/src/storage/mod.rs`:

```rust
pub trait CoreGenerationStore: Send + Sync {
    fn load_or_create_core_generation(
        &self,
        authority_domain_id: &AuthorityDomainId,
        candidate: Generation,
    ) -> impl std::future::Future<Output = Result<Generation, StorageError>> + Send;
}
```

- Add `StorageError::InvalidCoreGeneration(u64)` for zero/out-of-SQLite-range candidates.
- In `core/src/storage/rusqlite.rs`, advance the schema to v4 with `authority_domain_metadata(authority_domain_id TEXT PRIMARY KEY, core_generation INTEGER NOT NULL CHECK(core_generation > 0))`. Route initialization through the single writer actor and atomically insert-if-absent/read-back; concurrent candidates return the stored winner and consume no event LSN.
- Delegate the port through `core/src/storage/audited.rs`; metadata initialization is not an audit/log state transition.
- Add `server/src/identity.rs::random_core_generation() -> Generation` using existing `OsRng`, restricted to `1..=i64::MAX`.
- Make `ProjectionState` retain the value loaded during `rebuild*`, expose `core_generation(&self)`, and propagate the `CoreGenerationStore` bound through control/admin constructors. Do not add it to adapter-only consumers.

## Acceptance evidence

- [ ] First initialization stores the supplied candidate; repeat, concurrent, and same-file reopen calls return the unchanged stored value.
- [ ] Separate authority domains have separate rows; zero and values above `i64::MAX` fail before mutation.
- [ ] v3→v4 migration preserves events, snapshots, idempotency, and audit rows; malformed/future schemas still fail before mutation.
- [ ] Metadata initialization creates no durable event and consumes no LSN.
- [ ] `ProjectionState` and the production audited store carry one value for their lifetime; adapter-only test ports need no generation implementation.

## Ordering

No sibling prerequisite. This checkpoint blocks `snapshot-core-generation-semantics-snapshot-compatibility`, which must never populate or validate a process-local, unstored candidate.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the contract-bearing durable recovery epoch).
- Review weight: `thorough` (explicit operator selection); child checkpoint advanced directly to done after verification per delegated endpoint instructions.
- Files changed: `core/src/storage/{port,mod,rusqlite,audited}.rs`, `server/src/{identity,state,service,admin_service}.rs`, `core/tests/{rusqlite_storage,audit_records}.rs`, `server/tests/grpc_smoke.rs`.
- Tests added: insert-once/domain isolation/no-LSN, independent-writer concurrency, invalid-bound, file-reopen, v3→v4 preservation, and malformed-v4-metadata tests protect the new durability contract and migration boundary.
- Simplification: kept generation metadata out of the event/audit log and delegated one narrow port through the production decorator; no restart counter or compatibility window was added.
- Discrepancies from design: storage/migration tests scheduled in Unit 3 were landed with this checkpoint so its persistence and migration acceptance evidence was green before the dependent snapshot work began; no semantic discrepancy.
- Adjacent issues parked: none.
- Verification: `cargo check --workspace`; `cargo test -p patchbay-core --test rusqlite_storage`; `cargo test -p patchbay-core --test audit_records`; `cargo test -p patchbay-core-server state::tests --lib`.
