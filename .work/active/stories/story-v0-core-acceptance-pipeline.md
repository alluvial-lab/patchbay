---
id: story-v0-core-acceptance-pipeline
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: [story-v0-core-acceptance-state-machine]
created: 2026-07-12
updated: 2026-07-13
gate_origin: null
release_binding: v0.1.0
---

# Story: Acceptance pipeline and the three ports

## Scope

Implement the `submit` acceptance pipeline (validate → authorize → resolve-target → dedup → durable record) and the two Ports & Adapters seams it depends on: `GrantCheck` (authority) and `TargetResolver` (sessions). Both are async RPITIT traits, consistent with `Storage`. The pipeline writes through `Storage::append_dedup`.

## Units

- `core/src/acceptance/ports.rs` — `GrantCheck`, `TargetResolver` traits + `Authorized`/`GrantDenied`/`TargetBinding`/`TargetNotFound` types
- `core/src/acceptance/pipeline.rs` — `submit()` function
- `core/src/acceptance/mod.rs` — module root, `AcceptanceError`

## Key properties

- **BoundaryDedup** (promoted, shared with persistence): retry returns existing record, no double-apply.
- **Pre-acceptance failure**: `SubmissionOutcome = rejected` with no durable state.

## Acceptance criteria

- [ ] `submit` rejects unknown OperationKind with `validation_failed` (pre-grant).
- [ ] `submit` rejects unauthorized actor with `authorization_denied` (pre-acceptance, no durable state).
- [ ] `submit` rejects unknown target with `target_not_found` (pre-acceptance, no durable state).
- [ ] `submit` durably records the OPERATION event and returns `accepted` for a new command.
- [ ] `submit` returns the existing record for a retry (same command id + idempotency key + identical payload).
- [ ] `submit` rejects a differing-payload retry with `validation_failed` (IdempotencyConflict → validation_failed).
- [ ] Pre-acceptance failures create no durable command state.
- [ ] `GrantCheck` and `TargetResolver` are async RPITIT traits (consistent with `Storage`).

## Design reference

See `feature-v0-core-acceptance.md` § "Implementation Units" → "Unit 2".

## Implementation notes

- Files changed: `core/src/acceptance/ports.rs`, `core/src/acceptance/pipeline.rs`, `core/src/acceptance/mod.rs`, `core/src/acceptance/transitions.rs`, `core/tests/acceptance_pipeline.rs`.
- Tests added: seven integration tests covering fail-closed kind/field validation, pipeline ordering, no-trace authorization and target rejection, durable acceptance, identical retry deduplication, and differing-payload conflict rejection.
- Discrepancies from design: none. `AcceptanceError` gained transparent storage and invalid-target-scope variants to preserve normal rejection results while propagating infrastructure failures and malformed canonicalization.
- Dispatch rationale: direct-read only; the integration surface was bounded to the existing acceptance and storage modules plus generated contracts.
- Verification: `cargo build`, `cargo test`, `cargo clippy --all-targets`, and `cargo fmt -- --check` pass for `patchbay-core` with `CARGO_HOME=/tmp/cargo-home`.
- Adjacent issues parked: none.
