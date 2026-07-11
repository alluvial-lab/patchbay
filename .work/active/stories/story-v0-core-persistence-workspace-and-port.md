---
id: story-v0-core-persistence-workspace-and-port
kind: story
stage: review
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

## Implementation notes

- **Files created**: `Cargo.toml` (root workspace), `core/Cargo.toml`, `core/src/lib.rs`, `core/src/storage/mod.rs`, `core/src/storage/port.rs`, `core/tests/storage_port_smoke.rs`.
- **Rust edition**: 2021. Rust 1.94.0. Native async traits (stabilized 1.75) — no `async-trait` dependency needed; the `Storage` trait uses `impl Future` return-position syntax.
- **rusqlite version pinned to 0.31** (not 0.40 as the research cited). The latest `libsqlite3-sys` 0.38.1 (pulled by rusqlite 0.40) uses the unstable `cfg_select!` feature and fails to compile on stable Rust 1.94. Pinned to rusqlite 0.31 → libsqlite3-sys 0.28, which is stable-compatible. This is a mechanical version constraint, not a semantic change — the SQLite WAL/synchronous semantics are identical. Flag for revisit if the toolchain upgrades or rusqlite 0.40 stabilizes its build.
- **CARGO_HOME workaround**: the sandbox has a read-only `~/.cargo` registry cache. Builds require `CARGO_HOME=/tmp/cargo-home` (or equivalent writable location). This is an environment quirk, not a code issue — CI/local builds with a writable cargo home are unaffected. Noted here so the next implementer doesn't hit the same wall.
- **`event_id()` helper** added to `port.rs` and re-exported from `storage::` — constructs the canonical `(authority_domain_id, LSN)` tuple without `Option`-wrapping boilerplate. Used by the rusqlite impl (next story) and tests.
- **`ReadFailed` wrapper**: `StorageError` has two `#[from] rusqlite::Error` sources (write vs read), which requires a newtype wrapper to satisfy `#[from]` uniqueness. `ReadFailed` is that wrapper. Slightly more ceremony than a single `Backend(rusqlite::Error)` variant, but the distinction aids diagnostics (write failures vs read failures surface differently).
- **Smoke test** (`core/tests/storage_port_smoke.rs`): 4 tests verifying the trait is implementable, `event_id()` builds the canonical tuple, `RecordedEvent`/`StorageError` construct as designed. Full proptest suite lands in `story-v0-core-persistence-proptests`.
- **Discrepancies from design**: none. The trait signature, error variants, and Cargo.toml shape match the design spec exactly. The only deviation is the rusqlite version pin (0.31 vs 0.40), documented above.
- **Verification**: `cargo build --workspace` succeeds; `cargo test --package patchbay-core` passes 4/4 tests.
