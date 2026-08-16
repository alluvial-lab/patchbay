---
id: research-handoff-pi-adapter-capability-cursor-replay-resync
kind: story
stage: review
tags: [adapter, protocol, verification]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-control-session-integrity, research-handoff-pi-adapter-capability-rpc-process-supervisor, research-handoff-spawn-logical-target-identity-contract, research-handoff-spawn-cursor-authoritative-replacement-contract, research-handoff-spawn-runtime-evidence-promotion-contract, research-handoff-spawn-reconnect-cursor-reconcile]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-16
---

# Pi-session-scoped cursor replay and authoritative projection replacement

## Redesign disposition

Rewritten to resolve review BLOCKER 6 and the generation-scoped cursor MATERIAL. The old full-fetch upsert and logical-target+generation cursor key are superseded.

## Checkpoint

Implement the shared `AuthoritativeCursorReplacement` contract for Pi persisted entries. Scope the cursor by verified Pi continuity identity so process generation N+1 can load N's cursor when both select the same Pi session. Known suffixes are stable-id idempotent. Unknown cursor holds the old projection stale, validates a complete exact tree, and replaces projection membership + leaf + cursor + epoch; omissions delete stale Pi-derived presentation members.

A claimed successor may stage replacement evidence but cannot publish an ordinary transcript Observation until `SpawnPromotionCommitted` makes it current.

## Design

**Files**
- New `pi-adapter/src/cursor_store.ts` — 0600 versioned store, continuity→logical-target reverse binding, staged/current records, compare-and-swap, temp-file fsync/rename.
- New `pi-adapter/src/entry_reconciler.ts` — known-suffix and unknown-replacement state machine, current-leaf handling, candidate staging, promotion-aware publication, and commit-after-core-ack.
- New `pi-adapter/src/pi_projection.ts` — deterministic exact persisted-entry-to-presentation projection and generated suffix/replacement envelopes.
- `pi-adapter/src/{rpc_client,spawn_supervisor,main,core_client,transcript_projection}.ts` — typed unknown-cursor failure, current/claimed fence integration, and no live-stream authority.
- `contracts/proto/patchbay/pi_adapter.proto` — generated `PiPersistedProjectionSuffix` / `PiPersistedProjectionReplacement` with continuity id, epoch, exact stable items, cursor, leaf, and tree digest.
- `operator-domain/src/reconciliation/external_cursor.ts` — implement/consume the shared spawn leaf; known Pi compositor and `web-cockpit/src/domain/model.ts` fold replace one continuity scope atomically while retaining immutable audit history.
- Focused store/replay/presentation tests and `cursor-gap-repair` / upsert-mutation vectors.

```ts
export interface PiSessionContinuityKey {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly piSessionId: string;
  readonly configuredSessionRootId: string;
  readonly rootRelativePath: string;
}

export interface PiProjectionRecord {
  readonly scope: ExternalCursorScope;
  readonly logicalTargetId: string;
  readonly epoch: bigint;
  readonly exactEntries: ReadonlyMap<string, PiProjectedEntry>;
  readonly cursorEntryId: string | null;
  readonly leafId: string | null;
  readonly treeDigest: string;
  readonly state: "staged" | "current";
}

export interface PiEntryCursorStore {
  load(scope: ExternalCursorScope): Promise<PiProjectionRecord | undefined>;
  bindLogicalTarget(scope: ExternalCursorScope, logicalTargetId: string): Promise<void>;
  stageReplacement(expectedEpoch: bigint | undefined, replacement: PiProjectionRecord): Promise<void>;
  commitAfterCoreAck(expectedEpoch: bigint | undefined, replacement: PiProjectionRecord): Promise<void>;
}
```

`ExternalCursorScope.externalContinuityId` is a bounded digest of length-framed adapter id + deployment scope + verified Pi session id + stable configured-session-root id + canonical root-relative path. The configured-root id is derived from the canonical configured root, never from Pi's tree-root entry id; raw paths remain local. Patchbay generation is not an input. Consumers additionally key state by the authenticated adapter/deployment target plus the digest so a forced digest collision cannot merge scopes. A second logical target binding for the same continuity id rejects before projection/report.

Known cursor:

1. call `get_entries(since)` and require success;
2. validate every returned entry/parent relation against stored exact state and the returned current leaf;
3. build a deterministic suffix batch id from scope/base cursor/result/tree digest;
4. submit current-generation opaque Pi suffix Observation and await durable core acknowledgement;
5. atomically CAS-commit exact state/cursor/leaf. Response loss retries the same batch; stable item ids/content make it inert.

Unknown cursor:

