---
id: research-handoff-spawn-restart-continuation-orchestration
kind: story
stage: implementing
tags: [adapter, protocol]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-stale-event-fencing, research-handoff-spawn-idempotency-duplicate-handling]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Restart as a typed spawn continuation

## Checkpoint

Implement restart as a **new `spawn` Operation** with a new command id/idempotency key and a typed continuation payload naming the exact prior logical target/runtime generation. Do not add `restart` to `OperationKind`, and do not overload generic `session-management`. The adapter owns quiesce/terminate/respawn/native-session selection; the core owns the claim, durable lifecycle, identity, authority, and generation transition.

Continuation restores adapter-native logical context, not arbitrary process state. The result records `resumed`, `new_context`, or `unknown`. `/reload` is not a Pi/runtime package-upgrade boundary; process replacement is.

## Design

**Files**
- `contracts/proto/patchbay/operations.proto` — generated typed fresh/continuation spawn payload.
- `contracts/proto/patchbay/sessions.proto` — `ContinuationStatus` and exact `continuation_of` reference on the resulting generation.
- `pi-adapter/src/delivery.ts` — admit/parse `OperationKind.SPAWN`, validate target-spec shape, and route the persisted claim to the supervisor.
- `pi-adapter/src/spawn_supervisor.ts` (new) — sole owner of create/quiesce/terminate/respawn/continue policy.
- `pi-adapter/src/pi_session.ts` — explicit continuation/open API and binding-local incarnation checks; no independent generation increment.
- `pi-adapter/src/session_registry.ts` — index stable logical targets and replace one runtime entry only through supervisor commit.
- `pi-adapter/src/core_client.ts` — report the exact logical target, claim, spawn operation, continuation reference/status, and source cursor.
- `pi-adapter/src/main.ts` — capability manifest advertises supported target-spec shapes and spawn only when supervision is configured.
- `web-cockpit/src/main.ts`, `web-cockpit/src/ui/session-detail.ts`, `cli/src/commands/spawn.ts`, and `cli/src/main.ts` — fresh spawn/restart entry actions using generated payloads and target-before-intent output.
- `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md`, `docs/UX.md`, and `docs/GLOSSARY.md` — roll the chosen lifecycle and crash semantics forward with code.

```ts
export interface SpawnSupervisor {
  spawnFresh(operation: Operation, claim: SpawnGenerationClaim): Promise<SpawnedRuntime>;
  continue(
    operation: Operation,
    claim: SpawnGenerationClaim,
    prior: RuntimeGenerationRef,
  ): Promise<SpawnedRuntime & { continuationStatus: ContinuationStatus }>;
}
```

Required order for continuation:
1. validate delivery claim, target spec, deployment authority, and journal;
2. report delivery/running;
3. quiesce or abort the old runtime according to explicit policy;
4. terminate/dispose the old process/runtime and retain its persisted Pi session reference;
5. create/respawn using an explicit `--session`/session-manager reference (never ambiguous `--continue` when multiple candidates exist);
6. reconcile persisted entries before reporting live;
7. report the exact claimed generation and continuation status;
8. report successful spawn result; the core completion driver issues the new descendant grant and terminalizes last.

A failure before step 7 leaves the old generation failed/stale as justified by evidence and the claim terminal failed; it never fabricates generation N+1. A failure after external create but before proof is `execution_outcome_unknown` and follows duplicate reconciliation.

## Acceptance evidence

- [ ] Fresh spawn and continuation are both `OperationKind::Spawn`; continuation has a distinct new Operation identity and exact prior reference.
- [ ] Pi fresh spawn creates generation `1`; continuation produces exactly the core-claimed next generation and may change runtime/Pi session id.
- [ ] Native persisted-session restore reports `resumed`; shape-only/fresh-context fallback reports `new_context`; unprovable continuity reports `unknown`.
- [ ] Old callbacks/process handles cannot act after replacement; old generation is tombstoned before new live state is exposed.
- [ ] Successful continuation receives a new generation-scoped descendant grant before its spawn Operation becomes completed; the prior descendant grant remains independently revocable/auditable.
- [ ] `/reload` is never used to claim runtime-package upgrade or generation continuity.
- [ ] Web and CLI expose fresh spawn/restart without inventing new protocol states; unsupported adapters show canonical unavailability.

## UI fallback

No new screen is required. The web action composes into the existing session-list empty state and session-detail header; CLI follows existing Operation commands. Existing components already present canonical delivery/failure states, so no feature-level mockup is needed.

## Ordering constraint

Depends on stale-event fencing and duplicate/claim handling. Reconnect validation follows the completed orchestration path.
