---
id: feature-adapter-staleness-liveness
kind: feature
stage: implementing
tags: [security, protocol, fast-follower]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-23
updated: 2026-07-24
research_origin: null
---

# Feature: full-coverage adapter staleness (heartbeat / last-report-age, or long-poll delivery)

Surfaced by the epic-v0-1-0-implementation maximum review, pass 3 (convergence,
2026-07-23), as Important finding P3-I1 — parked, not a v0.1.0 blocker.
**Promoted 2026-07-24**: proven to bite in real use during the v0.1.0 live-test
arc (a turn stuck at `running` forever after an adapter restart mid-turn — the
P3-N1 running-rot instance), so it joins the pre-release fix wave.

## The gap

The epic pass-2 B3b fix marks an adapter's sessions `stale` on an abnormal
delivery-stream drop (operator decision Q2a: connection-liveness signal). The
mechanism is genuine and tested — but the v0.1.0 delivery model is a polling
fallback: the stream drains the durable tail and completes in milliseconds per
~100ms poll, and command execution happens after stream completion. So the
staleness signal only fires for deaths during an active stream drain. An
adapter that dies **between polls or mid-execution** (the majority of real
deaths) leaves its sessions presented as `live/working` until the adapter
restarts. Demonstrated by the pass-3 reviewer's probe test.

## Why parked

- The mechanism implements exactly what operator decision Q2a scoped.
- Commands are never lost (epic pass-2 B3a redelivery, verified).
- A replacement adapter process cannot compound the confusion (epic pass-2 B2
  fencing token, verified).
- The residual is presentational honesty in a single-operator deployment;
  natural recovery (restart the adapter) restores truth via re-attach + live
  reports.

## Fast-follower shape (two options, pick at design time)

1. **Heartbeat / last-report-age staleness:** the core tracks each adapter's
   last report time; a background sweep marks sessions stale after a threshold
   with no report. Covers all death modes; requires a core-side timer.
2. **Long-poll the delivery stream:** hold `ReceiveDeliveries` open until new
   events or a timeout, so the existing (already-tested) disconnect hook spans
   the adapter's lifetime. Moderate change confined to `receive_deliveries` +
   the adapter poll loop; turns the polling fallback into a long-poll.

Also fold in P3-N1 (commands rot at `running` after mid-execution death — the
documented Q1a bound) and P3-N2 (per-poll full-log command rebuild — perf note
for when the log grows). Documented limitation currently lives in
`docs/RUNBOOK.md` § Known v0.1.0 limitations.

## Simplification opportunity

Option 2 (long-poll) may let the polling-fallback machinery collapse into one
delivery model; whichever option lands should remove the documented
"running-rot" limitation from `docs/RUNBOOK.md` § Known v0.1.0 limitations.

## Design decisions

- **Adapter-liveness mechanism**: use one long-lived authenticated
  `ReceiveDeliveries` stream per current adapter attachment, rather than
  heartbeat/last-report-age — it reuses the already-authenticated delivery
  channel and B3b disconnect hook, covers the demonstrated restart-mid-turn
  path without a clock, threshold, persistence/restart policy, or new adapter
  capability, and removes the external 100ms poll/rebuild cycle. A stream
  remains open after it yields deliveries; ending it after each item would
  retain the mid-execution hole.
- **Clean timeout**: do not use an idle clean-completion timeout in v0.1.0 — a
  timeout creates a no-stream interval in which a dead adapter is again
  indistinguishable. The core leaves an idle stream pending; an abnormal
  transport close/error or replacement attachment is the connection-liveness
  signal. This relies on the transport surfacing a lost peer; a future
  application-level health deadline remains reserved.
- **Running command on adapter loss**: append `running -> failed` with
  `FailureCode::ExecutionOutcomeUnknown` for each non-terminal running command
  targeted to the disconnected adapter — execution may already have happened,
  so redelivery would be unsafe and the existing failure vocabulary is the
  honest outcome. Q1a's delivered-but-not-running redelivery bound remains
  unchanged.
- **N2 performance disposition**: replace per-RPC full-log command-index
  rebuilds with one rebuild at delivery-stream establishment plus incremental
  `read_after`/`CommandIndex::apply` catch-up while the stream remains open.
  This removes the observed full-log rebuild per external poll without adding a
  speculative performance target or a second durable index.
