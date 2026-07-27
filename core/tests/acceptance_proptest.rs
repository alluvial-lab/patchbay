//! Property tests for operation-acceptance invariants.
//!
//! The suite connects the promoted `TerminalFinality`,
//! `NoAcceptedToCompleted`, and `BoundaryDedup` model properties to the Rust
//! implementation. It also checks the stated-normative first-durable-terminal
//! and deterministic-replay obligations.
//!
//! Mutation tests use the same property oracles against deliberately faulty
//! implementations. Replay determinism is the one exception: the production
//! fold has no clock, randomness, iteration-order output, or injectable choice.
//! A storage adapter that changes events between reads would change the input,
//! not inject nondeterminism into the fold, so this suite does not pretend that
//! such an adapter is a replay-determinism mutant.

use std::future::ready;

use patchbay_contracts::patchbay::{
    AcceptedOperation, ActorEndpointRef, ActorId, AdapterId, AuthorityDomainId, CommandId, CommandTransition,
    DeviceId, EndpointId, EventId, FailureCode, Generation, GrantId, IdempotencyKey, Lsn, Operation,
    OperationKind, OperationState, PayloadContentType, PayloadEnvelope, RuntimeSessionId,
    StoredEventKind, StoredEventPayload, SubmissionOutcome, TargetScope, TargetScopeKind,
    TimeWindow,
};
use patchbay_core::acceptance::{
    apply_transition, is_terminal, rebuild_from_log, submit, AcceptanceError, ActiveElicitation,
    Authorized, CommandRecord, CommandSnapshot, CommandStateLookup, ElicitationContractLookup,
    GrantCheck, GrantDenied, TargetBinding, TargetNotFound, TargetResolver,
};
use patchbay_core::{
    authority::IssuerContext,
    storage::{
        DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError, StoredSnapshot,
        TargetKey,
    },
};
use proptest::prelude::*;
use prost::Message;
use prost_types::Timestamp;

const TERMINAL_STATES: [OperationState; 6] = [
    OperationState::Completed,
    OperationState::Rejected,
    OperationState::Failed,
    OperationState::Expired,
    OperationState::Cancelled,
    OperationState::Superseded,
];

const NON_COMPLETED_TERMINAL_STATES: [OperationState; 5] = [
    OperationState::Rejected,
    OperationState::Failed,
    OperationState::Expired,
    OperationState::Cancelled,
    OperationState::Superseded,
];

const RUNNING_TERMINAL_STATES: [OperationState; 5] = [
    OperationState::Completed,
    OperationState::Failed,
    OperationState::Expired,
    OperationState::Cancelled,
    OperationState::Superseded,
];

fn any_terminal_state() -> impl Strategy<Value = OperationState> {
    prop::sample::select(&TERMINAL_STATES)
}

fn any_non_completed_terminal_state() -> impl Strategy<Value = OperationState> {
    prop::sample::select(&NON_COMPLETED_TERMINAL_STATES)
}

fn any_running_terminal_state() -> impl Strategy<Value = OperationState> {
    prop::sample::select(&RUNNING_TERMINAL_STATES)
}

fn any_operation_state() -> impl Strategy<Value = OperationState> {
    prop::sample::select(&[
        OperationState::Unspecified,
        OperationState::Accepted,
        OperationState::Delivered,
        OperationState::Running,
        OperationState::Completed,
        OperationState::Rejected,
        OperationState::Failed,
        OperationState::Expired,
        OperationState::Cancelled,
        OperationState::Superseded,
    ])
}

fn authority_domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "acceptance-proptest-domain".to_owned(),
    }
}

