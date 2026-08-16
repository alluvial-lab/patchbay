---
id: capability-manifest-durability-and-reconciliation-depth-contract-validation
kind: story
stage: review
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

- [x] Buf generation changes Rust and TypeScript from the same `.proto`; generated drift is green and no generated file is edited manually.
- [x] A fresh manifest with V1 and `Some(false)` for every uncertain boolean validates; omission of V1 or any boolean rejects.
- [x] Every unknown or `UNSPECIFIED` assurance enum rejects; an unknown/future assurance branch is not treated as V1.
- [x] Fresh dual declaration through deprecated tag 7 plus V1 rejects, while tag 7 remains a replay-only migration input.
- [x] Each historical legacy dedup value replays into exactly one canonical V1 assurance; absent/unspecified legacy strength and all new fields normalize conservatively.
- [x] Missing V1 under `CapabilityValidationContext::Attach` rejects even when the same bytes qualify for historical Replay normalization.
- [x] Invalid fresh attach/redeclaration writes no adapter-registration Observation and publishes no replacement token.
- [x] Touched manifest enum/set registries reject unknown/unspecified/applicable duplicate values rather than silently defaulting.
- [x] `adapter-assurance-complete-manifest` runs an exact Rust implementation check and is described as implementation-checked/stated-normative, not model-promoted.
- [x] `adapter-assurance-advisory-only` proves maximal declaration cannot bypass a missing grant and conservative declaration cannot replace adapter-authoritative delivery behavior.
- [x] Mutations accepting omitted false, coercing unknown values, applying Replay normalization at Attach, or allowing capability-derived authority fail the vector/test oracle.

## Ordering constraint

No sibling dependency. This checkpoint must complete before `capability-manifest-durability-and-reconciliation-depth-consumer-wiring` and before the Pi manifest/profile consumer can implement its generic assurance fields.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; caller-selected for the generated-contract, durable-replay, and authority-sensitive validation leaf. Review weight: `thorough`; implementation intentionally stops at `stage: review` for the independent review.
- Contract and mechanism: `adapter.proto` now owns the frozen, versioned assurance registry and deprecates tag 7 as replay-only input. `ValidatedAdapterAssurance` parses complete current V1 declarations, requires explicit optional-boolean presence and known non-sentinel enums, rejects legacy/current dual declarations, and emits one canonical V1 projection. Only trusted adapter-log replay call sites select conservative legacy normalization; fresh attach and redeclaration select strict validation before append/token publication.
- Registry hardening: the touched `supported_operation_kinds` and `known_failure_modes` generated-enum sets now reject unknown, `UNSPECIFIED`, and duplicate members. Existing target/resource/schema validation remains unchanged.
- Generated and compatibility surfaces: Buf generated Rust and TypeScript bindings from the same proto. Rust server/core fixtures were migrated to complete V1. The Pi constructor was minimally migrated to its already-designed conservative current declaration (`AT_PATCHBAY_BOUNDARY`, false/false/false, `NONE`, `MANUAL_REQUIRED`) because strict attach otherwise makes the mandated real-core Pi E2E invalid; focused Pi manifest tests, diagnostics carriage, token-commune migration, and presentation remain owned by the dependent consumer-wiring story.
- Conformance evidence: added promoted implementation-checked vectors `adapter-assurance-complete-manifest` (`AdapterCapabilityAssuranceHonesty`, stated-normative/reserved-unmodeled) and `adapter-assurance-advisory-only` (`GrantAuthorityIsOperationKinds`). The Rust runner exercises the production validator, canonical replay projection, real authority submission path, and grant-authorized delivery admission; traceability and `docs/VERIFICATION.md` were regenerated/updated without claiming model promotion.
- Tests: boundary matrices cover complete explicit-false V1, each missing boolean, every unknown/sentinel assurance enum, missing/unknown version, dual declaration, all historical legacy values, conservative unknown legacy normalization, attach-only exclusion, and touched enum-set invalidity. Server tests prove rejected initial attach and newer-generation redeclaration append no registration, publish no replacement token, and leave the prior token current.
- Mutation kills: all injected mutants failed focused or vector oracles and were restored with exact baseline hashes: omitted uncertainty inferred/accepted; incomplete fresh manifest accepted; unknown version accepted; dual legacy/current declaration accepted; Replay normalization admitted at Attach; and capability-derived authority manufactured for the maximal declaration. Restored focused tests and vector checks pass.
- Discrepancies from design: only the minimal Pi producer migration noted above moved forward from child 2 to preserve the caller-mandated group-4 E2E under strict fresh validation. No diagnostics schema/presentation consumer, token-commune producer, or positive Pi capability activation moved into this leaf. Adjacent issues parked: none.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**; all workspace targets/tests/doctests pass with warnings denied.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 59 vectors, 19 promoted vectors, 28 implementation checks, and 38 registered mutation witnesses; generated drift and traceability are current.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 27/27 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 60/60 tests including the real core/adapter generation-bump, reconnect, and restart E2E.
