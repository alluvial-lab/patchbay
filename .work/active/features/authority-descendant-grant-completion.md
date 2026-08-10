---
id: authority-descendant-grant-completion
kind: feature
stage: done
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
---

# Descendant-grant live completion (audit producer + composition root)

## Brief
Close the descendant-grant provenance obligation for the live path. Absorbs two coupled findings split out of `authority-provenance-hardening` (currency verified 2026-08-09):

- `backlog-authority-durable-acceptance-metadata` — **PARTIAL**: acceptance now overwrites sender with the verified issuer + durably stores the grant (`core/src/acceptance/pipeline.rs:316-322`), replay reconstructs the grant-bearing command (`index.rs:139-175`), and the spawn-tail consumes it (`spawn_tail.rs:148-152`); **but `audit_id` is still `None`** (`spawn_tail.rs:292-295`). *Src:* authority review #2(C)+#3(2).
- `backlog-authority-live-composition` — **OPEN**: `SpawnDescendantTail` "does not write grants or own a live consumer loop" (`spawn_tail.rs:1-5`); no production composition root feeds committed events to it. *Src:* authority review #3(E).

## Direction
Add the spawn-completion audit producer and carry its `EventId` into `DescendantGrant.audit_id`; wire a live composition root (startup rebuild → bootstrap → cursor catch-up → continuous committed-event delivery) that drives `SpawnDescendantTail`'s `Issuance` into `ingest_descendant_grant` durably. Do not expose a spawn as complete until registration/bump, descendant grant, and audit record are durably committed through one decision (or equivalently crash-safe protocol). Couples with the ingress features (verified-issuer supply) and with the spawn redesign's descendant-authority requirement.

## Foundation references
- `docs/PROTOCOL.md` — descendant-grant provenance (`DescendantGrantProvenance { spawn_operation_id, spawning_grant_id }` + `DescendantGrant.audit_id`)
- `docs/SECURITY.md` — grant-lifecycle provenance + audit
- Code: `core/src/authority/spawn_tail.rs`, `core/src/acceptance/pipeline.rs`, `core/src/acceptance/index.rs`

## Design decisions
- **Adopt the review-vetted 2026-08-09 direction without reopening the descendant-authority model.** The implementation completes the existing same-actor/new-session grant, canonical allowed-kind set, deterministic descendant id, two-lever revocation, and audit linkage; it does not add delegation, inherited kinds, or cascade revocation.
- **A successful spawn `RESULT` is durable completion evidence, not immediate `CommandState = completed`.** Generic observation ingestion records that result but defers the spawn's terminal transition. The live completion driver writes `completed` only after the correlated registration or generation bump, spawn-completion audit, and descendant grant are durable. This makes the lifecycle state—not optimistic evidence—the public completion authority.
- **Use a crash-safe staged decision, not a storage-specific spawn transaction.** While holding the composition-root `CoreDecisionGate`, the driver durably records audit → descendant grant → completed transition and folds each result before continuing. The final transition is the exposure point. A crash before it leaves a replayable prefix that startup finishes before listeners open; no storage backend must decode or synthesize authority payloads.
- **Reuse `AuditEventKind::CommandCompleted` with `reason_code = "spawn_completion"`.** The canonical audit vocabulary already represents command completion. A new spawn-only enum would duplicate outcome semantics; the bounded reason code distinguishes this producer. The audit's `EventId` is the required `DescendantGrant.audit_id`.
- **Carry spawn origin through generation replacement.** Add `SessionGenerationBumped.spawn_origin` at new protobuf tag 11 and populate it from `SessionReport.spawn_origin`. Otherwise the brief's registration/**bump** completion path cannot be correlated durably.
- **The descendant subject keeps verified endpoint narrowing when present.** `AcceptedOperation.operation.sender` is already replaced at ingress with the verified actor/endpoint/device. The grant carries the verified actor and optional endpoint; the spawn-completion audit additionally carries the verified device. Self-asserted adapter/display fields never participate.
- **One fail-closed server driver owns live completion.** It replays from LSN 0, repairs any incomplete prefix to quiescence before service construction/listener binding, then continuously catches up the durable tail under the shared gate. Service handlers do not grow duplicate post-write hooks.
- **Historical completed spawns are repair input, not a second completion path.** If replay finds a legacy `completed` transition without its linked audit/grant, startup adds the missing durable records but does not append another terminal transition. New successful spawns always follow audit → grant → terminal transition.
- **Formal status remains honest.** This feature adds mutation-sensitive implementation evidence for the stated-normative `SpawnCreatesDescendantGrant` obligation. It does not promote the currently inadequate `authority.qnt` formula or the reserved `spawn-descendant-grant` vector by metadata alone.
- **Exploration posture:** direct-read only. The affected core/server/storage/test paths were bounded and inspected locally; the delegated endpoint explicitly forbids nested subagents and peer mechanisms.

