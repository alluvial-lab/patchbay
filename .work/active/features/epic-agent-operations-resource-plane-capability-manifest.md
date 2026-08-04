---
id: epic-agent-operations-resource-plane-capability-manifest
kind: feature
stage: done
tags: [foundation, protocol, adapter]
parent: epic-agent-operations-resource-plane
depends_on: [epic-agent-operations-resource-plane-resource-identity]
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-08-04
---

# Resource capability manifest & projection contract

## Brief

Extend the adapter capability manifest so an adapter can declare the resource
kinds it targets, the resource payload/projection schemas it emits, and
per-resource snapshot tiers — and define the executable projection-contract
boundary that `UX.md:52-62` specifies but does not mechanize: canonical
protocol/presentation primitives (delivery, reconciliation, stale-state,
authority, attention honesty) remain mandatory, while adapter-shaped domain
projections compose richer views above that floor.

Today the manifest (`contracts/proto/patchbay/adapter.proto`) has no target
categories, no resource kinds, no projection schema identifiers, and
`snapshot_support` is adapter-wide and session-termed — it cannot say which
resource collection is complete or which axes are partial. This feature adds
the resource-aware manifest fields and the target-category registry. The
registry must be **extensible** so the reserved OKF knowledge-bundle third
kind (see parent epic) is an additive future promotion, not a rearchitecture.

The manifest must support multiple resource kinds under the single admission
rule: pooled token-commune pools (adminable) and direct-provider usage windows
(read-only). It must not accidentally admit foreign data sources (the OKF
third kind) while staying honest about operational resources — that promotion
is reserved, with OKF v0.2 named as the candidate format.

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: contract foundation — depends on `resource-identity`; consumed by `cockpit-composition` (which renders the declared projections) and `conformance`.

## Simplification opportunity

- Extend the existing `AdapterCapability` rather than creating a separate resource-capability surface; one manifest, target-kind-discriminated fields.

## Foundation references

