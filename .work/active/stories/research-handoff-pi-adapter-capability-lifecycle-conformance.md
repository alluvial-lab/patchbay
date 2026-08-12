---
id: research-handoff-pi-adapter-capability-lifecycle-conformance
kind: story
stage: implementing
tags: [adapter, verification]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-cursor-replay-resync, research-handoff-pi-adapter-capability-resource-reload-rehydration, research-handoff-spawn-stale-event-fencing]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Integrated Pi lifecycle and manifest conformance

## Checkpoint

Bind the generated manifest, spawn continuation, process supervisor, cursor recovery, reload boundary, crash vocabulary, and stale-generation fence into implementation-backed v1 evidence. Use real Pi RPC child processes for lifecycle boundaries and deterministic fakes only for isolated parser/failure injection.

## Design

**Files**
- New `pi-adapter/tests/rpc_client.test.ts`, `spawn_supervisor.test.ts`, `cursor_reconcile.test.ts`, and `reload.test.ts` — focused boundary/regression tests.
- `pi-adapter/tests/e2e.test.ts` — real core + real Pi RPC fresh spawn, continuation, crash, reconnect, full resync, reload, and runtime-upgrade traces with post-test process cleanup.
- `contracts/vectors/` and vector runner registries — Pi refinements for `spawn-continuation`, `restart-native-resume`, `restart-shape-only`, `reconnect-after-stream-loss`, `cursor-gap-repair`, `manifest-overclaim`, and stale-generation evidence.
- `docs/VERIFICATION.md` and traceability generation — implementation-checked wording only unless a separately promoted formal property/vector clears its gate.

## Acceptance evidence

- [ ] The full manifest is emitted, validated, replayed, and rendered without becoming an authority/delivery gate.
- [ ] Fresh generation `1`, exact continuation `N→N+1`, descendant authority, and old-generation inertness hold through real process termination/restart.
- [ ] Cursor loss/full resync, crash-after-effect, corrupt session path, and reload-vs-runtime-upgrade mutations are killed.
- [ ] Explicit crash=`failed`, unexplained RPC loss=`stale`, and clean exit=`offline`; activity is `unknown` unless current evidence proves otherwise.
- [ ] Every test awaits child exit, observation flush, cursor/journal durability, and core terminal state; no late async error or orphan process can pass after assertions.
- [ ] Assurance prose says implementation-checked, not model-checked/release-verified, unless promotion evidence actually exists.

## Ordering constraint

Final checkpoint after manifest, supervisor, cursor, reload, and core stale-event fencing.
