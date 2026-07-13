---
id: feature-v0-core-acceptance
kind: feature
stage: review
tags: [protocol, verification, foundation]
parent: epic-v0-core
depends_on: [feature-v0-core-persistence]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-13
---

# Feature: Operation acceptance and command lifecycle

## Brief

Build the operation acceptance pipeline and command lifecycle state machine. A command accepted by Patchbay is durably recorded before delivery; after acceptance it remains visible as a `CommandState` until and after it reaches a terminal state. Acceptance creates a command record only after boundary validation, authority checking (via a grant-check port owned by the authority feature), idempotency reconciliation, and target identity binding (via a session-registry port owned by the sessions feature).

This feature owns the `CommandState` lifecycle (accepted → delivered → ... → terminal), idempotency-key dedup at the boundary, payload-equivalence checking, terminal-race resolution (first durable terminal commit wins via LSN ordering), and the failure vocabulary. It also owns observation ingestion — how adapter-reported Observations (output/events/status/terminal candidates) are written to the event log and reflected in command state. Elicitation lifecycle handling folds into this feature as part of the operation/observation/elicitations plane; if the scope is too large, `feature-design` may spawn a child story for elicitation specifically.

## Epic context

- Parent epic: `epic-v0-core`
- Position in epic: depends on persistence (for the event log and LSN). Interacts with authority (grant-check port) and sessions (target-identity port) through Ports & Adapters — those features implement ports this feature defines, so they can proceed in parallel.

## Formal-model backing

- `TerminalFinality` (promoted, `command_lifecycle.qnt`) — once a command reaches a terminal CommandState, later events do not mutate it
- `NoAcceptedToCompleted` (promoted, `command_lifecycle.qnt`) — a command cannot transition directly from `accepted` to `completed`; it must pass through `delivered` (or `running`)
- `BoundaryDedup` (promoted, `command_lifecycle.qnt`) — shared with persistence; the dedup boundary this feature enforces
- Idempotent retry, terminal races, session-identity binding — stated-normative obligations (v1 formal gate owns the real properties)

## Foundation references

- `docs/PROTOCOL.md` — Command lifecycle state; OperationKind registry; Submission outcome and local submission state; Acceptance semantics; Idempotency and retry; Cancellation, expiration, supersession, and race semantics; Failure and outcome vocabulary
- `docs/ARCHITECTURE.md` — Operation plane; Operation, Observation, and Elicitation plane
- `docs/VERIFICATION.md` — `TerminalFinality`, `NoAcceptedToCompleted`, `BoundaryDedup` promoted properties
- `contracts/proto/patchbay/operations.proto` — `Operation`, `OperationKind`, `OperationState`, `SubmissionOutcome`, `SubmissionResult`, `FailureCode`
- `contracts/proto/patchbay/observations.proto` — `Observation`, `ObservationKind`
- `contracts/proto/patchbay/elicitations.proto` — `Elicitation`, `ElicitationState`, `ResponseContract`
- `specs/seed/command_lifecycle.qnt` — `state`, `idemKey`, `appliedKeys`, `applyCount`, `lsn`, `terminalLsn`
- `specs/seed/elicitation_lifecycle.qnt` — stated-normative elicitation obligations

## Design decisions (feature-design, 2026-07-12)

Resolved interactively with the operator after unpacking each option's trade-offs.

