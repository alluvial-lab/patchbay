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

# Story: Property tests for authority invariants (8 properties)

## Scope
Implement Unit 6 of `feature-v0-core-authority` (revision 2): property tests for the **8** stated-normative obligations (review blocker #10: corrected count). None are formally checked, but each is testable as an executable oracle.

## Units
- `core/tests/authority_proptest.rs` — proptest strategies, 8 property oracles, mutation tests

## Properties (8 — matches authority.qnt exactly)
See `feature-v0-core-authority.md` Unit 6 for the full list:
1. `no_command_without_grant` — deny-by-default.
2. `compound_issuer` — accepted commands use verified `IssuerContext` identity, NOT self-asserted payload actor.
3. `grant_authority_is_command_kinds` — grant checks constrain by canonical OperationKinds, not adapter capability.
4. `revocation_prevents_future` — revoked grant denies subsequent checks.
5. `fleet_authority_for_spawn` — spawn requires a live fleet-scope spawn grant; per-session grants can't authorize spawn.
6. `spawn_creates_descendant_grant` — successful spawn produces a descendant grant with non-spawn OperationKinds.
7. `spawn_revocation_does_not_cascade` — revoking a spawn grant does NOT revoke descendant grants. Two levers. **Executable stand-in for the demoted formal property — MUST be mutation-survivable.** Test BOTH levers (revoke parent P → P denies + descendant D still authorizes; separately revoke D → D denies).
8. `elicitation_responder_authority` — response Operations accepted only when verified issuer maps to expected responder.

## Mutation tests (NON-VACUITY)
- A buggy registry that CASCADES revocation MUST fail #7.
- A buggy GrantCheck that trusts payload `Operation.sender` MUST fail #2.
Mirror the mutation-test discipline from `acceptance_proptest.rs` / `sessions_proptest.rs`.

## Implementation
Read `core/tests/sessions_proptest.rs` and `core/tests/acceptance_proptest.rs` FIRST. Use `RusqliteStorage::open_in_memory()` for full write→replay round-trips. `CARGO_HOME=/tmp/cargo-home` for cargo.

## Acceptance Criteria
- [ ] All 8 properties pass against the real implementation
- [ ] #7 fails against a cascade mutation (non-vacuous — executable stand-in for the demoted formal property)
- [ ] #2 fails against a payload-actor-trust mutation (non-vacuous)
- [ ] `replay_matches_live` passes (supplementary — replay determinism)

## Notes
- Depends on all 5 prior authority stories.
- #7 is the highest-value test — executable oracle for a demoted formal property, mutation-survivable by construction.
- All 8 are stated-normative (no promoted formulas). They document + enforce intended behavior as executable oracles.
