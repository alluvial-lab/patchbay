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
Implement Unit 2 of `feature-v0-core-authority` (revision 3): define the `IssuerContext` port (verified identity, R2) and `impl GrantCheck for AuthorityRegistry` evaluating against durable grants. Addresses review blockers #1 (no implicit bypass — evaluates durable grants) and #2 (verified identity, not self-asserted) + rev2 finding B (domain-equality).

## Units
- `core/src/authority/issuer.rs` — `IssuerContext` trait
- `core/src/authority/check.rs` — `impl GrantCheck for AuthorityRegistry`

## Implementation
See `feature-v0-core-authority.md` Unit 2 for exact signatures. Key points:
- `IssuerContext` trait: `verified_actor()`, `verified_endpoint()`, `verified_device()`, `endpoint_generation()`, `authority_domain_id()`. NOT self-asserted. v0.1.0 tests supply `TestIssuerContext`; real verifier lands with the ingress.
- `GrantCheck::check` signature CHANGES from `actor: &ActorEndpointRef` to `issuer: &dyn IssuerContext`. Coordinate with `story-acceptance-issuer-context` (call-site update). The trait is defined HERE; acceptance imports it. **Explicit edge:** this story (defines the trait) → `story-acceptance-issuer-context` (uses it). No "co-developed" ambiguity (rev2 finding F fixed).
- **Domain-equality pinned** (rev2 finding B): `issuer.authority_domain_id() != authority_domain_id` → denied. No payload-domain-override hole.
- `impl GrantCheck for AuthorityRegistry`: deny-by-default. No verified actor → denied. Build `IssuerRef` from the context; evaluate against `live_grants()` via `grant_authorizes`; first match → `Authorized { grant_id: Some(...) }`; no match → `GrantDenied::NoGrant`.
- No bootstrap grant, no implicit operator bypass (R1 dropped). The operator is evaluated against whatever durable grants exist (injected in tests).
- Read `core/src/session/resolver.rs` (impl `TargetResolver`) FIRST — direct template.

## Acceptance Criteria
- [ ] `IssuerContext` trait defined (verified identity port)
- [ ] `check` returns `Authorized` for a verified issuer with a live matching grant
- [ ] `check` returns `GrantDenied` for an unauthenticated issuer (no verified actor)
- [ ] `check` returns `GrantDenied` for a cross-domain issuer (domain mismatch — rev2 finding B)
- [ ] `check` returns `GrantDenied` for a revoked grant
- [ ] `check` returns `GrantDenied` for a kind/target not covered (deny-by-default)
- [ ] `GrantCheck::check` takes `&dyn IssuerContext` (port-shape change)

## Notes
- Depends on story 1 (registry + `grant_authorizes` + `IssuerRef`).
- The trait defined here is used by `story-acceptance-issuer-context`. Explicit edge, not co-developed.
- Add integration tests in `core/tests/authority_grant_check.rs` using a `TestIssuerContext` double.
