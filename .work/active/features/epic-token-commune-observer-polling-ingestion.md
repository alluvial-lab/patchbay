---
id: epic-token-commune-observer-polling-ingestion
kind: feature
stage: done
tags: [adapter, protocol]
parent: epic-token-commune-observer
depends_on: [epic-token-commune-observer-adapter-foundation, epic-token-commune-observer-snapshot-mapping]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-07
---

# token-commune polling ingestion and observations

## Brief

The **runtime capability**: the long-running poller that drives the snapshot
projection and emits reports/Observations to the core with honest gap and
staleness behavior. This feature owns the schedule, the streaming/gap logic, and
the Observation mapping — the projection function itself lives in
`snapshot-mapping`.

It delivers: the polling schedule over the gateway read endpoints (no upstream
stream/webhook/cursor exists); resource-report emission via
`IngestObservation.resource_report`; `PoolEvent` → generic `Observation` mapping
(source-authenticated status emissions with operational-resource target scope
and adapter-owned schema refs); deduplication; explicit gap behavior (the
latest-50-event window, missed polls, and reconnect reconciliation — the adapter
must never claim a stream or unlimited-history repair); source-timestamp
propagation (each reading's `observedAt`, since capacity polling is itself
gated/backoff-delayed upstream); and stale-state degradation on disconnect
(reusing the core's adapter-loss inference from the `ReceiveDeliveries` stream
drop).

It does NOT cover the projection logic (`snapshot-mapping`) or the cockpit.

## Epic context

- Parent epic: `epic-token-commune-observer`
- Position in epic: **runtime/streaming** — consumes the projection from
  `snapshot-mapping` and the attach lifecycle from `adapter-foundation`;
  produces the live resource state + Observations that `cockpit-panel` renders.

## Simplification opportunity

- Reuse the Pi adapter's report-ordering/backpressure and delivery-stream
  reconnect/reattach machinery; the poller only adds "when to fetch and what
  gap/staleness to report."
- Do not build a synthetic event stream. Polling is the honest delivery model;
  claiming otherwise is explicitly rejected.

## Foundation references

- `docs/PROTOCOL.md` — snapshots repair missed streams/gaps; partial/none tiers
  degrade as defined; a resource adapter may claim only the tier its complete
  external view can reconstruct.
- `docs/ARCHITECTURE.md` — adapter loss degrades owned resources honestly rather
  than fabricating liveness.
- `contracts/proto/patchbay/adapter_control.proto` — `IngestObservation`
  (`resource_report` + generic `event` arms), `ReceiveDeliveries`.
- `contracts/proto/patchbay/observations.proto` — `Observation`, `ObservationKind`.
- External contract: token-commune `/commune/events` (latest 50, no cursor;
  in-memory fallback ring of 200); declared vs actually-emitted event kinds
  (`window_exhausted`, `calibration` are declared but have no production emitter).

## Key design decisions (inherited)

- **Polling-only, honestly.** No upstream stream exists. The poller reports
  `observedAt` per reading and degrades to stale when the core connection or
  upstream poll is lost. It must not present polling cadence as streaming.
- **Gap behavior is bounded by reality.** A reconnecting observer cannot
  reconstruct more than the latest 50 events or history before initial install;
  the adapter reports the gap honestly rather than fabricating continuity.
  Cursor/replay is an external prerequisite.
- **Partial event coverage.** Only a subset of declared event kinds is emitted
  upstream today; the Observation mapping covers what is actually emitted and
  does not assert lifecycle coverage the upstream does not provide.

## Design decisions

- **Cycle shape:** run an immediate first poll, then non-overlapping cycles whose
  next delay starts after the prior cycle completes. Fetch the five snapshot
  endpoints plus `/commune/events` concurrently; represent each failed snapshot
  endpoint as `unavailable` and never reuse an adapter-side cached response.
  This preserves the pure projector's source-state contract and avoids making a
  slow endpoint serialize the whole refresh.
- **Cadence and upstream backoff:** use the configured
  `PATCHBAY_TOKEN_COMMUNE_POLL_INTERVAL_MS` as a minimum delay and extend it to
  the latest valid `Retry-After` signal returned by any endpoint. Do not shorten
  upstream backoff, overlap cycles, add a push/stream label, or invent a polling
  lag SLA. Invalid backoff headers are ignored with a bounded diagnostic.
- **Report time versus reading time:** sample an injected clock after endpoint
  collection for `ResourceReport.observed_at` (the report refresh time). Preserve
  every upstream capacity reading's existing `observedAt` string byte-for-byte;
  the poll timestamp never overwrites or defaults a reading timestamp. Pool
  event Observations use the event's `occurredAt` as `Observation.observed_at`.