fn operation(command_id: &str, idempotency_key: &str, payload: Vec<u8>) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command_id.to_owned(),
        }),
        authority_domain_id: Some(authority_domain()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: "operator".to_owned(),
            }),
            endpoint_id: Some(EndpointId {
                value: "web-proptest".to_owned(),
            }),
            endpoint_generation: Some(Generation { value: 1 }),
            ..ActorEndpointRef::default()
        }),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(AdapterId {
                value: "adapter-proptest".to_owned(),
            }),
            runtime_session_id: Some(RuntimeSessionId {
                value: "session-proptest".to_owned(),
            }),
            session_generation: Some(Generation { value: 1 }),
            deployment_scope: "local".to_owned(),
            ..TargetScope::default()
        }),
        idempotency_key: idempotency_key.to_owned(),
        payload: Some(PayloadEnvelope {
            payload,
            content_type: PayloadContentType::Binary as i32,
            schema_ref: "patchbay.test.acceptance-proptest.v0".to_owned(),
        }),
        validity_window: Some(TimeWindow {
            starts_at: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
            expires_at: Some(Timestamp {
                seconds: 253_402_300_799,
                nanos: 0,
            }),
        }),
        submitted_at: Some(Timestamp {
            seconds: 1,
            nanos: 0,
        }),
        ..Operation::default()
    }
}

fn record_at(command_id: &str, state: OperationState) -> CommandRecord {
    let mut record = CommandRecord::new(operation(command_id, "record-key", Vec::new()), 1)
        .expect("the controlled test operation has a command id");
    record.state = state;
    if is_terminal(state) {
        record.terminal_lsn = Some(1);
    }
    record
}

fn transition(
    command_id: &str,
    from_state: OperationState,
    to_state: OperationState,
) -> CommandTransition {
    CommandTransition {
        command_id: Some(CommandId {
            value: command_id.to_owned(),
        }),
        from_state: from_state as i32,
        to_state: to_state as i32,
        failure_code: FailureCode::Unspecified as i32,
        ..CommandTransition::default()
    }
}

fn event_payload<M: Message>(kind: StoredEventKind, message: &M) -> StoredEventPayload {
    StoredEventPayload {
        kind: kind as i32,
        payload: message.encode_to_vec(),
    }
}

async fn append_operation<S: Storage>(storage: &S, operation: &Operation) -> u64 {
    storage
        .append(
            &authority_domain(),
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: AcceptedOperation {
                    operation: Some(operation.clone()),
                    authorizing_grant_id: Some(GrantId { value: "test-grant".to_owned() }),
                }.encode_to_vec(),
            },
        )
        .await
        .expect("the in-memory test log accepts an operation")
        .lsn
        .expect("appended events have an LSN")
        .value
}

async fn append_transition<S: Storage>(storage: &S, transition: &CommandTransition) -> u64 {
    storage
        .append(
            &authority_domain(),
            event_payload(StoredEventKind::CommandTransition, transition),
        )
        .await
        .expect("the in-memory test log accepts a transition")
        .lsn
        .expect("appended events have an LSN")
        .value
}

struct TestIssuer {
    actor: ActorId,
    endpoint: EndpointId,
    device: DeviceId,
    generation: Generation,
    domain: AuthorityDomainId,
}

impl TestIssuer {
    fn new(domain: AuthorityDomainId) -> Self {
        Self {
            actor: ActorId {
                value: "operator".to_owned(),
            },
            endpoint: EndpointId {
                value: "web-proptest".to_owned(),
            },
            device: DeviceId {
                value: "device-proptest".to_owned(),
            },
            generation: Generation { value: 1 },
            domain,
        }
    }
}

impl IssuerContext for TestIssuer {
    fn verified_actor(&self) -> Option<&ActorId> {
        Some(&self.actor)
    }

    fn verified_endpoint(&self) -> Option<&EndpointId> {
        Some(&self.endpoint)
    }

    fn verified_device(&self) -> Option<&DeviceId> {
        Some(&self.device)
    }

    fn endpoint_generation(&self) -> Option<Generation> {
        Some(self.generation)
    }

    fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.domain
    }
}

struct AlwaysAuthorized;

impl GrantCheck for AlwaysAuthorized {
    fn check(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _issuer: &dyn IssuerContext,
        _operation_kind: OperationKind,
        _target_scope: &TargetScope,
    ) -> impl std::future::Future<Output = Result<Authorized, GrantDenied>> + Send {
        ready(Ok(Authorized { grant_id: Some(GrantId { value: "test-grant".to_owned() }) }))
    }
}

struct AlwaysResolved;

