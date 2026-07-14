---
id: story-v0-core-authority-registry
kind: story
stage: implementing
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Grant/revocation event model and AuthorityRegistry projection

## Scope
Implement Unit 1 of `feature-v0-core-authority` (revision 2): the in-memory grant record (with `expires_at`/`revocation_policy`/`provenance`), the full grant-matching predicate (domain, verified actor, endpoint narrowing, kind, scope containment), the canonical descendant allowed-kind set, the `target_scope_matches` scope-containment matrix, and the `AuthorityRegistry` projection. Mirrors `SessionRegistry`/`ElicitationSlotLayer`.

## Units
- `core/src/authority/mod.rs` — module root, `AuthorityError` enum, re-exports
- `core/src/authority/state.rs` — `GrantRecord`, `GrantProvenanceKind`, `grant_authorizes`, `target_scope_matches`, `DESCENDANT_GRANT_ALLOWED_KINDS`
- `core/src/authority/events.rs` — encoding helpers (Grant/DescendantGrant/Revocation → StoredEventPayload)
- `core/src/authority/registry.rs` — `AuthorityRegistry` projection (observe fold)
- `core/src/lib.rs` — add `pub mod authority;`

## Implementation
See `feature-v0-core-authority.md` Unit 1 for exact signatures. Key points (addressing review blockers #3, #8):
- `GrantRecord` stores `expires_at` + `revocation_policy` + full provenance (Stored but expiry not enforced in v0.1.0 — backlog).
- `grant_authorizes` takes the verified `IssuerContext` (not payload `ActorEndpointRef`) — but `IssuerContext` is defined in story 2. **For this story**, define `grant_authorizes` to take the actor/endpoint/domain as explicit parameters (or define a minimal `IssuerRef` struct story 2 will refine). Coordinate: the signature in the design takes `issuer: &IssuerContext`; if that trait isn't defined yet, define a minimal version here and story 2 extends it.
- `target_scope_matches` — the scope-containment matrix (fleet=any, adapter=same adapter, runtime-session=exact tuple, project-group=containment). **This is the semantic 50/50 the review flagged — pinned in the design, implemented here, not left to the implementer.** Read `TargetScopeKind` in `common.proto`.
- `DESCENDANT_GRANT_ALLOWED_KINDS` — 8 kinds (instruct/cancel/interrupt/query/approval-response/elicitation-response/reconfigure/session-management; spawn+attach excluded). SSOT from PROTOCOL.md.
- `observe` validates grant shape (Fail Fast); `observe_revocation` marks revoked (not delete); idempotent.
- Read `core/src/session/registry.rs` and `core/src/acceptance/elicitation.rs` FIRST — direct templates.

## Acceptance Criteria
- [ ] `observe` folds Grant, DescendantGrant, Revocation events correctly
- [ ] Revocation marks the grant revoked (not deleted); `is_live()` returns false after
- [ ] `grant_authorizes` returns true only when: live + domain matches + verified actor matches + (endpoint narrows if present) + kind allowed + target in scope
- [ ] `target_scope_matches` implements the scope-containment matrix
- [ ] `DESCENDANT_GRANT_ALLOWED_KINDS` matches PROTOCOL.md exactly (8 kinds)
- [ ] `observe` rejects malformed grants; idempotent for re-delivered events

## Notes
- No deps. Foundation for stories 2-6.
- `AuthorityError` enum in `mod.rs`: `CorruptRecord`, `CorruptLog`, `InvalidGrant`, `GrantNotFound`, `Storage(#[from])`.
- Do NOT implement GrantCheck, ingest, spawn-tail, or replay here.
