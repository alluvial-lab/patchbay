use std::future::ready;
use std::sync::atomic::{AtomicUsize, Ordering};

use patchbay_contracts::patchbay::{
    response_contract, typed_correlation, ActorEndpointRef, ActorId, AdapterId, AuthorityDomainId,
    CommandId, DeviceId, ElicitationId, ElicitationResponsePayload, EndpointId, FailureCode,
    Generation, Lsn, Operation, OperationKind, OperationState, PayloadContentType, PayloadEnvelope,
    QuestionContract, ResponseContract, ResponseContractKind, ResponseOption, RuntimeSessionId,
    StoredEventKind, SubmissionOutcome, TargetScope, TargetScopeKind, TypedCorrelation,
};
use patchbay_core::acceptance::{
    submit, AcceptanceError, ActiveElicitation, Authorized, CommandSnapshot, CommandStateLookup,
    ElicitationContractLookup, GrantCheck, GrantDenied, TargetBinding, TargetNotFound,
    TargetResolver,
};
use patchbay_core::{
    authority::IssuerContext,
    storage::{RusqliteStorage, Storage},
};
use prost::Message;

struct TestGrantCheck {
    authorized: bool,
    calls: AtomicUsize,
}

impl TestGrantCheck {
    fn new(authorized: bool) -> Self {
        Self {
            authorized,
            calls: AtomicUsize::new(0),
        }
    }
}

impl GrantCheck for TestGrantCheck {
    fn check(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        _target_scope: &TargetScope,
    ) -> impl std::future::Future<Output = Result<Authorized, GrantDenied>> + Send {
        self.calls.fetch_add(1, Ordering::Relaxed);
        ready(if self.authorized {
            Ok(Authorized { grant_id: None })
        } else {
            Err(GrantDenied::NoGrant {
                actor: "operator".to_owned(),
                kind: operation_kind,
                target: "session".to_owned(),
            })
        })
    }
}

struct NoElicitationContractLookup;

impl ElicitationContractLookup for NoElicitationContractLookup {
    async fn active_contract(
        &self,
        _elicitation_id: &patchbay_contracts::patchbay::ElicitationId,
    ) -> Option<ActiveElicitation> {
        None
    }
}

struct TestTargetResolver {
    found: bool,
    calls: AtomicUsize,
}

impl TestTargetResolver {
    fn new(found: bool) -> Self {
        Self {
            found,
            calls: AtomicUsize::new(0),
        }
    }
}

impl TargetResolver for TestTargetResolver {
    fn resolve(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _target_scope: &TargetScope,
    ) -> impl std::future::Future<Output = Result<TargetBinding, TargetNotFound>> + Send {
        self.calls.fetch_add(1, Ordering::Relaxed);
        ready(if self.found {
            Ok(TargetBinding {
                runtime_session_id: RuntimeSessionId {
                    value: "session-1".to_owned(),
                },
                session_generation: Generation { value: 7 },
                adapter_id: AdapterId {
                    value: "pi".to_owned(),
                },
            })
        } else {
            Err(TargetNotFound::NotFound {
                target: "session-1".to_owned(),
            })
        })
    }
}

/// A CommandStateLookup stub that always reports Accepted with no
/// correlations. Used by pipeline tests that don't build a full CommandIndex.
/// The retry test that needs the existing-state behavior uses a real
/// CommandIndex via rebuild_from_log.
struct AlwaysAccepted;

impl CommandStateLookup for AlwaysAccepted {
    async fn current_state(&self, _command_id: &CommandId) -> Option<CommandSnapshot> {
        Some(CommandSnapshot {
            state: OperationState::Accepted,
            correlations: vec![],
            terminal_lsn: None,
        })
    }
}

/// A CommandStateLookup that always returns None — simulates a missing
/// command in the index (inconsistency between the durable log and the
/// in-memory projection).
struct NotFoundLookup;

impl CommandStateLookup for NotFoundLookup {
    async fn current_state(&self, _command_id: &CommandId) -> Option<CommandSnapshot> {
        None
    }
}

fn authority_domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
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
                value: "web-1".to_owned(),
            },
            device: DeviceId {
                value: "device-1".to_owned(),
            },
            generation: Generation { value: 3 },
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

