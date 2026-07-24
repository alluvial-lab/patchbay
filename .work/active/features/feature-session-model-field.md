---
id: feature-session-model-field
kind: feature
stage: review
tags: [protocol, ux, fast-follower]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-23
updated: 2026-07-24
research_origin: null
---

# Feature: surface the agent model in session reports

**Promoted 2026-07-24** into the pre-release fix wave.

Surfaced in live use (2026-07-23): the operator asked "do we have a way to
return what model the pi agent is running with?" The answer today: it's
recorded (the session transcript's `model_change` event — e.g.
`provider: kimi-coding, modelId: k3`) and the adapter knows it
(`session.model`), but nothing surfaces it to the cockpit or CLI.

Operator decision (2026-07-23): park as a **proper small feature** (option b),
not the quick observation-channel hack.

## Shape

A `model` field on the session report contract:

- **Proto:** add `model` (string, e.g. `kimi-coding/k3`) to
  `SessionRegistered` (and `Session`, so it materializes in snapshots). This
  is a contract change → `contracts/` regen (buf generate TS; `git checkout --
  contracts/rust/src/gen` + `cargo build` for Rust).
- **Core:** ingest it through the session registry (the
  `SessionRegistered`/`SessionRelabeled` path or a `SessionModelChanged`
  mutation if the model can change mid-session — likely yes, so model the
  mutation, not just the registration field).
- **Adapter:** report the session's current model (`session.model`) at
  registration and on change (subscribe to `model_change`).
- **Surfaces:** cockpit session row/detail header shows it;
  `cli session-health` prints it.

## Considerations

- Models CAN change mid-session (`model_change` event), so treat it as
  mutable session state with its own mutation, not a registration-time
  constant.
- The quick alternative (emit a session-model observation, fold it into
  `SessionView.model`) requires no contract change but was rejected by the
  operator in favor of the proper field.

## Simplification opportunity

None identified — additive contract field plus plumbing.

## Design decisions

- **Mutable model shape**: `SessionModelChanged { identity, from, to }` is a
  first-class durable `SessionStateEvent` mutation; `model` is also carried on
  registration, generation replacement, and snapshots. Pi writes a
  `model_change` session entry, so registration-only metadata would make the
  canonical snapshot stale after a mid-session switch.
- **Model representation**: v0.1.0 uses an adapter-reported opaque string in
  normalized `provider/modelId` form (for example `kimi-coding/k3`). The empty
  string is the wire representation of unavailable/unknown; surfaces render it
  as `Model unknown`. The core does not parse it or make it routing/authority
  input, preserving adapter neutrality.
- **Generation replacement**: `SessionGenerationBumped` also carries `model`.
  A replacement report establishes the new generation's full current state;
  cloning the old generation's model would repeat the previously fixed
  state/metadata inheritance bug.
- **Durable-delta validation**: `SessionModelChanged` records both `from` and
  `to`, matching the existing state-axis deltas. Replay verifies the full
  identity tuple and that `from` equals the projected value before assigning
  `to`; it never treats model text as identity.
- **Pi change source**: the adapter listens for Pi's active-session
  `entry_appended` event whose `SessionEntry` is `ModelChangeEntry`
  (`type: "model_change"`, `provider`, `modelId`), rather than inferring a
  switch from transcript text or periodically polling. Registration reads the
  authoritative `session.model` getter.
- **UI treatment**: this is a minor extension of the established session
  row/detail and CLI diagnostic patterns, not a new screen or composition.
  No mockup is needed; the list, detail header, and `session-health` already
  render session metadata and the change adds one honest field.

## Extension pressure classification

- **Committed v0.1.0:** an adapter-reported current model string on the
  session contract, plus the durable `SessionModelChanged` mutation. It is
  mutable metadata: it never changes the session identity tuple or grants.
  `docs/PROTOCOL.md` will state that contract and add the consolidated
  extension-registry row.
- **Reserved seam:** richer structured model capability/availability metadata,
  model history as a separate operator projection, and adapter-specific model
  descriptors. The opaque current string preserves those extensions without
  introducing Pi concepts into core state.
- **Explicitly rejected for v0.1.0:** the observation-channel-only
  `SessionView.model` workaround. It has no durable snapshot/replay contract
  and loses correctness when the stream is missed.
