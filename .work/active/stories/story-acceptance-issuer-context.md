---
id: story-acceptance-issuer-context
kind: story
stage: done
tags: [security, protocol, foundation]
parent: feature-v0-core-acceptance
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Acceptance submit takes a verified IssuerContext (authority prerequisite)

## Scope
Change the acceptance `submit` pipeline to take a verified `IssuerContext` (defined by the authority feature, R2) instead of reading `Operation.sender` for the grant check. Also: retain the `Authorized.grant_id` on the command record so the descendant-grant reactor can populate provenance. This is a **prerequisite for `feature-v0-core-authority`** (the GrantCheck impl, `story-v0-core-authority-grant-check`, needs it for end-to-end testing).

## Why
The authority design review (blocker #2) proved that reading `Operation.sender` (a self-asserted payload field) for the grant check violates SECURITY.md's compound-issuer rule ("sender identity comes from the verified connection/session context, not from self-asserted payload fields"). The `IssuerContext` port (defined in the authority module) carries verified operator actor + transport endpoint, supplied by the authenticated ingress. The `submit` pipeline must pass it to `GrantCheck::check` instead of `validated.sender`.

Also (blocker #4 follow-on): the descendant-grant reactor needs `spawning_grant_id` for provenance, which comes from the spawn's `Authorized.grant_id`. Today the pipeline discards it (`grant_check.check(...).await.is_err()`). Retain it on the command record.

## The change
- `core/src/acceptance/pipeline.rs`:
  - `submit` signature: add an `issuer: &dyn IssuerContext` parameter (the authority module defines the trait; acceptance depends on the trait, not the impl).
  - The `GrantCheck::check` call passes `issuer` instead of `validated.sender`.
  - Retain the `Authorized.grant_id` (on success) on the `CommandRecord` / accepted operation state, so the spawn-tail reactor can read it.
- `core/src/acceptance/ports.rs`: the `GrantCheck::check` signature changes from `actor: &ActorEndpointRef` to `issuer: &dyn IssuerContext`. (The authority feature owns this port-shape change; this story updates the call site.)
- Update `TestGrantCheck` and `TestTargetResolver` in `core/tests/acceptance_pipeline.rs` to the new signature (supply a `TestIssuerContext` double).

## Acceptance Criteria
- [ ] `submit` takes `issuer: &dyn IssuerContext`; passes it to `GrantCheck::check`
- [ ] `GrantCheck::check` signature takes `&dyn IssuerContext` (not `&ActorEndpointRef`)
- [ ] `Authorized.grant_id` retained on the accepted command record (for spawn-tail provenance)
- [ ] `Operation.sender` remains for audit/recording but is NOT used for authority
- [ ] Existing acceptance tests updated; `cargo build`, `cargo test -p patchbay-core`, `cargo clippy --all-targets` clean

## Notes
- This is an acceptance-feature story (acceptance owns its pipeline) but exists to unblock authority. Filed under `feature-v0-core-acceptance` (re-opens its review surface — re-review the parent when this lands).
- The `IssuerContext` trait is defined in the authority module (`core/src/authority/issuer.rs`, story `story-v0-core-authority-grant-check`). **Sequence:** if authority story 2 hasn't landed the trait yet, this story can define a minimal version and authority refines it. Coordinate via the depends_on chain: this story and `story-v0-core-authority-grant-check` are co-developed.
- `CARGO_HOME=/tmp/cargo-home` for all cargo commands.
- Demanded by authority design review blockers #2 (compound issuer) and #4 (provenance).

## Implementation notes
- Files changed: `core/src/acceptance/pipeline.rs`, `core/src/acceptance/state.rs`, `core/tests/acceptance_pipeline.rs`, `core/tests/acceptance_proptest.rs` (with the `GrantCheck` port signature committed atomically in `story-v0-core-authority-grant-check`).
- Tests added: updated all acceptance pipeline and property-test grant doubles/call sites to supply a configurable verified `TestIssuer`; the authority grant-check integration suite also passes through the new seam.
- Discrepancies from design: `CommandRecord.grant_id` is present and initializes to `None`, but no durable population path was added. `submit` writes only the generated `Operation` event and does not construct the `CommandIndex`; the Operation/snapshot contracts carry no authorizing-grant field. This is the feature's R3 provenance-durability deferral, not a fabricated value.
- Verification: `cargo build -p patchbay-core` passed; focused `authority_grant_check`, `acceptance_pipeline`, and `acceptance_proptest` suites passed (24 tests).
- Adjacent issues parked: none.

## Review (fast lane, 2026-07-14)
Verdict: Approve with comments - story verified by implement; fast-lane advance. Independently confirmed green: 171 tests, clippy clean. Atomic GrantCheck/submit signature change correct; `Operation.sender` retained for audit. Comment: acceptance criterion #3 (`Authorized.grant_id` retained on command record) is nominally unmet — the check result is discarded (`_authorization`) and `CommandRecord.grant_id` is always `None`. This is the feature's explicit R3 provenance-durability deferral (submit doesn't own the CommandIndex; durable encoding needs an Operation-proto change, out of scope), tracked in `backlog-authority-durable-acceptance-metadata`. Field present as a seam. Nit: `ValidatedOperation.sender` is now dead code (`#[allow(dead_code)]`); audit value lives on `operation.sender`. Re-opens `feature-v0-core-acceptance` review surface.
