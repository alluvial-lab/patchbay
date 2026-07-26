---
id: epic-observability-dogfooding-cockpit-diagnostics
kind: feature
stage: done
tags: [observability, dogfooding, ui]
parent: epic-observability-dogfooding
depends_on: [epic-observability-dogfooding-core-diagnostics]
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-26
---

# Adapter diagnostics forwarding + cockpit surfacing

## Brief

The cockpit is the surface the operator actually has open while dogfooding,
but it shows nothing about adapter health: no connection state, no diagnostic
events, no adapter-side errors. Adapter failures are invisible until a session
silently goes stale.

This feature closes that gap end to end. The pi-adapter reports diagnostics
to the core as payload — promoting the reserved adapter-specific diagnostics
seam — the core records them (extending the core-diagnostics surface), and the
cockpit presents adapter health, connection state, and recent diagnostic
events within its existing views. One capability, three layers; kept as a
single feature because the contract shape, core recording, and presentation
must agree on the same diagnostic vocabulary.

The presentation composes into existing cockpit surfaces and reuses the
existing CommandState/status presentation patterns — no net-new screen, so no
mockups at epic tier per the mockup-first convention's skip rule. If feature
design finds the diagnostics presentation wants a dedicated view rather than
composition, it falls back to `/ux-ui-design:screens` at that point.

It does NOT cover: the adapter-local durable log file
(`epic-observability-dogfooding-adapter-log-sink`), the base core-diagnostics
query surface (`epic-observability-dogfooding-core-diagnostics`), a dedicated
health/status dashboard, or the delivery-trace timeline UI (both reserved).

## Epic context

- Parent epic: `epic-observability-dogfooding`
- Position in epic: consumer of `epic-observability-dogfooding-core-diagnostics`
  (its recording/query substrate) and producer of the adapter-diagnostics
  contract addition. Parallel with the CLI consumer. Priority 4 in the epic's
  seed order, but the highest dogfooding value — the cockpit is the primary
  inspection surface.

## Simplification opportunity

- Gives the cockpit an honest adapter-health signal, replacing the current
  implicit "silence until stale" behavior — sessions going `stale` with no
  visible cause is a presentation-honesty gap this feature closes at the
  source.
- Adapter diagnostic codes map onto the PROTOCOL failure vocabulary at the
  Patchbay boundary; design should reuse that mapping rather than inventing a
  parallel cockpit-only vocabulary.

## Foundation references

- `docs/PROTOCOL.md` — Payload; failure vocabulary (adapter diagnostic codes
  extension seam); extension seams registry
- `docs/UX.md` — delivery-state floor, presentation honesty
- `docs/SPEC.md` — post-v0.1.0 observability scope
- `docs/ADAPTER-PI.md` — adapter capability declarations

## Architectural choice

Use a **typed adapter diagnostic payload carried over a narrow adapter-facing
report RPC, atomically append the source Observation plus its correlated audit
record in the existing authority-domain log, extend `AdapterStatusPage` with a
bounded recent-audit slice, and compose that status into the existing cockpit
session list/detail/timeline**.

Three approaches were considered:

1. **Chosen: typed Observation payload + audited append.** The adapter sends a
   `PayloadEnvelope(schema_ref = "patchbay.AdapterDiagnosticPayload")` through
   `AdapterControlService.ReportDiagnostics`. The core validates and
   canonicalizes the authenticated source, then uses core-diagnostics'
   `append_audited` path to commit one safe `OBSERVATION` and one
   `ADAPTER_DIAGNOSTIC_REPORTED` audit record atomically. Live cockpit streams
   see the Observation; reconnect/inspection uses the same audit/adapter-status
   projection. This preserves source authentication, the single writer, the
   audit redaction discipline, and one diagnostics query surface.
2. **A new stored-event/table/channel for adapter diagnostics.** A dedicated
   `STORED_EVENT_KIND_ADAPTER_DIAGNOSTIC`, SQLite table, and web endpoint would
   make querying straightforward, but would create a parallel observability
   authority beside the audit/diagnostics substrate and duplicate attribution,
   retention, redaction, and replay rules. It is rejected.
3. **Fold diagnostics into `SessionReport` or infer them from silence.** This
   has fewer wire types, but conflates session state with adapter-process
   evidence, cannot represent adapter-wide or command-correlated failures, and
   risks quietly implementing the reserved heartbeat/last-report-age policy.
   It is rejected.

The trickiest unit is the **authenticated report → atomic source/audit append →
replayable status projection seam**. It must preserve adapter/session
attribution without trusting caller identity, retain only allowlisted fields,
and remain useful live and after reconnect without turning diagnostics into a
second liveness authority. It is designed first below.

Codebase mapping was direct-read only. The three layers were clear after
reading `contracts/proto/patchbay/{adapter,adapter_control,common,control,
observations}.proto`, `pi-adapter/src/{core_client,main}.ts`,
`server/src/adapter_service.rs`, `web-server/src/{core-client,routes/rpc}.ts`,
and `web-cockpit/src/{main,domain/model,domain/protocol-client,ui/session-list,
ui/session-detail,ui/shell}.ts`; no unresolved web structure question warranted
an Explore dispatch.

## Design decisions

- **Adapter diagnostics are source Observations with distinct audit records.**
  The Observation is the authenticated adapter fact and the audit record
  explains the accepted report; `AuditRecord.source_event_id` links them. Both
  commit through `append_audited`, exactly matching the upstream
  core-diagnostics source/audit distinction. Neither changes command, session,
  or adapter lifecycle state.
- **The RPC is narrow, but storage/query are not parallel.**
  `ReportDiagnostics` exists so the boundary can accept only the diagnostic
  envelope and return canonical `validation_failed`; it does not create a new
  service, store, writer, or browser route family. The report is normalized to
  an ordinary generated `Observation` before durable append.
- **Adapter-specific codes stay adapter-declared strings.** The generic wire
  contract owns severity, canonical `FailureCode`, identity/correlation, count,
  and bounds; the Pi adapter owns one mapping from its JSONL event registry to
  safe codes such as `pi_delivery_failed`. The capability manifest declares
  that code set. The core validates shape but does not use cached capability as
  an authority or ingestion gate.
- **Canonical failure mapping is mandatory for warn/error reports.** Every
  warning/error carries a known non-`UNSPECIFIED` `FailureCode`; informational
  lifecycle reports may use `UNSPECIFIED`. The same code is rendered by the
  cockpit's existing failure vocabulary. There is no cockpit-only failure enum.
