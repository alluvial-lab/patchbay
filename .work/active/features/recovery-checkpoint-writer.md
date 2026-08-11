---
id: recovery-checkpoint-writer
kind: feature
stage: done
tags: [perf, protocol, foundation]
parent: null
depends_on: [snapshot-core-generation-semantics, replay-integrity-prefix-discipline, session-registry-replay-domain-soundness, adapter-report-source-ordering]
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-11
---

# Recovery checkpoint writer + scheduling policy

## Brief
Add a production checkpoint writer so recovery replay cost stays bounded as the durable log grows. Split out of `storage-recovery-correctness` (a `[perf]`-bearing item the consolidation had silently dropped). Absorbs:

- `backlog-snapshot-checkpoint-writer` — **OPEN**: production state says durable checkpointing is deferred (`server/src/state.rs:242-247`) and recovery replays the whole log (`PROTOCOL.md:578-584`); a snapshot table + write port exist (`core/src/storage/rusqlite.rs:65-69,806-830`) but no production scheduling/materialization writer. *Src:* docs-audit 2026-07-27.

## Direction
Define an explicit policy over committed events/bytes (or measured replay cost), with crash-safe retry and failure observability; a snapshot write failure must leave the log authoritative and recovery-correct. **Scope the bound honestly**: the checkpoint namespace is session-only today, so a session-only checkpoint does NOT bound whole-core recovery (authority/command/Elicitation/resource rebuild from log) — either choose session-only with a narrowly stated bound, a typed composite checkpoint, or per-projection checkpoints each anchored to one durable prefix; don't claim globally bounded recovery until every load-bearing projection has a compatible checkpoint.

## The "no second state store" invariant (corrected)
The consolidation's "no second state store" was ambiguous and **foreclosing** (it could prohibit the existing derived snapshot table, replicas, or a second backend — all post-v1 seams). Restate the actual invariant: **no derived checkpoint, replica, or projection may become an independent ordering or authority source; recovery validates it against the durable log's domain/epoch/LSN anchor.** Physical-topology commitments (single backend, etc.) are scoped to v1.

## Foundation references
- `docs/PROTOCOL.md` (`:578-584`), `docs/ARCHITECTURE.md` (`:197-201` — snapshots are derived; log is ordering authority), `docs/SPEC.md` (post-v1 storage seams)
- Code: `core/src/storage/rusqlite.rs`, `server/src/state.rs`

## Design decisions