impl TargetResolver for AlwaysResolved {
    fn resolve(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _target_scope: &TargetScope,
    ) -> impl std::future::Future<Output = Result<TargetBinding, TargetNotFound>> + Send {
        ready(Ok(TargetBinding {
            runtime_session_id: RuntimeSessionId {
                value: "session-proptest".to_owned(),
            },
            session_generation: Generation { value: 1 },
            adapter_id: AdapterId {
                value: "adapter-proptest".to_owned(),
            },
        }))
    }
}

struct AlwaysAccepted;

impl ElicitationContractLookup for AlwaysAccepted {
    async fn active_contract(
        &self,
        _elicitation_id: &patchbay_contracts::patchbay::ElicitationId,
    ) -> Option<ActiveElicitation> {
        None
    }
}

impl CommandStateLookup for AlwaysAccepted {
    async fn current_state(&self, _command_id: &CommandId) -> Option<CommandSnapshot> {
        Some(CommandSnapshot {
            state: OperationState::Accepted,
            correlations: vec![],
            terminal_lsn: None,
        })
    }
}

fn terminal_finality_holds(
    applier: fn(&mut CommandRecord, &CommandTransition, u64) -> Result<(), AcceptanceError>,
    command_id: &str,
    terminal_state: OperationState,
    candidate_state: OperationState,
) -> bool {
    let mut record = record_at(command_id, terminal_state);
    let before = record.clone();
    let candidate = transition(command_id, terminal_state, candidate_state);
    let result = applier(&mut record, &candidate, 2);

    matches!(result, Err(AcceptanceError::AlreadyTerminal(_))) && record == before
}

fn no_accepted_to_completed_holds(
    applier: fn(&mut CommandRecord, &CommandTransition, u64) -> Result<(), AcceptanceError>,
    command_id: &str,
) -> bool {
    let mut record = record_at(command_id, OperationState::Accepted);
    let before = record.clone();
    let candidate = transition(
        command_id,
        OperationState::Accepted,
        OperationState::Completed,
    );
    let result = applier(&mut record, &candidate, 2);

    matches!(result, Err(AcceptanceError::CorruptLog(_))) && record == before
}