- **The report schema is an allowlist, not arbitrary metadata.** It carries only
  code, severity, adapter generation, optional generated `OperationKind`, and a
  bounded coalesced count. Target scope, typed command correlation, observed
  time, and failure code are outer generated fields. There is no message,
  payload body, metadata map, error text/stack/cause, prompt, transcript, tool
  content, attachment, descriptor, path, model, token, or credential field.
- **Reporting is event-driven and best-effort.** The Pi adapter forwards adapter
  start/stop, reattach outcome when reportable, registration/disposal failure,
  delivery subscription failure/retry, delivery rejection/failure, and
  observation/flush failure from the sibling diagnostics registry. It does not
  forward transcript deltas or successful per-command delivery checkpoints.
  Initial attach failure cannot be sent without an authenticated attachment and
  remains in the adapter-local durable JSONL log.
- **Rate bounds protect the control loop, not a performance SLA.** `record()`
  only sanitizes/enqueues. One sequential drain sends at most 10 reports/second,
  holds at most 256 pending keys, coalesces identical pending records up to
  `count = 1000`, applies a one-second RPC timeout, and drops on overflow or
  failure. Flush/close are non-throwing and bounded to one second. No report is
  retried and no forwarding failure recursively emits another report.
- **Diagnostics never establish liveness.** Adapter connection comes only from
  the upstream `AdapterStatus.state` projection over authenticated attachment,
  stream-loss, and lifecycle audit. Diagnostic timestamps, report cadence, and
  silence never move adapter/session connectivity. Heartbeats, freshness
  deadlines, timers, and last-report-age policy remain reserved.
- **Cockpit reads status once per evidence change, not on a timer.** It queries
  the selected session's adapter after initial reconciliation, selection
  change, reconnect, or selected-session connectivity change. Typed diagnostic
  Observations update the recent list live. There is no polling interval; an
  older query `as_of_lsn` cannot overwrite a newer streamed diagnostic.
- **“Health” is presented honestly as evidence, not a new protocol state.** The
  cockpit shows adapter connection (`attached|detached|failed|unknown`) plus
  “recent reported issue” or “no recent reported issues.” It never labels an
  adapter healthy merely because no report arrived. `AdapterDiagnosticState`
  maps explicitly to the existing connectivity-indicator presentation
  (`attached→live`, `detached→offline`, `failed→failed`, `unknown→unknown`)
  without changing protocol state.
- **Query Operations retain normal lifecycle and CSRF.** The browser builds an
  `OperationKind.QUERY` carrying `DiagnosticsQuery`; the web server applies the
  same verified compound issuer and server-stamped validity window as `Submit`.
  `QueryDiagnostics` is CSRF-protected because it durably records a lifecycle,
  even though its product effect is read-only.
- **Dispatch/review posture:** direct mapping was sufficient. The design's
  highest risks are the security boundary and cross-layer reconciliation; the
  implementation remains one feature ownership bundle with three durable
  checkpoints rather than one worker per story.

## UI fallback

**No mockups.** Re-evaluation confirms composition is sufficient: session rows
and the session-detail header reuse the existing connectivity indicator for the
adapter connection, and recent warn/error reports reuse existing alert,
failure-banner, and timeline/list primitives. There is no dedicated route,
dashboard, novel panel, navigation change, palette change, or new shared
component variant. The reserved dedicated health/status dashboard is the shape
that would require `/ux-ui-design:screens`; this feature does not promote it.

## Implementation Units

### Unit 1: Generated adapter diagnostic and query extensions

**Files:** `contracts/proto/patchbay/adapter.proto`,
`contracts/proto/patchbay/diagnostics.proto`,
`contracts/proto/patchbay/adapter_control.proto`,
`contracts/rust/src/gen/patchbay/patchbay.rs`,
`contracts/ts/src/gen/patchbay/{adapter,diagnostics,adapter_control}_pb.ts`,
`contracts/ts/src/index.ts`, `server/build.rs`

**Story:** `epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion`

```proto
// adapter.proto
message AdapterDiagnosticReportingCapability {
  repeated string diagnostic_codes = 1; // each [a-z0-9_]{1,64}; max 128
}

message AdapterCapability {
  // existing fields 1-9 unchanged
  AdapterDiagnosticReportingCapability diagnostic_reporting = 10;
}

// diagnostics.proto
// Existing AuditEventKind values 1-37 from core-diagnostics remain unchanged.
// AUDIT_EVENT_KIND_ADAPTER_DIAGNOSTIC_REPORTED = 38;

enum AdapterDiagnosticSeverity {
  ADAPTER_DIAGNOSTIC_SEVERITY_UNSPECIFIED = 0;
  ADAPTER_DIAGNOSTIC_SEVERITY_INFO = 1;
  ADAPTER_DIAGNOSTIC_SEVERITY_WARNING = 2;
  ADAPTER_DIAGNOSTIC_SEVERITY_ERROR = 3;
}

message AdapterDiagnosticPayload {
  string code = 1;                    // adapter-declared [a-z0-9_]{1,64}
  AdapterDiagnosticSeverity severity = 2;
  Generation adapter_generation = 3; // must match authenticated attachment
  OperationKind operation_kind = 4;  // optional/UNSPECIFIED when unrelated
  uint32 count = 5;                   // 1..1000
}

message AdapterDiagnosticDetail {
  AdapterId adapter_id = 1;
  Generation adapter_generation = 2;
  AdapterDiagnosticSeverity severity = 3;
  OperationKind operation_kind = 4;
  uint32 count = 5;
  google.protobuf.Timestamp adapter_observed_at = 6;
}

message AuditRecord {
  // existing core-diagnostics fields 1-14 unchanged
  AdapterDiagnosticDetail adapter_diagnostic = 15;
}

message AdapterStatusQuery {
  // existing adapter_ids/after_adapter_id/limit fields 1-3 unchanged
  optional uint32 recent_diagnostic_limit = 4; // absent=0; present 1..100
}

message AdapterCapabilitySummary {
  // existing redacted fields 1-9 unchanged
  AdapterDiagnosticReportingCapability diagnostic_reporting = 10;
}

message AdapterStatus {
  // existing fields 1-12 unchanged
  repeated AuditRecord recent_diagnostics = 13; // newest first, bounded by query
}

// adapter_control.proto
message AdapterDiagnosticReport {
  AuthorityDomainId authority_domain_id = 1;
  TargetScope target_scope = 2;       // ADAPTER or complete RUNTIME_SESSION
  repeated TypedCorrelation correlations = 3; // zero or one CommandId
  google.protobuf.Timestamp observed_at = 4;
  FailureCode failure_code = 5;
  PayloadEnvelope payload = 6;        // PROTOBUF / patchbay.AdapterDiagnosticPayload
}

message AdapterDiagnosticReportResult {
  bool accepted = 1;
  EventId observation_event_id = 2;
  EventId audit_event_id = 3;
  FailureCode failure_code = 4;
}

service AdapterControlService {
  // existing methods unchanged
  rpc ReportDiagnostics(AdapterDiagnosticReport)
      returns (AdapterDiagnosticReportResult);
}
```

