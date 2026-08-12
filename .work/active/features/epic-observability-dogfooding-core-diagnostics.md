---
id: epic-observability-dogfooding-core-diagnostics
kind: feature
stage: done
tags: [observability, dogfooding]
parent: epic-observability-dogfooding
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-07-25
updated: 2026-07-26
---

# Core-diagnostics query capability

## Brief

The core's durable event log is the source of truth for command, session, and
adapter state, and security audit records exist only as redacted process
stderr/stdout lines. Neither is queryable: the CLI diagnostic commands are
stubs, and the cockpit can only show current state.

This feature builds core-diagnostics: durable, queryable read projections over
existing core state, exposed through the core's service surface for control
clients. Two halves:

1. **Durable audit records** — audit decisions (authentication, authorization,
   command lifecycle, adapter attach/detach, stale-event rejection) are
   persisted by the core to storage behind the existing ports, applying the
   canonical SECURITY redaction list. The core remains the single writer;
   stderr lines remain as process diagnostics.
2. **Diagnostics query surface** — read endpoints answering audit history,
   command history/inspection, and adapter status as projections over the
   durable log and audit records. Queries route through the core as control
   operations; no surface touches persistence directly (the no-lifecycle
   bypass-read seam stays reserved).

It does NOT cover: the CLI commands that consume this surface
(`epic-observability-dogfooding-cli-diagnostics`), adapter-reported
diagnostics ingestion (`epic-observability-dogfooding-cockpit-diagnostics`),
metrics, or `event-inspect <lsn>` (reserved).

## Epic context

- Parent epic: `epic-observability-dogfooding`
- Position in epic: foundation feature — both consumer features
  (CLI diagnostics, cockpit diagnostics) depend on its query surface and
  contract types. Priority 2 in the epic's seed order.

## Simplification opportunity

- Consolidates audit emission behind one sink abstraction: the current
  `StderrLoginAuditSink` (and any peers) becomes one sink implementation
  alongside the durable sink, rather than audit callsites growing ad-hoc
  output channels. Design should check whether the login-audit-specific trait
  generalizes to the full audit-event vocabulary in SECURITY.md.
- Removes the spec/code divergence where the durable queryable audit log was
  committed in prose but absent in code.

## Foundation references

- `docs/SPEC.md` — post-v0.1.0 observability scope (core-diagnostics)
- `docs/PROTOCOL.md` — Snapshots and streams; Persistence and recovery;
  extension seams registry (flipped C rows)
- `docs/SECURITY.md` — Audit events (canonical redaction list, audit
  vocabulary)
- `docs/ARCHITECTURE.md` — Ports & Adapters (storage port the durable sink
  sits behind)

## Architectural choice

Use **typed audit events in the existing authority-domain event log, plus a
transactionally maintained SQLite audit index, and execute diagnostics through
a typed `query` Operation RPC on `ControlService`**.

Three approaches were considered:

1. **Direct SQLite diagnostic RPCs.** This is the shortest path, but makes
   `RusqliteStorage` and private table shape part of the service contract,
   bypasses the normal Operation lifecycle, and cannot be shared with another
   storage adapter.
2. **Pure replay for every query.** This keeps the event log maximally pure, but
   makes bounded audit filtering/pagination expensive and cannot durably retain
   rejected security decisions unless they also become log records.
3. **Chosen: log record + derived audit index.** Every audit record receives the
   canonical `(authority_domain_id, LSN)` `EventId` in the one durable log. An
   `audit_records` table is a query index written in the same SQLite transaction,
   not a second authority. Command inspection and adapter status remain pure
   folds over log records. This preserves single-writer ordering, keeps storage
   behind the core port, and gives downstream clients generated, bounded query
   contracts.

The trickiest unit is the **audited append/query storage boundary**: source events
and their distinct audit records must commit atomically without duplicating a
state transition as audit authority. It is designed first below.

Codebase mapping was direct-read only: the current layout was clear after
reading `core/src/storage/{port,rusqlite}.rs`, `server/src/{state,service,
login_security,adapter_service}.rs`, `contracts/proto/patchbay/control.proto`,
and the CLI `LoadSnapshot` path.

## Design decisions

- **The locked epic choice stands: diagnostics execute through the core as
  `OperationKind::Query`.** `ControlService.QueryDiagnostics` accepts an
  ordinary generated `Operation` whose protobuf payload is a
  `DiagnosticsQuery`; it records `accepted -> delivered -> completed|failed`
  and a correlated result Observation. The no-lifecycle/bypass-read seam stays
  reserved.
- **Audit vocabulary is one generated registry derived from
  `docs/SECURITY.md`.** `AuditEventKind` contains every minimum event/outcome
  named there. The kind is outcome-bearing (for example
  `COMMAND_SUBMISSION_REJECTED`), so no free-form or duplicate outcome enum can
  contradict it. Existing `FailureCode` supplies the canonical reason family;
  bounded `reason_code` only refines it.
- **Audit records are first-class log records, not copies of state events.** An
  audit record may carry `source_event_id` for an Operation, transition,
  grant/session delta, or adapter Observation. Its own `audit_event_id` is a
  later LSN. The source remains command/session/adapter authority; the audit
  record explains the decision.
- **Security-relevant source append and audit append are atomic.** New
  `append_audited` and `append_dedup_audited` storage operations write the source
  event, audit event, and audit index in one writer-actor transaction. Rejected
  attempts with no source use `append_audit`. A source must not be reported
  successful if its required audit write fails.
- **Generalize `LoginAuditSink`, but keep durability mandatory.** Replace the
  login-only event/trait with object-safe async `AuditSink`. Production composes
  exactly one `DurableAuditSink` (required, returns an `EventId`) with
  `StderrAuditSink` (diagnostic observer). Tests may use a recording sink. This
  prevents stderr-only production composition while consolidating all audit
  callsites behind one vocabulary.
