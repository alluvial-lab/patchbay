---
id: research-handoff-spawn-restart-continuation-orchestration
kind: story
stage: implementing
tags: [adapter, protocol, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-completion-promotion-driver, deployment-authority-workspace-scoped-revocable-keys]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Restart as typed spawn continuation orchestration

## Redesign disposition

Rewritten to consume the safe shared contracts. Concrete Pi RPC process/session-file mechanics belong to the following Pi-adapter redesign, not this spawn-side checkpoint.

## Checkpoint

Expose intentional restart as a new `spawn` Operation with a new command/key and exact continuation payload. Apply the phase/effect contract, pending-replacement fence, compound authority, claim, staged successor, and completion driver across the generic orchestration. Do not add a `restart` OperationKind or hide replacement in `session-management`.

Continuation restores adapter-native logical context, not arbitrary process state. The adapter reports `resumed`, `new_context`, or `unknown`; only promotion makes N+1 current.

## Design

**Files**
- Server/core orchestration seams and generated payload delivery.
- Operator action construction in `web-cockpit` and `cli` using existing action surfaces.
- Rolling foundation docs when implemented.
- Pi-specific supervisor implementation explicitly deferred to `research-handoff-pi-adapter-capability` redesign.

Required generic phase order:
1. accept exact claim + compound provenance and activate N delivery fence;
2. deliver claim/target spec to the adapter;
3. record phase evidence for quiesce and old-runtime outcome;
4. adapter terminates/replaces/reconciles using its declared implementation;
5. exact successor report stages and reserves external identity;
6. successful Result/phase evidence enters completion readiness;
7. completion driver atomically promotes or claim remains active/poisoned.

Failure mapping follows the parent table: pre-offer no-effect may release atomically; delivered/launch ambiguity poisons; prior clean termination leaves N offline/current until promotion; stream loss yields stale/unknown and poison; no failure allocates or publishes N+1.

## Acceptance evidence

- [ ] Fresh and continuation remain one `spawn` kind; continuation has exact prior and both authority Grants.
- [ ] New N work rejects after claim acceptance and old work is explicitly quiesced/resolved.
- [ ] Every orchestration phase emits the typed effect/connectivity evidence expected by the parent table.
- [ ] N+1 remains staged until the atomic promotion driver commits descendant authority/current/completed.
- [ ] `resumed/new_context/unknown` does not claim arbitrary process-state continuity.
- [ ] Ambiguous failure poisons; no automatic relaunch or same-generation new claim occurs.
- [ ] Existing web/CLI actions show canonical lifecycle/failure/retry risk without new protocol states.

## UI fallback

No new screen. Reuse session-list spawn and session-detail restart actions plus canonical Operation/failure/Grant presentation.

## Ordering constraint

Depends on verified atomic completion/promotion and adapter-local credential-reference boundary. Reconnect/cursor convergence follows.