- **Conformance-vector decision:** add one *draft* vector,
  `session-model-change-preserves-identity`, against the stated-normative
  `LabelsCannotOverrideIdentity` property. It constrains the new mutation's
  wire fields and proves the intended observable shape: model changes while
  adapter id, deployment scope, runtime id, and generation remain unchanged.
  It is intentionally draft because no promoted vector executor/model proves
  this metadata mutation; Rust replay tests are the executable enforcement.
  No new formal state-machine property is warranted because this adds no
  state-enum, authority, or safety rule beyond the existing identity boundary.

## Architectural choice

Use the existing **adapter report → core-derived session delta → registry fold
→ snapshot/event projection → surface** path, adding one schema-owned
mutation. The model is state owned by the session registry, not a Pi transcript
Observation or a web-only cache. This makes reconnect snapshots accurate and
keeps the core adapter-neutral.

### Options considered

1. **Durable `SessionModelChanged` mutation (chosen).** Registration and
   snapshots have `model`; equal-generation reports derive an identity-checked
   model delta. It costs a small contract regeneration but gives replay,
   snapshot, CLI, and cockpit one source of truth.
2. **Treat model as a `SessionRelabeled` field.** This avoids a new mutation
   but conflates orientation labels (`project`/`cwd`/`name`) with a mutable
   runtime fact, makes `from` validation impossible, and obscures the
   adapter's model-switch semantics.
3. **Emit a Pi `model_change` Observation and fold it only into the cockpit.**
   Rejected by the operator. It is cheaper initially but does not materialize
   in `SessionSnapshot`, duplicates a contract view in the browser, and loses
   state after stream gaps.

The chosen design is the least special case: it reuses the current report and
replay machinery while adding the mutation demanded by a real mutable runtime
fact.

## Implementation Units

### Unit 1: Session-model wire contract and generated bindings

**Files:** `contracts/proto/patchbay/sessions.proto`,
`contracts/proto/patchbay/adapter_control.proto`,
`contracts/rust/src/gen/patchbay/patchbay.rs` (generated),
`contracts/ts/src/gen/patchbay/sessions_pb.ts` (generated),
`contracts/ts/src/gen/patchbay/adapter_control_pb.ts` (generated),
`contracts/vectors/session-model-change-preserves-identity.json`,
`docs/VERIFICATION.md` (generated vector traceability), `docs/PROTOCOL.md`

Add the wire fields and oneof arm. Field tags are additive and never reuse an
existing number.

```proto
// contracts/proto/patchbay/sessions.proto
message Session {
  // existing fields 1..13
  string model = 14; // current adapter-reported opaque provider/model id
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
  }
}

message SessionRegistered {
  // existing fields 1..9, including spawn_origin
  string model = 10;
}

message SessionGenerationBumped {
  // existing fields 1..9
  string model = 10;
}

message SessionModelChanged {
  AdapterId adapter_id = 1;
  string deployment_scope = 2;
  RuntimeSessionId runtime_session_id = 3;
  Generation session_generation = 4;
  string from = 5;
  string to = 6;
}

// contracts/proto/patchbay/adapter_control.proto
message SessionReport {
  // existing fields 1..10
  string model = 11;
}
```

**Implementation notes:**
- `model` is not added to `SessionRelabeled`: labels remain label metadata and
  a runtime model switch must be observable as its own mutation.
- From `contracts/`, run `buf generate` for TypeScript, then restore the
  Buf-produced Rust output and let the checked-in prost build own it:
  `git checkout -- contracts/rust/src/gen` then
  `cargo build -p patchbay-contracts`. Finally run
  `cd contracts/ts && npm run build` and `npm run check:vectors`. Do not
  hand-edit generated code.
- Roll `docs/PROTOCOL.md` forward in the Sessions section and extension seams
  registry: current model is mutable, adapter-reported non-identity metadata;
  richer model descriptors/history remain reserved.
- The draft vector names `LabelsCannotOverrideIdentity`, constrains
  `patchbay.SessionModelChanged.{adapter_id,deployment_scope,runtime_session_id,session_generation,from,to}`
  and `patchbay.Session.model`, and expects a snapshot record with the new
  model and the same identity tuple.