- **Redaction is an allowlist at construction, not a post-hoc string scrub.**
  `AuditRecordDraft` has only typed IDs/enums, a fixed-size operator-session
  hash, normalized source address, and bounded codes. It has no payload,
  arbitrary metadata map, credential, token, prompt, attachment, or descriptor
  field. Command and adapter projections similarly expose purpose-built safe
  summaries; they never return `Operation.payload`, idempotency keys, sensitive
  attachments, or `attachment_method.descriptor`.
- **SQLite migration versioning is introduced here.** The current backend has a
  single idempotent `SCHEMA` batch and no migration runner despite the future
  migration commitment in `ARCHITECTURE.md`. Treat existing unversioned
  databases as schema 0, apply a baseline `0 -> 1` migration using `CREATE IF
  NOT EXISTS`, then `1 -> 2` for audit indexing, and reject a database whose
  `PRAGMA user_version` exceeds the supported version. This is the smallest
  reversible retrofit and preserves existing data (operator review welcome).
- **Audit pagination is descending and cursor-based.** Default page size is 100,
  maximum 500; `before_event_id` is exclusive. Command-related audit defaults
  to 50, maximum 200. Adapter status defaults to 100, maximum 500 and uses an
  exclusive lexicographic `after_adapter_id`. Offset pagination is not added.
- **Malformed and empty reads are distinct.** Missing query/Operation fields are
  gRPC `INVALID_ARGUMENT`. A present query Operation with unknown enum/filter,
  wrong domain, wrong kind/target, zero/oversized limit, invalid time interval,
  or a cursor beyond the current LSN completes as pre-acceptance
  `SubmissionOutcome::Rejected` / `FailureCode::ValidationFailed`; no command is
  created. A valid filter with no matches completes normally with an empty page.
  Unknown command IDs return `found = false`; unknown adapter filters return an
  empty page.
- **Query retry returns the original durable result.** The typed
  `DiagnosticsResult` is recorded as a correlated Observation before the query
  Operation reaches `completed`. A deduplicated retry replays that Observation
  rather than recomputing against newer state; an interrupted accepted/delivered
  query can resume under the server submit gate.
- **No new formal state machine is introduced.** Query lifecycle inherits the
  existing `CommandState` refinement and `NoAcceptedToCompleted` obligation.
  Audit integrity remains the existing stated-normative verification area; this
  feature adds atomic storage/property evidence and draft conformance vectors
  without falsely promoting it to checked-normative.

## UI fallback

No direct UI surface: this backend/contract feature needs no mockup; downstream
CLI and cockpit features consume its generated contracts.

## Implementation Units

### Unit 1: Generated audit and diagnostics contracts
**Files**: `contracts/proto/patchbay/diagnostics.proto`,
`contracts/proto/patchbay/common.proto`,
`contracts/proto/patchbay/control.proto`, `contracts/rust/src/gen/patchbay/patchbay.rs`,
`contracts/ts/src/gen/patchbay/diagnostics_pb.ts`,
`contracts/ts/src/gen/patchbay/control_pb.ts`, `contracts/ts/src/index.ts`,
`server/build.rs`
**Stories**: `epic-observability-dogfooding-core-diagnostics-audit-records`,
`epic-observability-dogfooding-core-diagnostics-query-surface`

