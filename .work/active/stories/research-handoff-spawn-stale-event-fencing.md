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
updated: 2026-08-15
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

- [x] Legitimate first successor report stages through `ClaimedSuccessor` without a bypass.
- [x] Generation-N evidence after N+1 promotion yields quarantine/stale audit and no live mutation.
- [x] Stale ack/result/transcript/status/Elicitation/report candidates are equally inert.
- [x] Hot fold and replay see only the outer quarantine kind; a nested raw Observation cannot reach normal projections.
- [x] Current evidence still obeys source order, terminal finality, correlation, and authority; classifier success bypasses none.
- [x] Replaced adapter token/epoch and runtime generation independently fence evidence.
- [x] Enumeration test fails when any runtime ingress lacks the port or dispatches quarantine incorrectly.

## Unit 5 implementation evidence

- Added the consumer-owned `RuntimeGenerationFence` over the generated quarantine-candidate oneof and one reconciled implementation that delegates to the existing SessionReport and runtime-target classifiers. The boundary rejects `ClaimedSuccessor` for every non-SessionReport candidate even if a faulty fence returns it.
- Routed authenticated SessionReport, delivery acknowledgement, Result, transcript Event, Status, Delta, and protobuf Elicitation ingress through that fence before their ordinary validators or writers. Current evidence still takes those existing paths; non-current evidence takes one typed atomic quarantine/audit writer.
- Added normal Elicitation ingestion and replay projection support so stale Elicitation mutations have useful pre-state and cannot mutate it from inside a quarantine envelope.
- Removed the former unclaimed-discovery `Unknown` exception: a current authenticated attachment's first valid SessionReport is classified `Current`, while unrelated unknown runtime evidence remains fenced.
- Added generated-contract inventory oracles for `ObservationRequest`, every `ObservationKind`, and every `QuarantinedRuntimeEvidence.candidate` arm. A new generated family now requires an explicit routing decision.
- Added all-family integration and projection tests proving outer-only durability plus hot-fold/replay inertness across session, command, Elicitation, authority, diagnostics, and adapter projections. Added an independent attachment-epoch versus runtime-generation fence test.
- Killed all required local mutants: non-report `ClaimedSuccessor` bypass; quarantine writer using raw `Observation` outer kind; and command projection recursively dispatching nested quarantined Observation/transcript payloads.
- Verification passed: workspace tests; targeted stale-generation/replay suites; workspace clippy with warnings denied; formatting and generated-contract drift check. No protocol or generated contract changed.

## Ordering constraint

Depends on the exact promotion/tombstone projection. Completion is blocked until the ingress inventory is complete.
