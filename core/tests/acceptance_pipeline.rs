use std::future::ready;
use std::sync::atomic::{AtomicUsize, Ordering};

use patchbay_contracts::patchbay::{
    response_contract, spawn_request, typed_correlation, AcceptedOperation, ActorEndpointRef,
    ActorId, AdapterId, ApprovalDecision, ApprovalResponsePayload, AuthorityDomainId, CommandId,
    ContinuationAuthorityProvenance, DeviceId, ElicitationId, ElicitationResponsePayload,
    EndpointId, ExternalRuntimeRef, FailureCode, FreshSpawn, Generation, GrantId, LogicalTargetId,
    Lsn, Operation, OperationKind, OperationState, PayloadContentType, PayloadEnvelope,
    QuestionContract, ResponseContract, ResponseContractKind, ResponseOption, RuntimeGenerationRef,
    RuntimeSessionId, SessionActivityState, SessionConnectivityState, SessionReportSourceCursor,
    SpawnContinuation, SpawnRequest, SpawnTargetSpec, StoredEventKind, SubmissionOutcome,
    TargetScope, TargetScopeKind, TimeWindow, TypedCorrelation,
};
use patchbay_core::acceptance::{
    submit, submit_with_clock, AcceptanceError, ActiveElicitation, Authorized, Clock,
    CommandSnapshot, CommandStateLookup, ElicitationContractLookup, GrantCheck, GrantDenied,
    ResolvedGrantCheck, TargetBinding, TargetNotFound, TargetResolver, SPAWN_REQUEST_SCHEMA,
};
use patchbay_core::{
    authority::IssuerContext,
    session::{ingest_session_report, SessionRegistry, SessionReport, SpawnClaimQuery},
    storage::{RusqliteStorage, Storage},
};
use prost::Message;
use prost_types::Timestamp;

struct TestGrantCheck {
    authorized: bool,
    calls: AtomicUsize,
    resolved_calls: AtomicUsize,
}

impl TestGrantCheck {
    fn new(authorized: bool) -> Self {
        Self {
            authorized,
            calls: AtomicUsize::new(0),
            resolved_calls: AtomicUsize::new(0),
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
            Ok(Authorized {
                grant_id: Some(GrantId {
                    value: "test-grant".to_owned(),
                }),
                continuation_authority: None,
            })
        } else {
            Err(GrantDenied::NoGrant {
                actor: "operator".to_owned(),
                kind: operation_kind,
                target: "session".to_owned(),
            })
        })
    }

    fn check_resolved_at(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _issuer: &dyn IssuerContext,
        request: ResolvedGrantCheck<'_>,
    ) -> impl std::future::Future<Output = Result<Authorized, GrantDenied>> + Send {
        self.resolved_calls.fetch_add(1, Ordering::Relaxed);
        ready(Ok(request.authorization))
    }
}

struct CompoundGrantCheck {
    spawn_calls: AtomicUsize,
    replacement_calls: AtomicUsize,
}

impl CompoundGrantCheck {
    fn new() -> Self {
        Self {
            spawn_calls: AtomicUsize::new(0),
            replacement_calls: AtomicUsize::new(0),
        }
    }
}

