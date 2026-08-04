---
id: epic-agent-operations-resource-plane-resource-state
kind: feature
stage: review
tags: [foundation, protocol, storage]
parent: epic-agent-operations-resource-plane
depends_on: [epic-agent-operations-resource-plane-resource-identity]
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-08-04
---

# Resource snapshot, revision & ingestion

## Brief

Give operational resources a durable state model distinct from runtime-session
state: a `ResourceSnapshot` record with view revisions, an explicit
completeness tier (authoritative / partial / none), tombstone/replacement
semantics, and a typed resource-report ingress path (analogous to
`SessionReport`) so adapters submit structured resource state rather than only
generic Observations. Resource Observation deltas fold into a revisioned
projection; reconnect reconciles against the resource snapshot.

Today there is no `ResourceSnapshot`, no resource revision record, no
completeness tier, no typed resource-report ingress, and no `StoredEventKind`
resource variant — `LoadSnapshot` returns opaque bytes implemented as
`SessionSnapshot`, and the live materializer (`server/src/state.rs`) emits
session records only. This feature adds durable resource state + replay and
the resource `LoadSnapshot` path. It must degrade honestly: a resource
adapter may claim only the snapshot tier its complete external view can
actually reconstruct.

It does not define the adapter capability manifest fields that declare which
resource kinds/snapshot tiers an adapter supports (`capability-manifest`) or
cockpit rendering (`cockpit-composition`).

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: state foundation — depends on `resource-identity`; consumed by `capability-manifest` and `cockpit-composition`.

## Simplification opportunity

- Reuse the opaque-domain-keyed storage port (`core/src/storage/port.rs`) and the consistent-prefix materializer pattern already used for sessions; add a resource materializer rather than a parallel store.
- Keep one snapshot/reconnect discipline rather than per-resource-type state stores.

## Foundation references