**Acceptance criteria:**
- [ ] Generated Rust and TS bindings expose every added field and the
  `model_changed` oneof case.
- [ ] `SessionReport` can carry the registration value from an adapter.
- [ ] The vector checker passes and refreshes `docs/VERIFICATION.md`.
- [ ] Proto lint/build succeeds; no generated source is manually edited.

**Story:** `feature-session-model-field-proto-contract`

### Unit 2: Core session report, mutation fold, and snapshot materialization

**Files:** `core/src/session/ingest.rs`, `core/src/session/events.rs`,
`core/src/session/registry.rs`, `core/src/session/mod.rs`,
`server/src/adapter_service.rs`, `server/src/state.rs`,
`core/tests/sessions_ingest.rs`, `core/tests/sessions_registry.rs`,
`server/src/adapter_service/tests.rs`

The highest-risk unit is the report/replay fold: it must add a durable model
change without weakening the session identity and LSN guards. Design this first.

```rust
// core/src/session/ingest.rs
pub struct SessionReport {
    // existing identity, axes, and labels
    pub model: String,
}

pub enum IngestResult {
    // existing variants
    ModelChanged { event_id: EventId, from: String, to: String },
}

fn model_changed(current: &SessionRecord, report: &SessionReport) -> bool {
    current.model != report.model
}

// equal-generation report order:
// connectivity -> activity -> model -> relabel
// append_and_warm(... events::model_changed(... SessionModelChanged {
//     adapter_id, deployment_scope, runtime_session_id, session_generation,
//     from: current.model.clone(), to: report.model.clone(),
// }))
```

```rust
// core/src/session/registry.rs
pub struct SessionRecord {
    pub identity: SessionIdentity,
    pub state: SessionState,
    pub project: String,
    pub cwd: String,
    pub name: String,
    pub model: String,
    // existing LSN/tombstone fields
}

fn observe_model_changed(
    &mut self,
    mutation: &SessionModelChanged,
    event_lsn: u64,
) -> Result<(), SessionError> {
    let identity = mutation_identity(/* full 4-tuple */, "model change", event_lsn)?;
    if self.is_stale_replay(&identity, event_lsn)? { return Ok(()); }
    let record = self.live_record_mut(&identity, "model change", event_lsn)?;
    if record.last_authoritative_lsn.is_some_and(|last| event_lsn <= last) {
        return Ok(());
    }
    if record.model != mutation.from {
        return Err(SessionError::CorruptLog(/* expected prior model */));
    }
    record.model.clone_from(&mutation.to);
    record.last_authoritative_lsn = Some(event_lsn);
    Ok(())
}
```

`events.rs` gains `model_changed(...)`; `SessionRegistry::observe` dispatches
its generated `Mutation::ModelChanged` arm; registration and generation-bump
construction/observation seed `SessionRecord.model`. `server/src/adapter_service.rs`
copies `report.model` into the core `SessionReport`; `server/src/state.rs`
projects `record.model` into `Session { model, .. }`.

**Implementation notes:**
- Model change is included in the existing equal-generation `change_count` and
  multi-delta retry path. After every committed prefix the hot registry is
  warmed; a retry derives only remaining axes/model/labels.
- Existing full identity validation, stale-generation guard, and immutable
  replay behavior apply unchanged. An event against a tombstoned generation is
  not allowed to mutate model state.
- Update all direct `SessionRegistered`, `SessionGenerationBumped`, and
  adapter-report fixtures with explicit model values rather than relying on an
  accidental default.

**Acceptance criteria:**
- [ ] First registration, generation bump, model-only delta, combined delta,
  replay, stale target, and partial-append retry tests cover model state.
- [ ] Snapshot materialization returns the current model at its authoritative
  LSN.
- [ ] A mismatched `from` or wrong identity fails as corrupt log without
  silently overwriting the projection.

**Story:** `feature-session-model-field-core-registry`

### Unit 3: Pi model reporting

**Files:** `pi-adapter/src/pi_session.ts`,
`pi-adapter/src/session_registry.ts`, `pi-adapter/src/core_client.ts`,
`pi-adapter/src/main.ts`, `pi-adapter/tests/pi_session.test.ts`,
`pi-adapter/tests/delivery.test.ts`, `pi-adapter/tests/e2e.test.ts`

