---
id: story-v0-core-authority-grant-check
kind: story
stage: implementing
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: GrantCheck impl (the acceptance seam)

## Scope
Implement Unit 2 of `feature-v0-core-authority`: `impl GrantCheck for AuthorityRegistry`. v0.1.0 hybrid: operator actor → implicit authority (`Authorized { grant_id: None }`); non-operator → durable grant evaluation (deny-by-default).

## Units
- `core/src/authority/check.rs` — `impl GrantCheck for AuthorityRegistry`, `is_operator`, `OPERATOR_ACTOR_ID`

## Implementation
See `feature-v0-core-authority.md` Unit 2 for exact signatures. The `GrantCheck` port ALREADY EXISTS in `core/src/acceptance/ports.rs` — read it first; you implement it, you do NOT redeclare it.

Key points:
- v0.1.0 hybrid: operator actor → `Authorized { grant_id: None }` (implicit). The `None` grant_id is reserved for this per `ports.rs`.
- `OPERATOR_ACTOR_ID = "operator"` constant — the single-operator v0.1.0 assumption made explicit (a deployment value, not a protocol constant).
- Non-operator: iterate `live_grants()`, return `Authorized { grant_id: Some(...) }` on first match via `grant_authorizes`.
- Deny-by-default: no match → `GrantDenied::NoGrant`.
- Read `core/src/session/resolver.rs` (impl `TargetResolver` for `SessionRegistry`) FIRST — it's the direct template for implementing an acceptance port on a registry.

## Acceptance Criteria
- [ ] `check` returns `Authorized { grant_id: None }` for the operator actor
- [ ] `check` returns `Authorized { grant_id: Some(...) }` for a non-operator with a live matching grant
- [ ] `check` returns `GrantDenied::NoGrant` for a non-operator with no matching grant (deny-by-default)
- [ ] `check` returns `GrantDenied` for a revoked grant (revocation prevents future)
- [ ] `check` returns `GrantDenied` for a kind not in the grant's allowed set

## Notes
- Depends on story 1 (registry + `grant_authorizes`).
- Add integration tests in `core/tests/authority_grant_check.rs`.
- Do NOT implement ingest, spawn-tail, or replay here.