- **Fresh-install event baseline:** the first successful latest-50 read becomes
  a baseline and emits a gap/baseline status, but its pre-install `PoolEvent`s
  are not replayed as newly observed events. This avoids duplicate historical
  facts after adapter restart and states the unobservable pre-install boundary
  honestly. A process restart is a new install boundary, not durable replay.
- **Within-window reconciliation:** stable upstream event ids are the only dedup
  key. If consecutive successful pages overlap, emit only newly visible ids. If
  the prior page was empty, a new page shorter than 50 is continuous; a full
  50-event page has an unmeasurable overflow risk and reports a gap. Any other
  no-overlap transition reports a discontinuity before emitting the currently
  visible facts. Never estimate a missed count or claim continuity before the
  oldest visible event.
- **Dedup durability:** dedup state is bounded, in-memory polling state, updated
  only after core acknowledgement. It covers repeat polls and same-process core
  reconnects; it does not claim exactly-once delivery across process restart.
  Cursor/replay or a core-visible upstream event-id index is the external
  prerequisite for that stronger guarantee.
- **Event mapping:** one adapter-owned disposition registry owns every accepted
  upstream kind. It marks `capacity_shift`, `auth_broken`, `windfall`,
  `fingerprint`, and `member` as production-emitted and maps them to
  `ObservationKind.STATUS`; it marks `window_exhausted` and `calibration` as
  declared-only. The gateway decoder and mapper derive from that registry, so
  the declared-only values remain decodable but produce one bounded diagnostic
  per visible id, not a coverage claim or Observation, until an upstream emitter
  and contract promotion exist. Their ids are consumed in the in-memory window
  after the diagnostic so every poll does not repeat it; this is explicitly not
  an Observation acknowledgement.
- **Observation target:** every mapped pool event targets the exact synthesized
  `token-commune.provider-pool` `ResourceIdentity` for its provider. Gap status
  is emitted once per currently known affected provider-pool target (from the
  current report/event page); if no resource target exists, the adapter records
  the source-wide boundary diagnostically rather than fabricating a resource.
- **Staleness ownership:** do not add an adapter-authored `stale` event or
  heartbeat. The core already degrades this adapter's resource records when the
  long-lived `ReceiveDeliveries` stream drops. Poll ingress failure cannot
  fabricate current state; after same-process reconnect, the next accepted
  PARTIAL report restores only listed resources and the retained event window
  reconciles only where the latest-50 overlap permits it.
- **Failure boundary:** expected gateway failures produce explicit unavailable
  source slices and diagnostics; an all-snapshot-endpoint failure still emits an
  empty PARTIAL report when the core is reachable, so cached resources degrade
  through core omission semantics. Retryable core transport failure preserves
  tracker state and retries on a later cycle. Projection/schema/internal
  invariant errors are fatal and tear down the supervised adapter process so the
  core cannot mistake a broken producer for healthy evidence.
- **Dispatch rationale:** direct-read only. The package is bounded and the
  gateway port, projector, generated ingress contract, core stale behavior, and
  existing tests expose the complete design surface. This delegated harness has
  no independent subagent/peer tool, so the thorough design advisory path is
  recorded as degraded and the design proceeds non-blockingly under Part IV.

## Architectural choice

Three approaches were considered:

1. **One polling runtime over pure projection plus a pure event-window tracker
   (chosen).** A scheduler owns time/network orchestration, feeds explicit
   endpoint states to `projectTokenCommuneSnapshot`, sends the report through a
   narrow core sink, and delegates latest-50 reconciliation to a deterministic
   tracker. This keeps HTTP, clock, and RPC outside mapping logic while making
   every honesty rule directly testable. It costs two small runtime modules.
2. **Fold polling into `AdapterProcess` and retain ad-hoc sets there.** This has
   fewer files, but mixes delivery liveness, HTTP collection, report projection,
   retry timing, and gap classification in one loop. Time/network tests would
   depend on private process control and gap mutations would be hard to isolate.
3. **Add an adapter-side durable event cache/cursor.** Persistence could suppress
   restart duplicates, but no upstream cursor proves that a stored watermark is
   continuous, and it would create a second state store beside the core. It
   cannot repair history older than 50 and would make a stronger claim than the
   source contract supports.

The first option is the least irreversible sound choice. Polling remains an
adapter implementation detail, resource reconciliation remains in the core,
and a future upstream cursor can replace the event-window tracker without
changing the emitted report/Observation seam.

