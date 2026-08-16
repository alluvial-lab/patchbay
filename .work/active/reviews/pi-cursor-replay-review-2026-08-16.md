---
id: pi-cursor-replay-review-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-cursor-replay-resync
created: 2026-08-16
updated: 2026-08-16
---

# Thorough review — Pi Unit 4 authoritative cursor replacement

## Verdict

**MATERIAL.** The exact unknown-cursor replacement itself removes omissions, the local file store passes the Leaf-4 CAS/overlapping-reader suite, publication follows core acknowledgement, candidate envelopes remain unpublished before promotion, same-epoch conflicts fail closed, and the generated payload probe found no raw path, session label, or custom-entry payload leakage. Four integration gaps still prevent advancing the story: known N→N+1 suffixes split one continued transcript across runtime generations; the wire consumer drops required continuity-scope dimensions; memory-only publication writes a durable epoch that cannot survive adapter restart; and promotion exposes a live successor before its exact projection is published and locally committed.

Review mode: independent fresh-context adversarial story review, effective weight `thorough`, implementation range `40c1d77..635e55c`. No temp worktree or subagent was used. All temporary mutations were restored with `git restore`; tracked status was clean before the review file was written.

## Findings

### MATERIAL — Known N→N+1 suffix leaves pre-cursor transcript members bound to tombstoned N

**Anchors:** `operator-domain/src/pi.ts:152-157`, `web-cockpit/src/domain/model.ts:1051-1083`, `web-cockpit/src/ui/session-detail.ts:214`.

A known suffix returns no removed memberships and only adds presentation items from `suffixEntries`. The cockpit therefore never retargets already-projected memberships to the Observation's current runtime generation. Session detail filters Observations by the full runtime identity. A direct production-fold probe applied a generation-1 replacement followed by a generation-2 known suffix for the same continuity id and observed:

```text
user-1      -> runtime-n,  generation 1
assistant-1 -> runtime-n1, generation 2
```

The current N+1 session consequently loses all transcript history before N's cursor, while the old items remain attached to tombstoned N. This defeats the promised generation-stable Pi continuity on the normal known-cursor path.

**Concrete fix:** make a continuity-preserving suffix atomically rebind all retained projected memberships to the suffix Observation's verified current runtime target, or model Pi transcript membership against the managed logical/continuity target rather than the runtime generation. Add an end-to-end replacement-at-N → known-suffix-at-N+1 → session-detail oracle requiring the complete old+new transcript under N+1 and none under tombstoned N.

### MATERIAL — The wire projection key omits required continuity-scope dimensions

**Anchors:** `pi-adapter/src/cursor_store.ts:137-153`, `operator-domain/src/pi.ts:27-35`, `web-cockpit/src/domain/model.ts:1051-1058`.

The local `ExternalCursorScope` includes adapter and deployment, but `externalContinuityId` hashes only Pi session id, Pi tree-root entry id, and root-relative path. The generated envelope carries only that digest, and the cockpit indexes projection state by the digest alone. Two direct probes found:

- different adapter/deployment targets with the same digest collide in one consumer state and produce `Pi same-epoch replacement content conflicts` rather than independent scopes;
- two distinct configured session roots containing the same relative path and copied Pi session/root ids derive the identical `externalContinuityId`.

The latter shows that the implementation uses the Pi tree root id where the design requires a stable configured session-root identity. This permits continuity confusion without exposing a raw path.

**Concrete fix:** define one canonical opaque wire scope over adapter id, deployment scope, verified Pi session id, an explicit stable configured-session-root id, and canonical root-relative path. Alternatively, key the consumer by a length-framed adapter/deployment plus opaque Pi digest, but the configured-root dimension must still enter the digest. Add independent one-dimension collision tests at both derivation and consuming-fold boundaries.

### MATERIAL — Memory-only “volatile” publication creates unrecoverable durable epochs

**Anchors:** `pi-adapter/src/entry_reconciler.ts:211-228`, `pi-adapter/src/entry_reconciler.ts:355-374`.

`restartStable: false` exists only in adapter-local staged evidence. `volatile-replacement` is nevertheless encoded as the same durable `PiPersistedProjectionReplacement` Observation consumed as authoritative state. Its epoch comes from an in-memory map and resets to 1 in a new adapter process. A two-process probe published different memory-only exact sets for the same continuity scope: both used epoch 1 and the durable consumer rejected the second with `Pi same-epoch replacement content conflicts`.