```ts
// pi-adapter/src/pi_session.ts
export type SessionModelChangeListener = (model: string) => void;

onModelChange(listener: SessionModelChangeListener): () => void;

// Only the currently bound AgentSession may emit this callback.
if (event.type === "entry_appended" && event.entry.type === "model_change") {
  this.#emitModelChange(`${event.entry.provider}/${event.entry.modelId}`);
}
```

`PiSession.getState()` remains the registration source (`session.model`);
`AdapterProcess` normalizes it as
`state.model ? `${state.model.provider}/${state.model.id}` : ""`.
`SessionRegistry.register` owns both transcript and model-change unsubscribe
handles so disposal remains complete. Its model observer calls a new private
`AdapterProcess.#queueSessionReport(entry, activity, connectivity)` path. That
path shares a per-runtime-session promise tail with every activity report and
reads the current model when the queued report executes:

```ts
interface SessionIdentity {
  // existing fields
  model: string;
}

async reportSession(
  identity: SessionIdentity,
  activity: SessionActivityState,
  connectivity = SessionConnectivityState.LIVE,
): Promise<EventId | undefined> {
  return this.#client.ingestObservation(create(ObservationRequestSchema, {
    authorityDomainId: this.#authorityDomainId(),
    observation: { case: "sessionReport", value: create(SessionReportSchema, {
      // existing fields
      model: identity.model,
    }) },
  }));
}
```

**Implementation notes:**
- `model_change` is a persisted Pi `SessionEntry`, not an `AgentSessionEvent`
  variant. `entry_appended` is therefore the supported live subscription
  seam; `session.model` supplies the initial value.
- Report only active bindings, preserving PiSession's existing generation
  isolation. The queue prevents a delayed activity report from re-reporting a
  stale model after a later `model_change`.
- Do not extend adapter capabilities or introduce Pi-specific values into core
  vocabulary.

**Acceptance criteria:**
- [ ] Registration report contains normalized model or `""` for unknown.
- [ ] `model_change` entry causes exactly one ordered report with its new
  normalized model.
- [ ] Stale/replaced bindings produce no model report; registry disposal
  unsubscribes both observer types.
- [ ] Adapter build/test and the real adapter/core smoke fixture demonstrate
  the model in a loaded snapshot.

**Story:** `feature-session-model-field-pi-adapter`

### Unit 4: Cockpit and CLI presentation

**Files:** `web-cockpit/src/domain/model.ts`,
`web-cockpit/src/ui/session-list.ts`, `web-cockpit/src/ui/session-detail.ts`,
`web-cockpit/src/ui/shell.css`, `web-cockpit/tests/model.test.ts`,
`web-cockpit/tests/shell.test.ts`, `cli/src/commands/session-health.ts`,
`cli/tests/output-diagnostics.test.ts`

```ts
// web-cockpit/src/domain/model.ts
export interface SessionView {
  // existing identity, labels, axes, liveness fields
  model?: string;
}

// registration / generation bump / snapshot:
model: value.model || undefined,

// SessionModelChanged projection:
case "modelChanged":
  // resolve full identity, ignore tombstones, preserve all other fields
  model.sessions.set(key, { ...current, model: value.to || undefined, lastLsn: lsn });
  return;
```

```ts
// cli/src/commands/session-health.ts
export function sessionHealthView(session: Session) {
  return {
    // existing script-facing fields
    model: session.model || null,
  };
}
// headers: IDENTITY, CONNECTIVITY, ACTIVITY, MODEL, NAME
```

`session-list.ts` renders `session.model ?? "Model unknown"` as a concise
metadata line and includes the real model string in `searchableSession`.
`session-detail.ts` repeats the same value beside the stable identity/status in
the header. CSS only composes the existing row/header metadata classes; it
adds no state color or semantic status.

**Acceptance criteria:**
- [ ] Model survives snapshot replacement and registration/generation/model
  delta folds without changing identity, axes, or liveness behavior.
- [ ] Session row, searchable text, and detail header show a populated model;
  absence visibly says `Model unknown`.