- **Q1 — Command state model: hybrid (event log is SSOT, in-memory index is the hot path).** Chosen over fully-derived (every `get_command` needs a warm index) and stored-projection (introduces a second source of truth for command state). The event log is the single source of truth; an in-memory `CommandState` index is the hot lookup path, rebuilt from replay on startup and snapshot-checkpointed. This matches how the persistence feature was built (recovery returns raw materials; the domain layer applies them).
- **Q1b — Event payload shape: uniform `CommandTransition` events for all transitions; accept reuses `OPERATION`.** Chosen over deriving transitions from the Observation stream (breaks on core-sourced transitions: `delivered`, `expired`, `cancelled`, `superseded` — no adapter Observation exists for these) and over a single mutable record per command (the log is append-only; a full-snapshot-per-change obscures transition semantics and wastes bytes). Each real transition (advance/commitTerminal in the formal model) is one `COMMAND_TRANSITION` event consuming an LSN. Late terminal candidates are audit-only `stale_event` Observations that never become transition events — making first-durable-terminal-wins structural: the first terminal `COMMAND_TRANSITION` in LSN order wins; later candidates are rejected before becoming transitions. This maps 1:1 to the formal model's `advance`/`commitTerminal` (LSN-consuming) vs `lateTerminalCandidate` (no-op). Added `CommandTransition` proto message + `STORED_EVENT_KIND_COMMAND_TRANSITION = 8` variant (Generated Contracts: schema owns the variant set).
- **Q2 — Port shapes: two separate ports, both async (RPITIT, static dispatch).** `GrantCheck` (authority: pure read, no side effects, the audit record IS the acceptance event) and `TargetResolver` (sessions: validation + binding). Chosen over a bundled `AcceptanceContext` (bundles security with routing — two unrelated concerns with different change drivers, violating interface segregation). Async chosen over sync (even though authority state is in-memory in v0.1.0) for consistency with `Storage` and forward-compatibility if authority ever needs I/O. Consistency > cheap.
- **Q3 — Observation ingestion scope: acceptance owns ingestion into the log + command-state reflection; streaming/subscription/cursor/fan-out is the protocol-seam.** Chosen over owning the full observation path including subscriptions (delivery-shaped — fan-out to web surfaces, cursor management, SSE/WS framing — belongs at the network boundary) and over owning only terminal-candidate reflection (leaves non-terminal Observations with no owner). Ingestion is a separate method on the acceptance service (distinct adapter→core ingress) but not a separate trait — shared owner, distinct entry point.
- **Q4 — Elicitation scope: Option A2 — acceptance accepts response Operations as plain operations; the Elicitation-slot terminalization is an independent event-log consumer (child story).** Chosen over Option C (defer all elicitation — ruled out: SPEC commits `approval-response`/`elicitation-response` as v0.1.0 OperationKinds and Elicitations in v0.1.0) and Option B (acceptance owns the full response→slot path — two state machines with different registries/properties/shapes bundled into one feature). A2 keeps acceptance ignorant of `ElicitationState` (cleaner Ports & Adapters): the slot layer tails the log, sees response terminal `COMMAND_TRANSITION` events, and terminalizes the slot with its own events. First-answer-wins is structurally identical to the command terminal race — first response terminal event in LSN order wins the slot — and decoupled from acceptance.

## Architectural choice

A hybrid event-sourced acceptance pipeline: the event log (owned by `feature-v0-core-persistence`) is the single source of truth for command state. Acceptance writes `OPERATION` events (accept) and `COMMAND_TRANSITION` events (each real state transition) through the `Storage::append_dedup` / `Storage::append` ports. An in-memory `CommandState` index is the hot lookup path, rebuilt from replay on startup and snapshot-checkpointed. The formal model's `advance`/`commitTerminal` are real `COMMAND_TRANSITION` events consuming an LSN; `lateTerminalCandidate` is a no-op realized as an audit-only `stale_event` Observation that never becomes a transition event.

Two Ports & Adapters seams (async, RPITIT) decouple acceptance from its sibling features:

- `GrantCheck` (implemented by the authority feature) — `async fn check(&self, actor, operation_kind, target_scope) -> Result<Authorized, GrantDenied>`. Pure read; the audit record IS the acceptance event.
- `TargetResolver` (implemented by the sessions feature) — `async fn resolve(&self, target_scope) -> Result<TargetBinding, TargetNotFound>`. Validation + binding to a concrete session handle for delivery.

Observation ingestion is a separate method on the acceptance service (distinct adapter→core ingress) but not a separate trait. Elicitation-slot terminalization is an independent event-log consumer spawned as a child story (A2): acceptance accepts `approval-response`/`elicitation-response` Operations as plain operations; the Elicitation layer tails the log, sees response terminal transitions, and terminalizes the slot with its own events.

This shape honors Ports & Adapters (domain logic depends on `Storage`, `GrantCheck`, `TargetResolver` traits — not on authority/sessions implementations), Single Source of Truth (the event log is the only source of command state; the in-memory index is a pure fold), Generated Contracts (`CommandTransition` is a generated proto message; `StoredEventKind` is the schema-owned variant set), and Fail Fast (invalid transitions rejected at the boundary; log-corruption detected on replay).

## Implementation Units

### Unit 1: Command state machine and transition validation

**File**: `core/src/acceptance/state.rs`, `core/src/acceptance/transitions.rs`

**Story**: `story-v0-core-acceptance-state-machine`

