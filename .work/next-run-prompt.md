# Resume prompt — spawn-stride autopilot (paste into a fresh session after clearing context)

> Copy everything below the line into the new session as your first message.

---

Autopilot the **spawn stride** in the Patchbay `.work/` substrate. Load + follow the agile-workflow `autopilot` skill (`/home/agent/.pi/agent/git/github.com/nklisch/skills/plugins/agile-workflow/skills/autopilot/SKILL.md`). CWD `/home/agent/projects/patchbay`.

## Scope — exactly these two features + their children (the v1-must)
- `.work/active/features/research-handoff-spawn.md` (16 children) — Patchbay's `spawn` OperationKind + restart-as-continuation (the herdr-replacement).
- `.work/active/features/research-handoff-pi-adapter-capability.md` (6 children) — the Pi adapter's spawn/restart/reload surface + minimum manifest.
- Do NOT touch the other active items (ppc public-compatibility/publication-governance, verification-discipline-checks) — those are out of scope. **EXCEPTION — `capability-manifest-durability-and-reconciliation-depth`:** see "⚠️ Pi/durability blocker" below; the operator must decide before the Pi half can complete.

## The designs are GATE-COMPLETE — implement, don't redesign
Passed research-grounding → feature-design → 5-reviewer adversarial review (10 BLOCKERs) → redesign (all resolved). The review is at `.work/active/reviews/spawn-stride-adversarial-review-2026-08-12.md`. **Implement against the resolved designs; do not re-litigate resolved BLOCKERs.** Key resolved decisions are in the two feature bodies (compound continuation authority; atomic `SpawnPromotionCommitted`; ambiguous-failure poisons the claim; `ClaimedSuccessor` fence; `QuarantinedRuntimeEvidence`; contract-leaves-first; completion-driver owner; Pi `patchbay-control` cwd handshake; `memory_only|materialized|invalid` JSONL; cursor replacement epoch).

## PROGRESS (session 2026-08-13) — 4 of ~22 children done, all pushed, CI green
Spawn-side contract leaves **done** (deep-lane reviewed, BLOCKERs/MATERIALs fixed, mutants fail):
- `research-handoff-spawn-logical-target-identity-contract` ✅
- `research-handoff-spawn-continuation-payload-authority-contract` ✅ (submit path guards continuations until `fleet-spawn-target-resolution` wires compound authority)
- `research-handoff-spawn-claim-registry-contract` ✅ (evidence-dependent transitions guarded; strict validation)
- `research-handoff-spawn-crash-external-effect-evidence-contract` ✅ (typed no-effect/external-effect evidence wired into claim validation; promotion still guarded for Leaf 6)

## RESUME HERE — the derived wave plan (from authoritative `depends_on`)
Compute with `.work/bin/work-view --ready` or by topological sort of the children's `depends_on`. Remaining spawn contract leaves first, then operation waves, then Pi:

```
NEXT → runtime-evidence-promotion-contract (Leaf 6: SpawnPromotionCommitted + QuarantinedRuntimeEvidence
        + SpawnSuccessorEvidenceStaged + atomic audited promotion append + RuntimeGenerationDisposition
        with ClaimedSuccessor. UN-GUARDS the promotion transition. Biggest contract leaf — BLOCKERs 2,4,5.)
  then → cursor-authoritative-replacement-contract (Leaf 4: adapter-neutral, TS in operator-domain/src/reconciliation/external_cursor.ts; depends identity only)
(all 6 spawn contract leaves done)
  → fleet-spawn-target-resolution  (Unit 1: compound Grant selection; UN-GUARDS continuation on submit path)
       ├─ deployment-authority-workspace-scoped-revocable-keys
       └─ spawn-delivery-atomic-claim-idempotency-generation
            └─ logical-target-registration (staging only)
                 ├─ idempotency-duplicate-handling
                 └─ generation-monotonicity-tombstoning → stale-event-fencing → completion-promotion-driver
                       + deployment-authority → restart-continuation-orchestration → reconnect-cursor-reconcile
(operation waves are mostly NON-proto Rust → can parallelize in groups; see "Serialization" below)
  → Pi children: control-session-integrity → rpc-process-supervisor → cursor-replay-resync → resource-reload → lifecycle-conformance
       (manifest-profile + reload + lifecycle + the Pi feature itself BLOCKED on capability-manifest-durability — see below)
```