- `docs/PROTOCOL.md` — snapshots/revisions, reconnect reconciliation, snapshot tiers
- `contracts/proto/patchbay/sessions.proto:40-61` — `SessionSnapshot` (the pattern to mirror, not reuse)
- `contracts/proto/patchbay/adapter_control.proto:38-55` — only `SessionReport` is typed today
- `contracts/proto/patchbay/control.proto:44-53` — `LoadSnapshot` returns opaque bytes
- `contracts/proto/patchbay/common.proto:120-158` — `StoredEventKind` has no resource variant
- `core/src/storage/port.rs:57-64` — opaque domain/LSN-keyed snapshots
- `core/src/acceptance/observation.rs:216-252` — Status/Result folds command state, not resource revisions

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`
- No direct UI; the snapshot/revision model the cockpit feature renders.

## Design decisions

- **Resource state is a separate projection keyed by the existing exact `ResourceIdentity`.** The identity-only `ResourceRegistry` becomes the canonical state registry; active membership, adapter-owned payload, freshness, revisions, and tombstones live there. No parallel membership cache and no runtime-session generation are introduced.
- **Completeness is per `(adapter_id, resource_kind)` view.** `ResourceSnapshot` carries one revision record per adapter-owned kind and reuses the existing generated `AdapterSnapshotSupport` vocabulary (`authoritative | partial | none`) rather than creating a second three-value registry. The sibling `capability-manifest` design owns each kind's maximum declared tier and schema descriptors; this feature consumes its exact admission lookup, permits only equal-or-weaker report tiers (`authoritative > partial > none`), and defines fold/degradation semantics without adding manifest fields.
- **Typed reports distinguish reconnect snapshots from live deltas.** `ResourceReport` is an authenticated adapter report with `snapshot` and `delta` variants, each containing one or more resource-kind views. A report becomes one normalized durable `ResourceStateEvent`, so a cross-kind replacement can tombstone the old identity and upsert the distinct replacement atomically.
- **Tier semantics are conservative.** An authoritative reconnect snapshot is a complete external view: listed resources become current and omitted active resources in that view are tombstoned. A partial snapshot updates listed resources and marks omitted cached resources stale. A none snapshot carries no reconstructed mutations and marks cached resources stale. Live deltas affect only explicit identities, regardless of tier. This makes an incomplete adapter choose partial/none rather than manufacture completeness.
- **Freshness is Patchbay reconciliation state, not domain health.** `ResourceFreshnessState = current | stale | unknown` describes confidence in the cached payload. Provider exhaustion, credential hold, model availability, and other adapter health remain inside the schema-identified payload and never become session connectivity/activity states.
- **Tombstones are terminal for an exact resource identity.** Reappearance after a permanent delete requires a distinct adapter-provided `ResourceIdentity`; a replacement relation retains `old → new`. This is the only safe shape without a resource generation: late or reordered evidence cannot silently resurrect a retired target. An adapter whose view can temporarily omit a resource must declare partial/none rather than authoritative.
- **Core LSNs own revisions.** Adapter reports carry source adapter generation and informational `observed_at`, but never assign Patchbay revisions. Durable commit assigns the record/view revision; replay, resolution, and snapshots use `(authority_domain_id, LSN)` ordering. Every accepted report is durable evidence and advances each reported view revision even when its payload is unchanged.
- **Reconnect is fenced by authenticated adapter generation.** The server compares report adapter id/generation with the current attachment under `CoreDecisionGate`. The first accepted report from a newer generation stales that adapter's prior active records before applying the report; abnormal disconnect durably stales all active resources. Old attachment tokens/stream epochs remain inert.
- **`LoadSnapshot` gains an explicit view discriminator.** `SnapshotViewKind = session | resource` is required in the request and echoed in the response before opaque payload decode. Existing owned callers are updated to request `session`; unknown/unspecified values fail fast. Resource reads are on-demand materializations over durable resource events and do not use the current undiscriminated session checkpoint slot.
- **The opaque snapshot store is not widened in this feature.** Production does not write periodic checkpoints today, and the table/port has no projection discriminator. Session checkpoints remain the only payload accepted from that namespace; resource requests always materialize from the replayable projection. A future checkpoint-activation feature may add a typed snapshot namespace/migration when recovery-cost evidence warrants it.
- **Autopilot rationale.** These choices minimize new concepts while preserving wrong-target and stale-state safety: one existing identity registry becomes stateful, one generated tier vocabulary is reused, one event atomically records a report, and one existing `LoadSnapshot` RPC is discriminated. No capability-manifest field, adapter-specific health enum, cockpit behavior, or promoted conformance claim is pulled forward.

## Codebase mapping

Direct reading covered the eight foundation docs, parent/identity feature, the concurrently landed `capability-manifest` design, generated Protobuf contracts, resource identity/composite resolution, session report/delta/replay patterns, command Observation folding, the storage snapshot port and SQLite namespace, server projection catch-up/materializers, adapter authentication/replacement/disconnect paths, operator-facing subscription filtering, and current CLI/web session snapshot consumers. The state design aligns to the sibling's `ResourceCapability` tier plus payload/projection schema descriptors and `AdapterRegistry::validate_resource_projection` API without taking ownership of those declarations. This is a broad but well-mapped contract/core/server change. Independent design-time dispatch would be warranted by the storage/replay risk, but this delegated worker exposes no subagent or peer tool; source verification, the pre-mortem below, and the caller-required `thorough` implementation review are the available scrutiny paths.

## Architectural choice

### Options considered

1. **Normalized resource event + revisioned registry + discriminated snapshot RPC (chosen).** Typed adapter reports are validated and normalized into explicit resource mutations, one durable event folds a stateful registry, and `LoadSnapshot` selects session/resource payloads. This optimizes for deterministic replay, honest tier semantics, and one ordinary target registry. It costs coordinated contract/core/server changes.
2. **Persist generic `ObservationKind::DELTA` and reconstruct state by decoding adapter payloads at read time.** This is smaller at ingress, but replay would depend on open adapter schemas, the core could not safely infer tombstones or partial-view omissions, and every consumer would reimplement state folding. It violates the generated-contract and durable-projection patterns.
3. **Create a separate resource store/RPC and leave `ResourceRegistry` identity-only.** This avoids changing the session-oriented snapshot path, but duplicates membership, revision ordering, reconnect, storage, and subscription behavior. Resolver membership could then disagree with the resource store after crash or partial failure.

The chosen approach follows the existing session durable-delta pattern without reusing session state. The trickiest unit is **normalizing reconnect reports into an explicit atomic event**: authoritative omission, partial/none degradation, replacement, source-generation fencing, and retry after failure must all replay identically. That unit is designed before server snapshot materialization.

## Implementation Units

### Unit 1: Generated resource-state, report, event, and snapshot-view contracts

**Files**: `contracts/proto/patchbay/resources.proto` (new), `contracts/proto/patchbay/common.proto`, `contracts/proto/patchbay/adapter_control.proto`, `contracts/proto/patchbay/control.proto`, `contracts/rust/src/gen/patchbay/patchbay.rs`, `contracts/ts/src/gen/patchbay/resources_pb.ts`, `contracts/ts/src/gen/patchbay/adapter_control_pb.ts`, `contracts/ts/src/gen/patchbay/control_pb.ts`

**Story**: `epic-agent-operations-resource-plane-resource-state-contract`

```proto
// resources.proto
message Resource {
  AuthorityDomainId authority_domain_id = 1;
  ResourceIdentity identity = 2;
  PayloadEnvelope resource_payload = 3; // absent when freshness is UNKNOWN
  PayloadEnvelope projection_payload = 4; // adapter-shaped cockpit projection
  ResourceFreshnessState freshness = 5;
  Generation source_adapter_generation = 6;
  Lsn revision_lsn = 7;
  google.protobuf.Timestamp observed_at = 8;
  bool tombstoned = 9;
  Lsn tombstoned_at_lsn = 10;
  ResourceIdentity replaced_by = 11;
}

enum ResourceFreshnessState {
  RESOURCE_FRESHNESS_STATE_UNSPECIFIED = 0;
  RESOURCE_FRESHNESS_STATE_CURRENT = 1;
  RESOURCE_FRESHNESS_STATE_STALE = 2;
  RESOURCE_FRESHNESS_STATE_UNKNOWN = 3;
}