- **v0.1.0 writes and consumes a complete session-projection checkpoint only.** The private checkpoint covers the authority-domain-bound `SessionRegistry`: every live record, retained generation tombstone, lockdown clamp, current source-order cursor once `adapter-report-source-ordering` lands, and the checkpoint prefix. It does not contain authority grants/operators, commands, Elicitation slots, resources, diagnostics, security/operator projections, adapter registration/delivery projections, or operator-session recovery state.
- **The bound is narrow and conditional, not a whole-core startup promise.** A healthy writer targets one checkpoint for each 256 newly observed authority-domain log events and checks once per second. A successful pass moves the session checkpoint to the applied head, so later session recovery folds only events committed after that anchor rather than session-folding history from LSN 1. The total core still reads/replays load-bearing sibling projections from the full log, and the adapter/control processes still do their non-session rebuilds. Events committed before the next scheduler pass, sustained write failure, process starvation, or overload may make the session tail exceed 256; the log-length-independent target is not an absolute wall-clock, failure, or adversarial-ingress bound.
- **Use event count, not bytes or invented replay-time budgets.** LSN distance is backend-neutral and already available. v0.1.0 has no measured load profile or quantitative performance SLA, and the storage port exposes neither canonical stored-byte accounting nor measured replay cost. `256 events`, a one-second poll, and a 1→30 second retry backoff are private operational defaults behind a typed policy, not public protocol/SLA values.
- **A recovery checkpoint must be complete for the projection it seeds.** The existing public `SessionSnapshot` is insufficient by itself because it omits the registry's retained generation tombstones and replay-boundary metadata. Add a generated private stored payload containing that snapshot plus tombstones, and restore only after structural and semantic validation. Do not copy opaque session bytes into a second handwritten DTO.
- **Change the private session checkpoint body in place.** Bump the private envelope format from 1 to 2 and reject format-1/public-`SessionSnapshot` bodies as disposable derived data. No dual reader or data migration is warranted; a one-time full replay rewrites the current format.
- **Covered-prefix redelivery fails closed.** After checkpoint hydration, the session registry records `covered_through_lsn = checkpoint_lsn` and accepts new replay only after that prefix. It does not retain every covered event byte in the checkpoint. Any direct event at or below the covered prefix is rejected rather than accepted on LSN alone; exact redelivery remains available for post-checkpoint events tracked by the session registry's equality ledger. This preserves the sibling replay-equality feature's safety rule without making checkpoint size grow with all prior session events.
- **Scheduling never participates in authority or availability.** Materialization occurs from the caught-up projection under the shared `CoreDecisionGate`; the gate is released before the atomic snapshot write because later log appends do not invalidate a checkpoint of prefix N. A write failure leaves the prior checkpoint and event log unchanged, does not fail accepted Operations or adapter reports, emits a bounded structured stderr observation, and retries from durable state.
- **Retain one production session checkpoint per authority domain.** `Storage::write_snapshot` atomically validates the event anchor, rejects regression behind a newer stored checkpoint, and replaces prior rows for that domain in the same transaction. Historical `at_or_before` reads may therefore return no stored row and already repair to current materialization; the derived table does not grow one full session payload per scheduling interval.
- **Sequence behind the active replay/session contract work.** `replay-integrity-prefix-discipline` supplies exact complete-tail validation, `session-registry-replay-domain-soundness` supplies the bound registry/equality ledger this feature compactly seeds, and `adapter-report-source-ordering` adds a load-bearing session source cursor that the checkpoint must carry. These were not known dependencies in the original split, but implementing against the pre-change registry would create an incomplete format and immediate rework. Cycle checks were clean, so they are now declared feature prerequisites.
- **Assurance remains implementation-checked and narrowly worded.** Extend executable recovery/storage/server evidence and the existing snapshot vector runner as needed for the new format, but do not promote the draft snapshot model or claim checked-normative scheduling/bounded-recovery semantics. The Quint file remains a draft abstract whole-core model and must be relabeled so it cannot be mistaken for evidence of this session-only writer.
- **Execution posture and review policy.** Direct-read only because the caller prohibits nested agents/peers; direct mapping covered the storage port/backend, checkpoint codec, aggregate and adapter session rebuilds, scheduler composition root, active overlapping work, tests, model, and all foundation assertions. Effective `review_weight` remains `thorough` (explicit operator selection). Implementation/feature/final review must retain it and treat reviewer findings as proposals for receiver adjudication.

## Codebase mapping

The current production state is exactly the brief's gap: SQLite can write/load snapshot rows and the server can encode/decode a typed exact domain/epoch/LSN session checkpoint, but no production task writes one. `ProjectionState::rebuild*` folds all events from zero; `AdapterControlServiceImpl` independently rebuilds sessions from zero at startup and around session reports. The current public session snapshot serializes live records and lockdown state but not retained generation tombstones, and the active session-soundness feature is adding a domain binding plus exact owned-event replay ledger. The storage table retains every `(domain, snapshot_lsn)` row today, so a periodic writer also needs an explicit latest-only retention rule.

No exploratory fan-out or design peer ran: the delegated endpoint forbids nested agents and peer mechanisms. That degradation is non-blocking under the advisory policy, and the direct map included all complete-log `SessionRegistry` rebuild call sites plus the three active prerequisite designs that will change the owned interfaces.

## UI fallback