impl GrantCheck for CompoundGrantCheck {
    async fn check(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        _target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        assert_eq!(operation_kind, OperationKind::Spawn);
        self.spawn_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Authorized {
            grant_id: Some(GrantId {
                value: "spawn-grant".to_owned(),
            }),
            continuation_authority: None,
        })
    }

    async fn check_resolved_at(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _issuer: &dyn IssuerContext,
        request: ResolvedGrantCheck<'_>,
    ) -> Result<Authorized, GrantDenied> {
        assert_eq!(request.operation_kind, OperationKind::Spawn);
        self.replacement_calls.fetch_add(1, Ordering::Relaxed);
        let TargetBinding::SpawnAdapter { claim, .. } = request.target_binding else {
            panic!("continuation must bind one spawn adapter");
        };
        Ok(Authorized {
            grant_id: request.authorization.grant_id,
            continuation_authority: Some(ContinuationAuthorityProvenance {
                exact_prior: claim.expected_prior.clone(),
                replacement_grant_id: Some(GrantId {
                    value: "replacement-grant".to_owned(),
                }),
                replacement_authority_kind: OperationKind::SessionManagement as i32,
            }),
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
        operation: &Operation,
        spawn_request: Option<&SpawnRequest>,
    ) -> impl std::future::Future<Output = Result<TargetBinding, TargetNotFound>> + Send {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let binding = if operation.kind == OperationKind::Spawn as i32 {
            let expected_prior = spawn_request.and_then(|request| match request.intent.as_ref() {
                Some(spawn_request::Intent::Continuation(continuation)) => {
                    continuation.prior.clone()
                }
                _ => None,
            });
            let claimed_generation = expected_prior
                .as_ref()
                .and_then(|prior| prior.external_runtime.as_ref())
                .and_then(|external| external.generation)
                .map_or(Generation { value: 1 }, |prior| Generation {
                    value: prior.value + 1,
                });
            TargetBinding::SpawnAdapter {
                adapter_id: AdapterId {
                    value: "pi".to_owned(),
                },
                claim: Box::new(patchbay_contracts::patchbay::SpawnGenerationClaim {
                    authority_domain_id: operation.authority_domain_id.clone(),
                    claim_operation_id: operation.command_id.clone(),
                    logical_target_id: expected_prior
                        .as_ref()
                        .and_then(|prior| prior.logical_target_id.clone())
                        .or_else(|| {
                            operation.command_id.as_ref().map(|id| LogicalTargetId {
                                value: id.value.clone(),
                            })
                        }),
                    expected_prior,
                    claimed_generation: Some(claimed_generation),
                }),
                continuation_authority: None,
            }
        } else {
            TargetBinding::RuntimeSession {
                adapter_id: AdapterId {
                    value: "pi".to_owned(),
                },
                deployment_scope: "local".to_owned(),
                runtime_session_id: RuntimeSessionId {
                    value: "session-1".to_owned(),
                },
                session_generation: Generation { value: 7 },
            }
        };
        ready(if self.found {
            Ok(binding)
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
            operation_kind: OperationKind::Instruct,
            target_scope: None,
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

fn verified_sender(test_issuer: &TestIssuer) -> ActorEndpointRef {
    ActorEndpointRef {
        actor_id: Some(test_issuer.actor.clone()),
        endpoint_id: Some(test_issuer.endpoint.clone()),
        device_id: Some(test_issuer.device.clone()),
        endpoint_generation: Some(test_issuer.generation),
    }
}

fn timestamp(seconds: i64) -> Timestamp {
    Timestamp { seconds, nanos: 0 }
}

fn validity_window(starts_at: i64, expires_at: i64) -> TimeWindow {
    TimeWindow {
        starts_at: Some(timestamp(starts_at)),
        expires_at: Some(timestamp(expires_at)),
    }
}

struct FixedClock(Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
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
        validity_window: Some(validity_window(1, 253_402_300_799)),
        submitted_at: Some(timestamp(1)),
        ..Operation::default()
    }
}

fn spawn_operation(intent: spawn_request::Intent, command: &str) -> Operation {
    let mut operation = operation();
    operation.command_id = Some(CommandId {
        value: command.to_owned(),
    });
    operation.kind = OperationKind::Spawn as i32;
    operation.target_scope = Some(TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        ..TargetScope::default()
    });
    operation.idempotency_key = format!("{command}-key");
    operation.payload = Some(PayloadEnvelope {
        payload: SpawnRequest {
            intent: Some(intent),
            target_spec: Some(SpawnTargetSpec {
                shape: "session".to_owned(),
                ..SpawnTargetSpec::default()
            }),
        }
        .encode_to_vec(),
        content_type: PayloadContentType::Protobuf as i32,
        schema_ref: SPAWN_REQUEST_SCHEMA.to_owned(),
    });
    operation
}

fn spawn_continuation() -> spawn_request::Intent {
    spawn_request::Intent::Continuation(SpawnContinuation {
        prior: Some(RuntimeGenerationRef {
            logical_target_id: Some(LogicalTargetId {
                value: "logical-1".to_owned(),
            }),
            external_runtime: Some(ExternalRuntimeRef {
                adapter_id: Some(AdapterId {
                    value: "pi".to_owned(),
                }),
                deployment_scope: "local".to_owned(),
                runtime_session_id: Some(RuntimeSessionId {
                    value: "session-1".to_owned(),
                }),
                generation: Some(Generation { value: 7 }),
            }),
        }),
    })
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

fn approval_response_operation() -> Operation {
    let mut operation = response_operation();
    operation.kind = OperationKind::ApprovalResponse as i32;
    operation.idempotency_key = "idempotency-approval-response-1".to_owned();
    operation.payload = Some(PayloadEnvelope {
        payload: ApprovalResponsePayload {
            decision: ApprovalDecision::Approved as i32,
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
        expected_responder_actor: Some(ActorId {
            value: "operator".to_owned(),
        }),
        is_terminal: false,
        winning_response: None,
    }
}

fn active_approval() -> ActiveElicitation {
    ActiveElicitation {
        contract: ResponseContract {
            contract_kind: ResponseContractKind::Approval as i32,
            ..ResponseContract::default()
        },
        expected_responder_actor: Some(ActorId {
            value: "operator".to_owned(),
        }),
        is_terminal: false,
        winning_response: None,
    }
}

fn response_fixture(kind: OperationKind) -> (Operation, ActiveElicitation) {
    match kind {
        OperationKind::ElicitationResponse => (response_operation(), active_question()),
        OperationKind::ApprovalResponse => (approval_response_operation(), active_approval()),
        _ => panic!("response fixture requires a response OperationKind"),
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
    durable_events_in(storage, &authority_domain()).await
}

async fn durable_events_in(
    storage: &RusqliteStorage,
    authority_domain_id: &AuthorityDomainId,
) -> Vec<patchbay_core::storage::RecordedEvent> {
    storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await
        .expect("test storage remains readable")
}

fn live_session_report() -> SessionReport {
    SessionReport {
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        deployment_scope: "local".to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "session-1".to_owned(),
        }),
        session_generation: Some(Generation { value: 7 }),
        connectivity: SessionConnectivityState::Live as i32,
        activity: SessionActivityState::Idle as i32,
        project: "patchbay".to_owned(),
        cwd: "/work/patchbay".to_owned(),
        name: "main".to_owned(),
        model: "provider/model".to_owned(),
        spawn_origin: None,
        source_cursor: Some(SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 1 }),
            revision: 1,
        }),
        continuation_context_status: 0,
    }
}

#[tokio::test]
async fn real_session_registry_resolves_same_domain_acceptance() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut sessions = SessionRegistry::new(authority_domain()).unwrap();
    ingest_session_report(
        &storage,
        &mut sessions,
        &authority_domain(),
        live_session_report(),
    )
    .await
    .unwrap();
    let grant = TestGrantCheck::new(true);

    let result = submit(
        &storage,
        &grant,
        &sessions,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        operation(),
    )
    .await
    .unwrap();

    assert_eq!(outcome(&result), SubmissionOutcome::Accepted);
    assert_eq!(failure(&result), FailureCode::Unspecified);
    assert_eq!(grant.calls.load(Ordering::Relaxed), 1);
    let events = durable_events(&storage).await;
    assert_eq!(
        events
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::Operation as i32)
            .count(),
        1
    );
}