async fn run_boundary_dedup_check<S: Storage>(
    storage: &S,
    submitted: Operation,
) -> Result<(), String> {
    let issuer = TestIssuer::new(authority_domain());
    let first = submit(
        storage,
        &AlwaysAuthorized,
        &AlwaysResolved,
        &AlwaysAccepted,
        &AlwaysAccepted,
        &issuer,
        submitted.clone(),
    )
    .await
    .map_err(|error| format!("initial submission failed: {error}"))?;
    let retry = submit(
        storage,
        &AlwaysAuthorized,
        &AlwaysResolved,
        &AlwaysAccepted,
        &AlwaysAccepted,
        &issuer,
        submitted.clone(),
    )
    .await
    .map_err(|error| format!("retry failed: {error}"))?;

    if first.outcome != SubmissionOutcome::Accepted as i32 || first.deduplicated {
        return Err(format!(
            "initial submission was not a new acceptance: {first:?}"
        ));
    }
    if retry.outcome != SubmissionOutcome::Accepted as i32 || !retry.deduplicated {
        return Err(format!("retry was not deduplicated: {retry:?}"));
    }
    if retry.command_id != first.command_id || retry.accepted_lsn != first.accepted_lsn {
        return Err(format!(
            "retry did not return the existing acceptance: first={first:?}, retry={retry:?}"
        ));
    }

    let events = storage
        .read_after(&authority_domain(), Lsn { value: 0 })
        .await
        .map_err(|error| format!("cannot inspect durable events: {error}"))?;
    if events.len() != 1 {
        return Err(format!(
            "retry double-applied: expected one event, found {}",
            events.len()
        ));
    }
    if StoredEventKind::try_from(events[0].payload.kind).ok() != Some(StoredEventKind::Operation) {
        return Err("acceptance event has the wrong stored-event kind".to_owned());
    }
    let accepted = AcceptedOperation::decode(events[0].payload.payload.as_slice())
        .map_err(|error| format!("cannot decode acceptance event: {error}"))?;
    let recorded = accepted.operation.ok_or_else(|| "accepted operation has no operation".to_owned())?;
    let mut expected = submitted;
    // Sender is the one durable field intentionally normalized rather than
    // retained from caller input.
    expected.sender = Some(ActorEndpointRef {
        actor_id: Some(issuer.actor.clone()),
        endpoint_id: Some(issuer.endpoint.clone()),
        device_id: Some(issuer.device.clone()),
        endpoint_generation: Some(issuer.generation),
    });
    if recorded != expected {
        return Err("durable acceptance payload differs after sender normalization".to_owned());
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum LifecyclePath {
    DirectTerminal(OperationState),
    Delivered,
    DeliveredTerminal(OperationState),
    Running,
    RunningTerminal(OperationState),
}

#[derive(Clone, Debug)]
struct LifecyclePlan {
    path: LifecyclePath,
    payload: Vec<u8>,
}

fn any_lifecycle_plan() -> impl Strategy<Value = LifecyclePlan> {
    let path = prop_oneof![
        any_non_completed_terminal_state().prop_map(LifecyclePath::DirectTerminal),
        Just(LifecyclePath::Delivered),
        any_terminal_state().prop_map(LifecyclePath::DeliveredTerminal),
        Just(LifecyclePath::Running),
        any_running_terminal_state().prop_map(LifecyclePath::RunningTerminal),
    ];

    (path, prop::collection::vec(any::<u8>(), 0..32))
        .prop_map(|(path, payload)| LifecyclePlan { path, payload })
}

async fn append_lifecycle_plan(
    storage: &RusqliteStorage,
    index: usize,
    plan: &LifecyclePlan,
) -> (CommandId, OperationState, Option<u64>) {
    let command_id = format!("replay-command-{index}");
    let operation = operation(
        &command_id,
        &format!("replay-idempotency-{index}"),
        plan.payload.clone(),
    );
    append_operation(storage, &operation).await;

    let append = |from, to| transition(&command_id, from, to);
    let (final_state, terminal_lsn) = match plan.path {
        LifecyclePath::DirectTerminal(terminal) => {
            let lsn = append_transition(storage, &append(OperationState::Accepted, terminal)).await;
            (terminal, Some(lsn))
        }
        LifecyclePath::Delivered => {
            append_transition(
                storage,
                &append(OperationState::Accepted, OperationState::Delivered),
            )
            .await;
            (OperationState::Delivered, None)
        }
        LifecyclePath::DeliveredTerminal(terminal) => {
            append_transition(
                storage,
                &append(OperationState::Accepted, OperationState::Delivered),
            )
            .await;
            let lsn =
                append_transition(storage, &append(OperationState::Delivered, terminal)).await;
            (terminal, Some(lsn))
        }
        LifecyclePath::Running => {
            append_transition(
                storage,
                &append(OperationState::Accepted, OperationState::Delivered),
            )
            .await;
            append_transition(
                storage,
                &append(OperationState::Delivered, OperationState::Running),
            )
            .await;
            (OperationState::Running, None)
        }
        LifecyclePath::RunningTerminal(terminal) => {
            append_transition(
                storage,
                &append(OperationState::Accepted, OperationState::Delivered),
            )
            .await;
            append_transition(
                storage,
                &append(OperationState::Delivered, OperationState::Running),
            )
            .await;
            let lsn = append_transition(storage, &append(OperationState::Running, terminal)).await;
            (terminal, Some(lsn))
        }
    };

    (
        operation
            .command_id
            .expect("the generated operation has a command id"),
        final_state,
        terminal_lsn,
    )
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        ..ProptestConfig::default()
    })]

    /// TerminalFinality: every terminal state rejects every candidate next
    /// state and remains byte-for-byte unchanged.
    #[test]
    fn terminal_state_rejects_further_transitions(
        command_suffix in "[a-z0-9]{1,16}",
        terminal_state in any_terminal_state(),
        candidate_state in any_operation_state(),
    ) {
        let command_id = format!("terminal-{command_suffix}");
        prop_assert!(terminal_finality_holds(
            apply_transition,
            &command_id,
            terminal_state,
            candidate_state,
        ));
    }

    /// NoAcceptedToCompleted: the prohibited adjacency is rejected as corrupt
    /// log input and leaves the accepted record unchanged.
    #[test]
    fn accepted_to_completed_is_rejected(command_suffix in "[a-z0-9]{1,16}") {
        let command_id = format!("adjacency-{command_suffix}");
        prop_assert!(no_accepted_to_completed_holds(
            apply_transition,
            &command_id,
        ));
    }

    /// BoundaryDedup: an identical retry returns the original acceptance and
    /// creates no second durable event.
    #[test]
    fn retry_returns_existing_no_double_apply(
        command_suffix in "[a-z0-9]{1,16}",
        key_suffix in "[a-z0-9]{1,16}",
        payload in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let submitted = operation(
                &format!("dedup-{command_suffix}"),
                &format!("dedup-{key_suffix}"),
                payload,
            );
            run_boundary_dedup_check(&storage, submitted)
                .await
                .map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// First-durable-terminal-wins: replay retains the first terminal state
    /// and LSN while treating the later race-produced terminal event as stale.
    #[test]
    fn first_terminal_wins_later_is_stale(
        command_suffix in "[a-z0-9]{1,16}",
        first_terminal in any_terminal_state(),
        second_offset in 1usize..TERMINAL_STATES.len(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let command_id = format!("terminal-race-{command_suffix}");
            let accepted = operation(&command_id, &format!("race-{command_suffix}"), Vec::new());
            append_operation(&storage, &accepted).await;
            append_transition(
                &storage,
                &transition(&command_id, OperationState::Accepted, OperationState::Delivered),
            )
            .await;
            let first_terminal_lsn = append_transition(
                &storage,
                &transition(&command_id, OperationState::Delivered, first_terminal),
            )
            .await;
            let first_index = TERMINAL_STATES
                .iter()
                .position(|state| *state == first_terminal)
                .unwrap();
            let second_terminal = TERMINAL_STATES
                [(first_index + second_offset) % TERMINAL_STATES.len()];
            let later_lsn = append_transition(
                &storage,
                // The real TOCTOU race: both candidates read the same pre-terminal
                // state before either append, so both encode Delivered as
                // from_state. A mutant that checks from_state before terminal
                // finality would reject this as a CorruptLog (Delivered != first_terminal),
                // masking the stale-candidate skip bug.
                &transition(&command_id, OperationState::Delivered, second_terminal),
            )
            .await;

            let rebuilt = rebuild_from_log(&storage, &authority_domain())
                .await
                .map_err(|error| TestCaseError::fail(format!("replay rejected a stale terminal candidate: {error}")))?;
            let record = rebuilt
                .get_command(accepted.command_id.as_ref().unwrap())
                .ok_or_else(|| TestCaseError::fail("replay lost the raced command"))?;

            prop_assert!(later_lsn > first_terminal_lsn);
            prop_assert_eq!(record.state, first_terminal);
            prop_assert_eq!(record.terminal_lsn, Some(first_terminal_lsn));
            let events = storage.read_after(&authority_domain(), Lsn { value: 0 }).await.unwrap();
            prop_assert_eq!(events.len(), 4, "the stale event remains durable audit evidence");
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// IdempotentLogReplay, end-to-end: the same valid event sequence produces
    /// identical command indexes, and each index matches the generated plan.
    #[test]
    fn replay_reconstructs_identical_index(
        plans in prop::collection::vec(any_lifecycle_plan(), 1..7),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut expected = Vec::with_capacity(plans.len());
            for (index, plan) in plans.iter().enumerate() {
                expected.push(append_lifecycle_plan(&storage, index, plan).await);
            }

            let first = rebuild_from_log(&storage, &authority_domain()).await
                .map_err(|error| TestCaseError::fail(format!("first replay failed: {error}")))?;
            let second = rebuild_from_log(&storage, &authority_domain()).await
                .map_err(|error| TestCaseError::fail(format!("second replay failed: {error}")))?;

            prop_assert_eq!(&first, &second, "unchanged committed events produced different indexes");
            prop_assert_eq!(first.len(), expected.len());
            for (command_id, expected_state, expected_terminal_lsn) in expected {
                let record = first.get_command(&command_id)
                    .ok_or_else(|| TestCaseError::fail(format!("missing command {command_id:?}")))?;
                prop_assert_eq!(record.state, expected_state);
                prop_assert_eq!(record.terminal_lsn, expected_terminal_lsn);
            }
            Ok::<(), TestCaseError>(())
        })?;
    }
}