No UI surface. v0.1.0 failure observability is a redacted structured process-stderr event; no cockpit, CLI, or mockup change is introduced.

## Architectural choice

### Options considered

1. **Chosen — complete session-only checkpoint plus event-gap background writer.** Persist the complete current `SessionRegistry` shape in the existing typed/versioned session namespace, restore every production session registry from it plus a strictly validated tail, and schedule best-effort writes from a background task. This makes one narrow recovery fold independent of total event history while keeping failure non-authoritative and leaving future namespaces open.
2. **Whole-core composite checkpoint now.** A composite could support a real whole-core replay bound, but it must include and validate authority/operator state, commands/inboxes, Elicitations, sessions, resources, diagnostics, security posture, adapter state, and operator-session recovery metadata at one prefix. Those projections do not expose a common complete serialization contract, and pretending a partial composite is whole-core would be unsafe. This is reserved, not smuggled into a performance feature.
3. **Per-projection namespace registry now.** Independent typed checkpoints are a sound long-term shape and avoid a giant composite, but they require a storage namespace/key registry, per-projection compatibility/retention rules, coordinated prefix selection, and writers/readers for every load-bearing projection. v0.1.0 has one proven session envelope and no second committed checkpoint consumer, so building the framework before the variants is premature.

A fourth apparent shortcut—periodically persisting the existing public `SessionSnapshot` and leaving recovery unchanged—was rejected because it omits retained registry state and bounds no production recovery path.

## Trickiest unit first

The hardest unit is a **complete, compact, fail-closed session recovery seed**. It must round-trip live session state, source cursors, generation tombstones, lockdown clamping, authority-domain ownership, and record revisions without copying the entire owned-event equality ledger. It must then admit only the contiguous tail after the validated anchor, while wrong type/version/domain/epoch/LSN, duplicate identities, malformed states, invalid tombstones, or inconsistent view revisions fall back to full replay. If this unit is incomplete, the scheduler would merely automate writing an unsafe cache.

## Implementation Units

### Unit 1: Complete session checkpoint and shared session recovery path

**Files**: `contracts/proto/patchbay/sessions.proto`, `contracts/rust/src/gen/patchbay/patchbay.rs`, `contracts/ts/src/gen/patchbay/sessions_pb.ts`, `core/src/session/registry.rs`, `core/src/session/replay.rs`, `core/src/session/mod.rs`, `core/src/storage/recovery.rs`, `core/src/storage/mod.rs`, `server/src/snapshot.rs`, `server/src/state.rs`, `server/src/adapter_service.rs`, `server/src/service.rs`

**Story**: `recovery-checkpoint-writer-session-recovery-state`

```proto
// Private persisted payload inside the existing typed/versioned envelope.
// It is generated wire shape, but is not returned by LoadSnapshot.
message StoredSessionCheckpoint {
  SessionSnapshot snapshot = 1;
  repeated SessionCheckpointTombstone tombstones = 2;
}

message SessionCheckpointTombstone {
  AdapterId adapter_id = 1;
  string deployment_scope = 2;
  RuntimeSessionId runtime_session_id = 3;
  Generation generation = 4;
  Lsn superseded_at_lsn = 5;
}
```