#[tokio::test]
async fn real_session_registry_domain_mismatch_rejects_without_command_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut sessions = SessionRegistry::new(authority_domain()).unwrap();
    ingest_session_report(
        &storage,
        &mut sessions,
        &authority_domain(),
        live_session_report(),
    )
    .await
    .unwrap();
    let other_domain = AuthorityDomainId {
        value: "authority-other".to_owned(),
    };
    let mut submitted = operation();
    submitted.authority_domain_id = Some(other_domain.clone());
    let grant = TestGrantCheck::new(true);
    let main_before = durable_events_in(&storage, &authority_domain()).await;
    let other_before = durable_events_in(&storage, &other_domain).await;

    let result = submit(
        &storage,
        &grant,
        &sessions,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &TestIssuer::new(other_domain.clone()),
        submitted,
    )
    .await
    .unwrap();

    assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
    assert_eq!(failure(&result), FailureCode::TargetNotFound);
    assert_eq!(result.reason_code, "target_not_found");
    assert_eq!(grant.calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        durable_events_in(&storage, &authority_domain()).await,
        main_before
    );
    assert_eq!(
        durable_events_in(&storage, &other_domain).await,
        other_before
    );
    assert!(main_before
        .iter()
        .all(|event| event.payload.kind != StoredEventKind::Operation as i32));
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

    let mut missing_window = operation();
    missing_window.validity_window = None;
    submissions.push(missing_window);

    let mut missing_submitted_at = operation();
    missing_submitted_at.submitted_at = None;
    submissions.push(missing_submitted_at);

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
async fn matching_verified_responder_reaches_grant_target_and_append_for_both_response_kinds() {
    for kind in [
        OperationKind::ApprovalResponse,
        OperationKind::ElicitationResponse,
    ] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let grant = TestGrantCheck::new(true);
        let resolver = TestTargetResolver::new(true);
        let (mut submitted, active) = response_fixture(kind);
        submitted.sender.as_mut().unwrap().actor_id = Some(ActorId {
            value: "forged-payload-actor".to_owned(),
        });

        let result = submit(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &TerminalRetryLookup { active },
            &issuer(),
            submitted,
        )
        .await
        .unwrap();

        assert_eq!(outcome(&result), SubmissionOutcome::Accepted, "{kind:?}");
        assert_eq!(failure(&result), FailureCode::Unspecified, "{kind:?}");
        assert_eq!(grant.calls.load(Ordering::Relaxed), 1, "{kind:?}");
        assert_eq!(resolver.calls.load(Ordering::Relaxed), 1, "{kind:?}");
        assert_eq!(durable_events(&storage).await.len(), 1, "{kind:?}");
    }
}