```rust
// core/src/acceptance/transitions.rs
use patchbay_contracts::patchbay::{CommandId, OperationState, FailureCode};

/// The canonical CommandState transition adjacency from docs/PROTOCOL.md.
/// This is the single source of truth for allowed transitions — derived
/// from the protocol, not invented. Mirrors command_lifecycle.qnt's
/// `allowedTransition`.
pub fn allowed_transition(from: OperationState, to: OperationState) -> bool {
    use OperationState::*;
    match from {
        Accepted => matches!(to, Delivered | Rejected | Failed | Expired | Cancelled | Superseded),
        Delivered => matches!(to, Running | Completed | Rejected | Failed | Expired | Cancelled | Superseded),
        Running => matches!(to, Completed | Failed | Expired | Cancelled | Superseded),
        // Terminal states are final — no transitions out (TerminalFinality).
        _ => false,
    }
}

/// The in-memory command record, derived from the event log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRecord {
    pub command_id: CommandId,
    pub operation: Operation,          // the accepted Operation message
    pub state: OperationState,
    pub terminal_lsn: Option<u64>,     // Some(LSN) once terminal
    pub failure_code: Option<FailureCode>,
}

/// Apply a transition event to a command record. Returns Err on corruption
/// (Fail Fast: a transition whose from_state doesn't match the current
/// state, or a transition out of a terminal state).
pub fn apply_transition(
    record: &mut CommandRecord,
    transition: &CommandTransition,
) -> Result<(), AcceptanceError> {
    if record.state.is_terminal() {
        return Err(AcceptanceError::CorruptLog(format!(
            "transition for already-terminal command {:?}", record.command_id
        )));
    }
    if transition.from_state != record.state {
        return Err(AcceptanceError::CorruptLog(format!(
            "from_state mismatch: log says {:?}, memory says {:?}",
            transition.from_state, record.state
        )));
    }
    if !allowed_transition(record.state, transition.to_state) {
        return Err(AcceptanceError::CorruptLog(format!(
            "disallowed transition {:?} -> {:?}", record.state, transition.to_state
        )));
    }
    record.state = transition.to_state;
    if transition.to_state.is_terminal() {
        record.terminal_lsn = Some(/* the event's LSN */);
        record.failure_code = if transition.failure_code != FailureCode::Unspecified {
            Some(transition.failure_code)
        } else { None };
    }
    Ok(())
}
```

**Implementation Notes**:
- `allowed_transition` is the SSOT for the transition adjacency — derived directly from `docs/PROTOCOL.md` § Command lifecycle state, mirroring `command_lifecycle.qnt`'s `allowedTransition`. Both the acceptance pipeline and the replay apply function call it.
- `apply_transition` enforces the three promoted properties by construction: `TerminalFinality` (terminal states reject transitions out), `NoAcceptedToCompleted` (accepted→completed is not in the adjacency), and the adjacency itself. The formal model *checks* these; this code *enforces* them. A transition that violates the adjacency is `CorruptLog` (Fail Fast), not silently applied.
- The `from_state` field on `CommandTransition` exists for replay validation — applying a transition whose recorded `from_state` doesn't match the current in-memory state indicates the log was tampered or corrupted.

**Acceptance Criteria**:
- [ ] `allowed_transition` matches the protocol table for all 9 states × 9 states.
- [ ] `apply_transition` rejects transitions out of terminal states (TerminalFinality).
- [ ] `apply_transition` rejects accepted→completed (NoAcceptedToCompleted).
- [ ] `apply_transition` rejects from_state mismatches (corruption detection).
- [ ] `is_terminal()` correctly identifies the 6 terminal states.

---

### Unit 2: Acceptance pipeline and the three ports

**File**: `core/src/acceptance/mod.rs`, `core/src/acceptance/ports.rs`, `core/src/acceptance/pipeline.rs`

**Story**: `story-v0-core-acceptance-pipeline`

```rust
// core/src/acceptance/ports.rs
use patchbay_contracts::patchbay::{
    ActorEndpointRef, AuthorityDomainId, CommandId, OperationKind, TargetScope, RuntimeSessionId, Generation,
};
use crate::storage::{EventId, StorageError, TargetKey};

/// The authority seam. Implemented by the authority feature. Pure read —
/// no side effects; the audit record IS the acceptance event.
pub trait GrantCheck: Send + Sync {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        actor: &ActorEndpointRef,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied>;
}

/// The sessions seam. Implemented by the sessions feature. Validation +
/// binding — does the target exist, return a concrete handle for delivery.
pub trait TargetResolver: Send + Sync {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> Result<TargetBinding, TargetNotFound>;
}

#[derive(Debug, Clone)]
pub struct Authorized {
    pub grant_id: Option<GrantId>,  // None for implicit operator authority
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GrantDenied {
    #[error("no grant for {actor} to {kind:?} on {target}")]
    NoGrant { actor: String, kind: OperationKind, target: String },
}

#[derive(Debug, Clone)]
pub struct TargetBinding {
    pub runtime_session_id: RuntimeSessionId,
    pub session_generation: Generation,
    pub adapter_id: AdapterId,
}
```

