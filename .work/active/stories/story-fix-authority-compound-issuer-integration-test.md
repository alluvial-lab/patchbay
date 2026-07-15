---
id: story-fix-authority-compound-issuer-integration-test
kind: story
stage: review
tags: [verification, security, protocol]
parent: feature-v0-core-authority
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Story: CompoundIssuer proptest must drive acceptance::submit end-to-end, not GrantCheck directly

## Source
Deep review of `feature-v0-core-authority` (Phase 1 + Phase 2, both reviewers).

## Finding
The `compound_issuer` property (#2) oracle `compound_issuer_holds` (`core/tests/authority_proptest.rs`) calls `GrantCheck::check` DIRECTLY on `AuthorityRegistry`, passing a `TestIssuerContext` whose verified actor differs from the payload actor. It asserts the real `AuthorityRegistry` denies the mismatched verified actor, and that a `PayloadTrustingGrantCheck` mutant authorizes the payload actor.

This tests that `AuthorityRegistry`'s `GrantCheck` impl uses verified identity — the core claim of property #2. BUT rev3-review finding 4 (recorded in `feature-v0-core-authority.md:474-477`) explicitly states this is an **ACCEPTANCE-AUTHORITY integration property**: "GrantCheck no longer receives `Operation.sender`, so the mutation must be acceptance constructing issuer identity from `Operation.sender`." The proptest's `depends_on` includes `story-acceptance-issuer-context` precisely because the mutation should be acceptance-side.

The current oracle does NOT drive `acceptance::submit`. So a regression where `submit` constructs the `IssuerContext` from `Operation.sender` (the payload) instead of from verified connection evidence would NOT be caught. The mutant tested is a hand-rolled `PayloadTrustingGrantCheck`, not the actual acceptance→issuer construction path.

## Impact
The property's non-vacuity is partial: it proves `AuthorityRegistry` rejects mismatched verified actors, but does not prove the `submit` call site passes a verified issuer (not the payload sender). The end-to-end compound-issuer guarantee is undertested. This is the exact class of gap (a property that passes green but misses a real bypass) the verification program exists to prevent.

## Fix
Add an integration test (or strengthen the proptest oracle) that:
1. Drives `acceptance::submit` with a real `AuthorityRegistry` (impl `GrantCheck`) + a `TestIssuerContext` whose verified actor is A.
2. Submits an `Operation` whose `sender.actor_id` is B (different from A).
3. Asserts `submit` rejects with `AuthorizationDenied` (because the grant is for A, the verified actor, and the registry denies B).
4. Mutation: a `submit` variant / test double that constructs the `IssuerContext` from `Operation.sender` (B) instead of the verified context (A) must AUTHORIZE (and the oracle must catch that).

This requires the `submit` signature (which takes `issuer: &dyn IssuerContext`) — so the test asserts the call site passes the verified issuer, not the payload. This is the integration boundary rev3-review finding 4 intended.

## Acceptance Criteria
- [ ] An integration test drives `submit` with a verified issuer A and a payload sender B (A != B); asserts rejection
- [ ] A mutation where `submit` derives the issuer from `Operation.sender` is caught by the oracle
- [ ] The existing direct-`GrantCheck` oracle is retained (it tests the impl); the new test covers the call site

## Notes
- This is a verification-coverage gap, not a code bug. The `submit` call site IS correct (passes `issuer`, not `validated.sender`) — confirmed in re-review of `story-acceptance-issuer-context`. The gap is that no test *proves* it stays correct.
- `[verification]`-tagged → deep review lane when this story reaches review.

## Implementation notes
- Files changed: `core/tests/authority_proptest.rs`; `.work/active/stories/story-fix-authority-compound-issuer-integration-test.md`.
- Tests added: `compound_issuer_integration_denies_payload_actor_mismatch_through_submit` drives the real `acceptance::submit` → `AuthorityRegistry::check` path twice: verified actor A with payload actor B is rejected with `AuthorizationDenied`, while substituting payload actor B as the issuer is accepted.
- Regression caught: `submit` deriving authority identity from `Operation.sender` instead of the ingress-supplied `IssuerContext` makes the first assertion fail; the accepted payload-derived control call proves the fixture is non-vacuous.
- Discrepancy from the brief: the literal phrase "grant for verified actor A" cannot produce both the required A-denial and B-acceptance mutation. The fixture follows the existing `compound_issuer` oracle and the required mutation behavior by granting payload actor B while verified actor A has no matching grant.
- Verification: package build, focused authority property test (11 passed), full `patchbay-core` suite, and Clippy with warnings denied all pass.
- Adjacent issues parked: none.