The trickiest unit is the latest-50 tracker. It must distinguish a fresh install,
an overlapping reconnect, an empty-to-short-page continuous transition, and an
unanchored/full-window discontinuity while acknowledging outputs transactionally.
The rest of the runtime depends on that classifier and is designed around it.

## Honesty invariants

1. **Polling, never streaming:** the manifest remains
   `streaming_support = false`; payloads identify `deliveryModel: "polling"` and
   `historyMode: "latest-50-no-cursor"`; no code or diagnostic calls gateway
   polling a stream.
2. **Bounded event knowledge:** a first page establishes a non-replayed baseline.
   Later overlap supports only within-window reconciliation. No overlap or a
   saturated unanchored page emits an explicit gap; no missed-event count,
   unlimited replay, or authoritative reconstruction is asserted.
3. **Acknowledged dedup:** an upstream event id becomes emitted only after the
   core accepts its Observation. Failed emission remains retryable; repeated
   successful polls do not duplicate it. The bounded tracker makes no
   process-restart exactly-once claim.
4. **Source time survives:** report refresh time and capacity/event source time
   remain separate. Capacity readings retain upstream `observedAt`; event
   Observations carry upstream `occurredAt`; gap status carries detection time.
5. **Partial stays partial:** every projected report remains snapshot mode with
   both manifest views at `PARTIAL`; endpoint failure becomes explicit
   `unavailable` or PARTIAL omission, never cached-source substitution,
   authoritative completeness, a tombstone, or fabricated zero telemetry.
6. **Source-authenticated resource scope:** generic Observations are submitted
   only through the current attachment, identify the adapter as sender, and use
   exact `TargetScopeKind.RESOURCE` provider-pool identity. Payload identity or
   display text never controls routing.
7. **Actual event coverage only:** the emitted-kind registry is exactly the five
   production kinds. Declared-only `window_exhausted` and `calibration` are not
   silently mapped, counted as covered, or promoted into a core enum.
8. **Disconnect never looks live:** stream loss, not poll cadence or silence, is
   the adapter-liveness signal. The adapter emits no liveness heartbeat and no
   stale-to-current assertion without a newly accepted resource report.

## Stable runtime interfaces

### Poll scheduler and source collection

**File:** `token-commune-adapter/src/poller.ts`

**Story:** `epic-token-commune-observer-polling-ingestion-poll-runtime`

```ts
import type { Timestamp } from "@bufbuild/protobuf/wkt";
import type { EventId, Observation, ResourceReport } from "@patchbay/contracts";
import type { AdapterDiagnostics } from "./adapter_diagnostics.js";
import type { TokenCommuneGatewayClient } from "./gateway_client.js";
import type { ResourceIdentitySynthesizer } from "./identity.js";

export interface PollClock {
  now(): Date;
}

export interface PollWaiter {
  wait(milliseconds: number, signal: AbortSignal): Promise<void>;
}

export interface PollerCoreSink {
  ingestResourceReport(report: ResourceReport): Promise<EventId | undefined>;
  ingestEvent(observation: Observation): Promise<EventId | undefined>;
}

export interface TokenCommunePollerOptions {
  adapterId: string;
  adapterGeneration: number;
  authorityDomainId: string;
  pollIntervalMs: number;
  gateway: TokenCommuneGatewayClient;
  core: PollerCoreSink;
  identities: ResourceIdentitySynthesizer;
  diagnostics?: AdapterDiagnostics;
  clock?: PollClock;
  waiter?: PollWaiter;
}

export class TokenCommunePoller {
  constructor(options: TokenCommunePollerOptions);
  run(signal: AbortSignal): Promise<void>;
  pollOnce(signal: AbortSignal): Promise<{ nextDelayMs: number }>;
}
```

`pollOnce` concurrently settles status, pool, me, fingerprints, models, and
recent events. Aborted calls stop the cycle; other gateway failures become
bounded endpoint outcomes. The five projection inputs never read a previous
cycle. The completion clock is converted to one validated Protobuf timestamp.
`run` starts immediately, waits only after the completed attempt, and never has
more than one cycle active.

**Acceptance criteria:**

- [ ] A fake clock/waiter proves immediate first execution, no overlap, and the
      configured completion-to-next-start minimum cadence.
- [ ] A valid upstream `Retry-After` extends (never shortens) the next delay;
      invalid advice cannot cause a hot loop or leak header/body data.