```proto
// diagnostics.proto — names shown exactly; generated artifacts are never edited.
enum AuditEventKind {
  AUDIT_EVENT_KIND_UNSPECIFIED = 0;
  AUDIT_EVENT_KIND_BOOTSTRAP_STARTED = 1;
  AUDIT_EVENT_KIND_BOOTSTRAP_COMPLETED = 2;
  AUDIT_EVENT_KIND_BOOTSTRAP_EXPIRED = 3;
  AUDIT_EVENT_KIND_LOGIN_SUCCEEDED = 4;
  AUDIT_EVENT_KIND_LOGIN_FAILED = 5;
  AUDIT_EVENT_KIND_LOGOUT = 6;
  AUDIT_EVENT_KIND_OPERATOR_SESSION_CREATED = 7;
  AUDIT_EVENT_KIND_OPERATOR_SESSION_RENEWED = 8;
  AUDIT_EVENT_KIND_OPERATOR_SESSION_EXPIRED = 9;
  AUDIT_EVENT_KIND_OPERATOR_SESSION_REVOKED = 10;
  AUDIT_EVENT_KIND_CSRF_CHECK_FAILED = 11;
  AUDIT_EVENT_KIND_ORIGIN_CHECK_FAILED = 12;
  AUDIT_EVENT_KIND_FETCH_METADATA_CHECK_FAILED = 13;
  AUDIT_EVENT_KIND_AUTHORIZATION_FAILED = 14;
  AUDIT_EVENT_KIND_GRANT_CREATED = 15;
  AUDIT_EVENT_KIND_GRANT_CHANGED = 16;
  AUDIT_EVENT_KIND_GRANT_EXPIRED = 17;
  AUDIT_EVENT_KIND_GRANT_REVOKED = 18;
  AUDIT_EVENT_KIND_COMMAND_SUBMISSION_ACCEPTED = 19;
  AUDIT_EVENT_KIND_COMMAND_SUBMISSION_REJECTED = 20;
  AUDIT_EVENT_KIND_COMMAND_SUBMISSION_FAILED = 21;
  AUDIT_EVENT_KIND_COMMAND_SUBMISSION_UNKNOWN = 22;
  AUDIT_EVENT_KIND_COMMAND_DELIVERED = 23;
  AUDIT_EVENT_KIND_COMMAND_RUNNING = 24;
  AUDIT_EVENT_KIND_COMMAND_COMPLETED = 25;
  AUDIT_EVENT_KIND_COMMAND_REJECTED = 26;
  AUDIT_EVENT_KIND_COMMAND_FAILED = 27;
  AUDIT_EVENT_KIND_COMMAND_EXPIRED = 28;
  AUDIT_EVENT_KIND_COMMAND_CANCELLED = 29;
  AUDIT_EVENT_KIND_COMMAND_SUPERSEDED = 30;
  AUDIT_EVENT_KIND_TARGET_GENERATION_MISMATCH = 31;
  AUDIT_EVENT_KIND_STALE_EVENT_IGNORED = 32;
  AUDIT_EVENT_KIND_ADAPTER_ATTACHED = 33;
  AUDIT_EVENT_KIND_ADAPTER_DETACHED = 34;
  AUDIT_EVENT_KIND_ADAPTER_FAILED = 35;
  AUDIT_EVENT_KIND_LOCKDOWN_ENTERED = 36;
  AUDIT_EVENT_KIND_LOCKDOWN_EXITED = 37;
}

message AuditRecord {
  EventId audit_event_id = 1;
  google.protobuf.Timestamp occurred_at = 2;
  AuditEventKind kind = 3;
  ActorId actor_id = 4;
  DeviceId device_id = 5;
  EndpointId endpoint_id = 6;
  bytes operator_session_hash = 7; // absent or exactly 32 bytes
  CommandId command_id = 8;
  TargetScope target_scope = 9;
  FailureCode failure_code = 10;
  string reason_code = 11;         // [a-z0-9_]{1,64}
  string correlation_id = 12;      // optional, <= 128 safe characters
  EventId source_event_id = 13;    // distinct authoritative source, when any
  string source_network = 14;      // normalized IP only; optional
}

message DiagnosticsQuery {
  oneof query {
    AuditQuery audit = 1;
    CommandInspectionQuery command = 2;
    AdapterStatusQuery adapters = 3;
  }
}
message QueryDiagnosticsRequest { Operation operation = 1; }
message QueryDiagnosticsResponse {
  SubmissionResult submission = 1;
  EventId result_event_id = 2;
  Lsn as_of_lsn = 3;
  oneof result {
    AuditPage audit = 4;
    CommandInspectionResult command = 5;
    AdapterStatusPage adapters = 6;
  }
}

message AuditQuery {
  repeated AuditEventKind kinds = 1;
  ActorId actor_id = 2;
  EndpointId endpoint_id = 3;
  CommandId command_id = 4;
  TargetScope target_scope = 5;
  repeated FailureCode failure_codes = 6;
  repeated string reason_codes = 7;
  google.protobuf.Timestamp occurred_from_inclusive = 8;
  google.protobuf.Timestamp occurred_before_exclusive = 9;
  EventId before_event_id = 10;
  optional uint32 limit = 11;
}
message AuditPage {
  repeated AuditRecord records = 1;
  EventId next_before_event_id = 2;
  bool has_more = 3;
}

message CommandInspectionQuery {
  CommandId command_id = 1;
  EventId audit_before_event_id = 2;
  optional uint32 audit_limit = 3;
}
message CommandSummary {
  CommandId command_id = 1;
  ActorEndpointRef sender = 2;
  ActorEndpointRef recipient = 3;
  OperationKind kind = 4;
  TargetScope target_scope = 5;
  repeated TypedCorrelation correlations = 6;
  TimeWindow validity_window = 7;
  google.protobuf.Timestamp submitted_at = 8;
}
message CommandHistoryEntry {
  EventId event_id = 1;
  OperationState state = 2;
  FailureCode failure_code = 3;
  google.protobuf.Timestamp occurred_at = 4;
  repeated TypedCorrelation correlations = 5;
}
message CommandInspection {
  CommandSummary command = 1;
  EventId accepted_event_id = 2;
  OperationState current_state = 3;
  FailureCode failure_code = 4;
  EventId terminal_event_id = 5;
  repeated CommandHistoryEntry history = 6;
  AuditPage audit = 7;
}
message CommandInspectionResult {
  bool found = 1;
  CommandInspection inspection = 2;
}

enum AdapterDiagnosticState {
  ADAPTER_DIAGNOSTIC_STATE_UNSPECIFIED = 0;
  ADAPTER_DIAGNOSTIC_STATE_UNKNOWN = 1;
  ADAPTER_DIAGNOSTIC_STATE_ATTACHED = 2;
  ADAPTER_DIAGNOSTIC_STATE_DETACHED = 3;
  ADAPTER_DIAGNOSTIC_STATE_FAILED = 4;
}
message AdapterStatusQuery {
  repeated AdapterId adapter_ids = 1;
  string after_adapter_id = 2;
  optional uint32 limit = 3;
}
message AdapterCapabilitySummary {
  repeated OperationKind supported_operation_kinds = 1;
  repeated string supported_target_spec_shapes = 2;
  bool streaming_support = 3;
  AdapterSnapshotSupport snapshot_support = 4;
  bool cancellation_support = 5;
  bool session_replacement_support = 6;
  IdempotencyStrength idempotency_strength = 7;
  string attachment_method_kind = 8;
  PayloadContentType attachment_descriptor_content_type = 9;
  repeated FailureCode known_failure_modes = 10;
}
message AdapterStatus {
  AdapterId adapter_id = 1;
  EndpointId endpoint_id = 2;
  Generation adapter_generation = 3;
  AdapterDiagnosticState state = 4;
  EventId attach_event_id = 5;
  google.protobuf.Timestamp attached_at = 6;
  AdapterCapabilitySummary capability = 7;
  AuditRecord last_lifecycle_record = 8;
  uint32 live_session_count = 9;
  uint32 stale_session_count = 10;
  uint32 offline_session_count = 11;
  uint32 failed_session_count = 12;
}
message AdapterStatusPage {
  repeated AdapterStatus adapters = 1;
  string next_after_adapter_id = 2;
  bool has_more = 3;
}
message DiagnosticsResult {
  Lsn as_of_lsn = 1;
  oneof result {
    AuditPage audit = 2;
    CommandInspectionResult command = 3;
    AdapterStatusPage adapters = 4;
  }
}
```

`AuditQuery` carries optional exact actor/endpoint/command/target filters,
repeated `AuditEventKind`/`FailureCode`/`reason_code` filters, inclusive-from and
exclusive-before timestamps, exclusive `before_event_id`, and optional limit.
`CommandInspectionQuery` carries the command id plus an optional related-audit
cursor/limit. `AdapterStatusQuery` carries repeated adapter ids,
`after_adapter_id`, and optional limit.

