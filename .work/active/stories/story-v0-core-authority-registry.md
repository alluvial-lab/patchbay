---
id: story-v0-core-authority-registry
kind: story
stage: done
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: []
release_binding: v0.1.0
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Grant/revocation event model and AuthorityRegistry projection

## Scope
Implement Unit 1 of `feature-v0-core-authority` (revision 3): the in-memory grant record (with `expires_at`/`revocation_policy`/`provenance`), the full grant-matching predicate (domain, verified actor, endpoint narrowing, kind, scope containment), the canonical descendant allowed-kind set, the `target_scope_matches` scope-containment matrix, and the `AuthorityRegistry` projection. Mirrors `SessionRegistry`/`ElicitationSlotLayer`.

## Units
- `core/src/authority/mod.rs` — module root, `AuthorityError` enum, re-exports
- `core/src/authority/state.rs` — `GrantRecord`, `GrantProvenanceKind`, `IssuerRef`, `grant_authorizes`, `target_scope_matches`, `DESCENDANT_GRANT_ALLOWED_KINDS`
- `core/src/authority/events.rs` — encoding helpers
- `core/src/authority/registry.rs` — `AuthorityRegistry` projection (observe fold)
- `core/src/lib.rs` — add `pub mod authority;`

## Implementation
See `feature-v0-core-authority.md` Unit 1 for exact signatures. Key points (addressing rev2 findings #3, B, F):
- `grant_authorizes` takes `IssuerRef<'a>` (a minimal struct: `actor`, `endpoint`, `authority_domain_id`), NOT the `IssuerContext` trait — so story 1 has NO forward dep on story 2's trait (rev2 finding F fix). Story 2's `IssuerContext` impl produces an `IssuerRef`.
- **Domain-equality** (rev2 finding B): `grant_authorizes` checks `grant.authority_domain_id == issuer.authority_domain_id`.
- `target_scope_matches` — the full scope-containment matrix, pinned (rev2 finding #3): FleetSupervisor=any, AuthorityDomain=any, Adapter=same adapter, RuntimeSession=exact tuple, ProjectSessionGroup=containment by project_or_group, Actor=same actor, Resource=exact, Unspecified=never. Read `TargetScopeKind` in `common.proto`.
- `GrantRecord` stores `expires_at` + `revocation_policy` + full provenance (expiry NOT enforced in v0.1.0 — backlog).
- `DESCENDANT_GRANT_ALLOWED_KINDS` — 8 kinds (spawn+attach excluded). SSOT from PROTOCOL.md.
- `observe` validates grant shape (Fail Fast); `observe_revocation` marks revoked (not delete); idempotent.
- Read `core/src/session/registry.rs` and `core/src/acceptance/elicitation.rs` FIRST — direct templates.

## Acceptance Criteria
- [ ] `observe` folds Grant, DescendantGrant, Revocation events correctly
- [ ] Revocation marks revoked (not deleted); `is_live()` returns false after
- [ ] `grant_authorizes` returns true only when: live + domain matches + verified actor matches + (endpoint narrows if present) + kind allowed + target in scope
- [ ] `target_scope_matches` implements the full scope-containment matrix (all 7 kinds + Unspecified=never)
- [ ] `DESCENDANT_GRANT_ALLOWED_KINDS` matches PROTOCOL.md exactly (8 kinds)
- [ ] `observe` rejects malformed grants; idempotent for re-delivered events

## Notes
- No deps. Foundation for stories 2-6.
- `AuthorityError` enum in `mod.rs`: `CorruptRecord`, `CorruptLog`, `InvalidGrant`, `GrantNotFound`, `Storage(#[from])`.
- Do NOT implement GrantCheck, ingest, spawn-tail, or replay here.

## Implementation notes
- Files changed: `core/src/authority/{mod,events,state,registry}.rs`, `core/src/lib.rs`, `core/Cargo.toml`, `Cargo.lock`, and `core/tests/authority_registry.rs`.
- Module structure: added schema-backed Grant/DescendantGrant/Revocation envelope helpers, the in-memory `GrantRecord` and provenance shapes, deny-by-default matching predicates, and an independent `AuthorityRegistry` log projection.
- Matching matrix: implemented the pinned seven-kind containment rules plus fail-closed Unspecified/unknown handling; descendant grants validate against the exact eight-kind canonical set.
- Fold behavior: validates event/message domain identity and grant shape, preserves revoked grants, rejects conflicting grant/revocation duplicates, and retains source creation records so replaying a full grant→revocation prefix remains idempotent.
- Mechanical decisions: added a direct `prost-types` dependency for the public timestamp fields; required non-empty mandatory IDs, concrete target kinds/policies, non-empty normal-grant kind sets, and provenance at the projection boundary under the story's Fail Fast requirement.
- Tests added: 10 behavior tests in `core/tests/authority_registry.rs`, covering all acceptance criteria, full-prefix replay idempotence, conflict detection, malformed records, and non-authority event filtering.
- Verification: `cargo build -p patchbay-core`, full `cargo test -p patchbay-core`, and `cargo clippy --all-targets -- -D warnings` pass with `CARGO_HOME=/tmp/cargo-home`.
- Dispatch: direct-read implementation only; the user-pinned semantics and existing session/elicitation templates left no unresolved integration unknowns.
- Discrepancies from design: none.
- Adjacent issues parked: none.