**Implementation Notes:**

- `diagnostics.proto` remains the wire source used by the upstream
  core-diagnostics feature. Run `buf generate`; never hand-edit generated Rust
  or TS. Preserve upstream field numbers and append only the numbers shown.
- The exact payload schema ref is `patchbay.AdapterDiagnosticPayload`; content
  type must be `PROTOBUF`. Unknown/unspecified enum numbers fail closed.
- Attach validation constrains the declared code list lexically and by count,
  but diagnostic ingestion does not reject a valid code merely because an
  older cached manifest omitted it; capabilities remain advisory.
- `AuditRecord.reason_code` stores the exact diagnostic code,
  `AuditRecord.failure_code` stores the canonical mapping,
  `AuditRecord.target_scope`/`command_id` store attribution, and
  `adapter_diagnostic` stores only the generated safe detail. No generic detail
  map is added.
- A syntactically valid, authenticated but malformed report returns
  `accepted=false`, `failure_code=VALIDATION_FAILED`, and no event ids. Missing
  authentication remains gRPC `UNAUTHENTICATED`; storage/internal failure is a
  non-accepting gRPC error, never a false report result.

**Acceptance Criteria:**

- [ ] Rust and TypeScript artifacts regenerate from the same proto source; Buf
  lint/breaking/generated-drift checks pass.
- [ ] The manifest, report, audit, and adapter-status messages expose the exact
  bounded fields above and no arbitrary text or opaque diagnostic payload beyond
  the one exact generated schema.
- [ ] Existing core-diagnostics clients that omit `recent_diagnostic_limit`
  receive no embedded recent list; the cockpit explicitly requests at most 20.
- [ ] Unspecified/unknown severity, unknown/reserved operation kinds, unknown
  failure codes, and invalid target/payload values reject fail-closed without a
  durable append; `OperationKind.UNSPECIFIED` is allowed only when unrelated,
  and `FailureCode.UNSPECIFIED` only for INFO.

---

### Unit 2: Authenticated validation and atomic audited ingestion

**Files:** `core/src/diagnostics/adapter_report.rs` (new),
`core/src/diagnostics/mod.rs`, `core/src/audit.rs`,
`server/src/adapter_service.rs`, `server/src/adapter_service/tests.rs`

**Story:** `epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion`

```rust
pub struct ValidatedAdapterDiagnostic {
    pub observation: Observation,
    pub audit: AuditRecordDraft,
}

pub struct AdapterDiagnosticReceipt {
    pub observation_event_id: EventId,
    pub audit_event_id: EventId,
}

pub fn validate_adapter_diagnostic_report(
    report: AdapterDiagnosticReport,
    authenticated_adapter: &AdapterId,
    registration: &AdapterRegistration,
    received_at: prost_types::Timestamp,
) -> Result<ValidatedAdapterDiagnostic, AdapterDiagnosticRejection>;

pub async fn ingest_adapter_diagnostic<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    diagnostic: ValidatedAdapterDiagnostic,
) -> Result<AdapterDiagnosticReceipt, DiagnosticsError>;
```

**Implementation Notes:**

- `AdapterControlServiceImpl::report_diagnostics` uses the existing attachment
  evidence/token authentication and current registration. It samples the
  injected core clock once for the audit timestamp; the adapter's validated
  `observed_at` remains separate evidence in `AdapterDiagnosticDetail`.
- Validation requires configured authority domain, current adapter id and
  adapter generation, target kind `ADAPTER` for adapter-wide events or a fully
  populated same-adapter runtime-session identity, zero/one non-empty CommandId
  correlation, exact payload content/schema, a code matching
  `[a-z0-9_]{1,64}`, count `1..=1000`, and known enum values. Warning/error
  requires a non-unspecified canonical failure; INFO permits unspecified.
- Reconstruct the durable `Observation` from verified registration/context:
  sender actor is the authenticated adapter principal, endpoint is the current
  registered endpoint, authority domain is configured, kind is `EVENT`, and
  only validated target/correlation/time/failure/payload survive. Never persist
  a caller-supplied sender or endpoint.
- Encode that source as `StoredEventKind::Observation`; build
  `AuditEventKind::AdapterDiagnosticReported` with `source_event_id`, verified
  actor/endpoint/target/command, exact code in `reason_code`, canonical failure,
  and safe detail; commit via the upstream `Storage::append_audited` operation.
- Diagnostics never call command observation ingestion and therefore cannot
  imply a `CommandTransition`. A stale/tombstoned session target may still be
  retained as diagnostic evidence but never mutates the live generation.

**Acceptance Criteria:**

- [ ] One accepted report produces exactly one source Observation followed by
  one correlated audit record in the same all-or-nothing writer transaction.
- [ ] Authentication/attachment context, not payload claims, determines
  adapter id, endpoint, authority domain, and current adapter generation.
- [ ] Every malformed case returns `validation_failed` with no source/audit
  half; injected transaction failure exposes neither half.
- [ ] A diagnostic correlated to a command or stale session is queryable as
  evidence but never changes command/session/adapter state.
- [ ] Forbidden sentinel strings are absent from encoded source, audit,
  SQLite bytes, gRPC result, and safe process diagnostics.

---

### Unit 3: Replayable recent diagnostics in adapter status

**Files:** `core/src/diagnostics/projection.rs`,
`core/src/diagnostics/query.rs`, `core/tests/diagnostics_projection.rs`,
`server/tests/grpc_smoke.rs`

**Story:** `epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion`

```rust
const MAX_RECENT_ADAPTER_DIAGNOSTICS: usize = 100;

impl DiagnosticsProjection {
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), DiagnosticsError>;
    pub fn adapter_page(
        &self,
        query: &AdapterStatusQuery,
        as_of: Lsn,
    ) -> Result<AdapterStatusPage, DiagnosticsError>;
}

pub fn validate_recent_diagnostic_limit(
    value: Option<u32>,
) -> Result<usize, DiagnosticsRejection>;
```

**Implementation Notes:**