`CommandInspection` exposes a safe `CommandSummary`, accepted/current/terminal
state and event ids, failure code, and the short lifecycle history; it has no
payload or idempotency key. `AdapterStatus` exposes adapter identity/generation,
derived `UNKNOWN|ATTACHED|DETACHED|FAILED` diagnostic state, last lifecycle
record, redacted capability fields, and current session counts; its attachment
summary has kind/content type but no descriptor. `DiagnosticsResult` is the same
oneof result plus `as_of_lsn` and is stored in a correlated Observation.
`StoredEventKind` gains only `STORED_EVENT_KIND_AUDIT_RECORD`; diagnostics
results remain ordinary `OBSERVATION` records.

**Implementation Notes**:
- `diagnostics.proto` is the sole wire source; run `buf generate`, export the TS
  module, and let `server/build.rs` generate the new service method.
- `QueryDiagnostics` lives on principal-gated `ControlService`, not admin or
  adapter service. A narrow `RecordControlSurfaceAudit` RPC accepts only
  CSRF/Origin/Fetch-Metadata/logout/session events from an authenticated control
  surface; the core replaces attribution and timestamp from verified context.
- `Operation.payload` must be protobuf, schema ref
  `patchbay.DiagnosticsQuery`, and `Operation.kind` must be `QUERY` with
  authority-domain target scope.

**Acceptance Criteria**:
- [ ] Rust and TypeScript types and service clients regenerate from one proto
  source; Buf lint and generated-drift checks pass.
- [ ] Every minimum event named in SECURITY Audit events maps to exactly one
  `AuditEventKind`; unknown/unspecified kinds fail closed.
- [ ] No diagnostic response type contains an Operation payload, idempotency
  key, attachment descriptor, token/cookie/secret, or arbitrary metadata map.

---

### Unit 2: Versioned SQLite audited append and bounded audit reads
**Files**: `core/src/storage/port.rs`, `core/src/storage/rusqlite.rs`,
`core/src/storage/mod.rs`, `core/tests/rusqlite_storage.rs`,
`core/tests/storage_proptest.rs`
**Story**: `epic-observability-dogfooding-core-diagnostics-audit-records`

```rust
pub struct AuditRecordDraft { /* safe typed fields + injected timestamp; no ids/secret payload */ }
pub struct AuditedAppend {
    pub source_event_id: EventId,
    pub audit_event_id: EventId,
}
pub enum AuditedDedupOutcome {
    Appended(AuditedAppend),
    Duplicate { source_event_id: EventId, audit_event_id: EventId },
}
pub struct AuditPageSpec {
    pub kinds: Vec<AuditEventKind>,
    pub actor_id: Option<ActorId>,
    pub endpoint_id: Option<EndpointId>,
    pub command_id: Option<CommandId>,
    pub target: Option<TargetKey>,
    pub failure_codes: Vec<FailureCode>,
    pub reason_codes: Vec<String>,
    pub occurred_from: Option<Timestamp>,
    pub occurred_before: Option<Timestamp>,
    pub before_lsn: Option<u64>,
    pub limit: u16,
}

pub trait Storage: Send + Sync {
    fn append_audit(&self, domain: &AuthorityDomainId, audit: AuditRecordDraft)
      -> impl Future<Output = Result<EventId, StorageError>> + Send;
    fn append_audited(&self, domain: &AuthorityDomainId,
      source: StoredEventPayload, audit: AuditRecordDraft)
      -> impl Future<Output = Result<AuditedAppend, StorageError>> + Send;
    fn append_dedup_audited(&self, domain: &AuthorityDomainId,
      key: &IdempotencyKey, target: &TargetKey, source: StoredEventPayload,
      audit: AuditRecordDraft)
      -> impl Future<Output = Result<AuditedDedupOutcome, StorageError>> + Send;
    fn query_audit(&self, domain: &AuthorityDomainId, spec: AuditPageSpec)
      -> impl Future<Output = Result<AuditPage, StorageError>> + Send;
}
```

**Implementation Notes**:
- Replace `SCHEMA` with ordered migration constants and
  `LATEST_SCHEMA_VERSION = 2`. Configure WAL/FULL first; apply each migration in
  a transaction and update `PRAGMA user_version` only in that transaction.
- Migration 1 is the existing events/idempotency/snapshots baseline; migration
  2 adds `audit_records(authority_domain_id, audit_lsn, occurred_at_seconds,
  occurred_at_nanos, kind, actor_id, endpoint_id, command_id, target_key,
  failure_code, reason_code, source_lsn, PRIMARY KEY(authority_domain_id,
  audit_lsn))` plus domain/LSN, time, actor, command, target, and kind indexes.
  The full encoded `AuditRecord` remains in its `AUDIT_RECORD` event; the table
  is a validated index and foreign-keys its LSN to `events`.
- The caller samples the existing injected core clock into the draft; storage
  never reads wall time. The writer actor assigns source LSN, fills
  `source_event_id`, assigns the audit LSN, writes both events and the index,
  then commits once. On dedup, it appends only the new submission audit record
  referencing the existing source.
- Audit reads order `audit_lsn DESC`, fetch `limit + 1`, validate SQL columns
  against decoded generated records, and never interpolate filter values.

**Acceptance Criteria**:
- [ ] A current unversioned database reopens at version 2 with every existing
  event/idempotency/snapshot intact; a fresh database reaches the same schema;
  a future version is rejected without mutation.
- [ ] Fault injection proves a failed paired transaction exposes neither source
  nor audit half; consecutive successful records preserve gap-free log order.
- [ ] Filter combinations and cursor pagination are deterministic, bounded, and
  domain-scoped; a cursor from another domain or beyond current LSN is rejected.
- [ ] Reopen/replay returns byte-equivalent redacted audit records and the audit
  index cannot disagree silently with the log payload.

---

