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

# Story: IssuerContext port + GrantCheck impl (the acceptance seam)

## Scope
Implement Unit 2 of `feature-v0-core-authority` (revision 2): define the `IssuerContext` port (verified identity, R2) and `impl GrantCheck for AuthorityRegistry` evaluating against durable grants (R1). Addresses review blockers #1 (durable grants, not implicit) and #2 (verified identity, not self-asserted).

## Units
- `core/src/authority/issuer.rs` — `IssuerContext` trait
- `core/src/authority/check.rs` — `impl GrantCheck for AuthorityRegistry`

## Implementation
See `feature-v0-core-authority.md` Unit 2 for exact signatures. Key points:
- `IssuerContext` trait: `verified_actor()`, `verified_endpoint()`, `verified_device()`, `endpoint_generation()`, `authority_domain_id()`. NOT self-asserted — supplied by the authenticated ingress. v0.1.0 tests supply `TestIssuerContext`; real impl lands with `feature-v0-protocol-seam`/`feature-v0-web-server`.
- `GrantCheck::check` signature CHANGES from `actor: &ActorEndpointRef` to `issuer: &dyn IssuerContext`. This is a port-shape change — coordinate with `story-acceptance-issuer-context` (the acceptance call-site update). The GrantCheck impl can be developed against a `TestIssuerContext` double in parallel with the acceptance change.
- `impl GrantCheck for AuthorityRegistry`: deny-by-default. No verified actor → denied. Evaluate against `live_grants()`; first match → `Authorized { grant_id: Some(...) }`. No match → `GrantDenied::NoGrant`.
- The operator is evaluated against the **bootstrap operator grant** (story 3's `ensure_bootstrap_operator_grant`), NOT implicit bypass. No special-casing.
- Read `core/src/session/resolver.rs` (impl `TargetResolver`) FIRST — direct template for implementing an acceptance port on a registry.

## Acceptance Criteria
- [ ] `IssuerContext` trait defined (verified identity port)
- [ ] `check` returns `Authorized { grant_id: Some(...) }` for a verified operator with the bootstrap grant
- [ ] `check` returns `Authorized` for a non-operator with a live matching descendant grant
- [ ] `check` returns `GrantDenied` for an unauthenticated issuer (no verified actor)
- [ ] `check` returns `GrantDenied` for a revoked grant
- [ ] `check` returns `GrantDenied` for a kind/target not covered (deny-by-default)
- [ ] `GrantCheck::check` takes `&dyn IssuerContext` (port-shape change); `TestGrantCheck` updated

## Notes
- Depends on story 1 (registry + `grant_authorizes`).
- Co-developed with `story-acceptance-issuer-context` (the call-site update). The trait is defined HERE; acceptance imports it.
- Add integration tests in `core/tests/authority_grant_check.rs` using a `TestIssuerContext` double.