- Extend the upstream fold; do not create another projection. On
  `AUDIT_RECORD/ADAPTER_DIAGNOSTIC_REPORTED`, validate source id, adapter detail,
  target, reason/failure, and strictly increasing LSN, then retain the newest
  100 audit records per adapter. Replay and catch-up use the same observer.
- `recent_diagnostic_limit` absent means zero; present accepts `1..=100`.
  Materialization returns newest-first records no newer than the query's
  `as_of_lsn`. Existing adapter pagination stays unchanged.
- Adapter status remains based only on authenticated lifecycle evidence from
  core-diagnostics. A diagnostic report never changes
  `AdapterDiagnosticState`, session counts, capabilities, or last lifecycle
  record.

**Acceptance Criteria:**

- [ ] Replay and incremental catch-up at the same LSN return identical status
  and recent records, ordered newest-first and bounded exactly by the request.
- [ ] A core restart still reports adapter state `UNKNOWN` until a current
  authenticated attachment; historical diagnostics remain visible as history.
- [ ] Absent/valid/no-match recent limits behave normally; zero/oversized
  present limits reject before query acceptance with `validation_failed`.
- [ ] Adapter capability summaries include only declared code strings and the
  upstream redacted fields; attachment descriptors remain absent.

---

### Unit 4: Shared Pi diagnostics port and best-effort core forwarder

**Files:** `pi-adapter/src/core_diagnostics_forwarder.ts` (new),
`pi-adapter/src/adapter_diagnostics.ts`, `pi-adapter/src/core_client.ts`,
`pi-adapter/src/main.ts`, `pi-adapter/tests/core_diagnostics_forwarder.test.ts`
(new), `pi-adapter/tests/delivery.test.ts`, `pi-adapter/tests/e2e.test.ts`

**Story:** `epic-observability-dogfooding-cockpit-diagnostics-adapter-forwarding`

```ts
import {
  AdapterDiagnosticSeverity,
  type AdapterDiagnosticReport,
  type AdapterDiagnosticReportResult,
} from "@patchbay/contracts";
import type {
  AdapterDiagnosticEvent,
  AdapterDiagnosticInput,
  AdapterDiagnostics,
} from "./adapter_diagnostics.js";

export const PI_FORWARDED_DIAGNOSTIC_CODES = {
  "adapter.started": "pi_adapter_started",
  "adapter.stopping": "pi_adapter_stopping",
  "adapter.attach.failed": "pi_adapter_attach_failed",
  "session.register.failed": "pi_session_register_failed",
  "session.dispose.failed": "pi_session_dispose_failed",
  "delivery.subscription.failed": "pi_delivery_subscription_failed",
  "delivery.subscription.retrying": "pi_delivery_subscription_retrying",
  "delivery.rejected": "pi_delivery_rejected",
  "delivery.failed": "pi_delivery_failed",
  "observation.failed": "pi_observation_failed",
  "observation.flush_failed": "pi_observation_flush_failed",
} as const satisfies Partial<Record<AdapterDiagnosticEvent, string>>;

export interface CoreDiagnosticsForwarderOptions {
  maxPending?: number;       // production default 256
  reportsPerSecond?: number; // production default 10
  reportTimeoutMs?: number;  // production default 1000
  maxFlushMs?: number;       // production default 1000
  now?: () => Date;
  delay?: (milliseconds: number) => Promise<void>;
}

export class CoreDiagnosticsForwarder implements AdapterDiagnostics {
  constructor(
    report: (value: AdapterDiagnosticReport) => Promise<AdapterDiagnosticReportResult>,
    context: { authorityDomainId: string; adapterId: string; adapterGeneration: number },
    options?: CoreDiagnosticsForwarderOptions,
  );
  record(input: AdapterDiagnosticInput): void;
  flush(): Promise<void>;
  close(): Promise<void>;
}

export function composeAdapterDiagnostics(
  sinks: readonly AdapterDiagnostics[],
): AdapterDiagnostics;

// PatchbayCoreClient: deliberately bypasses #postAttach retry.
reportDiagnostic(report: AdapterDiagnosticReport): Promise<AdapterDiagnosticReportResult>;
```

**Implementation Notes:**

- Extend the sibling adapter-log-sink port rather than duplicating its
  instrumentation. The local JSONL sink receives every existing record; the
  forwarder filters through the one mapping above. The manifest's
  `diagnosticReporting.diagnosticCodes` derives from `Object.values(...)`.
- Each mapped event constructs an exact protobuf payload. Session context
  becomes a runtime-session `TargetScope`, otherwise adapter scope; command id
  becomes one typed correlation. Map/carry only generated OperationKind and
  FailureCode. Classify Connect timeout as `TRANSPORT_TIMEOUT`, attachment/
  subscription unavailability as `ADAPTER_UNAVAILABLE`, delivery refusal as
  its existing narrow code, and local execution/observation failures as the
  narrow existing code recorded at the instrumentation callsite.
- Do not forward arbitrary `reason` or `error`. Events lacking a defensible
  canonical warning/error mapping remain local-only. Initial attach failure is
  attempted but normally cannot authenticate; the local durable log is the
  intended fallback.
- `record()` catches mapping/enqueue errors. A single promise drain coalesces
  identical `(code,target,command,failure,operation)` pending keys, sends at
  least 100 ms apart, times out each call after one second, never retries, and
  discards rejection/error. Queue overflow preferentially coalesces or drops;
  it never blocks or recursively records forwarding failure.
- `reportDiagnostic` does not use `#postAttach`: diagnostics must not trigger
  token refresh or compete with control traffic. Normal attach/delivery calls
  retain their current reattach semantics. Fanout catches every sink so a
  broken local or forwarding sink cannot veto another sink or the control loop.

**Acceptance Criteria:**

- [ ] The generated manifest code set and forwarded code set come from the one
  Pi mapping object; no core/cockpit copy enumerates Pi codes.
- [ ] Event construction preserves safe adapter/session/command/generation/
  failure attribution and excludes all content-bearing local fields.
- [ ] A burst cannot exceed 10 network sends/second or 256 pending keys;
  matching records coalesce to at most 1000 and overflow remains non-blocking.
- [ ] RPC reject/auth loss/timeout, a throwing sink, and flush/close deadline
  leave attach, delivery, observation, and shutdown outcomes unchanged.
- [ ] No forwarding failure recursively reports itself and no empty/timed
  report is ever emitted.

---

### Unit 5: Authenticated gRPC-Web diagnostics query path