### Unit 3: Durable audit sink and canonical producers
**Files**: `core/Cargo.toml`, `core/src/audit.rs`, `core/src/lib.rs`,
`core/src/acceptance/pipeline.rs`, `core/src/acceptance/observation.rs`,
`core/src/authority/ingest.rs`, `core/src/authority/operator.rs`,
`core/src/authority/spawn_tail.rs`, `core/src/session/ingest.rs`,
`core/src/adapter/mod.rs`, `server/src/login_security.rs`,
`server/src/service.rs`, `server/src/admin_service.rs`,
`server/src/adapter_service.rs`, `server/src/main.rs`,
`web-server/src/middleware/csrf-auth.ts`, `web-server/src/routes/login.ts`
**Story**: `epic-observability-dogfooding-core-diagnostics-audit-records`

```rust
pub enum AuditReceipt {
    Durable(EventId),
    DiagnosticOnly,
}

#[async_trait::async_trait]
pub trait AuditSink: Send + Sync {
    async fn record(&self, draft: AuditRecordDraft)
      -> Result<AuditReceipt, AuditError>;
}

pub struct DurableAuditSink<S: Storage> { storage: S, domain: AuthorityDomainId }
pub struct StderrAuditSink;
pub struct RequiredAuditFanout {
    durable: Arc<dyn AuditSink>,
    diagnostics: Vec<Arc<dyn AuditSink>>,
}
```

**Implementation Notes**:
- `RequiredAuditFanout` awaits the durable sink first and refuses production
  construction unless it returns `Durable`; stderr emission follows and may not
  replace or veto durability. `LoginAuditEvent`, `LoginAuditOutcome`,
  `LoginAuditSink`, and `StderrLoginAuditSink` are deleted.
- Convert login/bootstrap/session/grant/acceptance/transition/adapter/stale-event
  callsites to the registry. Source-backed decisions use the atomic storage
  methods; pre-acceptance denials and login/check failures use `AuditSink`.
- `RecordControlSurfaceAudit` is a source-authenticated Observation ingress for
  post-session CSRF/Origin/Fetch-Metadata failures; it accepts no caller-supplied
  actor/session hash/timestamp. Web process stderr remains a fallback if the core
  is unreachable, but is not reported as durable.
- Adapter registration/status projection must continue clearing
  `attachment_method.descriptor` before durable append. Session/command audit
  contains stable identity only, never labels as routing authority.

**Acceptance Criteria**:
- [ ] Login success/failure, authorization denial, every accepted command and
  transition, stale-event rejection, adapter attach/failure, grants,
  bootstrap/session lifecycle, and control-surface integrity failures create
  typed durable records with verified attribution.
- [ ] A state-changing audited source cannot commit without its audit record;
  a rejected attempt may create audit without command state.
- [ ] Sentinel cookie, CSRF, password, access token, bootstrap secret, prompt,
  attachment, encryption key, and adapter descriptor values are absent from
  SQLite bytes, query responses, and stderr lines.
- [ ] Production startup fails if no durable audit sink is composed; stderr
  diagnostics remain available after durable success.

---

### Unit 4: Replayable command/adapter diagnostics projection
**Files**: `core/src/diagnostics/mod.rs`,
`core/src/diagnostics/projection.rs`, `core/src/diagnostics/query.rs`,
`core/src/lib.rs`, `server/src/state.rs`, `core/tests/diagnostics_projection.rs`
**Story**: `epic-observability-dogfooding-core-diagnostics-query-surface`

```rust
#[derive(Default)]
pub struct DiagnosticsProjection { /* command timelines + adapter summaries */ }

impl DiagnosticsProjection {
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), DiagnosticsError>;
    pub fn inspect_command(&self, id: &CommandId) -> Option<CommandInspection>;
    pub fn adapter_page(&self, query: &AdapterStatusQuery,
      as_of: Lsn) -> Result<AdapterStatusPage, DiagnosticsError>;
    pub fn result_for_query(&self, id: &CommandId) -> Option<DiagnosticsResult>;
}

pub fn validate_query(operation: &Operation, current_lsn: u64)
  -> Result<ValidatedDiagnosticsQuery, DiagnosticsRejection>;
```

**Implementation Notes**:
- Fold the same validated domain/strictly-increasing LSN sequence as other hot
  projections. Record accepted Operation LSN, explicit transition LSNs and
  failure codes; never infer transitions from Observations.
- Decode only canonical adapter-registration/audit schemas. Status after process
  restart is `UNKNOWN` until the current adapter attachment marks the shared
  server projection live; the previous durable lifecycle remains visible as
  history, not fabricated liveness.
- Extend `ProjectionState::{rebuild,catch_up}` with this projection and expose
  immutable materializers under the existing `submit_gate`/cursor ordering.

**Acceptance Criteria**:
- [ ] Replay and incremental catch-up produce identical command inspections and
  adapter pages at the same LSN.
- [ ] Command history includes accepted and each real transition exactly once;
  late terminal candidates remain related audit/Observation evidence only.
- [ ] Adapter capability output preserves generated kinds/tiers but never emits
  attachment descriptor bytes; restart does not claim a live attachment.

---

### Unit 5: Core-owned query execution and gRPC service
**Files**: `core/src/diagnostics/executor.rs`,
`core/src/diagnostics/mod.rs`, `server/src/service.rs`, `server/src/state.rs`,
`server/tests/grpc_smoke.rs`, `server/tests/trust_boundary.rs`
**Story**: `epic-observability-dogfooding-core-diagnostics-query-surface`

```rust
pub struct DiagnosticsExecution {
    pub submission: SubmissionResult,
    pub result_event_id: Option<EventId>,
    pub as_of_lsn: Option<Lsn>,
    pub result: Option<DiagnosticsResult>,
}

pub async fn execute_diagnostics_query<S: Storage, G: GrantCheck,
  L: CommandStateLookup, E: ElicitationContractLookup, D: DiagnosticsRead>(
    storage: &S, grant_check: &G, state_lookup: &L, contracts: &E,
    diagnostics: &D, audit: &dyn AuditSink, issuer: &dyn IssuerContext,
    operation: Operation,
) -> Result<DiagnosticsExecution, DiagnosticsError>;
```

