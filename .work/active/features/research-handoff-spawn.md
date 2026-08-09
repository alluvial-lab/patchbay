---
id: research-handoff-spawn
kind: feature
stage: drafting
tags: [adapter, protocol, v1]
parent: epic-public-product-contract
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-08
updated: 2026-08-08
---

# Spawn — logical target + generation lifecycle (v1 must)

Wire Patchbay's committed `spawn` OperationKind (+ restart-as-spawn-continuation) so an operator can spawn fresh agents on demand and restart them, replacing herdr. This is the operator's stated v1-must and the herdr-replacement.

**Lifecycle direction (proposed — 3 open forks):**
- `spawn` creates a **stable logical target identity + a first runtime generation**; restart-as-continuation is a **new generation on the same logical target** (continuation ref to prior generation; old generation tombstoned before the new one is live). Stale-generation events/results are fenced.
- **Core/adapter split:** core owns identity, authority, durable lifecycle, generation monotonicity, stale-event fencing; the adapter owns *how* continuation is realized (terminate process, preserve session, respawn, cursor-reconcile).
- **Project/cwd seam — DECIDED (adapter-owned in v1):** core `spawn` carries an opaque typed `target_spec`; no universal `Project` entity; no shared-cwd semantics in core. `project_ref`/cwd/template/repo/worktree are adapter-declared target-spec shapes. (Reserved seam: promote a core `ProjectRef` only after defining authority/portability/lifecycle/non-shared-cwd.)
- Fields: `logical_target_id`, `runtime_session_id`, `generation`, `continuation_of`, `spawn_operation_id`, `project_ref`/`cwd_spec` (adapter payload), adapter-declared `idempotency_strength`.
- Transitions + obligations: spawn (accept before deliver; register logical target + generation; descendant grant tied to the spawn op) · detach (endpoint loss ≠ target death) · crash (record generation unavailable/stale per adapter evidence; never silently allocate a new generation) · restart-as-continuation (new generation; expose `resumed`/`new_context`/`unknown` — continuation restores adapter-native logical context, not arbitrary process state) · reconnect (snapshot/cursor reconcile; never infer live from a remembered stream) · duplicate/stale (same idempotency key → existing command; lower/equal generation → no-op or `stale_event`).

**Open forks (resolve in feature-design):** initial generation 0 vs 1; restart as a new spawn Operation vs a typed continuation payload; crash represented as unavailable/failed/stale.

## Research grounding

**Source**: `.research/analysis/campaigns/v1-control-plane-and-spawn/parent.md` (slug: `v1-control-plane-and-spawn`) — facets `spawn-lifecycle` + `pi-adapter-probe`.

The campaign grounded the lifecycle contract in herdr's session/process split, Pi's persisted-session/runtime-replacement split, and the peer landscape (no fetched peer attaches a monotonic runtime generation to command/event mutation — incarnation fencing is itself a moat component). The project/cwd seam decision is grounded in the divergent peer workspace models (herdr = terminal/process container; Coder = template compute; Pi = cwd-bound session — no universal shape, so core-neutral).