- `docs/ARCHITECTURE.md` — adapter registration/lifecycle, capability manifests
- `docs/UX.md:40-62` — the presentation conformance floor + adapter-shaped projections above it (the projection contract to mechanize)
- `docs/PROTOCOL.md:553-593` — capability declarations, snapshot tiers
- `contracts/proto/patchbay/adapter.proto:9-34` — current manifest + `snapshot_support` tiers
- `contracts/proto/patchbay/adapter_control.proto` — typed report ingress paths

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`
- No direct UI; the contract the cockpit feature composes against.

## Design decisions

- **Target category and resource kind remain different registries.** `AdapterTargetCategory` is a closed generated registry for Patchbay admission/composition (`runtime_session`, `operational_resource`, and reserved `knowledge_bundle`); `ResourceKind` remains the open adapter-owned identifier landed by `resource-identity`. This keeps provider/pool names out of core while preventing a knowledge bundle from entering through the operational-resource category by omission.
- **The OKF seam is wire-present but not admitted.** `ADAPTER_TARGET_CATEGORY_KNOWLEDGE_BUNDLE` is a reserved generated enum value with OKF v0.2 named as its candidate payload format. Registration rejects it until an explicit promotion adds its own capability/report/presentation contract; it is not accepted as an operational `ResourceCapability` and no current resolver handles it.
- **One declaration owns each resource kind.** `AdapterCapability.resource_capabilities` is keyed by exact `ResourceKind`; each declaration carries its snapshot tier and a projection contract. Duplicate kinds, missing operational-resource category, unspecified tiers, missing schema descriptors, category mismatch, unknown enum values, and the reserved knowledge-bundle category fail adapter registration before durable append.
- **The old snapshot field becomes explicitly session-scoped.** Protobuf tag 4 is renamed in place from `snapshot_support` to `session_snapshot_support`; resource tiers live only on their exact `ResourceCapability`. Patchbay owns all live callers, so generated Rust/TypeScript producers update together rather than retaining API aliases or fallback precedence. Existing durable registration events are a verified data consumer: replay alone may normalize a pre-category manifest to session-only when it contains no resource declarations; fresh attach always requires explicit categories, and the legacy path can never admit a resource or knowledge bundle.
- **The projection contract is structural and executable, not an adapter UI plugin.** `ResourceProjectionContract` names the mandatory `operational_resource` conformance target plus exact payload and domain-projection schema descriptors. A validated core API admits the exact `(adapter_id, resource_kind)` and checks report envelopes against those descriptors. It never loads adapter code, HTML, CSS, or policy; later surfaces use a local known renderer/decoder and nest adapter data beneath canonical identity/revision/staleness/authority/attention/Operation presentation.
- **Schema binding is not misrepresented as schema validation.** A `SchemaDescriptor` binds a non-empty bounded `schema_ref` and known `PayloadContentType`; exact descriptor matching rejects undeclared formats. It does not prove arbitrary bytes satisfy that schema. Typed decoder failure must remain fail-closed in `resource-state`/`cockpit-composition`, and the closing conformance feature owns executable malformed-payload vectors.
- **Capabilities remain advisory, not authority.** The manifest can declare that an adapter targets resources and which schemas/tiers it supports, but grants remain the only authority gate and delivery remains adapter-authoritative. Pool administration versus read-only provider windows is expressed by the existing supported canonical OperationKinds, actual grants, and delivery outcomes; this feature adds no `access_mode`, resource-kind wildcard grant, or capability-derived authority.
- **Autopilot rationale.** These choices reuse the typed resource identity and existing manifest/registration projection, make the reserved third category explicit, and choose the smallest fail-closed shape that does not require a dynamic UI/plugin system. No contradictory state or hard halt was found.

## Codebase mapping

Direct reading covered the generated adapter/common/diagnostics contracts, `AdapterRegistry` durable registration projection and validation, server attach ingress, diagnostic capability projection/CLI output, Pi's manifest producer and e2e assertions, the landed resource identity/registry/resolver, and the `resource-state`/cockpit/conformance sibling boundaries. Commits `c10cf5d`, `ed8b1d5`, `6e8d084`, and `577ef54` were checked against the current source: this design reuses `ResourceKind`, private validated `ResourceIdentity`, exact adapter routing, and the identity-only resource registration seam rather than defining another resource identity. The surface is cross-package but bounded to one generated contract and its existing projections, so direct-read mapping was preferable to duplicated exploratory fan-out.

## Architectural choice

### Options considered

1. **Generated target-category registry + per-resource declarations + validated projection contract (chosen).** Extend `AdapterCapability` with one closed category enum and exact open resource-kind declarations. Store a validated domain projection beside each adapter record and expose one admission/schema-binding API. This optimizes for registry ownership, fail-closed reserved values, and additive future promotion, at the cost of coordinated generated-contract and consumer updates.
2. **Open string categories and opaque schema-ref lists.** This is mechanically smallest and maximally open, but typo variants silently become categories, OKF can be mislabeled as a resource without a promotion point, and each consumer must rediscover the relation among kind, tier, and schema. It does not mechanize the UX contract.
3. **A generic adapter-supplied projection/plugin bundle.** Loading adapter-provided renderers or schemas could make arbitrary future categories flexible, but it creates a code-loading/security boundary, bypasses the shared presentation floor, and turns this feature into the explicitly deferred plugin marketplace. It is rejected for this arc.

The chosen approach makes `AdapterTargetCategory` the single source of truth for both admission and the projection conformance target. `ResourceKind` remains open beneath `operational_resource`, so pooled token-commune resources and direct-provider usage windows use the same rule without becoming core variants. The trickiest unit is **validated admission and projection binding**: it must reject reserved/contradictory declarations, survive durable replay, and provide the exact lookup consumed by resource ingress without turning a capability into authority. That unit follows the wire registry and precedes all diagnostics/producer/documentation integration.

## Implementation Units

### Unit 1: Generated target-category and resource projection contract

**Files**: `contracts/proto/patchbay/adapter.proto`, `contracts/rust/src/gen/patchbay/patchbay.rs`, `contracts/ts/src/gen/patchbay/adapter_pb.ts`

**Story**: `epic-agent-operations-resource-plane-capability-manifest-contract-registry`

```proto
message AdapterCapability {
  repeated OperationKind supported_operation_kinds = 1;
  repeated string supported_target_spec_shapes = 2;
  bool streaming_support = 3;
  // Tag-preserving clarification of the existing session contract.
  AdapterSnapshotSupport session_snapshot_support = 4;
  bool cancellation_support = 5;
  bool session_replacement_support = 6;
  IdempotencyStrength idempotency_strength = 7;
  AttachmentMethod attachment_method = 8;
  repeated FailureCode known_failure_modes = 9;
  AdapterDiagnosticReportingCapability diagnostic_reporting = 10;
  repeated AdapterTargetCategory target_categories = 11;
  repeated ResourceCapability resource_capabilities = 12;
}