## UI alignment
No UI surface changes. Existing command-state and grant/audit projections consume the completed durable records; mockups are skipped.

## Architectural choice

### Option A — specialized atomic spawn-completion storage primitive
Allocate an audit id and write the audit, descendant grant, and terminal transition in one backend transaction. This maximizes physical atomicity, but the storage port would need a spawn-specific payload builder or would have to decode/re-encode authority contracts after assigning LSNs. That violates the backend-neutral storage boundary and duplicates the domain state machine in infrastructure.

### Option B — replayable staged decision under the shared gate (chosen)
Treat the successful result as durable evidence, then have one domain-aware server driver write audit, grant, and final transition in that order while retaining `CoreDecisionGate` until the chain is quiescent. Every partial prefix is recognizable from the log and restart-safe; external core reads cannot observe an intermediate prefix because they use the same gate, and listeners do not open until bootstrap repair finishes. This reuses `Storage`, `AuditSink`, `SpawnDescendantTail`, and `ingest_descendant_grant` without backend coupling.

### Option C — invoke completion hooks from Submit/session/observation handlers
Call the tail after each relevant RPC. This is superficially direct, but it scatters ownership across two services, misses crash prefixes and non-RPC appends, and risks one service using a different cursor/projection. It does not satisfy the requested production composition root.

**Choice:** Option B. It is the smallest adapter-neutral design that preserves final-state honesty and has a complete crash story. Option A is unnecessary specialization; Option C is not a live composition root.

## Trickiest unit first
The riskiest unit is the **replayable completion fold plus final-exposure rule**. It must distinguish durable success evidence from a public terminal state, survive every crash prefix, respect an earlier competing terminal LSN, and never issue from self-asserted identity. The design therefore makes durable facts—not an in-memory `issued` set—the checkpoints and makes `CommandTransition(completed)` the last action for new spawns.

## Implementation units

### Unit 1: Complete the durable correlation contract and fold
**Story:** `authority-descendant-grant-completion-contract-fold`

**Files:**
- `contracts/proto/patchbay/sessions.proto`
- generated `contracts/rust/src/gen/patchbay/patchbay.rs`
- generated `contracts/ts/src/gen/patchbay/sessions_pb.ts`
- `core/src/session/ingest.rs`
- `core/src/authority/spawn_tail.rs`
- `core/src/authority/state.rs`
- `core/src/authority/registry.rs`
- `core/src/authority/ingest.rs`
- `core/src/authority/mod.rs`
- `core/tests/authority_spawn_tail.rs`
- `core/tests/authority_registry.rs`
- `core/tests/authority_ingest.rs`
- `core/tests/authority_proptest.rs`
- `core/tests/sessions_ingest.rs`

Add the missing generation-bump correlation without renumbering existing fields:

```proto
message SessionGenerationBumped {
  // existing tags 1..10 unchanged
  TypedCorrelation spawn_origin = 11;
}
```