#[tokio::test]
async fn responder_mismatch_or_missing_expected_actor_denies_before_grant_target_and_append() {
    for kind in [
        OperationKind::ApprovalResponse,
        OperationKind::ElicitationResponse,
    ] {
        for responder_evidence in ["mismatch", "missing", "empty"] {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let grant = TestGrantCheck::new(true);
            let resolver = TestTargetResolver::new(true);
            let (submitted, mut active) = response_fixture(kind);
            let mut test_issuer = issuer();
            match responder_evidence {
                "mismatch" => test_issuer.actor.value = "different-operator".to_owned(),
                "missing" => active.expected_responder_actor = None,
                "empty" => active.expected_responder_actor = Some(ActorId::default()),
                _ => unreachable!(),
            }

            let result = submit(
                &storage,
                &grant,
                &resolver,
                &AlwaysAccepted,
                &TerminalRetryLookup { active },
                &test_issuer,
                submitted,
            )
            .await
            .unwrap();

            assert_eq!(
                outcome(&result),
                SubmissionOutcome::Rejected,
                "{kind:?} {responder_evidence}"
            );
            assert_eq!(
                failure(&result),
                FailureCode::AuthorizationDenied,
                "{kind:?} {responder_evidence}"
            );
            assert_eq!(
                result.reason_code, "authorization_denied",
                "{kind:?} {responder_evidence}"
            );
            assert_eq!(
                state(&result),
                OperationState::Unspecified,
                "{kind:?} {responder_evidence}"
            );
            assert!(
                result.decision_grant_id.is_none(),
                "{kind:?} {responder_evidence}"
            );
            assert_eq!(
                result.diagnostic_message,
                "verified issuer is not authorized to answer this elicitation",
                "{kind:?} {responder_evidence}"
            );
            assert_eq!(
                grant.calls.load(Ordering::Relaxed),
                0,
                "{kind:?} {responder_evidence}"
            );
            assert_eq!(
                resolver.calls.load(Ordering::Relaxed),
                0,
                "{kind:?} {responder_evidence}"
            );
            assert!(
                durable_events(&storage).await.is_empty(),
                "{kind:?} {responder_evidence}"
            );
        }
    }
}

#[tokio::test]
async fn responder_authority_precedes_payload_diagnostics_for_known_elicitation() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let (mut submitted, active) = response_fixture(OperationKind::ElicitationResponse);
    submitted.payload = None;
    let mut wrong_issuer = issuer();
    wrong_issuer.actor.value = "different-operator".to_owned();

    let result = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &TerminalRetryLookup { active },
        &wrong_issuer,
        submitted,
    )
    .await
    .unwrap();

    assert_eq!(failure(&result), FailureCode::AuthorizationDenied);
    assert_eq!(result.reason_code, "authorization_denied");
    assert_eq!(grant.calls.load(Ordering::Relaxed), 0);
    assert_eq!(resolver.calls.load(Ordering::Relaxed), 0);
    assert!(durable_events(&storage).await.is_empty());
}