- [ ] All six endpoints receive the same cycle abort signal and settle
      independently; one snapshot failure cannot reuse its preceding value.
- [ ] Unexpected projection/internal errors terminate the poller, while expected
      gateway and retryable core transport failures retain safe retry state.

### Report construction and ingress

**Files:**

- `token-commune-adapter/src/poller.ts`
- `token-commune-adapter/src/core_client.ts`
- `token-commune-adapter/src/gateway_client.ts`

**Story:** `epic-token-commune-observer-polling-ingestion-report-emission`

```ts
export interface GatewayBackoffSignal {
  readonly retryAfterMs?: number;
  readonly retryAt?: string;
}

export class GatewayClientError extends Error {
  readonly kind: GatewayErrorKind;
  readonly endpoint: GatewayEndpoint;
  readonly status?: number;
  readonly backoff?: GatewayBackoffSignal;
}

export class PatchbayCoreClient implements PollerCoreSink {
  ingestResourceReport(report: ResourceReport): Promise<EventId | undefined>;
  ingestEvent(observation: Observation): Promise<EventId | undefined>;
}
```

The HTTP adapter parses only valid delta-seconds or HTTP-date `Retry-After` on
retryable/rate-limited responses and exposes normalized safe advice. The poller
maps settled snapshot calls to the projectors' `reported(value) | unavailable`
input, samples report refresh time after settlement, calls
`projectTokenCommuneSnapshot`, then sends `ObservationRequest.resource_report`
through the existing attachment/one-reauth wrapper. Events are processed only
after the report succeeds, so their exact resource targets are admitted first.

**Acceptance criteria:**

- [ ] Every successful cycle emits exactly the pure projector's report, unchanged
      except for its injected completion timestamp.
- [ ] Partial endpoint failure remains schema-valid and PARTIAL; all five
      snapshot failures emit two empty PARTIAL views, staling cached resources
      by core-owned omission behavior.
- [ ] Every capacity reading `observedAt` equals the gateway value after report
      ingress; report `observed_at` equals the fake completion clock and cannot
      replace reading time.
- [ ] Core ingress uses the authenticated current attachment and returns the
      core event id; caller identity fields cannot override adapter/domain.

### Pool-event Observation contract and mapping

**Files:**

- `token-commune-adapter/src/event_observation.ts`
- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/schemas/pool-event-observation.schema.json`
- `token-commune-adapter/schemas/event-gap-observation.schema.json`

**Story:** `epic-token-commune-observer-polling-ingestion-event-observation-map`

```ts
export const TOKEN_COMMUNE_EVENT_KINDS = {
  capacity_shift: "production-emitted",
  auth_broken: "production-emitted",
  windfall: "production-emitted",
  fingerprint: "production-emitted",
  member: "production-emitted",
  window_exhausted: "declared-only",
  calibration: "declared-only",
} as const;
export type GatewayEventKind = keyof typeof TOKEN_COMMUNE_EVENT_KINDS;
export type TokenCommuneEmittedEventKind = {
  [K in GatewayEventKind]:
    (typeof TOKEN_COMMUNE_EVENT_KINDS)[K] extends "production-emitted" ? K : never
}[GatewayEventKind];

export const TOKEN_COMMUNE_OBSERVATION_SCHEMAS = {
  poolEvent: "patchbay.token_commune.pool_event.v1",
  eventGap: "patchbay.token_commune.event_gap.v1",
} as const;

export type PoolEventMapResult =
  | { readonly status: "mapped"; readonly observation: Observation }
  | { readonly status: "declared-but-unemitted";
      readonly kind: "window_exhausted" | "calibration" };

export function mapPoolEvent(input: {
  authorityDomainId: string;
  adapterId: string;
  identities: ResourceIdentitySynthesizer;
  event: GatewayEvent;
}): PoolEventMapResult;

