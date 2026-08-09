---
id: research-handoff-pi-adapter-capability
kind: feature
stage: drafting
tags: [adapter, v1]
parent: epic-public-product-contract
depends_on: [research-handoff-spawn]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-08
updated: 2026-08-08
---

# Pi adapter capability surface for v1 (spawn/restart/reload + manifest)

Ground the Patchbay Pi adapter's v1 spawn/restart/reload design in current Pi's actual capabilities. (Consumes + implements `research-handoff-spawn` for the Pi adapter.)

**Grounded findings:**
- **Reconnect/replay:** Pi RPC `get_entries(since)` is an authoritative **append-order cursor** over the persisted session tree (stable entry ids, `leafId`, includes pre-compaction + abandoned branches); **fails on an unknown cursor** → the adapter needs an explicit cursor-loss/full-resync path. The live RPC event stream is NOT a universal total order (parallel-tool execution interleaves) — treat RPC events as live notifications, `get_entries` as authoritative gap recovery. Persist the cursor only after handling the entry.
- **`/reload` vs restart:** `/reload`/`ctx.reload()` picks up fresh **extension-entrypoint** code (jiti `moduleCache:false` for extension imports) but does **not** replace the running Pi/runtime package graph (loader aliases runtime packages to the running `dist`). So **process restart is the reliable code-upgrade boundary** for runtime/package upgrades; `/reload` suffices only for extension-resource refresh.
- **Restart-as-continuation:** Pi documents the continuation pattern, not a restart RPC. Continuation preserves **persisted JSONL** (transcript/tree/compaction/extension custom entries), **not** in-memory runtime/extension/loader state. The adapter owns: quiesce/abort policy, terminate process, preserve/verify session path, respawn with `--session`/`--continue`, then RPC state/cursor reconciliation.
- **Capability surface:** two viable substrates — RPC subprocess (process isolation, language-agnostic) or SDK embedding (`createAgentSession`/`AgentSessionRuntime`, requires re-subscribe after replacement).
- **Minimum manifest:** declare transport / prompting / events (parallel-tool ordering caveat) / cursor_replay (`get_entries(since)`, unknown-cursor failure) / session_persistence / session_replacement / reload (resource only; restart for runtime upgrades) / resource_scope (cwd, trust, extensions, skills, prompts, themes, context) / state_rehydration.

## Research grounding

**Source**: `.research/analysis/campaigns/v1-control-plane-and-spawn/parent.md` (slug: `v1-control-plane-and-spawn`) — facet `pi-adapter-probe` (+ attestations `pi-rpc`, `pi-sessions`, `pi-extensions`, `pi-loader`, `pi-sdk`).

The probe grounds the adapter's v1 spawn/restart/reload contracts in current Pi's documented behavior, including the `/reload`-vs-fresh-`/dist` limitation that drives the operator's restart-on-rebuild workflow.