fn issuer() -> TestIssuer {
    TestIssuer::new(authority_domain())
}

fn operation() -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: "command-1".to_owned(),
        }),
        authority_domain_id: Some(authority_domain()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: "operator".to_owned(),
            }),
            endpoint_id: Some(EndpointId {
                value: "web-1".to_owned(),
            }),
            endpoint_generation: Some(Generation { value: 3 }),
            ..ActorEndpointRef::default()
        }),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            runtime_session_id: Some(RuntimeSessionId {
                value: "session-1".to_owned(),
            }),
            session_generation: Some(Generation { value: 7 }),
            deployment_scope: "local".to_owned(),
            ..TargetScope::default()
        }),
        idempotency_key: "idempotency-1".to_owned(),
        payload: Some(PayloadEnvelope {
            payload: b"status".to_vec(),
            content_type: PayloadContentType::TextUtf8 as i32,
            schema_ref: "patchbay.test.instruct.v0".to_owned(),
        }),
        ..Operation::default()
    }
}

struct TerminalRetryLookup {
    active: ActiveElicitation,
}

impl ElicitationContractLookup for TerminalRetryLookup {
    async fn active_contract(&self, _elicitation_id: &ElicitationId) -> Option<ActiveElicitation> {
        Some(self.active.clone())
    }
}

fn response_operation() -> Operation {
    let mut operation = operation();
    operation.kind = OperationKind::ElicitationResponse as i32;
    operation.idempotency_key = "idempotency-response-1".to_owned();
    operation.correlations = vec![TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::ElicitationId(ElicitationId {
            value: "elicitation-1".to_owned(),
        })),
    }];
    operation.payload = Some(PayloadEnvelope {
        payload: ElicitationResponsePayload {
            selected_option_id: "yes".to_owned(),
            ..ElicitationResponsePayload::default()
        }
        .encode_to_vec(),
        content_type: PayloadContentType::Protobuf as i32,
        ..PayloadEnvelope::default()
    });
    operation
}

fn active_question() -> ActiveElicitation {
    ActiveElicitation {
        contract: ResponseContract {
            contract_kind: ResponseContractKind::Question as i32,
            contract_body: Some(response_contract::ContractBody::Question(
                QuestionContract {
                    options: vec![ResponseOption {
                        option_id: "yes".to_owned(),
                        label: "Yes".to_owned(),
                    }],
                    allow_free_text: false,
                },
            )),
            ..ResponseContract::default()
        },
        is_terminal: false,
        winning_response: None,
    }
}

fn outcome(result: &patchbay_contracts::patchbay::SubmissionResult) -> SubmissionOutcome {
    SubmissionOutcome::try_from(result.outcome).expect("result has a generated outcome")
}

fn failure(result: &patchbay_contracts::patchbay::SubmissionResult) -> FailureCode {
    FailureCode::try_from(result.failure_code).expect("result has a generated failure code")
}

fn state(result: &patchbay_contracts::patchbay::SubmissionResult) -> OperationState {
    OperationState::try_from(result.operation_state).expect("result has a generated state")
}

async fn durable_events(storage: &RusqliteStorage) -> Vec<patchbay_core::storage::RecordedEvent> {
    storage
        .read_after(&authority_domain(), Lsn { value: 0 })
        .await
        .expect("test storage remains readable")
}

#[tokio::test]
async fn unknown_and_reserved_operation_kinds_reject_before_grant() {
    for raw_kind in [
        OperationKind::Unspecified as i32,
        OperationKind::ReservedAgentSend as i32,
        OperationKind::ReservedAdapterUtilityExec as i32,
        999,
    ] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let grant = TestGrantCheck::new(true);
        let resolver = TestTargetResolver::new(true);
        let mut submitted = operation();
        submitted.kind = raw_kind;

        let result = submit(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &NoElicitationContractLookup,
            &issuer(),
            submitted,
        )
        .await
        .unwrap();

        assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
        assert_eq!(failure(&result), FailureCode::ValidationFailed);
        assert_eq!(state(&result), OperationState::Unspecified);
        assert_eq!(grant.calls.load(Ordering::Relaxed), 0);
        assert_eq!(resolver.calls.load(Ordering::Relaxed), 0);
        assert!(durable_events(&storage).await.is_empty());
    }
}