```rust
// core/src/acceptance/pipeline.rs
/// Submit an operation for acceptance. The pipeline:
/// 1. Boundary validation (OperationKind known, fields present, payload valid)
/// 2. Authority check (GrantCheck port)
/// 3. Target identity binding (TargetResolver port)
/// 4. Idempotency reconciliation (Storage::append_dedup)
/// 5. Durably record the OPERATION event (if new)
/// 6. Return the command record + state
///
/// Pre-acceptance failure (steps 1-3) returns SubmissionOutcome = rejected
/// without creating durable state. Acceptance (step 4-5) creates the
/// command record at CommandState = accepted.
pub async fn submit<S, G, R>(
    storage: &S,
    grant_check: &G,
    target_resolver: &R,
    operation: Operation,
) -> Result<SubmissionResult, AcceptanceError>
where
    S: Storage,
    G: GrantCheck,
    R: TargetResolver,
{
    // 1. Boundary validation.
    if let Err(code) = validate_operation(&operation) {
        return Ok(SubmissionResult::rejected(code));
    }
    // 2. Authority check.
    match grant_check.check(&operation.authority_domain_id, &operation.sender, operation.kind, &operation.target_scope).await {
        Ok(_auth) => {},
        Err(GrantDenied::NoGrant { .. }) => {
            return Ok(SubmissionResult::rejected(FailureCode::AuthorizationDenied));
        }
    }
    // 3. Target identity binding.
    if let Err(_nf) = target_resolver.resolve(&operation.authority_domain_id, &operation.target_scope).await {
        return Ok(SubmissionResult::rejected(FailureCode::TargetNotFound));
    }
    // 4-5. Idempotency reconciliation + durable record.
    let payload = StoredEventPayload { kind: StoredEventKind::Operation as i32, payload: encode(&operation) };
    let target_key = TargetKey::new(target_key_for(&operation))?;
    let idem_key = IdempotencyKey { value: operation.idempotency_key.clone() };
    match storage.append_dedup(&operation.authority_domain_id, &idem_key, &target_key, payload).await? {
        DedupOutcome::Appended(event_id) => {
            // New command — state is accepted.
            Ok(SubmissionResult::accepted(operation.command_id, event_id))
        }
        DedupOutcome::Duplicate(event_id) => {
            // Retry — return existing record.
            Ok(SubmissionResult::accepted_dedup(operation.command_id, event_id))
        }
    }
}
```

**Implementation Notes**:
- The pipeline order (validate → authorize → resolve-target → dedup) matches `docs/PROTOCOL.md` § Acceptance semantics. Pre-acceptance failure is `SubmissionOutcome = rejected` with no durable state.
- `append_dedup` is the atomic boundary: the `OPERATION` payload is byte-stable across retries (the `Operation` proto), so the dedup payload-equivalence check works. A retry with a differing payload returns `IdempotencyConflict` → `validation_failed`.
- The `target_key_for` projection (Operation → TargetKey) is the canonicalization the persistence feature flagged as a reserved seam. The acceptance feature owns this projection: it's `Operation.target_scope` serialized canonically. This is where `TargetScope` → `TargetKey` binding happens.
- A `CommandTransition` (advance/commit) is a separate `Storage::append` (not dedup'd — transitions aren't retries). The in-memory state check guards against duplicate transitions (e.g. an adapter reporting `running` twice applies once; the second is a no-op or `stale_event`).

**Acceptance Criteria**:
- [ ] `submit` rejects unknown OperationKind with `validation_failed` (pre-grant).
- [ ] `submit` rejects unauthorized actor with `authorization_denied` (pre-acceptance, no durable state).
- [ ] `submit` rejects unknown target with `target_not_found` (pre-acceptance, no durable state).
- [ ] `submit` durably records the OPERATION event and returns `accepted` for a new command.
- [ ] `submit` returns the existing record for a retry (same command id + idempotency key + identical payload).
- [ ] `submit` rejects a differing-payload retry with `validation_failed`.
- [ ] Pre-acceptance failures create no durable command state.

---

### Unit 3: Observation ingestion and command-state reflection

**File**: `core/src/acceptance/observation.rs`

**Story**: `story-v0-core-acceptance-observation-ingestion`