- **Design dispatch**: direct-read only — the delivery, session, command, and
  Pi-loop paths were bounded and inspected locally; no distinct unresolved
  codebase question justified exploratory fan-out.

## Architectural choice

### Options considered

1. **Heartbeat / last-report-age sweep.** Every adapter regularly sends a
   liveness report and the core timer marks its sessions stale after a deadline.
   It can detect a silent/black-holed transport after the deadline, but requires
   a clock and restart policy, a false-stale threshold, periodic reports from
   every participating adapter, and likely a capability/contract promotion.
2. **Long-lived delivery subscription (chosen).** Reuse the existing
   server-streaming `ReceiveDeliveries` RPC as a single pending stream for the
   active attachment. It incrementally scans the durable tail, yields eligible
   operations without completing, and uses the existing stream-drop callback
   for the adapter's whole lifetime. It is the smallest change and directly
   covers the proven adapter-restart-mid-turn failure.
3. **Keep finite polling and add running-command expiry.** This fixes only the
   running record and leaves stale `live/working` presentation, adapter-death
   detection, and per-poll rebuild cost intact. It is insufficient.

The chosen shape uses the existing generated `ReceiveRequest`/`Delivery` wire
contract; no new protocol enum, state, or Pi-specific core primitive is added.
The most difficult unit is the core delivery subscription because it must scan
new durable events, retain the `CommandIndex` state required to filter them,
and never turn an in-flight delivery into an accidental duplicate.

## Extension pressure classification

- **Committed v0.1.0:** a current Pi adapter attachment maintains one
  long-lived authenticated `ReceiveDeliveries` stream. An abnormal loss of
  that stream marks the adapter's sessions `stale` and changes each of its
  `running` commands to `failed` with `execution_outcome_unknown`. This uses
  the existing `SessionConnectivityState`, `CommandState`, and failure
  vocabulary registries; it needs no new state or wire value. Implementation
  updates the adapter-liveness entry in `docs/PROTOCOL.md`'s Extension seams
  registry and the canonical lifecycle/failure prose.
- **Reserved seam:** heartbeat/last-report-age liveness policy (including its
  timer, freshness deadline, and any adapter-declared liveness capability).
  It is deliberately not smuggled in as a Pi-only assumption; promotion must
  name its capability and failure/degradation policy in `docs/PROTOCOL.md`.
- **Explicitly rejected for v0.1.0:** finite clean-completing delivery tails as
  the liveness mechanism. They have already demonstrated a gap between polls
  and after delivery; returning to that shape would be a reversal, not a
  fallback.

## Implementation units

### Unit 1: Durable long-lived delivery subscription and disconnect reconciliation

**Files:**
- `server/src/adapter_service.rs`
- `server/src/adapter_service/tests.rs`
- `core/src/adapter/mod.rs`
- `core/src/acceptance/index.rs`
- `server/Cargo.toml`
- `docs/PROTOCOL.md`

**Story:** `feature-adapter-staleness-liveness-core-delivery-subscription`

```rust
// server/src/adapter_service.rs
const DELIVERY_SCAN_INTERVAL: Duration = Duration::from_millis(100);

type DeliveryStream = Pin<Box<dyn Stream<Item = Result<Delivery, Status>> + Send + 'static>>;

fn delivery_subscription<S>(
    storage: S,
    authority_domain_id: AuthorityDomainId,
    adapter_id: AdapterId,
    initial_cursor: Lsn,
    commands: CommandIndex,
) -> DeliveryStream
where
    S: Storage + Clone + Send + Sync + 'static;

// core/src/adapter/mod.rs
pub async fn fail_running_commands_for_adapter<S: Storage>(
    storage: &S,
    commands: &CommandIndex,
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
) -> Result<Vec<EventId>, AdapterError>;

// core/src/acceptance/index.rs
pub fn records(&self) -> impl Iterator<Item = &CommandRecord>;
```

**Implementation notes:**
- At `ReceiveDeliveries` establishment, rebuild `CommandIndex` once from the
  durable log, install that same projection in `self.commands` for subsequent
  acknowledgement/observation ingestion, and initialize the subscription's
  scan cursor from `ReceiveRequest.cursor`.