#[tokio::test]
async fn unknown_elicitation_and_matching_responder_malformed_payload_remain_validation_failed() {
    let unknown_storage = RusqliteStorage::open_in_memory().unwrap();
    let unknown_grant = TestGrantCheck::new(true);
    let unknown_resolver = TestTargetResolver::new(true);
    let unknown = submit(
        &unknown_storage,
        &unknown_grant,
        &unknown_resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        response_operation(),
    )
    .await
    .unwrap();
    assert_eq!(failure(&unknown), FailureCode::ValidationFailed);
    assert_eq!(unknown.reason_code, "validation_failed");
    assert_eq!(unknown_grant.calls.load(Ordering::Relaxed), 0);
    assert_eq!(unknown_resolver.calls.load(Ordering::Relaxed), 0);
    assert!(durable_events(&unknown_storage).await.is_empty());

    let malformed_storage = RusqliteStorage::open_in_memory().unwrap();
    let malformed_grant = TestGrantCheck::new(true);
    let malformed_resolver = TestTargetResolver::new(true);
    let (mut malformed_operation, active) = response_fixture(OperationKind::ElicitationResponse);
    malformed_operation.payload = None;
    let malformed = submit(
        &malformed_storage,
        &malformed_grant,
        &malformed_resolver,
        &AlwaysAccepted,
        &TerminalRetryLookup { active },
        &issuer(),
        malformed_operation,
    )
    .await
    .unwrap();
    assert_eq!(failure(&malformed), FailureCode::ValidationFailed);
    assert_eq!(malformed.reason_code, "validation_failed");
    assert_eq!(malformed_grant.calls.load(Ordering::Relaxed), 0);
    assert_eq!(malformed_resolver.calls.load(Ordering::Relaxed), 0);
    assert!(durable_events(&malformed_storage).await.is_empty());
}

#[tokio::test]
async fn expired_operation_rejects_before_grant_without_durable_state() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let mut submitted = operation();
    submitted.validity_window = Some(validity_window(1, 2));
    submitted.submitted_at = Some(timestamp(1));

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
    assert_eq!(failure(&result), FailureCode::Expired);
    assert_eq!(grant.calls.load(Ordering::Relaxed), 0);
    assert_eq!(resolver.calls.load(Ordering::Relaxed), 0);
    assert!(durable_events(&storage).await.is_empty());
}

#[tokio::test]
async fn operation_inside_active_window_is_accepted() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let mut submitted = operation();
    submitted.validity_window = Some(validity_window(10, 30));
    submitted.submitted_at = Some(timestamp(15));

    let result = submit_with_clock(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted,
        &FixedClock(timestamp(20)),
    )
    .await
    .unwrap();

    assert_eq!(outcome(&result), SubmissionOutcome::Accepted);
    assert_eq!(failure(&result), FailureCode::Unspecified);
    assert_eq!(durable_events(&storage).await.len(), 1);
}

#[tokio::test]
async fn validity_window_start_is_inclusive_and_expiry_is_exclusive() {
    let start_storage = RusqliteStorage::open_in_memory().unwrap();
    let mut at_start = operation();
    at_start.validity_window = Some(validity_window(10, 30));
    at_start.submitted_at = Some(timestamp(10));
    let start_result = submit_with_clock(
        &start_storage,
        &TestGrantCheck::new(true),
        &TestTargetResolver::new(true),
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        at_start,
        &FixedClock(timestamp(10)),
    )
    .await
    .unwrap();
    assert_eq!(outcome(&start_result), SubmissionOutcome::Accepted);

    let expiry_storage = RusqliteStorage::open_in_memory().unwrap();
    let mut at_expiry = operation();
    at_expiry.validity_window = Some(validity_window(10, 30));
    at_expiry.submitted_at = Some(timestamp(20));
    let expiry_result = submit_with_clock(
        &expiry_storage,
        &TestGrantCheck::new(true),
        &TestTargetResolver::new(true),
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        at_expiry,
        &FixedClock(timestamp(30)),
    )
    .await
    .unwrap();
    assert_eq!(outcome(&expiry_result), SubmissionOutcome::Rejected);
    assert_eq!(failure(&expiry_result), FailureCode::Expired);
    assert!(durable_events(&expiry_storage).await.is_empty());
}

#[tokio::test]
async fn not_yet_valid_and_future_dated_operations_fail_validation() {
    for (now, submitted_at) in [(9, 9), (20, 21)] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let grant = TestGrantCheck::new(true);
        let resolver = TestTargetResolver::new(true);
        let mut submitted = operation();
        submitted.validity_window = Some(validity_window(10, 30));
        submitted.submitted_at = Some(timestamp(submitted_at));

        let result = submit_with_clock(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &NoElicitationContractLookup,
            &issuer(),
            submitted,
            &FixedClock(timestamp(now)),
        )
        .await
        .unwrap();

        assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
        assert_eq!(failure(&result), FailureCode::ValidationFailed);
        assert_eq!(grant.calls.load(Ordering::Relaxed), 0);
        assert!(durable_events(&storage).await.is_empty());
    }
}

