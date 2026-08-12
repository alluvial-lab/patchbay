---
id: research-handoff-spawn-runtime-evidence-promotion-contract
kind: story
stage: implementing
tags: [protocol, security, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-identity-contract, research-handoff-spawn-continuation-payload-authority-contract, research-handoff-spawn-claim-registry-contract, research-handoff-spawn-crash-external-effect-evidence-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Runtime evidence quarantine and atomic promotion contract

## Checkpoint

Define the durable replay envelopes that make claimed-successor staging, stale-evidence quarantine, and authority-bearing promotion structural. This is the last contract leaf; no spawn operation begins until it and the cursor leaf are complete.

`ClaimedSuccessor` validates exact Operation provenance + expected prior + claimed generation. It routes only a SessionReport to staged evidence; it does not make the successor current. Stale/unknown/mismatched candidates persist, when useful, only inside `QuarantinedRuntimeEvidence`, never as raw authoritative Observations.

`SpawnPromotionCommitted` is one semantic event consumed by session, claim, authority, and command projections. It contains or references complete success evidence and the descendant Grant. Its source and audit commit atomically; replay cannot observe a candidate-live/grantless prefix.

## Design

**Files**
- `contracts/proto/patchbay/{common,observations,sessions,authority}.proto` — classifier and self-contained event envelopes; generated Rust/TypeScript artifacts follow from the schema.
- `contracts/proto/patchbay/common.proto` stored-event registry — dedicated staged/quarantined/promotion kinds.
- `core/src/storage/port.rs` plus backend contract tests — atomic audited promotion append that stamps exact event/audit linkage in one transaction.
- Replay-dispatch and generated contract tests.

```rust
pub enum RuntimeGenerationDisposition {
    Current,
    ClaimedSuccessor {
        claim_operation_id: CommandId,
        expected_prior: Option<RuntimeGenerationRef>,
        claimed_generation: Generation,
    },
    Tombstoned { superseded_at_lsn: u64 },
    Unknown,
    IdentityMismatch,
}
```

The quarantine envelope uses a generated `oneof` for each admitted candidate family (Observation, SessionReport, delivery acknowledgement, transcript/status evidence, Elicitation mutation), plus authenticated source binding, candidate target, classifier context, and canonical reason. It is not an arbitrary payload/schema escape hatch. Normal projections switch on the outer stored kind and cannot unwrap quarantine as normal authoritative ingress.

Promotion readiness requires accepted compound provenance, delivered/running lifecycle, successful Result, staged successor, exact external-runtime uniqueness reservation, exact generation transition, live-at-promotion exact-prior replacement Grant for continuation, completion audit linkage, and descendant Grant. Promotion installs authority before session current/live and command completed within the event fold order.

## Acceptance evidence

- [ ] Legitimate first fresh/N+1 reports classify `ClaimedSuccessor` only on exact durable claim provenance.
- [ ] Wrong Operation, expected prior, adapter/deployment, logical id, runtime id, or generation never stages.
- [ ] Every quarantined candidate is carried by an admitted generated `oneof`; unknown/untyped candidates reject, and nested Observation/report/transcript/ack/Elicitation evidence cannot mutate any normal projection on hot fold or replay.
- [ ] Promotion source is a self-contained replay unit carrying the complete descendant Grant; its distinct audit record commits in the same storage transaction, and no crash prefix publishes N+1 without authority.
- [ ] Every consuming projection validates the same promotion event and exact pre-state; disagreement fails closed.
- [ ] Revoked/expired exact-prior replacement authority before promotion suppresses promotion.
- [ ] Mutations dispatching quarantine as raw evidence or exposing staged successor as current fail.

## Ordering constraint

Final invariant contract leaf. Target resolution waits on this leaf and the parallel cursor replacement leaf, ensuring all operations consume settled shared contracts.