- [ ] `session-health --json` emits `model: string | null`; table output gains
  a MODEL column while retaining its existing canonical state/identity fields.
- [ ] Cockpit and CLI focused tests pass for populated and unknown values.

**Story:** `feature-session-model-field-surfaces`

## Implementation Order

1. `feature-session-model-field-proto-contract` — define/additively regenerate
   contract fields, vector, and protocol classification.
2. `feature-session-model-field-core-registry` — make report ingestion,
   replay, and snapshots durably correct.
3. In parallel after core: `feature-session-model-field-pi-adapter` reports
   the source fact; `feature-session-model-field-surfaces` consumes the
   snapshot/event fact.

The dependency graph was cycle-checked with
`.work/bin/work-view --blocking` for all four stories before assigning sibling
dependencies: `proto-contract → core-registry → {pi-adapter, surfaces}`.

## Simplification

- One `SessionReport` remains the adapter-to-core update seam; no second
  model-observation channel or browser-only cache is introduced.
- `SessionRelabeled` stays limited to orientation labels. The explicit model
  mutation removes the tempting but misleading metadata overload.
- No cleanup/refactor child is justified: all touched paths extend the existing
  schema-owned session-state pattern.

## Testing

- **Core interface/replay tests:** assert registration, replacement, model-only
  update, combined report ordering, stale identity rejection, and
  partial-append retry/rebuild behavior. These protect the durable source of
  truth and are the primary executable evidence.
- **Generated contract/vector checks:** run `buf generate`, prost `cargo build
  -p patchbay-contracts`, TypeScript build, and `check:vectors`. The vector
  constrains wire shape and identity intent; it is draft, not a substitute for
  the core tests.
- **Adapter tests:** use a real/fake `entry_appended` `model_change` entry to
  prove normalized reporting and stale-binding inertness; retain existing
  registration/delivery coverage.
- **Surface tests:** fold snapshot/event examples into the cockpit model and
  assert row/header text; assert CLI JSON/table output includes model and
  represents unavailable as null/unknown.
- **Test removal:** none. Existing session and state-axis cases retain their
  purpose and gain explicit `model` fixture values as required by regenerated
  Rust structs.

## Implementation notes
- Execution capability: inline single-owner implementation across the contract, durable registry, Pi adapter, and existing cockpit/CLI projections.
- Review weight: standard (default); caller explicitly requested the feature remain at review for a separate orchestrator.
- Files changed: contract protos/generated bindings/vector/docs; core session ingest/event/registry plus snapshot materialization; Pi adapter report and model-entry subscription; cockpit model/list/detail; CLI session-health and focused tests.
- Tests added/removed: durable model change/replay/mismatch and combined-retry coverage; adapter model observer/disposal coverage; cockpit mutation/unknown rendering; CLI JSON/table model rendering. No tests removed.
- Simplification: a single adapter report → registry fold → snapshot/event projection path; no Pi-specific core vocabulary or browser-only state cache.
- Discrepancies from design: `buf lint` remains blocked by pre-existing RPC request/response naming violations across adapter/admin/control protos. The live stack was left untouched.
- Adjacent issues parked: none.
- Integrated verification: `cargo build && cargo test` passed; `contracts/ts`: `npm run build`, `npm run check:vectors`, and `npm run check:presentation` passed; `web-cockpit`: `npm test` passed (45); `cli`: `npm test` passed (17); `pi-adapter`: `npm test` passed (10, including the core smoke/reconnect e2e).

## Risks

- **Out-of-order adapter reports could roll a model backward.** Existing
  activity reports and the new model callback can race. Mitigation: one
  per-session report tail covers both, and model is read at queued-report
  execution time; core's `from` validation fails corrupted durable order.
- **Pi emits a persisted entry, not a bespoke live model event.** Mitigation:
  subscribe to the documented `entry_appended`/`ModelChangeEntry` path and use
  `session.model` only for registration; add a focused adapter test.
- **Contract regeneration has two Rust generators.** Mitigation: use the
  repository's established `buf generate` → restore Rust gen → prost
  `cargo build` sequence and never hand-edit generated artifacts.
- **Unavailable model must not be invented.** An empty wire value remains
  unknown in every surface; no provider/model fallback or Pi-specific label is
  fabricated.
