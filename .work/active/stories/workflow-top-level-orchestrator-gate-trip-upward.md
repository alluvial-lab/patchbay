---
id: workflow-top-level-orchestrator-gate-trip-upward
kind: story
stage: drafting
tags: [workflow, infra, security]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-12
updated: 2026-08-12
---

# Orchestrator/release agents run top-level; gate spawn-blocks trip upward, not inline

## Context (what happened)
`release-deploy` for v0.2.0 was delegated to a background subagent (to keep the operator's session context clean). That subagent ran the 5 release gates, but each gate fell back to its **inline** scanner path because the scanner subagents could not be spawned. The release shipped with degraded gate rigor, and the degradation was only surfaced by the operator digging into a caveat — not by the workflow itself.

## Root cause (verified in the harness)
The `subagent` tool is provided by `@gotgenes/pi-subagents`. That package applies an **unconditional recursion guard** (`create-subagent-session.ts`):

```ts
const EXCLUDED_TOOL_NAMES = ["subagent", "get_subagent_result", "steer_subagent"];
function applyRecursionGuard(session) {
  const filtered = session.getActiveToolNames().filter((t) => !EXCLUDED_TOOL_NAMES.includes(t));
  session.setActiveToolsByName(filtered);
}
```

It runs at every child session's creation, after extensions bind. So a spawned subagent inherits the parent's other extensions but **cannot see the three dispatch tools** — by design, not accident. The agile-workflow gate skills spawn their scanners *as subagents*, so inside a delegated orchestrator the spawn has no tool to call, and the gate silently falls to its documented inline fallback ("If scanner agents are unavailable, run the audit inline").

## The principle (operator decision, 2026-08-12)
1. **Orchestrator and release agents must run at the top level** (the only session that retains the dispatch tools under `pi-subagents`' recursion guard). Do not delegate `release-deploy` (or any agent whose gates/workers fan out via subagents) one level down.
2. **A sub-agent blocked on a subagent spawn must trip upward (escalate), not continue inline.** The current gate behavior — silently degrading to an inline scan and reporting "gates passed" — masks reduced rigor from the operator. The correct behavior is to fail loudly / surface the blockage so the operator (or top-level orchestrator) can re-drive the gate from a context that can spawn.

## Actions
- **Patchbay convention (enacted in this commit):** `AGENTS.md` now records that orchestrator/release agents run top-level and must not be delegated where their gates fan out.
- **Upstream recommendation (for the agile-workflow skills repo, `~/.pi/agent/git/github.com/nklisch/skills`):** change the gate skills' "scanner agents unavailable → run inline" fallback to **trip upward** — emit a blocking error / escalation noting the recursion-guard block, so a delegated orchestrator surfaces it instead of shipping with a silent inline gate. Keep the inline path only as an explicit operator opt-in (e.g. an env flag), never a silent default. The same applies to any skill that spawns subagents (`autopilot`, `implement-orchestrator`, `bug-scan`, `deep-code-scan`, `e2e-test-design`, etc.).

## Why it matters
The v0.2.0 retroactive scan (run top-level) found 1 High + 2 Medium security findings the inline gate missed — including a shared attachment secret that lets one adapter assume another's identity. Silent inline fallback would have shipped that undetected.