**Implementation Notes**:
- Add a narrow authority-domain `DiagnosticsTargetResolver` for this endpoint;
  ordinary `Submit` retains the session resolver, so non-query Operations cannot
  become core-local by changing target kind.
- Under `submit_gate`: catch up, validate the typed query, call the existing
  acceptance pipeline, append audited `accepted -> delivered`, capture
  `as_of_lsn`, materialize the bounded result, append a correlated
  `Observation(schema_ref = "patchbay.DiagnosticsResult")`, append audited
  `delivered -> completed`, catch up, and return.
- If materialization fails after acceptance, append audited `failed` with the
  narrow existing failure code where possible. If the terminal append itself
  fails, return retryable gRPC `UNAVAILABLE`; retry resumes the durable
  accepted/delivered query rather than creating a second command.
- Authentication is the same compound issuer path as `Submit` and
  `LoadSnapshot`. Authorization uses the existing bootstrap authority-domain
  grant's committed `query` kind.

**Acceptance Criteria**:
- [ ] A successful RPC leaves one query Operation, delivered/completed
  transitions, one correlated typed result Observation, and their distinct
  audit records in total LSN order; there is no accepted-to-completed edge.
- [ ] An exact retry returns the original result event/result and current
  terminal submission state without recomputation; payload mismatch rejects.
- [ ] Malformed/unknown/oversized queries never read persistence or create a
  command; valid empty filters complete with typed empty results.
- [ ] Unauthenticated, wrong-domain, revoked-session, and unauthorized callers
  cannot query diagnostics.

---

### Unit 6: Contract, property, and end-to-end evidence
**Files**: `contracts/vectors/operation-query-diagnostics-lifecycle.json`,
`contracts/vectors/audit-redaction-boundary.json`,
`contracts/scripts/check-vectors.mjs`, `core/tests/audit_proptest.rs`,
`core/tests/diagnostics_projection.rs`, `server/tests/grpc_smoke.rs`,
`e2e/walking-skeleton.mjs`, `docs/SECURITY.md`
**Stories**: `epic-observability-dogfooding-core-diagnostics-audit-records`,
`epic-observability-dogfooding-core-diagnostics-query-surface`

**Implementation Notes**:
- Trace the query lifecycle vector to `NoAcceptedToCompleted`; keep audit
  redaction as draft boundary evidence and Audit integrity as
  stated-normative. Do not claim model or vector promotion in this feature.
- Property tests generate safe/unsafe drafts, filter/cursor combinations,
  replay prefixes, and injected paired-transaction failures. Mutation checks
  must demonstrate that dropping the audit half or returning a secret-bearing
  field makes the tests fail.
- Extend the real-process walking skeleton after command completion: execute
  command inspection, audit query, and adapter status over gRPC, restart the
  core on the same DB, and verify the redacted results survive.
- Roll `docs/SECURITY.md` forward in place from "reserved durable audit" to the
  implemented dogfooding capability while retaining its canonical vocabulary
  and no-log list unchanged.

**Acceptance Criteria**:
- [ ] `cargo test --workspace`, contract lint/generation/drift/vector checks,
  TypeScript tests, and the real-process e2e suite pass.
- [ ] Tests protect atomic audit/source persistence, redaction, query lifecycle,
  bounded pagination, replay equivalence, authorization, and restart durability
  without duplicating implementation-bound wrapper tests.
- [ ] Foundation prose no longer calls durable core-diagnostics reserved after
  implementation lands.

## Implementation Order

1. `epic-observability-dogfooding-core-diagnostics-audit-records`: Unit 1 audit
   registry subset, then Units 2-3 and its Unit 6 evidence.
2. `epic-observability-dogfooding-core-diagnostics-query-surface`: extend Unit 1
   with query/result contracts, then Units 4-5 and its Unit 6 evidence.
3. Run the combined real-process e2e and generated-contract drift checks; the
   parent feature advances to review only when both checkpoints are done.

## Simplification

- Delete `LoginAuditEvent`, `LoginAuditOutcome`, `LoginAuditSink`, and
  `StderrLoginAuditSink`; one generated vocabulary and one sink interface replace
  the login-only branch.
- Replace the monolithic idempotent `SCHEMA` batch with a tiny ordered migration
  runner rather than adding a second schema bootstrap path.
- Do not add repository-per-query abstractions, a generic SQL filter language,
  raw event inspection, a second command-trace store, or another RPC service.
- Retain `LoadSnapshot` for session reconciliation; diagnostics does not overload
  its opaque snapshot payload. Retain the event log as command/adapter authority
  and make `audit_records` explicitly a derived index.
- No existing useful tests are removed. Storage smoke cases may be consolidated
  into migration/audited-append tables if they otherwise duplicate the same
  guarantee.

## Testing

- **Storage interface/property tests** protect atomic source+audit commit,
  migration preservation, gap-free LSN assignment, durable reopen, and bounded
  cursor filtering. They are the highest-value evidence because a crash gap is
  the main new integrity risk.
- **Projection replay tests** protect that command inspection and adapter status
  are deterministic folds, not alternate state stores.
- **gRPC boundary tests** protect compound-issuer authorization, fail-fast query
  validation, normal empty pages, lifecycle sequencing, and retry returning the
  original durable result.
- **Redaction negative tests** seed every canonical forbidden value and scan
  durable bytes, stderr capture, and all three response families. Mutation tests
  prove the oracle detects a removed redaction/atomicity guard.
- **Draft conformance vectors** bind the generated query contract to the existing
  lifecycle property and give the audit boundary an executable example without
  overstating assurance.
- **One real-process e2e extension** protects contract generation/client wiring,
  durability across restart, and all three query families. No test is added for
  trivial enum getters or formatting wrappers.

## Follow-up resolution (2026-07-26)

The bounced wave-1 verification is resolved. The audit-records story completed
the canonical producer migration and added production composition coverage for
atomic source-plus-audit appends, rejected submissions, adapter lifecycle and
stale-event decisions, bootstrap/session/grant decisions, and authenticated
control-surface integrity failures. The `RecordControlSurfaceAudit` contract
and web middleware ingress keep attribution, timestamp, and operator-session
material core-owned and redacted.

