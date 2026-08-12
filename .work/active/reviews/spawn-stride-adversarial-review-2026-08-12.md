---
id: spawn-stride-adversarial-review-2026-08-12
kind: story
stage: done
tags: [review, design, spawn]
parent: research-handoff-spawn
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-12
updated: 2026-08-12
---

# Adversarial design review — spawn stride (2026-08-12)

Five fresh-context `openai-codex/gpt-5.6-sol` reviewers (lifecycle/invariants, core/adapter authority seam, decomposition/dependency graph, research fidelity/pre-mortem, Pi-substrate realism) attacked the spawn-stride designs (`research-handoff-spawn` + `research-handoff-pi-adapter-capability` + ~14 child stories). **Verdict: not safe to implement as designed.** Raw 14 BLOCKER / 14 MATERIAL / 6 MINOR → ~10 distinct BLOCKER themes after dedup (four confirmed by two reviewers independently).

This review is the design gate for the spawn stride; the re-feature-design must resolve every BLOCKER below before advancing to a sound implementing decomposition.

## BLOCKERs (deduplicated)

### Authority (security)
1. **Continuation authority escalation** *(authority-seam + lifecycle)* — `fleet-spawn-target-resolution.md:18`, `research-handoff-spawn.md:147`, `docs/SECURITY.md:207,214-218`. Continuation authorizes over a prior target with only an adapter-wide `spawn` grant; no authority over the *exact prior generation*. An endpoint permitted to spawn disposable workers can continuation-kill + acquire a protected target/gen, or revive a revoked one (descendant grant for N+1 undoes revocation of N). **Fix:** compound authorization before acceptance — adapter-scoped `spawn` **plus** an exact-prior-generation replacement/session-management grant, both preserved in durable provenance.
2. **Generation promoted to current before success + descendant authority** *(lifecycle + research-fidelity)* — `research-handoff-spawn.md:149-151,297`, `generation-monotonicity-tombstoning.md:30`, `restart-continuation-orchestration.md:55-56`, `docs/PROTOCOL.md:654,728`. N+1 is advanced/tombstoned/published before the Result + descendant grant; a crash there permanently authority-strands N+1 (replay reconstructs N+1 current with a failed creating Operation + no grant). **Fix:** promotion of N+1 to current/live must be atomic/staged on *all* success evidence + descendant authority.

