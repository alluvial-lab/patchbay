---
id: story-formal-model-realignment-spawn
kind: story
stage: review
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
- Files changed: `specs/seed/authority.qnt`, `docs/VERIFICATION.md`, `contracts/scripts/check-vectors.mjs`, `.work/active/stories/story-formal-model-realignment-spawn.md`.
- Tests added: no separate test files; promoted four `authority.qnt` model properties with `@promotion` blocks.
- Model structure: rewrote `authority.qnt` to a focused bounded authority model for Unit SA. Grant records remain explicit tuples (`GrantLive` plus issuer/subject/scope/endpoint/kind/status/target scalar tuple fields for the bounded grant ids). `g-spawn` is the fleet-scope spawn grant; `g-session-spawn` is an active per-session negative-control grant used for the B5 mutation; successful spawn creates the explicit descendant grant record `g-desc-os3` with non-spawn command kinds.
- Mechanical Quint discovery: Apalache temporal checking in this model produced runtime `Operator EQ` errors when boolean/string equality appeared in `and`/`or` chains that compiled but translated ambiguously for temporal checking. Resolved mechanically by using nested `if` helpers and string flags for state booleans; no protocol semantics changed.
- Baseline verification (all exit 0):
  - `quint parse specs/seed/authority.qnt`.
  - `quint compile specs/seed/authority.qnt`.
  - `quint verify specs/seed/authority.qnt --invariant fleet_authority_for_spawn --max-steps 12` → `[ok]`.
  - `quint verify specs/seed/authority.qnt --invariant spawn_creates_descendant_grant --max-steps 12` → `[ok]`.
  - `quint verify specs/seed/authority.qnt --invariant elicitation_responder_authority --max-steps 12` → `[ok]`.
  - `echo y | quint verify specs/seed/authority.qnt --temporal spawn_revocation_does_not_cascade --max-steps 10` → `[ok]`.
- Mutation tests (all intentionally exit 1 with `[violation]`):
  - `FleetAuthorityForSpawn`: mutated `liveFleetSpawnGrant` to authorize from the active per-session `g-session-spawn` grant after the fleet grant is revoked; `fleet_authority_for_spawn` failed with `[violation]`.
  - `SpawnCreatesDescendantGrant`: mutated the successful-spawn action to leave `gDescOs3Live` unchanged instead of inserting the descendant grant record; `spawn_creates_descendant_grant` failed with `[violation]`.
  - `SpawnRevocationDoesNotCascade`: mutated `revokeSpawnGrant` to also set the live descendant grant status to `revoked`; `spawn_revocation_does_not_cascade` failed with `[violation]`.
  - `ElicitationResponderAuthority`: mutated `responseAuthorityOk` to `true`; `elicitation_responder_authority` failed with `[violation]`.
- Traceability: moved `FleetAuthorityForSpawn`, `SpawnCreatesDescendantGrant`, `SpawnRevocationDoesNotCascade`, and `ElicitationResponderAuthority` from `STATED_NORMATIVE_PROPERTIES` to `CHECKED_MODEL_PROPERTIES` in `contracts/scripts/check-vectors.mjs`; updated `docs/VERIFICATION.md`; regenerated model/vector traceability tables.
- Final script checks: `node contracts/scripts/check-models.mjs` exit 0; `node contracts/scripts/check-vectors.mjs` exit 0.
- Existing authority draft properties remained `status: draft`: `NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture`.
- Discrepancies from design: `authority.qnt` uses scalar tuple fields for the bounded grant ids instead of map-valued state to keep Apalache temporal checking reliable; the promoted oracles still query raw grant tuple facts and do not use boolean authority side-channels.
- Adjacent issues parked: none.
