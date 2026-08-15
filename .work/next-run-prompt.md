# Resume prompt — spawn-stride autopilot (paste into a fresh session after clearing context)

> Copy everything below the line into the new session as your first message.

---

Autopilot the **spawn stride** in the Patchbay `.work/` substrate. Load + follow the agile-workflow `autopilot` skill (`/home/agent/.pi/agent/git/github.com/nklisch/skills/plugins/agile-workflow/skills/autopilot/SKILL.md`). CWD `/home/agent/projects/patchbay`.

## Scope — exactly these two features + their children (the v1-must)
- `.work/active/features/research-handoff-spawn.md` (16 children) — Patchbay's `spawn` OperationKind + restart-as-continuation (the herdr-replacement).
- `.work/active/features/research-handoff-pi-adapter-capability.md` (6 children) — the Pi adapter's spawn/restart/reload surface + minimum manifest.
- Do NOT touch the other active items (ppc public-compatibility/publication-governance, verification-discipline-checks) — out of scope. **EXCEPTION — `capability-manifest-durability-and-reconciliation-depth` is IN SCOPE** (operator lifted the flag 2026-08-13): hard `depends_on` for the Pi feature + 3 Pi children; run `feature-design` on it when approaching the Pi manifest gate; it does NOT block spawn-side work.

## The designs are GATE-COMPLETE — implement, don't redesign
Passed research-grounding → feature-design → 5-reviewer adversarial review (10 BLOCKERs) → redesign (all resolved). Review: `.work/active/reviews/spawn-stride-adversarial-review-2026-08-12.md`. **Implement against the resolved designs; do not re-litigate resolved BLOCKERs.**

## PROGRESS (session 2026-08-14/15) — ALL 6 CONTRACT LEAVES + UNIT 1 DONE; pushed through `ef00e8f`
- Leaf 1 identity ✅, Leaf 2 continuation ✅, Leaf 3 claim-registry ✅, Leaf 5 crash-evidence ✅ (prior session).
- **Leaf 6 `runtime-evidence-promotion-contract` ✅ DONE** — converged CLEAN at pass 7 after 6 fix rounds (6→5→2→1→3→1→CLEAN; commits `eee06b2`→`cc7cbb1`→`4d18ce0`→`3154368`→`c9a0eee`→`87b3882`+`8eda91c`→`9ae4488`). Full production wiring: classifier+quarantine+ordered-promotion-fold in real ingress/observation/authority/server-aggregate; storage boundary exclusivity on ALL routes; descendant authority bound to exact accepted Operation; result-ordering + conflict suppression; idempotent staged-successor + deferred-success retries; five dedicated writers pairwise disjoint. Reviews: `.work/active/reviews/leaf6-runtime-evidence-*` (7 files; `-rereview6` = pass 7 CLEAN).
- **Leaf 4 `cursor-authoritative-replacement-contract` ✅ DONE** — converged at NITs pass 3 after 2 fix rounds (`b9bb6ec`→`72a2678`→`233ef83`). Non-subclassable exported `AuthoritativeCursorReplacement`; overlapping-reader store conformance; pending-guard oracles; 23/23 operator-domain tests. Reviews: `.work/active/reviews/leaf4-cursor-replacement-*` (3 files).
- **Unit 1 `fleet-spawn-target-resolution` ✅ DONE** — pass-2 CLEAN (`f217140` + fix `61f9c13`). Compound two-Grant resolution + exact-prior machinery landed; pass-1 BLOCKER (durable envelope dropped claim/replacement Grant) fixed by RE-GUARDING the continuation submit path (zero durable events, canonical `unsupported_command`) until Unit 2 lands the atomic `SpawnClaimAccepted` writer — **Unit 2 MUST remove that one enumerated guard** as part of its acceptance writer. Reviews: `.work/active/reviews/fleet-target-resolution-*{,-rereview*}`.

## ⚠️ CI is BLOCKED on GitHub Actions billing (operator action needed)
Every push since `08ecec8` fails in ~5s: "The job was not started because recent account payments have failed or your spending limit needs to be increased" (Billing & plans). NOT a code failure. Every worker + reviewer ran the FULL four-group suite locally (all green); treat local verification as the gate and RETRY CI (`gh run list --workflow=ci.yml`) at each wave boundary until billing is fixed. Operator must fix billing; do not modify CI config to work around it.