// ===== Mutation discipline =====

/// Mutant: terminal records are directly overwritten instead of rejecting the
/// candidate. This models removal of the finality guard plus the matching
/// terminal adjacency protection.
fn apply_without_terminal_finality(
    record: &mut CommandRecord,
    transition: &CommandTransition,
    event_lsn: u64,
) -> Result<(), AcceptanceError> {
    if is_terminal(record.state) {
        let to_state = OperationState::try_from(transition.to_state).map_err(|_| {
            AcceptanceError::CorruptLog("terminal-finality mutant received an unknown state".into())
        })?;
        record.state = to_state;
        record.terminal_lsn = is_terminal(to_state).then_some(event_lsn);
        record.failure_code = None;
        return Ok(());
    }
    apply_transition(record, transition, event_lsn)
}

/// Mutant: the forbidden accepted-to-completed edge is admitted directly.
fn apply_with_accepted_to_completed_edge(
    record: &mut CommandRecord,
    transition: &CommandTransition,
    event_lsn: u64,
) -> Result<(), AcceptanceError> {
    let to_state = OperationState::try_from(transition.to_state).map_err(|_| {
        AcceptanceError::CorruptLog("adjacency mutant received an unknown state".into())
    })?;
    if record.state == OperationState::Accepted && to_state == OperationState::Completed {
        record.state = OperationState::Completed;
        record.terminal_lsn = Some(event_lsn);
        record.failure_code = None;
        return Ok(());
    }
    apply_transition(record, transition, event_lsn)
}

