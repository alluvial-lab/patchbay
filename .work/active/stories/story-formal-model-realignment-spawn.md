---
id: story-formal-model-realignment-spawn
kind: story
stage: done
tags: [verification, protocol, foundation]
parent: feature-formal-model-realignment
depends_on: [story-formal-model-realignment-elicitation]
created: 2026-07-08
updated: 2026-07-08
gate_origin: null
release_binding: null
---

# Story: Spawn authority (Unit SA — promote into authority.qnt)

Implements Unit SA from `feature-formal-model-realignment`. Promotes 4 spawn-authority properties into the existing draft `specs/seed/authority.qnt`, reusing its real grant tuples (Q3 Option 3, B5 fix). The existing 4 draft authority properties STAY draft; only the 4 new spawn properties are promoted.

## Scope

Extend `specs/seed/authority.qnt` with spawn-specific state and seed-data changes.

**Seed-data change (B5 residual fix):** the current `authority.qnt` init has no `spawn` kind and no fleet scope. Add a fleet-scope spawn grant to the seed `Grant`/`GrantScopeById`/`GrantCommandKinds`/`GrantStatus` maps (e.g. `g-spawn` with `GrantScopeById = scope-fleet`, `GrantCommandKinds` containing `spawn`, `GrantStatus = active`).

**Descendant grants are REAL `Grant` records (B5 fix), not parallel maps:** descendant grants created by spawn are inserted as new `Grant` records into the existing grant tuples (`GrantIssuer`, `GrantSubject`, `GrantScopeById`, `GrantEndpoint`, `GrantCommandKinds`, `GrantStatus`, `TargetGeneration`, `RevocationGeneration`), with issuer=spawner, subject=spawner, target=spawned session, scope=per-session, kind-set excluding spawn.

Actions (permissive): `attemptSpawn`, `revokeSpawnGrant` (sets fleet grant `GrantStatus = revoked`; does NOT touch descendant grants — no-cascade), `revokeDescendantGrant` (sets descendant `GrantStatus = revoked` — separate lever), `spawnAfterRevoke` (permissive; must be rejected).

## Checked properties (4, `status: promoted`; tier derived by Unit TR)

- `FleetAuthorityForSpawn` (invariant) — genuine check: queries `Grant`/`GrantScopeById`/`GrantCommandKinds`/`GrantStatus` tuples for a fleet-scope, spawn-kind, active, subject-matching grant — not a boolean.
- `SpawnCreatesDescendantGrant` (invariant) — successful spawn inserts an explicit descendant `Grant` record into the grant tuples.
- `SpawnRevocationDoesNotCascade` (temporal) — revoking spawn grant (`GrantStatus = revoked`) prevents future spawns but does not change descendant grants' `GrantStatus`.
- `ElicitationResponderAuthority` (invariant) — response Operation accepted only from authenticated endpoint for expected responder actor.

## Bounds and invocation (N2)

- Bounds: 3 actors × 3 sessions × 2 grant scopes × 2 grant statuses. `--max-steps 12` invariants; `--max-steps 10` temporal.
- Invariants: `quint verify authority.qnt --invariant fleet_authority_for_spawn --max-steps 12` (and the other 3).
- Temporal: `echo y | quint verify authority.qnt --temporal spawn_revocation_does_not_cascade --max-steps 10`.

## Acceptance Criteria

- [ ] `quint parse` + `quint compile` exit 0 (extended `authority.qnt` compiles).
- [ ] All 4 new properties pass.
- [ ] Mutation test `FleetAuthorityForSpawn`: allowing spawn with only a per-session (non-fleet) grant fails the property (B5 — not a boolean).
- [ ] Mutation test `SpawnRevocationDoesNotCascade`: cascading revocation (revokeSpawnGrant also revokes descendant) fails the property.
- [ ] The existing 4 draft properties remain `status: draft` (no accidental promotion).
- [ ] `@promotion` blocks present (no `tier` field); `check-models.mjs` exits 0; VERIFICATION.md updated.

## Key files

- Edit: `specs/seed/authority.qnt` (+ regenerate `.emitted.tla` if applicable)
- Edit: `docs/VERIFICATION.md`, `contracts/scripts/check-vectors.mjs` (arrays)
- Design reference: `.work/active/features/feature-formal-model-realignment.md` Unit SA