**Files:** `web-server/src/routes/rpc.ts`,
`web-server/tests/integration.test.ts`,
`web-cockpit/src/domain/protocol-client.ts`,
`web-cockpit/tests/protocol-client.test.ts`

**Story:** `epic-observability-dogfooding-cockpit-diagnostics-cockpit-composition`

```ts
// web-server/src/routes/rpc.ts
function stampVerifiedOperation(
  operation: Operation | undefined,
  request: FastifyRequest,
  nowMs?: number,
): void;

// web-cockpit/src/domain/protocol-client.ts
const LIFECYCLE_RPC_METHODS = new Set(["Submit", "QueryDiagnostics"]);
```

**Implementation Notes:**

- Add `/patchbay.ControlService/QueryDiagnostics` to the existing binary
  gRPC-Web bridge using generated request/response schemas and the same unary
  framing/error helpers. Require an operator session and CSRF because the query
  creates durable Operation lifecycle/audit records.
- Extract the current `Submit` sender/time-window replacement into
  `stampVerifiedOperation` and call it for both methods. The browser's sender
  and timestamps remain non-authoritative; compound issuer headers remain
  unchanged.
- Extend the browser interceptor to attach CSRF to both lifecycle RPCs. Do not
  add a REST route, JSON DTO, direct SQLite read, or special no-lifecycle query.

**Acceptance Criteria:**

- [ ] Connect-Web `QueryDiagnostics` forwards generated protobuf, verified
  actor/session/principal headers, and server-stamped validity exactly like
  `Submit`.
- [ ] Missing/invalid/revoked browser session or CSRF fails before a core call;
  core `UNAUTHENTICATED` invalidates the matching browser session.
- [ ] Browser and server tests fail if either method bypasses the shared
  stamping/CSRF gate or uses hand-copied response data.

---

### Unit 6: Adapter diagnostics query/controller and LSN-safe model merge

**Files:** `web-cockpit/src/domain/adapter-diagnostics.ts` (new),
`web-cockpit/src/domain/model.ts`, `web-cockpit/src/main.ts`,
`web-cockpit/tests/model.test.ts`, `web-cockpit/tests/main.test.ts`

**Story:** `epic-observability-dogfooding-cockpit-diagnostics-cockpit-composition`

```ts
export interface AdapterDiagnosticView {
  sourceEventId: string; // authority-domain + source LSN key
  lsn: bigint;
  adapterId: string;
  adapterGeneration: bigint;
  target?: SessionIdentity;
  commandId?: string;
  severity: AdapterDiagnosticSeverity;
  code: string;
  failureCode?: FailureCode;
  operationKind?: OperationKind;
  count: number;
  observedAt?: Date;
}

export interface AdapterView {
  adapterId: string;
  status?: AdapterStatus;
  asOfLsn: bigint;
  recentDiagnostics: readonly AdapterDiagnosticView[];
}

export interface PresentationModel {
  // existing fields unchanged
  adapters: Map<string, AdapterView>;
}

export function buildAdapterStatusQueryOperation(
  authorityDomainId: AuthorityDomainId,
  adapterId: string,
  ids: { commandId: string; idempotencyKey: string },
): Operation;

export function mergeAdapterStatusResult(
  model: PresentationModel,
  response: QueryDiagnosticsResponse,
): PresentationModel;

export function foldAdapterDiagnosticObservation(
  model: PresentationModel,
  observation: Observation,
  lsn: bigint,
): void;
```

**Implementation Notes:**

- Build `DiagnosticsQuery{adapters:{adapter_ids:[selected],limit:1,
  recent_diagnostic_limit:20}}` as protobuf payload on an authority-domain
  target `OperationKind.QUERY`; call only through generated
  `QueryDiagnostics`.
- On initial reconciled selection, selection change, reconnect, or a change to
  the selected session's connectivity, issue at most one in-flight query for
  that adapter/trigger tuple. Do not use a timer. Selection callback is added
  to `CockpitShellOptions`; the shell remains presentation-only.
- `foldObservation` recognizes only the exact diagnostic schema and validates
  the generated payload, verified target, one command correlation, code/count,
  and source LSN. It adds evidence but never changes adapter status or session
  connectivity. Existing transcript and query-result Observations retain their
  current handling.
- Query audit records and live source Observations normalize into the same view
  keyed by source EventId. Merge keeps a live diagnostic with LSN greater than
  `response.as_of_lsn`; an older response cannot remove it. Keep newest 20 per
  adapter in browser memory. Snapshot replacement clears adapter status and
  causes a fresh query rather than presenting old attachment as live.
- A failed/rejected query leaves adapter status unknown and preserves newer
  live diagnostic evidence; it surfaces a bounded existing failure banner and
  does not mark the session unreconciled.

**Acceptance Criteria:**

- [ ] The query operation uses the generated schema, normal query lifecycle,
  authority-domain target, unique id/key, selected adapter, and explicit limit
  20.
- [ ] Query/live races deduplicate by source EventId and preserve every record
  newer than the response LSN; reconnect never retains stale attached status.
- [ ] Diagnostics cannot mutate command/session/adapter lifecycle state or make
  stale connectivity look live.
- [ ] Query triggers are evidence-driven and deduplicated; no periodic request,
  empty report, heartbeat, last-report-age, or silence-derived health exists.

---

### Unit 7: Existing-view cockpit composition

**Files:** `web-cockpit/src/ui/session-list.ts`,
`web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/shell.ts`,
`web-cockpit/src/ui/shell.css`, `web-cockpit/tests/shell.test.ts`,
`contracts/scripts/check-presentation.mjs`

**Story:** `epic-observability-dogfooding-cockpit-diagnostics-cockpit-composition`

```ts
export function adapterConnectionPresentation(
  state: AdapterDiagnosticState,
): {
  connectivity: "live" | "offline" | "failed" | "unknown";
  label: "attached" | "detached" | "failed" | "unknown";
};

export function renderAdapterStatus(
  document: Document,
  adapter: AdapterView | undefined,
): HTMLElement;

export function diagnosticsForSession(
  adapter: AdapterView | undefined,
  session: SessionIdentity,
): readonly AdapterDiagnosticView[];
```

**Implementation Notes:**

- Session rows keep the existing session connectivity/activity badge and append
  a compact, labelled adapter connection indicator only when queried evidence
  exists. The detail header shows the full adapter indicator and either
  “recent reported issue” or “no recent reported issues.” Adapter status never
  replaces session status.