/// Mutant: an already-terminal record accepts the later terminal transition,
/// implementing last-writer-wins instead of first-durable-terminal-wins.
fn apply_with_last_terminal_wins(
    record: &mut CommandRecord,
    transition: &CommandTransition,
    event_lsn: u64,
) -> Result<(), AcceptanceError> {
    match apply_transition(record, transition, event_lsn) {
        Err(AcceptanceError::AlreadyTerminal(_)) => {
            let to_state = OperationState::try_from(transition.to_state).map_err(|_| {
                AcceptanceError::CorruptLog("terminal-race mutant received an unknown state".into())
            })?;
            record.state = to_state;
            record.terminal_lsn = Some(event_lsn);
            record.failure_code = None;
            Ok(())
        }
        result => result,
    }
}

/// Mutant storage adapter: `append_dedup` ignores its key and target and
/// always appends, so an identical retry double-applies at the boundary.
struct DoubleApplyStorage(RusqliteStorage);

impl Storage for DoubleApplyStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        self.0.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        _key: &IdempotencyKey,
        _target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.0
            .append(authority_domain_id, payload)
            .await
            .map(DedupOutcome::Appended)
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.0.read_after(authority_domain_id, cursor).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.0
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        self.0
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }
}

#[test]
fn terminal_finality_catches_injected_bug() {
    let command_id = "terminal-mutation";
    assert!(
        terminal_finality_holds(
            apply_transition,
            command_id,
            OperationState::Completed,
            OperationState::Delivered,
        ),
        "the production implementation must satisfy TerminalFinality"
    );
    assert!(
        !terminal_finality_holds(
            apply_without_terminal_finality,
            command_id,
            OperationState::Completed,
            OperationState::Delivered,
        ),
        "the finality property did not catch a terminal-state overwrite"
    );
}

#[test]
fn no_accepted_to_completed_catches_injected_edge() {
    let command_id = "adjacency-mutation";
    assert!(
        no_accepted_to_completed_holds(apply_transition, command_id),
        "the production implementation must reject accepted-to-completed"
    );
    assert!(
        !no_accepted_to_completed_holds(apply_with_accepted_to_completed_edge, command_id),
        "the adjacency property did not catch an admitted accepted-to-completed edge"
    );
}