```rust
// core/src/acceptance/observation.rs
/// Ingest an adapter-reported Observation. This is the adapter→core ingress,
/// distinct from the operator-submission path (Unit 2). The method:
/// 1. Validates the Observation is source-authenticated (adapter channel).
/// 2. Durably records the OBSERVATION event.
/// 3. If the Observation implies a command transition (status→running,
///    result→terminal), emits a COMMAND_TRANSITION event and updates the
///    in-memory index.
/// 4. Late terminal candidates (command already terminal) are recorded as
///    stale_event Observations — NOT transition events.
pub async fn ingest_observation<S: Storage>(
    storage: &S,
    observation: Observation,
) -> Result<IngestResult, AcceptanceError> {
    // Durably record the raw Observation.
    let event_id = storage.append(
        &observation.authority_domain_id,
        StoredEventPayload { kind: StoredEventKind::Observation as i32, payload: encode(&observation) },
    ).await?;

    // Determine if this Observation implies a transition.
    let transition = derive_transition(&observation);
    match transition {
        Some((cmd_id, to_state, failure_code)) => {
            // Check in-memory state: is the command already terminal?
            // If yes, this is a late candidate → stale_event audit only.
            // If no, emit a COMMAND_TRANSITION event.
            // (The in-memory check is the first-durable-terminal-wins guard.)
            ...
        }
        None => {
            // Non-transition Observation (event, delta, non-terminal status).
            // Recorded above; no transition emitted.
        }
    }
}
```