enum AdapterTargetCategory {
  ADAPTER_TARGET_CATEGORY_UNSPECIFIED = 0;
  ADAPTER_TARGET_CATEGORY_RUNTIME_SESSION = 1;
  ADAPTER_TARGET_CATEGORY_OPERATIONAL_RESOURCE = 2;
  // Reserved; candidate payload family is OKF v0.2. Registration rejects it.
  ADAPTER_TARGET_CATEGORY_KNOWLEDGE_BUNDLE = 3;
}

message SchemaDescriptor {
  string schema_ref = 1;
  PayloadContentType content_type = 2;
}

message ResourceProjectionContract {
  // Selects the mandatory canonical compositor; committed declarations use
  // OPERATIONAL_RESOURCE. KNOWLEDGE_BUNDLE remains reserved.
  AdapterTargetCategory target_category = 1;
  SchemaDescriptor payload_schema = 2;
  SchemaDescriptor projection_schema = 3;
}

message ResourceCapability {
  ResourceKind resource_kind = 1;
  AdapterSnapshotSupport snapshot_support = 2;
  ResourceProjectionContract projection_contract = 3;
}
```

**Implementation notes**:

- Keep field numbers 1-10 stable and rename tag 4 only at the generated API level. Append fields 11-12; do not hand-edit generated artifacts.
- `SchemaDescriptor` identifies format and content type. It is not an inline executable schema, URL fetch instruction, or assertion that arbitrary bytes validate.
- Both `provider_pool` and `usage_window` are test/example `ResourceKind` values under `OPERATIONAL_RESOURCE`; they are not members of `AdapterTargetCategory`. Adapter implementations remain free to choose stable resource-kind identifiers and must preserve them as identity.
- Generate Rust and TypeScript from the same proto, then run contract build/drift checks.

**Acceptance criteria**:

- [ ] Generated Rust and TypeScript expose the same category, descriptor, projection-contract, and per-resource snapshot shape.
- [ ] Tag 4 remains wire-compatible while generated consumers use the unambiguous `session_snapshot_support` name.
- [ ] `KNOWLEDGE_BUNDLE` is wire-present and documented as reserved/OKF-v0.2-candidate; no generated default makes it admitted.
- [ ] Resource kinds stay typed open values rather than new core enum members.

### Unit 2: Validated manifest projection and single resource-admission boundary

**Files**: `core/src/adapter/capability.rs` (new), `core/src/adapter/mod.rs`, `core/src/resource/identity.rs`, `core/tests/adapter_capability.rs` (new)

**Story**: `epic-agent-operations-resource-plane-capability-manifest-core-admission`

```rust
// core/src/adapter/capability.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedAdapterCapability {
    target_categories: HashSet<AdapterTargetCategory>,
    resources: HashMap<ResourceKind, ValidatedResourceCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedResourceCapability {
    resource_kind: ResourceKind,
    snapshot_support: AdapterSnapshotSupport,
    projection_contract: ValidatedProjectionContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedProjectionContract {
    target_category: AdapterTargetCategory,
    payload_schema: ValidatedSchemaDescriptor,
    projection_schema: ValidatedSchemaDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSchemaDescriptor {
    schema_ref: String,
    content_type: PayloadContentType,
}

impl ValidatedAdapterCapability {
    pub fn try_from_wire(
        capability: &AdapterCapability,
        context: CapabilityValidationContext,
    ) -> Result<Self, CapabilityValidationError>;
    pub fn targets(&self, category: AdapterTargetCategory) -> bool;
    pub fn resource(&self, kind: &ResourceKind)
        -> Option<&ValidatedResourceCapability>;
}

impl AdapterRegistry {
    pub fn resource_capability(
        &self,
        identity: &ResourceIdentity,
    ) -> Option<&ValidatedResourceCapability>;

    pub fn validate_resource_projection<'a>(
        &'a self,
        identity: &ResourceIdentity,
        payload: &PayloadEnvelope,
        projection: &PayloadEnvelope,
    ) -> Result<&'a ValidatedResourceCapability, CapabilityValidationError>;
}
```

**Implementation notes**:

- Parse the wire manifest once in registration preflight and again while replay-validating the durable registration event; store the validated projection in `AdapterRecord` beside the redacted generated registration. Both paths use one validator with an explicit `Attach | Replay` context. Replay may normalize only a legacy pre-category, resource-empty record to session-only so existing durable adapter-registration events remain readable; fresh attach never receives that compatibility treatment, and the normalized record cannot admit a resource or knowledge bundle.
- Fresh registration requires at least one unique committed target category. `UNSPECIFIED`, unknown numeric values, and `KNOWLEDGE_BUNDLE` reject. `RUNTIME_SESSION` requires a non-unspecified session snapshot tier. If runtime-session is absent, the session tier must remain unspecified so a resource-only adapter cannot imply session support.
- `OPERATIONAL_RESOURCE` requires at least one resource declaration; any resource declaration requires that category. Resource kinds are non-empty and unique, each tier is authoritative/partial/none, and each projection contract targets `OPERATIONAL_RESOURCE` with complete descriptors. Cap declarations are bounded (at most 128 resource kinds; schema refs 1-256 bytes, no ASCII control/whitespace) to keep untrusted attach input finite without defining adapter-domain vocabulary.
- `resource_capability` is the sole manifest-admission lookup: the authenticated adapter id comes from the validated `ResourceIdentity`, and the exact kind must be declared by that adapter record. It does not register resource identity, resolve an Operation, or authorize a grant; `resource-state` will call it before accepting a typed report.
- `validate_resource_projection` compares `(content_type, schema_ref)` for both opaque envelopes exactly. It rejects undeclared or mismatched formats but deliberately does not claim semantic byte validation. A local typed decoder must reject malformed bytes before the cockpit installs the domain projection.
- Resource-capability removal/redeclaration remains an audited adapter registration change. Degrading existing resource state after a kind/tier loss belongs to `resource-state`; this unit exposes the validated old/new manifest data and does not mutate snapshot state.

**Acceptance criteria**:

- [ ] Session-only Pi, resource-only, and mixed session/resource manifests validate only when category/tier/declaration relationships are complete.
- [ ] One adapter can declare both `provider_pool` and `usage_window`, each with different snapshot tiers and exact schema descriptors, and exact identity lookup returns the right declaration.
- [ ] Duplicate kinds/categories, missing category on fresh attach, unspecified/unknown tier/content type, incomplete descriptors, category mismatch, resource declaration on a session-only manifest, and reserved knowledge-bundle declarations fail before durable append and also fail replay as corrupt records.
- [ ] A legacy durable registration without target categories replays only as session-only when resource declarations are empty; the same shape is rejected on fresh attach and can never satisfy resource admission.
- [ ] An identity under another adapter or undeclared kind is not admitted; schema mismatch is rejected without changing resource/session state.
- [ ] A plausible OKF v0.2 descriptor carried under `KNOWLEDGE_BUNDLE` remains rejected until promotion.

### Unit 3: Redacted diagnostics, adapter producer, server evidence, and rolling foundation

**Files**: `contracts/proto/patchbay/diagnostics.proto`, `contracts/rust/src/gen/patchbay/patchbay.rs`, `contracts/ts/src/gen/patchbay/diagnostics_pb.ts`, `core/src/diagnostics/mod.rs`, `core/tests/diagnostics_projection.rs`, `server/src/adapter_service/tests.rs`, `pi-adapter/src/core_client.ts`, `pi-adapter/tests/e2e.test.ts`, `cli/src/commands/diagnostics.ts`, `cli/tests/output-diagnostics.test.ts`, `docs/SPEC.md`, `docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/UX.md`, `docs/GLOSSARY.md`

**Story**: `epic-agent-operations-resource-plane-capability-manifest-integration-foundation`

```proto
message AdapterCapabilitySummary {
  repeated OperationKind supported_operation_kinds = 1;
  repeated string supported_target_spec_shapes = 2;
  bool streaming_support = 3;
  AdapterSnapshotSupport session_snapshot_support = 4;
  bool cancellation_support = 5;
  bool session_replacement_support = 6;
  IdempotencyStrength idempotency_strength = 7;
  string attachment_method_kind = 8;
  PayloadContentType attachment_descriptor_content_type = 9;
  repeated FailureCode known_failure_modes = 10;
  AdapterDiagnosticReportingCapability diagnostic_reporting = 11;
  repeated AdapterTargetCategory target_categories = 12;
  repeated ResourceCapability resource_capabilities = 13;
}
```

```ts
// pi-adapter/src/core_client.ts
function piCapabilityManifest(): AdapterCapability {
  return create(AdapterCapabilitySchema, {
    targetCategories: [AdapterTargetCategory.RUNTIME_SESSION],
    sessionSnapshotSupport: AdapterSnapshotSupport.PARTIAL,
    resourceCapabilities: [],
    // existing Operation/failure/attachment/diagnostic declarations unchanged
  });
}
```

**Implementation notes**:

- Extend the existing redacted diagnostics summary rather than exposing the secret-bearing attachment descriptor or creating another capability endpoint. Centralize the capability-to-summary mapping so future fields cannot silently disappear from `adapter-status`.
- `adapter-status --json` reports canonical category/tier/schema names; human output distinguishes session snapshot support from each resource-kind tier. It remains diagnostic projection data, not admission or authority.
- Update every repository-owned `AdapterCapability` fixture to declare its real category. Pi stays session-only and must not claim operational resources. Integration fixtures exercise a resource-only manifest with two kinds and prove attach rejection for the reserved category.
- Roll foundation assertions in place: PROTOCOL owns the target-category registry, validation/admission rule, projection contract, and per-resource tiers; ARCHITECTURE owns the canonical-wrapper/domain-projection separation; UX promotes the executable projection-contract seam while keeping renderer behavior with cockpit composition; VERIFICATION labels these checks implementation-checked and leaves promoted vectors/formal claims to the conformance sibling; GLOSSARY defines category/schema/projection terms. Do not claim semantic schema validation or checked-normative assurance.

**Acceptance criteria**:

- [ ] Pi attach/e2e remains green with `RUNTIME_SESSION`, the renamed session tier, and no resource declaration.
- [ ] Server attach accepts valid two-kind operational-resource declarations and rejects malformed or reserved declarations without a durable registration event.
- [ ] Durable adapter replay and `adapter-status` preserve exact target categories, resource kinds, per-kind tiers, and schema descriptors while continuing to redact attachment material.
- [ ] CLI JSON/human output cannot relabel a resource tier as adapter-wide or session-wide support.
- [ ] Foundation docs classify operational resources as committed post-v0.1 direction, knowledge bundles/OKF v0.2 as a reserved third category, and dynamic adapter renderer/plugin loading as rejected for this arc.

## Implementation Order

1. `epic-agent-operations-resource-plane-capability-manifest-contract-registry`
2. `epic-agent-operations-resource-plane-capability-manifest-core-admission` after generated types exist.
3. `epic-agent-operations-resource-plane-capability-manifest-integration-foundation` after the validator/admission API is stable.
4. Child checkpoints advance directly to `done` on green evidence; the integrated feature receives the caller-mandated `thorough` review before `done`.

## Simplification

- Extend the one existing `AdapterCapability` and durable `AdapterRegistry`; do not add a resource-only registration service, capability store, or second writer.
- Rename tag 4 in place and remove the ambiguous adapter-wide generated name instead of supporting two snapshot fields or precedence rules.
- Use one `AdapterTargetCategory` registry for manifest admission and projection conformance selection; do not duplicate category strings in Rust, TypeScript, or docs.
- Reuse open `ResourceKind` and `ResourceIdentity` from `resource-identity`; do not create core enums for `provider_pool`/`usage_window` or accept a local resource id as admission evidence.
- Keep adapter projection bytes opaque and nested. No dynamic renderer code, generic plugin bundle, inline HTML/CSS, `access_mode` enum, or capability-derived grant rule is added.
- Retain the redacted diagnostics summary because attachment descriptors are verified secrets; centralize its mapping rather than exposing the raw capability.

## Testing

- **Contract checks** protect field-number stability, enum generation, Rust/TypeScript parity, and generated drift. They are the durable wire-shape evidence.
- **Core interface tests** protect the registration/replay boundary and exact `(adapter_id, resource_kind)` admission lookup. A compact invalid-manifest table covers unknown/reserved categories and contradictory declarations; focused two-kind tests protect per-resource tier/schema selection.
- **Projection-boundary tests** protect exact schema/content-type matching and the distinction between declaration binding and semantic decoding. No tautological getter tests are added.
- **Server/Pi integration tests** protect authenticated attach behavior, no-durable-event-on-rejection, Pi's honest session-only declaration, and durable replay of valid resource manifests.
- **Diagnostics/CLI regression tests** protect redaction and honest session-vs-resource tier output. Existing adapter lifecycle/diagnostic tests remain green.
- **Deferred evidence**: malformed typed resource reports, stale-tier degradation, cockpit decoder/rendering behavior, and promoted resource-plane conformance vectors remain with `resource-state`, `cockpit-composition`, and `conformance` respectively.
- **Verification commands**: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`; contract generation/build/drift; Pi and CLI builds/tests; vector/model/presentation checks after foundation traceability updates.

## Risks

- **Semantic misclassification cannot be inferred from bytes.** An authenticated adapter could falsely label arbitrary telemetry as an operational resource. The contract prevents accidental category fallthrough and rejects the reserved knowledge category; trust-root policy and adapter conformance remain the semantic enforcement. The core must not claim that a schema-ref string proves the product admission rule.
- **Schema-reference false confidence.** Exact descriptors establish declared format identity only. Malformed or semantically invalid bytes must fail in the typed decoder and never install a projection; the design explicitly avoids the partial-validator problem named by the parent epic.
- **Dual snapshot ambiguity.** Leaving tag 4 named `snapshot_support` would invite resource fallback. Renaming it session-wide and requiring an exact resource declaration removes fallback; `resource-state` must never consult the session field for a resource.
- **Capability loss can strand a cached resource.** This feature records and exposes the new manifest, but the resource-state sibling owns stale/unknown degradation and tombstone/replacement policy. Until that consumer lands, manifest admission is fail-closed and does not fabricate live resource state.
- **Renderer bypass.** A future surface could spread adapter projection fields into canonical state. The contract keeps adapter bytes nested and target-category-bound; cockpit composition must use a local decoder/compositor and the closing conformance feature must mutation-test that stale/authority/delivery primitives cannot be overridden.
- **Legacy replay widening.** Durable pre-category registration events must remain readable, but a generic default could turn that compatibility path into implicit resource admission. The replay-only normalization is restricted to resource-empty records and yields `RUNTIME_SESSION` only; fresh attach rejects the same shape.
- **Reserved-value erosion.** Generic enum parsing or protobuf defaults could accidentally admit `KNOWLEDGE_BUNDLE`. Registration uses an explicit committed-category allowlist and tests unknown, unspecified, and reserved values as rejection cases.

## Extension pressure classification

- **Committed post-v0.1 direction:** `runtime_session` and `operational_resource` are admitted target categories; operational-resource declarations are exact per adapter-owned `ResourceKind`, with per-kind snapshot tier and schema-bound domain projection above the mandatory conformance floor.
- **Reserved seam:** `knowledge_bundle` is wire-present but registration-rejected; OKF v0.2 is the named candidate format. Promotion requires its own report, snapshot/reconciliation, authority, presentation, and conformance ceremony. Per-resource OperationKind maps and dynamically registered local renderer catalogs remain additive seams if a real multi-kind adapter requires them.
- **Explicitly rejected for this arc:** treating knowledge/work-ledger bundles as operational resources, making provider/pool/window names core target categories, loading adapter-supplied UI code, allowing projection payloads to override canonical state, or using capabilities as authority/delivery gates.

## Other agent review

- Invoked because: the feature changes a future public adapter contract and the executable presentation boundary.
- Fixed/active blockers: the design makes the reserved OKF category wire-visible but fail-closed, distinguishes session/resource snapshot tiers, and treats schema refs as bindings rather than semantic validators.
- Parked: none from this pass.
- Rejected: generic dynamic projection plugins and stringly target categories, for the reasons in the alternatives and pre-mortem above.
- Skipped/degraded: this delegated worker exposes no independent subagent/peer dispatch tool, so design-time fresh-context advisory review could not run. Direct source/commit verification and the pre-mortem above were used; the caller-specified `thorough` implementation review remains mandatory.

## Implementation summary

All three dependency-ordered checkpoints are complete:

1. The generated adapter contract now owns `AdapterTargetCategory`, schema
   descriptors, per-resource snapshot/projection declarations, and the
   tag-preserving `session_snapshot_support` rename in Rust and TypeScript.
2. Core registration validates manifests on attach and replay, stores one
   validated capability projection, preserves only the narrow legacy
   session-only replay path, and exposes exact resource-kind admission/schema
   binding without changing grants, resolution, delivery, or resource state.
3. Redacted diagnostics, CLI output, Pi's session-only producer, server attach
   evidence, and rolling foundation assertions now carry the resource-aware
   contract end to end.

Implementation used direct host ownership because this delegated harness exposed
no generic worker dispatch adapter. That kept the one-feature write boundary
coherent across the generated schema, Rust projection, TypeScript consumers, and
foundation updates. The effective review weight is `thorough`, explicitly set by
the autopilot caller.

## Integrated verification

- `cargo test --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- Contract Rust/TypeScript builds, `buf generate`, generated drift, and focused
  lint of the modified adapter/diagnostics protos — passed.
- CLI build/tests — 37 passed.
- Pi adapter build/tests, including the real core/adapter e2e — 24 passed.
- Model-promotion, conformance-vector, and presentation checks — passed.
- Repository-wide `buf lint` still reports pre-existing RPC request/response
  naming debt in unchanged services; repository-wide `cargo fmt --check` still
  reports pre-existing broad Rust formatting drift. Neither was rewritten or
  misreported as introduced by this feature.

## Review (2026-08-04)

**Verdict**: Approve

**Blockers**: none unresolved. Pass 1 identified two receiver-confirmed material
verification gaps: exact schema binding was not mutation-sensitive across both
schema refs and content types, and durable attachment-descriptor redaction was
not tested at the storage/replay boundary. Both were corrected and verified in
`26705d1`.
**Important**: none parked.
**Nits**: pass 1's ambiguous “no durable append” wording was corrected to “no
durable adapter-registration append,” preserving the required rejection audit.
**Rejected**: none.

**Notes**: Effective review weight was `thorough`, explicitly supplied by the
autopilot caller. Two same-harness fresh-context passes ran with
`openai-codex/gpt-5.6-sol` at xhigh effort. Pass 1 requested the evidence fixes
above; after adjudication, correction, focused verification, and clippy, pass 2
returned `ready` with no findings and confirmed every pass-1 disposition
resolved. The corrected aggregate passed full Rust workspace tests, warnings-
denied clippy, CLI tests (37), Pi tests including real-process e2e (24), contract
build/drift and focused proto lint, model/vector checks, and presentation
conformance. No lower-risk finding remained to park.