```rust
// core/src/session/registry.rs
impl SessionRegistry {
    pub fn from_checkpoint(
        authority_domain_id: AuthorityDomainId,
        checkpoint_lsn: u64,
        live_records: Vec<SessionRecord>,
        tombstones: Vec<SessionTombstone>,
        lockdown_active: bool,
    ) -> Result<Self, SessionError>;

    pub fn tombstones(&self) -> impl Iterator<Item = &SessionTombstone>;
    pub fn covered_through_lsn(&self) -> Option<u64>;
}

// server/src/snapshot.rs
pub struct CompatibleSessionCheckpoint {
    pub snapshot: SessionSnapshot,
    pub registry: SessionRegistry,
}

pub struct RecoveredSessionRegistry {
    pub registry: SessionRegistry,
    pub checkpoint_lsn: u64,
    pub replayed_event_count: usize,
}

pub fn encode_session_checkpoint(checkpoint: &StoredSessionCheckpoint) -> Vec<u8>;

pub fn decode_compatible_session_checkpoint(
    stored: &StoredSnapshot,
    expected_domain: &AuthorityDomainId,
    expected_core_generation: &Generation,
) -> Result<CompatibleSessionCheckpoint, SessionCheckpointRejection>;

pub async fn recover_session_registry<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    core_generation: &Generation,
) -> Result<RecoveredSessionRegistry, SessionError>;

// server/src/state.rs
impl ProjectionState {
    pub async fn materialize_session_checkpoint(
        &self,
        authority_domain_id: AuthorityDomainId,
        materialized_at: Timestamp,
    ) -> StoredSessionCheckpoint;
}
```

**Implementation notes**:

- Generate the two stored-payload messages from `sessions.proto`; never hand-edit generated Rust/TypeScript. The outer magic and session kind stay unchanged, while `CHECKPOINT_FORMAT_VERSION` becomes 2 and the body becomes `StoredSessionCheckpoint`.
- Convert the generated snapshot's live records into domain `SessionRecord`s and the generated tombstones into domain `SessionTombstone`s, then let `SessionRegistry::from_checkpoint` own semantic validation. Require the bound domain; positive checkpoint LSN/generations; known non-unspecified states; non-empty identity fields; unique live keys and tombstone keys; record/tombstone revisions in `1..=checkpoint_lsn`; no live record marked tombstoned; no tombstone for the current live generation; one view revision per live record matching its target and `last_authoritative_lsn`; and lockdown consistency.
- Carry every field in the post-prerequisite `SessionRecord`, including `last_source_cursor`. Do not freeze the pre-source-order record shape in a checkpoint-specific mirror.
- Construct a checkpoint-seeded registry with `covered_through_lsn = checkpoint_lsn`, an empty equality ledger for the covered prefix, and normal exact ledger tracking for tail events. Reject any direct event at/below the covered prefix; never treat its LSN alone as proof of equality.
- `recover_session_registry` calls the validator-aware generic `recover`; the prerequisite shared replay validator enforces an exact tail beginning at `checkpoint_lsn + 1`. Any codec/semantic rejection returns `None` to generic recovery and folds from zero. Tail application uses the same `SessionRegistry::observe` path as live ingestion.
- Use this helper for `ProjectionState` startup and every production `AdapterControlServiceImpl` session rebuild. The aggregate state still folds the full log through all sibling projections; it must skip only the session fold at/below the recovered anchor. Adapter service retains its shared decision gate around report-time rebuilds.
- `LoadSnapshot` unwraps and returns only `CompatibleSessionCheckpoint.snapshot`; the private tombstone payload never crosses the public RPC.

**Acceptance criteria**:

- [ ] Format-2 round-trip restores every live record, retained tombstone, lockdown clamp, revision, model, and source cursor, then tail replay is byte-for-byte projection-equivalent to a fresh full-log session rebuild.
- [ ] Control-service startup and adapter-service startup/report refresh consume the shared session recovery helper and report only tail event applications when a compatible checkpoint exists.
- [ ] Legacy format 1/raw bytes and wrong type/version/domain/epoch/row-or-embedded LSN, malformed live records/tombstones, duplicate identities, or inconsistent view revisions never seed a registry and fall back to exact full replay.
- [ ] A checkpoint-seeded registry rejects all covered-prefix direct re-feed, including conflicting bytes, and retains exact-redelivery behavior for tail events.
- [ ] No command, grant, Elicitation, resource, diagnostic, adapter, security/operator, or operator-session projection is skipped because a session checkpoint exists.

### Unit 2: Event-gap scheduler, atomic replacement, and failure observation