Replace the ephemeral three-fact/`issued` fold with a durable-action fold. Exact public surface:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnCompletionAction {
    RecordAudit(SpawnCompletionAudit),
    IssueDescendantGrant(DescendantGrantIssuance),
    CommitCompleted(SpawnCompletionCommit),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCompletionAudit {
    pub authority_domain_id: AuthorityDomainId,
    pub spawn_operation_id: CommandId,
    pub completion_source_event_id: EventId,
    pub spawning_grant_id: GrantId,
    pub subject_actor_id: ActorId,
    pub subject_endpoint_id: Option<EndpointId>,
    pub subject_device_id: Option<DeviceId>,
    pub spawned_session_scope: TargetScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescendantGrantIssuance {
    pub spawn_operation_id: CommandId,
    pub spawning_grant_id: GrantId,
    pub spawned_session_scope: TargetScope,
    pub subject_actor_id: ActorId,
    pub subject_endpoint_id: Option<EndpointId>,
    pub authority_domain_id: AuthorityDomainId,
    pub allowed_operation_kinds: Vec<OperationKind>,
    pub descendant_grant_id: GrantId,
    pub created_at: Timestamp,
    pub audit_id: EventId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCompletionCommit {
    pub spawn_operation_id: CommandId,
    pub from_state: OperationState,
    pub correlations: Vec<TypedCorrelation>,
}

impl SpawnDescendantTail {
    pub fn new() -> Self;
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError>;
    pub fn next_action(&self) -> Result<Option<SpawnCompletionAction>, AuthorityError>;
}
```

**Implementation notes:**
- Track, per `(authority_domain_id, command_id)`, the verified accepted spawn, latest non-terminal command state, successful result Observation, correlated session registration/generation bump, qualifying completion audit, observed descendant grant, and any terminal transition.
- A qualifying completion Observation is `ObservationKind::Result` + `FailureCode::Unspecified` with exactly one non-empty command correlation matching an accepted `OperationKind::Spawn`; its target must equal the accepted spawn target. Status/event/delta and failed/rejected results never arm issuance.
- A `SessionRegistered` or `SessionGenerationBumped` fact qualifies only with `spawn_origin = CommandId` and yields the exact runtime-session target tuple. Conflicting targets for one spawn are corrupt log history.
- A qualifying audit is an `AuditRecord` with `kind = CommandCompleted`, `reason_code = "spawn_completion"`, matching command/verified attribution/spawning grant/spawned target, and `source_event_id` equal to the successful result Observation (or the legacy completed transition during repair). Its own event id and `occurred_at` become the grant's `audit_id` and `created_at`.
- An observed `DescendantGrant` is the durable issued checkpoint. It must have the deterministic id, exact provenance, exact target/allowed kinds/subject, and exact audit id. Remove the in-memory `issued: HashSet`; redelivery and restart derive issuance state from the log.
- `next_action` is deterministic across multiple ready spawns: choose the lowest completion-evidence LSN, then command-id value. For one spawn it returns audit first, grant only after the audit is observed, and completion only after the grant is observed. An earlier non-completed terminal transition suppresses all later actions. A legacy completed transition suppresses only the final-commit action while allowing audit/grant repair.
- Require non-empty `spawn_operation_id`, `spawning_grant_id`, and same-domain `audit_id` for descendant grants. Retain the creation audit id in `GrantProvenanceKind::Descendant` so replay/conflicting-duplicate comparison includes it.
- `ingest_descendant_grant` verifies that `audit_id` resolves to the prior same-domain qualifying audit before append. Use `Storage::read_through` for the exact immutable event and the same validation helper used by `AuthorityRegistry` replay; do not trust an arbitrary earlier LSN.
- Populate `SessionGenerationBumped.spawn_origin` from the report in `ingest_session_report`; the session registry otherwise continues to ignore correlation for session identity/state.

**Acceptance criteria:**
- [ ] Registration and generation-bump facts produce the same exact descendant target.
- [ ] All relevant arrival orders converge on the same ordered actions and deterministic grant id.
- [ ] Missing/empty/foreign provenance, wrong audit kind/reason/source/command/actor/endpoint/grant/target, or a forged audit LSN fails before a descendant append.
- [ ] Restart/redelivery sees an observed audit or grant as durable progress and does not request a duplicate.
- [ ] A failed/rejected/cancelled/expired/superseded spawn never requests an audit or descendant grant.
- [ ] Generated Rust and TypeScript bindings are regenerated from `.proto`; generated files are never hand-edited.

### Unit 2: Defer spawn terminalization and execute the crash-safe writer
**Story:** `authority-descendant-grant-completion-crash-safe-writer`

**Files:**
- `core/src/acceptance/observation.rs`
- `core/src/acceptance/index.rs`
- `core/src/acceptance/mod.rs`
- `core/tests/acceptance_observation.rs`
- `core/tests/acceptance_pipeline.rs` (fixture compile updates only)
- `core/tests/acceptance_proptest.rs` (fixture compile updates only)
- `core/tests/authority_proptest.rs` (fixture compile updates plus spawn property evidence)
- `server/src/spawn_completion.rs` (new)
- `server/src/lib.rs`
- `server/Cargo.toml` only if an actually missing dependency is required (none is expected)
- `server/tests/spawn_completion.rs` (new)

Extend the command lookup result and observation outcome:

```rust
pub struct CommandSnapshot {
    pub state: OperationState,
    pub operation_kind: OperationKind,
    pub target_scope: Option<TargetScope>,
    pub correlations: Vec<TypedCorrelation>,
    pub terminal_lsn: Option<u64>,
}

pub enum IngestResult {
    Recorded { event_id: EventId },
    Transitioned { observation_event_id: EventId, transition_event_id: EventId, to_state: OperationState },
    CompletionDeferred { observation_event_id: EventId },
    StaleCandidate { observation_event_id: EventId },
}
```

After the existing boundary/target/current-state checks, `ingest_observation` returns `CompletionDeferred` only for `(operation_kind = Spawn, result success, current state = Delivered | Running)`. It appends the Observation and no terminal transition/audit. `Accepted → Completed` remains invalid, and every non-spawn path retains existing lifecycle behavior.

Add the server owner:

```rust
pub struct SpawnCompletionDriver<S> {
    storage: S,
    authority_domain_id: AuthorityDomainId,
    decision_gate: CoreDecisionGate,
    audit: Arc<dyn AuditSink>,
    clock: Arc<dyn Clock>,
    tail: SpawnDescendantTail,
    authority: AuthorityRegistry,
    cursor: u64,
    scan_interval: Duration,
}

impl<S> SpawnCompletionDriver<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    pub async fn bootstrap(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        decision_gate: CoreDecisionGate,
        audit: Arc<dyn AuditSink>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, SpawnCompletionError>;

    pub async fn catch_up_to_quiescence(&mut self) -> Result<(), SpawnCompletionError>;
    pub async fn run(mut self) -> Result<(), SpawnCompletionError>;
}
```

**Implementation notes:**
- `bootstrap` starts with empty projections/cursor, acquires the shared gate, folds the complete log in LSN order through both `SpawnDescendantTail` and `AuthorityRegistry`, and executes actions until a read returns no new events and `next_action()` is `None`.
- Keep the gate for the entire catch-up/action/catch-up cycle. Never sleep while holding it. The continuous loop sleeps only after a quiescent empty read, then reacquires the gate.
- `RecordAudit`: sample the injected clock once; create the allowlisted `CommandCompleted/spawn_completion` draft with verified actor/endpoint/device, command, spawning grant, spawned target, and completion source; require `AuditReceipt::Durable`.
- `IssueDescendantGrant`: construct the generated `DescendantGrant` with `Continue` policy, no expiry/revocation, canonical kinds, exact provenance, `created_at`, and non-optional audit id; call `ingest_descendant_grant` against the driver's authority projection.
- `CommitCompleted`: raw-append one `CommandTransition { from_state, to_state: Completed, failure_code: Unspecified, correlations }`. Do not call `append_decision` here: the required completion audit is already durable and linked. The raw transition is the last durable step and the only public lifecycle exposure point.
- After every successful append, read/fold from the current cursor instead of mutating tail state optimistically. If append succeeded but response/fold failed, restart recognizes the committed prefix.
- Any malformed fact, impossible lifecycle state, non-durable audit receipt, or storage/authority error terminates the driver with a typed error. It must not skip a spawn and advance the cursor as though completion succeeded.

**Acceptance criteria:**
- [ ] A successful non-spawn result retains its current Observation + transition behavior.
- [ ] A successful spawn result appends evidence only and leaves command state delivered/running until the driver finishes.
- [ ] New spawn log order is successful result/registration (either order), completion audit, descendant grant (plus its normal grant-created audit), then completed transition.
- [ ] `DescendantGrant.audit_id` equals the exact durable completion-audit event id and survives authority replay.
- [ ] A terminal competitor committed before the driver acts prevents grant issuance; if the driver commits first, later terminal candidates remain stale under existing first-terminal rules.
- [ ] Crash-prefix tests starting after audit-only and after audit+grant both repair to one grant and one terminal transition; restart after full completion appends nothing.
- [ ] No control-plane reader using `CoreDecisionGate` can observe the intermediate staged prefix.

### Unit 3: Wire the production composition root and end-to-end evidence
**Story:** `authority-descendant-grant-completion-live-composition`

**Files:**
- `server/src/main.rs`
- `server/src/adapter_service.rs`
- `server/src/adapter_service/tests.rs`
- `server/tests/spawn_completion.rs`
- `server/tests/grpc_smoke.rs` only for a real RPC-boundary case not already covered by the focused integration test

**Composition order:**

```rust
let storage = AuditedStorage::new(RusqliteStorage::open(&database_path)?);
let decision_gate = CoreDecisionGate::default();
let audit: Arc<dyn AuditSink> = Arc::new(RequiredAuditFanout::new(
    Arc::new(DurableAuditSink::new(storage.clone(), authority_domain_id.clone())),
    vec![Arc::new(StderrAuditSink)],
));
let completion = SpawnCompletionDriver::bootstrap(
    storage.clone(),
    authority_domain_id.clone(),
    decision_gate.clone(),
    audit,
    Arc::new(SystemClock),
).await?;
// Build control/admin/adapter services from the repaired final prefix, then
// bind listeners and run `completion.run()` in the same fail-fast join set.
```

**Implementation notes:**
- Bootstrap repair runs before constructing service projections and before either listener binds. Service constructors therefore rebuild from the repaired prefix rather than starting stale.
- The continuous driver is a peer future in the existing `tokio::try_join!`; an unexpected driver exit/error drops the serving futures and fails the process rather than serving without descendant authority.
- Update `AdapterControlServiceImpl::ingest_observation` only to handle `CompletionDeferred` as an observation event id. Do not call the driver from the handler; the shared durable log is the handoff.
- Use the existing 100 ms durable-tail scan cadence as mechanism reuse, not a performance SLA. There is no new timer-based authority or liveness claim.
- Keep startup and live paths storage-port based. No SQLite handle, direct table query, global singleton, or adapter-specific spawn code enters the driver.

**Acceptance criteria:**
- [ ] Production startup repairs legacy or crash-partial spawn completion before the core listeners open.
- [ ] After startup, committed spawn Operation/result/session facts are consumed without another RPC trigger and yield one live descendant grant.
- [ ] Registration and generation-bump end-to-end cases both authorize a subsequent existing-session Operation through the generated descendant grant.
- [ ] Verified actor/endpoint and authorizing grant survive Submit → durable acceptance → restart → issuance; spoofed payload sender does not receive authority.
- [ ] Revoking the parent spawn grant after issuance does not revoke the descendant; revoking the descendant still blocks future session Operations.
- [ ] Whole-workspace tests, generated-contract drift, vector/model metadata checks, and lint/build checks stay green without claiming formal promotion.

## Implementation order
1. `authority-descendant-grant-completion-contract-fold` — land the durable correlation and action state machine.
2. `authority-descendant-grant-completion-crash-safe-writer` — defer terminalization and implement replayable staged completion.
3. `authority-descendant-grant-completion-live-composition` — bootstrap before bind, run the continuous owner, and prove the real path.
4. Review the parent feature as one integrated authority/provenance change using effective review weight `thorough`.

## Child dependency chain

```text
authority-descendant-grant-completion-contract-fold
  → authority-descendant-grant-completion-crash-safe-writer
    → authority-descendant-grant-completion-live-composition
```

All three proposed ids were checked with `.work/bin/work-view --blocking <id>` before the edges were written; no reverse edge or cycle exists. `authority-writer-correctness` remains downstream of this parent and is not absorbed here.

## Simplification
- Remove `SpawnDescendantTail.issued`; durable audit/grant/terminal facts become the only progress checkpoints.
- Reuse `CommandCompleted`, `CoreDecisionGate`, `AuditSink`, `Storage::read_after/read_through`, `AuthorityRegistry`, and `ingest_descendant_grant`; add no spawn-only storage API, event kind, database table, or service callback mesh.
- Keep one production completion driver rather than parallel projection loops in control and adapter services.
- Do not fold general grant check-and-append idempotency into this feature. `authority-writer-correctness` owns the reusable atomic conflict/no-op writer after this live coordination layer lands.
- No test removal is planned initially. During implementation, replace obsolete assertions that expect `audit_id == None` or immediate spawn completion rather than retaining compatibility tests for behavior this feature intentionally removes.

## Testing
- **Core fold tests** protect exact verified provenance, registration/bump equivalence, audit-link validation, deterministic action order, competing terminals, and restart/redelivery. These are the smallest stable boundary for the novel state machine.
- **Observation regression tests** protect the one deliberate lifecycle exception: successful spawn results defer completion while every non-spawn result behaves unchanged.
- **Crash-prefix integration tests** are the highest-value evidence. Seed each durable prefix (evidence only; audit only; audit+grant; full chain), bootstrap a fresh driver, and assert convergence to identical bytes/ids with no duplicate grant or transition.
- **Production-composition integration** uses `AuditedStorage<RusqliteStorage>`, the real shared gate, real generated events, and a restart. It asserts listener-prebind bootstrap ordering through the public bootstrap function rather than timing sleeps.
- **Mutation-sensitive witnesses:** tests must fail if spawn results use generic immediate transition, the completion transition moves before the grant, audit id is dropped/forged, generation-bump correlation is omitted, payload sender is trusted, the driver uses a private gate, or restart appends a duplicate.
- **Formal/conformance honesty:** run existing model/vector checks, but leave `SpawnCreatesDescendantGrant` stated-normative until a genuine independent-attempt model and reviewed vector promotion exist.

## Verification commands

```bash
PATH="$HOME/.npm-global/bin:$PATH" npm --prefix contracts/ts run gen
npm --prefix contracts/ts run build
# Run after staging/committing the intended generated outputs; the check deliberately
# rejects unstaged generated changes before regenerating.
PATH="$HOME/.npm-global/bin:$PATH" npm --prefix contracts/ts run check:drift
cargo test -p patchbay-core --test authority_spawn_tail --test authority_ingest --test authority_registry --test sessions_ingest --test acceptance_observation
cargo test -p patchbay-core-server --test spawn_completion --test grpc_smoke
cargo test --workspace
npm --prefix contracts/ts run check:models
npm --prefix contracts/ts run check:vectors
```

The implementation worker should also run `cargo fmt --all -- --check` and the repository's configured Clippy command if available in the environment.

## Risks
- **Intermediates become externally visible if any read bypasses the shared gate.** The design relies on the existing production decision-gate invariant. Integration tests must race a reader against a barrier-controlled completion and prove it observes either the pre-decision prefix or the final prefix, never audit/grant-only state.
- **A successful result can be durable while completion waits forever for registration.** This is intentional fail-closed behavior: the command remains delivered/running and no descendant authority exists. Operational diagnostics retain the result and missing fact; the driver must not fabricate a target.
- **Legacy data may already expose completed without a grant.** Startup can repair provenance but cannot undo past exposure. The repair path is explicit and idempotent; new writes enforce final-transition-last.
- **A silent driver exit would reopen the original bug.** The runner is joined as a load-bearing process future. Errors fail the core rather than degrade to serving without authority completion.
- **Audit-link validation can become self-referential.** The audit matcher must compare independently decoded durable audit fields and source event identity against the accepted/result/registration facts; it must not accept merely because the grant repeats its own audit id.
- **General descendant writer retry correctness remains downstream.** The gate and durable checkpoints prevent this single driver from duplicating a live completion; generic concurrent `ingest_descendant_grant` conflict/no-op behavior remains explicitly owned by `authority-writer-correctness`.

## Extension pressure classification
- **Committed v0.1.0:** same-actor spawned-session descendant grant; exact canonical existing-session kinds; deterministic grant id; verified actor/optional endpoint; completion audit link; registration and generation-bump correlation; final transition after durable authority; one authoritative core driver.
- **Reserved seams:** delegation/cross-actor lineage, allowed-kind inheritance, cascade revocation, multiple authority domains/cores, and a future storage-native linked transaction if measured pressure justifies it. Existing domain ids, typed provenance, and storage ports preserve these seams.
- **Explicitly rejected for this feature:** implicit grant matching, completing before authority durability, trusting adapter/payload display identity, adding a spawn-only audit enum, or putting spawn payload construction inside SQLite.
- This feature does not foreclose the parked multi-human, desktop, agent-mesh, or skin ideas; it changes no surface or non-operator sender contract.

## Advisory review record
- **Risk:** high — authority creation, durable provenance, terminal-state exposure, restart repair, and a production background owner are cross-cutting security semantics.
- **Design-time advisory:** not dispatched because this delegated endpoint explicitly forbids nested subagents and peer mechanisms. Per the non-blocking design-time policy, the design proceeds from direct foundation/code evidence and records the degradation rather than recursing.
- **Effective implementation/feature review weight:** `thorough` (source: explicit operator selection). Pass unchanged to feature review and final completion review; reviewers propose, and the receiving orchestrator independently adjudicates materiality.

## Implementation notes
- Execution capability: Sol xhigh (explicit autopilot caller selection for security/provenance/live durability); one feature owner carried all three dependency-ordered checkpoints without nested agents, peers, or push.
- Review weight: thorough (explicit caller selection); implementation stops at `stage: review` for the requested fresh review.
- Child checkpoints: `authority-descendant-grant-completion-contract-fold`, `authority-descendant-grant-completion-crash-safe-writer`, and `authority-descendant-grant-completion-live-composition` are each `stage: done` in commits `c2a9a8c`, `bd60460`, and `43a83fe`.
- Files changed: generated session contract and bindings; session bump ingestion; authority fold/registry/ingress/state; spawn observation deferral and command snapshots; production spawn-completion driver/composition; adapter result mapping; focused core/server tests; rolling foundation assertions in `ARCHITECTURE`, `PROTOCOL`, `SECURITY`, and `VERIFICATION`.
- Integrated verification: `cargo test --workspace`; focused core/server spawn, acceptance, authority, session, and gRPC suites; `cargo clippy --workspace --all-targets -- -D warnings`; TypeScript contract build; generated drift; model metadata; and conformance vectors all pass. The original implementation-owned Rust files passed `rustfmt --check --config skip_children=true`.
- Tests added/updated: durable action ordering and arrival-order convergence; exact same-domain immutable audit/source validation; forged/missing provenance rejection; registration/bump equivalence and generated tag carriage; successful-spawn `CompletionDeferred`; crash-prefix repair; non-durable audit failure; gate-hidden intermediate prefix; continuous authenticated adapter consumption; restart idempotence; verified attribution; descendant authorization; and two-lever revocation.
- Simplification: removed the in-memory `issued` truth and generic immediate spawn terminalization; one durable-log owner now derives progress and exposes completion last, without a spawn-specific storage API or handler callback mesh.
- Discrepancies from design/current repo: `SessionGenerationBumped.model` already occupied tag 10, so generated `spawn_origin` correctly uses unique tag 11 without renumbering. Pass 2 promoted the previously only-seamed adapter scope into the committed real v0.1.0 spawn target: an attached adapter now resolves through the OperationKind-aware target boundary and accepts through ordinary `Submit`. Fleet-supervisor/authority-domain default selection remains separate reserved rework; broadcasting a non-idempotent spawn remains excluded. The completion fold retains broader historical/future containment without presenting it as live fleet routing.
- Verification discrepancy: repository-wide `cargo fmt --all -- --check` reports pre-existing formatting drift across many Rust files under the current toolchain. Per ownership constraints those files were not broadly reformatted. Pass-1 changed hunks were kept rustfmt-compatible; `server/src/state.rs` and `server/src/adapter_service/tests.rs` still retain pre-existing whole-file drift, while the other pass-1 Rust files pass the bounded check.
- Protocol/formal status: generated Rust and TypeScript outputs derive from `.proto`, model/vector metadata checks remain green, and `SpawnCreatesDescendantGrant` remains honestly stated-normative with no vector/model promotion.
- Adjacent issues parked: none. Backlog/excluded-item edits were prohibited; the fleet delivery-selector gap is recorded above for receiver/reviewer disposition.

## Review pass 1 remediation

All accepted pass-1 findings were fixed in place; the feature intentionally remains at `stage: review` for re-review.

- **Durable eligibility and ordering:** the completion tail now folds the canonical authority and command projections beside its spawn facts. Issuance requires the exact prior authorizing grant, the verified accepted spawn, and a valid `delivered`/`running` lifecycle before successful result evidence. Accepted-only and pre-acceptance result/session preseeds are inert. Historical completed repair is accepted only when the canonical lifecycle proves a prior eligible state. Full replays and tail catch-up share one gap-free, exact-next-LSN validator.
- **Terminal winner:** `Revocation.command_effects` now enter the same command fold as explicit transitions. Cancellation and require-reauthorization outcomes suppress audit, descendant-grant, and terminal completion actions even when success or a staged audit exists later.
- **Spawn containment:** registration and generation-bump facts must follow the accepted spawn and fall within its scope. Adapter-scoped spawns require the same adapter; cross-adapter facts fail closed. The already-modeled fleet-supervisor and authority-domain scopes retain their explicit broader containment without adding routing behavior.
- **Ingress authority:** descendant-grant ingress replays and validates the complete durable prefix and compares the candidate with the exact issuance derived by the completion fold. A forged but self-consistent source/audit/grant chain without the parent grant, accepted command, lifecycle, result, and contained session context is rejected before append.
- **No duplicate external spawn:** `CommandIndex` records durable deferred successful spawn evidence, and adapter delivery reconstruction suppresses redelivery after reconnect or core restart while descendant completion is pending.
- **Boundary validation:** session report ingress rejects missing, non-command, or empty `spawn_origin` references before any registration/bump append.
- **Public completion honesty:** the staged durable audit is no longer sent through the immediate stderr completion fanout. Production emits the redacted `spawn completion finalized` diagnostic only after the descendant grant and final transition are durable.
- **Real-path evidence at the committed boundary:** pass 2 replaces the seeded adapter handoff with ordinary authenticated `Submit` against an attached adapter, then exercises delivery, acknowledgement, duplicate-qualified successful result, session report, background completion, SQLite restart, and a subsequent real control `Submit` authorized by the replayed descendant grant. Runtime-session and resource spawn targets reject before durable acceptance; fleet-default selection remains reserved and no broadcast path exists.
- **Coverage added:** accepted-only/preseed negatives; cancellation and require-reauthorization winner cases; same-adapter and cross-adapter registration containment; forged durable-context rejection; malformed `spawn_origin`; restart/reconnect redelivery suppression; crash-prefix repair; and real adapter/control restart authorization.
- **Pass-1 verification:** focused core authority/session/acceptance suites; focused server adapter/spawn/gRPC suites; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; TypeScript contract build and generated drift; model metadata; and conformance vectors all pass. Repository-wide rustfmt remains blocked only by the pre-existing drift recorded above.
- **Scope discipline:** no fleet delivery selector, adapter broadcast, backlog/peer item, or nested work was added.

## Review pass 2 remediation

All receiver-accepted pass-2 blockers were fixed against the current atomic authority writer. The feature remains at `stage: review` for the caller's thorough convergence pass.

- **One real OperationKind-aware spawn boundary:** `TargetResolver` now receives the generated `OperationKind`. The composite target registry folds durable adapter registrations and admits `spawn` only for one canonical attached-adapter scope, returning a typed adapter binding. Runtime-session, operational-resource, mixed adapter, unattached adapter, fleet-supervisor, and authority-domain spawn targets fail before a command append. Existing-session/resource resolution remains available to non-spawn kinds.
- **Real attached-adapter lifecycle:** the former seeded E2E now uses authenticated ordinary `Submit` with an adapter-scoped spawn grant, receives the delivery through the attached adapter stream, acknowledges it, reports success plus registration, waits for audit → descendant grant → terminal completion, restarts SQLite-backed projections, and proves a subsequent real existing-session `Submit` selects the replayed descendant grant. The same test proves incompatible runtime/resource spawns have no durable Operation record.
- **Correlation qualification single source:** observation ingestion exports one exact-command qualifier. Identical non-empty duplicate command references collapse to one logical correlation; empty or conflicting ids do not qualify. `CommandIndex`, `SpawnDescendantTail`, `AuthorityRegistry` completion-source indexing, and descendant audit-source validation all consume that helper. Fresh restart projections over a duplicate-correlated prefix prove redelivery suppression and completion eligibility agree.
- **Deferred evidence diagnostics:** successful spawn evidence atomically appends a bounded/redacted `CommandRunning` audit record with reason `spawn_completion_deferred` and a source-event link. `inspect-command` exposes it through its existing bounded audit page while canonical state remains `delivered`/`running` with no terminal event. The allowlisted audit draft carries no result payload, prompt, arbitrary metadata, or adapter diagnostic body.
- **Foundation honesty:** `SPEC`, `ARCHITECTURE`, `PROTOCOL`, `SECURITY`, `VERIFICATION`, and `GLOSSARY` now classify explicit adapter-scoped spawn resolution as committed v0.1.0; deterministic fleet-default selection is reserved, and spawn broadcast is explicitly rejected for this path. Draft `FleetAuthorityForSpawn` vocabulary is retained only as future selector work, not a current assurance claim.
- **Pass-2 reviewer disposition:** the three supplied findings were receiver-confirmed material blockers and fixed. No lower-risk follow-up was parked because backlog/other-item changes are excluded. No independent pass was dispatched from this endpoint because nested/peer review is explicitly prohibited; this is the requested fix endpoint, not the subsequent clean convergence pass.
- **Pass-2 verification:** focused core acceptance/authority/resource/session suites and focused server spawn/gRPC suites pass; `cargo test --workspace` passes; `cargo clippy --workspace --all-targets -- -D warnings` passes; the TypeScript contract build, generated drift regeneration check, model metadata check, and full conformance-vector runner pass; and the complete Pi-adapter suite (including the real-process E2E) passes after building the core binary at its expected test path. `git diff --check` passes. Repository-wide `cargo fmt --all -- --check` still reports the pre-existing whole-repo formatting drift already recorded above; the same affected files fail from `HEAD` under the current rustfmt, so this fix does not broaden the diff to reformat unrelated code.

## Review pass 3 remediation

The receiver-accepted registration-ingress/replay blocker is fixed; the feature remains at `stage: review` for the next thorough convergence pass.

- **Blocker disposition and rationale:** `Attach` is now the sole authenticated RPC producer for durable adapter registration. Generic authenticated Event ingress rejects the shared registration schema after current-attachment/domain/target checks and before generic Observation ingestion can append. Replay compares registration-schema Observations with the complete canonical durable envelope built by the actual attachment producer, covering event/Observation domain, canonical adapter target, canonical adapter actor plus endpoint sender, Observation kind, protobuf schema/content type, embedded adapter/domain/endpoint/generation/capability identity, and attachment redaction. This keeps one producer-derived contract rather than reinterpreting arbitrary Observations.
- **Replay integrity:** `AdapterRegistry::rebuild_from_log` still uses `core/src/storage/prefix.rs::validate_next_replay_event` for the complete gap-free prefix. The registration-envelope validator is only the projection-owned content fold; it neither replaces nor duplicates LSN-prefix responsibility. The legacy category-less/resource-empty manifest exception remains available only inside a canonical envelope.
- **Regression evidence:** the core mutation table preserves one valid producer-generated replay and rejects every named envelope dimension before registry mutation. The authenticated adapter-A/canonical-target-A/embedded-adapter-B server regression proves the RPC appends nothing, A remains generation 1, B has no generation/registry entry, A remains spawn-resolvable after replay, and B is not spawn-resolvable.
- **Standing semantics:** `ARCHITECTURE`, `PROTOCOL`, and `SECURITY` now distinguish durable replayed registration/routing identity from process-local attachment tokens and live delivery subscriptions. Restart replay keeps a registered adapter eligible for explicit spawn resolution; actual delivery continues to wait or fail through existing adapter delivery behavior. `VERIFICATION` records the implementation evidence without formal/vector promotion.
- **Verification:** isolated-target `cargo test -p patchbay-core --test adapter_capability --test resource_resolver --test replay_integrity` (18 passed); isolated-target `cargo test -p patchbay-core-server --lib adapter_service::tests` (25 passed); `cargo test --workspace` (passed); `cargo clippy --workspace --all-targets -- -D warnings` (passed); and the full `npm --prefix contracts/ts run check:vectors` runner (53 vectors, 16 promoted vectors, 21 implementation checks, and 37 killed mutation witnesses) passed after installing the repository's pinned npm dependencies and building the TypeScript contracts. `git diff --check` passed.
- **Scope discipline:** no attachment-token persistence, new delivery/liveness state, fleet/default selection, routing broadcast, generated-contract change, other substrate item, backlog entry, nested agent, or peer artifact was added.

## Review closure — pass 4

Fresh-context pass 4 approved with no material, important, or nit findings. All prior lifecycle, provenance, target, correlation, redelivery, registration-ingress, restart, and atomic-writer blockers were rechecked and did not recur. Effective weight: `thorough` (explicit operator). Verdict: approved.
