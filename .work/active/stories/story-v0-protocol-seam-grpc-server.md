---
id: story-v0-protocol-seam-grpc-server
kind: story
stage: done
tags: [protocol, adapter, security]
parent: feature-v0-protocol-seam
depends_on: [story-v0-protocol-seam-proto-services]
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: v0.1.0
research_origin: null
---

# Story: protocol-seam gRPC server crate

The driving adapter: a tonic gRPC server binary (`patchbay-core-server`) that wraps the `patchbay-core` library and exposes `ControlService` over gRPC/HTTP2. Owns the shared-secret auth, the `IssuerContext`-from-metadata translation, the projection-state concurrency wrapper, and the error mapping. Owns NO domain logic — pure translation.

## Design (from feature-v0-protocol-seam Units 2, 2b, 3)

**Files**: `server/Cargo.toml`, `server/src/main.rs`, `server/src/service.rs`, `server/src/issuer.rs`, `server/src/state.rs`, `server/tests/grpc_smoke.rs`

### Server crate (`server/`)

A new workspace member (`[workspace] members = ["contracts/rust", "core", "server"]`). Depends on `patchbay-core`, `patchbay-contracts`, `tonic` (server + transport features), `tonic-prost` (runtime codec), `tonic-types` (richer error model), `tonic-prost-build` (codegen), `tokio`, `tokio-stream`. Standalone `[workspace]` NOT needed — it's a real workspace member (unlike the throwaway spike).

### ControlService impl (`server/src/service.rs`)

Translates each RPC to the core's existing APIs:
- `Submit` → `acceptance::submit(storage, grant_check, target_resolver, state_lookup, issuer, operation)`.
- `Subscribe` → `Storage::read_after(domain, cursor)`, returned as a one-shot stream of `SubscribeEvent`.
- `LoadSnapshot` → `Storage::load_latest_snapshot(domain, at_or_before)`.

### IssuerContext from metadata (`server/src/issuer.rs`)

`MetadataIssuerContext` implements `patchbay_core::authority::IssuerContext` by reading:
- `verified_actor` — derived from `x-patchbay-operator-session-id` metadata (the web server vouches for this; it verified the operator's cookie). Per the design decision (a), the core trusts the web server's verification.
- `verified_endpoint` — the web-server's own endpoint id (its transport principal).
- `authority_domain_id` — from the Operation body.

### Projection-state concurrency (`server/src/state.rs`) — the trickiest part

The core's in-memory projections (`SessionRegistry`, authority registry, command-state lookup) are `&mut self` / single-threaded. The server wraps them in `Arc<Mutex<>>` (NOT by editing `patchbay-core`) so `acceptance::submit` is callable from concurrent tonic handlers. Lock ordering documented: `storage → grant_check → target_resolver → state_lookup` (matching `acceptance::submit`'s parameter order). Fallback if deadlock surfaces: promote to an actor inside the server crate (no core change).

### Auth interceptor

A tonic `Interceptor` checking `x-patchbay-core-secret` metadata against a configured value. Fail safe: server refuses to start without a configured secret (no open mode). Unauthenticated RPCs return `Status::unauthenticated`.

### Error mapping

`AcceptanceError` / `StorageError` → gRPC statuses using the richer error model (confirmed by the spike):
- `StorageError::Unavailable` → `Status::unavailable` (retryable hint as `RetryInfo`).
- `IdempotencyConflict` → `Status::failed_precondition`.
- `CorruptRecord` → `Status::internal`.

### Smoke test (`server/tests/grpc_smoke.rs`)

Integration test: spin up the server on an ephemeral port, exercise the three RPCs with a tonic gRPC client against the real `patchbay-core` storage + projections. Protects the contract the web server consumes.

## Acceptance criteria

- [x] `patchbay-core-server` binary starts, binds an h2c gRPC listener on a configurable address, refuses to start without a configured shared secret.
- [x] At startup it rebuilds in-memory projections by replaying the log via `read_after(domain, 0)`.
- [x] A `createGrpcTransport()`-equivalent tonic client can `Submit` an Operation and receive a `SubmissionResult`; an unauthorized secret returns `Status::unauthenticated`.
- [x] `Subscribe(cursor)` returns `> cursor` events and completes; `LoadSnapshot` returns the latest snapshot (None until one is written).
- [x] `StorageError` variants map to the gRPC statuses above.
- [x] Concurrency test: N parallel `Submit` calls complete without UB or deadlock.
- [x] Smoke test (Unit 3): submit → accepted; subscribe-from-0 → receives the event; load-snapshot → None initially.
- [x] Unauthenticated call returns `Status::unauthenticated`.
- [x] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --all --check` clean for the `server/` crate.

## Notes

- This story carries Units 2, 2b, and 3 as one cohesive ownership bundle — the server crate, concurrency wrapper, and smoke test are tightly coupled and share the same integration surface. Splitting them into separate stories would just manufacture handoff overhead.
- The TS client/translator is NOT in this story — that's `feature-v0-web-server`'s job. This story delivers only the Rust server half.
- Live-tailing `Subscribe` (blocking read_until_new) is a follow-on, not v0.1.0: the web server polls by re-subscribing with the highest LSN it received.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` at `high` effort; one feature-owning inline worker carried both dependency-ordered checkpoints, with no delegation per caller instruction.
- Review weight: `standard` (caller/default); not applicable to this child-story checkpoint, which advances directly to done on green verification.
- Files changed: root `Cargo.toml`/`Cargo.lock`; `server/Cargo.toml`, `server/build.rs`, `server/src/lib.rs`, `server/src/main.rs`, `server/src/service.rs`, `server/src/issuer.rs`, `server/src/state.rs`, `server/tests/grpc_smoke.rs`.
- Tests added/removed: `server/tests/grpc_smoke.rs` adds a real tonic interface test for auth, accepted Submit, cursor-bounded completing Subscribe, empty/latest LoadSnapshot, 16 parallel Submit calls with a deadlock timeout, rich storage-error mapping, and actual binary refusal without `PATCHBAY_CORE_SECRET`; no tests removed.
- Simplification: generated tonic service/client code reuses `patchbay-contracts` through `extern_path`, the server owns no domain state machine or authority rule, and one replay pass warms all three projections.
- Discrepancies from design: added `server/src/lib.rs` so the binary and integration test share one generated service/API; the core registries already satisfy the ports' `Send + Sync` bounds despite using mutable replay methods, but the required server-local `Arc<tokio::sync::Mutex<_>>` wrappers remain the concurrency boundary; Submit is serialized through projection catch-up so an immediate retry cannot observe a stale command index; `ActorId` is deterministically derived as the forwarded `OperatorSessionId` value; `Cargo.lock` necessarily changed for the new workspace member/dependencies even though the caller's write-scope list named only root `Cargo.toml`.
- Verification: `cargo build -p patchbay-core-server`, `cargo test -p patchbay-core-server`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all --check`, and `CARGO_HOME=/tmp/cargo-home cargo test -p patchbay-core` pass. The read-only `/tmp/cargo-home` emitted a cache-write warning after the workspace lock gained tonic packages, but core compilation/tests completed successfully.
- Adjacent issues parked: none.