**Implementation Notes**:
- Ingestion is a separate method on the acceptance service (distinct ingress), not a separate trait. The adapter calls `ingest_observation`; the operator calls `submit`.
- `derive_transition` is the mapping from `ObservationKind`/`failure_code` to a candidate transition: `result` + no failure → `completed`; `result` + failure → `failed`/`execution_outcome_unknown`; `status` → `running`; etc. This is the ONE place Observations map to transitions — but the transition is emitted as a first-class `COMMAND_TRANSITION` event (not derived at replay), so the log is self-describing.
- Late terminal candidates (command already terminal) are `stale_event` audit Observations — they do NOT become `COMMAND_TRANSITION` events. This is the structural realization of `lateTerminalCandidate` (the model's no-op) and what makes `TerminalFinality` honest.
- The streaming/subscription/cursor/fan-out layer is NOT in this unit — that's the protocol-seam. Acceptance writes the event + updates the index; subscription tails the log.

**Acceptance Criteria**:
- [ ] `ingest_observation` durably records the OBSERVATION event for all kinds.
- [ ] A `result` Observation with no failure emits a `completed` transition.
- [ ] A `result` Observation with a failure emits the appropriate terminal transition.
- [ ] A late terminal candidate (command already terminal) is recorded as `stale_event`, NOT a transition.
- [ ] Non-transition Observations (event, delta) record without emitting a transition.

---

### Unit 4: Replay and in-memory index reconstruction

**File**: `core/src/acceptance/replay.rs`, `core/src/acceptance/index.rs`

**Story**: `story-v0-core-acceptance-replay`

```rust
// core/src/acceptance/index.rs
/// The in-memory command index — the hot lookup path. Rebuilt from replay
/// on startup; snapshot-checkpointed to bound recovery cost.
pub struct CommandIndex {
    commands: HashMap<CommandId, CommandRecord>,
    // idempotency-key → command-id for retry lookup (mirrors appliedKeys).
    key_to_command: HashMap<(AuthorityDomainId, String, String), CommandId>,
}

impl CommandIndex {
    /// Apply an event during replay. This is the domain-layer fold the
    /// persistence recovery module hands off to.
    pub fn apply(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        match event.payload.kind() {
            StoredEventKind::Operation => {
                let op: Operation = decode(&event.payload.payload)?;
                let id = op.command_id.clone();
                self.commands.insert(id.clone(), CommandRecord::new(op, event.event_id.lsn));
                self.key_to_command.insert(...);
            }
            StoredEventKind::CommandTransition => {
                let t: CommandTransition = decode(&event.payload.payload)?;
                let record = self.commands.get_mut(&t.command_id)
                    .ok_or(AcceptanceError::CorruptLog("transition for unknown command"))?;
                apply_transition(record, &t, event.event_id.lsn)?;
            }
            StoredEventKind::Observation => {
                // Observations are evidence; they don't directly mutate
                // command state (transitions are separate events). No-op
                // for the index.
            }
            _ => {}  // authority/sessions events — not ours.
        }
    }
}
```

**Implementation Notes**:
- This is the domain-layer `apply` that `IdempotentLogReplay` (stated-normative) depends on. The persistence feature's recovery returns raw materials; this fold consumes them. Deterministic `apply` is what makes the full `IdempotentLogReplay` property hold end-to-end.
- The fold is a pure function of the event sequence — same events in, same index out. This is the determinism the persistence proptest `replay_deterministic_for_unchanged_contents` depends on (it tested the storage-layer portion; this completes it).
- Snapshot checkpointing: the index serializes to the snapshot payload via the persistence `write_snapshot` port. The snapshot bounds replay to the tail (the storage-layer portion, already tested).

**Acceptance Criteria**:
- [ ] Replaying `OPERATION` + `COMMAND_TRANSITION` events reconstructs the full command index.
- [ ] Replay is deterministic: same events → same index.
- [ ] Replay rejects a `COMMAND_TRANSITION` for an unknown command (CorruptLog).
- [ ] Replay rejects a transition whose `from_state` mismatches (CorruptLog).
- [ ] The index lookup (`get_command`) is O(1).

---

### Unit 5: Elicitation-slot terminalization (child story, A2)

**File**: `core/src/acceptance/elicitation.rs`

**Story**: `story-v0-core-acceptance-elicitation-slot`

```rust
// core/src/acceptance/elicitation.rs
/// The Elicitation-slot layer. Independent event-log consumer (A2):
/// acceptance accepts response Operations as plain operations; this layer
/// tails the log, sees response terminal COMMAND_TRANSITION events, and
/// terminalizes the Elicitation slot with its own events.
///
/// First-answer-wins is structural: the first response terminal event in
/// LSN order wins the slot; later response attempts are rejected as
/// already-terminal/stale.
pub struct ElicitationSlotLayer<S: Storage> {
    storage: S,
    // in-memory slot state, rebuilt from replay
    slots: HashMap<ElicitationId, ElicitationRecord>,
}

impl<S: Storage> ElicitationSlotLayer<S> {
    /// Observe a committed event. Called by the replay/recovery path and
    /// by the live event tail. This is the log-consumer seam (A2).
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        match event.payload.kind() {
            StoredEventKind::Elicitation => {
                // A new Elicitation was opened (by an adapter). Record it.
                ...
            }
            StoredEventKind::CommandTransition => {
                let t: CommandTransition = decode(&event.payload.payload)?;
                // Is this a response Operation going terminal?
                if let Some(elicitation_id) = correlation_to_elicitation(&t.correlations) {
                    if t.to_state.is_terminal() {
                        self.terminalize_slot(&elicitation_id, &t, event.event_id.lsn)?;
                    }
                }
            }
            _ => {}
        }
    }
}
```

**Implementation Notes**:
- This is the A2 decoupling: acceptance knows nothing about `ElicitationState`. The slot layer is an independent consumer of the event log. It sees response `COMMAND_TRANSITION` events (via `correlations` → `ElicitationId`) and terminalizes the slot.
- First-answer-wins is "first response terminal event in LSN order" — structurally identical to the command terminal race, and decoupled from acceptance's own terminal-race logic.
- The Elicitation layer has its own state machine (`ElicitationState`: opened/pending/answered/declined/expired/cancelled/withdrawn/superseded/stale) and its own stated-normative properties (`ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, etc.). Those are this story's concern, not acceptance's.
- Spawned as a child story because it's a distinct state machine with its own formal-model backing — but it depends on the acceptance pipeline existing (response Ops must be accepted first).

**Acceptance Criteria**:
- [ ] The slot layer terminalizes an Elicitation slot when a response Operation reaches terminal.
- [ ] First-answer-wins: the first response terminal event wins; later responses are `stale`.
- [ ] The slot layer is decoupled from acceptance (no direct call; observes the log).
- [ ] The slot layer reconstructs slot state from replay.

---

### Unit 6: Property tests for acceptance invariants

**File**: `core/tests/acceptance_proptest.rs`

**Story**: `story-v0-core-acceptance-proptests`

```rust
// core/tests/acceptance_proptest.rs
proptest! {
    /// TerminalFinality: once terminal, no transition mutates the command.
    /// Mutant: a transition out of a terminal state must be rejected.
    #[test]
    fn terminal_state_rejects_further_transitions(...) { ... }

    /// NoAcceptedToCompleted: accepted→completed is never a direct transition.
    #[test]
    fn accepted_to_completed_is_rejected(...) { ... }

    /// BoundaryDedup: retrying the same idempotency key returns the existing
    /// record; no double-apply.
    #[test]
    fn retry_returns_existing_no_double_apply(...) { ... }

    /// First-durable-terminal-wins: the first terminal COMMAND_TRANSITION in
    /// LSN order wins; a later candidate is a stale_event, not a transition.
    #[test]
    fn first_terminal_wins_later_is_stale(...) { ... }

    /// Replay determinism: same events → same command index.
    #[test]
    fn replay_reconstructs_identical_index(...) { ... }

    /// Mutation discipline: each property catches its named bug.
    #[test]
    fn terminal_finality_catches_injected_bug(...) { ... }
}
```

**Acceptance Criteria**:
- [ ] All promoted-property proptests pass (TerminalFinality, NoAcceptedToCompleted, BoundaryDedup).
- [ ] First-durable-terminal-wins proptest passes.
- [ ] Replay determinism proptest passes.
- [ ] Mutation tests prove non-vacuity for each property.

## Implementation Order

1. `story-v0-core-acceptance-state-machine` — transition adjacency + apply (no deps; the SSOT for allowed transitions)
2. `story-v0-core-acceptance-pipeline` — submit pipeline + the three ports (depends on 1 + persistence)
3. `story-v0-core-acceptance-observation-ingestion` — observation → event + transition (depends on 1, 2)
4. `story-v0-core-acceptance-replay` — in-memory index fold + snapshot checkpoint (depends on 1, 2)
5. `story-v0-core-acceptance-elicitation-slot` — A2 elicitation layer (depends on 1, 2, 4)
6. `story-v0-core-acceptance-proptests` — property tests for all promoted + stated-normative obligations (depends on 1-5)

Stories 1-2 are sequential (the pipeline needs the state machine). Story 3 and 4 can proceed in parallel once 2 lands (ingestion and replay both consume the state machine + ports). Story 5 depends on 4 (it observes the log the replay produces). Story 6 depends on all.

## Testing

### Unit Tests: `core/tests/acceptance_proptest.rs`
- TerminalFinality (terminal states reject transitions out)
- NoAcceptedToCompleted (accepted→completed rejected)
- BoundaryDedup (retry returns existing, no double-apply)
- First-durable-terminal-wins (first terminal in LSN order wins; later candidates are stale)
- Replay determinism (same events → same index)
- Mutation discipline (each property catches its named bug)

### Integration Points
- The `Storage` trait (persistence) is the event log the pipeline writes through.
- The `GrantCheck` port (authority feature implements) — tested with a mock/stub that returns Authorized/Denied.
- The `TargetResolver` port (sessions feature implements) — tested with a mock/stub.
- The replay fold is the domain-layer `apply` that completes the persistence feature's `IdempotentLogReplay` stated-normative obligation.
- The Elicitation-slot layer is an independent log consumer (A2) — tested in isolation with a synthetic event stream.

## Risks

- **In-memory index warm-up cost.** Rebuilding the command index from replay on startup could be slow for large logs. Mitigated by snapshot checkpointing (the persistence feature's snapshot bounds replay to the tail). v0.1.0 has no quantitative performance target; monitored against feel.
- **`derive_transition` correctness.** The mapping from `ObservationKind`/`failure_code` to a candidate transition is the one place Observations become transitions. An incorrect mapping (e.g. mapping a `status` Observation to `completed`) would violate the state machine. Mitigated by the state-machine proptests + the `allowed_transition` SSOT.
- **Elicitation decoupling (A2) ordering.** The slot layer observes the log asynchronously; there's a window where a response is terminal but the slot isn't yet. This is acceptable (the slot terminalizes when it observes the event), but the slot layer must handle re-processing (idempotent observe). Mitigated by the slot layer's own state checks.
- **`TargetKey` canonicalization.** The `Operation.target_scope` → `TargetKey` projection must be canonical (same target → same key). The acceptance feature owns this projection. If two different `TargetScope` serializations map to the same target, dedup could fail or double-apply. Mitigated by a canonical serialization + proptest.

## Extension pressure classification

Per `AGENTS.md` extension pressure-test checklist:

- **Q1 hybrid (event log SSOT + in-memory index): committed v0.1.0.** The event log is the only source of command state; the in-memory index is a derived hot path. This is adapter-neutral and backend-neutral.
- **Q1b `CommandTransition` events: committed v0.1.0.** The transition-event shape is part of the generated contract. New transition types (if the `CommandState` registry grows) add `OperationState` variants + use the same `CommandTransition` envelope.
- **Q2 `GrantCheck` / `TargetResolver` ports: committed v0.1.0.** These are the Ports & Adapters seams. Authority and sessions implement them; acceptance depends on the traits, not the implementations.
- **Q3 observation ingestion: committed v0.1.0.** The ingestion method is the adapter→core ingress. The streaming/subscription layer is a reserved seam (protocol-seam owns it).
- **Q4 A2 elicitation decoupling: committed v0.1.0.** The slot layer is an independent log consumer. The responder-binding seam and responder-identity audit seam are reserved (per `docs/PROTOCOL.md` § Elicitation) — v0.1.0 binds to the operator actor; multi-operator responder distinction is a future promotion.
- **`StoredEventKind::CommandTransition`: committed v0.1.0.** Added to the schema-owned variant set. Future event kinds add variants.

## Deep review (feature-level, 2026-07-12)

Feature-level deep review after all 6 stories reached `done`. Verdict: **Request changes** — 5 blockers + 4 important. The pure state machine, replay fold, and dedup boundary are sound; the gaps are at the end-to-end boundary and the port shapes for sibling features. Findings:

### Blockers (to resolve before advancing to `done`)

1. **Retry returns `accepted`, not the existing command's state.** `DedupOutcome::Duplicate` calls the same `accepted_result` constructor as a new command, which hardcodes `OperationState::Accepted`. A retry after completion/cancellation/failure returns the wrong state (PROTOCOL.md § Idempotency: a retry returns the existing command record and state). **Fix:** the duplicate path must look up the existing command's `OperationState` (via the `CommandIndex`/`CommandStateLookup`) and return it, not hardcode `Accepted`.

2. **Target resolution discards the binding.** `TargetResolver::resolve`'s result is reduced to `.is_err()` and discarded — the `TargetBinding` is never used for delivery or recorded. `TargetBinding` also only represents runtime-session targets, but committed Operations include fleet/adapter/actor/resource targets (and `spawn` has no pre-existing runtime session). **Status:** the `TargetBinding` shape and its consumption are a forward-dependency on the sessions feature — the sessions `feature-design` will define what target binding means for delivery. The port is the seam; this is not a persistence-blocking gap.

3. **Authorization boundary cannot distinguish verified identity from a payload claim.** `GrantCheck` receives `operation.sender` directly from the wire — no verified authenticated principal is passed for comparison. **Status:** this is a forward-dependency on the protocol-seam / web-server (which holds the authenticated session) and the authority feature (which defines grant evaluation against a verified principal). The port is the seam; the sibling features fill it in.

4. **Observation ingress is not source-authenticated.** `ingest_observation` validates only a non-empty authority-domain before persisting. `CommandStateLookup` accepts only a command id — it cannot verify adapter identity, target session, or generation. **Status:** source-authentication of adapter reports is a protocol-seam / pi-adapter concern (the adapter's authenticated channel is established at attach time). The ingestion method is the seam; the authentication is upstream. Documented as a reserved seam.

5. **A2 correlations don't flow end-to-end.** The Elicitation layer requires `CommandTransition` to carry an `ElicitationId` correlation, but observation ingestion copies only the Observation's correlations into the transition — a response Operation correlated to an Elicitation followed by a result Observation correlated only to its command won't close the slot. Tests mask this by manually placing the Elicitation correlation on synthetic transitions. **Fix:** `derive_transition` must carry the originating Operation's correlations (looked up via the command record) into the `CommandTransition`, so the Elicitation correlation flows from the Operation → through the transition → to the slot layer.

### Important

- Transition metadata not semantically validated (failure codes on non-terminal transitions; `Completed + ExecutionFailed` inconsistency).
- Storage failures bypass the submission outcome vocabulary (should be `SubmissionOutcome::Failed`/`Unknown`, not `Err(AcceptanceError::Storage)`).
- Story records stale: the replay story still claims snapshot checkpointing is implemented; the feature body still claims late candidates never become transition events (the TOCTOU policy allows race-produced duplicates, skipped at replay).
- Replay silently ignores `StoredEventKind::Unspecified` instead of fail-fast.

### Assessment

The pure command lifecycle (state machine, transition adjacency, terminal finality, replay fold, dedup boundary) is a sound foundation. The three promoted properties hold by construction and have mutation-tested evidence. The gaps are at the end-to-end boundary — the port shapes and the authenticated-principal/source-auth concerns that the sibling features (authority, sessions, protocol-seam) are designed to fill. Blocker 1 (retry state) is a real bug fixable now; blocker 5 (correlation flow) is a real design gap fixable now; blockers 2-4 are forward-dependencies on sibling features and are correctly framed as reserved seams. The feature stays at `stage: review` until blockers 1 and 5 are resolved and the forward-dependency seams are documented as reserved.