#[tokio::test]
async fn malformed_validity_windows_fail_validation() {
    let mut reversed = operation();
    reversed.validity_window = Some(validity_window(30, 10));
    reversed.submitted_at = Some(timestamp(20));

    let mut invalid_timestamp = operation();
    invalid_timestamp.validity_window = Some(TimeWindow {
        starts_at: Some(Timestamp {
            seconds: 10,
            nanos: 1_000_000_000,
        }),
        expires_at: Some(timestamp(30)),
    });
    invalid_timestamp.submitted_at = Some(timestamp(20));

    for submitted in [reversed, invalid_timestamp] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let grant = TestGrantCheck::new(true);
        let resolver = TestTargetResolver::new(true);
        let result = submit_with_clock(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &NoElicitationContractLookup,
            &issuer(),
            submitted,
            &FixedClock(timestamp(20)),
        )
        .await
        .unwrap();

        assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
        assert_eq!(failure(&result), FailureCode::ValidationFailed);
        assert_eq!(grant.calls.load(Ordering::Relaxed), 0);
        assert!(durable_events(&storage).await.is_empty());
    }
}

#[tokio::test]
async fn submitted_at_must_be_inside_the_half_open_window() {
    for submitted_at in [9, 30] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let grant = TestGrantCheck::new(true);
        let resolver = TestTargetResolver::new(true);
        let mut submitted = operation();
        submitted.validity_window = Some(validity_window(10, 30));
        submitted.submitted_at = Some(timestamp(submitted_at));

        let result = submit_with_clock(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &NoElicitationContractLookup,
            &issuer(),
            submitted,
            &FixedClock(timestamp(20)),
        )
        .await
        .unwrap();

        assert_eq!(outcome(&result), SubmissionOutcome::Rejected);
        assert_eq!(failure(&result), FailureCode::ValidationFailed);
        assert_eq!(grant.calls.load(Ordering::Relaxed), 0);
        assert!(durable_events(&storage).await.is_empty());
    }
}

#[tokio::test]
async fn replay_after_window_expiry_is_rejected_without_a_second_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let mut submitted = operation();
    submitted.validity_window = Some(validity_window(10, 30));
    submitted.submitted_at = Some(timestamp(15));

    let first = submit_with_clock(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted.clone(),
        &FixedClock(timestamp(20)),
    )
    .await
    .unwrap();
    assert_eq!(outcome(&first), SubmissionOutcome::Accepted);

    let replay = submit_with_clock(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        submitted,
        &FixedClock(timestamp(30)),
    )
    .await
    .unwrap();
    assert_eq!(outcome(&replay), SubmissionOutcome::Rejected);
    assert_eq!(failure(&replay), FailureCode::Expired);
    assert!(!replay.deduplicated);
    assert_eq!(durable_events(&storage).await.len(), 1);
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
async fn continuation_acceptance_round_trips_compound_claim_while_fresh_uses_single_grant() {
    let continuation_storage = RusqliteStorage::open_in_memory().unwrap();
    let continuation_grant = CompoundGrantCheck::new();
    let continuation_resolver = TestTargetResolver::new(true);

    let accepted_continuation = submit(
        &continuation_storage,
        &continuation_grant,
        &continuation_resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        spawn_operation(spawn_continuation(), "continuation-command"),
    )
    .await
    .unwrap();

    assert_eq!(outcome(&accepted_continuation), SubmissionOutcome::Accepted);
    assert_eq!(failure(&accepted_continuation), FailureCode::Unspecified);
    assert_eq!(continuation_grant.spawn_calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        continuation_grant.replacement_calls.load(Ordering::Relaxed),
        1
    );
    assert_eq!(continuation_resolver.calls.load(Ordering::Relaxed), 1);
    let continuation_events = durable_events(&continuation_storage).await;
    assert_eq!(continuation_events.len(), 2, "source plus acceptance audit");
    assert_eq!(
        continuation_events[0].payload.kind,
        StoredEventKind::SpawnClaim as i32
    );
    let accepted_claim = patchbay_contracts::patchbay::SpawnClaimEvent::decode(
        continuation_events[0].payload.payload.as_slice(),
    )
    .expect("accepted continuation claim decodes");
    let patchbay_contracts::patchbay::spawn_claim_event::Mutation::Accepted(accepted_claim) =
        accepted_claim.mutation.expect("claim mutation")
    else {
        panic!("expected accepted claim mutation");
    };
    let prior = match spawn_continuation() {
        spawn_request::Intent::Continuation(continuation) => continuation.prior,
        _ => unreachable!(),
    };
    assert_eq!(accepted_claim.claim.as_ref().unwrap().expected_prior, prior);
    assert_eq!(
        accepted_claim
            .accepted_operation
            .as_ref()
            .and_then(|accepted| accepted.authorizing_grant_id.as_ref())
            .map(|grant| grant.value.as_str()),
        Some("spawn-grant")
    );
    assert_eq!(
        accepted_claim
            .compound_authority
            .as_ref()
            .and_then(|authority| authority.replacement_grant_id.as_ref())
            .map(|grant| grant.value.as_str()),
        Some("replacement-grant")
    );
    let replayed_claims = patchbay_core::session::rebuild_spawn_claims_from_log(
        &continuation_storage,
        &authority_domain(),
    )
    .await
    .expect("claim replay succeeds");
    assert_eq!(
        replayed_claims
            .claim_for_operation(&CommandId {
                value: "continuation-command".to_owned(),
            })
            .expect("replayed claim")
            .compound_authority,
        accepted_claim.compound_authority
    );

    let fresh_storage = RusqliteStorage::open_in_memory().unwrap();
    let fresh_grant = TestGrantCheck::new(true);
    let fresh_resolver = TestTargetResolver::new(true);
    let accepted = submit(
        &fresh_storage,
        &fresh_grant,
        &fresh_resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        spawn_operation(
            spawn_request::Intent::Fresh(FreshSpawn {}),
            "fresh-spawn-command",
        ),
    )
    .await
    .unwrap();

    assert_eq!(outcome(&accepted), SubmissionOutcome::Accepted);
    assert_eq!(fresh_grant.calls.load(Ordering::Relaxed), 1);
    assert_eq!(fresh_grant.resolved_calls.load(Ordering::Relaxed), 1);
    assert_eq!(fresh_resolver.calls.load(Ordering::Relaxed), 1);
    let events = durable_events(&fresh_storage).await;
    assert_eq!(events.len(), 2, "source plus acceptance audit");
    assert_eq!(events[0].payload.kind, StoredEventKind::SpawnClaim as i32);
    let accepted =
        patchbay_contracts::patchbay::SpawnClaimEvent::decode(events[0].payload.payload.as_slice())
            .expect("accepted fresh-spawn claim decodes");
    let patchbay_contracts::patchbay::spawn_claim_event::Mutation::Accepted(accepted) =
        accepted.mutation.expect("claim mutation")
    else {
        panic!("expected accepted claim mutation");
    };
    assert_eq!(
        accepted
            .accepted_operation
            .and_then(|accepted| accepted.authorizing_grant_id),
        Some(GrantId {
            value: "test-grant".to_owned(),
        })
    );
    assert!(accepted.compound_authority.is_none());
    assert_eq!(
        accepted.claim.and_then(|claim| claim.claimed_generation),
        Some(Generation { value: 1 })
    );
}