**Files**: `server/src/checkpoint.rs` (new), `server/src/lib.rs`, `server/src/service.rs`, `server/src/main.rs`, `core/src/storage/port.rs`, `core/src/storage/rusqlite.rs`, `core/src/storage/audited.rs`, `core/tests/rusqlite_storage.rs`

**Story**: `recovery-checkpoint-writer-scheduling-runtime`

```rust
// server/src/checkpoint.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCheckpointPolicy {
    pub events_per_checkpoint: NonZeroU64,
    pub poll_interval: Duration,
    pub retry_initial: Duration,
    pub retry_max: Duration,
}

impl Default for SessionCheckpointPolicy {
    // 256 events; 1s poll; 1s initial retry; 30s retry cap.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointTickOutcome {
    EmptyLog,
    NotDue { checkpoint_lsn: u64, current_lsn: u64 },
    Written { prior_lsn: u64, checkpoint_lsn: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointFailureStage { CatchUp, Load, Materialize, Write }

pub trait CheckpointObserver: Send + Sync {
    fn observe_failure(
        &self,
        stage: CheckpointFailureStage,
        attempted_lsn: Option<u64>,
        consecutive_failures: u32,
        error: &str,
    );
}

pub struct SessionCheckpointWriter<S> { /* storage, state, domain, clock, policy, observer */ }

impl<S> SessionCheckpointWriter<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    pub async fn run_once(&self) -> Result<CheckpointTickOutcome, CheckpointWriterError>;
    pub async fn run(self);
}
```

**Implementation notes**:

- `ControlServiceImpl::session_checkpoint_writer` constructs the worker with clones of storage/state, the configured domain, existing injected `Clock`, default/private policy, and `StderrCheckpointObserver`. Tests inject a fixed clock, small threshold, and recording observer.
- On each immediate/periodic pass, acquire the state's shared decision gate, catch it up, load/decode the latest compatible session checkpoint, and compare its anchor with the applied head. If the log is empty or the gap is below the policy threshold, release and skip. If due, materialize one complete checkpoint at the caught-up head, release the gate, encode, and write it. Later appends do not invalidate prefix N.
- The run loop resets backoff after `EmptyLog`, `NotDue`, or `Written`. On any failure it emits one redacted structured process-stderr event with stage, attempted LSN, retryability/class, and consecutive count, sleeps exponentially from 1 to 30 seconds, then recomputes everything from durable state. It never appends an audit/log event for its own failure and therefore cannot recursively advance the scheduling LSN.
- Start and monitor the worker from `server/src/main.rs` alongside both tonic servers. Unexpected task termination is surfaced; ordinary checkpoint errors stay inside the retry loop and never take down serving.
- Tighten production `write_snapshot`: inside one SQLite writer transaction, validate the anchor exists, reject a write below the current stored checkpoint LSN, delete prior rows for that authority domain, insert/replace the candidate, and commit. Any error rolls back to the old row. `AuditedStorage` remains a transparent delegate and checkpoint writes consume no event/audit LSN.
- Keep the current session-only storage key shape for v0.1.0. Do not add a namespace registry until a second projection checkpoint is promoted.

**Acceptance criteria**:

- [ ] With threshold N, gaps `< N` skip and a gap `>= N` writes the caught-up head; the next successful pass skips until N more observed events.
- [ ] A materialized checkpoint reflects every session/security event through its anchor and none after it even when later events commit before the SQLite write completes.
- [ ] A failed write leaves the log and prior checkpoint byte-for-byte unchanged, emits a typed failure observation, and a later pass succeeds without process restart or Operation failure.
- [ ] SQLite retains at most one production snapshot row per authority domain after a successful write, rejects stale replacement, and consumes no LSN.
- [ ] Worker cancellation/crash during materialize/write leaves either the old or fully committed new checkpoint; restart validates whichever exists and otherwise replays the log.

### Unit 3: Recovery-bound evidence and honest foundation/model roll-forward