message ResourceViewRevision {
  AdapterId adapter_id = 1;
  ResourceKind resource_kind = 2;
  AdapterSnapshotSupport completeness = 3;
  Generation source_adapter_generation = 4;
  Lsn revision_lsn = 5;
  google.protobuf.Timestamp observed_at = 6;
}

message ResourceSnapshot {
  AuthorityDomainId authority_domain_id = 1;
  Lsn snapshot_lsn = 2;
  Generation core_generation = 3;
  repeated Resource resources = 4;
  repeated ResourceViewRevision view_revisions = 5;
  google.protobuf.Timestamp materialized_at = 6;
}

message ResourceReport {
  AdapterId adapter_id = 1;
  Generation adapter_generation = 2;
  oneof report {
    ResourceSnapshotReport snapshot = 3;
    ResourceDeltaReport delta = 4;
  }
  google.protobuf.Timestamp observed_at = 5;
}

message ResourceSnapshotReport { repeated ResourceViewReport views = 1; }
message ResourceDeltaReport { repeated ResourceViewReport views = 1; }
message ResourceViewReport {
  ResourceKind resource_kind = 1;
  AdapterSnapshotSupport completeness = 2;
  repeated ResourceReportMutation mutations = 3;
}
message ResourceReportMutation {
  ResourceIdentity identity = 1;
  oneof mutation {
    ResourceStateUpsert upsert = 2;
    ResourceStateUnknown unknown = 3;
    ResourceStateTombstone tombstone = 4;
  }
}
message ResourceStateUpsert {
  PayloadEnvelope resource_payload = 1;
  PayloadEnvelope projection_payload = 2;
}
message ResourceStateUnknown {}
message ResourceStateTombstone { ResourceIdentity replaced_by = 1; }

// Normalized durable payload for StoredEventKind::RESOURCE_STATE.
message ResourceStateEvent {
  AuthorityDomainId authority_domain_id = 1;
  AdapterId source_adapter_id = 2;
  Generation source_adapter_generation = 3;
  repeated ResourceViewStateUpdate views = 4;
  repeated ResourceStateMutation mutations = 5;
  google.protobuf.Timestamp observed_at = 6;
}
message ResourceViewStateUpdate {
  ResourceKind resource_kind = 1;
  AdapterSnapshotSupport completeness = 2;
}
message ResourceStateMutation {
  ResourceIdentity identity = 1;
  Lsn from_revision_lsn = 2; // absent for first registration
  oneof mutation {
    ResourceStateUpsert upsert = 3;
    ResourceStateUnknown unknown = 4;
    ResourceStateTombstone tombstone = 5;
    ResourceFreshnessChanged freshness_changed = 6;
  }
}
message ResourceFreshnessChanged {
  ResourceFreshnessState from = 1;
  ResourceFreshnessState to = 2;
}
```

```proto
// common.proto
STORED_EVENT_KIND_RESOURCE_STATE = 15; // payload: ResourceStateEvent

// control.proto
message LoadSnapshotRequest {
  AuthorityDomainId authority_domain_id = 1;
  optional Lsn at_or_before = 2;
  SnapshotViewKind view_kind = 3;
}
message LoadSnapshotResponse {
  bool present = 1;
  EventId event_id = 2;
  bytes snapshot_payload = 3;
  SnapshotViewKind view_kind = 4;
}
enum SnapshotViewKind {
  SNAPSHOT_VIEW_KIND_UNSPECIFIED = 0;
  SNAPSHOT_VIEW_KIND_SESSION = 1;
  SNAPSHOT_VIEW_KIND_RESOURCE = 2;
}