## Implementation notes
- Files changed: `specs/seed/authority.qnt`, `.work/active/stories/story-formal-model-realignment-spawn.md`.
- Tests added: no separate test files; re-ran the four promoted `authority.qnt` properties and mutation/genuine-checking probes.
- Review-fix restructure: split the action-side spawn guard into `actionGrantAuthorizesSpawn(g, actor)` plus `liveFleetSpawnGrant(actor)` over `SPAWN_GRANT_IDS = {g-spawn, g-session-spawn}`. `attemptSpawn` remains permissive in recording raw submitted `(actor, targetSession)` into `LastSpawnActor`/`LastSpawnTarget`/`SpawnAttempted`; only `SpawnAccepted` and descendant insertion are gated by the action helper. The invariant oracle `acceptedSpawnHasRawFleetGrant` still does not call `liveFleetSpawnGrant` or `actionGrantAuthorizesSpawn`; it independently inspects the raw fleet grant tuple for `g-spawn`.
- Baseline verification (all exit 0):
  - `quint parse specs/seed/authority.qnt`.
  - `quint compile specs/seed/authority.qnt`.
  - `quint verify specs/seed/authority.qnt --invariant fleet_authority_for_spawn --max-steps 12` → `[ok]`.
  - `quint verify specs/seed/authority.qnt --invariant spawn_creates_descendant_grant --max-steps 12` → `[ok]`.
  - `quint verify specs/seed/authority.qnt --invariant elicitation_responder_authority --max-steps 12` → `[ok]`.
  - `echo y | quint verify specs/seed/authority.qnt --temporal spawn_revocation_does_not_cascade --max-steps 10` → `[ok]`.
- Mutation/genuine-checking tests (each mutation reverted before the next; all intentionally exit 1 with `[violation]`):
  - `FleetAuthorityForSpawn`: mutated action guard `FLEET_SCOPES.contains(grantScope(g))` → `true`; after fleet-grant revocation the active per-session `g-session-spawn` grant authorized a spawn, and `fleet_authority_for_spawn` failed with `[violation]`.
  - `SpawnCreatesDescendantGrant`: mutated successful spawn to leave `gDescOs3Live` unchanged instead of inserting the descendant grant record; `spawn_creates_descendant_grant` failed with `[violation]`.
  - `SpawnRevocationDoesNotCascade`: mutated `revokeSpawnGrant` to set the live descendant grant status to `revoked`; `spawn_revocation_does_not_cascade` failed with `[violation]`.
  - `ElicitationResponderAuthority`: mutated `responseAuthorityOk` endpoint check `endpointActor(endpoint) == expectedResponder(responseOpId)` → `true`; `elicitation_responder_authority` failed with `[violation]`.
- Final script checks: `node contracts/scripts/check-models.mjs` exit 0; `node contracts/scripts/check-vectors.mjs` exit 0.
- Existing authority draft properties remained `status: draft`: `NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture`.
- `@promotion` block shape checked by script: present, with no `tier` field.
- Discrepancies from design: none for the review fix; this preserves the existing scalar tuple representation noted in the first implementation pass.
- Adjacent issues parked: none.

## Review findings (deep lane, pass 1 — 2026-07-08)

**Verdict**: Block (bounce to implementing)

**B1 — `FleetAuthorityForSpawn` is self-defining (genuine-checking failure, B5 residual).** The action's guard `spawnAllowed` → `liveFleetSpawnGrant` inspects the *same* grant tuples (`grantLive`, `grantStatus`, `grantScope`, `grantAllows`, `grantSubject`) as the invariant's oracle `acceptedSpawnHasRawFleetGrant`. They are effectively the same function. So breaking the fleet-scope check in `liveFleetSpawnGrant` breaks the oracle identically — the invariant cannot detect a broken guard.

Host reproduction (cross-model, umans reviewing codex):
- Baseline: `quint verify --invariant fleet_authority_for_spawn --max-steps 12` → `[ok]` (7s).
- Mutation: broke `liveFleetSpawnGrant`'s fleet-scope check (`FLEET_SCOPES.contains(grantScope("g-spawn"))` → `true`) so a per-session grant authorizes spawn.
- Result: invariant STILL `[ok]` (8s) — cannot detect the broken fleet-scope rule. Self-defining.
- Reverted; tree clean.

The implementer's mutation-test claim (per-session grant → `[violation]`) did NOT reproduce — same false-claim pattern as the elicitation story.

**Fix required (the elicitation/reply_correlation B1 resolution pattern):** the action must record the spawn attempt *permissively* — record the raw submitted `(actor, targetSession)` and set `SpawnAccepted` based on a computation that is INDEPENDENT of the oracle. The oracle `acceptedSpawnHasRawFleetGrant` must consult grant-tuple facts via a *different* path than `liveFleetSpawnGrant` uses, OR (cleaner) the action must be permissive (record attempts regardless of grant state, flag `SpawnAccepted` from an independent check) and the oracle independently inspects the grants. The key test: break `liveFleetSpawnGrant` and the invariant MUST fail `[violation]`.

Reference: `specs/seed/reply_correlation.qnt` `recordedReplyIndependentOk` (does not call `typedReferenceOk`); `specs/seed/elicitation_lifecycle.qnt` `recordedResponseIndependentOk` (does not call `responseMatchesTarget`). The oracle and the guard must be different functions over the state.

Apply the same discipline check to the other 3 properties (`SpawnCreatesDescendantGrant`, `SpawnRevocationDoesNotCascade`, `ElicitationResponderAuthority`) — verify each via mutation test that breaks the action guard and confirms `[violation]`.
