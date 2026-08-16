---
id: pi-cursor-replay-rereview-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-cursor-replay-resync
created: 2026-08-16
updated: 2026-08-16
---

# Thorough re-review — Pi Unit 4 cursor replay/resync r1

## Verdict

**CLEAN.** Commit `e00c729` closes all four pass-1 MATERIAL findings. The complete N→N+1 and fresh N→N+1→N+2 transcript oracles pass; all continuity dimensions remain independent through derivation and the authenticated wire consumer; volatile snapshots are epoch-free and materialize through authoritative epoch one; and successor presentation remains stale/unknown until projection publication, local cursor CAS, and journal publication commit precede the live report. No material finding, stale-member survival, collision, pre-publication visibility, or nit remains.

Review mode: independent fresh-context thorough convergence pass 2 over `5a2e656..e00c729`, grounded in the pass-1 review, story notes, and feature design sections 4–5. No temp worktree or subagent was used. Temporary probes and mutants were restored with `git restore`; the tracked tree was clean before this review file was written.

## Findings

None.

## Closure evidence

### 1. Suffix rebinding — closed

`web-cockpit/src/domain/model.ts` now keys Pi presentation membership by the full consuming scope and retargets all retained members in that scope before adding suffix members. The committed replacement-at-N → suffix-at-N+1 test renders the complete old+new transcript under N+1 and none under tombstoned N.

A temporary fresh probe extended the production fold through a second known suffix at N+2. All three turns rebound to generation 3; session detail contained all three only under N+2 and contained none under N or N+1. Removing the retained-membership rebinding block made that oracle fail with the exact 1/2/3 generation split, so the oracle is non-vacuous.

### 2. Canonical scope digest — closed

`derivePiSessionContinuityKey` length-frames adapter id, deployment scope, verified Pi session id, opaque configured-root identity, and canonical root-relative path. Its public input no longer accepts Pi's tree-root entry id. The configured-root identity derives from the canonical configured root and remains path-opaque on the wire. The consumer independently length-frames the authenticated adapter/deployment target plus the opaque digest.

The committed derivation test varies each of the five dimensions independently. Consumer tests force the same digest across adapter and deployment targets. A temporary end-to-end probe derived all five one-dimension variants, encoded generated replacement envelopes, and folded them through `piProjectionObservationScope` plus the production persisted consumer; all six states remained independent. A second wire probe used independent reconciler instances and changed Pi's tree-root entry id while keeping configured continuity fixed; decoded generated volatile envelopes retained the same `external_continuity_id`, and later materialization still entered epoch one.

Omitting configured-root identity from the producer digest was killed by the derivation oracle. Omitting authenticated adapter/deployment from the consumer key was killed as a same-epoch collision.

### 3. Volatile snapshots — closed

`PiVolatileProjectionSnapshot` is a distinct generated schema with no replacement epoch. The reconciler has no process-local volatile epoch map and publishes volatile state only through the non-authoritative envelope. Replayed volatile observations are last-observation-wins; first materialization initializes an empty durable baseline and forces a complete authoritative replacement at epoch one.

The adapter-restart test replays changed volatile content through the production operator consumer, verifies no cursor-store directory exists before materialization, then verifies generated authoritative replacement epoch one and a current local cursor record. The cockpit removes volatile presentation membership when the persisted replacement arrives. A temporary fresh assertion confirmed `foldPiPersistedProjectionObservation` returns `undefined` for the volatile schema, so a volatile snapshot cannot participate in or trigger an epoch conflict. Relabeling the volatile encoder as the persisted replacement schema was killed immediately.

### 4. Publication before live — closed

Successor staging now carries literal `stale` / `unknown` presentation evidence. The active path orders exact core promotion → projection envelope acknowledgement and local cursor CAS → journal `publicationCommitted` → live report. On publication failure it explicitly reports stale/unknown and never reports live.

Committed crash-window tests cover failure before the envelope and after envelope acknowledgement but before local CAS. Cockpit promotion remains non-live after promotion and after projection alone, becoming live only on the later connectivity event. A temporary recovery probe seeded a publication journal marker without a cursor commit and verified startup recovery reported only `recovered-session:stale:unknown`, with no live event. Mutating successor staging back to live was killed by both the ordered success oracle and both crash windows.