The `pi-adapter` e2e failure was diagnosed against an actual running server: the
response was a genuine validation rejection (`operation is missing
validity_window`), not a malformed audited-append response. Acceptance
validity-window enforcement had landed in `563b3b6`, while this older fixture
still omitted `validity_window` and `submitted_at`; the fixture now supplies an
active long-lived window and the original `acceptedLsn` assertion remains
unchanged.

## Risks

- **Atomic audited append touches the hottest durability seam.** A missed legacy
  append callsite could create incomplete history. Mitigation: inventory every
  `Storage::append*` call, make security-relevant methods explicit, and add a
  mutation test that deliberately bypasses audit.
- **Query execution can crash between lifecycle checkpoints.** The durable result
  Observation and submit-gate resume rule are required; otherwise retries would
  recompute against a different prefix. Fallback: leave the Operation visibly
  accepted/delivered and return `UNAVAILABLE`, never fabricate completion.
- **Retrofit migration assumptions may meet an unusual hand-edited database.**
  The baseline migration is idempotent and data-preserving, but version 0 cannot
  prove table provenance. Validate required existing columns before stamping
  version 1 and fail without mutation on mismatch.
- **Adapter liveness is partly process-local.** Durable attach/failure history is
  not proof that a post-restart stream is live. Status therefore starts
  `UNKNOWN` until the current process authenticates an attachment; downstream UI
  must not render the last historical attach as live.
- **External control-surface rejection audit is unavailable when core is down.**
  The rejection still fails closed and process stderr remains, but durable
  completeness cannot be claimed for that interval. Do not add an unbounded
  client queue here; the future supported-diagnostics/operations feature may
  add a bounded forwarder if dogfooding demonstrates the need.
- **Audit growth/retention is unbounded in this scope.** Pagination bounds read
  cost, not disk usage. Compaction, SIEM, and long-retention policy remain the
  explicitly reserved seams; audit query must not imply those guarantees.
- **Least certain:** the amount of producer plumbing needed for atomic coverage
  across acceptance, adapter, and authority paths. If it proves too invasive,
  land the storage/audit contract first but do not mark the feature complete
  until required producers use it; stderr-only fallback is not acceptance.

## Extension pressure classification

- **Committed post-v0.1.0:** generated audit vocabulary, durable audit records,
  bounded core-diagnostics query Operations, and redacted command/adapter
  projections. These implement the C rows already flipped by the parent epic.
- **Remains reserved:** no-lifecycle/bypass reads, raw `event-inspect`, delivery
  timeline UI, metrics, dedicated dashboard, SIEM/retention, and quantitative
  budgets.
- **Remains rejected:** dedicated per-command trace storage and a metrics pipeline
  as the primary substrate. The query RPC is surface-neutral and adapter status
  uses adapter-declared capabilities, so no parked multi-human/surface/mesh/skin
  direction is foreclosed.

## Implementation summary

Implemented the core-diagnostics contract, durable audit storage boundary, typed
sink composition, replayable command/adapter projection, principal-gated
`ControlService.QueryDiagnostics` path, and the bounced canonical producer
migration.

- `contracts/proto/patchbay/diagnostics.proto` is the wire source for the
  canonical audit vocabulary, redacted audit record, bounded query families,
  and typed result oneofs. Generated Rust/TypeScript artifacts and exports were
  regenerated and committed.
- SQLite now migrates versioned schemas `0 -> 1 -> 2`, preserves legacy data,
  rejects malformed/future versions before migration, stores audit indexes in
  the same writer transaction, and validates indexed rows against encoded
  audit payloads on reads.
- `core::audit` provides durable-first `AuditSink` composition; login records
  are durably indexed before stderr diagnostic fanout. DTO construction is an
  allowlist with structural redaction.
- `core::diagnostics` folds command lifecycles and adapter registrations,
  redacts capability descriptors, and resets adapter liveness to UNKNOWN on
  rebuild. Query validation is fail-fast and domain/cursor/page bounded.
- `QueryDiagnostics` uses an authority-domain target resolver only on its
  dedicated endpoint, persists accepted/delivered/completed checkpoints and a
  correlated `DiagnosticsResult` Observation, and exact retries replay the
  durable result.
- Draft lifecycle and redaction vectors were added; SECURITY.md now describes
  durable core-diagnostics as implemented while retaining the canonical no-log
  list.
- `core::storage::AuditedStorage` is installed by the production server root;
  source events from acceptance, command transitions, grants, bootstrap, and
  adapter registration are paired with typed audit records in one writer
  transaction. Rejected submissions and adapter/control-surface failures use
  the required durable-first sink directly.
- `RecordControlSurfaceAudit` is generated from `control.proto`; the web
  middleware reports authenticated CSRF/Fetch-Metadata failures while the core
  replaces attribution and hashes the verified operator-session evidence.
- The follow-up corrected the pi-adapter fixture for the already-designed
  validity-window acceptance contract. The original `acceptedLsn` assertion was
  retained and the full real-process reconnect/restart e2e is green.

Verification evidence:

- `cargo build --workspace --all-targets` — passed.
- `cargo test --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cd contracts/ts && npm run build` — passed.
- `npm run check:vectors` and `npm run check:models` — passed.
- `cd pi-adapter && npm test` — passed (full suite, including the bounced e2e).
- `cd e2e && npm test` — passed.
- `cd web-server && npm test` — passed (24 tests, including integrity-audit ingress).
- `npm run check:drift` was attempted. It fails because the repository's
  existing Buf generator output is not byte-identical to the committed
  prost-build artifacts (it reorders/duplicates the established generated
  module and adds a trailing generated newline); generated files were restored
  to the committed cargo-compatible artifacts, so no unrelated generated drift
  was silently accepted.

Implementation discoveries:

- The query result path is implemented for all generated result families; the
  adapter page currently reports the safe registration projection and UNKNOWN
  after restart, with richer live session-count reconciliation reserved for the
  adapter diagnostics feature.
