---
id: capability-manifest-durability-and-reconciliation-depth-contract-validation
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: capability-manifest-durability-and-reconciliation-depth
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
research_refs: [v1-control-plane-and-spawn]
created: 2026-08-15
updated: 2026-08-15
---

# Generated assurance contract, complete-manifest validation, and vectors

## Checkpoint

Establish the sole generated durability/reconciliation registry before any adapter profile consumes it. Fresh attach and capability redeclaration require one complete `AdapterAssuranceManifest.v1`; uncertain capability values are explicit false/none. Unknown/unspecified values, absent fields, unknown contract versions, and simultaneous replay-only/current dedup declarations fail before durable registration.

Preserve already-written v0.2 adapter registrations through a conservative **Replay-only** normalization. Do not expose the compatibility rule to fresh adapters.

Grounding: Mission Control separates a complete declared manifest from runtime detection and defaults uncertainty false, while its unprovable accepted dispatch requires manual reconciliation. `[mission-control-src]{9}` `[mission-control-src]{3}` See `.research/analysis/campaigns/v1-control-plane-and-spawn/specialists/peer-protocol-deep-dive.md` and `.research/attestation/mission-control-src.md` passages 9 and 3.

## Design

### Files

- `contracts/proto/patchbay/adapter.proto` — `AdapterAssuranceManifest`, frozen `AdapterAssuranceManifestV1`, `AdapterReconciliationStrength`, and `ReconciliationAction`; add the assurance field to `AdapterCapability`; retain tag 7 idempotency only as a deprecated replay input.
- Generated `contracts/rust/src/gen/patchbay/patchbay.rs` and `contracts/ts/src/gen/patchbay/adapter_pb.ts` — regenerate through Buf; never hand-edit.
- `core/src/adapter/capability.rs` — `ValidatedAdapterAssurance`, complete V1 validation, exact enum parsing, explicit boolean presence, fresh/replay split, and canonical `to_wire_v1` projection.
- `core/src/adapter/mod.rs` — expose the validated assurance type/accessor and keep attach/replay context selection inside trusted call sites.
- `core/tests/adapter_capability.rs` — boundary matrix, replay migration, redeclaration, and no-mutation evidence.
- `server/src/adapter_service/tests.rs` — invalid fresh manifests do not append registration or publish an attachment token.
- `contracts/vectors/adapter-assurance-complete-manifest.json` — exact implementation-checked completeness/conservative-replay example under a new stated-normative `AdapterCapabilityAssuranceHonesty` property id.
- `contracts/vectors/adapter-assurance-advisory-only.json` — authority/delivery regression example tracing existing `GrantAuthorityIsOperationKinds`.
- Conformance runner/traceability registration and rolling `docs/VERIFICATION.md` property/evidence text.

### Contract

```proto
message AdapterAssuranceManifest {
  oneof contract {
    AdapterAssuranceManifestV1 v1 = 1;
  }
}

message AdapterAssuranceManifestV1 {
  IdempotencyStrength deduplication_strength = 1;
  optional bool continuation_proof_support = 2;
  optional bool cursor_support = 3;
  optional bool generation_fence_support = 4;
  AdapterReconciliationStrength reconciliation_strength = 5;
  ReconciliationAction unproven_outcome_action = 6;
}
```

`AdapterReconciliationStrength` admits only `NONE`, `BOUNDED`, and `AUTHORITATIVE` after the required sentinel. `ReconciliationAction` admits only `NONE` and `MANUAL_REQUIRED` after the required sentinel. Unknown outcome itself remains the existing `SubmissionOutcome.UNKNOWN` or `FailureCode.EXECUTION_OUTCOME_UNKNOWN`; this story adds no competing terminal state or failure.

### Validation interface

```rust
impl ValidatedAdapterAssurance {
    fn try_from_wire(
        capability: &AdapterCapability,
        context: CapabilityValidationContext,
    ) -> Result<Self, CapabilityValidationError>;

    fn to_wire_v1(self) -> AdapterAssuranceManifest;
}

impl ValidatedAdapterCapability {
    pub fn assurance(&self) -> ValidatedAdapterAssurance;
}
```

Fresh `Attach` requires V1, explicit presence for all three booleans, known/non-sentinel enums, and legacy tag 7 unset. Replay with current V1 is equally strict. Replay without V1 maps recognized legacy dedup to the canonical value and every other dimension to false/none; fresh ingress never uses this path.

The V1 branch is frozen. New semantic fields require V2. A current decoder that cannot select a future oneof branch rejects the missing admitted contract instead of accepting a partial V1. Unknown numeric values in generated enum registries fail closed; no catch-all/default-success branch is permitted.

## Acceptance evidence

- [ ] Buf generation changes Rust and TypeScript from the same `.proto`; generated drift is green and no generated file is edited manually.
- [ ] A fresh manifest with V1 and `Some(false)` for every uncertain boolean validates; omission of V1 or any boolean rejects.
- [ ] Every unknown or `UNSPECIFIED` assurance enum rejects; an unknown/future assurance branch is not treated as V1.
- [ ] Fresh dual declaration through deprecated tag 7 plus V1 rejects, while tag 7 remains a replay-only migration input.
- [ ] Each historical legacy dedup value replays into exactly one canonical V1 assurance; absent/unspecified legacy strength and all new fields normalize conservatively.
- [ ] Missing V1 under `CapabilityValidationContext::Attach` rejects even when the same bytes qualify for historical Replay normalization.
- [ ] Invalid fresh attach/redeclaration writes no adapter-registration Observation and publishes no replacement token.
- [ ] Touched manifest enum/set registries reject unknown/unspecified/applicable duplicate values rather than silently defaulting.
- [ ] `adapter-assurance-complete-manifest` runs an exact Rust implementation check and is described as implementation-checked/stated-normative, not model-promoted.
- [ ] `adapter-assurance-advisory-only` proves maximal declaration cannot bypass a missing grant and conservative declaration cannot replace adapter-authoritative delivery behavior.
- [ ] Mutations accepting omitted false, coercing unknown values, applying Replay normalization at Attach, or allowing capability-derived authority fail the vector/test oracle.

## Ordering constraint

No sibling dependency. This checkpoint must complete before `capability-manifest-durability-and-reconciliation-depth-consumer-wiring` and before the Pi manifest/profile consumer can implement its generic assurance fields.
