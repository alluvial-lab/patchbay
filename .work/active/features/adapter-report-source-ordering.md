---
id: adapter-report-source-ordering
kind: feature
stage: implementing
tags: [adapter, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Adapter report source-ordering (stale-report rollback prevention)

## Brief
Close the adapter-source-ordering gap split out of `sessions-soundness-coverage`. Absorbs:

- `backlog-session-report-source-ordering` — **OPEN**: `SessionReport` (`contracts/proto/patchbay/adapter_control.proto:35-47`) carries no adapter-side revision, so the core treats arrival order as source order. A delayed-but-sequential stale report carrying an older value derives a valid backward mutation (`B → A`) and rolls a mutable field backward. LSN/`from` checks protect durable replay order but cannot identify a stale *source* report. *Src:* `feature-session-model-field` review (2026-07-24).

## Direction
Add a monotonic, generation-scoped adapter-side report revision to `SessionReport`; core ingest rejects (or marks stale) reports whose revision is not greater than the last applied for that session generation. This is a wire **contract change** → requires a conformance vector + extension-seams-registry classification; consider whether it generalizes beyond `model` to other report-carried mutable fields. Distinct from `session-registry-replay-domain-soundness`: LSN ordering (core arrival) ≠ source ordering (adapter). The Pi adapter's promise-tail serialization mitigates this for v0.1.0's only adapter; this closes it at the contract.

## Foundation references
- `docs/PROTOCOL.md` — session reports; mutable non-identity metadata (current model)
- `contracts/proto/patchbay/adapter_control.proto` — `SessionReport`
- Code: `core/src/session/ingest.rs`, `core/src/session/registry.rs`

## Design decisions

- **Guard the whole report, not only `model`.** One source cursor orders connectivity, activity, project/cwd/name, model, and any later fields carried by `SessionReport`. A per-field model revision would leave the same rollback defect on the other mutable fields and permit mixed snapshots that no adapter actually reported.
- **The cursor is `(adapter_generation, revision)` inside one runtime-session generation.** `revision` is strictly increasing within the current adapter incarnation; a strictly newer authenticated adapter generation may reset it. A strictly newer runtime-session generation starts a fresh cursor scope. This preserves the brief's generation scoping without deadlocking a legitimate adapter-process replacement whose local counter restarts.
- **The current authenticated attachment binds the producer epoch.** Session-report ingress requires `source_cursor.adapter_generation` to equal the current registered adapter generation and replaces source identity with the authenticated adapter context. Old attachment tokens, old adapter generations, lower runtime generations, and non-increasing revisions are inert.
- **Only a strictly newer cursor can mutate or advance authority.** Missing cursors and revision zero fail boundary validation. For the same runtime-session and adapter generation, `revision <= last_applied_revision` returns a stale-source rejection, appends no session-state event, and records `STALE_EVENT_IGNORED` audit evidence. Equality is rejected rather than silently accepting same-cursor equivocation.
- **Persist one atomic full-report event for equal-generation reports.** Every accepted newer report advances the durable source watermark even when its visible values are unchanged. Registration and runtime-generation replacement remain their existing atomic event kinds with the source cursor added; an equal-generation report becomes one `SessionReportApplied` event rather than a partially committed sequence of field deltas.
- **Retain old session delta variants for durable replay and core-owned degradation only.** Existing logs may contain `SessionConnectivityChanged`, `SessionActivityChanged`, `SessionRelabeled`, and `SessionModelChanged`, and adapter disconnect still needs a core-authored stale transition that must not advance adapter source order. New adapter reports do not dual-write those deltas.
- **Fresh wire input changes in place; durable history remains readable.** Patchbay owns the Pi adapter and unpublished generated contracts, so old adapters receive no compatibility shim. Existing durable session events are real persisted data and remain replayable with `last_source_cursor = None` until the first revised report establishes the watermark.
- **Assurance target is a named checked property plus a promoted executable example.** `SessionReportSourceOrdering` uses pre-state pending-report evidence in a focused Quint model, a claim-breaking mutation, and `session-report-source-ordering.json` executed through authenticated server ingress. If the model or vector cannot satisfy the genuine-checking/promotion gates, the behavior must remain honestly stated-normative and the feature cannot claim checked-normative completion.
- **Autopilot rationale and review posture.** The review-vetted 2026-08-09 direction settles the product contract. Routine choices were resolved toward one generated cursor, one atomic event, and fail-closed stale handling. The caller selected `openai-codex/gpt-5.6-sol` and `review_weight: thorough`; implementation, feature review, and final completion review must retain that weight.

## Codebase mapping

Direct reading covered all eight foundation docs; generated Protobuf, registry-boundary, fail-fast, and durable-projection patterns; the adapter service authentication gate; session report ingestion, event encoding, replay, registry, snapshot materialization, and property tests; the Pi adapter's per-session promise tails and reattach behavior; the session-generation model; and vector promotion/runner machinery. No exploratory subagent was used because the caller explicitly prohibited nested delegation; the report surface was bounded and enumerable through `SessionReport` construction and fold call sites.

## Current-HEAD reconciliation (2026-08-10)

- `SessionRegistry::new(authority_domain_id)` is now fallible and domain-bound. Its `applied_events` ledger retains exact raw envelopes for owned `SESSION_STATE` and `SECURITY_LOCKDOWN` events, accepts only exact owned-event re-delivery, and rejects conflicting same-LSN envelopes. The new report fold must claim an owned event identity only after the complete atomic fold succeeds and must preserve sibling-event no-op behavior.
- Full recovery already uses the shared `validate_next_replay_event` boundary, so gap/duplicate/`UNSPECIFIED` rejection remains outside the registry's owned-event high-water logic. Server aggregate catch-up stages every projection and publishes only after the complete suffix validates; this feature does not replace either prefix discipline.
- `SessionGenerationBumped.spawn_origin` has landed at tag 11. The next stable tag for `SessionGenerationBumped.source_cursor` is therefore **12**, not the design-time sketch's tag 11. The other next tags remain `Session.last_source_cursor = 15`, `SessionRegistered.source_cursor = 11`, and `SessionStateEvent.report_applied = 8` after regeneration from `.proto`.
- Session report ingress currently rebuilds under `CoreDecisionGate`, converts the generated RPC report into a handwritten core DTO, then append-and-folds up to four legacy deltas before rebuilding again. Reconciliation keeps the gate/rebuild safety and the append-then-fold hot path, removes the DTO and only the adapter-report multi-delta writer, and retains legacy/core-authored delta constructors and folds.
- The authenticated adapter registry now owns the current attachment generation. Server ingress will validate the generated source cursor against that registration before core lookup/append; core still independently restores and compares the durable last cursor.
- Pi currently computes `#identity(entry, model)` only when its promise tail runs. The sequencer must allocate the revision and capture identity/model/state before chaining while retaining transcript/session ordering and same-process reattachment behavior.
- Current registries contain 53 vectors (16 promoted), 53 modeled properties (8 promoted), and no checked-normative property. The implementation must add the property/vector to the live registries and regenerate both documentation blocks; no design-time count or table shape is copied by hand.
- Delivery posture remains direct-read, single feature owner, no nested agents/peeragent (caller boundary). Execution capability is `openai-codex/gpt-5.6-sol` at maximum reasoning for the wire/durability/formal surface; effective review weight is `thorough` from the caller, with stop-at-review for the feature and the `[verification]` child reserved for the project deep lane.

## UI fallback

No UI surface. The current snapshot stays on the same session fields and gains only source-order evidence; no screen structure or presentation state changes, so no mockup is required.

## Architectural choice

### Options considered

1. **Keep an in-memory last-report counter in the server.** This is the smallest patch, but a core restart forgets the fence and can accept the delayed rollback the feature exists to prevent. It also makes hot behavior disagree with replay.
2. **Attach a revision to each existing field delta plus a no-change watermark event.** This preserves the current event taxonomy, but one source report can append several events. A failure after the first append either advances the watermark before the report is complete or requires equal-revision continuation rules that weaken the simple fence.
3. **Chosen: generated producer cursor plus one atomic full-report event.** The adapter report carries an authenticated producer epoch and revision; the core appends one report-shaped event, and replay restores both values and the last cursor. This costs one new generated event variant but removes the multi-delta partial-append path and generalizes the guard to every report field.

The chosen option follows generated-contract, fail-fast-boundary, and durable-log-projection patterns. The trickiest unit is **making the source watermark atomic and restart-stable while preserving legacy durable events and core-authored stale transitions**; that unit is designed before the Pi producer and promotion evidence.

## Implementation Units

### Unit 1: Generated source cursor, atomic report event, and foundation classification

**Files**: `contracts/proto/patchbay/sessions.proto`, `contracts/proto/patchbay/adapter_control.proto`, `contracts/rust/src/gen/patchbay/patchbay.rs`, `contracts/ts/src/gen/patchbay/sessions_pb.ts`, `contracts/ts/src/gen/patchbay/adapter_control_pb.ts`, `docs/PROTOCOL.md`, `docs/SECURITY.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`, `docs/ADAPTER-PI.md`

**Story**: `adapter-report-source-ordering-contract-foundation`

```proto
// sessions.proto — wire-shape authority
message SessionReportSourceCursor {
  Generation adapter_generation = 1;
  uint64 revision = 2;
}

// Move the existing package-level message here so adapter ingress and the
// durable event reuse one generated report shape.
message SessionReport {
  AdapterId adapter_id = 1;
  string deployment_scope = 2;
  RuntimeSessionId runtime_session_id = 3;
  Generation session_generation = 4;
  SessionConnectivityState connectivity = 5;
  SessionActivityState activity = 6;
  string project = 7;
  string cwd = 8;
  string name = 9;
  TypedCorrelation spawn_origin = 10;
  string model = 11;
  SessionReportSourceCursor source_cursor = 12;
}

message SessionReportApplied {
  SessionReport report = 1;
  // Absent only when upgrading a legacy projected generation whose durable
  // events predate source cursors.
  SessionReportSourceCursor previous_source_cursor = 2;
}

message SessionStateEvent {
  AuthorityDomainId authority_domain_id = 1;
  oneof mutation {
    SessionRegistered registered = 2;
    SessionGenerationBumped generation_bumped = 3;
    SessionConnectivityChanged connectivity_changed = 4;
    SessionActivityChanged activity_changed = 5;
    SessionRelabeled relabeled = 6;
    SessionModelChanged model_changed = 7;
    SessionReportApplied report_applied = 8;
  }
}

// New fields use the next available tags.
// Session.last_source_cursor = 15
// SessionRegistered.source_cursor = 11
// SessionGenerationBumped.source_cursor = 11
```

**Implementation notes**:

- Move, do not copy, `patchbay.SessionReport` from `adapter_control.proto` to `sessions.proto`; the package-qualified type remains unchanged and `ObservationRequest` continues to reference it. Generate both Rust and TypeScript outputs from the proto source and never edit generated files manually.
- Use a nested message so missing source evidence is distinguishable from zero. `revision = 0` is invalid on fresh ingress; adapter generation follows the existing generated `Generation` shape and must equal the current attachment.
- `Session.last_source_cursor` makes the accepted producer watermark observable in snapshots and available to future typed checkpoint recovery without confusing it with the core-owned `last_authoritative_lsn`.
- Add the source-ordering rules to PROTOCOL's Sessions/Session state axes and Extension seams registry; extend SECURITY's stale-adapter-report boundary; reserve and classify `SessionReportSourceOrdering` in VERIFICATION; distinguish source cursor/revision from core LSN `Revision` in GLOSSARY; document the Pi producer rule in ADAPTER-PI.
- Classification: committed session-adapter contract; reserved future multi-producer/vector-clock or per-field merge policy; explicitly reject treating core arrival LSN or promise-tail serialization as source authority.

**Acceptance criteria**:

- [ ] Rust and TypeScript expose one generated `patchbay.SessionReport`, `SessionReportSourceCursor`, `SessionReportApplied`, and snapshot cursor shape with stable tags.
- [ ] Missing cursor, missing cursor adapter generation, zero revision, unknown session states, and malformed identity fail before lookup or append.
- [ ] Foundation prose distinguishes runtime-session generation, adapter generation, adapter source revision, and core LSN and records the three-way extension classification without claiming Pi-specific core semantics.
- [ ] `buf generate`, both contract builds, and generated-drift verification pass with no handwritten DTO mirror.

### Unit 2: Atomic core fence, replay fold, audit, and snapshot watermark

**Files**: `core/src/session/ingest.rs`, `core/src/session/events.rs`, `core/src/session/registry.rs`, `core/src/session/mod.rs`, `core/tests/sessions_ingest.rs`, `core/tests/sessions_registry.rs`, `core/tests/sessions_proptest.rs`, `server/src/adapter_service.rs`, `server/src/adapter_service/tests.rs`, `server/src/state.rs`

**Story**: `adapter-report-source-ordering-core-fence`

```rust
// core/src/session/registry.rs
pub struct SessionRecord {
    pub identity: SessionIdentity,
    pub state: SessionState,
    pub project: String,
    pub cwd: String,
    pub name: String,
    pub model: String,
    pub last_source_cursor: Option<SessionReportSourceCursor>,
    pub last_authoritative_lsn: Option<u64>,
    pub tombstoned: bool,
    pub superseded_at_lsn: Option<u64>,
}

// core/src/session/ingest.rs — consume the generated boundary type directly.
pub async fn ingest_session_report<S, L>(
    storage: &S,
    session_lookup: &L,
    authority_domain_id: &AuthorityDomainId,
    report: patchbay_contracts::patchbay::SessionReport,
) -> Result<IngestResult, SessionError>
where
    S: Storage,
    L: SessionLookup;

pub enum IngestResult {
    Registered { event_id: EventId },
    GenerationBumped {
        event_id: EventId,
        from_generation: Generation,
        to_generation: Generation,
    },
    ReportApplied { event_id: EventId },
}

pub enum SessionError {
    // existing variants retained
    StaleSourceCursor {
        live: SessionReportSourceCursor,
        reported: SessionReportSourceCursor,
    },
}
```

**Implementation notes**:

- Server ingress authenticates under `CoreDecisionGate`, verifies report adapter id, loads the current registration, requires exact source adapter-generation equality, and passes the generated report plus envelope authority domain to core. Payload identity never overrides authenticated source.
- Core validation precedes lookup. Compare runtime-session generation first. Lower generation remains `StaleGeneration`; greater generation atomically resets source scope through `SessionGenerationBumped`; equal generation requires a cursor strictly after `last_source_cursor`, comparing adapter generation first and revision second. A legacy `None` cursor accepts the first valid revised report.
- Validate connectivity/activity transitions only after the source cursor is known current. Stale reports therefore receive the precise stale-source outcome rather than being misclassified by whatever backward field transition they imply.
- Registration and generation bump remain one event and carry their incoming cursor. Equal-generation reports append one `SessionReportApplied { previous_source_cursor, report }`, even when all displayed values are unchanged, so restart cannot forget a no-visible-change watermark.
- `SessionRegistry::observe_report_applied` requires the full identity and `previous_source_cursor` to match projected pre-state, rechecks strict cursor progression and state-axis adjacency, then atomically replaces all report-carried mutable fields and advances both cursor and LSN. Existing delta folds preserve `last_source_cursor`; this is required for disconnect/lockdown degradation and legacy replay.
- Remove the adapter-report multi-delta append/warm/retry machinery and its result variants. One accepted report is one durable event; storage failure leaves both projection and watermark unchanged, so the same report can be retried before any commit.
- Map stale source cursors to `FAILED_PRECONDITION`; record `AUDIT_EVENT_KIND_STALE_EVENT_IGNORED` with `FAILURE_CODE_STALE_EVENT` and bounded reason `session_report_source_cursor_stale`. No session-state event or projection mutation occurs.
- Materialize `last_source_cursor` into `Session` snapshots; keep `last_authoritative_lsn` as core commit order. Rebuild before/after ingress remains the production warm path, without claiming the server gate as core writer safety.

**Acceptance criteria**:

- [ ] Report `r3: model=B` followed by delayed `r2: model=A` leaves hot, replayed, and materialized state at `B/r3`, appends no second session-state mutation, and records stale audit evidence.
- [ ] Equal and lower revisions cannot mutate any report-carried field; a higher identical-value report still commits its watermark and survives restart.
- [ ] A newer adapter generation and a newer runtime-session generation may each establish a fresh positive local revision, while old adapter/runtime generations remain inert.
- [ ] Registration, generation replacement, lockdown clamping, disconnect staleness, spawn-origin handling, legacy delta replay, and full identity isolation remain correct.
- [ ] One report changing connectivity, activity, labels, and model appends exactly one report event; an injected append failure creates no partial report or in-memory advance.

### Unit 3: Pi source sequencing and immutable enqueue snapshots

**Files**: `pi-adapter/src/core_client.ts`, `pi-adapter/src/main.ts`, `pi-adapter/src/session_registry.ts`, `pi-adapter/tests/report_source_ordering.test.ts` (new), `pi-adapter/tests/e2e.test.ts`

**Story**: `adapter-report-source-ordering-pi-sequencer`

```ts
// pi-adapter/src/core_client.ts
async reportSession(
  identity: SessionIdentity,
  sourceCursor: SessionReportSourceCursor,
  activity: SessionActivityState,
  connectivity?: SessionConnectivityState,
): Promise<EventId | undefined>;

// pi-adapter/src/main.ts
interface SessionReportSequence {
  sessionGeneration: number;
  revision: bigint;
}

readonly #sessionReportSequences =
  new WeakMap<RuntimeSessionEntry, SessionReportSequence>();

#nextSessionReportCursor(
  entry: RuntimeSessionEntry,
): SessionReportSourceCursor;
```

**Implementation notes**:

- Allocate a monotonically increasing bigint revision at enqueue time, not when the promise tail later executes. Capture the complete identity/model/state report at that same point, then enqueue that immutable report/cursor pair. This prevents an older revision from being paired with newer mutable state.
- Scope the local counter to the stable runtime entry and current runtime-session generation. Reset the local revision to one when Pi reports a strictly newer session generation. Include the process's configured adapter generation in the generated cursor.
- Reattachment inside the same process retains the counter. A replacement adapter process must use a newer adapter generation under the existing lifecycle contract; it may then restart local revision at one. Misconfigured equal/lower process generations fail closed instead of rolling state backward.
- Keep promise-tail serialization for efficient in-order emission, but document it as defense in depth. Core acceptance depends only on authenticated cursor order.
- Build `SessionReportSourceCursor` with generated schemas/types. Do not add a parallel string or number DTO; guard bigint overflow before constructing the uint64 field.

**Acceptance criteria**:

- [ ] Reports queued for one runtime generation carry revisions `1, 2, ...` in enqueue order and immutable field snapshots, including model-change reports queued behind another observation.
- [ ] Runtime-session generation replacement resets local revision to one; adapter-generation replacement also starts at one with the newer producer epoch.
- [ ] An unauthenticated reattach retry reuses the same report cursor and payload rather than allocating a second revision.
- [ ] Independent sessions maintain independent counters; existing transcript ordering, delivery, model-change, reconnect, and real-process E2E tests remain green.

### Unit 4: Genuine model, promoted vector, and mutation-sensitive integration evidence

**Files**: `specs/seed/session_report_source_ordering.qnt` (new), `specs/seed/session_report_source_ordering.emitted.tla` (generated), `contracts/vectors/session-report-source-ordering.json` (new), `contracts/scripts/check-models.mjs`, `contracts/scripts/check-vectors.mjs`, `server/tests/conformance_vectors.rs`, `docs/VERIFICATION.md`

**Story**: `adapter-report-source-ordering-conformance`

```quint
// Model shape; arrival and application are separate so Apply cannot rewrite
// the pending evidence used by the oracle.
var phase: str
var liveSessionGeneration: int
var liveAdapterGeneration: int
var lastSourceRevision: int
var mutableValue: str
var pendingSessionGeneration: int
var pendingAdapterGeneration: int
var pendingRevision: int
var pendingValue: str

// Exact promoted property id: SessionReportSourceOrdering
temporal session_report_source_ordering = always(
  phase == "pending"
  and pendingSessionGeneration == liveSessionGeneration
  and (pendingAdapterGeneration < liveAdapterGeneration
    or (pendingAdapterGeneration == liveAdapterGeneration
      and pendingRevision <= lastSourceRevision))
  implies (
    next(mutableValue) == mutableValue
    and next(lastSourceRevision) == lastSourceRevision
  )
)
```

```json
{
  "vector_id": "session-report-source-ordering",
  "property_id": "SessionReportSourceOrdering",
  "promotion_status": "promoted",
  "implementation_checks": [
    { "runner": "rust-server", "case": "session_report_source_ordering" }
  ],
  "proto_fields_constrained": [
    "patchbay.SessionReport.session_generation",
    "patchbay.SessionReport.source_cursor",
    "patchbay.SessionReportSourceCursor.adapter_generation",
    "patchbay.SessionReportSourceCursor.revision",
    "patchbay.SessionReportApplied.report",
    "patchbay.Session.last_source_cursor",
    "patchbay.Session.model"
  ]
}
```

**Implementation notes**:

- Model a separate environment arrival step and pending pre-state, then an application/rejection step. The property must not reuse a production guard or action-recorded claim. Include runtime-generation reset, adapter-generation reset, equal/lower revision, and delayed-value rollback traces.
- Add a claim-breaking mutation that admits `pendingRevision <= lastSourceRevision`; the independent property/check must find a counterexample. Compile and commit the emitted TLA inspection artifact, but do not describe it as an independent checker lane.
- Add the property to model/vector registries and an invariant expectation checker over the vector's raw expected outcome. Promote only when the model metadata, mutation witness, vector expectation, exact runner report, and generated traceability tables all pass.
- The authenticated server runner applies `A/r1`, `B/r3`, then delayed `A/r2`; asserts stale status/audit, no session event, snapshot `B/r3`, and hot/replay agreement. It also proves that a new runtime or adapter generation can reset local revision without admitting the old producer.
- Add bounded Rust property coverage over report sequences with an independent `(runtime generation, adapter generation, revision)` truth table and a stale-guard mutant. Generated Rust/TypeScript builds and Pi producer tests are supporting evidence, not substitutes for the real ingress vector.

**Acceptance criteria**:

- [ ] `SessionReportSourceOrdering` passes the real checker at the documented bound and its claim-breaking mutation fails.
- [ ] The promoted vector executes the exact authenticated server case and traceability names every constrained proto field without manual generated-table edits.
- [ ] The vector fails if source comparison is weakened, the source watermark is not durable, stale audit is omitted, or the snapshot rolls back.
- [ ] Assurance language remains exact: checked-model + promoted implementation vector/checked-normative only after both promotion gates pass; no claim of unbounded, multi-writer, or end-to-end adapter correctness.

## Implementation Order

1. `adapter-report-source-ordering-contract-foundation` — establish the generated cursor/event contract and normative classification.
2. In one feature-owned implementation wave after the contract:
   - `adapter-report-source-ordering-core-fence` — consume and durably enforce the cursor.
   - `adapter-report-source-ordering-pi-sequencer` — emit generated cursors and immutable report snapshots.
3. `adapter-report-source-ordering-conformance` — attack the integrated seam with the genuine model, mutation, promoted vector, real ingress runner, and traceability checks.
4. Run focused and workspace verification, close child checkpoints by their evidence policy, then review the integrated feature at the caller's explicit `thorough` weight until a pass has no receiver-confirmed material current-cycle blocker.

One feature-owning worker should carry all four checkpoints. Core and Pi files are disjoint after the contract but the producer/consumer semantics and promotion evidence are one wire-contract review boundary; splitting ownership would increase handoff risk.

## Simplification

- Replace the report path's connectivity/activity/model/relabel multi-append branches, `DeltasApplied`, partial-prefix warm helpers, and retry-after-partial-failure test fixture with one atomic report append.
- Move the generated `SessionReport` definition rather than create a durable-event copy or handwritten Rust/TypeScript DTO.
- Retain legacy delta decoders and core-owned connectivity changes because durable history and adapter-disconnect degradation earn those paths; do not keep the old adapter-report writer in parallel.
- Add no per-field revision map, vector clock, wall-clock ordering, second store, capability toggle, or Pi-specific core state.
- Test the stable trust/durability seam and one generalized rollback witness; do not duplicate generated-field serialization tests across every package.

## Testing

- **Generated boundary tests** protect presence, positive revision, current attachment generation, generated Rust/TypeScript parity, and drift.
- **Core interface/regression tests** protect strict cursor ordering before transition derivation, atomic append-before-fold, generation resets, all-field fencing, stale audit, legacy replay, and hot/replay/snapshot convergence.
- **Pi producer tests** protect enqueue-order allocation, immutable snapshots, per-session/generation scope, and retry cursor reuse; the promise tail is not treated as the safety oracle.
- **Formal + mutation evidence** protects the stale-source-inert property with pre-state pending evidence and proves the checker catches a weakened comparison.
- **Promoted server vector** protects the actual authenticated wire/core/replay/snapshot seam. It is the required executable example, not metadata-only traceability.
- **Test removal**: retire the implementation-bound multi-delta ordering/partial-append tests after their path is deleted; retain legacy event-fold fixtures and existing generation/identity property coverage.

## Verification commands

```bash
(cd contracts && buf generate)
cargo test -p patchbay-contracts
npm --prefix contracts/ts run build
cargo test -p patchbay-core --test sessions_ingest
cargo test -p patchbay-core --test sessions_registry
cargo test -p patchbay-core --test sessions_proptest
cargo test -p patchbay-core-server adapter_service
npm --prefix pi-adapter test
(cd specs/seed && quint parse session_report_source_ordering.qnt)
(cd specs/seed && quint compile session_report_source_ordering.qnt)
(cd specs/seed && echo y | quint verify session_report_source_ordering.qnt --temporal session_report_source_ordering --max-steps 10)
node contracts/scripts/check-models.mjs
node contracts/scripts/check-vectors.mjs
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
# Run after generated artifacts are committed to the candidate snapshot:
npm --prefix contracts/ts run check:drift
```

## Risks

- **Producer restart with a reused adapter generation fails closed.** A new process that resets revision without advancing its existing adapter-generation epoch will be rejected. That is safer than rollback and matches the current adapter-generation lifecycle, but configuration/E2E coverage must make the operational requirement explicit.
- **The atomic event is a durable-schema change.** Legacy events must continue replaying, while new adapter reports must use only the new atomic path. Mixed histories are the key compatibility case; fallback is to preserve the legacy folds, not dual-write or migrate production data autonomously.
- **Core-authored staleness must not consume adapter source order.** Disconnect and lockdown may change projected connectivity between reports. Those paths preserve `last_source_cursor`; the next current report needs a newer cursor to restore live state.
- **Equality rejection is intentionally loud.** It prevents same-cursor equivocation but means adapters must retry a report with a new revision unless they know no commit occurred. Pi's unauthenticated retry reuses the request only because authentication fails before report acceptance; ambiguous transport retry policy remains outside this feature.
- **Formal promotion can overclaim if arrival evidence is action-authored.** The separate pending phase and mutation witness are mandatory. If the checker/tooling cannot establish the named claim, keep it stated-normative and do not promote the vector as checked-normative evidence.
- **Field growth can bypass the fence if a later writer adds a parallel event path.** Reusing the generated full `SessionReport` in `SessionReportApplied` makes new report fields atomic by default; review must reject per-field adapter writers that omit the cursor.

## Extension pressure classification

- **Committed session-adapter contract:** an authenticated runtime-session report carries `SessionReportSourceCursor { adapter_generation, revision }`; only a current producer cursor strictly newer within the runtime-session generation can mutate report-carried state; accepted reports durably advance the cursor in one event.
- **Reserved seams:** multiple concurrent report producers for one session, vector-clock or per-field merge semantics, richer structured session metadata, and future typed checkpoint recovery from `Session.last_source_cursor`. These are named but not implemented; the current cursor retains producer and runtime-generation demarcators so promotion is additive rather than a retrofit.
- **Explicitly rejected for this feature:** core arrival LSN as adapter source order, wall-clock timestamps as ordering authority, Pi promise tails as the contract, model-only revisioning, independent per-field counters, missing/zero cursor compatibility shims on fresh ingress, and silent mutation on equal/lower cursors.
- **Adapter/surface neutrality:** the cursor is mandatory generated runtime-session adapter evidence, not a Pi capability or UI state. Operational-resource report ordering keeps its separate generated report/reconciliation contract. No surface behavior changes.
- **Parked-idea pressure:** multi-human/federation remains isolated by authority-domain and authenticated adapter identity; desktop/mobile/skin and agent-mesh ideas are unaffected.

## Other agent review

- Invoked because: this is a cross-language wire change whose stale-source failure can roll durable session state backward after restart.
- Fixed/active blockers: the design adds an authenticated producer epoch, persists no-change watermarks, makes equal-generation reports atomic, keeps legacy/core-authored events distinct, and requires genuine model + real ingress evidence.
- Parked: multi-producer/vector-clock and per-field merge policies wait for demonstrated adapter pressure; richer model descriptors remain the existing reserved seam.
- Rejected: in-memory watermarking, fragmented delta cursors, and arrival-order authority because each fails restart or partial-append safety.
- Skipped/degraded: the delegated endpoint explicitly forbids nested subagents and peeragent, so no independent design-time pass ran. This is non-blocking by policy. The effective implementation/feature/final completion review weight remains `thorough` (source: explicit operator selection).

## Status (reconciled 2026-08-10)
Current HEAD reconciliation is recorded above. Implementation remains at `implementing`; the landed domain-bound exact-envelope session replay, shared prefix validation, cancellation-safe aggregate publication, diagnostics replay fixes, and tag-11 generation-bump correlation are preserved inputs rather than stale design assumptions.