- Map all generated adapter states explicitly:
  `ATTACHED→connectivity-indicator--live`,
  `DETACHED→--offline`, `FAILED→--failed`, and
  `UNKNOWN|UNSPECIFIED→--unknown`. Labels retain adapter vocabulary, so the CSS
  reuse does not claim the enums are identical. Extend the presentation checker
  with a derived-member mapping from `AdapterDiagnosticState` to the already
  checked connectivity CSS/showcase bindings; add no new CSS state variant.
- Insert adapter-wide plus matching-session diagnostics into the existing
  session timeline by source LSN. Warning/error uses `failure-banner` with
  `failureCodeName`; informational evidence uses the existing base `alert`.
  Show safe code, adapter generation, count, and observed time only. Never show
  an arbitrary diagnostic message.
- Empty query results say “no recent reported issues,” query failure says
  “adapter diagnostics unavailable,” and unknown connection remains visibly
  unknown. Preserve existing mobile drill-in and desktop two-pane composition,
  ARIA roles, and timeline scroll behavior.

**Acceptance Criteria:**

- [ ] Adapter and session connection signals remain separately labelled; all
  adapter states render through the exhaustive generated-enum mapping, and the
  presentation conformance check proves every member reaches an existing
  checked CSS/showcase binding without inventing a new protocol state.
- [ ] Recent events are ordered/deduplicated, scoped to the selected adapter and
  session (plus adapter-wide events), and use canonical failure terms.
- [ ] Absence, query failure, detached, failed, and unknown never render as
  healthy/live; stale session dominance remains unchanged.
- [ ] Desktop/mobile shell, accessibility, filtering, detail scroll, command
  delivery, Elicitation, and composer regression tests remain green.

---

### Unit 8: Cross-layer evidence and rolling foundation updates

**Files:** `contracts/vectors/audit-redaction-boundary.json`,
`contracts/scripts/check-vectors.mjs`, `server/tests/grpc_smoke.rs`,
`pi-adapter/tests/e2e.test.ts`, `web-server/tests/integration.test.ts`,
`web-cockpit/tests/{model,shell,main}.test.ts`, `e2e/walking-skeleton.mjs`,
`docs/PROTOCOL.md`, `docs/SECURITY.md`, `docs/UX.md`, `docs/ADAPTER-PI.md`

**Stories:** all three child checkpoints

**Implementation Notes:**

- Extend the upstream draft audit-redaction vector with a diagnostic source/
  audit pair; keep `AuditIntegrity` stated-normative and do not claim a new
  checked state-machine property. No `specs/` model changes are warranted:
  diagnostics add evidence and a query projection but no lifecycle or liveness
  transition.
- Extend the real-process walking skeleton after Pi attach: wait for the
  automatically reported `pi_adapter_started` event, execute adapter-status
  through `QueryDiagnostics`, and assert the safe recent record survives core
  restart. The cockpit generated-client/model/DOM tests cover the surface
  translation without adding a browser automation framework solely here.
- Roll `docs/PROTOCOL.md` forward from reserved adapter diagnostic codes to the
  committed post-v0.1.0 capability/report mapping, and state explicitly that
  reporting is not heartbeat/liveness. Update `docs/SECURITY.md` to make the
  structural adapter-report allowlist and no arbitrary text rule explicit,
  `docs/UX.md` to describe the existing-view composition and honest labels,
  and `docs/ADAPTER-PI.md` with the Pi diagnostic-reporting manifest
  declaration. `docs/SPEC.md` already states the promoted scope and needs no
  additive coverage unless implementation reveals a contradiction.

**Acceptance Criteria:**

- [ ] Workspace Rust tests, all TypeScript package tests, Buf lint/breaking/
  generation/drift/vector checks, and the real-process e2e pass.
- [ ] Cross-layer evidence protects authenticated attribution, atomic append,
  redaction, report failure isolation/rate bounds, replay/restart, query
  lifecycle/CSRF, LSN-safe merge, and honest presentation.
- [ ] Foundation docs consistently classify adapter diagnostic reporting as
  committed post-v0.1.0 while heartbeat, metrics, dashboard, delivery trace,
  SIEM/retention, and bypass reads remain reserved.

## Implementation Order

1. `epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion` —
   Units 1-3: generated contract, authenticated atomic ingestion, and bounded
   adapter-status projection.
2. `epic-observability-dogfooding-cockpit-diagnostics-adapter-forwarding` —
   Unit 4: reuse the adapter diagnostics port and land failure-isolated Pi
   forwarding against the generated endpoint.
3. `epic-observability-dogfooding-cockpit-diagnostics-cockpit-composition` —
   Units 5-7: authenticated browser query, LSN-safe model merge, and existing-
   view composition.
4. Complete Unit 8 across the checkpoints: real-process report/query/restart
   evidence, full package verification, and code-first foundation roll-forward.

The parent feature remains one cohesive implementation/review bundle. The
stories are heterogeneous contract/core, adapter-failure-isolation, and UI
acceptance checkpoints, not separate worker assignments by default.

## Simplification

- Reuse one adapter diagnostics instrumentation port for both local JSONL and
  core forwarding; do not instrument attach/delivery/observation callsites a
  second time.
- Reuse `append_audited`, `AuditEventKind`/`AuditRecord`,
  `DiagnosticsProjection`, and `QueryDiagnostics`; add no diagnostics table,
  repository, store, service, raw-event endpoint, or browser REST DTO.
- Extract web operation stamping/CSRF method classification once instead of
  copying the `Submit` trust-boundary logic into `QueryDiagnostics`.
- Reuse connectivity indicator, alert, failure banner, session row, detail
  header, and timeline primitives. Add no dedicated dashboard, route, tab,
  panel, or component-library variant.
- Do not add `AdapterHealthState`, heartbeat timers, polling, last-report-age,
  metrics, trace storage, or retry queues. “No recent reported issues” is a
  bounded evidence statement, not a health state.
- No useful test is removed. Extend the upstream redaction/query/e2e oracles
  rather than creating test-per-code formatting matrices.

## Testing

- **Contract/boundary tests** protect exact generated shape, enum/code/count/
  target validation, authenticated attribution, canonical `validation_failed`,
  and advisory-only capabilities. These guard the cross-language trust boundary.
- **Atomic storage/projection tests** protect source+audit all-or-nothing commit,
  source correlation, replay/live equivalence, bounded newest-first query, and
  restart state honesty. They extend upstream core-diagnostics tests rather than
  duplicate storage mechanics.
- **Adapter queue tests** use injected clock/delay/report function to prove rate,
  capacity, coalescing, timeout, non-recursion, and non-throwing flush/close
  without real sleeps. Representative instrumentation tests protect mappings;
  no brittle test asserts every local JSONL event is forwarded.
