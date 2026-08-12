---
id: research-handoff-pi-adapter-capability-resource-reload-rehydration
kind: story
stage: implementing
tags: [adapter, protocol]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-manifest-profile, research-handoff-pi-adapter-capability-rpc-process-supervisor, research-handoff-pi-adapter-capability-cursor-replay-resync]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Extension-resource reload and persisted state rehydration

## Checkpoint

Expose reload as a typed Pi `reconfigure` action whose effect is exactly extension/resource refresh. It does not replace the Pi executable/package graph, does not increment runtime generation, and cannot satisfy a runtime/package upgrade request; those require a new spawn continuation and supervised process replacement.

Because built-in interactive `/reload` is not a general RPC command, ship a minimal adapter extension command that calls `await ctx.reload(); return`. The command writes a bounded persisted request marker, and the new extension instance writes a completion marker during `session_start(reason=reload)`; the adapter waits for that marker through persisted-entry reconciliation instead of treating prompt acceptance as completion.

## Design

**Files**
- New `pi-adapter/extensions/patchbay-control.ts` — adapter-managed reload bridge with request/completion custom entries and no tool or policy surface.
- `contracts/proto/patchbay/pi_adapter.proto` and `pi-adapter/src/delivery.ts` — typed `PiReconfigureRequest.reload_resources`; reject runtime-upgrade intent with the canonical unsupported/failure path.
- New `pi-adapter/src/reload_controller.ts` — command-id-derived nonce, persisted completion reconciliation, timeout/ambiguity behavior, and no generation mutation.
- `pi-adapter/src/spawn_supervisor.ts`, `entry_reconciler.ts`, and `core_client.ts` — rebind/reconcile extension state and report operation outcome honestly.
- `docs/ADAPTER-PI.md` and `docs/ARCHITECTURE.md` — replace the obsolete out-of-scope reload wording with the bounded v1 behavior.

## Acceptance evidence

- [ ] Reload invokes the adapter extension command and succeeds only after a new-runtime `session_start(reason=reload)` completion marker is reconciled.
- [ ] The invoking old call frame performs no post-reload state mutation; future commands use the reloaded extension/resources.
- [ ] Reload preserves logical target, runtime session id, process, and generation; it re-subscribes/rebinds process-local hooks and reconstructs declared persisted state.
- [ ] A request to upgrade Pi/runtime/package code cannot route to reload and instead requires process continuation.
- [ ] Missing bridge, timeout, malformed marker, or process loss never reports successful reload; outcome and connectivity degrade using canonical vocabulary.
- [ ] Tests change an extension/resource and observe reload, then change a runtime-package dependency and prove only process replacement observes it.

## Ordering constraint

Depends on the manifest, managed RPC process, and persisted-entry reconciler.