#[tokio::test]
async fn continuation_fence_rejects_new_n_bound_submission_canonically() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let continuation_grant = CompoundGrantCheck::new();
    let resolver = TestTargetResolver::new(true);
    let continuation = submit(
        &storage,
        &continuation_grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        spawn_operation(spawn_continuation(), "continuation-command"),
    )
    .await
    .unwrap();
    assert_eq!(outcome(&continuation), SubmissionOutcome::Accepted);

    let grant = TestGrantCheck::new(true);
    let mut n_bound = operation();
    n_bound.command_id = Some(CommandId {
        value: "after-fence".to_owned(),
    });
    n_bound.idempotency_key = "after-fence-key".to_owned();
    let rejected = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &issuer(),
        n_bound,
    )
    .await
    .unwrap();
    assert_eq!(outcome(&rejected), SubmissionOutcome::Rejected);
    assert_eq!(failure(&rejected), FailureCode::Superseded);
    assert_eq!(rejected.reason_code, "replacement_pending");
    assert!(rejected.diagnostic_message.contains("replacement pending"));
    assert_eq!(
        durable_events(&storage)
            .await
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::Operation as i32)
            .count(),
        0
    );
}

#[tokio::test]
async fn new_command_is_durably_recorded_before_acceptance_returns() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let submitted = operation();
    let test_issuer = issuer();

    let result = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &test_issuer,
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
    let recorded = AcceptedOperation::decode(events[0].payload.payload.as_slice())
        .unwrap()
        .operation
        .unwrap();
    // Durable sender attribution is normalized from authenticated issuer
    // evidence; it is not an assertion that caller-supplied sender data persists.
    let mut expected = submitted;
    expected.sender = Some(verified_sender(&test_issuer));
    assert_eq!(recorded, expected);
}