1. mark/retain old projection stale; do not clear it;
2. call full `get_entries`, validate raw file/RPC exact set and complete tree through `PiSessionTreeValidator`;
3. derive exact projected membership and next epoch; compare exact ids/content to the previous record for diagnostics and removed-member evidence;
4. atomically persist a **staged** replacement record without making it current;
5. if the runtime disposition is `Current`, send one replacement envelope and await its durable event id; if `ClaimedSuccessor`, put its digest in staged successor evidence and wait for promotion;
6. the consumer folds replacement as one operation, deleting every prior Pi-persisted member in scope omitted from the exact set;
7. after current-generation core acknowledgement, CAS/rename the local current record containing projection+leaf+cursor+epoch together.

Retrying identical `(scope,epoch)` is a no-op. Conflicting content for the same epoch, cross-session identity, stale generation, or wrong logical-target reverse binding fails closed. A crash before the replacement event leaves the prior state stale. A crash after event acknowledgement but before local commit resends the identical epoch then commits; cursor never leads the projection.

Live `entry_appended` only wakes reconciliation. Parallel tool updates remain transient partial-order notifications; finalized persisted entries repair them. Control-extension custom entries participate in tree/cursor identity but do not become transcript presentation members.

For `memory_only` sessions, the reconciler publishes a distinct non-authoritative, last-observation-wins volatile presentation snapshot with no replacement epoch or restart-stable cursor claim. When the first assistant materializes the file, it performs a full authoritative replacement from epoch one before enabling restart-stable cursor capability.

## Acceptance evidence

- [x] N+1 resumed against the same verified Pi session loads N's cursor without transfer; a different Pi id/path/root or second logical target rejects.
- [x] Known suffix commits only after core acknowledgement and is inert under acknowledgement-loss retry.
- [x] Unknown cursor cannot return empty/current, clear first, or upsert over the old projection.
- [x] A full set omitting previously projected entry X removes X in the consuming current projection after one replacement fold; durable source/audit history remains.
- [x] Projection, cursor, leaf, and epoch become current together; cursor-before-projection and partial-file crash mutations fail.
- [x] Claimed successor replacement stays staged and emits no normal transcript until promotion; post-promotion publication precedes `live` report.
- [x] Pre-compaction and abandoned-branch entries validate and remain in the exact tree; current leaf does not imply live-process order.
- [x] Memory-only state is not advertised as restart-stable; materialization triggers exact replacement.
- [x] Upsert-only, generation-keyed, clear-before-fetch, same-epoch-conflict, and missing reverse-binding mutations fail.

## Ordering constraint

Consumes the logical identity, authoritative cursor replacement, runtime evidence/promotion, and reconnect contracts plus the concrete control/session proof and RPC supervisor. It is the sole Pi implementation of the shared cursor leaf.

## Implementation notes

- Execution capability: inline cohesive implementation, grounded in the completed Leaf-4 `AuthoritativeCursorReplacement` contract and the existing spawn supervisor.
- Added generated Pi suffix/replacement protobuf envelopes, a path-opaque Pi continuity derivation, a private temp-fsync-rename cursor store with reverse logical-target binding and exported CAS conformance instrumentation, deterministic persisted-entry projection, and the Pi Leaf-4 port adapter.
- Production spawn now stages cursor evidence under the exact claim, journals a recovery capsule, publishes only after exact promotion, and commits the local cursor only after durable core acknowledgement. Recovered promotions resend the same deterministic envelope. Current managed sessions use persisted-entry notifications only as reconciliation wakes.
- Added the operator-domain Pi compositor and cockpit fold. Exact replacement deletes omitted Pi projection memberships while leaving immutable source events untouched; duplicate suffix/replacement delivery is inert and same-epoch conflicts fail closed.
- Added/extended the `spawn-reconnect-cursor-convergence` conformance vector with Pi adapter and Pi presentation runners. Generated envelopes contain stable ids, hashes, opaque continuity, and transcript presentation facts but no raw local path, session label, or custom-entry payload.
- Acceptance tests cover same-Pi N→N+1 continuity, cross-Pi/reverse-binding rejection, known and unknown cursor paths, response loss, crash after core acknowledgement before CAS, memory-only materialization, abandoned branches/pre-compaction membership, publication gating, exact omission deletion, and POSIX-private storage.

## Verification

- `contracts/ts: npm run build` — passed.
- `contracts/ts: npm run check:vectors` — passed (59 vectors, 31 implementation checks, 38 mutation witnesses).
- `cargo test -p patchbay-contracts` — passed.
- `operator-domain: npm test` — passed (30/30).
- `pi-adapter: npm test` — passed (108/108).
- `pi-adapter: npm run test:mutations` — passed (18/18 killed, including Pi continuity collapse, skipped exact replacement publication, and acknowledged-without-write CAS mutants).
- `web-cockpit: npm test` — passed (145/145).
- `cli: npm test` — passed (53/53 plus real-core resource projection).
- `token-commune-adapter: npm test` — passed (63/63).

