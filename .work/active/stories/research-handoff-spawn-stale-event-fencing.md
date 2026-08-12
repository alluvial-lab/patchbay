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

# Shared runtime-generation fence and durable evidence quarantine

## Redesign disposition

Rewritten. The old design appended a raw Observation plus separate stale audit and lacked `ClaimedSuccessor`. Both shapes are superseded.

## Checkpoint

Route every runtime-targeted adapter ingress through one reconciled classifier: SessionReport, Observation/Result, delivery acknowledgement, transcript/status/delta, and Elicitation mutation. `Current` continues through normal checks. `ClaimedSuccessor` is valid only for exact managed successor SessionReport staging. `Tombstoned`, `Unknown`, and `IdentityMismatch` reject or persist a self-contained `QuarantinedRuntimeEvidence` outer event with an atomic audit.

No normal projection may dispatch the nested quarantined candidate as an Observation or other authoritative event during hot fold or replay.

## Design

**Files**
- `core/src/session/logical_target.rs` — shared classifier implementation.
- `core/src/acceptance/{ports,observation,elicitation}.rs`, `core/src/adapter/mod.rs` — consumer-owned fence port at every ingress.
- `server/src/adapter_service.rs` — authenticated source + gate catch-up + atomic quarantine/audit append.
- Transcript/ack/Elicitation paths and enumerate-first tests.

```rust
pub trait RuntimeGenerationFence: Send + Sync {
    fn classify(
        &self,
        domain: &AuthorityDomainId,
        candidate: &RuntimeEvidenceCandidate,
    ) -> Result<RuntimeGenerationDisposition, SessionError>;
}
```

Quarantine carries the original typed candidate, exact classified/current/claim context, attachment source, and canonical reason. It is durable diagnostics/audit evidence only. It never becomes a command transition, completion fact, transcript entry, Elicitation response, session mutation, or authority input.

## Acceptance evidence

- [ ] Legitimate first successor report stages through `ClaimedSuccessor` without a bypass.
- [ ] Generation-N evidence after N+1 promotion yields quarantine/stale audit and no live mutation.
- [ ] Stale ack/result/transcript/status/Elicitation/report candidates are equally inert.
- [ ] Hot fold and replay see only the outer quarantine kind; a nested raw Observation cannot reach normal projections.
- [ ] Current evidence still obeys source order, terminal finality, correlation, and authority; classifier success bypasses none.
- [ ] Replaced adapter token/epoch and runtime generation independently fence evidence.
- [ ] Enumeration test fails when any runtime ingress lacks the port or dispatches quarantine incorrectly.

## Ordering constraint

Depends on the exact promotion/tombstone projection. Completion is blocked until the ingress inventory is complete.
