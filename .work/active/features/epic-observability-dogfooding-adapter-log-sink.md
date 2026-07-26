---
id: epic-observability-dogfooding-adapter-log-sink
kind: feature
stage: done
tags: [observability, dogfooding]
parent: epic-observability-dogfooding
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-26
---

# Adapter durable diagnostics log sink

## Brief

pi-adapter currently keeps all diagnostics process-local: `TranscriptEventLog`
is an in-memory partial-snapshot log, `#observationError` and delivery/attach
failures die with the process, and nothing is configurable. The only way to
inspect the adapter during live testing is whatever shell redirect the
operator happened to launch it with.

This feature gives pi-adapter a durable, structured diagnostics log: an
env-configured file sink (`PATCHBAY_ADAPTER_LOG`, defaulting to an XDG state
dir such as `~/.local/state/patchbay/adapter.log`) capturing attach,
registration, delivery, observation, and lifecycle events with their error
detail. It is the fastest inspection unblock in the epic and is deliberately
adapter-local — forwarding diagnostics to core is
`epic-observability-dogfooding-cockpit-diagnostics`.

It does NOT cover: log shipping, rotation policy beyond a sane local default,
or any core-side change.

## Epic context

- Parent epic: `epic-observability-dogfooding`
- Position in epic: independent capability — no shared types with the other
  children; parallelizable from day one. Priority 1 in the epic's seed order
  (fastest operator unblock).

## Simplification opportunity

