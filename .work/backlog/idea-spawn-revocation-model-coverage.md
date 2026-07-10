---
id: idea-spawn-revocation-model-coverage
kind: backlog
stage: backlog
tags: [verification, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# SpawnRevocationDoesNotCascade model coverage is narrower than its name

## Origin

Deep review (Phase 2 adversarial) of `epic-public-product-contract-verification-claim-correction`. Filed as an important finding, not a blocker: the property's narrowed `semantics:` text is accurate to what its formula proves over reachable traces, but the formula's reach is narrower than the property name implies.

## Finding

`SpawnRevocationDoesNotCascade` (`specs/seed/authority.qnt`) is `status: promoted`. Its temporal formula `spawn_revocation_does_not_cascade = always(spawnRevocationNoCascadeState)` checks:

> when `gSpawnStatus == "revoked"`: `(if gDescOs3Live == "yes" then gDescOs3Status == "active" else true) and SpawnAccepted == "no"`

The `revokeSpawnGrant` action preserves `gDescOs3Live` and `gDescOs3Status`, so fleet revocation genuinely does not touch the descendant — and the narrowed semantics ("when a descendant grant exists, does not revoke it") is accurate over reachable traces.

However:
- When `gDescOs3Live == "no"` (no descendant exists), the descendant clause is `true`, so the formula cannot detect a mutation that *deletes* a descendant grant. The model has only one descendant slot (`g-desc-os3`), so this is not currently exploitable, but the formula does not explicitly assert pre/post existence preservation.
- `revokeDescendantGrant` (the separate-revocation lever, line 233) exists as an action but is **not in the `step` relation** (only phases 0-3: attemptSpawn/revokeSpawnGrant/spawnAfterRevoke/attemptElicitationResponse are reachable). So the model never explores the interaction between fleet revocation and a separately-revoked descendant.

## Why not a blocker

- The narrowed `semantics:` text (corrected in Unit 6) matches what the formula proves over reachable traces.
- The design explicitly defers real failure-boundary modeling (two-state pre/post, competing candidates, the cascade path) to the v1 formal gate (`epic-public-product-contract-executable-release-assurance`).
- The demotion pattern used for the other 11 properties is not the right call here: the formula IS a genuine mutation-survivable independent oracle for the reachable traces; the gap is model coverage, not formula vacuity.

## Suggested v1 gate work

When the v1 formal gate models this property:
- Add `revokeDescendantGrant` to the `step` relation so the separate-revocation path is reachable.
- Strengthen the formula to assert pre/post state: if a descendant grant existed before fleet-grant revocation, the revocation transition preserves its existence and status; distinguish fleet revocation from separate descendant revocation and mutation-test both status revocation and grant deletion.
- Consider whether the single-descendant-slot abstraction is sufficient or whether a multi-descendant model is needed.