export function mapEventGap(input: {
  authorityDomainId: string;
  adapterId: string;
  targets: readonly SynthesizedResourceIdentity[];
  detectedAt: Timestamp;
  gap: EventGapEvidence;
}): readonly Observation[];
```

Each pool-event payload is closed JSON containing source event id, exact emitted
kind, provider, nullable contribution id, bounded message, source occurrence
time, and the two honesty labels. `Observation.kind = STATUS`,
`failure_code = UNSPECIFIED`, sender is the adapter actor, target is the exact
provider-pool resource, and `observed_at` is parsed from `occurredAt`. The gap
payload contains reason, visible/prior window sizes, overlap count, detection
time, and `continuity: "unknown-before-visible-window"`; it never contains an
estimated missed count.

**Acceptance criteria:**

- [ ] Literal independent fixtures prove all five production kinds map to STATUS
      and exact provider-pool resource targets with the two registered schema
      refs and JSON content type.
- [ ] `window_exhausted`, `calibration`, unknown kinds, malformed timestamps,
      duplicate page ids, and schema-invalid payloads fail closed or return the
      declared-only diagnostic result before core ingress.
- [ ] Event `occurredAt` becomes `Observation.observed_at`; poll/report time is
      absent from the event payload except for separate gap detection time.
- [ ] Payloads contain no gateway credential, attachment material, prompt/model
      traffic, raw fingerprint capture, or arbitrary extension metadata.

### Latest-50 deduplication and gap classification

**File:** `token-commune-adapter/src/event_window.ts`

**Story:** `epic-token-commune-observer-polling-ingestion-dedup-gap`

```ts
export type EventGapReason =
  | "initial-baseline"
  | "window-discontinuity"
  | "window-saturated-without-anchor"
  | "history-became-empty";

export interface EventGapEvidence {
  readonly key: string;
  readonly reason: EventGapReason;
  readonly previousWindowSize: number;
  readonly visibleWindowSize: number;
  readonly overlapCount: number;
  readonly reconstruction: "visible-window-only";
  readonly continuity: "unknown-before-visible-window";
}

export interface EventWindowPlan {
  readonly gap?: EventGapEvidence;
  readonly baselineOnly: boolean;
  readonly events: readonly GatewayEvent[];
}

