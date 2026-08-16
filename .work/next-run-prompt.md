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

## PROGRESS (sessions 2026-08-13→16) — spawn FEATURE ✅ DONE; capability-durability FEATURE ✅ DONE; Pi feature 5/6 children DONE (control-integrity p2, manifest-profile p2, rpc-supervisor p4, cursor-replay p2); pushed through this checkpoint; remaining: resource-reload → lifecycle-conformance → Pi feature review → Phase 8
- Leaf 1 identity ✅, Leaf 2 continuation ✅, Leaf 3 claim-registry ✅, Leaf 5 crash-evidence ✅ (prior session).
- **Leaf 6 `runtime-evidence-promotion-contract` ✅ DONE** — converged CLEAN at pass 7 after 6 fix rounds (6→5→2→1→3→1→CLEAN; commits `eee06b2`→`cc7cbb1`→`4d18ce0`→`3154368`→`c9a0eee`→`87b3882`+`8eda91c`→`9ae4488`). Full production wiring: classifier+quarantine+ordered-promotion-fold in real ingress/observation/authority/server-aggregate; storage boundary exclusivity on ALL routes; descendant authority bound to exact accepted Operation; result-ordering + conflict suppression; idempotent staged-successor + deferred-success retries; five dedicated writers pairwise disjoint. Reviews: `.work/active/reviews/leaf6-runtime-evidence-*` (7 files; `-rereview6` = pass 7 CLEAN).
- **Leaf 4 `cursor-authoritative-replacement-contract` ✅ DONE** — converged at NITs pass 3 after 2 fix rounds (`b9bb6ec`→`72a2678`→`233ef83`). Non-subclassable exported `AuthoritativeCursorReplacement`; overlapping-reader store conformance; pending-guard oracles; 23/23 operator-domain tests. Reviews: `.work/active/reviews/leaf4-cursor-replacement-*` (3 files).
- **Unit 1 `fleet-spawn-target-resolution` ✅ DONE** — pass-2 CLEAN (CONTINUED BELOW)
- **Unit 2 `spawn-delivery-atomic-claim-idempotency-generation` ✅ DONE** — pass-2 CLEAN (`6f986fb` + fix `dce6872`). Atomic dedup acceptance (claim+provenance one transaction, exclusive key, exact-retry dedup, no cached N+1); fence atomic with acceptance (superseded/replacement_pending, never-offered superseded in-decision, delivered/running → quiesce, fence through poison); Unit 1's continuation guard REMOVED (round-trip oracles closed); pass-1 gaps fixed: delivery now carries generated `Delivery.accepted_spawn` exact envelope (hot+restart, per-field mutation rejection); continuations EXCLUDED from legacy broad-Grant completion tail (fresh bridge kept); dedicated writer binds payload/claim/provenance/fence intent (distinct Grants, exact N+1, prior equality). Reviews: `.work/active/reviews/spawn-delivery-atomic-claim-*` (2 files).
- **Unit 8 `deployment-authority-workspace-scoped-revocable-keys` ✅ DONE** — pass-2 CLEAN (`7e2d03f` + fix `ab83bda`). Workspace/project/shape/target-scoped resolver, revocable+expiry, per-continuation recheck, credential-free path still validates policy+provenance (only handle lookup skipped), distinct Grant ids + runtime-session identity required, closed exception-metadata redaction (frozen error-code registry). 38/38 adapter tests. Reviews: `.work/active/reviews/deployment-authority-*` (2 files).
- **Disk — MECHANICAL GUARD INSTALLED (2026-08-15, after a SECOND near-full event):** cargo never GCs incremental sessions and convergence loops regrew `target/debug/incremental` to 18G/91% despite prose conventions. Now: cron `*/10 * * * * ~/.local/bin/patchbay-disk-guard.sh` clears the incremental cache whenever free < 15G AND no cargo/rustc is running (safe by construction; logs to ~/.local/state/patchbay-disk-guard.log). Belt-and-suspenders for the orchestrator: check `df -h /` before EVERY dispatch; below 20G → clear incremental yourself first. Reviewer dispatches additionally set `CARGO_INCREMENTAL=0` for their one-shot full-suite runs (zero incremental artifacts, no iteration cost). `target/test-tmp` stays orchestrator-managed at true wave boundaries only (deleting mid-suite breaks tests). **Guard #4 (2026-08-15):** `[profile.dev] debug = "line-tables-only"` in the root Cargo.toml — full DWARF was most of the artifact weight; a clean rebuild took target 33G→1.8G (box ~63G free steady). Revert that profile + `cargo clean` if interactive variable-inspection debugging is ever needed. Together guards #3+#4 make the workspace footprint ~2G steady, no unbounded pools.
- **Unit 1 `fleet-spawn-target-resolution` ✅ DONE** — pass-2 CLEAN (`f217140` + fix `61f9c13`). Compound two-Grant resolution + exact-prior machinery landed; pass-1 BLOCKER (durable envelope dropped claim/replacement Grant) fixed by RE-GUARDING the continuation submit path (zero durable events, canonical `unsupported_command`) until Unit 2 lands the atomic `SpawnClaimAccepted` writer — **Unit 2 MUST remove that one enumerated guard** as part of its acceptance writer. Reviews: `.work/active/reviews/fleet-target-resolution-*{,-rereview*}`.