### 5. Unknown-cursor and regression contract — closed

The shared seven-step unknown-cursor path still marks and retains old state stale, fetches and validates the complete exact set, stages the next epoch, publishes one complete replacement, removes omissions in one consumer fold, and atomically commits projection + cursor + leaf + epoch after acknowledgement. Crash tests expose only old-stale or complete-new; cursor never leads projection.

The registered mutation cycle killed **18/18**. Explicit prior-kill spot checks remained killed:

- skipped exact replacement publication — killed by `unknown cursor stages old projection stale then one exact replacement removes omission`;
- acknowledged cursor CAS without durable write — killed by the reusable overlapping-reader/CAS conformance oracle.

## Mutation and fresh-probe matrix

| Mutation / probe | Focused oracle | Result |
|---|---|---|
| N replacement → N+1 suffix | Production fold + session detail | **PASS** — complete transcript only under N+1. |
| N replacement → N+1 suffix → N+2 suffix | Temporary production-fold/session-detail probe | **PASS** — complete transcript only under N+2; none under N/N+1. |
| Remove retained-membership rebinding | Fresh N→N+1→N+2 oracle | **KILLED** — turns remained split across generations 1/2/3. |
| Vary adapter, deployment, Pi session, configured root, and relative path | Derivation plus generated-wire production-consumer probe | **PASS** — six independent consuming scopes. |
| Independent deployments with changed Pi tree-root id | Decoded generated wire envelopes | **PASS** — same configured continuity digest; later epoch-one materialization succeeded. |
| Omit configured-root identity from producer digest | Canonical scope-dimension test | **KILLED** — configured roots collided. |
| Key consumer by digest alone | Forced target-collision consumer test | **KILLED** — same-epoch conflict replaced three independent scopes. |
| Adapter restart → changed volatile snapshot → materialization | Replayed operator consumer + cursor store | **PASS** — last snapshot wins, no epoch conflict, durable epoch one. |
| Feed volatile envelope to persisted epoch consumer | Temporary direct fold probe | **PASS** — not consumed (`undefined`). |
| Relabel volatile envelope as persisted replacement | Volatile restart/materialization oracle | **KILLED**. |
| Promotion before envelope / before local CAS | Spawn crash-window tests | **PASS** — stale/unknown only, no live report. |
| Journal marker without cursor commit | Temporary recovery probe | **PASS** — recovered stale/unknown, never live. |
| Stage successor as live | Ordered success + crash-window tests | **KILLED**. |
| Skip unknown exact replacement publication | Registered mutation #2 | **KILLED**. |
| Acknowledge CAS without durable write | Registered mutation #3 | **KILLED**. |
| Complete registered Pi mutation cycle | `npm run test:mutations` | **18/18 KILLED**; sources restored. |

## Full clean verification

1. **Rust group — PASS:** `cargo fmt --all -- --check`; workspace all-target build; workspace tests; warnings-denied all-target clippy.
2. **Contracts group — PASS:** generated drift, 59 vectors / 19 promoted / 31 implementation checks / 38 mutation witnesses, models, TypeScript build, presentation conformance, and presentation meta-tests.
3. **Operator-domain group — PASS:** 32/32.
4. **Pi-adapter group — PASS:** 109/109 plus 18/18 registered mutations.
5. **Web cockpit — PASS:** 148/148.
6. **CLI — PASS:** 53/53 plus real-core resource projection.
7. **token-commune adapter — PASS:** 63/63.
8. **Hygiene — PASS:** `git diff --check`; clean tracked tree before this review file; `/` retained 53G free.

One initial focused attempt incorrectly built the operator domain and its dependents concurrently; the operator build intentionally removes its own `dist`, so dependent compilers briefly lacked that artifact. The same focused groups were rerun sequentially and passed, and the clean full suite above was also sequential.

## Recommendation

**Approve the r1 fix and close the convergence loop.** The receiving autopilot agent may advance `research-handoff-pi-adapter-capability-cursor-replay-resync` to `done` and continue the parent feature.