#[tokio::test]
async fn mismatched_sender_claim_is_overwritten_by_verified_issuer() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grant = TestGrantCheck::new(true);
    let resolver = TestTargetResolver::new(true);
    let test_issuer = issuer();
    let mut submitted = operation();
    submitted.sender = Some(ActorEndpointRef {
        actor_id: Some(ActorId {
            value: "forged-actor".to_owned(),
        }),
        endpoint_id: Some(EndpointId {
            value: "forged-endpoint".to_owned(),
        }),
        device_id: Some(DeviceId {
            value: "forged-device".to_owned(),
        }),
        endpoint_generation: Some(Generation { value: 999 }),
    });

    let result = submit(
        &storage,
        &grant,
        &resolver,
        &AlwaysAccepted,
        &NoElicitationContractLookup,
        &test_issuer,
        submitted,
    )
    .await
    .unwrap();
    assert_eq!(outcome(&result), SubmissionOutcome::Accepted);

    let events = durable_events(&storage).await;
    let recorded = AcceptedOperation::decode(events[0].payload.payload.as_slice())
        .unwrap()
        .operation
        .unwrap();
    assert_eq!(recorded.sender, Some(verified_sender(&test_issuer)));
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
            operation_kind: OperationKind::Instruct,
            target_scope: None,
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
async fn terminal_response_retry_ignores_forged_sender_against_normalized_winner() {
    for kind in [
        OperationKind::ApprovalResponse,
        OperationKind::ElicitationResponse,
    ] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let grant = TestGrantCheck::new(true);
        let resolver = TestTargetResolver::new(true);
        let (mut submitted, active) = response_fixture(kind);
        submitted.sender = Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: "forged-actor".to_owned(),
            }),
            endpoint_id: Some(EndpointId {
                value: "forged-endpoint".to_owned(),
            }),
            device_id: Some(DeviceId {
                value: "forged-device".to_owned(),
            }),
            endpoint_generation: Some(Generation { value: 999 }),
        });
        let test_issuer = issuer();

        let first = submit(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &TerminalRetryLookup {
                active: active.clone(),
            },
            &test_issuer,
            submitted.clone(),
        )
        .await
        .unwrap();
        assert_eq!(outcome(&first), SubmissionOutcome::Accepted, "{kind:?}");

        let events = durable_events(&storage).await;
        assert_eq!(events.len(), 1, "{kind:?}");
        let winning_response = AcceptedOperation::decode(events[0].payload.payload.as_slice())
            .unwrap()
            .operation
            .unwrap();
        assert_eq!(
            winning_response.sender,
            Some(verified_sender(&test_issuer)),
            "the production winner must carry normalized sender identity for {kind:?}"
        );
        assert_ne!(winning_response.sender, submitted.sender, "{kind:?}");

        let retry = submit(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &TerminalRetryLookup {
                active: ActiveElicitation {
                    is_terminal: true,
                    winning_response: Some(winning_response.clone()),
                    ..active.clone()
                },
            },
            &test_issuer,
            submitted.clone(),
        )
        .await
        .unwrap();

        assert_eq!(outcome(&retry), SubmissionOutcome::Accepted, "{kind:?}");
        assert!(retry.deduplicated, "{kind:?}");
        assert_eq!(state(&retry), OperationState::Accepted, "{kind:?}");
        assert_eq!(durable_events(&storage).await.len(), 1, "{kind:?}");

        let mut wrong_issuer = issuer();
        wrong_issuer.actor.value = "different-operator".to_owned();
        let wrong_actor_retry = submit(
            &storage,
            &grant,
            &resolver,
            &AlwaysAccepted,
            &TerminalRetryLookup {
                active: ActiveElicitation {
                    is_terminal: true,
                    winning_response: Some(winning_response),
                    ..active
                },
            },
            &wrong_issuer,
            submitted,
        )
        .await
        .unwrap();

        assert_eq!(
            outcome(&wrong_actor_retry),
            SubmissionOutcome::Rejected,
            "{kind:?}"
        );
        assert_eq!(
            failure(&wrong_actor_retry),
            FailureCode::AuthorizationDenied,
            "{kind:?}"
        );
        assert_eq!(
            wrong_actor_retry.reason_code, "authorization_denied",
            "{kind:?}"
        );
        assert!(!wrong_actor_retry.deduplicated, "{kind:?}");
        assert_eq!(
            grant.calls.load(Ordering::Relaxed),
            2,
            "wrong actor must be denied before grant evaluation for {kind:?}"
        );
        assert_eq!(
            resolver.calls.load(Ordering::Relaxed),
            2,
            "wrong actor must be denied before target/dedup work for {kind:?}"
        );
        assert_eq!(durable_events(&storage).await.len(), 1, "{kind:?}");
    }
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
