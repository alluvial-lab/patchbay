---
id: capability-manifest-durability-and-reconciliation-depth
kind: feature
stage: done
tags: [adapter, architecture, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
research_refs: [v1-control-plane-and-spawn]
created: 2026-08-12
updated: 2026-08-15
---

# Capability manifest: declared durability + reconciliation depth

## Origin and grounding

This feature harvests Mission Control directions **1 + 4** as adapter-neutral design direction, not code reuse:

- `.research/analysis/campaigns/v1-control-plane-and-spawn/specialists/peer-protocol-deep-dive.md` § “Mission Control architectural direction to harvest” separates complete declared capability depth from runtime detection and requires uncertain capabilities to be false; its `manifest-overclaim` vector requires a shipping/conformance path for every advertised assurance. `[mission-control-src]{9}`
- The same facet records accepted work without an external run id as `manual_required` because the substrate cannot safely prove completion. Patchbay must preserve that uncertainty rather than infer success from acceptance. `[mission-control-src]{3}`
- `.research/attestation/mission-control-src.md` passages 3 and 9 attest those claims against the pinned MIT repository at commit `17186288ef28341723999a040b3b7baa55427a2c`.
- `.work/active/stories/mc-architectural-harvest.md` directions 1 and 4 perform the operator-confirmed research handoff.

The shipped v0.2.0 baseline is `contracts/proto/patchbay/adapter.proto`, `core/src/adapter/capability.rs`, and `core/tests/adapter_capability.rs`: target category, session/per-resource snapshot tier, and projection-schema declarations are validated at attach/replay. `docs/ARCHITECTURE.md` and `docs/PROTOCOL.md` make the manifest advisory; grants remain authority and the adapter's delivery result remains authoritative for support/outcome.

The immediate consumer is `.work/active/features/research-handoff-pi-adapter-capability.md` §7 and its `research-handoff-pi-adapter-capability-manifest-profile` child. The generic registry below owns the durability/reconciliation dimensions. Pi may carry one bounded opaque generated profile for Pi-only facts, but it must import this registry and keep every unproved field false/none until its lifecycle conformance checkpoint activates the claim.

**Dispatch rationale:** direct-read design. The feature is bounded to one generated contract, its existing Rust attach validator, diagnostics consumers, and two adapter manifest constructors. The research campaign already passed full groundedness gates; no exploratory fan-out would create a distinct unknown. The forced-adversary pre-mortem below was run inline as requested.

## Direction and non-goal

Extend the generic capability manifest with a **complete, versioned assurance declaration** for:

1. deduplication strength;
2. continuation-proof support;
3. external cursor support;
4. generation-fence support;
5. reconciliation strength; and
6. the action required when authoritative outcome evidence is unavailable.

These are static, adapter-declared guarantees. They are deliberately separate from `AdapterDiagnosticState`, attachment-token liveness, delivery-subscription reachability, session connectivity, and future heartbeat/installed-runtime detection. Attach success never turns an assurance field true.

**Non-goal / invariant:** capability declarations do not authorize an Operation, suppress grant checks, terminalize an Operation, prove external execution, or override an adapter-authoritative delivery/result Observation. They remain an advisory diagnostics/presentation layer exactly as in v0.2.0.

## Work-nature test

**Non-zero design surface; full feature-design lane.** This feature chooses generated enum/field shape, completeness and legacy-replay semantics, unknown-version handling, reconciliation vocabulary, attach/redeclaration failure behavior, diagnostics carriage, and the Pi consumer seam. Those choices affect a public adapter contract and retry/outcome honesty.

No UI mock is needed. The new data appears through the existing `adapter-status` diagnostics and existing retry/unknown presentation. It adds no screen, journey, protocol state, CSS state, or adapter-provided renderer.

## Design decisions

- **One canonical generated assurance registry:** add a versioned `AdapterAssuranceManifest` beneath `AdapterCapability`; Rust validated types, TypeScript adapter constructors, diagnostics, CLI/web presentation, and conformance vectors consume it. No Rust/TypeScript string union or second field list is allowed.
- **Move dedup into the registry without losing durable replay:** retain tag 7 `idempotency_strength` only as a deprecated replay input. Fresh attach/redeclaration requires it to be `UNSPECIFIED` and uses `assurance.v1.deduplication_strength`. Replay of a pre-feature registration normalizes the legacy value into the V1 validated view. This keeps one current semantic source while preserving already-written v0.2 registration events.
- **Presence-bearing conservative booleans:** the three binary dimensions are `optional bool` inside V1. Fresh manifests must encode each field, including explicit `false`; omission is invalid rather than silently interpreted as support. When evidence is uncertain, producers set `false`.
- **Three reconciliation strengths:** `none`, `bounded`, and `authoritative`. The adapter-wide value is the conservative minimum across its declared supported Operations/targets. Per-Operation/per-target overrides are reserved rather than invented now.
- **Reuse Patchbay's existing unknown terminals:** do not add `CommandState = unknown`, a second `SubmissionOutcome.UNKNOWN`, or an adapter-specific failure. Pre-acceptance uncertainty remains `SubmissionOutcome.UNKNOWN`; accepted external ambiguity remains `CommandState.FAILED` with `FailureCode.EXECUTION_OUTCOME_UNKNOWN`. The assurance registry adds only a closed generated `ReconciliationAction` (`none` / `manual_required`) that qualifies what the operator must do after that canonical unknown. Presentation therefore has exactly `unknown` or `manual-required`, derived without duplicating the unknown terminal.
- **Versioned fail-closed evolution:** `AdapterAssuranceManifest` is a oneof whose only admitted branch is frozen `v1`. A missing branch, unknown future branch as observed by the current generated decoder, unknown enum numeric, `UNSPECIFIED`, duplicate registry member, or deprecated/current dual declaration rejects fresh attach before durable registration. Future semantic fields require a new manifest branch; they may not be smuggled into V1 and defaulted by an old core.
- **Compatibility is replay-only:** conservative legacy normalization is available only in `CapabilityValidationContext::Replay`. It is never applied to fresh attach, same-generation redeclaration, or newer-generation replacement.
- **Truth is conformance-backed, not self-attested at runtime:** generic core validation proves shape and conservative completeness, not whether an adapter is honest. A `true`/non-`none` declaration must be exercised by the adapter's promoted implementation vector. Pi's final lifecycle checkpoint is the activation gate for its stronger values.

## Architectural options

### Option A — Add ordinary top-level booleans

Append three proto3 booleans and one enum directly to `AdapterCapability`, retaining the existing top-level idempotency field. This is the smallest diff, but omitted and explicit-false booleans are indistinguishable, future additions can silently default, and the dimensions remain easy to copy into parallel DTOs. **Rejected:** it cannot enforce the complete-manifest requirement.

### Option B — Use an open map/repeated key-value capability bag

Represent dimensions as string keys or a repeated generic value union. This makes extension easy but moves names and type checking out of the generated contract, invites duplicate keys, and requires every consumer to recreate dispatch. **Rejected:** it violates generated contracts, fail-fast boundaries, and the single-source registry principle.

### Option C — Versioned generated assurance block with explicit scalar presence

Add one frozen V1 message under a oneof, require all V1 fields at fresh attach, normalize only historical replay, and expose the same generated message through diagnostics. This adds one wrapper but makes completeness, unknown-version failure, compatibility, and future V2 promotion explicit. **Chosen.**

## Canonical dimension registry

`contracts/proto/patchbay/adapter.proto` owns the wire shape:

```proto
message AdapterCapability {
  repeated OperationKind supported_operation_kinds = 1;
  repeated string supported_target_spec_shapes = 2;
  bool streaming_support = 3;
  AdapterSnapshotSupport session_snapshot_support = 4;
  bool cancellation_support = 5;
  bool session_replacement_support = 6;

  // Replay-only input for durable pre-assurance registrations. Fresh attach
  // requires UNSPECIFIED and uses assurance.v1.deduplication_strength.
  IdempotencyStrength idempotency_strength = 7 [deprecated = true];

  AttachmentMethod attachment_method = 8;
  repeated FailureCode known_failure_modes = 9;
  AdapterDiagnosticReportingCapability diagnostic_reporting = 10;
  repeated AdapterTargetCategory target_categories = 11;
  repeated ResourceCapability resource_capabilities = 12;
  AdapterAssuranceManifest assurance = 13;
}

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

enum AdapterReconciliationStrength {
  ADAPTER_RECONCILIATION_STRENGTH_UNSPECIFIED = 0;
  ADAPTER_RECONCILIATION_STRENGTH_NONE = 1;
  ADAPTER_RECONCILIATION_STRENGTH_BOUNDED = 2;
  ADAPTER_RECONCILIATION_STRENGTH_AUTHORITATIVE = 3;
}

enum ReconciliationAction {
  RECONCILIATION_ACTION_UNSPECIFIED = 0;
  RECONCILIATION_ACTION_NONE = 1;
  RECONCILIATION_ACTION_MANUAL_REQUIRED = 2;
}
```

### Normative field semantics

| Field | Meaning | Conservative value / proof threshold |
|---|---|---|
| `deduplication_strength` | Existing `IdempotencyStrength` semantics: `none`, Patchbay-boundary-only, or end-to-end external deduplication. | `NONE`. `END_TO_END` requires adapter conformance across response loss/restart; Patchbay core dedup alone proves only `AT_PATCHBAY_BOUNDARY`. |
| `continuation_proof_support` | The adapter can produce current external evidence for the declared continuation context and bind it to the exact prior/successor contract. It does not promise arbitrary process-memory restoration. | `false`. Session replacement, resume flags, or a reused id alone do not prove it. |
| `cursor_support` | The adapter has an external continuity cursor or exact-set authoritative replacement path whose scope and commit rules are conformance-tested. A core LSN or remembered stream position does not count. | `false`. Partial/full fetch without omission-removing replacement does not count. |
| `generation_fence_support` | The adapter/external-runtime seam prevents an old runtime incarnation from mutating or acknowledging the current generation. Core-side stale-event rejection alone does not let an adapter claim this end-to-end dimension. | `false`. Reporting a generation field without enforcing it does not count. |
| `reconciliation_strength` | Conservative minimum across declared supported behavior: `NONE` has no automatic external outcome proof; `BOUNDED` proves only a declared subset/window; `AUTHORITATIVE` can query/rebuild authoritative outcome for every declared externally-effecting path inside its stated retention scope. | `NONE`. A partial snapshot or reachability probe is not reconciliation authority. |
| `unproven_outcome_action` | Qualifies the canonical unknown result when evidence is unavailable: `NONE` leaves the result visibly unknown; `MANUAL_REQUIRED` adds the closed operator-action label `manual-required`. | `NONE`. It never converts unknown into success/failure and never creates a new terminal state. |

The V1 message is the sole current registry. Its field order is not priority, and no consumer may infer one field from another. In particular, `session_replacement_support=true` does not imply continuation proof; snapshot support does not imply cursor support; adapter generation fields do not imply an external generation fence; and an attached/reachable adapter does not imply any durability strength.

## Validation and compatibility semantics

### Fresh attach and redeclaration

`ValidatedAdapterCapability::try_from_wire(..., CapabilityValidationContext::Attach)` must complete before registration append or replacement-token publication:

1. Require `capability.assurance.contract = v1`.
2. Require legacy `capability.idempotency_strength = UNSPECIFIED`; a dual legacy/current declaration is ambiguous and rejects.
3. Parse `deduplication_strength`, `reconciliation_strength`, and `unproven_outcome_action` through generated enums; reject unknown numeric and `UNSPECIFIED`.
4. Require `continuation_proof_support`, `cursor_support`, and `generation_fence_support` to be present. `Some(false)` is complete; `None` is not.
5. Apply the same generated-enum discipline to all touched manifest registries: unknown/`UNSPECIFIED` values reject when the field is applicable, and duplicate set-like values reject. No default-success or catch-all branch is permitted.
6. Validate all target/resource/schema relationships already enforced by `capability.rs`.
7. Return one canonical `ValidatedAdapterAssurance`; attach/redeclaration persists only after the whole manifest is valid.

A validation failure is an invalid-argument/registration rejection before a durable adapter-registration Observation. It is not `unsupported_command`, because no accepted Operation has reached delivery.

### Durable replay

`CapabilityValidationContext::Replay` preserves v0.2 durable registration data without making compatibility a fresh-ingress bypass:

- if `assurance.v1` exists, apply the exact current validator and require legacy tag 7 to be `UNSPECIFIED`;
- if assurance is absent, accept only the existing replay compatibility path and normalize a recognized legacy `idempotency_strength` to the canonical dedup value; unknown/`UNSPECIFIED` becomes `NONE`; all three booleans become `false`, reconciliation becomes `NONE`, and action becomes `NONE`;
- canonical diagnostics and downstream decisions read the validated normalized view, not the raw replay-only field;
- an unknown/invalid current V1 record is corruption, not a candidate for conservative normalization.

This is a one-way compatibility reader for substantial durable data. Fresh adapters must redeclare V1 on their next attachment.

### Unknown-field policy

V1 is frozen. Future semantic capability fields must land in a new `AdapterAssuranceManifest` oneof branch and a protocol/registry update. A current decoder sees an unknown future branch as no admitted contract and rejects attach; it never treats the future field as false while accepting the rest. Unknown enum numerics and unknown current registry members reject explicitly. Ordinary protobuf unknown bytes that have no admitted generated V1 meaning cannot establish a capability and must not be consulted by any consumer.

## Reconciliation outcome mapping

The assurance declaration does not add a competing lifecycle:

| Boundary | Canonical outcome | `ReconciliationAction` presentation |
|---|---|---|
| Core acceptance cannot be determined | existing `SubmissionOutcome.UNKNOWN` | `NONE` → `unknown`; `MANUAL_REQUIRED` → `manual-required` when adapter reconciliation is the limiting factor |
| Operation was accepted but external execution cannot be proved | existing terminal `CommandState.FAILED` + `FailureCode.EXECUTION_OUTCOME_UNKNOWN` | `NONE` → `unknown`; `MANUAL_REQUIRED` → `manual-required` |
| Reconciliation proves an outcome | existing adapter-authoritative delivery/result transition | no unknown/manual qualifier; manifest never supplies the transition |

`manual-required` is a generated action qualifier in diagnostics/presentation, not a `CommandState`, `SubmissionOutcome`, `FailureCode`, or proof that a human has reconciled anything. This preserves the closed vocabulary and avoids duplicating Patchbay's existing unknown terminals.

## v0.2.0 invariant guardrails

- Grant evaluation uses only canonical Patchbay `OperationKind`, subject, endpoint, target scope, expiry, and revocation facts. No assurance value enters grant matching.
- The core continues to deliver accepted Operations rather than gating on cached capabilities. A declared false/none value may hide or warn in UX, but the adapter's delivery response remains authoritative; unsupported behavior reports `unsupported_command` after acceptance.
- A declared true/authoritative value never terminalizes an Operation or proves an external effect. Only authenticated adapter evidence under the existing lifecycle may do that.
- Capability redeclaration is audited and atomically registered as today; changing assurance values cannot retroactively rewrite grants, Operations, results, or historical Observations.
- Runtime installed/reachable/liveness detection remains in attachment/session/diagnostic state. It is neither stored in nor inferred into the assurance registry.

## Trickiest unit first: complete declaration without a second semantic source

The highest-risk unit is migration from the existing top-level idempotency field to a complete V1 block while keeping durable v0.2 registrations replayable and preventing the replay exception from admitting incomplete fresh manifests.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedAdapterAssurance {
    deduplication_strength: IdempotencyStrength,
    continuation_proof_support: bool,
    cursor_support: bool,
    generation_fence_support: bool,
    reconciliation_strength: AdapterReconciliationStrength,
    unproven_outcome_action: ReconciliationAction,
}

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

The context is supplied only by trusted attach/replay call sites; it is not a wire field an adapter can select. `to_wire_v1` is the only diagnostics/migration projection and always emits every optional boolean as `Some(true|false)`.

## Implementation units and child checkpoints

### Unit 1 — Generated assurance contract, validator, and exact vectors

**Story:** `capability-manifest-durability-and-reconciliation-depth-contract-validation`

**Files:**

- `contracts/proto/patchbay/adapter.proto`
- generated `contracts/rust/src/gen/patchbay/patchbay.rs` and `contracts/ts/src/gen/patchbay/adapter_pb.ts` via Buf (never hand-edited)
- `core/src/adapter/capability.rs` and `core/src/adapter/mod.rs`
- `core/tests/adapter_capability.rs` and focused `server/src/adapter_service/tests.rs`
- `contracts/vectors/adapter-assurance-complete-manifest.json`
- `contracts/vectors/adapter-assurance-advisory-only.json`
- vector runner/traceability updates and `docs/VERIFICATION.md`

Acceptance requires complete fresh V1, conservative legacy replay, unknown/unspecified rejection, no durable append on invalid attach, and mutation-sensitive implementation checks for the two vectors. `AdapterCapabilityAssuranceHonesty` is recorded as implementation-checked/stated-normative rather than falsely model-promoted; the advisory-only vector traces the existing `GrantAuthorityIsOperationKinds` obligation.

### Unit 2 — Attach/diagnostics consumers and Pi-profile seam

**Story:** `capability-manifest-durability-and-reconciliation-depth-consumer-wiring`

**Files:**

- `contracts/proto/patchbay/diagnostics.proto`
- `core/src/diagnostics/mod.rs`
- `server/src/adapter_service/{mod.rs,tests.rs}` and relevant diagnostics/server tests
- `pi-adapter/src/core_client.ts` and focused manifest tests
- `token-commune-adapter/src/manifest.ts` and focused manifest tests
- `cli/src/commands/diagnostics.ts`, existing web retry/adapter-diagnostics consumers, and their tests
- rolling assertions in `docs/{ARCHITECTURE,PROTOCOL,VERIFICATION,UX,GLOSSARY}.md`; Pi-specific claims remain in the Pi feature/profile story

`AdapterCapabilitySummary` carries the generated canonical assurance message rather than re-listing six fields. Diagnostics are built from `ValidatedAdapterCapability` so legacy replay displays conservative normalized V1. Current Pi emits only evidence-backed values; continuation/cursor/generation strengths remain false/none until its final conformance checkpoint. The later Pi profile consumes this exact generated block and does not create a parallel Pi assurance registry.

## Implementation order

1. `capability-manifest-durability-and-reconciliation-depth-contract-validation` — establish the generated registry, normalization, validator, and vectors.
2. `capability-manifest-durability-and-reconciliation-depth-consumer-wiring` — depend on Unit 1; migrate diagnostics and current adapters, then expose the Pi-profile consumption seam.
3. The dependent Pi manifest/profile story imports the landed registry and activates stronger claims only after its own lifecycle conformance evidence.

## Simplification and cleanup

- Deprecate top-level `idempotency_strength` as replay-only input; all current consumers move to `assurance.v1.deduplication_strength`. Do not maintain both as live declarations.
- Reuse the generated assurance message in diagnostics instead of flattening six copied fields into `AdapterCapabilitySummary`.
- Keep one Rust validated value object and one conversion back to canonical V1. Eliminate ad hoc defaulting in Pi, token-commune, CLI, or web consumers.
- Do not add a runtime detection service, evidence database, capability grant, new Operation state, new failure code, or generic adapter profile parser.
- Existing duplicated fixture construction may use one test helper, but broader manifest-builder refactoring is outside this feature unless required to keep explicit-false test evidence clear.

## Testing and assurance

- **Generated-contract drift:** Buf generation updates Rust and TypeScript together; generated drift rejects hand-copy divergence.
- **Boundary matrix:** missing assurance, missing each optional false, unknown/unspecified enum, unknown version/oneof, dual legacy/current dedup, and invalid existing manifest registries all reject before registration append/token publication.
- **Conservative replay:** a pre-feature registration with each legacy idempotency value replays into exactly one canonical V1 view; absent/unspecified maps to none/false, and the same missing V1 shape fails under Attach.
- **Advisory invariant:** a manifest claiming every strength without a live grant still cannot accept an Operation; a manifest declaring none does not prevent delivery of an otherwise authorized Operation, whose adapter may still accept or return `unsupported_command`.
- **Diagnostics:** raw deprecated fields and future Pi profile bytes do not appear; safe V1 fields survive attach, replay, redeclaration, CLI JSON/human output, and web retry-safety consumption.
- **Adapter honesty:** Pi and token-commune constructor tests require every field explicitly. Pi cannot flip continuation/cursor/fence or reconciliation above none/bounded until the named Pi conformance runner passes.
- **Mutation witnesses:** accepting omitted booleans, coercing unknown enum/version to default, enabling replay normalization at Attach, deriving assurance from attachment/liveness, or letting capability bypass grant/delivery must fail a committed test/vector.
- **No new formal claim:** the completeness example is implementation-checked. The authority regression remains under `GrantAuthorityIsOperationKinds`; no abstract model is called checked merely because a vector passes.

## Adversarial pre-mortem and risks

### Forced adversary 1 — lying manifest overclaims a guarantee

**Attack:** an adapter sets end-to-end dedup, continuation, cursor, fence, and authoritative reconciliation true merely because its process is attached or its SDK exposes similarly named calls.

**Defense:** field semantics exclude those proxies; current attach proves only completeness. Each non-conservative adapter profile must have a promoted implementation vector exercising the claimed failure boundary, and Pi keeps claims false/none until final lifecycle conformance activates them. Diagnostics label them “declared,” never “verified live.”

**Residual risk/fallback:** a malicious authenticated adapter can still lie about external reality; this is already outside the core proof boundary. Revoke/detach it, degrade affected state, retain grants/outcomes as separate authority, and never let the declaration produce completion.

### Forced adversary 2 — silent uncertainty becomes support

**Attack:** omit a proto3 boolean or send an unknown future manifest version so an old core reads a default and accepts a seemingly complete declaration; a consumer later treats the default as true or “probably supported.”

**Defense:** V1 uses required-presence optional booleans, non-`UNSPECIFIED` enums, and a required admitted oneof branch. Every uncertain producer writes explicit false/none. Future semantic additions require V2; no admitted V1 default may stand for a new field.

**Residual risk/fallback:** legacy durable data lacks V1. Replay alone normalizes it to the all-conservative V1 view and requires fresh redeclaration before any stronger claim reappears.

### Forced adversary 3 — declaration replaces authority or outcome

**Attack:** cached capability says an Operation is supported/authoritative, so the core skips grant evaluation, pre-completes it, suppresses delivery, or rewrites `execution_outcome_unknown` to completed.

**Defense:** an executable advisory-only vector exercises both directions: maximal capability without grant still rejects pre-acceptance; conservative capability with a grant still reaches adapter delivery. Only existing authenticated delivery/result evidence may transition the Operation. Capability changes are non-retroactive.

**Residual risk/fallback:** if an implementation seam cannot keep this separation, stop implementation and extend the existing authority/delivery boundary test. Do not add a capability-derived authorization shortcut.

### Riskiest assumption

The riskiest assumption is that an adapter-wide conservative minimum is useful enough for v1 while some adapters have per-Operation differences. It is safe because it underclaims; it may reduce UX precision. The fallback is a future V2 per-Operation/per-target override registry, with V1 retained as the minimum and old cores rejecting the unknown V2 branch. Do not add an open map now.

## UI fallback / Mockups

No net-new screen or flow. The dimensions are safe diagnostics fields and retry/outcome qualifiers rendered through existing adapter-status and canonical unknown/failure presentation. No mockup is required.

## Extension pressure classification

- **Committed v1.0.0 contract direction:** one complete generated V1 assurance registry; deduplication, continuation proof, cursor, generation fence, reconciliation strength, and unproven-outcome action; explicit false/none under uncertainty; fresh-attach/redeclaration fail-fast validation; conservative replay-only v0.2 normalization; canonical diagnostics; and the v0.2.0 advisory-not-authority/delivery invariant.
- **Reserved seams:** V2 and later assurance branches; per-Operation/per-target/resource-kind strength overrides; cryptographic or signed capability attestations; richer automatic/manual reconciliation workflows; evidence retention windows; future heartbeat/installed-runtime detection (kept separate); other adapter-specific opaque profiles; and formal modeling if this static declaration later participates in a dynamic safety property.
- **Explicitly rejected for this feature/v1:** capability-derived grants or completion; capability-gated suppression of otherwise authorized delivery; inferring true from install/reachability, attachment, SDK method presence, snapshot support, or similarly named fields; omission/unknown-version defaulting; open string/map capability bags; Pi-specific assurance fields in core; a new unknown Operation/command state or failure code; and dynamic adapter-provided renderer/policy code.

The parked multi-human, desktop, mesh, and skin ideas remain pressure-test inputs only. This registry is actor-, authority-domain-, adapter-, and surface-neutral and adds no single-operator, single-surface, or Pi-only assumption.

## Child stories

- `capability-manifest-durability-and-reconciliation-depth-contract-validation` — no sibling dependency.
- `capability-manifest-durability-and-reconciliation-depth-consumer-wiring` — depends on `capability-manifest-durability-and-reconciliation-depth-contract-validation`.
