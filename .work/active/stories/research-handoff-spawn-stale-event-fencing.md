---
id: research-handoff-spawn-stale-event-fencing
kind: story
stage: implementing
tags: [protocol, security, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-generation-monotonicity-tombstoning]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Fence stale-generation events at every adapter ingress

## Checkpoint

Close the review's BLOCKER 3: a stale runtime generation must be inert at the adapter trust boundary, not merely rejected by `SessionRegistry` reports. Today a generic Observation can exactly match its old command target and still advance command state after that target was tombstoned. Every runtime-targeted SessionReport, Observation, result, delivery acknowledgement, transcript event, and Elicitation mutation must consult the same reconciled generation disposition before durable mutation.

The outpost_pi harvest field-corrobates this risk: an old action killed its successor after consulting mutable current state rather than an incarnation token.

## Design

**Files**
- `core/src/session/logical_target.rs` — one `RuntimeGenerationDisposition` classifier: `Current`, `Tombstoned`, `Unknown`, `IdentityMismatch`.
- `core/src/acceptance/ports.rs` — consumer-owned `RuntimeGenerationFence` port.
- `core/src/acceptance/observation.rs` — consult the fence before appending a state-changing candidate; stale evidence appends only the raw Observation + `stale_event` audit as one decision and emits no command transition.
- `core/src/adapter/mod.rs` — apply the same fence to delivery acknowledgements.
- `core/src/acceptance/elicitation.rs` — stale target terminalizes/keeps inert according to the canonical Elicitation rule.
- `server/src/adapter_service.rs` — rebuild/fold the fence projection under the authenticated attachment and shared decision gate before every runtime-targeted branch.
- `pi-adapter/src/pi_session.ts` — retain binding-local generation tokens so callbacks from disposed/replaced Pi runtime objects never emit current-generation reports.
- `core/tests/acceptance_observation.rs`, `server/tests/trust_boundary.rs`, and `pi-adapter/tests/e2e.test.ts` — old-generation mutations across every ingress family.

```rust
pub enum RuntimeGenerationDisposition {
    Current,
    Tombstoned { superseded_at_lsn: u64 },
    Unknown,
    IdentityMismatch,
}

pub trait RuntimeGenerationFence: Send + Sync {
    fn classify(
        &self,
        domain: &AuthorityDomainId,
        target: &TargetScope,
    ) -> Result<RuntimeGenerationDisposition, SessionError>;
}
```

A stale event remains durable audit/reconciliation evidence but cannot mutate the current session, a command, an Elicitation, transcript projection, or authority. `stale_event` is not a new session state. Unknown/malformed target identity fails closed rather than being normalized to current.

## Acceptance evidence

- [ ] A generation-N result arriving after N+1 is current produces `stale_event` evidence and no command transition, even when it exactly matches the original command target.
- [ ] Stale delivery acknowledgements, transcript events, session reports, Elicitation responses, and generic status/delta events are equally inert.
- [ ] A current-generation event still follows source-order, target, command-terminal, and authority checks; the fence does not bypass those boundaries.
- [ ] A replaced adapter token/stream epoch and an old runtime-generation token each independently reject the event.
- [ ] Replay and snapshots retain the tombstone and cannot resurrect stale event effects.
- [ ] An enumerate-first test fails if any runtime-targeted adapter ingress omits the shared fence.
- [ ] A mutation that accepts the old generation fails the v1 `LateGenerationInert` model/vector evidence.

## Ordering constraint

Depends on atomic generation advance/tombstones. Restart orchestration must not ship until this ingress inventory is complete.
