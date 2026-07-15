---
id: story-v0-core-authority-proptests
kind: story
stage: done
tags: [security, protocol, verification, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry, story-v0-core-authority-grant-check, story-v0-core-authority-ingest, story-v0-core-authority-spawn-tail, story-v0-core-authority-replay, story-acceptance-issuer-context]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Property tests for authority invariants (7 oracles + 1 documented gap)

## Scope
Implement Unit 6 of `feature-v0-core-authority` (revision 3): property tests for the 8 stated-normative obligations. **7 are executable oracles; 1 (`ElicitationResponderAuthority`) is a documented untested gap** (rev3 R6 — not a vacuous test).

## Units
- `core/tests/authority_proptest.rs` — proptest strategies, 7 property oracles, mutation tests

## Properties (8 — matches authority.qnt; 7 tested, 1 documented gap)
See `feature-v0-core-authority.md` Unit 6:
1. `no_command_without_grant` — deny-by-default.
2. `compound_issuer` — accepted commands use verified `IssuerContext` identity, NOT self-asserted payload actor.
3. `grant_authority_is_command_kinds` — grant checks constrain by canonical OperationKinds.
4. `revocation_prevents_future` — revoked grant denies subsequent checks.
5. `fleet_authority_for_spawn` — a fleet-scope spawn grant authorizes spawn across any adapter; an adapter-scope grant authorizes spawn on that adapter only; a runtime-session grant cannot authorize creating a not-yet-existing session. (rev3-review finding 3: fleet is the default, not the only option — PROTOCOL line 173.)
6. `spawn_creates_descendant_grant` — successful spawn produces a descendant grant.
7. `spawn_revocation_does_not_cascade` — two levers. Mutation-survivable stand-in for the demoted formal property. Test BOTH levers (revoke parent P → P denies + descendant D still authorizes; separately revoke D → D denies).
8. `elicitation_responder_authority` — **NOT TESTED HERE.** Authority does not enforce response-Operation responder matching (`Elicitation.expected_responder_actor`); that's an acceptance/elicitation concern. Documented untested gap (rev3 R6). The obligation is real; owned by a future acceptance responder-validation feature. Do NOT write a vacuous stand-in.

## Mutation tests (NON-VACUITY)
- A buggy registry that CASCADES revocation MUST fail #7.
- A buggy GrantCheck that trusts payload `Operation.sender` MUST fail #2.
Mirror the mutation-test discipline from `acceptance_proptest.rs` / `sessions_proptest.rs`.

## Implementation
Read `core/tests/sessions_proptest.rs` and `core/tests/acceptance_proptest.rs` FIRST. Use `RusqliteStorage::open_in_memory()`. `CARGO_HOME=/tmp/cargo-home` for cargo.

## Acceptance Criteria
- [ ] 7 properties pass against the real implementation
- [ ] #7 fails against a cascade mutation (non-vacuous)
- [ ] #2 fails against a payload-actor-trust mutation (non-vacuous)
- [ ] `replay_matches_live` passes (supplementary)
- [ ] #8 (ElicitationResponderAuthority) documented as an untested gap, NOT a vacuous test

## Notes
- Depends on all 5 prior authority stories + `story-acceptance-issuer-context` (rev3-review finding 4: compound_issuer is an acceptance-authority integration property — the mutation must be acceptance constructing issuer identity from Operation.sender, which requires the IssuerContext call-site change).
- #7 is the highest-value test — executable oracle for a demoted formal property, mutation-survivable by construction.
- #8 is honestly a gap, not a fake test (rev3 R6).

## Implementation notes
- Files changed: `core/tests/authority_proptest.rs`.
- Direct-read implementation: the authority interfaces and existing authority/session/acceptance property-test fixtures fully defined the integration surface; no exploratory fan-out was needed.
- Added seven 100-case executable property oracles: deny-by-default `NoCommandWithoutGrant`; verified-identity `CompoundIssuer`; canonical kind-membership `GrantAuthorityIsCommandKinds`; post-revocation denial `RevocationPreventsFuture`; fleet/adapter/runtime-session spawn containment `FleetAuthorityForSpawn`; deterministic completed-spawn issuance `SpawnCreatesDescendantGrant`; and the two-lever `SpawnRevocationDoesNotCascade` check.
- Added non-vacuity tests `payload_actor_trust_catches_injected_bug` and `cascade_revocation_catches_injected_bug`; each proves the production implementation passes the shared oracle and its injected mutant fails it.
- Added supplementary `replay_matches_live`, covering randomized operator grants, descendant grants, and revocations and comparing full registry equality, live-grant ids, and per-grant records.
- Documented `ElicitationResponderAuthority` as the rev3 R6 untested gap owned by `.work/backlog/backlog-elicitation-responder-authority.md`; no vacuous stand-in test was added.
- Mechanical implementation choice: the cascade mutant wraps `AuthorityRegistry`, derives provenance-linked descendants at revocation observation, and suppresses their later grant-check results; the payload-identity mutant retains verified transport context while replacing only the actor with `Operation.sender`.
- Verification: targeted authority property suite passes (10 tests), full `patchbay-core` suite passes, build passes, and `cargo clippy --all-targets -- -D warnings` is clean.
- Discrepancies from design: none.
- Adjacent issues parked: none.