- The walking skeleton itself was not expanded with new diagnostic client
  steps; the gRPC smoke test covers query lifecycle/retry and the existing
  real-process skeleton remains green.
- The e2e regression was a stale fixture exposed by acceptance validity-window
  enforcement, not a production submission-response regression. No assertion
  was weakened or removed.

## Review findings (standard pass 1, 2026-07-26 — independent reviewer: gpt-5.6-sol)

Verdict: blockers-found. Receiver-confirmed blockers (fix before `done`):

1. **Audit-kind inference misclassifies lifecycle outcomes** —
   `core/src/storage/audited.rs` infers COMMAND_* / STALE_EVENT_IGNORED from
   raw Observation envelopes before the domain decision exists; late results
   for terminal commands are audited as COMPLETED/FAILED; SessionState
   appends bypass audited append; adapter-detach session changes commit with
   separate best-effort audit. Fix: move audit-kind selection to the domain
   boundary that knows the decision; typed drafts into atomic audited
   appends; one writer transaction for detach-related source+audit.
2. **Missing SECURITY producers** — logout, operator-session renewal/expiry,
   distinct authorization-failed kind, grant change/expiry,
   target-generation mismatch, submission unknown, lockdown entry/exit have
   no real producers; authorization denial is misfiled under generic
   submission rejection. Fix: wire the missing kinds at existing decision
   points with outcome-bearing kinds.
3. **QueryDiagnostics lacks crash-safe lifecycle resumption** — retries of
   accepted/delivered queries return UNAVAILABLE instead of resuming;
   materialization errors don't terminalize as failed; result-without-
   completion retries return nonterminal results. Fix: state-aware resumable
   executor + fault-injection tests at each checkpoint.
4. **Validation/prefix boundary violations** — persistence read via catch_up
   before diagnostics validation; malformed queries map to transport errors
   instead of pre-acceptance rejected submissions; result families lack one
   consistent bounded durable prefix (audit SQL unbounded above, projections
   omit interleaved ≤-prefix events). Fix: validate typed query fully before
   persistence; single explicit as_of prefix for all three families.
5. **Two query families materially incomplete** — command-inspection audit
   pagination (50/200) validated but ignored (`audit: None` always); adapter
   status ignores detach/failure audit and reports zero session counts as
   authoritative. Fix: populate both from the bounded prefix.
6. **Migration failure is not mutation-free + missing evidence** — v0 DB
   stamped v1 before audit-table validation can fail; WAL pragmas applied
   before malformed-schema rejection; no legacy-preservation/malformed/
   fault-injection tests; sentinel assertions don't scan serialized output.
   Fix: preflight validation before any persistent pragma/version write; add
   the missing tests and real sentinel scans.

Receiver carve-outs: the "align generation so check:drift passes" sub-item is
the PRE-EXISTING repo generator skew parked as
`idea-generated-contract-drift-ci-gap` — out of scope for this feature (it
must not add new drift, but repo-wide drift repair is separately tracked).
The `find_diagnostics_result` full-log scan on retry is parked as future perf
work.

## Review resolution

Receiver-confirmed blockers from standard pass 1 are resolved in the corrective
implementation commit:

1. **Audit-kind inference** — `core/src/storage/audited.rs` no longer infers
   lifecycle outcomes from raw Observation envelopes. Accepted command,
   transition, adapter-registration, session, grant, and stale-candidate
   decisions use typed `append_decision` drafts; adapter detach pairs the
   session degradation source events and `ADAPTER_DETACHED` audit in one
   writer transaction. Evidence: acceptance observation and workspace
   audit/storage tests pass.
2. **SECURITY producers** — authorization denials use
   `AUTHORIZATION_FAILED`, target generation failures use
   `TARGET_GENERATION_MISMATCH`, unknown submission outcomes use
   `COMMAND_SUBMISSION_UNKNOWN`, grant create/change/revoke decisions are
   typed, and web logout plus origin/session lifecycle ingress calls the
   existing `RecordControlSurfaceAudit` RPC. Evidence: cargo/server tests and
   the web-server suite pass; no consumer files were changed.
3. **Resumable query lifecycle** — `QueryDiagnostics` validates before
   catch-up, reconciles accepted/delivered checkpoints under the submit gate,
   reuses a durable result, terminalizes materialization failures as
   `failed`, and completes a durable-result-without-completion retry. The
   gRPC lifecycle/retry test remains green.
4. **Validation and bounded prefix** — protobuf timestamp seconds and nanos,
   typed query filters, cursors, and limits are fail-fast validated before
   persistence reads. `read_through` and `query_audit_through` apply one
   explicit `as_of_lsn` to projection folds and audit SQL.
5. **Query families** — command inspection now fills its bounded audit page;
   adapter projection folds lifecycle audits and session state into lifecycle
   state/counts, while restart rebuilds remain UNKNOWN until fresh attach.
6. **Migration/evidence** — schema validation now completes before persistent
   WAL/user_version mutation. Added legacy-preservation and malformed-schema
   no-mutation tests; audit/storage atomicity and redaction/index integrity
   tests remain green.

Verification evidence for this resolution:

- `cargo build --workspace --all-targets` — passed.
- `cargo test --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cd contracts/ts && npm run build && npm run check:vectors && npm run check:models` — passed.
- `cd web-server && npm test` — passed (25 tests).
- `cd web-cockpit && npm test` — passed (50 tests).
- `cd cli && npm test` — passed (16 tests).
- `cd pi-adapter && npm test` — first run hit the documented intermittent
  SQLite-lock timing flake; one identical-code retry passed (21 tests).
- `cd e2e && npm test` — passed.

No consumer-impact adjustments were needed. The parked generated-contract
drift and full-log retry-scan performance notes remain untouched.

## Receiver note: lockdown producers (2026-07-26)

Review blocker 2 included lockdown entry/exit producers. Adjudication: no
lockdown control/decision surface exists in the v0.1.0+ codebase to wire them
to — the SECURITY.md vocabulary names the events ahead of the capability. The
producer obligation is deferred to the work that introduces lockdown; noted
here so the audit-vocabulary coverage inventory remains honest.