- **Web trust-boundary tests** protect compound issuer replacement, stamped
  validity, CSRF on lifecycle reads, binary framing, session invalidation, and
  generated response forwarding.
- **Cockpit model/property tests** protect LSN merge/dedup, adapter-vs-session
  state separation, explicit enum mapping, stale-never-live, and safe labels.
  DOM tests protect existing-row/detail/timeline composition and accessibility.
- **One real-process extension** protects automatic Pi lifecycle report →
  authenticated core append/audit → adapter-status query → restart durability.
  Package-level cockpit tests protect final rendering; a new browser runner is
  not justified solely for this feature.
- **Negative redaction evidence** seeds every canonical SECURITY forbidden value
  plus arbitrary error message/stack/cause and scans source bytes, audit query,
  adapter status, stderr, and rendered text. Mutation must show the oracle fails
  if arbitrary text or non-atomic append is introduced.

## Risks

- **Riskiest assumption — a best-effort queue provides enough live evidence.**
  Core outage, initial attach failure, queue overflow, or process death can lose
  forwarded reports. Blocking/retrying would endanger the control loop and can
  recursively amplify failure. Fallback: the sibling durable local JSONL log
  remains the complete local inspection path; dogfooding may later scope a
  bounded durable shipper, but this feature does not.
- **Cross-feature write overlap with the adapter log sink.** Both designs touch
  `adapter_diagnostics.ts`, `core_client.ts`, and `main.ts`. Implementers must
  sequence write ownership and reuse the sibling port/registry; landing a
  second logging interface would violate the design even though the features
  lack an implementation dependency edge.
- **Atomic source/audit plumbing is security-sensitive.** A legacy plain append
  could make a live Observation visible without its durable audit explanation.
  Mitigation: one `ingest_adapter_diagnostic` entry point, use only
  `append_audited`, and mutation/fault-injection evidence.
- **Query/live races can regress presentation honesty.** A delayed status query
  could overwrite a newer streamed error or historical `ATTACHED` could look
  current after reconnect. Mitigation: source-EventId dedup, `as_of_lsn` merge,
  clear/requery on reconciliation, and adapter state never derived from reports.
  Fallback: show adapter connection unknown while preserving recent evidence.
- **Failure vocabulary does not describe every local logging fault.** Forcing an
  inaccurate canonical code would be worse than omitting the report. Only
  events with a defensible mapping forward; the broader local JSONL registry
  remains richer. New mappings are adapter-registry updates, not cockpit text.
- **Audit/read growth remains unbounded in durable history.** The status
  projection retains only 100 recent records per adapter in memory and returns
  at most 100, but the authority log retains all records under upstream policy.
  Retention/SIEM/compaction remain reserved and are not implied here.
- **Least certain:** exact integration timing with the upstream
  core-diagnostics implementation, whose body defines the contracts but whose
  code has not landed. The fallback is to land Units 1-3 immediately after its
  query checkpoint and preserve its field numbers/signatures; do not fork a
  temporary diagnostics service or handwritten DTO.

## Extension pressure classification

- **Committed post-v0.1.0:** adapter-declared diagnostic reporting capability;
  typed adapter diagnostic payloads; authenticated source Observation + atomic
  audit recording; bounded `AdapterStatusPage` recent records; and cockpit
  composition within the existing session views. These promote the reserved
  adapter-specific diagnostic-code seam recorded by the parent epic.
- **Remains reserved:** heartbeat/last-report-age liveness policy and any
  adapter-declared liveness capability; dedicated health/status dashboard;
  delivery-trace timeline UI; metrics; raw event inspection; no-lifecycle
  reads; durable shipping/retry; SIEM/long retention; notification/mobile/
  desktop-specific presentation.
- **Remains rejected:** a second diagnostics writer/table/channel as authority,
  a dedicated per-command trace store, metrics as the primary observability
  substrate, diagnostic silence as proof of health, and Pi-specific diagnostic
  codes as core protocol enums.
- Adapter-specific codes are declared by the adapter and surface composition is
  cockpit-owned, preserving adapter- and surface-neutrality. Authority-domain
  ids and generated targets remain in every durable/query identity; none of the
  parked multi-human, desktop, mesh, or skin directions is foreclosed.

## Implementation summary

Implemented the feature across the three child checkpoints:

- **Contract ingestion (`f392c17`)** — generated adapter capability/report
  contracts; fail-closed authenticated report validation; safe source
  Observation plus `ADAPTER_DIAGNOSTIC_REPORTED` audit record via one atomic
  `append_audited`; and replayable newest-first recent diagnostics bounded by
  the query/projection limits. Adapter identity, endpoint, domain, and
  generation come from the current authenticated attachment.
- **Adapter forwarding (`d19489a`)** — reused the landed
  `AdapterDiagnostics` port and JSONL event registry. The single
  `PI_FORWARDED_DIAGNOSTIC_CODES` mapping drives both the Pi manifest and the
  best-effort forwarder. The queue is sequential, rate/capacity bounded,
  coalescing, timeout-limited, non-retrying, and sink-failure isolated. It
  never sends messages, stacks, prompts, transcript/tool content, or arbitrary
  metadata and never invokes attachment refresh.
- **Cockpit composition (`0a8782b`)** — added authenticated binary
  gRPC-Web `QueryDiagnostics` with shared Submit stamping/CSRF, generated query
  operations with recent limit 20, source-EventId/LSN-safe model merging, and
  composition into existing session rows/detail/timeline primitives. Adapter
  status is explicitly separate from session liveness and maps exhaustively to
  existing connectivity presentation bindings; no new screen, route, dashboard,
  protocol state, or CSS state variant was introduced.
- **Foundation roll-forward** — updated the protocol extension registry,
  security allowlist/redaction statement, Pi manifest declaration, and generated
  UX presentation traceability. Mockups remain intentionally skipped because
  the feature reuses existing cockpit views.

### Sink-vocabulary reconciliation

The landed sibling sink's `AdapterDiagnosticEvent` registry remains the one
instrumentation vocabulary. Forwarding promotes only the mapped operational
subset and maps it to adapter-owned bounded wire codes. In particular,
`delivery.failed` maps to `pi_delivery_failed`, `delivery.rejected` to
`pi_delivery_rejected`, subscription retry/unavailability map to
`pi_delivery_subscription_retrying`/`pi_delivery_subscription_failed`, and
local observation failures map to `pi_observation_failed` or
`pi_observation_flush_failed`. No parallel event channel or core enum was
created; unmapped local events remain JSONL-only by design.