- `TranscriptEventLog` may be subsumed or repositioned: decide whether it
  survives as an in-memory ring feeding the durable sink, or is deleted in
  favor of the sink plus core-side transcript durability (transcript events
  already reach the core's durable log via `ingestTranscript`).
- Retain: Pi's own persisted session remains the durable transcript record;
  the sink is for adapter diagnostics, not a second transcript store.

## Foundation references

- `docs/SPEC.md` — post-v0.1.0 observability scope (adapter-process durable diagnostics)
- `docs/ADAPTER-PI.md` — adapter behavior and snapshot-tier context
- `docs/SECURITY.md` — redaction discipline applies to anything the sink writes

## Architectural choice

Use one adapter-owned diagnostics port with a JSON Lines file implementation,
created at the process composition root and injected into `AdapterProcess` and
`PatchbayCoreClient`. Calls to `record` only sanitize and enqueue a bounded
record; one serialized asynchronous writer performs filesystem I/O. Startup and
shutdown may await open/flush/close, but the attach, delivery, observation, and
session control paths never await a log write.

Three approaches were considered:

1. **Typed JSON Lines sink (chosen).** A narrow `AdapterDiagnostics` interface
   keeps instrumentation independent of the filesystem, while one record per
   line is append-friendly, grep-friendly, and machine-readable for later
   forwarding. It costs a small queue and explicit lifecycle management, but
   gives the redaction boundary and failure isolation this control plane needs.
2. **Plain text through stdout/stderr.** This has the fewest new lines of code,
   but remains dependent on shell redirection, makes field-safe redaction
   inconsistent, and is awkward to consume programmatically. It does not solve
   the durable configurable-sink brief.
3. **Turn `TranscriptEventLog` into the file log.** This reuses an existing
   class, but mixes transcript content (including prompts, tool arguments, and
   results) with operational diagnostics, creates a second transcript store,
   and violates the security and single-source-of-truth boundaries. It is
   rejected.

The trickiest unit is the sink itself: it must preserve record order, bound
memory under a slow disk, rotate without becoming a process dependency, redact
before serialization, and make every write failure non-throwing. It is designed
first; all instrumentation depends only on its small port.

## Design decisions

- **Log format:** newline-delimited JSON, one allowlisted structured record per
  line — machine readability and reliable field redaction outweigh the small
  verbosity over plain text.
- **Location:** `PATCHBAY_ADAPTER_LOG` overrides the path; otherwise use
  `$XDG_STATE_HOME/patchbay/adapter.log`, falling back to
  `~/.local/state/patchbay/adapter.log` when `XDG_STATE_HOME` is absent or not
  absolute — this applies the parent epic's locked XDG decision without making
  the working directory persistent state.
- **Rotation:** on open, rotate a file at or above 10 MiB to `adapter.log.1`
  (replacing the prior single backup), then append; otherwise append in place.
  Rotation failure falls back to append, and there is no runtime/time-based
  rotation, compression, retention service, or shipping in this feature.
- **Write behavior:** `record()` is synchronous only for bounded sanitization and
  enqueueing, returns `void`, and performs no filesystem I/O. A serialized async
  queue preserves order, is capped at 1,024 pending records, and emits one
  `log.records_dropped` warning with the dropped count when capacity returns.
  This avoids blocking the adapter control loop and avoids unbounded memory.
- **Failure behavior:** open/write/flush/close/rotation failures never reject an
  adapter operation or terminate the process. The sink emits at most one
  code-only warning to stderr and degrades to dropping diagnostics; callers also
  guard the port so a broken injected implementation cannot escape into the
  control loop.
- **Lifecycle:** the environment composition root opens the sink before
  `AdapterProcess.run`; `AdapterProcess.dispose()` is idempotent and terminal,
  records process/session shutdown, disposes the registry, then awaits
  `flush()`/`close()`. Existing `SIGINT`/`SIGTERM` handlers and the top-level
  `finally` provide graceful process-exit flush. `SIGKILL` and host power loss
  remain best-effort and do not gain `fsync` semantics.
- **Captured events:** the registry-derived event vocabulary covers adapter
  start/stop; every initial attach and token-refresh reattach attempt/result;
  `registerSession` attempt/result; session activity/model reports, generation
  replacement, and disposal; delivery subscription failure/retry plus
  received/acknowledged/running/completed/rejected/failed; and observation
  failures both when `#trackObservation` catches them and when
  `#observationError` is consumed by `flushObservations`.
- **Redaction boundary:** records may carry adapter id/generation, runtime
  session identity, command id, generated enum names, outcome/failure code,
  counts, adapter-authored safe reasons, and code-only normalized errors. They
  never accept or serialize an Operation payload, transcript event, prompt
  body, tool arguments/result, attachment evidence/descriptor, session cookie,
  CSRF/access token, password/bootstrap secret, encryption key, sensitive
  attachment, `sessionOptions`, error stack, or error cause. Configured secrets
  and credential-shaped assignments are replaced before serialization as a
  defense in depth.
- **`TranscriptEventLog`:** delete it rather than feeding transcript content to
  diagnostics. `PiSession` retains only current-generation event-id
  deduplication; `snapshotTranscript()` projects Pi's persisted entries
  directly for core ingestion. Pi remains the durable transcript record and the
  core remains the durable Observation record.
- **Dispatch:** direct-read only; the bounded adapter module and existing tests
  left no distinct discovery unknown that would justify exploratory fanout.
- **Extension pressure:** this is committed post-v0.1.0 adapter-local
  diagnostics. Log shipping, long retention/SIEM, metrics, and core/cockpit
  forwarding remain reserved; a diagnostics-owned transcript store remains
  rejected. No protocol enum or core authority semantics change.

## UI surface

No UI surface: Phase 4.6 is skipped; this feature is an adapter-process backend
capability and the later cockpit-diagnostics feature owns presentation.

## Implementation Units

### Unit 1: Typed, redacting, failure-isolated JSON Lines sink

**File:** `pi-adapter/src/adapter_diagnostics.ts` (new)

```ts
import {
  FailureCode,
  OperationKind,
  SessionActivityState,
  SessionConnectivityState,
} from "@patchbay/contracts";

export const ADAPTER_DIAGNOSTIC_EVENTS = [
  "adapter.starting",
  "adapter.started",
  "adapter.stopping",
  "adapter.stopped",
  "adapter.attach.started",
  "adapter.attach.succeeded",
  "adapter.attach.failed",
  "session.register.started",
  "session.register.succeeded",
  "session.register.failed",
  "session.activity.reported",
  "session.model.changed",
  "session.generation.changed",
  "session.dispose.started",
  "session.dispose.succeeded",
  "session.dispose.failed",
  "delivery.subscription.failed",
  "delivery.subscription.retrying",
  "delivery.received",
  "delivery.acknowledged",
  "delivery.running",
  "delivery.completed",
  "delivery.rejected",
  "delivery.failed",
  "observation.failed",
  "observation.flush_failed",
  "log.records_dropped",
] as const;

export type AdapterDiagnosticEvent =
  (typeof ADAPTER_DIAGNOSTIC_EVENTS)[number];
export type AdapterDiagnosticLevel = "info" | "warn" | "error";

export interface AdapterDiagnosticSessionRef {
  runtimeSessionId: string;
  deploymentScope: string;
  generation: number;
}

export interface AdapterDiagnosticError {
  name: string;
  code?: string;
}

export interface AdapterDiagnosticInput {
  event: AdapterDiagnosticEvent;
  level: AdapterDiagnosticLevel;
  session?: AdapterDiagnosticSessionRef;
  commandId?: string;
  operationKind?: OperationKind;
  failureCode?: FailureCode;
  outcome?: string;
  observationKind?: "transcript" | "session-report";
  sessionActivity?: SessionActivityState;
  sessionConnectivity?: SessionConnectivityState;
  fromGeneration?: number;
  toGeneration?: number;
  reason?: string;
  error?: AdapterDiagnosticError;
  count?: number;
}

export interface AdapterDiagnostics {
  record(input: AdapterDiagnosticInput): void;
  flush(): Promise<void>;
  close(): Promise<void>;
}

export interface OpenAdapterDiagnosticsOptions {
  path: string;
  adapterId: string;
  adapterGeneration: number;
  secrets?: readonly string[];
  now?: () => Date;
  rotateAtBytes?: number;
  maxPendingRecords?: number;
  reportFailure?: (code: string) => void;
}

export function resolveAdapterLogPath(
  env?: NodeJS.ProcessEnv,
  homeDirectory?: string,
): string;

export function diagnosticError(error: unknown): AdapterDiagnosticError;

export async function openAdapterDiagnostics(
  options: OpenAdapterDiagnosticsOptions,
): Promise<AdapterDiagnostics>;

export const NOOP_ADAPTER_DIAGNOSTICS: AdapterDiagnostics;
```

**Implementation Notes:**

- Serialize a stable wire shape with `ts`, `level`, `event`, `adapter_id`, and
  `adapter_generation`, plus only the optional allowlisted snake-case fields
  represented above. Generated enum reverse names supply operation/failure
  vocabulary rather than a hand-copied mapping.
- Create parent directories, open with append semantics, and set the log file to
  mode `0600`. Perform the one-backup size check before opening the append
  handle. An invalid relative `XDG_STATE_HOME` is ignored per the XDG contract;
  an explicit relative `PATCHBAY_ADAPTER_LOG` resolves from the process cwd.
- One promise tail owns writes. `record` catches serialization/enqueue errors;
  each queued write catches its own error, reports one safe code, and leaves the
  tail resolved. `flush` and `close` are idempotent and never reject.
- `diagnosticError` emits only a bounded error name and string/number `code`; it
  intentionally omits arbitrary `.message`, `.stack`, `.cause`, and thrown
  object fields. Adapter-authored non-content reasons use the separately
  sanitized `reason` field.

**Acceptance Criteria:**

- [ ] Every successful call writes exactly one parseable JSON object followed by
  `\n`, in call order, with the required adapter context and no `undefined` or
  raw enum-number ambiguity.
- [ ] Default and override paths resolve exactly as decided, files are appended
  below 10 MiB, and startup rotates at/above 10 MiB to one `.1` backup.
- [ ] A pending queue above 1,024 records drops without blocking and later emits
  a counted `log.records_dropped` record.
- [ ] Open, rotation, write, flush, and close failures are non-throwing and emit
  no more than one code-only stderr warning.
- [ ] Redaction tests prove configured attachment evidence and
  bearer/token/password-shaped strings are removed, arbitrary errors cannot
  contribute message/stack/cause fields, and the typed input has no payload,
  transcript, prompt, tool-data, or attachment-material field.

---

### Unit 2: Composition, attach instrumentation, and orderly sink ownership

**Files:**
- `pi-adapter/src/main.ts`
- `pi-adapter/src/core_client.ts`

```ts
// main.ts
export interface AdapterProcessOptions {
  // existing fields unchanged
  diagnostics?: AdapterDiagnostics;
}

// AdapterProcess additions
readonly #diagnostics: AdapterDiagnostics;
#disposed = false;

#record(input: AdapterDiagnosticInput): void;
#sessionRef(entry: RuntimeSessionEntry): AdapterDiagnosticSessionRef;

// core_client.ts
export class PatchbayCoreClient {
  constructor(options: CoreClientOptions, diagnostics?: AdapterDiagnostics);
}
```

**Implementation Notes:**

- `runFromEnvironment` resolves `PATCHBAY_ADAPTER_LOG`, opens the sink with
  `attachmentEvidence` in its secret-redaction set, passes it to
  `AdapterProcess`, and retains the existing top-level `try/finally` disposal.
  Tests constructing `AdapterProcess` directly receive the no-op sink unless
  they inject one.
- Move attach attempt/success/failure instrumentation into
  `PatchbayCoreClient.attach` so both initial attach and automatic token-refresh
  reattach use the same path. Do not log the core address, attachment evidence,
  attachment token/header, capability descriptor, or full registration.
- `AdapterProcess.start`, `registerSession`, and `dispose` record lifecycle
  boundaries with identity only after it is known. Registration failure records
  a code-only normalized error before preserving the existing cleanup/rethrow.
- `dispose` snapshots registered session refs, records disposal start, awaits
  registry disposal, records success/failure, then always closes diagnostics in
  `finally`. It is safe to call twice; `start` after terminal disposal rejects.
- `#record` catches a misbehaving injected sink. Awaited `flush`/`close` calls are
  similarly wrapped so diagnostics cannot change adapter success/failure.

**Acceptance Criteria:**

- [ ] Initial attach and automatic reattach each produce ordered started plus
  succeeded/failed records without authentication material.
- [ ] Adapter and session start/register/dispose paths produce the decided
  lifecycle records, and a failed registration preserves its original thrown
  error after logging.
- [ ] Normal return, `SIGINT`, `SIGTERM`, and a thrown run-loop error all reach
  registry disposal and sink close through the existing top-level `finally`.
- [ ] A sink whose `record`, `flush`, or `close` implementation throws/rejects
  cannot fail an attach, registration, delivery, or disposal operation.

---

### Unit 3: Delivery and observation event coverage

**File:** `pi-adapter/src/main.ts`

```ts
interface ObservationDiagnosticContext {
  session: AdapterDiagnosticSessionRef;
  observationKind: "transcript" | "session-report";
}

#trackObservation(
  promise: Promise<void>,
  context: ObservationDiagnosticContext,
): void;
```

**Implementation Notes:**

- Record delivery receipt before acknowledgement; acknowledgement, running, and
  completed only after the corresponding core call succeeds. Target validation
  and `UnsupportedCommandError` use `delivery.rejected` with the generated
  failure code; other execution failures use `delivery.failed`.
- Include only command id, generated `OperationKind`/`FailureCode`, target
  session identity, outcome, and adapter-authored validation reason. Never pass
  `operation.payload` or translated result values to diagnostics.
- Record retryable subscription errors before the existing 100 ms retry and
  record non-retryable/clean-unexpected termination before rethrowing. Logging
  does not alter retry policy.
- Give every transcript/session-report promise a safe context. When
  `#trackObservation` catches a rejection, record `observation.failed` and
  preserve the first error in `#observationError`; when `flushObservations`
  consumes that stored error, record `observation.flush_failed` before the
  existing throw.
- Session reports record activity/model-change events using generated state
  values but exclude model id, project, cwd, name, and transcript content.
  Session replacement records old/new generation through safe numeric fields or
  a fixed reason, not through transcript replay.

**Acceptance Criteria:**

- [ ] One successful instruction can be followed in the log through received,
  acknowledged, running, and completed using the same command/session identity.
- [ ] Missing/stale targets and unsupported kinds are distinguishable from
  execution failure by event and canonical failure code.
- [ ] Retryable subscription loss is visible without changing reconnect
  behavior, and a non-retryable loss remains fatal exactly as before.
- [ ] Both the original asynchronous observation rejection and the later
  `#observationError` flush boundary are visible while `flushObservations`
  preserves its current error semantics.
- [ ] Delivery/transcript payloads, prompt bodies, tool data, result values, and
  model/cwd/project metadata never enter a diagnostic input.

---

### Unit 4: Remove the process-local transcript store

**Files:**
- `pi-adapter/src/pi_session.ts`
- `pi-adapter/src/transcript_event_log.ts` (delete)
- `pi-adapter/tests/pi_session.test.ts`

```ts
// PiSession replacement state
readonly #seenTranscriptEventIds = new Set<string>();

// Existing public method retained with direct projection semantics.
snapshotTranscript(): readonly TranscriptEvent[];

// Remove the unused public method.
// transcriptEvents(): readonly TranscriptEvent[];
```

**Implementation Notes:**

- `snapshotTranscript` projects `sessionManager.getEntries()` directly, seeds
  the current generation's seen-id set, and returns the projected events for
  core ingestion. `#append` deduplicates by stable event id before notifying
  listeners; it retains no event bodies.
- Clear the seen-id set when a replacement Pi session binds a new generation.
  Old-generation listeners are already invalidated, and event ids include the
  generation, so no cross-generation replay is needed locally.
- Delete the isolated `TranscriptEventLog` test. Strengthen the existing real
  Pi-session test to assert duplicate Pi hooks emit one stable transcript event,
  preserving the behavioral value rather than testing a removed container.

**Acceptance Criteria:**

- [ ] Persisted Pi entries still replay as the adapter's partial snapshot and
  live duplicate hooks still emit each stable event id at most once.
- [ ] No process-local collection retains prompt/transcript event bodies after
  listener delivery.
- [ ] Session replacement clears current-generation dedup state while stale old
  bindings remain inert.
- [ ] `transcript_event_log.ts`, its imports, its unused `transcriptEvents()`
  accessor, and its container-only test are removed.

---

### Unit 5: Contract-level tests and foundation wording

**Files:**
- `pi-adapter/tests/adapter_diagnostics.test.ts` (new)
- `pi-adapter/tests/delivery.test.ts`
- `pi-adapter/tests/e2e.test.ts`
- `docs/ADAPTER-PI.md`

**Implementation Notes:**

- Use temporary directories and injected clock/failure reporter for sink tests;
  do not write tests to the operator's real XDG state directory.
- Add one fake-sink `AdapterProcess` test for failure isolation and one existing
  e2e-path assertion for representative attach/register/delivery/generation/
  shutdown records plus forbidden sentinel absence. Do not assert every event
  name at every call site.
- Replace `TranscriptEventLog` wording in the Pi adapter snapshot-tier and e2e
  comments with “Pi persisted entries projected as transcript events.” This is
  a rolling-foundation correction, not a new snapshot guarantee.

**Acceptance Criteria:**

- [ ] `npm test` in `pi-adapter/` passes on Node `>=22.19.0` and exercises JSONL
  ordering, append/rotation, bounded drop reporting, redaction, failure
  isolation, graceful close, and representative instrumentation.
- [ ] Existing delivery ordering, reconnect, generation, transcript projection,
  and partial-snapshot tests remain green after the local log deletion.
- [ ] `docs/ADAPTER-PI.md` still describes the `partial` snapshot tier accurately
  without naming a deleted implementation class.

## Implementation Order

1. Implement and test Unit 1's diagnostics port/file sink; it is the redaction
   and failure-isolation boundary all other units consume.
2. Wire sink ownership and attach/process/session lifecycle in Unit 2.
3. Add delivery/observation/session-report instrumentation in Unit 3.
4. Delete `TranscriptEventLog` and preserve only event-id deduplication in Unit
   4.
5. Complete the representative integration/e2e assertions and roll
   `docs/ADAPTER-PI.md` wording forward in Unit 5, then run `npm test` from
   `pi-adapter/`.

No child stories are created: the sink, instrumentation, transcript cleanup, and
verification are tightly coupled in one bounded TypeScript package and form a
single-stride implementation/review bundle.

## Simplification

- Delete `pi-adapter/src/transcript_event_log.ts`, the unused
  `PiSession.transcriptEvents()` API, and the container-only unit test.
- Replace retained transcript bodies with current-generation stable-id dedup
  state; Pi persistence plus core Observation durability remain authoritative.
- Keep one diagnostics port and one file implementation. Do not add a general
  logging framework, transport, schema package, background worker, metrics
  abstraction, or core-side DTO.
- Reuse generated enum reverse names instead of copying OperationKind,
  FailureCode, or session-state registries into the logger.
- No independent cleanup/refactor story is warranted; broader log shipping and
  cockpit forwarding remain in their already-scoped feature.

## Testing

- **Sink interface tests:** protect the stable JSONL/redaction/path/rotation and
  non-throwing lifecycle contract using temp files, a fixed clock, and a forced
  writer/open failure. These are the highest-value isolated tests because the
  sink owns novel state and filesystem behavior.
- **Adapter regression test:** inject a collecting or deliberately broken sink
  into the existing `AdapterProcess` tests. Protect that logging cannot alter
  attach/register/observation semantics and that the first observation error is
  still rethrown only at `flushObservations`.
- **E2e representative trace:** extend the existing real core→adapter test to
  assert one command's lifecycle and one generation change are present after
  close, and that known attachment secret and prompt sentinels are absent. This
  protects call-site coverage and shutdown flush without duplicating every unit
  test.
- **Transcript regression:** preserve the existing real-session and reconnect
  tests, changing only the implementation-specific `TranscriptEventLog`
  assertions/wording. Assert stable-id dedup behavior, not container internals.
- **No test-per-event matrix:** event-name exhaustiveness is typechecked from
  `ADAPTER_DIAGNOSTIC_EVENTS`; exhaustive call-site tests would be brittle and
  add little confidence.

## Risks

- **Riskiest assumption — queued writes are durable enough for diagnostics.** A
  graceful signal/error path flushes, but `SIGKILL`, kernel crash, or power loss
  may lose queued tail records. Adding synchronous writes or per-line `fsync`
  would endanger the control loop; the fallback is a later batching/fsync policy
  only if dogfooding demonstrates unacceptable loss.
- **Sensitive text can hide inside arbitrary errors.** The design does not log
  arbitrary messages/stacks/causes and structurally excludes all content-bearing
  objects; configured-secret and credential-pattern replacement is defense in
  depth. The fallback is code-only records at additional call sites, never
  widening raw content logging.
- **A slow or failed disk can create gaps.** The bounded queue deliberately drops
  rather than applying backpressure, and the counted warning makes the gap
  visible once writing recovers. Persistent failure remains visible only via the
  one safe stderr warning because the file cannot report its own outage.
- **Startup rotation assumes one writer per configured path.** Concurrent
  adapter processes sharing `adapter.log` can race rotation. The single-VM
  dogfooding topology uses one Pi adapter process; multiple processes must set
  distinct `PATCHBAY_ADAPTER_LOG` paths until a shared-log policy is explicitly
  scoped.
- **Where least certain:** exact useful error detail under the strict no-content
  boundary. The chosen code/name/failure/reason fields favor non-disclosure over
  verbose third-party messages; dogfooding can safely add specific allowlisted
  fields later without changing the record transport.

## Implementation summary

- Implemented `AdapterDiagnostics` and the bounded, serialized JSONL file sink in
  `pi-adapter/src/adapter_diagnostics.ts`, including XDG/override path resolution,
  startup rotation, mode `0600`, structural redaction, drop accounting, and
  non-throwing lifecycle failure handling.
- Injected diagnostics through `AdapterProcess` and `PatchbayCoreClient`, with
  attach, registration, lifecycle, delivery, subscription, observation, model,
  activity, generation, and shutdown instrumentation. Added environment-root sink
  ownership and graceful flush/close behavior.
- Removed `TranscriptEventLog`; `PiSession` now retains only current-generation
  event-id deduplication while projecting persisted entries directly. Updated the
  adapter foundation wording to “Pi persisted entries projected as transcript
  events.”
- Added sink contract tests, broken-sink lifecycle isolation coverage,
  representative e2e log assertions, and retained real-session deduplication
  coverage. No design deviations or implementation discoveries were required.
- Verification: `cd pi-adapter && npm run build` passed; all non-e2e adapter tests
  passed (`15/15`) via the equivalent compiled `node --test` invocation. The full
  `cd pi-adapter && npm test` command remains externally blocked by concurrent
  out-of-scope core/server edits: `cargo build -p patchbay-core-server` currently
  fails because `ControlService::query_diagnostics` and new `StorageError` arms are
  not yet implemented in `server/src/service.rs`. No production or test changes
  were made outside the owned paths to work around that blocker.
- Code/tests/docs landed in commit `0934b9d` (`implement: epic-observability-dogfooding-adapter-log-sink`).
- Nothing was parked for later.

## Review findings (standard pass 1, 2026-07-26 — independent reviewer: gpt-5.6-sol)

Verdict: blockers-found. Receiver-confirmed blockers (fix before `done`):

1. **Bearer-assignment redaction gap** — `#sanitize` handles `Bearer <token>`
   but not `bearer=<token>` (`adapter_diagnostics.ts`); reviewer reproduced an
   emitted record containing an unconfigured bearer secret. Fix: recognize
   `bearer\s*[:=]` shapes; test with a value absent from `secrets`.
2. **`diagnosticError` is not total** — reading arbitrary `.name`/`.code`/
   `.constructor.name` can throw (hostile getter) before the guarded
   diagnostics call, altering adapter failure behavior. Fix: make
   normalization total with a fixed code-only fallback; test throwing getters.
3. **Transcript dedup test tautological** — `pi_session.test.ts` compares two
   deterministic projections; never delivers duplicate live hooks. Fix:
   inject duplicate hook events with the same stable id, assert one listener
   event, including after generation reset.

Parked notes: maxPending+1 outstanding bound (fine); redaction test name
overstates coverage; feature-level `npm test` evidence to be refreshed at fix
time.

## Review resolution

Receiver-confirmed standard-pass blockers are resolved without changing the
adapter-forwarding composition that shares `adapter_diagnostics.ts`:

1. **Bearer-assignment redaction gap:** `#sanitize` now recognizes both
   `Bearer <token>` and case-insensitive `bearer` assignment forms with `:` or
   `=` (including surrounding whitespace). A temp-file sink test supplies an
   unconfigured bearer value alongside a different configured secret and proves
   the credential-shaped matcher removes it. Evidence: the test passed in both
   full `cd pi-adapter && npm test` runs.
2. **Non-total `diagnosticError`:** all `Error`/object name, code, and
   constructor inspection is inside one guarded normalization boundary; any
   throwing getter or hostile proxy returns the fixed safe
   `{name: "Error", code: "DIAGNOSTIC_ERROR"}` fallback. A real
   `AdapterProcess.registerSession` catch→log path now rejects with the exact
   hostile original error while recording only that fallback. Evidence: the
   hostile-registration test and both full suites passed.
3. **Tautological transcript dedup test:** the real `AgentSession` hook
   subscription is captured through the production `subscribe` path and fed
   duplicate live `entry_appended` hooks with one stable entry id. The listener
   receives one event in generation 1 and one event in generation 2 after
   replacement resets dedup state; each duplicate pair is asserted once.
   Evidence: the updated `pi_session.test.ts` passed in both full suites.

Shared-file compatibility: the forwarding tests remained green in both runs
(21/21 tests passed each time). No e2e timing flake occurred.