export class LatestEventWindowTracker {
  plan(page: GatewayEventsPage): EventWindowPlan;
  acknowledgeGap(key: string): void;
  acknowledgeEvent(eventId: string): void;
  consumeDeclaredOnly(eventId: string): void;
  commitWindow(page: GatewayEventsPage): void;
}
```

The tracker stores only the most recent successful page ids, acknowledged ids
still relevant to that page, consumed declared-only ids, and the last
acknowledged gap key. `plan` is pure against current state. The fresh-install
plan is `baselineOnly` and contains no events. Normal plans order new events
deterministically by `(occurredAt, id)` for delivery while retaining source
timestamps. The poller acknowledges each gap/mapped event only after successful
core ingress; a declared-only id uses the separately named consume path after
its bounded diagnostic. `commitWindow` runs only after all planned outputs
finish. If an RPC fails midway, a retry suppresses already accepted output but
reattempts the remainder.

**Acceptance criteria:**

- [ ] Repeated/overlapping pages emit each acknowledged source id once and retain
      newly failed ids for retry.
- [ ] Initial baseline emits no historical PoolEvent Observation and one
      baseline gap per current target.
- [ ] Overlap reconciles newly visible events without a new gap; empty→short is
      continuous; empty→50 and every other no-overlap transition emit a gap.
- [ ] Gap acknowledgement itself is deduplicated across partial RPC failure.
- [ ] State remains bounded by latest-window evidence and explicitly resets at
      process start; tests make no restart exactly-once assertion.

### Process supervision, disconnect, and reconnect

**File:** `token-commune-adapter/src/main.ts`

**Story:** `epic-token-commune-observer-polling-ingestion-disconnect-reconnect`

```ts
export interface AdapterProcessOptions extends TokenCommuneAdapterConfig {
  gateway: TokenCommuneGatewayClient;
  diagnostics?: AdapterDiagnostics;
  forwardDiagnostics?: boolean;
  coreClient?: PatchbayCoreClient;
  poller?: TokenCommunePoller;
  retryDelayMs?: number;
}
```

After one successful attach, `AdapterProcess.run` supervises the existing
long-lived delivery loop and the poller under one child abort controller. An
external abort stops both. An unexpected fatal exit from either child aborts the
other and rejects the process; ordinary retryable delivery/core failures retain
the established loop behavior. No poller status substitutes for the stream as
adapter-liveness evidence.

On core disconnect the server observes `ReceiveDeliveries` loss and degrades
owned resources. The poller cannot mark them live while ingress is unavailable.
After reattachment, the next accepted PARTIAL report restores only reported
resources; the in-memory latest-50 state remains available for overlap-based
event reconciliation. If the current page no longer overlaps, the gap status
precedes currently visible events.

**Acceptance criteria:**

- [ ] Fake stream/core tests prove one attach, one delivery loop, one nonoverlap
      poller, coordinated abort, and no orphan timer/RPC/diagnostic handle.
- [ ] A simulated core outage accepts no report/Observation and does not advance
      event acknowledgement; recovery emits a fresh PARTIAL report before event
      reconciliation.
- [ ] Same-process reconnect with overlap emits only missed ids; reconnect after
      window rollover emits gap then visible facts; process restart follows the
      initial-baseline rule.
- [ ] The adapter emits no heartbeat, session connectivity state, resource
      freshness enum, fabricated stale mutation, or polling-as-streaming signal.
- [ ] Existing core `ResourceStaleNeverLive` stream-drop evidence remains the
      stale-state authority; the downstream conformance feature binds this
      adapter to that real-core behavior rather than duplicating a second stale
      state machine here.

### Mutation-sensitive honesty evidence

**Files:**

- `token-commune-adapter/tests/poller.test.ts`
- `token-commune-adapter/tests/event_observation.test.ts`
- `token-commune-adapter/tests/event_window.test.ts`
- `token-commune-adapter/tests/main.test.ts`
- `token-commune-adapter/tests/gateway_client.test.ts`

**Story:** `epic-token-commune-observer-polling-ingestion-honesty-mutation-evidence`

Tests use fake gateway/core/clock/waiter ports and no wall-clock sleeps or real
network. Independent expected fixtures do not derive the emitted-kind list,
schema refs, gap reasons, target identities, or timestamps from production
registries. The feature records executed/reverted mutants in its implementation
summary.

**Acceptance criteria:**

- [ ] Mutants that overlap cycles, ignore `Retry-After`, cache a failed endpoint,
      or promote PARTIAL fail stable tests.
- [ ] Mutants that replay the initial 50, suppress a no-overlap gap, dedup before
      acknowledgement, clear tracker state on same-process reconnect, or claim
      a missed count fail stable tests.
- [ ] Mutants that use poll time for event/readings, map declared-only kinds,
      target adapter/session scope, emit EVENT instead of STATUS, or omit the
      polling/no-cursor labels fail stable tests.
- [ ] A disconnect mutant that emits fabricated liveness/current/stale evidence
      or advances acknowledgement while ingress fails is rejected.
- [ ] Strict build, full package tests, and `git diff --check` pass; real-core
      promoted conformance remains in `epic-token-commune-observer-conformance`.

## Implementation order

1. `poll-runtime`
2. `report-emission` depends on `poll-runtime`
3. `event-observation-map` depends on `report-emission`
4. `dedup-gap` depends on `event-observation-map`
5. `disconnect-reconnect` depends on `dedup-gap`
6. `honesty-mutation-evidence` depends on `disconnect-reconnect`

These stories are durable design/verification checkpoints for one cohesive
feature owner, not six independent worker assignments.

## Simplification

- Reuse the gateway port, pure projector, identity synthesizer, generated
  `ObservationRequest` contract, attachment/reauth wrapper, diagnostics, and
  core resource omission/degradation machinery.
- Keep one scheduler and one latest-window tracker. Do not add an event-stream
  abstraction, adapter-side database, replay cursor, state store, heartbeat,
  resource freshness enum, new RPC, ResourceKind, or core protocol state.
- Keep event and gap schema refs in one adapter registry and emitted upstream
  kinds in one literal registry; tests use independent fixtures rather than a
  second production list.
- No existing test is obsolete. Process tests expand around the existing
  delivery loop instead of replacing its unsupported-Operation evidence.

## Testing

- **Scheduler/interface tests:** fake time, waiter, gateway, and core sink protect
  non-overlap, cadence/backoff, endpoint availability, report-before-event
  ordering, and retry state without sleeping.
- **Event mapper tests:** independent JSON/protobuf fixtures protect source
  authentication, resource targeting, emitted-kind coverage, schema refs, and
  source timestamps.
- **Tracker regression tests:** short traces protect fresh baseline, overlap,
  empty/full boundaries, rollover/reset gaps, acknowledgement-aware retry, and
  bounded state. These test the highest-risk logic directly.
- **Process tests:** mocked delivery-stream loss and ingress recovery protect the
  composition contract and prove the adapter does not implement a competing
  stale/liveness state machine.
- **Mutation evidence:** every honesty invariant above has at least one named
  production mutant whose focused test fails before restoration. Do not add
  tautological getter, generated-message, every-HTTP-status, or wall-time tests.

## Risks

- **Riskiest assumption — overlap is sufficient within the latest-50 contract.**
  This relies on stable ids and an append-only latest window within one gateway
  DB. The fallback is conservative: any absent anchor or saturated unanchored
  page emits a gap and never estimates continuity. Cursor/replay remains the
  only route to stronger reconstruction.
- **Process restart cannot dedup prior core history:** there is no core query by
  upstream event id and no durable upstream cursor. The fresh baseline suppresses
  replay of the visible pre-install page and reports the boundary; it may miss
  facts that occurred before restart. That loss is preferable to fabricated
  exactly-once history and is covered by conformance language.
- **Composite resource target churn:** a gateway URL/provider normalization
  change creates a new target and can leave the old resource stale. The mapper
  reuses the existing synthesizer and never infers replacement.
- **Per-slice freshness remains subtle:** a successful composite upsert may
  contain unavailable nested source slices. The projector carries explicit
  slice state and the cockpit must show reading ages/source availability; if
  that proves misleading, the safe fallback is to omit the provider mutation
  for that poll and stale the whole wrapper.
- **Retry-After can delay visible refresh:** honoring upstream backoff makes the
  panel's report refresh old while underlying readings are older still. The UI
  must show both; the poller must not refresh timestamps without a new fetch.
- **Design advisory degradation:** effective review weight is **thorough** from
  the explicit caller. No independent reviewer mechanism is available on this
  worker surface, so design-time advisory review is degraded, not mislabeled as
  independent/cross-model. Implementation/feature/final reviews must receive
  `thorough` unchanged and adjudicate findings as proposals.

## UI surface

This feature adds no human control surface. It produces the stream consumed by
`cockpit-panel`; the parent epic's selected mock remains the UI authority. No
feature-level fallback mockup is needed.

## Other agent review

- Invoked because: polling/backoff, transactional dedup, a bounded no-cursor
  event window, source timestamps, and disconnect/stale semantics are
  correctness-sensitive integration seams.
- Effective weight: **thorough** (explicit caller).
- Skipped/degraded: this delegated worker exposes no subagent or peer review
  mechanism. Independent design review could not be commissioned; Part IV makes
  that non-blocking at design time. Direct source, contract, foundation, and
  pre-mortem evidence was used instead, and no pass is labeled cross-model.
- Fixed/active blockers: the direct pre-mortem chose fresh-install baselining to
  prevent restart replay, acknowledgement-aware tracker commits to prevent lost
  events, empty→50 saturation as an explicit gap, and core-owned stream-drop
  staleness rather than an adapter heartbeat.
- Parked: durable cross-restart dedup, source cursor/replay, heartbeat/age-based
  liveness, and authoritative event reconstruction all require external or
  protocol prerequisites.
- Rejected: a local durable event cache and synthetic stream abstraction; neither
  can make a latest-50 no-cursor endpoint authoritative.

## Extension pressure classification

- **Committed post-v0.1 direction:** configurable non-overlapping polling; exact
  PARTIAL report ingress; production-emitted pool-event STATUS Observations;
  bounded in-process id dedup; explicit initial/rollover gaps; source timestamp
  propagation; core-owned stream-drop staleness and report-based reconnect.
- **Reserved seams:** upstream push/webhook/SSE, cursor/pagination/replay,
  documented retention/lag SLA, persistent cross-restart event dedup, heartbeat
  or last-report-age liveness, full emitted lifecycle-kind coverage, and
  authoritative snapshot/event history.
- **Explicitly rejected for this feature:** presenting polls as a stream,
  estimating missed counts, replaying a fresh install's visible page as new,
  fabricating liveness/staleness, promoting PARTIAL to AUTHORITATIVE, or adding
  adapter-owned durable reconciliation state.
- **Non-foreclosure check:** event/polling vocabulary stays in adapter-owned JSON
  schemas; exact resource identities retain the existing adapter/kind/id tuple;
  no Pi-specific/core enum, surface-only state, second-operator assumption,
  federation key, dynamic renderer, or parked UI/mesh direction is introduced.

## Implementation summary

Implemented the complete polling ingestion runtime with one cohesive owner and
no sub-worker fan-out. `TokenCommunePoller` runs immediate non-overlapping
cycles over six concurrent reads, converts failed snapshot sources to explicit
unavailable evidence, honors only normalized safe Retry-After advice, projects
and ingests a fresh PARTIAL report, and then performs acknowledgement-aware
latest-50 event reconciliation. The process supervises the poller beside the
held-open delivery subscription under one abort scope.

The event boundary now has one disposition registry and two adapter-owned closed
JSON schemas. Exactly `capacity_shift`, `auth_broken`, `windfall`, `fingerprint`,
and `member` become resource-scoped STATUS Observations. `window_exhausted` and
`calibration` stay declared-only. The first visible page is a non-replayed
baseline; overlap emits newly visible ids; unanchored/saturated/history-empty
transitions emit measured gaps with no missed count or continuity claim.
Acknowledged state is bounded and process-local and advances only after a core
event id. Reconnect always accepts a new PARTIAL report before event repair, and
no heartbeat, stale/current mutation, or polling-as-streaming signal was added.

Implementation discovery: the existing core classified every STATUS Observation
as command-lifecycle evidence and therefore rejected the designed generic
resource status shape. The foundation and generated Observation contract already
commit generic source-authenticated status emissions, so implementation added a
narrow fail-closed acceptance case: no command correlation, exact RESOURCE
target, complete non-empty identity, and `FailureCode.UNSPECIFIED`. Correlated
STATUS remains command lifecycle; malformed uncorrelated STATUS still rejects.
The audit projection also avoids labeling generic resource status as
`CommandRunning`. This was the smallest change that made the existing foundation
contract executable without weakening command transition validation.

### Execution and verification

- Execution capability: `openai-codex/gpt-5.6-sol`, reasoning `high` (explicit
  caller worker capability). Direct host ownership was used because the feature
  is one tightly coupled scheduler/projector/tracker/process seam and the caller
  explicitly prohibited sub-worker fan-out.
- Effective review weight: `thorough` (explicit caller/autopilot override).
  Per the explicit boundary, implementation stops at feature `review`; this
  worker did not self-review.
- `cd token-commune-adapter && npm run build`: pass.
- `cd token-commune-adapter && npm test`: pass, 55/55 tests (34 baseline + 21).
- `cargo test -p patchbay-core --test acceptance_observation`: pass, 16/16.
- `cargo test -p patchbay-core --tests`: pass. The separate repository doctest
  invocation remains pre-existingly broken by rustdoc dependency resolution and
  is not part of this package's requested gate.
- `git diff --check`: pass.
- Mutation self-checks, all observed failing and reverted: PARTIAL promoted to
  AUTHORITATIVE (2 focused failures); event dedup advanced before core ack (1
  focused retry failure); `calibration` promoted out of declared-only handling
  (1 focused coverage failure). The restored tree passed the full suite.

All six child checkpoints advanced directly from `implementing` to `done` after
integrated verification. No foundation assertion became false: polling remains
explicitly non-streaming, resource reports remain PARTIAL, stale authority stays
with core stream loss, and stronger replay/exactly-once behavior remains a
reserved external prerequisite.

## Review handoff

Effective implementation review weight is **thorough** (source: explicit
caller). Child stories close directly on green verification; the integrated
feature then runs review → receiver adjudication → fix/verify → fresh-context
review until a pass yields no receiver-confirmed material current-cycle blocker.
Reviewer findings are proposals, not authority. The active autopilot final
completion review must receive the same weight unchanged.

## Review (thorough, 2026-08-07)

Cross-model (gpt-5.6-sol vs zai/kimi host), convergence.

- **Pass 1 (REQUEST CHANGES):** 2 blockers + 2 importants. (1) the core STATUS exception (`core/src/acceptance/observation.rs`) reimplemented resource-shape validation and accepted noncanonical mixed resource/session scopes (resource + runtime_session_id/adapter/legacy_audit) — an ontology leak; (2) Retry-After parsing/scheduling was unbounded (permissive Date.parse accepted "-1"/year-9999; huge values stalled for days or exceeded Node's setTimeout max → hot poll loop) — remotely steerable. Importants: the new core STATUS seam lacked authenticated service-path coverage (direct core test only); dedup accepted acknowledgement from the wrong authority domain. All fixed at `1afc46f` — core now derives validity via `ResourceIdentity::try_from_scope` (canonical, adapter-neutral); strict Retry-After parsing + 1h cap below Node's timer ceiling; authenticated server/service test (attach→report→STATUS→one Observation, cross-adapter + mixed-target rejection); dedup requires exact domain equality + positive LSN. Adapter 55→57 tests; cargo core tests + server service test green; mutation-checked.
- **Pass 2 (APPROVE):** all four fixes verified correct + mutation-sensitive; polling≠streaming, bounded-gap, source-time, declared-only-kind, disconnect→stale honesty invariants re-confirmed intact; no core-ontology leak; 57/57 adapter tests, cargo core green, server test green.

Note: this feature added narrow, adapter-neutral core support for uncorrelated resource STATUS observations (canonical source-authenticated status facts are core ontology per `docs/PROTOCOL.md`); review confirmed no token-commune vocabulary entered core. Converged. Advanced to `done`.