**Files**: `core/tests/recovery.rs`, `core/tests/sessions_replay_resolver.rs`, `server/src/snapshot.rs` (tests), `server/src/state.rs` (tests), `server/src/adapter_service/tests.rs`, `server/tests/grpc_smoke.rs`, `server/tests/conformance_vectors.rs`, `contracts/vectors/snapshot-reconciliation.json`, `specs/seed/snapshot_recovery.qnt`, `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`

**Story**: `recovery-checkpoint-writer-bounded-recovery-evidence`

**Implementation notes**:

- Add a deterministic file-backed integration: construct session history with generation replacement, lockdown entry/exit, source-cursor changes, and sibling non-session events; run a small-threshold writer; append a short tail; drop/reopen; then compare full-log and checkpoint+tail `SessionRegistry` values and assert the recovery helper applied exactly the tail count.
- Exercise both production consumers: aggregate `ProjectionState` and `AdapterControlServiceImpl`. Assert pre-checkpoint commands/grants/Elicitations/resources/security/operator state still rebuild from the log, proving the session cursor did not hide sibling history.
- Inject a storage wrapper that fails the first snapshot write after materialization. Assert the accepted event prefix remains readable, the prior checkpoint stays selected, the observer sees the failure, and the next pass advances it. Add an atomic latest-row regression at the SQLite port.
- Update the existing promoted snapshot-reconciliation runner only as required to construct/validate format 2 and continue proving its existing stale-snapshot example. Do not change its property id/classification or pretend it proves scheduling/recovery bounds.
- Roll foundation assertions forward in place: periodic session checkpoint writer implemented; exact healthy-policy qualification; session-only projection fold bound; whole-core replay still unbounded because all listed sibling projections are absent; one latest derived row; failure/retry observability; composite/per-projection namespaces reserved. Remove the timeless/general claim that any current snapshot bounds whole-core recovery.
- Update `snapshot_recovery.qnt` comments/deferral text so its command-oriented abstract checkpoint remains a draft future whole-core model, not a model of the session-only writer. Compile and run traceability checks without promoting any property.

**Acceptance criteria**:

- [x] A real restart restores the same complete session projection from format 2 plus only the post-anchor tail, for both production session-registry consumers.
- [x] The same fixture proves load-bearing non-session projections still full-replay pre-checkpoint facts; no test or doc reports whole-core bounded recovery.
- [x] Crash/write-failure/retry evidence proves checkpoints affect cost only, never accepted-state durability, authority, log order, or service availability.
- [x] The promoted snapshot vector still runs with unchanged `SnapshotStaleRejected` classification; model metadata remains draft/stated-normative.
- [x] Foundation docs consistently state the committed narrow v0.1.0 behavior, reserved namespace/composite seams, and explicit rejection of checkpoint-as-authority.

## Implementation record

- Capability: highest (`openai-codex/gpt-5.6-sol`) for protocol, persistence, generated-contract, and recovery work.
- Landed implementation: `09f36c2`; adversarial fixes: `24c3475`, `d45efe7`, `8f9e582`; central restart/failure/mutation evidence: `c0c238b`.
- The final shape restores complete session state only, forces full replay and prompt repair for rejected or tail-inconsistent checkpoints, keeps sibling-owned state on full-log replay, and retains one atomic latest row without gating service availability.
- Full workspace tests, warnings-denied Clippy, Quint compile, model/vector checks, TypeScript build, generated drift, and diff hygiene pass.

## Implementation Order

1. Wait for `replay-integrity-prefix-discipline`, `session-registry-replay-domain-soundness`, and `adapter-report-source-ordering` to reach `done`; re-read their landed interfaces before source changes.
2. `recovery-checkpoint-writer-session-recovery-state` — land the complete format-2 payload, compact covered-prefix semantics, validator-aware recovery, and both production session consumers.
3. `recovery-checkpoint-writer-scheduling-runtime` — add the deterministic event-gap tick, latest-only atomic storage write, retry loop, observer, and production task wiring.
4. `recovery-checkpoint-writer-bounded-recovery-evidence` — attack restart equivalence/failure mutations, update the existing vector runner, and roll model/foundation wording forward.
5. Run focused and workspace verification, close child checkpoints by evidence, then review the integrated feature at explicit `thorough` weight until a pass yields no receiver-confirmed material current-cycle blockers.