- `delivery_subscription` loops until its transport is dropped. Each scan reads
  only `storage.read_after(domain, scan_cursor)`, applies every fetched event to
  its private `CommandIndex`, advances `scan_cursor` through all fetched LSNs,
  then yields only operation events targeted at this adapter whose *post-batch*
  state is `Accepted` or `Delivered`. An empty tail sleeps for
  `DELIVERY_SCAN_INTERVAL`; it never returns `None` for idleness. Post-batch
  filtering preserves B3a while preventing historical operations that have
  become `Running` or terminal from being delivered.
- Keep `DeliveryTail` as the single transport-drop wrapper, but remove finite
  tail completion as normal healthy polling. A dropped stream or error invokes
  the epoch-fenced callback. A newer attachment still makes an older callback
  inert.
- In that callback, under the existing epoch fence and command projection lock,
  call `fail_running_commands_for_adapter` before rebuilding the projection.
  The helper filters command records by target `adapter_id` and
  `OperationState::Running`, appends one canonical `CommandTransition`
  (`Running -> Failed`, `FailureCode::ExecutionOutcomeUnknown`, original
  correlations) per candidate, and leaves `Accepted`/`Delivered` commands
  alone for Q1a redelivery. The existing first-terminal/replay rules remain
  the conflict authority.
- Add Tokio's `time` feature in `server/Cargo.toml`; do not add a second event
  log, a timer-owned domain state, or hand-written boundary DTOs.
- Roll `docs/PROTOCOL.md` forward with the committed/reserved/rejected liveness
  classification and the `running` adapter-loss outcome. It remains code-first
  documentation: the exact final wording lands with the implementation.

**Acceptance criteria:**
- [ ] An empty `ReceiveDeliveries` stream remains pending rather than ending
  cleanly; an operation accepted after it opens is delivered on that same
  stream.
- [ ] Dropping that idle or in-flight stream marks only the current
  attachment's non-stale sessions `stale`; a newer attachment's obsolete drop
  cannot mutate them.
- [ ] A stream loss after `running` commits exactly one `running -> failed`
  transition with `execution_outcome_unknown`; a concurrent/late terminal
  cannot rewrite the first durable terminal outcome.
- [ ] A `delivered`-but-not-`running` command is not failed by loss and remains
  eligible for the existing re-ack/redelivery behavior.
- [ ] The subscription performs one initial full command rebuild per stream
  establishment and incremental tail application thereafter, not a full-log
  rebuild per external poll.

---

### Unit 2: Pi adapter continuous delivery consumption and operator-facing recovery docs

**Files:**
- `pi-adapter/src/core_client.ts`
- `pi-adapter/src/main.ts`
- `pi-adapter/tests/e2e.test.ts`
- `docs/RUNBOOK.md`

**Story:** `feature-adapter-staleness-liveness-pi-delivery-loop`

```ts
// pi-adapter/src/core_client.ts
receiveDeliveries(cursor: bigint, signal?: AbortSignal): AsyncIterable<Delivery>;
async *#receiveDeliveries(
  cursor: bigint,
  signal?: AbortSignal,
): AsyncGenerator<Delivery>;

// pi-adapter/src/main.ts
async run(signal?: AbortSignal): Promise<void>;
async #consumeDeliveries(signal?: AbortSignal): Promise<void>;
async #beginDelivery(
  delivery: Delivery,
  operation: Operation,
): Promise<StartedDelivery>;
```

**Implementation notes:**
- Replace the finite `pollOnce()` + `delay(100)` production loop with
  `#consumeDeliveries`: pass its abort signal to the generated Connect
  server-stream call, update `#cursor` after each received delivery, retain
  the existing acknowledgement-before-execution order, and keep the stream
  active while instruction completions run. `run` re-establishes the stream
  only after a retryable transport failure or attachment refresh, never as an
  idle poll.
- Preserve the current deliberate instruction concurrency: an `INSTRUCT`
  completion is tracked while the consumer continues receiving later
  deliveries (including cancellation); non-instruct work may still await its
  completion. Observation-tail ordering and `flushObservations()` remain the
  completion/error boundary.
