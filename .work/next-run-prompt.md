# Resume prompt — spawn-stride autopilot (paste into a fresh session after clearing context)

> Copy everything below the line into the new session as your first message.

---

Autopilot the **spawn stride** in the Patchbay `.work/` substrate. Load + follow the agile-workflow `autopilot` skill (`/home/agent/.pi/agent/git/github.com/nklisch/skills/plugins/agile-workflow/skills/autopilot/SKILL.md`). CWD `/home/agent/projects/patchbay`.

## Scope — exactly these two features + their children (the v1-must)
- `.work/active/features/research-handoff-spawn.md` (16 children) — Patchbay's `spawn` OperationKind + restart-as-continuation (the herdr-replacement).
- `.work/active/features/research-handoff-pi-adapter-capability.md` (6 children) — the Pi adapter's spawn/restart/reload surface + minimum manifest.
- Do NOT touch the other active items (ppc public-compatibility/publication-governance, verification-discipline-checks, capability-manifest-durability-and-reconciliation-depth) — those are out of scope.

## The designs are GATE-COMPLETE — implement, don't redesign
These designs already passed: research-grounding (`v1-control-plane-and-spawn`) → `feature-design` → **5-reviewer adversarial review (10 BLOCKERs)** → **redesign (all resolved)** → **closure re-review (GO on all 10)**. The review is at `.work/active/reviews/spawn-stride-adversarial-review-2026-08-12.md`. **Implement against the resolved designs; do not re-litigate resolved BLOCKERs or re-open forks.** The key resolved decisions (in the feature bodies):
- Spawn lifecycle: generation from **1**; restart = new `spawn` Operation + typed continuation payload; crash = `failed`/`stale`/`offline`.
- Compound continuation authority (adapter-spawn **+** exact-prior-generation grant); atomic `SpawnPromotionCommitted` promotion; ambiguous-failure poisons the claim; `ClaimedSuccessor` fence; `QuarantinedRuntimeEvidence`; contract-leaves-first; completion-driver owner.
- Pi adapter: one `pi --mode rpc` subprocess per generation; `patchbay-control` cwd handshake; `memory_only|materialized|invalid` JSONL handling; cursor replacement epoch; manifest `depends_on` the durability sibling.

## HOW — the convention is non-negotiable (learned the hard way this session)
- **Autopilot runs TOP-LEVEL (you, the orchestrator). Do NOT delegate autopilot itself to a background subagent.** `@gotgenes/pi-subagents` applies a recursion guard that strips the `subagent`/`get_subagent_result`/`steer_subagent` tools from every spawned child — so a delegated orchestrator's worker/review spawns silently fail to inline fallback. You have the spawn capability at the top level; use it. (See `AGENTS.md` "Workflow execution" + `.work/active/stories/workflow-top-level-orchestrator-gate-trip-upward.md`.)
- **Delegate implementation/review to worker subagents** (you spawn them, top-level → they run cleanly). All workers on `openai-codex`; pass `model` explicitly per task complexity (`gpt-5.6-sol` for the security/invariant children, `gpt-5.6-luna` for lighter ones). Review weight: `thorough` on the authority/invariant/cursor children.
- **Pick ready items by `depends_on` + `stage`** (the substrate tracks this; 6 contract-leaf leaves are ready now). Implement → bounded standalone-story/feature review per `.work/CONVENTIONS.md` → advance to `done` → commit → next wave. The validated 18-wave bottom-up ordering is in the closure review (decomposition reviewer's output) + derivable from the children's `depends_on`.
- **Tests are safe to run directly** — a root-cause test-tempfile fix landed (`patchbay-test-support` `#[ctor]` + `.cargo/config.toml [env] TMPDIR`); `cargo test`/`npm test` scope to `target/test-tmp`, no `/tmp` leak.
- **Subagents commit but don't push** — you push + verify CI after each wave (or at stride end).
- **Context discipline:** carry only queue state + summaries; delegate heavy work. If you approach ~70% context, wrap to a clean checkpoint (commit + status-note the current wave) and stop — the substrate resumes cleanly. Don't blow through the ceiling mid-stride.

## Acceptance
Drive the spawn stride to `done`: all ~22 child stories done (implemented + reviewed), both features done. Push; confirm CI green. Do not cut a release (that's a separate release-deploy stride).

Begin: read `.work/CONVENTIONS.md` + `AGENTS.md` + the two feature bodies + the review, then start draining the ready contract leaves.