### Invariants (correctness)
3. **Ambiguous-failure claim released → duplicate runtime** *(research-fidelity + lifecycle)* — `spawn-delivery-atomic-claim-idempotency-generation.md:53,62`, `idempotency-duplicate-handling.md:41,48`, `research-handoff-spawn.md:306`. Any non-success terminal (incl. `failed/cancelled/expired`) releases the claim for reuse; but `execution_outcome_unknown` means an external runtime may already exist → a retry creates a second runtime on the same generation (per-Operation journal can't correlate cross-command). **Fix:** only failures with durable proof of "no external effect" may release a claim; ambiguous/delivered cancellation/expiry must poison/retain it pending reconciliation or target abandonment.
4. **Stale-fence can't classify a legitimate first successor report** *(lifecycle)* — `stale-event-fencing.md:19,26,52`, `generation-monotonicity-tombstoning.md:21`. Classifier has only `Current/Tombstoned/Unknown/IdentityMismatch`; `Unknown` fails closed. A fresh gen-1 or N+1 report is necessarily not-current until applied → classifies `Unknown` → faithful implementation rejects every spawn (or an ad-hoc SessionReport bypass breaks "one fence for every ingress"). **Fix:** add a claim-correlated `ClaimedSuccessor` disposition validating exact Operation provenance + N→N+1.
5. **Stale classification not durably encoded in the source event** *(lifecycle)* — `stale-event-fencing.md:28,52`, `docs/PROTOCOL.md:390,618,623`. Design appends a raw `Observation` + separate `stale_event` audit; the raw event is indistinguishable from authoritative to transcript/completion/diagnostics/future projections, and replay applies it before the audit. **Fix:** persist a self-contained stale-evidence envelope, or define an atomic replay decision unit no projection can consume as a normal Observation.
6. **Unknown-cursor replay lacks authoritative replacement** *(research-fidelity)* — `research-handoff-pi-adapter-capability.md` Unit 3, `cursor-replay-resync.md`. Idempotent upsert over a full-fetch leaves stale projected entries (unknown cursor is exactly where append-only may have failed via truncation/wrong-session/corruption). **Fix:** atomic projection epoch/replacement or exact-set comparison before installing the new cursor/leaf.

### Decomposition (implementability)
7. **Hidden dependency cycles / missing contract leaves** *(decomposition, ×4)* — declared DAG is acyclic but implementation contracts reverse-depend: `fleet-spawn-target-resolution` consumes `logical-target-registration`'s types/projection; `generation-monotonicity-tombstoning` needs the claim registry defined in downstream `spawn-delivery-atomic-claim`; restart/reconnect require Pi mechanisms owned downstream. **Fix:** split the shared contract leaves (logical-target-id shape, claim-registry/cursor contract, continuation-payload schema, crash-evidence format) into early leaf stories *before* the operations that consume them; re-derive a valid bottom-up ordering.
8. **No owner for the grant-before-completed completion driver** *(decomposition + authority-seam)* — `research-handoff-spawn.md:31,151,266`, `restart-continuation-orchestration.md:214-222`. `SpawnCompletionDriver` / `server/src/spawn_completion.rs` / `core/src/authority/spawn_tail.rs` (grant-before-completed + crash-prefix migration) is referenced but assigned to no child. **Fix:** explicit child checkpoint owning both core files + the grant-before-completed tests.

### Pi-substrate (false premises)
9. **RPC handshake can't verify `cwd`** *(pi-substrate)* — `research-handoff-pi-adapter-capability.md` continuation step 7. `get_state`/`get_session_stats` expose `sessionFile`/`sessionId`, not `cwd` (`pi-rpc`{9}); the post-launch proof is unimplementable as named. **Fix:** a real control-extension handshake that reports cwd, or a revised proof contract that doesn't require it.
10. **Durable-JSONL assumption is false** *(pi-substrate)* — continuation seals/re-verifies a regular JSONL before respawn; reload depends on "persisted" markers. Pi defers writing a new session until the first assistant message (`pi-sessions`{2,3}; `dist/core/session-manager.js:724-736`). A fresh/aborted generation can have a `sessionFile` but no sealable file; reload markers can be in-memory only. **Fix:** explicit session-materialization/flush behavior or narrower availability conditions.

## MATERIALs (rolled up, ~10 — address in the redesign)
- Pi-specific vocabulary (cwd/project_trust/extensions/skills/themes/context_files) embedded as mandatory core manifest fields — leaks Pi-shaped ontology into core (`ARCHITECTURE.md:48`); move to a generated Pi extension/opaque adapter profile.
- Reload not fenced against active Pi execution *(research-fidelity + pi-substrate)* — reload tears down extension runtime under in-flight work; a completion marker proves reload ran, not that work settled. Reject/quiesce reload while streaming/compacting.
- Session-file seal validates header/inode/framing but not the JSONL tree (interior malform, dup ids, broken parent links) — `resumed` can be over a partially loaded conversation. Require full parse/tree validation or prove Pi fails closed.
- Pi manifest declares itself "complete" but defers the harvested durability dimensions (dedup/continuation-proof/cursor/generation-fence/reconciliation strength) to the drafting sibling + has no `depends_on` it — recreates the manifest-overclaim the research flagged.
- Prior 5-BLOCKER review not traceably closed (only BLOCKERs 3-5 retained; 1-2 text not preserved) — cannot substantiate "all five addressed"; do not count it as the fresh-context gate.
- External-runtime identity uniqueness across logical targets unspecified — two logical targets can claim the same `(adapter, deployment, runtime_session_id, generation)`. Enforce a reverse index (`duplicate-native-reference` vector).
- Active restart claim doesn't fence ordinary N-bound work during quiescence — an `instruct` accepted/delivered post-claim-pre-termination executes against a dying runtime. Add a durable pending-replacement delivery fence.
- Reload scope (`RELOAD_SCOPE_EXTENSION_RESOURCES`) broader than the loader evidence (`pi-loader`{3,4}) — extension dependency graphs / `/dist` aren't refreshed by reload; needs process replacement. Distinguish entrypoints/enumerated resources from arbitrary deps.
- Generation-scoped cursor storage discards Pi's restart-stable cursor (`pi-rpc`{4}) — N+1 can't load N's last cursor without an unstated transfer; key by verified Pi session identity, not Patchbay generation.
- "Authenticated exact claim" overclaims runtime provenance — authentication ≠ proof against a dishonest/buggy adapter stamping old output as N+1. State the trust assumption explicitly.
- (lifecycle MINOR→effectively material) Failure-phase connectivity mapping under-specified — define the N/N+1 state outcome per orchestration phase (authority-fail at step 1 ≠ clean step-4 termination ≠ explicit crash ≠ unexplained stream loss).

## Required redesign scope
The re-feature-design must resolve BLOCKERs 1–10 (1–5 non-negotiable authority/invariant; 6–8 decomposition; 9–10 Pi-substrate) + address the MATERIALs, then re-derive a sound (acyclic, no hidden contract cycles, all-BLOCKERs-addressed) child decomposition with explicit contract leaves + the completion-driver owner. The authority/invariant cluster (1–5) is the hardest; the decomposition re-slice (7) + contract leaves must precede a valid bottom-up ordering.