The current test proves only that no cursor-store directory is created; it does not prove restart-safe consumer behavior. The wire cannot distinguish the allegedly volatile claim from a restart-stable replacement.

**Concrete fix:** do not publish a durable authoritative-replacement epoch from state whose epoch cannot be recovered. Use a distinct non-authoritative volatile presentation path, or persist/recover an epoch-only publication anchor without claiming a restart-stable cursor, then force the designed full replacement on materialization. Add adapter-process-restart and later-materialization tests against a replayed consumer.

### MATERIAL — `SpawnPromotionCommitted` can publish a live successor before projection commit

**Anchors:** `pi-adapter/src/main.ts:221-230`, `pi-adapter/src/spawn_supervisor.ts:711-724`, `web-cockpit/src/domain/model.ts:935-951`.

The staged successor SessionReport is sent with `connectivity = LIVE`. Core promotion publishes that report and the cockpit renders its connectivity immediately. Only after promotion does the supervisor publish the exact Pi envelope, commit the local cursor record, and mark journal publication committed. A crash or publication failure therefore leaves a window where N+1 is current and visibly live while its exact transcript projection is absent/stale; the later catch can only downgrade after the failure is observed.

This contradicts the feature's required order: promotion, projection publication/local commit, then fresh live report.

**Concrete fix:** stage the successor as stale/unknown for presentation (while retaining separate readiness evidence), and allow only the post-publication `reportSessionState(..., "live", ...)` to establish live connectivity. Add crash-window tests for promotion-before-envelope and promotion-before-local-CAS proving the successor never renders live until the publication journal marker and cursor record agree.

### NIT — Real-process handshake test remains close to its timeout

The first clean `pi-adapter` full run failed only `real offline pi --mode rpc child handshakes...` at 2.061s with the control-response timeout. The focused rerun passed, and the complete clean rerun passed 108/108. Consider widening or isolating the test's startup timeout to reduce non-product full-suite flakes.

## Mutation and fresh-probe matrix

| Mutation / probe | Focused oracle | Result |
|---|---|---|
| Upsert-only replacement (`removedMembershipIds = []`) | Pi compositor omission test | **KILLED** — omitted membership remained. |
| Cursor/local CAS before replacement publication | Temporary acknowledgement-loss assertion on unknown replacement | **KILLED** — store became `current` where the oracle required retained `stale`. |
| Candidate replacement published inside staging | Unknown-cursor claimed-successor test | **KILLED** — publication count became 1 before promotion. |
| Same-epoch conflicting replacement treated as inert | Pi compositor same-epoch conflict test | **KILLED** — expected exception disappeared. |
| Distinct configured roots, otherwise identical Pi continuity inputs | Direct `derivePiSessionContinuityKey` probe | **FOUND MATERIAL** — opaque ids collided. |
| Same opaque id under different adapter/deployment targets | Direct operator-domain fold probe | **FOUND MATERIAL** — independent scopes collided as a same-epoch conflict. |
| Generation-1 replacement then generation-2 known suffix | Direct cockpit production-fold probe | **FOUND MATERIAL** — old/new messages remained split across N/N+1. |
| Two memory-only adapter processes with changed exact content | Direct reconciler + replay consumer probe | **FOUND MATERIAL** — both emitted epoch 1; second conflicted. |
| Registered Pi mutation cycle | `npm run test:mutations` | **18/18 KILLED** and sources restored. |

## Full clean verification

1. **Rust group — PASS:** `cargo fmt --all -- --check`; workspace all-target build; workspace tests; warnings-denied all-target clippy.
2. **Contracts group — PASS:** generated drift, vectors, models, TypeScript build, presentation conformance, and presentation meta-tests. Summary: 59 vectors, 19 promoted, 31 implementation checks, 38 mutation witnesses.
3. **Operator-domain group — PASS:** 30/30.
4. **Pi-adapter group — PASS on complete rerun:** 108/108 plus 18/18 registered mutations. The first full run had the single timeout NIT recorded above; its focused and complete reruns passed.
5. **Web cockpit — PASS:** 145/145.
6. **CLI — PASS:** 53/53 plus real-core resource projection.
7. **token-commune adapter — PASS:** 63/63.
8. **Hygiene — PASS:** `git diff --check`; clean tracked tree before this review file; `/` retained 53G free.

## Recommendation

**Return `research-handoff-pi-adapter-capability-cursor-replay-resync` to `implementing`.** Fix the four MATERIAL integration classes, mutation-test each fix, and rerun the thorough review convergence pass. Do not advance the story to `done` yet.