### Fix round r1 — 2026-08-16

- Execution capability: `openai-codex/gpt-5.6-sol`, inline cohesive fix ownership across the generated Pi envelope, adapter reconciler, operator-domain compositor, and cockpit projection. Direct-read only; no subagent was needed or invoked.
- Review weight: `thorough`, supplied by the active autopilot caller; a fresh independent re-review follows this transition.
- Known-suffix rebinding: the cockpit now keys each Pi membership by the full consuming scope and atomically rebinds every retained membership in that scope to the suffix Observation's verified runtime target before adding the suffix. The production fold + `renderSessionDetail` oracle proves replacement at N followed by a known suffix at N+1 renders the complete old+new transcript only under N+1.
- Canonical scope digest: the producer digest now length-frames adapter id, deployment scope, verified Pi session id, an opaque stable identity derived from the canonical configured session root, and canonical root-relative path. Pi's tree-root entry id is no longer a continuity input. The consumer key independently length-frames authenticated adapter/deployment target plus opaque digest, so forced cross-target digest collisions stay separate. One-dimension tests cover adapter, deployment, Pi session, configured root, and relative path, including copied session/tree ids under two roots.
- Volatile-path separation: added generated `PiVolatileProjectionSnapshot`, a non-authoritative last-observation-wins envelope with no epoch. Removed the in-memory epoch map, durable replacement encoding, and empty-memory special mode; even empty volatile snapshots can clear stale volatile presentation. The staged journal records `replacementEpoch: null`, and first materialization forces a complete authoritative epoch-one replacement. Adapter-restart → changed volatile snapshot → materialization is replayed through the production operator consumer and cursor store.
- Publication-before-live: successor staging carries literal `stale/unknown` presentation evidence; readiness remains the separate staged digest. Only the report after projection acknowledgement, local cursor CAS, and publication-journal commit may say `live`. Crash-window tests at pre-envelope and post-envelope/pre-CAS prove no live report, while cockpit promotion remains stale through projection until the later live state event.
- NIT applied: the real-process handshake test alone supplies a 10-second RPC response timeout while retaining its 30-second test bound; production defaults remain unchanged.
- Files changed: `contracts/proto/patchbay/pi_adapter.proto` plus generated Rust/TypeScript bindings; `operator-domain/src/pi.ts` and Pi tests; `pi-adapter/src/{cursor_store,entry_reconciler,main,pi_process,pi_projection,spawn_supervisor}.ts`, mutation registry, and focused cursor/reconciler/supervisor/real-process tests; `web-cockpit/src/domain/model.ts` and model/session-detail tests; this story.
- Tests added/strengthened: suffix N→N+1 session-detail ownership; derivation and consuming-fold scope collisions; adapter-process volatile restart and later materialization replay; volatile-to-persisted cockpit replacement; promotion-before-envelope and promotion-before-CAS windows; successful cursor→journal→live order.
- Mutation evidence: manually re-injected and killed (1) retained-membership non-rebinding, (2a) omitted producer digest dimensions/configured root, (2b) digest-only consumer keying, (3) volatile state encoded as epoch-one persisted replacement, and (4) live successor staging. Each focused oracle failed, and `git restore --worktree` restored the staged implementation after every probe. The registered mutation cycle remained **18/18 killed** and rebuilt the clean sources.
- Full verification: Rust group `cargo fmt --all -- --check && cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — passed. Contracts group `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build && npm run check:presentation && npm run test:presentation` — passed (59 vectors, 19 promoted, 31 implementation checks, 38 mutation witnesses). Operator-domain group `cd operator-domain && npm test` — passed (32/32). Pi-adapter group `cd pi-adapter && npm test && npm run test:mutations` — passed (109/109, 18/18 mutations). `web-cockpit: npm test` — passed (148/148). `cli: npm test` — passed (53/53 plus real-core resource projection). `token-commune-adapter: npm test` — passed (63/63).
- Simplification: one scope object now owns target dimensions and opaque continuity at the consumer; volatile state no longer impersonates the durable replacement state machine or carries a recoverable-epoch fiction.
- Discrepancies from design: the original memory-only paragraph permitted a process-local volatile exact set but did not define its wire semantics; r1 resolves the review-discovered ambiguity with a generated non-authoritative snapshot rather than persisting an epoch-only anchor. Configured-root identity is derived from canonical configured-root realpath because no separate configured root id exists in current target configuration; this is restart-stable, distinguishes copied roots, remains path-opaque, and avoids widening target configuration.
- Adjacent issues parked: none.
