---
id: story-v0-core-persistence-workspace-and-port
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-persistence
depends_on: []
created: 2026-07-11
updated: 2026-07-11
gate_origin: null
release_binding: null
---

# Story: Workspace scaffolding and storage port trait

## Scope

Bootstrap the Rust application workspace and define the `Storage` port trait that the rest of the core depends on. This is the foundation every other persistence unit builds on.

## Units

- `core/Cargo.toml` — `patchbay-core` crate depending on `patchbay-contracts`, tokio, rusqlite, prost, thiserror, tracing
- Root `Cargo.toml` — workspace with `contracts/rust` and `core/` as members
- `core/src/lib.rs` — crate root
- `core/src/storage/mod.rs` — storage module
- `core/src/storage/port.rs` — `Storage` trait, `RecordedEvent`, `StorageError`

## Acceptance criteria

- [ ] Root `Cargo.toml` workspace compiles with `contracts/rust` and `core/` as members.
- [ ] `patchbay-core` crate depends on `patchbay-contracts`; `cargo build` succeeds.
- [ ] `Storage` trait compiles against the generated `EventId`/`Lsn`/`AuthorityDomainId` types from `patchbay-contracts`.
- [ ] No hand-written code in `contracts/rust/` (Generated Contracts principle holds — that crate remains purely generated).
- [ ] `StorageError` distinguishes stale-snapshot and wrong-domain rejections from raw rusqlite failures (Fail Fast).

## Design reference

See `feature-v0-core-persistence.md` § "Implementation Units" → "Unit 1" for the exact trait signature, error type, and Cargo.toml shape.