## ⚠️ Pi/durability blocker (operator decision needed BEFORE the Pi half can finish)
The **Pi feature itself + 3 of its 6 children** (`manifest-profile`, `resource-reload-rehydration`, `lifecycle-conformance`) hard-`depends_on` `capability-manifest-durability-and-reconciliation-depth` (at `stage: drafting`, off-limits per original scope). The Pi manifest consumes its generic-assurance fields. So "both features done, all 22 children" is **not achievable under 'don't touch capability-manifest-durability.'** Options the operator must pick:
- **(A)** Lift the off-limits flag → design+implement `capability-manifest-durability` first (small, well-scoped: extends v0.2.0 manifest with declared durability dims + reconciliation-strength; default uncertain→false).
- **(B)** Deliver the spawn feature `done` + the 3 unblocked Pi children (`control-session-integrity`, `rpc-process-supervisor`, `cursor-replay-resync`); the Pi feature + 3 manifest-gated children land `blocked` pending the durability sibling.
3 unblocked Pi children can be done either way. Raise this if the operator hasn't answered by the time you reach the Pi manifest gate (~wave 3 of the Pi side).

## HOW — conventions (non-negotiable, learned the hard way)
- **Autopilot runs TOP-LEVEL (you). Do NOT delegate autopilot itself.** pi-subagents' recursion guard strips spawn tools from children → a delegated orchestrator's worker/review spawns silently degrade to inline. Spawn workers/reviewers top-level (they run cleanly).
- **All workers on `openai-codex`, model explicit** (`gpt-5.6-sol` high/xhigh for the security/invariant/cursor/authority children — the whole spawn spine is security-critical, so sol throughout; `gpt-5.6-luna` only for genuinely mechanical work). Review weight `thorough`; `[verification]`-tagged stories get the deep lane (completeness→adversarial, converge to nits, mutation-test every invariant — green tests are NOT proof).
- **Serialization (critical):** `npm run gen` (`buf generate`) regenerates ALL committed Rust+TS bindings. Any two proto-editing leaves running concurrently corrupt each other's gen. → **all proto/contract leaves must be serialized** (one proto editor at a time). Non-proto operation leaves (Rust core/server, no new protos) can parallelize in groups (cargo's workspace lock serializes builds safely). Compute topological waves from `depends_on` and run proto leaves serially.
- **Workers MUST run the FULL TS suite, not just Rust + contracts drift.** The continuation leaf's proto regen broke a pi-adapter e2e test that the worker missed (it only ran cargo). Every worker that touches protos must verify: `cargo build/test/clippy --workspace`; `cd contracts/ts && npm run check:drift && check:vectors && check:models && build`; `cd operator-domain && npm run build && npm test`; `cd pi-adapter && npm test`. Reviewers must run these too.
- **Disk hygiene:** the SQLite WAL/SHM sidecar leak is FIXED (`core/src/storage/rusqlite.rs` `open_in_memory` now uses a `TempDir`, commit `f35eda9` — 18G→22M residue). `target/debug` (43G) is the stable build cache — KEEP it. To clear accumulated `target/test-tmp`, use `rm -rf target/test-tmp` (the WHOLE dir — the leaked files are hidden dotfiles `.tmp*-wal`, so a `/*` glob misses them), and **only** at a wave boundary when no worker is mid-test. Do NOT `rm -rf target/test-tmp` after a build without touching a source file first (`build.rs` creates it; deleting post-build makes tests fail with dir errors).
- **Subagents commit but don't push** — you push + verify CI per wave (`gh run list/view --workflow=ci.yml`). CI ≈ 9–10 min (jobs: contracts-and-conformance, rust, typescript-suites).
- **Context discipline:** carry only queue state + summaries; delegate heavy work. At ~70% context, checkpoint (commit + update THIS file) and stop — the substrate resumes cleanly.

## Acceptance
Drive the spawn stride to `done`: all ~22 child stories done (implemented + reviewed), both features done. Push; confirm CI green. Do not cut a release (separate release-deploy stride).

Begin: read `.work/CONVENTIONS.md` + `AGENTS.md` + the two feature bodies + the review, confirm progress above, then resume at **runtime-evidence-promotion-contract (Leaf 6)**.