- Replace batch-oriented adapter tests that call `pollOnce()` with bounded
  run-loop fixtures: start the consumer, submit an operation after the stream
  is established, await its durable transition, then abort and dispose. Add
  the restart-mid-turn regression: adapter-stream loss makes the session stale
  and exposes the command as failed/`execution_outcome_unknown`, never as
  indefinitely running.
- Update `docs/RUNBOOK.md` § Known v0.1.0 limitations: remove both resolved
  bullets. State the remaining honest ambiguity — a command failed with
  `execution_outcome_unknown` may already have executed, so retry safety is
  determined by `idempotency_strength`, not by the failed state alone.

**Acceptance criteria:**
- [ ] While idle, the Pi adapter owns a live authenticated delivery stream and
  does not issue a 100ms `ReceiveDeliveries` poll loop.
- [ ] A command accepted after the stream opens is acknowledged, reported
  running/completed or failed, and observation ordering remains intact.
- [ ] An adapter restart during an instruction produces stale session
  presentation and a terminal `failed` command carrying
  `execution_outcome_unknown` rather than a permanent `running` record.
- [ ] Abort/shutdown ends the stream and releases adapter resources without an
  unhandled async rejection; restart/unauthenticated attachment refresh still
  reconnects using the existing fenced token path.

## Implementation order

1. `feature-adapter-staleness-liveness-core-delivery-subscription`: make the
   server stream durable-tail incremental and non-completing; reconcile running
   commands on its epoch-fenced abnormal loss; prove the server-side cases.
2. `feature-adapter-staleness-liveness-pi-delivery-loop` (depends on step 1):
   replace finite polling with continuous stream consumption, update the
   integration regression, and roll the runbook forward.
3. Verify the combined core/adapter live-restart scenario and run the focused
   Rust and Pi-adapter suites before feature review.

## Simplification

- Replace finite client polling plus per-RPC full-log `CommandIndex` rebuild
  with one live stream plus incremental tail application; retain the one
  canonical durable log and `CommandIndex` projection.
- Do not add heartbeat messages, last-report timestamps, a liveness timer, a
  new protobuf field, a capability boolean, or a parallel health subsystem.
- Keep B2 attachment fencing, B3a delivered-not-running redelivery, and the
  existing `DeliveryTail` epoch guard; this feature extends their coverage
  rather than replacing them.

## Testing

- **Server delivery-stream regression:** an empty live stream accepts a later
  operation without reopening; its drop marks session state stale. This guards
  the demonstrated between-poll/mid-turn liveness defect.
- **Core reconciliation regression:** a running command on the dropped
  adapter transitions once to failed with `execution_outcome_unknown`, while a
  delivered-but-not-running command remains redeliverable. This protects the
  Q1a boundary and first-terminal finality.
- **Pi integration regression:** abort/restart during a delayed instruction
  yields stale session + terminal failed command; normal continuous delivery
  still reaches completed/idle. This protects the actual live-use failure.
- Existing batch-poll assertions are removed or converted because a normally
  idle stream no longer completes; retain assertions about durable lifecycle
  and observation ordering, which are the useful contract.

## Risks

- **Transport loss detection is not a general application heartbeat.** A
  network black hole that leaves a TCP/HTTP2 stream apparently open can delay
  stale marking until transport failure detection. This is acceptable for the
  v0.1.0 Pi restart failure being repaired; heartbeat/age policy remains the
  named reserved escalation if operations demonstrate that transport liveness
  is insufficient.
- **An incremental stream projection can duplicate or miss delivery if it
  advances its scan cursor before applying a complete batch, or filters against
  pre-transition rather than post-batch state.** Apply all fetched records in
  LSN order, then choose deliverable operations, and test operation arrival
  after stream establishment.
- **Terminal races remain possible at disconnect.** Use the existing command
  projection lock, append-only transition rules, and replay's first-terminal
  handling; do not overwrite an already-terminal command.
- **A permanently held stream changes batch-oriented test control flow.** Use
  cancellation-aware test fixtures with explicit durable-state waits, not
  arbitrary sleeps or normal `None` completion.

## Other agent review

- **Skipped/degraded:** this is a high-risk delivery/state design, but this
  harness invocation exposes no subagent dispatch tool. The design therefore
  records direct code evidence and targeted regression seams for the normal
  feature-level independent review after implementation.
