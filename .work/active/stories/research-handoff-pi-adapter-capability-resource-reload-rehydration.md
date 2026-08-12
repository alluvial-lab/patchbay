---
id: research-handoff-pi-adapter-capability-resource-reload-rehydration
kind: story
stage: implementing
tags: [adapter, protocol]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-manifest-profile, research-handoff-pi-adapter-capability-control-session-integrity, research-handoff-pi-adapter-capability-rpc-process-supervisor, research-handoff-pi-adapter-capability-cursor-replay-resync]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Idle-only Pi entrypoint/resource reload and persisted rehydration

## Redesign disposition

Rewritten after the review. The previous completion marker did not fence active execution and could be in-memory only. `RELOAD_SCOPE_EXTENSION_RESOURCES` also overclaimed arbitrary extension dependency refresh. Both premises are removed.

## Checkpoint

Expose a typed Pi `reconfigure` action whose effect is narrowly the loaded extension entrypoint plus resource paths Pi enumerates on reload. Admit it only under an exclusive settled runtime gate and on a materialized valid session. Success requires a materialized request marker, matching new-instance completion marker, new challenged handshake, subscription rebind, and cursor reconciliation. It never increments generation or claims runtime/package/dependency-graph upgrade.

## Design

**Files**
- `pi-adapter/extensions/patchbay-control.ts` — bounded request/completion custom entries and `await ctx.reload(); return` command.
- `contracts/proto/patchbay/pi_adapter.proto` — typed `PiReconfigureRequest`, Pi reloadable-resource enum, and process-replacement-required outcome/reason.
- New `pi-adapter/src/reload_controller.ts` — exact command correlation, action-gate admission, marker reconciliation, re-handshake, rebind, timeout/ambiguity mapping.
- `pi-adapter/src/{runtime_action_gate,entry_reconciler,spawn_supervisor,delivery,core_client}.ts` — shared serialization and current-generation evidence.
- Generated Pi profile declaration and `docs/ADAPTER-PI.md` / architecture wording during implementation.

```ts
export interface ReloadController {
  reloadEnumeratedResources(operation: Operation, runtime: PiRpcRuntime): Promise<PiReloadResult>;
}

export interface RuntimeActionGate {
  withExclusiveCurrent<T>(
    runtime: PiRpcRuntime,
    action: (snapshot: SettledRuntimeSnapshot) => Promise<T>,
  ): Promise<T>;
}
```

Admission while holding the only managed stdin/delivery gate requires:

- current core/runtime-generation disposition and attachment/process token;
- no in-flight adapter delivery, direct bash/action, or outstanding stateful RPC;
- `get_state.isStreaming === false`;
- `get_state.isCompacting === false`;
- `get_state.pendingMessageCount === 0`;
- either no agent/retry/compaction start in the current process incarnation or a tracked `agent_settled` epoch newer than the most recent such start;
- `PiSessionMaterialization.kind === "materialized"` with current strict seal.

Any failed condition rejects before the request marker or `ctx.reload()`. Reload does not abort/quiesce user work; the operator may retry after settlement.

The generated Pi reload request admits only the entrypoint/enumerated resource classes attested by Pi's reload lifecycle. The profile may name extension entrypoint, skills, prompts, themes, and context files. It must separately declare these process-replacement-only classes: arbitrary imported extension dependency graph, Pi/runtime installed package aliases, compiled `/dist`, native dependencies, executable, and unknown scope. Project trust/cwd are launch/resource-context facts, not generally mutable reload guarantees.

Execution:

1. append `patchbay.control.reload-request.v1` with exact command id, bounded nonce, prior extension epoch, and requested admitted resource set;
2. verify the marker is physically materialized in the strict tree;
3. call `await ctx.reload(); return`;
4. the new extension instance scans the exact unmatched request during `session_start(reason=reload)` and appends one matching completion with a greater extension epoch;
5. verify completion is materialized, reject duplicates/conflicts, and re-run the challenge handshake against the new epoch;
6. rebind process-local subscriptions/hooks and reconcile the Pi cursor/projection; keep connectivity/activity stale/unknown during the rebind;
7. report success only after all evidence is current. Prompt acceptance or completion marker alone cannot complete the Operation.

Failure before the request marker proves no reload effect. Loss after request/reload invocation is ambiguous and reconciles from exact persisted markers/current extension epoch; it never reports success from absence or retries `ctx.reload()` blindly. No path changes runtime generation.

## Acceptance evidence

- [ ] Streaming, compacting, queued, auto-retrying, direct-RPC-busy, stale-generation, and memory-only sessions reject before marker/effect.
- [ ] The gate remains held from state/settled checks through command invocation, closing a new-delivery race.
- [ ] Request and completion markers must both exist in the materialized raw/RPC-valid tree; in-memory markers cannot pass.
- [ ] Old call-frame code performs no post-reload mutation; the new extension epoch supplies the completion/handshake.
- [ ] Success requires re-handshake, re-subscription, and cursor reconciliation, not marker presence alone.
- [ ] Extension entrypoint and enumerated resource changes can refresh; an arbitrary transitive dependency or Pi/runtime `/dist` change remains old until spawn continuation/process replacement.
- [ ] Busy/unsupported scope is distinct from execution ambiguity and carries canonical retry guidance without a new core state.
- [ ] Logical target, Pi session continuity identity, process, and generation remain unchanged.
- [ ] Mutations invoking while streaming, allowing unmaterialized markers, broadening dependency scope, or completing on prompt/marker alone fail.

## Ordering constraint

Consumes the opaque Pi profile schema, strict control/materialization proof, shared runtime gate/supervisor, and authoritative reconciler. It adds no spawn-side contract or generation transition.