### Verification and deviations

- Passed `cargo build --workspace --all-targets`, `cargo test --workspace`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed contracts TypeScript build, vectors/models/presentation checks,
  web-server tests (25), web-cockpit tests (50), and the walking-skeleton e2e.
  The isolated Pi real-process e2e passes; the existing parallel `pi-adapter`
  `npm test` has an intermittent cancellation in that e2e, recorded in the
  adapter story rather than hidden by test changes.
- `contracts/proto` `buf lint` still reports the repository's existing RPC
  request/response naming violations (including the exact designed diagnostic
  report names); no unrelated contract renaming was performed. The known
  pre-existing TypeScript generated-drift failure remains untouched.
- The landed server has no injectable clock port, so report ingestion uses the
  existing core `now_timestamp()` boundary for the audit timestamp; adapter
  `observed_at` remains separate evidence. This is an implementation seam
  limitation, not a liveness policy.
- No tests were deleted, weakened, skipped, or made conditional. No work was
  parked.

## Review findings (standard pass 1, 2026-07-26 — independent reviewer: gpt-5.6-sol)

Verdict: blockers-found. Receiver-confirmed blockers (fix before `done`):

1. **Adapter-status projection ignores fresh detach/failure** —
   `core/src/diagnostics/mod.rs`: `AdapterDetached`/`AdapterFailed` audits are
   stored but never clear `live_adapters`; ATTACHED wins indefinitely; prefix
   materialization resets liveness globally so historical pre-restart
   lifecycle collapses to UNKNOWN. Fix: per-adapter current-process
   attachment freshness; fresh detach/failure projects DETACHED/FAILED;
   pre-restart history stays UNKNOWN; attach→detach/failure/restart tests.
2. **Cockpit treats stale/failed diagnostic queries as authoritative** —
   probed: rejected responses silently retain prior status; a lower as_of_lsn
   response overwrote FAILED@20 with ATTACHED@10. Fix: require accepted +
   completed + adapter-result else clear to unavailable/unknown; monotonic
   asOfLsn merge retaining newer live diagnostics.
3. **Reconciliation clears status without a reliable refresh signal** — shell
   only renders final reconcile events, so the false→true transition is
   invisible; filtered audit-LSN gaps trigger snapshot replacement that can
   erase a query's own just-returned status. Fix: explicit
   reconciliation-complete signal to the diagnostics controller; distinguish
   filtered audit gaps from stream loss; integration test for
   query-lifecycle → filtered-gap → reconcile.
4. **Forwarder 1s timeout doesn't cancel network work** — probed: two
   never-resolving reports → two simultaneously active RPCs (Promise.race
   only abandons the local await). Fix: AbortController per report, signal
   through reportDiagnostic to Connect, abort on timeout/close; test active
   RPC count returns to zero and never exceeds one.
5. **Missing cross-layer acceptance evidence** — no e2e real-process
   pi_adapter_started → query → restart durability path; no redaction vector;
   forwarder tests lack capacity/rate/timeout-cancellation/rejection/close;
   projection tests lack recent-diagnostics and detach/failure coverage.
   Fix: add the specified evidence.

Temporal-integration finding (recorded): ingestion composes consistently with
the final typed-audit-decision substrate (typed AuditRecordDraft →
append_audited, no envelope inference); the four inconsistency points above
are the blockers.

## Review resolution

1. **Fresh adapter lifecycle projection** — `core/src/diagnostics/mod.rs` now tracks
   per-adapter current-process evidence separately from replayed lifecycle history.
   Fresh attach/detach/failure records project ATTACHED/DETACHED/FAILED and clear
   the prior live state; reset/rebuild clears that evidence so historical records
   remain UNKNOWN. `core/tests/diagnostics_projection.rs` covers attach→detach,
   attach→failure, restart honesty, and bounded recent diagnostics.
2. **Cockpit query authority and LSN monotonicity** —
   `web-cockpit/src/domain/adapter-diagnostics.ts` accepts status only for an
   accepted+completed query with an adapter result and an `as_of_lsn`; rejected,
   failed, incomplete, missing, or non-adapter results clear cached status while
   retaining diagnostics. Equal/newer responses replace status, older responses
   retain newer status and live evidence. `web-cockpit/tests/adapter-diagnostics.test.ts`
   covers rejected clearing and FAILED@20 versus ATTACHED@10.
3. **Reconciliation signal and filtered gaps** —
   `web-cockpit/src/domain/reconcile.ts` no longer treats successful LSN holes as
   stream loss because the operator stream filters authority/audit records; only
   transport/fold failures snapshot-reconcile. It emits an explicit completed
   stream-reconnect callback, consumed by `web-cockpit/src/main.ts`; shell render
   transitions in `web-cockpit/src/ui/shell.ts` no longer drive diagnostic refresh.
   `web-cockpit/tests/reconcile.test.ts` covers the filtered query-lifecycle hole,
   status preservation, and the explicit reconnect signal.
4. **Forwarder cancellation** — `pi-adapter/src/core_diagnostics_forwarder.ts`
   creates one AbortController per RPC, aborts on timeout and close, and awaits
   cancellation before the sequential drain starts another report.
   `pi-adapter/src/core_client.ts` passes the signal through Connect RPC and
   `pi-adapter/src/main.ts` supplies it. Forwarder tests cover active-call
   capacity, timeout cancellation, close cancellation, rate/capacity bounds,
   rejection isolation, and sink isolation.
5. **Cross-layer evidence** — `pi-adapter/tests/e2e.test.ts` now exercises the
   real forwarded `pi_adapter_started` report, QueryDiagnostics, core restart,
   durable recent-record recovery, UNKNOWN-before-reattach, and ATTACHED-after-
   reattach. `contracts/vectors/audit-redaction-boundary.json` and generated
   `docs/VERIFICATION.md` trace the adapter diagnostic allowlist and sentinel
   exclusions. Projection and forwarder regression suites provide the remaining
   recent-diagnostic, detach/failure, capacity, timeout, rejection, and close
   evidence. No proto changes were needed.

Verification after the blocker fixes: `cargo build --workspace --all-targets`,
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
web-cockpit tests (53), web-server tests (25), contracts build/vector/model checks,
and the isolated real-process Pi e2e pass. The full parallel `pi-adapter npm test`
continues to exhibit the repository's known cancellation timing flake in its
real-process e2e; the focused real-process test passes, and the permitted identical
retry was recorded without changing or weakening test code.