## RESUME HERE — the operation waves (from authoritative `depends_on`)
All deps of `fleet-spawn-target-resolution` are done → **dispatch it NOW** (worker `gpt-5.6-sol` xhigh; body was rewritten by the redesign — read it fresh). Wave plan (serialize ANY worker that edits protos — only one `npm run gen` at a time; parallel non-proto Rust units in pairs; every worker + reviewer runs the FULL four-group suite):

```
NOW  → wave B: spawn-delivery-atomic-claim-idempotency-generation ∥ deployment-authority-workspace-scoped-revocable-keys (both dep fleet ✅; Rust non-proto — parallel OK). Unit 2 owns: atomic deduplicating `SpawnClaimAccepted` writer (claim + ContinuationAuthorityProvenance + pending-replacement fence + prior-work effects, ONE transaction) AND removing Unit 1's enumerated continuation guard.
then → control-session-integrity SOLO (Pi Unit 2, ready; edits pi_adapter.proto — proto editor)
then → logical-target-registration → {idempotency-duplicate-handling ∥ generation-monotonicity-tombstoning}
       → stale-event-fencing → completion-promotion-driver → restart-continuation-orchestration → reconnect-cursor-reconcile
then → capability-manifest-durability (feature-design it) → manifest-profile; Pi chain:
       rpc-process-supervisor → cursor-replay-resync → {resource-reload ∥ lifecycle-conformance}
```

Review lane: `[verification]`-tagged stories get the DEEP lane (completeness→adversarial convergence, mutation-test every invariant). Leaf 6/4 needed 6/2 fix rounds — expect multi-round convergence on the operation units too; dispatch fix rounds with the same conventions until a pass returns CLEAN/nits. Every fix worker gets: the review file path, "fix not redesign", "fix the CLASS (boundary exclusivity + durable pre-state validation + retry reconciliation)", mutation-test own fixes, full four groups, commit-not-push, story bookkeeping (stage review + implementation notes).

## HOW — conventions (non-negotiable, learned the hard way)
- **Autopilot runs TOP-LEVEL (you). Do NOT delegate autopilot itself** (recursion guard strips spawn tools from children). Spawn workers/reviewers top-level.
- **All workers on `openai-codex`, model explicit** — `gpt-5.6-sol` high/xhigh throughout the spawn spine (security-critical); `gpt-5.6-luna` only for genuinely mechanical work. Reviewers fresh-context `gpt-5.6-sol` xhigh.
- **Serialization:** one proto editor at a time (`npm run gen` rewrites ALL Rust+TS bindings). Non-proto Rust units may parallelize in pairs (cargo lock serializes builds). Workers' four-group verification includes `check:drift` which runs buf generate — two workers are only safe together when NEITHER changes proto content.
- **Full four-group verification (workers AND reviewers, no substitutions — an r5 worker once skipped clippy and shipped a lint error):** (1) `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; (2) `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`; (3) `cd operator-domain && npm run build && npm test`; (4) `cd pi-adapter && npm test`.
- **Reviewers run in detached temp worktrees when mutating** (pass-3+ Leaf 6 precedent) or restore with `git restore`; tree must be clean at end; never commit mutations.
- **Subagents commit but don't push** — push + verify CI per wave (currently billing-blocked; keep pushing, keep noting).
- **Disk hygiene:** SQLite WAL/SHM leak FIXED (f35eda9). KEEP `target/debug` (43G build cache). `target/test-tmp` ~1G — clear ONLY at a wave boundary AND only when the next worker will touch Rust sources first (deleting it after a build without a source change breaks tests with dir errors). Do NOT rm after a TS-only worker boundary.
- **Context discipline:** carry only queue state + summaries; delegate heavy work. At ~70% context, checkpoint (commit + update THIS file) and stop.

## Acceptance
Drive the spawn stride to `done`: all ~22 child stories done (implemented + reviewed), both features done. Push; confirm CI green once billing is fixed. Do not cut a release (separate release-deploy stride).

Begin: read `.work/CONVENTIONS.md` + `AGENTS.md` + the two feature bodies, then dispatch the `fleet-spawn-target-resolution` worker (its story body + feature §"Unit 1" + Leaf-2/Leaf-3 contracts; guard note: the continuation submit path is un-guarded until THIS unit wires compound authority).
