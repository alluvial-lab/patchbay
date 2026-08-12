---
id: research-handoff-spawn-cursor-authoritative-replacement-contract
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-identity-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# External cursor authoritative-replacement contract

## Checkpoint

Define the spawn-side adapter-neutral cursor contract consumed by the Pi redesign. External persisted-state cursors are scoped by verified external continuity identity, not by Patchbay generation. A known cursor may apply a suffix. An unknown cursor requires a staged exact-set/tree rebuild and atomic replacement of projection + leaf + cursor + epoch.

A full fetch cannot be applied as upserts over an old projection: omissions must remove stale projected entries before the new cursor becomes authoritative.

## Design

**Files**
- `contracts/proto/patchbay/adapter.proto` — adapter-neutral cursor replacement capability/epoch shape where wire carriage is required.
- New `operator-domain/src/reconciliation/external_cursor.ts` — generated-contract-consuming state-machine interface shared by adapter profiles without making the generated-artifact package own domain logic.
- Contract tests for exact-set replacement, crash prefixes, and cursor scoping.

```ts
export interface ExternalCursorScope {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly externalContinuityId: string;
}

export interface ProjectionReplacement<Entry, Cursor, Leaf> {
  readonly replacementEpoch: bigint;
  readonly exactEntries: readonly Entry[];
  readonly cursor: Cursor;
  readonly leaf: Leaf;
}
```

The following Pi redesign defines `externalContinuityId` from verified Pi session identity and implements storage/reconciliation. This leaf does not import Pi session paths, `get_entries`, or JSONL into core ontology.

## Acceptance evidence

- [ ] Cursor scope survives Patchbay generation replacement when verified external continuity remains the same.
- [ ] Known-cursor suffix applies idempotently without replacing unrelated state.
- [ ] Unknown cursor keeps the old projection stale while a replacement is staged.
- [ ] Atomic replacement removes entries absent from the authoritative exact set/tree and installs cursor/leaf/epoch together.
- [ ] Crash before commit preserves old stale state/cursor; crash after commit exposes only the complete replacement.
- [ ] Upsert-only, clear-before-fetch, cursor-before-projection, and generation-keyed-continuity mutations fail.

## Ordering constraint

Independent early leaf after logical identity. Every spawn reconnect operation and the Pi cursor redesign consume it; this story owns no Pi implementation.