// adapter_control.proto, ObservationRequest.oneof observation
ResourceReport resource_report = 4;
```

**Implementation notes**:

- `resources.proto` imports `adapter.proto` only for the existing canonical `AdapterSnapshotSupport` enum. The manifest remains unchanged; the sibling feature later reuses the same enum for per-kind declaration.
- A view report's every identity must carry the authenticated adapter and the view's exact kind. Replacement is represented by a tombstone whose `replaced_by` points to a distinct same-adapter identity; a matching upsert for that replacement must occur in the same report/event.
- Both `resource_payload` and `projection_payload` must be present for `upsert`, have known non-unspecified content types, and carry non-empty `schema_ref`s. The sibling manifest binds them to its exact payload/projection descriptors through `AdapterRegistry::validate_resource_projection`; this feature still treats bytes as adapter-owned metadata and leaves semantic decoding to local typed decoders. `unknown` deliberately carries neither envelope.
- `StoredEventKind` remains the single durable discriminator. Update generated Rust and TypeScript from `.proto`; never hand-edit artifacts.

**Acceptance criteria**:

- [ ] The wire can distinguish current/stale/unknown resource confidence, exact tombstone/replacement identity, per-kind completeness/revision, and snapshot-vs-delta reports without session fields.
- [ ] `LoadSnapshot` cannot return ambiguous bytes: request and response both identify session/resource view.
- [ ] Unknown/unspecified enums, missing report oneof, duplicate view keys, incomplete identities, invalid mutation combinations, and malformed payload envelopes reject at ingress.
- [ ] Rust/TypeScript contract build and drift checks pass.

### Unit 2: Canonical resource projection, normalized fold, and replay

**Files**: `core/src/resource/state.rs` (new), `core/src/resource/events.rs` (new), `core/src/resource/registry.rs`, `core/src/resource/replay.rs` (new), `core/src/resource/mod.rs`, `core/src/target.rs`, `core/src/lib.rs`, `core/tests/resource_state.rs` (new), `core/tests/resource_replay.rs` (new), existing exhaustive `StoredEventKind` receivers under `core/src/{acceptance,authority,diagnostics,storage}/`

**Story**: `epic-agent-operations-resource-plane-resource-state-projection-replay`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceRecord {
    pub identity: ResourceIdentity,
    pub resource_payload: Option<PayloadEnvelope>,
    pub projection_payload: Option<PayloadEnvelope>,
    pub freshness: ResourceFreshnessState,
    pub source_adapter_generation: Generation,
    pub revision_lsn: u64,
    pub observed_at: Timestamp,
    pub tombstoned_at_lsn: Option<u64>,
    pub replaced_by: Option<ResourceIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceViewKey {
    pub adapter_id: AdapterId,
    pub resource_kind: ResourceKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceViewRecord {
    pub key: ResourceViewKey,
    pub completeness: AdapterSnapshotSupport,
    pub source_adapter_generation: Generation,
    pub revision_lsn: u64,
    pub observed_at: Timestamp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResourceRegistry {
    resources: HashMap<ResourceIdentity, ResourceRecord>,
    views: HashMap<ResourceViewKey, ResourceViewRecord>,
}

impl ResourceRegistry {
    pub fn new() -> Self;
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), ResourceError>;
    pub fn contains(&self, identity: &ResourceIdentity) -> bool; // active only
    pub fn get(&self, identity: &ResourceIdentity) -> Option<&ResourceRecord>;
    pub fn resources(&self) -> impl Iterator<Item = &ResourceRecord>;
    pub fn views(&self) -> impl Iterator<Item = &ResourceViewRecord>;
    pub fn active_in_view(&self, key: &ResourceViewKey)
        -> impl Iterator<Item = &ResourceRecord>;
}

pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<ResourceRegistry, ResourceError>;
```

**Implementation notes**:

- `observe` ignores sibling event kinds, decodes only `RESOURCE_STATE`, validates event/payload domain equality and source adapter consistency, then folds mutations in event order. The event LSN becomes every touched resource/view revision.
- `from_revision_lsn` makes normalized deltas self-checking. A redelivered event at or below the record revision is inert; a later event claiming the wrong prior revision is corruption, not a best-effort merge.
- Tombstoned records stay in `resources()` for snapshot/audit context but `contains` and ordinary resolver membership exclude them. Upsert/unknown against a tombstone fails; replacement identity must be distinct, same-adapter, and not tombstoned.
- Replace `TargetRegistry::observe_session_event` with `observe_event`, delegating to both session and resource registries. Rebuild and catch-up populate resolver membership only from durable events.
- Replay reads from LSN 0 because the shared checkpoint slot remains undiscriminated, validates one domain and strictly increasing LSN, and folds the same function used live.

**Acceptance criteria**:

- [ ] Active exact identities resolve after replay; tombstones and unknown identities return `target_not_found`; adapter/kind/id collisions remain fenced.
- [ ] Repeated replay is deterministic and cannot lower revisions, resurrect tombstones, or apply a cross-domain/non-monotonic event.
- [ ] Per-view completeness/source generation and per-resource freshness/payload/replacement survive restart.
- [ ] Every existing exhaustive event consumer either handles `ResourceState` or explicitly ignores it; no wildcard hides a registry omission.

### Unit 3: Typed report normalization, authenticated ingress, and adapter reconciliation

**Files**: `core/src/resource/ingest.rs` (new), `core/src/resource/events.rs`, `core/src/resource/replay.rs`, `core/src/session/ingest.rs`, `core/src/adapter/mod.rs`, `core/src/adapter/capability.rs`, `server/src/adapter_service.rs`, `server/src/adapter_service/tests.rs`, `core/tests/resource_ingest.rs` (new), `core/tests/resource_reconciliation.rs` (new)