One feature-owning implementation worker should carry all three checkpoints. `snapshot.rs`, `state.rs`, session recovery, and the restart fixtures overlap; splitting ownership would create non-green format/scheduler handoffs. The stories are durable acceptance checkpoints, not parallel agent assignments.

## Simplification

- Reuse the existing snapshot table, `Storage` port, writer actor, typed/versioned header, persisted generation, generic validator-aware recovery, shared replay-prefix validator, generated `SessionSnapshot`, injected clock, and composition-root decision gate.
- Replace the stale “snapshot discriminator absent / checkpointing deferred” comments and duplicate full-session rebuild call sites with one recovery helper. Do not create a second recovery framework.
- Retain one checkpoint row, not a growing checkpoint history. Retain projection state/tombstones, not the full covered event-equality ledger.
- Add no byte counter, replay timer, metrics subsystem, public tuning API, audit recursion, checkpoint authority, resource checkpoint, composite registry, storage-backend assumption in domain code, or format-1 compatibility shim.
- No valuable test is removed. Update implementation-bound tests that expected multiple historical snapshot rows; preserve RPC behavior by repairing an unavailable historical bound to current authority.

## Testing

- **Codec/registry interface tests** protect complete state, exact anchors, format discrimination, semantic validation, covered-prefix fail-closed behavior, and full-fallback repair.
- **Recovery equivalence tests** protect that checkpoint+tail equals full session replay while sibling projections still consume the entire log. This is the central honesty oracle.
- **Scheduler interface tests** protect due/skip decisions independently of real time by calling `run_once` with a small policy and fixed clock.
- **Failure/atomicity regression** protects old-checkpoint retention, retry, no LSN/log mutation, and observable failure.
- **Existing promoted vector** protects the public stale-snapshot scenario only; it is not repurposed into a scheduling proof.
- **No low-value tests** for generated accessors, every backoff step, or arbitrary default constants. The contract is policy comparison, atomic failure, and restart equivalence.

## Verification commands

```bash
(cd contracts && buf generate)
cargo test -p patchbay-contracts
npm --prefix contracts/ts run build
cargo test -p patchbay-core --test recovery
cargo test -p patchbay-core --test rusqlite_storage
cargo test -p patchbay-core --test sessions_replay_resolver
cargo test -p patchbay-core-server snapshot::tests --lib
cargo test -p patchbay-core-server checkpoint --lib
cargo test -p patchbay-core-server adapter_service
cargo test -p patchbay-core-server --test grpc_smoke checkpoint
cargo test -p patchbay-core-server --test conformance_vectors snapshot_reconciliation
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
quint compile specs/seed/snapshot_recovery.qnt
node contracts/scripts/check-models.mjs
node contracts/scripts/check-vectors.mjs
npm --prefix contracts/ts run check:drift
```

## Risks

