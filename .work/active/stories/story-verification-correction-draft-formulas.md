---
id: story-verification-correction-draft-formulas
kind: story
stage: done
tags: [verification]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-alloy-and-toys]
release_binding: v0.1.0
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Remove misleading draft formulas and demote SpawnCreatesDescendantGrant

## Scope

Remove the `val` definitions for eleven draft properties whose formulas mislead — they look like real checks but don't model the claimed behavior. Removing the `val` entirely (rather than replacing with `true`) ensures `quint verify --invariant <name>` fails because the invariant doesn't exist, rather than passing vacuously. Also demote `SpawnCreatesDescendantGrant` from promoted to draft (it uses invented kind names contradicting PROTOCOL.md:181 and a hard-coded allowed-kind function that isn't mutation-survivable). The `@promotion` blocks stay (status: draft, invocation: `<TBD>`) so the property ids survive as stated-normative obligations. Also fix the VERIFICATION.md authority-tier contradiction.

## Unit

`Unit 4` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/snapshot_recovery.qnt` — six draft `val` definitions to remove
- `specs/seed/authority.qnt` — five `val` definitions to remove (four already-draft + `SpawnCreatesDescendantGrant` to demote first)
- `contracts/scripts/check-vectors.mjs` — move `SpawnCreatesDescendantGrant` from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES`
- `docs/VERIFICATION.md` — authority-tier contradiction (lines ~116-133)

## Implementation

### Demote SpawnCreatesDescendantGrant

1. In `specs/seed/authority.qnt`, in the `SpawnCreatesDescendantGrant` `@promotion` block:
   - Change `status: promoted` → `status: draft`
   - Replace `invocation` with `<TBD — demoted; model uses invented kind names (reboot/snapshot/stop_session) contradicting PROTOCOL.md:181; allowed-kind set is a hard-coded pure function, not action-created state; v1 formal gate owns the real property>`
   - Add `demotion_reason: uses invented kind names contradicting canonical descendant-grant allowed-kind set; allowed-kind set is hard-coded, not mutation-survivable`

2. In `contracts/scripts/check-vectors.mjs`:
   - Remove `SpawnCreatesDescendantGrant` from `CHECKED_MODEL_PROPERTIES`
   - Add it to `STATED_NORMATIVE_PROPERTIES`

### Remove misleading `val` definitions

For each of the eleven properties, remove the `val <name> = <formula>` definition entirely. Keep the `@promotion` block above it (status stays draft, invocation stays `<TBD>`). Do not change the model's actions or state.

**`snapshot_recovery.qnt`** (6 properties):
- `SnapshotStaleRejected` — checks `SnapshotRevision >= Cursor` (non-decreasing revision), not that stale snapshots are rejected as authority sources.
- `SnapshotCrossDomainRejected` — checks current snapshot origin matches core, not that cross-domain snapshots are rejected.
- `SnapshotConsistentPrefix` — checks lookup-table consistency, not that materialization reads a consistent log prefix.
- `LateEventNoRewrite` — checks key existence, not that late events don't rewrite state.
- `CrashNoAcceptedLost` — copies `PreCrashRecoveredState` into `RecoveredCommandState` during replay rather than deriving from log entries — assumes the answer.
- `IdempotentLogReplay` — checks numeric bounds, not that replay produces identical state.

**`authority.qnt`** (5 properties):
- `NoCommandWithoutGrant = true` — literal placeholder.
- `CompoundIssuer = true` — literal placeholder.
- `GrantAuthorityIsCommandKinds = true` — literal placeholder.
- `RevocationPreventsFuture = always(true)` — literal placeholder.
- `SpawnCreatesDescendantGrant` — demoted above; remove the `val` definition.

### Fix VERIFICATION.md authority-tier contradiction

The `### authority.qnt promotion status` section (lines ~116-133) contradicts itself: it says the four general authority properties "remain draft/stated-normative" but then lists them under "Checked-model spawn properties." Correct the list to include only the genuinely promoted spawn properties (`FleetAuthorityForSpawn`, `SpawnRevocationDoesNotCascade`, `ElicitationResponderAuthority`) and explicitly state the five properties (`NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture`, `SpawnCreatesDescendantGrant`) are stated-normative with no executable formula.

### Verification

Run `node contracts/scripts/check-vectors.mjs` (exits 0, regenerates conformance table), then `node contracts/scripts/check-models.mjs` (exits 1, regenerates model table), then `node contracts/scripts/check-models.mjs` again (exits 0, confirms current).

## Acceptance criteria

- [ ] `SpawnCreatesDescendantGrant` `@promotion` block changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] `SpawnCreatesDescendantGrant` moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] All eleven draft property `val` definitions removed from `snapshot_recovery.qnt` and `authority.qnt`.
- [ ] `@promotion` blocks for the other ten unchanged (status stays draft, invocation stays `<TBD>`).
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; `node contracts/scripts/check-models.mjs` exits 0 on second run.
- [ ] `quint parse specs/seed/snapshot_recovery.qnt` and `quint parse specs/seed/authority.qnt` exit 0.
- [ ] VERIFICATION.md authority-tier contradiction fixed: only the 3 genuinely promoted spawn properties (`FleetAuthorityForSpawn`, `SpawnRevocationDoesNotCascade`, `ElicitationResponderAuthority`) listed as checked-model; the 5 general/descendant properties explicitly stated-normative with no executable formula.


## Review (2026-07-11)

**Verdict**: Approve - fast-lane advance.

Story verified by implement (green `quint parse` + checkers); the cumulative diff across all 8 units was covered by the feature's 6-round deep-review convergence loop, which confirmed the final state (8 promoted / 39 stated-normative, 24 demotions, 24 formulas removed, 8 survivors mutation-confirmed sound).
## Implementation notes

- Files changed: `specs/seed/snapshot_recovery.qnt`, `specs/seed/authority.qnt`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`.
- Removed all eleven misleading executable definitions while preserving their `@promotion` obligations; demoted `SpawnCreatesDescendantGrant` and moved its property id to the stated-normative registry.
- Corrected the authority promotion prose and regenerated both generated traceability tables. The generated model row now classifies `SpawnCreatesDescendantGrant` as `draft` / `stated-normative`.
- Verification: both required `quint parse` commands exited 0; `check-vectors.mjs` exited 0; `check-models.mjs` exited 1 on regeneration and 0 on the confirming second run; `git diff --check` passed.
- Tests added: none; this story's verification surfaces are Quint parsing and the existing traceability checkers.
- Discrepancies from design: none. Dependency readiness followed the caller-confirmed policy that the prerequisite at `stage: review` is satisfied for this chain.
- Dispatch: direct-read only; the bounded, explicitly enumerated edits did not warrant exploratory fan-out.
- Adjacent issues parked: none; remaining stale summary prose is assigned to the already-sequenced follow-on stories in the parent design.