#[test]
fn boundary_dedup_catches_injected_double_apply() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let submitted = operation("dedup-mutation", "dedup-mutation-key", vec![1, 2, 3]);
        let real = RusqliteStorage::open_in_memory().unwrap();
        assert!(
            run_boundary_dedup_check(&real, submitted.clone())
                .await
                .is_ok(),
            "the production implementation must deduplicate an identical retry"
        );
        // Verify the real store has exactly ONE event (not just that the oracle passed).
        let real_events = real
            .read_after(&authority_domain(), Lsn { value: 0 })
            .await
            .unwrap();
        assert_eq!(
            real_events.len(),
            1,
            "production must persist exactly one event"
        );

        let mutant = DoubleApplyStorage(RusqliteStorage::open_in_memory().unwrap());
        assert!(
            run_boundary_dedup_check(&mutant, submitted).await.is_err(),
            "the BoundaryDedup property did not catch an always-append storage adapter"
        );
        // Directly prove the mutant persisted TWO events (double-apply),
        // independent of the oracle's early exit on deduplicated=false.
        let mutant_events = mutant
            .read_after(&authority_domain(), Lsn { value: 0 })
            .await
            .unwrap();
        assert_eq!(
            mutant_events.len(),
            2,
            "the always-append mutant must persist exactly two events (double-apply)"
        );
    });
}

#[test]
fn first_terminal_wins_catches_injected_last_writer_bug() {
    fn run(
        applier: fn(&mut CommandRecord, &CommandTransition, u64) -> Result<(), AcceptanceError>,
    ) -> CommandRecord {
        let command_id = "terminal-race-mutation";
        let mut record = record_at(command_id, OperationState::Accepted);
        applier(
            &mut record,
            &transition(
                command_id,
                OperationState::Accepted,
                OperationState::Delivered,
            ),
            2,
        )
        .expect("accepted-to-delivered is valid");
        applier(
            &mut record,
            &transition(
                command_id,
                OperationState::Delivered,
                OperationState::Completed,
            ),
            3,
        )
        .expect("delivered-to-completed is valid");
        let _ = applier(
            &mut record,
            // The real TOCTOU race: the stale candidate encodes the same
            // pre-terminal from_state (Delivered), not the first terminal.
            &transition(
                command_id,
                OperationState::Delivered,
                OperationState::Failed,
            ),
            4,
        );
        record
    }

    let real = run(apply_transition);
    assert_eq!(real.state, OperationState::Completed);
    assert_eq!(real.terminal_lsn, Some(3));

    let mutant = run(apply_with_last_terminal_wins);
    assert!(
        mutant.state != OperationState::Completed || mutant.terminal_lsn != Some(3),
        "the first-terminal oracle did not catch last-writer-wins"
    );
}

/// Exhaustive TerminalFinality: every terminal-state × candidate-state pair
/// is rejected. The proptest samples this finite domain; this test enumerates
/// all 6 terminal × 9 candidate = 54 pairs deterministically so a bug
/// confined to one pair cannot escape by chance.
#[test]
fn terminal_finality_exhaustive_all_pairs() {
    let terminal_states = [
        OperationState::Completed,
        OperationState::Rejected,
        OperationState::Failed,
        OperationState::Expired,
        OperationState::Cancelled,
        OperationState::Superseded,
    ];
    let candidate_states = [
        OperationState::Unspecified,
        OperationState::Accepted,
        OperationState::Delivered,
        OperationState::Running,
        OperationState::Completed,
        OperationState::Rejected,
        OperationState::Failed,
        OperationState::Expired,
        OperationState::Cancelled,
        OperationState::Superseded,
    ];

    for terminal in &terminal_states {
        for candidate in &candidate_states {
            let command_id = format!("exhaustive-{terminal:?}-{candidate:?}");
            assert!(
                terminal_finality_holds(apply_transition, &command_id, *terminal, *candidate,),
                "TerminalFinality failed for terminal={terminal:?}, candidate={candidate:?}"
            );
        }
    }
}