- **Scope language can accidentally overclaim whole-core recovery.** The implementation evidence must name `SessionRegistry` and tail applications, then separately prove pre-checkpoint sibling facts still full-replay. Foundation text lists every excluded load-bearing projection rather than saying “core recovery is bounded.”
- **A structurally valid but incomplete checkpoint could erase derived session facts.** Generated format 2 includes tombstones and post-prerequisite source cursors; `SessionRegistry::from_checkpoint` owns unique/domain/revision/state validation; equality against full replay is the independent oracle.
- **Checkpoint compaction can weaken replay equality.** The seeded registry never accepts covered events by LSN alone. It rejects the entire covered range and tracks exact payload equality only for new tail events. If a future caller requires covered-prefix re-feed, it must design authenticated immutable-record evidence rather than silently restore LSN-only acceptance.
- **The scheduling target is not a hard bound under failure or scheduler delay.** The one-second check and 256-event target are honest healthy-steady-state policy. Failure is observable and retrying, but correctness—not cost—is the only unconditional guarantee.
- **Writing under the decision gate could harm responsiveness.** The design holds the gate only through catch-up/materialization, releases it before encoding/storage I/O, and relies on the immutable prefix anchor. If materialization itself becomes measurable, the fallback is a projection-owned clone under the same locks, not blocking Operations on snapshot success.
- **Replacing historical rows changes internal bounded-load behavior.** Public `LoadSnapshot` already repairs missing/older historical materialization to the current view. Storage tests must lock this behavior, and the internal API is not a published external compatibility surface.
- **Private format 1 becomes unusable.** This causes one full replay and format-2 rewrite, not authoritative data loss. A dual reader would add unsafe ambiguity and ongoing cost for disposable derived data.
- **Active prerequisite interfaces may land differently from their designs.** Implementation must re-read the actual commits and adapt the checkpoint payload to the final bound registry/source cursor/shared prefix helper. The declared dependencies prevent guessing against draft interfaces.
- **Design-time independent review was unavailable by instruction.** Direct exhaustive mapping, explicit negative mutations, and mandatory `thorough` integrated review mitigate this non-blocking degradation.

## Extension pressure classification

- **Committed v0.1.0:** one best-effort periodic, typed/versioned, latest-only session checkpoint per authority domain; complete `SessionRegistry` restoration; exact domain/epoch/LSN and semantic validation; 256-observed-event/one-second private healthy-policy target; structured failure observation and retry; log authority/full fallback.
- **Reserved seams:** typed composite whole-core checkpoints; per-projection namespace/key registry and coordinated prefix selection; resource/command/authority/Elicitation/diagnostics/security/adapter/operator checkpoints; byte- or measured-cost scheduling after instrumentation; external tuning; HA/replica checkpoint ownership and epoch rollover.
- **Explicitly rejected for this feature:** calling the session checkpoint a whole-core recovery bound, using public `SessionSnapshot` alone, accepting covered events by LSN alone, blocking accepted work on checkpoint failure, making checkpoint order authoritative, retaining unbounded historical session checkpoint rows, a legacy dual reader, and a speculative generic checkpoint framework.
- **Parked-idea pressure:** multi-human/federation remains demarcated by authority domain and requires its own authority/replication checkpoint design; agent mesh, desktop/mobile surfaces, and operator skins are unaffected.

## Other agent review

- **Invoked because:** recovery scheduling/durability is protocol- and performance-bearing, and an incomplete checkpoint could silently hide authoritative history.
- **Thorough convergence:** five fresh `openai-codex/gpt-5.6-sol` passes attacked checkpoint/tail disagreement, semantic completeness, all session consumers, decision-gate scope, idle cost, retry observability, zero generations, structural repair, and central file-backed evidence.
- **Receiver-adjudicated fixes:** full-replay fallback plus forced replacement; stricter cursor/session/tombstone/lockdown semantics; diagnostics embedded-session seeding; idle fast paths; stable redacted failure classes; encoding outside the gate; positive-generation property domains; below-threshold incompatible repair; byte-exact prior-row preservation; semantic mutation matrix; file-backed dual-consumer restart with pre-anchor sibling facts.
- **Final verdict:** `ready`, no material current-cycle blockers. Whole-core composite/per-projection checkpointing, measured-cost scheduling, formal promotion, and process-kill SQLite fault injection remain optional/reserved rather than current acceptance gaps.
- **Rejected:** public-snapshot-only writing, whole-core claims, checkpoint-authoritative ordering, and checkpoint failure backpressure.

## Status (closed 2026-08-11)
Implemented, fully verified, and approved at `thorough` review weight. Session checkpointing is production-wired with honest session-only recovery scope; all three child checkpoints are done.