#[tokio::test]
async fn missing_required_fields_reject_before_grant_without_durable_state() {
    let mut submissions = Vec::new();

    let mut missing_command = operation();
    missing_command.command_id = None;
    submissions.push(missing_command);

    let mut missing_domain = operation();
    missing_domain.authority_domain_id = None;
    submissions.push(missing_domain);

    let mut missing_sender = operation();
    missing_sender.sender = None;
    submissions.push(missing_sender);

    let mut missing_target = operation();
    missing_target.target_scope = None;
    submissions.push(missing_target);

    let mut missing_key = operation();
    missing_key.idempotency_key.clear();
    submissions.push(missing_key);

    for submitted in submissions {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let grant = TestGrantCheck::new(true);
        let resolver = TestTargetResolver::new(true);

        let result = submit(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &NoElicitationContractLookup,
            &issuer(),
            submitted,
        )
        .await
        .unwrap();

        assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
        assert_eq!(failure(&result), FailureCode::ValidationFailed);
        assert_eq!(grant.calls.load(Ordering::Relaxed), 0);
        assert_eq!(resolver.calls.load(Ordering::Relaxed), 0);
        assert!(durable_events(&storage).await.is_empty());
    }
}

#[tokio::test]
async fn malformed_response_rejects_before_grant_without_durable_state() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let mut submitted = operation();
    submitted.kind = OperationKind::ElicitationResponse as i32;

    let result = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted,
    )
    .await
    .unwrap();

    assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
    assert_eq!(failure(&result), FailureCode::ValidationFailed);
    assert_eq!(grant.calls.load(Ordering::Relaxed), 0);
    assert_eq!(resolver.calls.load(Ordering::Relaxed), 0);
    assert!(durable_events(&storage).await.is_empty());
}

#[tokio::test]
async fn unauthorized_submission_rejects_without_durable_state() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(false);
    let resolver = TestTargetResolver::new(true);

    let result = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        operation(),
    )
    .await
    .unwrap();

    assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
    assert_eq!(failure(&result), FailureCode::AuthorizationDenied);
    assert_eq!(state(&result), OperationState::Unspecified);
    assert_eq!(grant.calls.load(Ordering::Relaxed), 1);
    assert_eq!(resolver.calls.load(Ordering::Relaxed), 0);
    assert!(durable_events(&storage).await.is_empty());
}

#[tokio::test]
async fn unknown_target_rejects_without_durable_state() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(false);

    let result = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        operation(),
    )
    .await
    .unwrap();

    assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
    assert_eq!(failure(&result), FailureCode::TargetNotFound);
    assert_eq!(state(&result), OperationState::Unspecified);
    assert_eq!(grant.calls.load(Ordering::Relaxed), 1);
    assert_eq!(resolver.calls.load(Ordering::Relaxed), 1);
    assert!(durable_events(&storage).await.is_empty());
}

#[tokio::test]
async fn new_command_is_durably_recorded_before_acceptance_returns() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let submitted = operation();

    let result = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted.clone(),
    )
    .await
    .unwrap();

    assert_eq!(outcome(&result), SubmissionOutcome::Accepted);
    assert_eq!(failure(&result), FailureCode::Unspecified);
    assert_eq!(state(&result), OperationState::Accepted);
    assert!(!result.deduplicated);
    assert_eq!(result.command_id, submitted.command_id);
    assert_eq!(result.accepted_lsn, Some(Lsn { value: 1 }));

    let events = durable_events(&storage).await;
    assert_eq!(events.len(), 1);
    assert_eq!(
        StoredEventKind::try_from(events[0].payload.kind).unwrap(),
        StoredEventKind::Operation
    );
    let recorded = Operation::decode(events[0].payload.payload.as_slice()).unwrap();
    assert_eq!(recorded, submitted);
}

#[tokio::test]
async fn identical_retry_returns_existing_acceptance_without_double_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let submitted = operation();

    let first = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted.clone(),
    )
    .await
    .unwrap();
    let retry = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted,
    )
    .await
    .unwrap();

    assert_eq!(outcome(&retry), SubmissionOutcome::Accepted);
    assert_eq!(state(&retry), OperationState::Accepted);
    assert!(retry.deduplicated);
    assert_eq!(retry.command_id, first.command_id);
    assert_eq!(retry.accepted_lsn, first.accepted_lsn);
    assert_eq!(durable_events(&storage).await.len(), 1);
}

