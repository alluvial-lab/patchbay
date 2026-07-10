---
id: story-verification-correction-draft-formulas
kind: story
stage: implementing
tags: [verification]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-alloy-and-toys]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Remove misleading draft formulas from snapshot_recovery and authority

## Scope

Remove the `val` definitions for ten draft properties whose formulas mislead — they look like real checks but don't model the claimed behavior. Removing the `val` entirely (rather than replacing with `true`) ensures `quint verify --invariant <name>` fails because the invariant doesn't exist, rather than passing vacuously. The `@promotion` blocks stay (status: draft, invocation: `<TBD>`) so the property ids survive as stated-normative obligations. Also fix the VERIFICATION.md authority-tier contradiction.

## Unit

`Unit 4` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/snapshot_recovery.qnt` — six draft `val` definitions to remove
- `specs/seed/authority.qnt` — four draft `val` definitions to remove
- `docs/VERIFICATION.md` — authority-tier contradiction (lines ~116-133)

## Implementation

### Remove misleading `val` definitions

For each of the ten properties, remove the `val <name> = <formula>` definition entirely. Keep the `@promotion` block above it (status stays draft, invocation stays `<TBD>`). Do not change the model's actions or state.

**`snapshot_recovery.qnt`** (6 properties):
- `SnapshotStaleRejected` — checks `SnapshotRevision >= Cursor` (non-decreasing revision), not that stale snapshots are rejected as authority sources.
- `SnapshotCrossDomainRejected` — checks current snapshot origin matches core, not that cross-domain snapshots are rejected.
- `SnapshotConsistentPrefix` — checks lookup-table consistency, not that materialization reads a consistent log prefix.
- `LateEventNoRewrite` — checks key existence, not that late events don't rewrite state.
- `CrashNoAcceptedLost` — copies `PreCrashRecoveredState` into `RecoveredCommandState` during replay rather than deriving from log entries — assumes the answer.
- `IdempotentLogReplay` — checks numeric bounds, not that replay produces identical state.

**`authority.qnt`** (4 properties):
- `NoCommandWithoutGrant = true` — literal placeholder.
- `CompoundIssuer = true` — literal placeholder.
- `GrantAuthorityIsCommandKinds = true` — literal placeholder.
- `RevocationPreventsFuture = always(true)` — literal placeholder.

### Fix VERIFICATION.md authority-tier contradiction

The `### authority.qnt promotion status` section (lines ~116-133) contradicts itself: it says the four general authority properties "remain draft/stated-normative" but then lists them under "Checked-model spawn properties." Correct the list to include only the genuinely promoted spawn properties (`FleetAuthorityForSpawn`, `SpawnCreatesDescendantGrant`, `SpawnRevocationDoesNotCascade`, `ElicitationResponderAuthority`) and explicitly state the four general properties (`NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture`) are stated-normative with no executable formula.

## Acceptance criteria

- [ ] All ten draft property `val` definitions removed from `snapshot_recovery.qnt` and `authority.qnt`.
- [ ] `@promotion` blocks unchanged (status stays draft, invocation stays `<TBD>`).
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (no metadata change).
- [ ] `quint parse specs/seed/snapshot_recovery.qnt` and `quint parse specs/seed/authority.qnt` exit 0.
- [ ] VERIFICATION.md authority-tier contradiction fixed: only the 4 genuinely promoted spawn properties listed as checked-model; the 4 general properties explicitly stated-normative with no executable formula.
- [ ] No generated table change required (tables derive from `@promotion` metadata, which is unchanged).
