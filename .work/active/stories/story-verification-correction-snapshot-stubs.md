---
id: story-verification-correction-snapshot-stubs
kind: story
stage: implementing
tags: [verification]
parent: epic-public-product-contract-verification-claim-correction
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Replace misleading snapshot_recovery.qnt draft formulas with honest stubs

## Scope

The six `snapshot_recovery.qnt` properties are already `status: draft`, so no tier change is needed. But their formulas mislead: they look like real (if weak) checks but don't model the claimed behavior. Replace each formula with `true` plus an honest comment so no one mistakes the draft for a behavioral check. The real crash/replay/snapshot convergence modeling is v1 gate work owned by `epic-public-product-contract-executable-release-assurance`.

## Unit

`Unit 3` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/snapshot_recovery.qnt` — six draft property `val` definitions

## Implementation

Replace each of the six draft property formulas with `true` plus a comment. Do NOT change `@promotion` status (stays draft), invocation (stays `<TBD>`), or the model's actions/state — only the property `val` definitions.

The six properties and why their current formulas mislead:

- `SnapshotStaleRejected` — checks `SnapshotRevision >= Cursor and SnapshotRevision >= SnapshotMaterializedLSN` (non-decreasing revision), not that stale snapshots are rejected as authority sources.
- `SnapshotCrossDomainRejected` — checks `SnapshotAuthorityDomain == AuthorityDomain and SnapshotCoreGeneration == CoreGeneration` (current snapshot origin matches core), not that cross-domain snapshots are rejected.
- `SnapshotConsistentPrefix` — checks `SnapshotMaterializedEvents == SNAPSHOT_PREFIX_EVENTS.get(SnapshotMaterializedLSN)` (lookup table consistency), not that materialization reads a consistent log prefix.
- `LateEventNoRewrite` — checks `RecoveredCommandState.keys().contains(cmd)` (key existence), not that late events don't rewrite state.
- `CrashNoAcceptedLost` — checks `PreCrashRecoveredState.get(cmd) == "accepted" implies RecoveredCommandState.get(cmd) == "accepted"`, but replay copies `PreCrashRecoveredState` into `RecoveredCommandState` rather than deriving from log entries — it assumes the answer.
- `IdempotentLogReplay` — checks `CommittedPrefixLSN >= 0 and Cursor <= CommittedPrefixLSN and SnapshotMaterializedLSN <= CommittedPrefixLSN` (numeric bounds), not that replay produces identical state.

Replacement pattern for each:

```quint
// formula deferred to promotion; current placeholder is not a behavioral check
val SnapshotStaleRejected = true
```

## Acceptance criteria

- [ ] All six draft property formulas in `snapshot_recovery.qnt` replaced with `true` + deferred-to-promotion comment.
- [ ] `@promotion` blocks unchanged (status stays draft, invocation stays `<TBD>`).
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (no metadata change).
- [ ] `quint parse specs/seed/snapshot_recovery.qnt` exits 0 (model still parses).
- [ ] No VERIFICATION.md change required (generated tables derive from `@promotion` metadata, which is unchanged).