**Story**: `epic-agent-operations-resource-plane-resource-state-report-ingress-reconciliation`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceReportMode { Snapshot, Delta }

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedResourceReport {
    pub authority_domain_id: AuthorityDomainId,
    pub adapter_id: AdapterId,
    pub adapter_generation: Generation,
    pub mode: ResourceReportMode,
    pub views: Vec<ResourceViewReport>,
    pub observed_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceIngestResult {
    pub event_id: EventId,
    pub touched_resources: usize,
    pub touched_views: usize,
}

pub async fn ingest_resource_report<S: Storage>(
    storage: &S,
    registry: &mut ResourceRegistry,
    report: ValidatedResourceReport,
) -> Result<ResourceIngestResult, ResourceError>;

pub fn adapter_stale_event(
    registry: &ResourceRegistry,
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
    adapter_generation: Generation,
    observed_at: Timestamp,
) -> Result<Option<StoredEventPayload>, ResourceError>;
```

**Implementation notes**:

- Server validation occurs inside `CoreDecisionGate`: authenticate the current attachment, require request domain, compare report adapter id and adapter generation with the current redacted `AdapterRegistration`, require `AdapterRegistry::resource_capability` for every exact kind, reject a report tier stronger than the declared maximum, and call `validate_resource_projection` for every upsert's payload/projection envelopes before building the domain report. Payload-supplied source never overrides attachment evidence. If implementation reaches this unit before the parallel manifest checkpoint lands, stop at this explicit integration prerequisite rather than inventing fallback manifest fields or admitting undeclared resources.
- Normalize before append. Snapshot mode derives explicit omissions from the current registry: authoritative → terminal tombstone; partial → stale; none → reject any input mutation and stale. Delta mode preserves omissions and emits only explicit changes. A higher authenticated source generation first emits stale changes for all prior active records owned by that adapter, then applies the report in the same event.
- Each valid report appends exactly one `ResourceStateEvent` even if state bytes are unchanged; the report itself is durable source evidence and advances its view revisions/observed time. No projection mutation occurs before append. After append, fold the committed event; on a fold failure, discard/rebuild the hot projection before reuse.
- An explicit replacement requires old active identity + distinct same-adapter replacement + replacement upsert in the same report. Cross-adapter replacement, missing replacement state, duplicate mutations, and tombstone resurrection reject before append.
- Refactor session disconnect degradation into a pure source-event builder and combine its outputs with `adapter_stale_event` in one existing `append_batch_audited` call. One `ADAPTER_DETACHED` audit covers the session/resource degradation batch; then both projections rebuild. This removes a session-only writer instead of adding a duplicate resource detach audit path.
- The typed report path owns resource projection state. Generic `Observation` remains durable evidence/command-lifecycle input; an opaque adapter delta cannot mutate resource state unless it arrives through the schema-owned `ResourceReport.delta` variant.

**Acceptance criteria**:

- [ ] Authenticated current-generation reports append and fold; wrong adapter/domain/generation, old token/epoch, undeclared kind, overclaimed tier, schema mismatch, malformed view, unknown tier, and bad payload/projection reject without durable resource state.
- [ ] Authoritative omission tombstones, partial/none omission stales, explicit live delta affects only its identities, and a valid replacement is atomic across kinds.
- [ ] Adapter loss stales resource payloads without coercing domain health or session axes; replacement stream fences make an old disconnect inert.
- [ ] A failed append leaves the projection unchanged; a committed-prefix/fold error causes rebuild; retry cannot duplicate or contradict the normalized delta.

### Unit 4: Server projection composition and resource `LoadSnapshot`

**Files**: `server/src/state.rs`, `server/src/service.rs`, `server/tests/grpc_smoke.rs`, `web-cockpit/src/domain/reconcile.ts`, `web-cockpit/tests/reconcile.test.ts`, `cli/src/commands/sessions.ts`, `cli/tests/output-diagnostics.test.ts`, `pi-adapter/tests/e2e.test.ts`

**Story**: `epic-agent-operations-resource-plane-resource-state-snapshot-load`

```rust
impl ProjectionState {
    pub async fn materialize_resource_snapshot(
        &self,
        authority_domain_id: AuthorityDomainId,
        materialized_at: Timestamp,
    ) -> ResourceSnapshot;
}

async fn load_snapshot(
    &self,
    request: Request<LoadSnapshotRequest>,
) -> Result<Response<LoadSnapshotResponse>, Status>;
```

**Implementation notes**:

- `ProjectionState::rebuild` replays `ResourceRegistry` instead of constructing it empty; `catch_up` folds resource events through `TargetRegistry::observe_event`. Acquire `last_applied_lsn` before the target-registry lock, matching the session materializer, so records/views and `snapshot_lsn` are one consistent prefix.
- Stable-sort resources by `(adapter_id, resource_kind, resource_id)` and views by `(adapter_id, resource_kind)`. Materialize active and tombstoned records, exact per-record/view LSNs, completeness, source generation, payload/freshness, observed time, replacement, and core `materialized_at`. Core generation stays absent until its reserved persistence feature lands.
- `load_snapshot` validates the generated view enum before catch-up and re-verifies the compound issuer under the gate. `SESSION` may return a stored checkpoint only when it decodes as `SessionSnapshot`, matches domain/response LSN, and is not older than the current projection; otherwise it materializes current session state. `RESOURCE` always materializes the current resource projection and never reads the session-only checkpoint slot.
- A historical `at_or_before` that cannot be reconstructed returns the newer current authoritative view, matching the existing stale-snapshot repair rule. It must not return an empty or older view and call it authoritative.
- Existing session consumers explicitly set `SnapshotViewKind::SESSION` before decoding. This mechanical contract update is not cockpit resource rendering. The sibling cockpit feature will add the resource consumer/projection.
- Include `StoredEventKind::ResourceState` in operator-facing subscription output so a later resource reconciler can fold live deltas; this feature only proves delivery/decoding and does not render them.

**Acceptance criteria**:

- [ ] Resource snapshot payload and response event carry the same domain/LSN and deterministic ordering; restart produces an equivalent snapshot.
- [ ] Session and resource requests cannot decode each other's payload; unspecified/unknown view rejects; a raw/corrupt/older session checkpoint is not returned as current authority.
- [ ] Authoritative/partial/none views preserve honest freshness and tombstones in the snapshot; cached partial/none state never appears current after reconnect/loss.
- [ ] Current CLI, web reconciler, and Pi E2E session reads remain green with explicit session selection.

### Unit 5: Integrated evidence and rolling foundation

**Files**: `core/tests/resource_ingest.rs`, `core/tests/resource_replay.rs`, `server/src/adapter_service/tests.rs`, `server/tests/grpc_smoke.rs`, `contracts/scripts/check-generated-drift.mjs`, `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`

**Story**: `epic-agent-operations-resource-plane-resource-state-integration-foundation`

**Implementation notes**:

- Add one real-path test from authenticated `ResourceReport` through durable `RESOURCE_STATE`, projection catch-up, ordinary exact resource resolution, restart replay, and `LoadSnapshot(RESOURCE)`. Include replacement and all three completeness tiers.
- Property-test ordering and normalization risks: independent adapter/kind/id dimensions; arbitrary ordered report sequences; replay twice; authoritative omission vs partial/none; terminal tombstone mutation attempts; and wrong-domain/non-increasing event prefixes. Mutation evidence must fail if tier branches, source generation checks, or any identity component are removed.
- Roll foundation assertions forward in place. PROTOCOL becomes canonical intent for resource freshness/tier/replacement/reconnect; ARCHITECTURE names the state projection; SECURITY prohibits credentials/data-plane content in resource payloads and keeps authenticated source binding; VERIFICATION records implementation-checked replay/reconnect evidence without promoting formal/vector status; GLOSSARY defines the new terms.
- The closing `conformance` sibling owns promoted vectors/formal assurance. `capability-manifest` owns admitted kinds, schema declarations, and the maximum per-kind tier. `cockpit-composition` owns UI state binding/rendering.

**Acceptance criteria**:

- [ ] End-to-end tests prove resource existence and snapshot state survive restart and cannot be widened across adapter/kind/id, source generation, or authority domain.
- [ ] Full Rust workspace tests/clippy, TypeScript suites, generated drift, vector/model metadata, and presentation checks pass.
- [ ] No test is weakened to accept production output; no generated artifact is hand-edited; neither resource nor projection payload contains credentials or model data-plane traffic.
- [ ] Docs use the post-v0.1 classification and state the assurance tier honestly.

## Implementation Order

1. `epic-agent-operations-resource-plane-resource-state-contract`
2. `epic-agent-operations-resource-plane-resource-state-projection-replay`
3. `epic-agent-operations-resource-plane-resource-state-report-ingress-reconciliation` after both the resource projection and sibling `epic-agent-operations-resource-plane-capability-manifest-core-admission` API exist
4. `epic-agent-operations-resource-plane-resource-state-snapshot-load`
5. `epic-agent-operations-resource-plane-resource-state-integration-foundation`
6. Advance child stories directly to `done` on green checkpoint evidence, then review the integrated feature at the caller's explicit `thorough` weight until no receiver-confirmed material blocker remains.

The feature is cohesive despite five checkpoints: schema, fold, ingress, and materializer share one event/registry invariant and overlapping files, so one feature owner is safer than one worker per story.

## Simplification

- Evolve the existing `ResourceRegistry`; do not add a resource store beside resolver membership.
- Reuse `AdapterSnapshotSupport`, `PayloadEnvelope`, `ResourceIdentity`, the authority-domain log, `StoredEventPayload`, `TargetRegistry`, `CoreDecisionGate`, `append_batch_audited`, and `LoadSnapshot` rather than parallel tier, storage, acceptance, or RPC systems.
- Replace session-only target observation with one composite `TargetRegistry::observe_event` dispatch.
- Consolidate adapter-loss session/resource degradation into one audited source batch rather than duplicating disconnect audit writers.
- Keep checkpoint storage undiscriminated and session-only because production checkpoint writing is inactive; do not add a database migration or recovery abstraction without a measured checkpoint need.
- Retain generic Observations for evidence/command lifecycle, but forbid opaque payloads from mutating the resource projection. Only the typed report contract owns resource state.

## Testing

- **Contract/boundary tests** protect generated enum/oneof handling, exact authenticated source binding, timestamp/resource/projection validation, required snapshot view kind, and cross-language drift.
- **Projection/replay interface tests** protect deterministic fold, domain/LSN validation, full-tuple membership, revision monotonicity, terminal tombstones, and replacement relation.
- **Reconnect regression tests** protect the three tier branches, newer adapter-generation stale fence, abnormal disconnect, partial-append recovery, and old-stream inertness. These are the highest-value tests because dishonest cached state is the primary product risk.
- **LoadSnapshot interface tests** protect consistent-prefix lock ordering, view discrimination, stale/corrupt checkpoint rejection, stable ordering, and response/payload domain+LSN equality.
- **Integrated real-server test** protects report → log → resolver → restart → snapshot. Existing session/command/security/diagnostics suites protect sibling regressions.
- **Deferred evidence**: promoted resource-plane vectors and formal properties stay with `epic-agent-operations-resource-plane-conformance`; this feature must not relabel implementation/property tests as checked-model or checked-normative.
- **No low-value tests**: do not test trivial getters or generated serialization independently of behavior; update raw arbitrary server snapshot fixtures to real encoded session snapshots rather than preserving an invalid compatibility path.

## Risks

- **False authoritative claim can delete membership.** An authoritative report makes omission a terminal tombstone. The contract therefore makes the consequence explicit, partial/none never infer deletion, and the capability-manifest sibling must cap the claim against declared external reconstruction ability. Fallback: downgrade the kind to partial; cached omissions become stale, not deleted.
- **No resource generation makes resurrection unsafe.** Terminal exact-identity tombstones require the adapter to supply stable lifetime ids and a distinct replacement identity. This may expose an external API deficiency; the safe fallback is partial/none plus explicit replacement, not silently reusing session generation or resurrecting the target.
- **Schema binding does not prove semantic bytes.** The manifest admission API exactly matches both resource and projection envelope descriptors, and this feature rejects undeclared/mismatched formats, but the core still cannot interpret every adapter schema. Local typed decoders in adapter/cockpit work must fail closed; credentials and LLM data-plane content remain prohibited.
- **Current snapshot table cannot hold multiple projection types safely.** Resource reads bypass it and replay from durable events. This preserves correctness at current scale; if replay cost becomes material, a later typed-checkpoint migration adds a projection key and decoders together.
- **Multi-view normalization is the highest failure surface.** A bad omission or replacement derivation could make replay diverge. One normalized atomic event with explicit `from_revision_lsn`, deterministic sorting, property sequences, and rebuild-after-fold-failure are the fallback controls.
- **Design-time independent review was unavailable.** This worker could not dispatch the warranted advisory pass. The explicit `thorough` feature review is mandatory and should focus on tier downgrade/omission, terminal identity reuse, adapter-generation races, and checkpoint type confusion.

## Extension pressure classification

- **Committed post-v0.1 direction:** exact-identity resource records; per-adapter-kind authoritative/partial/none completeness capped by the sibling manifest declaration; schema-bound resource and projection envelopes; current/stale/unknown reconciliation freshness; core-LSN record/view revisions; typed snapshot/delta report ingress; terminal tombstones and explicit replacement; durable resource event replay; and discriminated session/resource `LoadSnapshot`.
- **Related committed sibling contract:** `capability-manifest` owns admitted target categories/resource kinds, exact payload/projection schema descriptors, and per-kind tier ceilings; this feature consumes that admission contract but does not duplicate it.
- **Reserved seams:** resource checkpoint namespace and periodic materialization; core-generation rejection; adapter sequence numbers; non-terminal identity reactivation with a future explicit lifetime discriminator; cross-domain resource references; promoted formal/conformance evidence; and cockpit rendering.
- **Explicitly rejected for this arc:** resource runtime-session generations, coercing adapter health into session axes, opaque generic Observations mutating resource state, a parallel resource store/RPC, treating partial/none cached data as current, cross-adapter replacement, and storing credentials or model data-plane traffic in resource payloads.

## Other agent review

- Invoked because: the feature is storage/replay-critical and defines destructive authoritative-omission semantics.
- Fixed/active blockers: the design uses explicit normalized mutations, terminal exact-identity tombstones, source-generation fencing, request/response snapshot discrimination, and the sibling manifest's exact kind/tier/payload/projection admission API without duplicating its fields.
- Parked: typed periodic checkpoint namespaces remain reserved until checkpointing is activated or replay cost is measured.
- Rejected: generic Observation folding and a second resource store because they cannot preserve deterministic schema-neutral replay.
- Skipped/degraded: no independent subagent/peer mechanism is exposed in this delegated worker. Direct source verification and pre-mortem were completed; caller-required `thorough` implementation review remains mandatory.

## Implementation summary

All five dependency-ordered checkpoints are complete:

1. Generated Rust/TypeScript contracts define resource records, report/event
   variants, freshness, view completeness/revision, `RESOURCE_STATE`, and
   discriminated snapshots.
2. The canonical `ResourceRegistry` folds and replays durable state with exact
   typed identity, core-LSN revisions, active/tombstoned membership, terminal
   replacement, and atomic corruption handling.
3. Authenticated adapter ingress consumes the landed manifest tier/schema APIs,
   normalizes snapshot/delta reports, fences adapter generations, appends before
   fold, and composes session/resource disconnect degradation into one audited
   batch.
4. The server materializes stable session/resource snapshots, repairs invalid or
   stale session checkpoints, delivers resource events, and all owned session
   callers select the session view explicitly.
5. Real-process report → durable event → restart resolver → resource snapshot
   evidence, generated-sequence reconciliation testing, and rolling foundation
   updates close the integrated boundary without promoting conformance claims.

Implementation stayed with one cohesive host worker because this delegated
harness exposes no generic subagent dispatch tool and schema/core/server/caller
write sets overlap heavily. Worker capability was `openai-codex/gpt-5.6-sol`,
high reasoning, supplied by the harness; the caller explicitly selected
`review_weight: thorough`.

## Integrated verification

- `cargo test --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- Contract Rust/TypeScript builds, `buf generate`, and generated drift — passed.
- CLI tests — 37 passed; web cockpit — 76; web server — 31; Pi adapter — 24,
  including real core/adapter/Pi E2E.
- Model-promotion, conformance-vector metadata, and presentation conformance —
  passed; no resource property/vector was promoted.
- Repository-wide `cargo fmt --check` still reports pre-existing broad Rust
  formatting drift; repository-wide `buf lint` still reports pre-existing RPC
  request/response naming debt. Neither was weakened or attributed to this
  feature.

## Review (2026-08-04)

**Verdict**: Block (required fresh-context path unavailable)

**Blockers**: The explicit `thorough` weight requires iterative fresh-context
review. This delegated endpoint exposes no generic subagent, peer, or agent-mesh
tool, so it cannot truthfully complete or label that required independent path.
The feature remains at `review` for the parent orchestrator to dispatch an
`openai-codex/gpt-5.6-sol` high/xhigh reviewer.
**Important**: none.
**Nits**: none.
**Rejected**: none.

**Notes**: A receiver-owned inline pre-review walked correctness, contract,
replay, tier, generation, snapshot, source-authentication, stream-consumption,
and foundation lenses. It found and fixed four material current-cycle issues in
`87e98d4` and `a8c4b05`: `RESOURCE_STATE` delivery previously triggered the
session cockpit's unsupported-event reconnect loop; UNKNOWN freshness could
retain payloads; later committed events could lower source adapter generation;
and NONE-tier live deltas were incorrectly rejected despite delta omission/
mutation semantics being tier-independent. Full Rust workspace tests,
warnings-denied clippy, web cockpit tests (76), contract drift, model/vector
metadata, and presentation checks are green after those fixes. This inline work
is not represented as fresh-context review and cannot satisfy `thorough`
closure. No lower-risk finding remained to park.

## Thorough cross-model review remediation (2026-08-04)

The parent orchestrator's thorough cross-model pass returned `NEEDS-REVISION`
with three material current-cycle stale/completeness blockers. All three are
fixed, but this feature deliberately remains at `stage: review` for the next
fresh-context convergence pass.

### Material fixes

1. **No-payload `unknown` can no longer become `stale`** (`4195014`). Partial/
   none omission, adapter-generation fencing, disconnect degradation, and
   manifest redeclaration emit `current → stale` only for records carrying both
   cached envelopes. The fold rejects active stale/current records without both
   envelopes, and tombstoning a no-payload record preserves `unknown`.
2. **Authoritative snapshots cannot list surviving unknown resources**
   (`dea8c18`). Both server ingress and core normalization reject
   `snapshot + authoritative + unknown` before resource append or projection
   mutation; surviving authoritative members must be schema-admitted upserts.
3. **Manifest redeclaration degrades resource state before attachment
   publication** (`3eeefc2`). Same-generation removed, down-tiered, and
   schema-incompatible declarations produce a durable degradation event;
   newer-generation attachments fence every prior resource view even without a
   follow-up report. Required degradation and the redacted registration append
   in one audited storage transaction, and the replacement token is installed
   only after successful fold.

### Regression evidence

- UNKNOWN-start tests cover partial/none omission, report-generation fencing,
  disconnect, authoritative-omission tombstone, deterministic replay, and
  resource snapshot materialization; the registry corruption test rejects
  `unknown → stale` without payload.
- Authoritative-unknown tests at core and authenticated server boundaries assert
  unchanged projection state and no durable resource append.
- Same-generation redeclaration covers removal, tier downgrade, and schema
  replacement in one atomic registration; newer-generation attachment proves
  cached state/view degradation with no subsequent resource report.

### Parked lower-risk findings

- `.work/backlog/backlog-resource-generation-obsolete-event-no-op.md` — resolve
  the generation guard versus obsolete-event no-op observer contract with the
  resource conformance work.
- `.work/backlog/backlog-resource-reconciliation-arbitrary-sequences.md` — grow
  the two-report branch sampler into arbitrary report/generation/replacement/
  terminal-attempt traces in the conformance feature.

### Verification after remediation

- `cargo test --workspace` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `buf build proto`, TypeScript contract build, and generated drift — passed.
- Model metadata, conformance-vector, and presentation checks — passed.
- CLI — 37 passed; web cockpit — 76 passed; web server — 31 passed; Pi adapter
  — 24 passed, including the real core/adapter/Pi E2E.