## CI — UNBLOCKED (repo went public 2026-08-15)
Public repos get free Actions minutes; the billing failures stopped. Still push + verify CI per wave (`gh run list/view --workflow=ci.yml`, ~9-10 min; jobs: contracts-and-conformance, rust, typescript-suites). If a run fails fast again with a billing annotation, surface it to the operator — do not modify CI config.

## RESUME HERE — the operation waves (from authoritative `depends_on`)
All deps of `fleet-spawn-target-resolution` are done → **dispatch it NOW** (worker `gpt-5.6-sol` xhigh; body was rewritten by the redesign — read it fresh). Wave plan (serialize ANY worker that edits protos — only one `npm run gen` at a time; parallel non-proto Rust units in pairs; every worker + reviewer runs the FULL four-group suite):

```
NOW  → logical-target-registration (Unit 3: staging only; Rust core/server, dep Unit 2 ✅) — SOLO (next Pi unit is a proto editor; serialize)
then → control-session-integrity (Pi; edits pi_adapter.proto — proto editor, SOLO) — CAN overlap Unit 3 only if Unit 3 does not edit protos, but buf-generate reads the shared working tree, so SERIALIZE: Unit 3 first, then Pi.
then → {idempotency-duplicate-handling ∥ generation-monotonicity-tombstoning} (both dep Unit 3; Rust non-proto — parallel OK)
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


## Session-4 additions (2026-08-15/16)
- **Units 5, 7, 9, 10 DONE** (stale-event-fencing p4; completion-driver p1 CLEAN; restart-orchestration p3 incl. new generated `ContinuationContextStatus` carriage; reconnect-cursor p2 incl. lineage-anchored cockpit snapshots). ALL 16 children done.
- **Feature integrated review** (`.work/active/reviews/spawn-feature-review-2026-08-15.md`): core spine judged strong (no authority bypass, no duplicate-generation, no partial promotion); 4 MATERIALs at operator/substrate boundaries (see wave plan above). Pi concrete spawn explicitly deferred-by-scope to the Pi feature — correct.
- Workers DO sometimes halt correctly on proto-window needs (Unit 9 finding 2) — re-dispatch with explicit proto-editor authorization worked cleanly.
- CI green through `3518197` (public repo).

## Session-3 additions (2026-08-15)
- **Unit 3 `logical-target-registration` ✅ DONE** — pass-4 nits after 3 fix rounds (`03b9ea0`→`7a058cc`→`014f3a5`→orchestrator-inline `0420025`+`0871d13`). Managed staging-only + reverse-index reservation; schema-v6 indexed retry reconciliation; bounded end-to-end gate path (AdapterRegistryLookup point materialization + feature-gated whole-registry clone counter seam). Lesson recorded: `git restore <file>` reverts uncommitted work too — COMMIT BEFORE mutating.
- **Unit 4 `generation-monotonicity-tombstoning` ✅ DONE** — pass-4 CLEAN after 3 rounds (`7d57a39`→`faa71e4`→`de0c48c`→`d7af211`). Rounds 1-2 patched tombstone-presence inference (kept failing); round 3 ROOT-CAUSED: explicit managed-lineage provenance marker in checkpoint format 3 (format 2 = unmarked legacy) — converged immediately. Heuristic inference of intent from state shape is a known trap; prefer explicit provenance.
- **Unit 6 `idempotency-duplicate-handling` ✅ DONE** — pass-5 CLEAN after 4 fix rounds (`972942f`→`3e8cc88`→`c3db45e`→`d64019c`→`ec6f7ee`). Effect-before-ack poisons; identified-success stages; ownership-index binding; identified-launch poisons; running-state oracle; per-stream offer tracking made DURABLE (audited offer marker before delivery yield + CommandIndex replay reconstruction + startup poison of offered-unacked). 5 review files.
- **Parallel-pair pattern works**: disjoint Rust file sets + cargo lock + orchestrator verifies merged state after both commit; then parallel read-only reviews on the clean tree.
- CI still billing-blocked (every push fails in ~5s; local four-group verification is the gate; operator must fix GitHub billing).

## Acceptance
Drive the spawn stride to `done`: all ~22 child stories done (implemented + reviewed), both features done. Push; confirm CI green once billing is fixed. Do not cut a release (separate release-deploy stride).

Begin: read `.work/CONVENTIONS.md` + `AGENTS.md` + the two feature bodies, then dispatch the `fleet-spawn-target-resolution` worker (its story body + feature §"Unit 1" + Leaf-2/Leaf-3 contracts; guard note: the continuation submit path is un-guarded until THIS unit wires compound authority).
