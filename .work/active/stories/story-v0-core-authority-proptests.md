---
id: story-v0-core-authority-proptests
kind: story
stage: implementing
tags: [security, protocol, verification, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry, story-v0-core-authority-grant-check, story-v0-core-authority-ingest, story-v0-core-authority-spawn-tail, story-v0-core-authority-replay]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Property tests for authority invariants

## Scope
Implement Unit 6 of `feature-v0-core-authority`: property tests for the stated-normative obligations. None are formally checked (all `authority.qnt` properties are draft), but each is testable as an executable oracle — mirroring how sessions tested stated-normative obligations as properties.

## Units
- `core/tests/authority_proptest.rs` — proptest strategies, property oracles, mutation tests

## Properties
See `feature-v0-core-authority.md` Unit 6 for the full list:
- `no_command_without_grant` — stated-normative (`NoCommandWithoutGrant`): deny-by-default.
- `revocation_prevents_future` — stated-normative (`RevocationPreventsFuture`): revoked grant denies subsequent checks.
- `spawn_revocation_does_not_cascade` — stated-normative (`SpawnRevocationDoesNotCascade`, one of the DEMOTED formal properties): revoking a spawn grant does NOT revoke descendant grants. **This is the executable stand-in for the demoted formal property — it MUST be mutation-survivable.**
- `descendant_grant_allowed_kinds_exact` — descendant grants have exactly the canonical allowed-kind set.
- `replay_matches_live` — replay determinism.

## Mutation tests (NON-VACUITY)
A buggy registry that CASCADES revocation MUST fail `spawn_revocation_does_not_cascade`. Mirror the mutation-test discipline from `acceptance_proptest.rs` / `sessions_proptest.rs`.

## Implementation
Read `core/tests/sessions_proptest.rs` and `core/tests/acceptance_proptest.rs` FIRST — they're the templates for proptest strategies + non-vacuous mutation tests. Use `RusqliteStorage::open_in_memory()` for full write→replay round-trips.

## Acceptance Criteria
- [ ] `no_command_without_grant` passes (deny-by-default)
- [ ] `revocation_prevents_future` passes
- [ ] `spawn_revocation_does_not_cascade` passes AND FAILS against a cascade mutation (non-vacuous — executable stand-in for the demoted formal property)
- [ ] `descendant_grant_allowed_kinds_exact` passes
- [ ] `replay_matches_live` passes

## Notes
- Depends on all 5 prior stories.
- `spawn_revocation_does_not_cascade` is the highest-value test — it's the executable oracle for a formal property that was demoted (not mutation-survivable in `authority.qnt`). The property test here IS mutation-survivable by construction.
- All properties are stated-normative (no promoted formulas). They document + enforce intended behavior as executable oracles.
