---
id: reconnect-cursor-reconcile-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-reconnect-cursor-reconcile
created: 2026-08-15
updated: 2026-08-15
---

# Thorough review — Unit 10 reconnect and cursor convergence

## Verdict

**MATERIAL** — return `research-handoff-spawn-reconnect-cursor-reconcile` to `implementing`.

Core promotion/replay, the sealed Leaf-4 external-cursor consumer, exact omission replacement, detach-does-not-retire, staged non-publication, and same-lineage newer-snapshot dominance all passed focused inspection and mutation. The surface continuity fence is incomplete, however: the normal stream-first cockpit has no core-generation anchor, and its first reconnect snapshot may silently adopt another storage lineage with the same authority-domain id.

Review mode: independent fresh-context story review, effective weight `thorough`, one rigorous pass over `3518197..6b8ddcd`.

## Findings

### MATERIAL — stream-first cached state can adopt a different core continuity epoch

**Locations:** `web-cockpit/src/domain/model.ts:302-314,1959-1974`; `web-cockpit/src/domain/reconcile.ts:84-104`; `web-cockpit/src/main.ts:361-380`

`fold` records an authority domain and LSN for streamed state but cannot record `coreGeneration`. `assertNewerCoreAuthority` checks generation equality only when the cached model already has a generation. The normal cockpit startup loads only the separate security snapshot and then starts `subscribe`; it does not establish a session/resource snapshot baseline first. A model populated by that normal stream path therefore remains continuity-unanchored.

A focused reviewer probe folded a same-domain event at LSN 3, marked the model unreconciled, and supplied same-domain session/resource snapshots at LSN 9 from core generation 99. Production accepted the replacement and installed `{cursor: 9, coreGeneration: 99}`. Thus a destructive store replacement or endpoint switch can replace cached logical/current state without proving same core continuity; the replacement may resurrect a different lineage even though its bare LSN is higher. Clean finite-tail resubscriptions have the same blind spot because stream events carry no continuity epoch.

**Concrete fix:** establish and retain a session/resource core-generation anchor before streamed state becomes authoritative, and bind every subscription/resumption to that anchor. Either carry `core_generation` in the subscription establishment/event envelope or perform a generation-checked snapshot handshake before each resumed stream, including clean-tail reconnects. Once a model has a domain/cursor, an absent cached generation must not authorize adoption of an arbitrary epoch. Add a regression that builds state from a stream prefix, presents a higher-LSN snapshot/stream from another generation, and proves the old model remains unreconciled and unchanged.

## Checklist disposition

- **Core replay determinism:** pass. The mixed continuation prefix reconstructs the poisoned claim/fence and keeps staged N+1 non-live; promotion replay reconstructs N tombstone, N+1 ownership, claim consumption, completion, and descendant authority. The exhaustive quarantine-family oracle remains outer-only across normal hot/replay folds.
- **Adapter cursor authority:** pass. The Unit-10 runner consumes `AuthoritativeCursorReplacement.create`, whose private constructor/brand prevents a structural or subclass reimplementation. Leaf-4 tests cover known suffix retry, stale staging, exact omission deletion, one-CAS projection/leaf/cursor/epoch installation, crash prefixes, scope continuity across Patchbay generations, and cross-native rejection.
- **Surface authority:** partial. Same-domain, same-generation, strictly newer snapshots exactly replace cached N; equal-LSN streams/snapshots are inert. Cross-lineage continuity fails as the material finding above.
- **Authority separation:** pass. No core-LSN/external-cursor translation was introduced. Existing current-token and rebuilt-core reattachment tests authenticate attachment generation before accepting current evidence.
- **Missing-stream reconciliation:** pass. Pre-promotion replay leaves N current/fenced and N+1 staged/non-live; promotion alone tombstones N and publishes authorized N+1. The surface exact replacement removes N rather than upserting N+1 beside it.
- **Scope boundary:** accepted as designed. The concrete Pi durable cursor store and cross-process atomicity evidence remain downstream; the new vector remains draft rather than claiming formal promotion.

## Mutation matrix

Every source mutant was applied alone on the main tree, run with focused tests, restored with `git restore`, and followed by a clean status check. No temporary worktree was created.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Accept an equal-LSN remembered stream (`<=` → `<`) | web newer-core snapshot convergence test | **KILLED**; cached N was reinserted beside N+1. |
| Accept an equal-LSN snapshot (`<=` → `<`) | web newer-core snapshot convergence test | **KILLED**; the required stale/equal rejection disappeared. |
| Install new cursor/leaf/epoch while retaining the old exact projection | Leaf-4 atomic replacement test and Unit-10 operator-domain vector | **KILLED by both**; omitted stale membership remained visible. |
| Install a transient cursor-first record, then the complete replacement | Leaf-4 one-write atomicity test; Unit-10 vector also observed | **KILLED by Leaf-4** (two writes observed). The Unit-10 vector alone stayed green, as expected: atomicity is consumed from the sealed Leaf-4 contract rather than reimplemented in the cross-layer runner. |
| Target detach degradation at generation N+1 instead of retaining N | server abnormal-stream-drop test | **KILLED**; replay rejected the fabricated successor generation and could not mark N stale. |
| Stream-derived model with no epoch receives same-domain generation-99 snapshot | direct production-seam probe | **SURVIVED**; snapshot was accepted at LSN 9, producing the material finding. |

## Full clean-tree suite

Run after all mutations were restored:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — includes 35 runtime-evidence/promotion tests, 82 server unit tests, all workspace tests/doctests, and warnings-denied clippy.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 57 vectors, 17 promoted, 26 implementation checks, 38 mutation witnesses; generated bindings clean.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 27/27.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including real-core reconnect/restart E2E.
5. `cd web-cockpit && npm test`: **PASS** — 133/133.
6. `cd cli && npm test`: **PASS** — 48/48 plus real-core resource projection.

The tracked tree was clean before mutation work, after every restore, before and after the final full suite, and before this review file was written. `git diff --check` passed. `/` retained 57G free.

## Recommendation

**Return to implementing.** Make surface stream/snapshot continuity generation-bound, add the cross-lineage stream-first regression, rerun the clean-tree suite, and submit a new thorough review pass.