#[tokio::test]
async fn differing_payload_retry_is_validation_rejection_without_second_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let original = operation();

    submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        original.clone(),
    )
    .await
    .unwrap();

    let mut conflicting = original;
    conflicting.payload.as_mut().unwrap().payload = b"different instruction".to_vec();
    let result = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        conflicting,
    )
    .await
    .unwrap();

    assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
    assert_eq!(failure(&result), FailureCode::ValidationFailed);
    assert_eq!(state(&result), OperationState::Unspecified);
    assert!(!result.deduplicated);
    assert_eq!(durable_events(&storage).await.len(), 1);
}

/// A CommandStateLookup that reports a specific command as Completed. Used
/// to verify a retry of an already-completed command returns Completed,
/// not a hardcoded Accepted.
struct CompletedLookup;

impl CommandStateLookup for CompletedLookup {
    async fn current_state(&self, _command_id: &CommandId) -> Option<CommandSnapshot> {
        Some(CommandSnapshot {
            state: OperationState::Completed,
            correlations: vec![],
            terminal_lsn: Some(5),
        })
    }
}

#[tokio::test]
async fn retry_returns_existing_state_not_hardcoded_accepted() {
    // Blocker 1 fix: a deduplicated retry must return the EXISTING command's
    // state, not a hardcoded Accepted. The command may have advanced to
    // Completed (or any other state) since the original accept.
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let submitted = operation();

    let first = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted.clone(),
    )
    .await
    .unwrap();
    assert_eq!(state(&first), OperationState::Accepted);

    // The command has since reached Completed (via observation transitions).
    // The retry must return Completed, not Accepted.
    let retry = submit(
        &storage,
        &grant,
        &resolver,
        &CompletedLookup,
        &NoElicitationContractLookup,
        &issuer(),
        submitted,
    )
    .await
    .unwrap();

    assert_eq!(outcome(&retry), SubmissionOutcome::Accepted);
    assert_eq!(
        state(&retry),
        OperationState::Completed,
        "retry must return the existing command's state (Completed), not hardcoded Accepted"
    );
    assert!(retry.deduplicated);
    assert_eq!(retry.command_id, first.command_id);
}

#[tokio::test]
async fn terminal_response_retry_returns_existing_command_record() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let submitted = response_operation();

    let first = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &TerminalRetryLookup {
            active: active_question(),
        },
        &issuer(),
        submitted.clone(),
    )
    .await
    .unwrap();
    assert_eq!(outcome(&first), SubmissionOutcome::Accepted);

    let retry = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &TerminalRetryLookup {
            active: ActiveElicitation {
                is_terminal: true,
                winning_response: Some(submitted.clone()),
                ..active_question()
            },
        },
        &issuer(),
        submitted,
    )
    .await
    .unwrap();

    assert_eq!(outcome(&retry), SubmissionOutcome::Accepted);
    assert!(retry.deduplicated);
    assert_eq!(state(&retry), OperationState::Accepted);
    assert_eq!(durable_events(&storage).await.len(), 1);
}

#[tokio::test]
async fn retry_with_missing_index_entry_fails_fast() {
    // If storage says Duplicate (the command exists in the durable log) but
    // the command index doesn't have it (inconsistency), the pipeline must
    // fail fast — NOT silently return Accepted (which would reproduce the
    // original blocker). This is the Fail Fast discipline: an inconsistent
    // projection is a corruption, not a reason to guess.
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let submitted = operation();

    // First submit — accepted (AlwaysAccepted returns Some).
    let first = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted.clone(),
    )
    .await
    .unwrap();
    assert_eq!(state(&first), OperationState::Accepted);

    // Retry with a lookup that returns None (command not in index).
    let result = submit(
        &storage,
        &grant,
        &resolver,
        &NotFoundLookup,
        &NoElicitationContractLookup,
        &issuer(),
        submitted,
    )
    .await;

    assert!(
        matches!(result, Err(AcceptanceError::CorruptRecord(_))),
        "missing index entry must fail fast, not silently return Accepted; got {result:?}"
    );
}
