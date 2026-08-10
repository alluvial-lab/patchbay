use std::{collections::HashSet, pin::Pin, sync::Arc, time::Duration};

use patchbay_contracts::patchbay::{
    ActorEndpointRef, ActorId, AuditEventKind, AuditRecord, AuthorityDomainId, CommandTransition,
    EnterSecurityLockdownRequest, EnterSecurityLockdownResult,
    DiagnosticsResult, GrantRevocationEffect, GrantRevocationPolicy, Generation, Revocation,
    ControlSurfaceRevocation, OperatorSessionRevocation, RevokeAllOperatorSessionsRequest,
    RevokeAllOperatorSessionsResult, RevokeControlSurfaceEndpointRequest,
    RevokeControlSurfacePrincipalRequest, RevokeControlSurfaceResult,
    EnrollControlSurfacePrincipalRequest, EnrollControlSurfacePrincipalResult, EventId,
    LoadSnapshotRequest, LoadSnapshotResponse, LoadSecuritySnapshotRequest,
    LoadSecuritySnapshotResponse, Lsn,
    QueryDiagnosticsRequest, QueryDiagnosticsResponse, RecordControlSurfaceAuditRequest,
    RevokeGrantRequest, RevokeGrantResult, SecurityLockdownEntered,
    RecordControlSurfaceAuditResponse, RevokeOperatorSessionRequest, RevokeOperatorSessionResult,
    StoredEventKind, SubmissionOutcome, SubmissionResult, SubmitRequest,
    SubscribeEvent, SubscribeRequest, Observation, ObservationKind, PayloadContentType,
    TypedCorrelation, typed_correlation, OperationState, FailureCode, TargetScope,
    TargetScopeKind,
    VerifyOperatorPasswordRequest, VerifyOperatorPasswordResult,
};
use patchbay_core::{
    acceptance::{
        self, AcceptanceError, CommandRecord, CommandStateLookup, GrantCheck, OperationPosture,
        OperationPostureDenied,
    },
    security,
    time::{Clock, SystemClock},
    audit::{AuditReceipt, AuditSink, DurableAuditSink, RequiredAuditFanout, StderrAuditSink},
    authority::{authorize_self_revocation_at, hash_principal_credential, GrantAdministrationDenied, GrantRecord, IssuerContext, IssuerRef, OperatorError},
    diagnostics::{self, AuthorityDomainTargetResolver, ValidatedDiagnosticsQuery},
    storage::{
        validate_next_replay_event, AuditPageSpec, AuditRecordDraft, CoreGenerationStore,
        RecordedEvent, Storage, StorageError,
    },
};
use prost::Message;
use tokio_stream::{self as stream, Stream};
use tonic::{service::Interceptor, Code, Request, Response, Status};
use tonic_types::{ErrorDetails, StatusExt};

use crate::{
    decision_gate::CoreDecisionGate,
    identity::issue_principal,
    issuer::MetadataIssuerContext,
    login_security::{
        LoginAuditEvent, LoginAuditOutcome, LoginAuditSink, LoginLimitDimension, LoginLimiter,
        StderrLoginAuditSink,
    },
    operator_session::{OperatorSessionBinding, DEFAULT_OPERATOR_SESSION_TTL},
    rpc::control_service_server::ControlService,
    snapshot::decode_compatible_session_checkpoint,
    state::ProjectionState,
};

pub const CORE_SECRET_HEADER: &str = "x-patchbay-core-secret";

#[derive(Clone)]
pub struct CoreSecretInterceptor {
    expected: Vec<u8>,
}

impl CoreSecretInterceptor {
    pub fn new(secret: impl Into<String>) -> Result<Self, String> {
        let secret = secret.into();
        if secret.is_empty() {
            return Err("PATCHBAY_CORE_SECRET must be configured and non-empty".to_owned());
        }
        if !secret.is_ascii() {
            return Err("PATCHBAY_CORE_SECRET must contain ASCII metadata characters".to_owned());
        }
        Ok(Self {
            expected: secret.into_bytes(),
        })
    }
}

impl Interceptor for CoreSecretInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let supplied = request
            .metadata()
            .get(CORE_SECRET_HEADER)
            .map(|value| value.as_encoded_bytes());
        if !supplied.is_some_and(|value| constant_time_eq(value, &self.expected)) {
            return Err(Status::unauthenticated("invalid core principal secret"));
        }
        Ok(request)
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[derive(Clone)]
pub struct ControlServiceImpl<S> {
    pub(crate) storage: S,
    pub(crate) state: ProjectionState,
    pub(crate) authority_domain_id: AuthorityDomainId,
    login_limiter: LoginLimiter,
    login_audit: Arc<dyn LoginAuditSink>,
    audit: Arc<dyn AuditSink>,
    clock: Arc<dyn Clock>,
    decision_gate: CoreDecisionGate,
}

impl<S> ControlServiceImpl<S>
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    pub async fn new(storage: S, authority_domain_id: AuthorityDomainId) -> Result<Self, String> {
        Self::new_with_clock_security_and_gate(
            storage,
            authority_domain_id,
            DEFAULT_OPERATOR_SESSION_TTL,
            LoginLimiter::default(),
            Arc::new(StderrLoginAuditSink),
            Arc::new(SystemClock),
            CoreDecisionGate::default(),
        )
        .await
    }

    pub async fn new_with_security(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        operator_session_ttl: Duration,
        login_limiter: LoginLimiter,
        login_audit: Arc<dyn LoginAuditSink>,
    ) -> Result<Self, String> {
        Self::new_with_clock_security_and_gate(
            storage,
            authority_domain_id,
            operator_session_ttl,
            login_limiter,
            login_audit,
            Arc::new(SystemClock),
            CoreDecisionGate::default(),
        )
        .await
    }

    pub async fn new_with_security_and_decision_gate(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        operator_session_ttl: Duration,
        login_limiter: LoginLimiter,
        login_audit: Arc<dyn LoginAuditSink>,
        decision_gate: CoreDecisionGate,
    ) -> Result<Self, String> {
        Self::new_with_clock_security_and_gate(
            storage,
            authority_domain_id,
            operator_session_ttl,
            login_limiter,
            login_audit,
            Arc::new(SystemClock),
            decision_gate,
        )
        .await
    }

    pub async fn new_with_clock(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, String> {
        Self::new_with_clock_security_and_gate(
            storage,
            authority_domain_id,
            DEFAULT_OPERATOR_SESSION_TTL,
            LoginLimiter::default(),
            Arc::new(StderrLoginAuditSink),
            clock,
            CoreDecisionGate::default(),
        )
        .await
    }

    async fn new_with_clock_security_and_gate(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        operator_session_ttl: Duration,
        login_limiter: LoginLimiter,
        login_audit: Arc<dyn LoginAuditSink>,
        clock: Arc<dyn Clock>,
        decision_gate: CoreDecisionGate,
    ) -> Result<Self, String> {
        if authority_domain_id.value.is_empty() {
            return Err("authority domain id must not be empty".to_owned());
        }
        let state = ProjectionState::rebuild_with_session_ttl_and_gate(
            &storage,
            &authority_domain_id,
            operator_session_ttl,
            decision_gate.clone(),
        )
        .await?;
        let durable = Arc::new(DurableAuditSink::new(
            storage.clone(),
            authority_domain_id.clone(),
        ));
        let audit: Arc<dyn AuditSink> = Arc::new(RequiredAuditFanout::new(
            durable,
            vec![Arc::new(StderrAuditSink)],
        ));
        Ok(Self {
            storage,
            state,
            authority_domain_id,
            login_limiter,
            login_audit,
            audit,
            clock,
            decision_gate,
        })
    }

    pub async fn is_bootstrapped(&self) -> bool {
        self.state.operator_exists().await
    }

    #[must_use]
    pub fn projection_state(&self) -> &ProjectionState {
        &self.state
    }
}

type SubscribeStream = Pin<Box<dyn Stream<Item = Result<SubscribeEvent, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl<S> ControlService for ControlServiceImpl<S>
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    async fn submit(
        &self,
        request: Request<SubmitRequest>,
    ) -> Result<Response<SubmissionResult>, Status> {
        let operation = request
            .get_ref()
            .operation
            .clone()
            .ok_or_else(|| Status::invalid_argument("submit request is missing operation"))?;
        let authority_domain_id = operation
            .authority_domain_id
            .clone()
            .ok_or_else(|| Status::invalid_argument("operation is missing authority_domain_id"))?;
        self.require_configured_domain(&authority_domain_id)?;
        let _ = self
            .issuer_from_request(&request, authority_domain_id.clone())
            .await?;

        // Reconcile and submit under one gate. Pre-submit catch-up repairs the
        // projection after a prior append whose handler did not complete; the
        // post-append catch-up makes a newly durable command visible before the
        // next submit acquires the gate.
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, authority_domain_id.clone())
            .await?;
        let result = match acceptance::submit_with_clock_and_posture(
            &self.storage,
            self.state.grant_check(),
            self.state.target_resolver(),
            self.state.state_lookup(),
            self.state.elicitation_contract_lookup(),
            self.state.operation_posture(),
            &issuer,
            operation.clone(),
            self.clock.as_ref(),
        )
        .await {
            Ok(result) => result,
            Err(error) => {
                self.audit_submission_unknown(&issuer, &operation).await?;
                return Err(map_acceptance_error_to_status(error));
            }
        };
        if result.outcome == SubmissionOutcome::Rejected as i32 {
            self.audit_submission_rejection(&issuer, &operation, &result)
                .await?;
        }
        if result.outcome == SubmissionOutcome::Accepted as i32 && !result.deduplicated {
            self.state
                .catch_up(&self.storage, &authority_domain_id)
                .await
                .map_err(map_storage_error_to_status)?;
        }

        Ok(Response::new(result))
    }

    type SubscribeStream = SubscribeStream;

    async fn enter_security_lockdown(
        &self,
        request: Request<EnterSecurityLockdownRequest>,
    ) -> Result<Response<EnterSecurityLockdownResult>, Status> {
        let requested_domain = required_domain(request.get_ref().authority_domain_id.clone())?;
        self.require_configured_domain(&requested_domain)?;
        validate_control_surface_reason(&request.get_ref().reason_code)?;
        let _ = self
            .issuer_from_request(&request, requested_domain.clone())
            .await?;

        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &requested_domain)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, requested_domain.clone())
            .await?;
        let scope = authority_domain_scope();
        let evaluated_at = self.clock.now();
        let grant = match self
            .state
            .grant_check()
            .check_at(
                &requested_domain,
                &issuer,
                patchbay_contracts::patchbay::OperationKind::SessionManagement,
                &scope,
                &evaluated_at,
            )
            .await
        {
            Ok(authorized) => authorized.grant_id,
            Err(_) => {
                let mut audit = AuditRecordDraft::new(evaluated_at, AuditEventKind::AuthorizationFailed);
                audit.actor_id = issuer.verified_actor().cloned();
                audit.endpoint_id = issuer.verified_endpoint().cloned();
                audit.device_id = issuer.verified_device().cloned();
                audit.target_scope = Some(scope);
                audit.failure_code = Some(FailureCode::AuthorizationDenied);
                audit.reason_code = "security_lockdown_authorization_denied".to_owned();
                self.record_audit(audit).await?;
                return Err(Status::permission_denied(
                    "security lockdown requires authority-domain session-management grant",
                ));
            }
        };
        let grant = grant.ok_or_else(|| Status::internal("lockdown authorization omitted grant provenance"))?;

        let current = self.state.lockdown_state().await;
        if current.active {
            let entered_event_id = current.entered_event_id.clone();
            let generation = issuer
                .verified_actor()
                .map(|actor| async { self.state.current_operator_session_generation(actor).await });
            let generation = match generation {
                Some(future) => future.await,
                None => return Err(Status::internal("verified lockdown issuer has no actor")),
            };
            let actor = issuer
                .verified_actor()
                .cloned()
                .ok_or_else(|| Status::internal("verified lockdown issuer has no actor"))?;
            let mut audit = AuditRecordDraft::new(evaluated_at, AuditEventKind::LockdownEntered);
            audit.actor_id = Some(actor);
            audit.endpoint_id = issuer.verified_endpoint().cloned();
            audit.device_id = issuer.verified_device().cloned();
            audit.grant_id = Some(grant.clone());
            audit.target_scope = Some(authority_domain_scope());
            audit.reason_code = "security_lockdown_already_active".to_owned();
            self.record_audit(audit).await?;
            return Ok(Response::new(EnterSecurityLockdownResult {
                lockdown: Some(current),
                lockdown_event_id: entered_event_id,
                already_active: true,
                affected_runtime_session_count: self.state.current_runtime_session_count().await,
                invalidated_through_operator_session_generation: Some(generation),
            }));
        }

        let actor = issuer
            .verified_actor()
            .cloned()
            .ok_or_else(|| Status::internal("verified lockdown issuer has no actor"))?;
        let generation = self.state.current_operator_session_generation(&actor).await;
        let affected = self.state.current_runtime_session_count().await;
        let occurred_at = self.clock.now();
        let source = security::events::entered(
            requested_domain.clone(),
            SecurityLockdownEntered {
                reason_code: request.get_ref().reason_code.clone(),
                occurred_at: Some(occurred_at),
                entered_by: Some(issuer_to_endpoint_ref(&issuer)),
                invalidated_through_operator_session_generation: Some(generation),
                affected_runtime_session_count: affected,
            },
        );
        let mut audit = AuditRecordDraft::new(occurred_at, AuditEventKind::LockdownEntered);
        audit.actor_id = Some(actor);
        audit.endpoint_id = issuer.verified_endpoint().cloned();
        audit.device_id = issuer.verified_device().cloned();
        audit.grant_id = Some(grant);
        audit.target_scope = Some(authority_domain_scope());
        audit.reason_code = request.get_ref().reason_code.clone();
        let event_id = self
            .storage
            .append_decision(&requested_domain, security::events::encode(&source), audit)
            .await
            .map_err(map_storage_error_to_status)?;
        self.state
            .catch_up(&self.storage, &requested_domain)
            .await
            .map_err(map_storage_error_to_status)?;
        let lockdown = self.state.lockdown_state().await;
        if !lockdown.active {
            return Err(Status::internal("committed lockdown event did not activate posture"));
        }
        Ok(Response::new(EnterSecurityLockdownResult {
            lockdown: Some(lockdown),
            lockdown_event_id: Some(event_id),
            already_active: false,
            affected_runtime_session_count: affected,
            invalidated_through_operator_session_generation: Some(generation),
        }))
    }

    async fn load_security_snapshot(
        &self,
        request: Request<LoadSecuritySnapshotRequest>,
    ) -> Result<Response<LoadSecuritySnapshotResponse>, Status> {
        let requested_domain = required_domain(request.get_ref().authority_domain_id.clone())?;
        self.require_configured_domain(&requested_domain)?;
        let _ = self
            .issuer_from_request(&request, requested_domain.clone())
            .await?;
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &requested_domain)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, requested_domain.clone())
            .await?;
        let scope = authority_domain_scope();
        let evaluated_at = self.clock.now();
        let grant = match self
            .state
            .grant_check()
            .check_at(
                &requested_domain,
                &issuer,
                patchbay_contracts::patchbay::OperationKind::Query,
                &scope,
                &evaluated_at,
            )
            .await
        {
            Ok(authorized) => authorized.grant_id,
            Err(_) => {
                let mut audit = AuditRecordDraft::new(evaluated_at, AuditEventKind::AuthorizationFailed);
                audit.actor_id = issuer.verified_actor().cloned();
                audit.endpoint_id = issuer.verified_endpoint().cloned();
                audit.device_id = issuer.verified_device().cloned();
                audit.target_scope = Some(scope.clone());
                audit.failure_code = Some(FailureCode::AuthorizationDenied);
                audit.reason_code = "security_snapshot_authorization_denied".to_owned();
                self.record_audit(audit).await?;
                return Err(Status::permission_denied("security snapshot is not authorized"));
            }
        };
        let grant = grant
            .ok_or_else(|| Status::internal("security snapshot authorization omitted grant provenance"))?;
        let snapshot = self
            .state
            .materialize_security_snapshot(requested_domain)
            .await;
        let mut audit = AuditRecordDraft::new(evaluated_at, AuditEventKind::SubscriptionEstablished);
        audit.actor_id = issuer.verified_actor().cloned();
        audit.endpoint_id = issuer.verified_endpoint().cloned();
        audit.device_id = issuer.verified_device().cloned();
        audit.grant_id = Some(grant);
        audit.target_scope = Some(scope);
        audit.reason_code = "security_snapshot_loaded".to_owned();
        self.record_audit(audit).await?;
        Ok(Response::new(LoadSecuritySnapshotResponse {
            snapshot: Some(snapshot),
        }))
    }

    async fn revoke_grant(
        &self,
        request: Request<RevokeGrantRequest>,
    ) -> Result<Response<RevokeGrantResult>, Status> {
        let requested_domain = required_domain(request.get_ref().authority_domain_id.clone())?;
        self.require_configured_domain(&requested_domain)?;
        let _ = self
            .issuer_from_request(&request, requested_domain.clone())
            .await?;
        let grant_id = request
            .get_ref()
            .grant_id
            .clone()
            .ok_or_else(|| Status::invalid_argument("grant_id is required"))?;
        if grant_id.value.is_empty() || grant_id.value.len() > 256 {
            return Err(Status::invalid_argument("grant_id must be non-empty and bounded"));
        }
        validate_revocation_reason(&request.get_ref().reason)?;

        let _decision_guard = self.decision_gate.acquire().await;
        self.state.catch_up(&self.storage, &requested_domain).await.map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, requested_domain.clone())
            .await?;
        self.require_operations_open(&issuer).await?;
        let request = request.into_inner();
        let Some(grant) = self.state.grant(&grant_id).await else {
            self.audit_revocation_denied(&issuer).await?;
            return Err(Status::permission_denied("grant revocation is not authorized"));
        };
        let Some(actor) = issuer.verified_actor() else {
            self.audit_revocation_denied(&issuer).await?;
            return Err(Status::permission_denied("grant revocation is not authorized"));
        };
        let issuer_ref = IssuerRef {
            actor,
            endpoint: issuer.verified_endpoint(),
            authority_domain_id: &requested_domain,
        };
        let evaluated_at = self.clock.now();
        match authorize_self_revocation_at(&grant, &issuer_ref, &evaluated_at) {
            Ok(()) => {}
            Err(GrantAdministrationDenied::Expired { grant_id }) => {
                let mut audit = AuditRecordDraft::new(self.clock.now(), AuditEventKind::GrantExpired);
                audit.actor_id = Some(actor.clone());
                audit.endpoint_id = issuer.verified_endpoint().cloned();
                audit.device_id = issuer.verified_device().cloned();
                audit.grant_id = Some(grant_id);
                audit.reason_code = "grant_expired".to_owned();
                self.record_audit(audit).await?;
                let _ = grant_id;
                return Err(Status::permission_denied("grant revocation is not authorized"));
            }
            Err(GrantAdministrationDenied::MissingOrForeign | GrantAdministrationDenied::EndpointMismatch) => {
                self.audit_revocation_denied(&issuer).await?;
                return Err(Status::permission_denied("grant revocation is not authorized"));
            }
        }
        if grant.is_revoked() {
            let mut audit = AuditRecordDraft::new(evaluated_at, AuditEventKind::GrantRevoked);
            audit.actor_id = issuer.verified_actor().cloned();
            audit.endpoint_id = issuer.verified_endpoint().cloned();
            audit.device_id = issuer.verified_device().cloned();
            audit.grant_id = Some(grant_id.clone());
            audit.target_scope = Some(grant.target_scope.clone());
            audit.reason_code = "grant_revocation_idempotent".to_owned();
            self.record_audit(audit).await?;
            return Ok(Response::new(RevokeGrantResult {
                changed: false,
                already_revoked: true,
                revocation_event_id: None,
                applied_policy: grant.revocation_policy as i32,
                command_effects: Vec::new(),
            }));
        }

        if grant.is_recovery_capable_authority_domain()
            && self
                .state
                .recovery_capable_authority_domain_grant_count(&evaluated_at)
                .await
                <= 1
        {
            let mut audit = AuditRecordDraft::new(evaluated_at, AuditEventKind::AuthorizationFailed);
            audit.actor_id = Some(actor.clone());
            audit.endpoint_id = issuer.verified_endpoint().cloned();
            audit.device_id = issuer.verified_device().cloned();
            audit.grant_id = Some(grant_id.clone());
            audit.target_scope = Some(grant.target_scope.clone());
            audit.failure_code = Some(FailureCode::AuthorizationDenied);
            audit.reason_code = "last_recovery_authority_grant".to_owned();
            self.record_audit(audit).await?;
            return Err(Status::failed_precondition(
                "authorization_denied/last_recovery_authority_grant",
            ));
        }

        let mut records = self.state.commands_for_grant(&grant_id).await;
        records.sort_unstable_by(|left, right| left.command_id.value.cmp(&right.command_id.value));
        let effects = revocation_effects(&grant, &records);
        let revoked_by = ActorEndpointRef {
            actor_id: issuer.verified_actor().cloned(),
            endpoint_id: issuer.verified_endpoint().cloned(),
            device_id: issuer.verified_device().cloned(),
            endpoint_generation: issuer.endpoint_generation(),
        };
        let revocation = Revocation {
            authority_domain_id: Some(requested_domain.clone()),
            grant_id: Some(grant_id.clone()),
            revoked_by: Some(revoked_by),
            revoked_at: Some(self.clock.now()),
            revocation_generation: Some(Generation { value: 1 }),
            accepted_operation_policy: grant.revocation_policy as i32,
            reason: request.reason,
            command_effects: effects.clone(),
            ..Revocation::default()
        };
        let event_id = self.state.ingest_revocation(&self.storage, &requested_domain, revocation)
            .await
            .map_err(map_authority_error_to_status)?;
        self.state.catch_up(&self.storage, &requested_domain).await.map_err(map_storage_error_to_status)?;
        Ok(Response::new(RevokeGrantResult {
            changed: true,
            already_revoked: false,
            revocation_event_id: Some(event_id),
            applied_policy: grant.revocation_policy as i32,
            command_effects: effects,
        }))
    }

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let authority_domain_id = required_domain(request.get_ref().authority_domain_id.clone())?;
        self.require_configured_domain(&authority_domain_id)?;
        let cursor = request.get_ref().cursor.unwrap_or(Lsn { value: 0 });
        let _ = self
            .issuer_from_request(&request, authority_domain_id.clone())
            .await?;
        let scope = subscription_scope();
        let _decision_guard = self.decision_gate.acquire().await;
        self.state.catch_up(&self.storage, &authority_domain_id).await.map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, authority_domain_id.clone())
            .await?;
        // Compare the cursor only after catch-up and issuer re-verification.
        // Adapter-originated events may already be durable while the control
        // projection is still warming its LSN, so an earlier comparison can
        // reject a valid reconnect cursor.
        let current_lsn = self.state.current_lsn().await;
        if cursor.value > current_lsn {
            return Err(Status::invalid_argument("subscription cursor is beyond current LSN"));
        }
        let evaluated_at = self.clock.now();
        let decision = self.state.grant_check().check_at(
            &authority_domain_id,
            &issuer,
            patchbay_contracts::patchbay::OperationKind::Query,
            &scope,
            &evaluated_at,
        ).await;
        let grant_id = match decision {
            Ok(authorized) => authorized.grant_id,
            Err(error) => {
                let (kind, failure_code, reason_code, denied_grant_id) = match error {
                    patchbay_core::acceptance::GrantDenied::NoGrant { actor, .. }
                        if actor.starts_with("grant_expired:") => (AuditEventKind::GrantExpired, FailureCode::Expired, "subscription_grant_expired", actor.strip_prefix("grant_expired:").map(|value| patchbay_contracts::patchbay::GrantId { value: value.to_owned() })),
                    patchbay_core::acceptance::GrantDenied::NoGrant { actor, .. }
                        if actor.starts_with("grant_revoked:") => (AuditEventKind::SubscriptionDenied, FailureCode::AuthorizationDenied, "grant_revoked", actor.strip_prefix("grant_revoked:").map(|value| patchbay_contracts::patchbay::GrantId { value: value.to_owned() })),
                    _ => (AuditEventKind::SubscriptionDenied, FailureCode::AuthorizationDenied, "authorization_denied", None),
                };
                let mut audit = AuditRecordDraft::new(evaluated_at, kind);
                audit.actor_id = issuer.verified_actor().cloned();
                audit.endpoint_id = issuer.verified_endpoint().cloned();
                audit.device_id = issuer.verified_device().cloned();
                audit.grant_id = denied_grant_id;
                audit.target_scope = Some(scope.clone());
                audit.failure_code = Some(failure_code);
                audit.reason_code = reason_code.to_owned();
                self.record_audit(audit).await?;
                return Err(Status::permission_denied("subscription grant is not authorized"));
            }
        };
        let grant_id = grant_id.ok_or_else(|| Status::internal("subscription authorization omitted grant provenance"))?;
        let mut audit = AuditRecordDraft::new(evaluated_at, AuditEventKind::SubscriptionEstablished);
        audit.actor_id = issuer.verified_actor().cloned();
        audit.endpoint_id = issuer.verified_endpoint().cloned();
        audit.device_id = issuer.verified_device().cloned();
        audit.grant_id = Some(grant_id);
        audit.target_scope = Some(scope);
        audit.reason_code = "subscription_established".to_owned();
        self.record_audit(audit).await?;
        let events = self.storage.read_after(&authority_domain_id, cursor).await.map_err(map_storage_error_to_status)?;
        let events = events.into_iter().filter_map(operator_facing_subscribe_event).map(Ok);
        drop(_decision_guard);
        Ok(Response::new(Box::pin(stream::iter(events))))
    }

    async fn load_snapshot(
        &self,
        request: Request<LoadSnapshotRequest>,
    ) -> Result<Response<LoadSnapshotResponse>, Status> {
        let authority_domain_id = required_domain(request.get_ref().authority_domain_id.clone())?;
        self.require_configured_domain(&authority_domain_id)?;
        let view_kind = patchbay_contracts::patchbay::SnapshotViewKind::try_from(
            request.get_ref().view_kind,
        )
        .map_err(|_| Status::invalid_argument("unknown snapshot view kind"))?;
        if view_kind == patchbay_contracts::patchbay::SnapshotViewKind::Unspecified {
            return Err(Status::invalid_argument("snapshot view kind is required"));
        }
        self.issuer_from_request(&request, authority_domain_id.clone())
            .await?;
        let at_or_before = request.get_ref().at_or_before;

        // View validation precedes stateful work. Catch-up and compound-issuer
        // re-verification then share the core decision gate with writers.
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, authority_domain_id.clone())
            .await?;

        if view_kind == patchbay_contracts::patchbay::SnapshotViewKind::Session {
            let current_lsn = self.state.current_lsn().await;
            if let Some(stored) = self
                .storage
                .load_latest_snapshot(&authority_domain_id, at_or_before)
                .await
                .map_err(map_storage_error_to_status)?
            {
                let stored_lsn = stored.event_id.lsn.as_ref().map(|lsn| lsn.value);
                if let Ok(snapshot) = decode_compatible_session_checkpoint(
                    &stored,
                    &authority_domain_id,
                    self.state.core_generation(),
                ) {
                    if stored_lsn.is_some_and(|lsn| lsn >= current_lsn) {
                        return Ok(Response::new(LoadSnapshotResponse {
                            present: true,
                            event_id: Some(stored.event_id),
                            snapshot_payload: snapshot.encode_to_vec(),
                            view_kind: view_kind as i32,
                        }));
                    }
                }
            }

            let snapshot = self
                .state
                .materialize_session_snapshot(
                    authority_domain_id.clone(),
                    crate::identity::now_timestamp()?,
                )
                .await;
            let snapshot_lsn = snapshot
                .snapshot_lsn
                .ok_or_else(|| Status::internal("materialized session snapshot has no LSN"))?;
            return Ok(Response::new(LoadSnapshotResponse {
                present: true,
                event_id: Some(EventId {
                    authority_domain_id: Some(authority_domain_id),
                    lsn: Some(snapshot_lsn),
                }),
                snapshot_payload: snapshot.encode_to_vec(),
                view_kind: view_kind as i32,
            }));
        }

        // Resource checkpoints cannot share the current session-only typed
        // slot. Materialize directly from the replayable projection;
        // a historical bound repairs to the newer current authority.
        let mut snapshot = self
            .state
            .materialize_resource_snapshot(
                authority_domain_id.clone(),
                crate::identity::now_timestamp()?,
            )
            .await;
        let evaluated_at = self.clock.now();
        let mut authorized_views = HashSet::new();
        let mut authorized_resources = Vec::with_capacity(snapshot.resources.len());
        for resource in std::mem::take(&mut snapshot.resources) {
            let Some(identity) = resource.identity.clone() else {
                continue;
            };
            let scope = TargetScope {
                kind: TargetScopeKind::Resource as i32,
                resource: Some(identity.clone()),
                ..TargetScope::default()
            };
            if self
                .state
                .grant_check()
                .check_at(
                    &authority_domain_id,
                    &issuer,
                    patchbay_contracts::patchbay::OperationKind::Query,
                    &scope,
                    &evaluated_at,
                )
                .await
                .is_ok()
            {
                let adapter_id = identity.adapter_id.map(|id| id.value).unwrap_or_default();
                let resource_kind = identity.resource_kind.map(|kind| kind.value).unwrap_or_default();
                authorized_views.insert((adapter_id, resource_kind));
                authorized_resources.push(resource);
            }
        }
        snapshot.resources = authorized_resources;
        snapshot.view_revisions.retain(|view| {
            authorized_views.contains(&(
                view.adapter_id.as_ref().map(|id| id.value.clone()).unwrap_or_default(),
                view.resource_kind.as_ref().map(|kind| kind.value.clone()).unwrap_or_default(),
            ))
        });
        let snapshot_lsn = snapshot
            .snapshot_lsn
            .ok_or_else(|| Status::internal("materialized resource snapshot has no LSN"))?;
        Ok(Response::new(LoadSnapshotResponse {
            present: true,
            event_id: Some(EventId {
                authority_domain_id: Some(authority_domain_id),
                lsn: Some(snapshot_lsn),
            }),
            snapshot_payload: snapshot.encode_to_vec(),
            view_kind: view_kind as i32,
        }))
    }

    async fn verify_operator_password(
        &self,
        request: Request<VerifyOperatorPasswordRequest>,
    ) -> Result<Response<VerifyOperatorPasswordResult>, Status> {
        let network_address = caller_network_address(&request);
        let request = request.into_inner();
        let actor_id = required_actor(request.operator_actor_id)?;
        if request.password.is_empty() {
            self.audit_login(
                &actor_id,
                &network_address,
                LoginAuditOutcome::Failure,
                "password_required",
                Vec::new(),
            )
            .await?;
            return Err(Status::invalid_argument("password must not be empty"));
        }
        let attempt = match self
            .login_limiter
            .begin_attempt(&actor_id.value, &network_address)
        {
            Ok(attempt) => attempt,
            Err(limit) => {
                self.audit_login(
                    &actor_id,
                    &network_address,
                    LoginAuditOutcome::Failure,
                    "login_throttled",
                    limit.blocked_dimensions,
                )
                .await?;
                return Err(Status::with_error_details(
                    Code::ResourceExhausted,
                    "operator password verification is throttled",
                    ErrorDetails::with_retry_info(Some(limit.retry_after)),
                ));
            }
        };
        let verified = match self
            .state
            .verify_password(&actor_id, &request.password)
            .await
        {
            Ok(verified) => verified,
            Err(error) => {
                attempt.failure();
                self.audit_login(
                    &actor_id,
                    &network_address,
                    LoginAuditOutcome::Failure,
                    "verification_error",
                    Vec::new(),
                )
                .await?;
                return Err(map_operator_error_to_status(error));
            }
        };
        if !verified {
            attempt.failure();
            self.audit_login(
                &actor_id,
                &network_address,
                LoginAuditOutcome::Failure,
                "invalid_credentials",
                Vec::new(),
            )
            .await?;
            return Err(Status::unauthenticated("invalid operator credentials"));
        }
        let enrollment = match request.principal {
            Some(enrollment) => enrollment,
            None => {
                attempt.failure();
                self.audit_login(
                    &actor_id,
                    &network_address,
                    LoginAuditOutcome::Failure,
                    "principal_enrollment_required",
                    Vec::new(),
                )
                .await?;
                return Err(Status::invalid_argument("principal enrollment is required"));
            }
        };
        let (record, credential) = match issue_principal(
            actor_id.clone(),
            enrollment,
            self.authority_domain_id.clone(),
        ) {
            Ok(issued) => issued,
            Err(error) => {
                attempt.failure();
                self.audit_login(
                    &actor_id,
                    &network_address,
                    LoginAuditOutcome::Failure,
                    "principal_enrollment_invalid",
                    Vec::new(),
                )
                .await?;
                return Err(error);
            }
        };
        let session_binding = OperatorSessionBinding {
            actor_id: actor_id.clone(),
            endpoint_id: credential
                .endpoint_id
                .clone()
                .ok_or_else(|| Status::internal("issued principal has no endpoint"))?,
            device_id: credential
                .device_id
                .clone()
                .ok_or_else(|| Status::internal("issued principal has no device"))?,
            endpoint_generation: credential
                .endpoint_generation
                .ok_or_else(|| Status::internal("issued principal has no endpoint generation"))?,
        };
        let _decision_guard = self.decision_gate.acquire().await;
        if let Err(error) = self
            .state
            .ingest_principal(&self.storage, &self.authority_domain_id, record)
            .await
        {
            attempt.failure();
            self.audit_login(
                &actor_id,
                &network_address,
                LoginAuditOutcome::Failure,
                "principal_enrollment_failed",
                Vec::new(),
            )
            .await?;
            return Err(map_operator_error_to_status(error));
        }
        if let Err(error) = self
            .state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
        {
            attempt.failure();
            self.audit_login(
                &actor_id,
                &network_address,
                LoginAuditOutcome::Failure,
                "projection_catch_up_failed",
                Vec::new(),
            )
            .await?;
            return Err(map_storage_error_to_status(error));
        }
        let operator_session = self.state.issue_operator_session(session_binding).await;
        attempt.success();
        self.audit_login(
            &actor_id,
            &network_address,
            LoginAuditOutcome::Success,
            "authenticated",
            Vec::new(),
        )
        .await?;
        let mut session_audit = AuditRecordDraft::new(
            crate::identity::now_timestamp()?,
            AuditEventKind::OperatorSessionCreated,
        );
        session_audit.actor_id = Some(actor_id.clone());
        session_audit.reason_code = "operator_session_created".to_owned();
        self.record_audit(session_audit).await?;
        Ok(Response::new(VerifyOperatorPasswordResult {
            operator_session_id: Some(operator_session.id),
            principal: Some(credential),
            operator_session_generation: Some(operator_session.session_generation),
        }))
    }

    async fn revoke_operator_session(
        &self,
        request: Request<RevokeOperatorSessionRequest>,
    ) -> Result<Response<RevokeOperatorSessionResult>, Status> {
        let _ = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        let actor_id = issuer
            .verified_actor()
            .cloned()
            .ok_or_else(|| Status::internal("verified issuer lost its actor"))?;
        // Verify first, then make the required audit append before the local
        // session mutation. If durable audit fails, the session remains active
        // and there is no unaudited successful revocation.
        if !self
            .state
            .verify_operator_session(issuer.operator_session_id(), &issuer.binding())
            .await
        {
            return Err(Status::failed_precondition(
                "operator session is no longer active",
            ));
        }
        let mut session_audit = AuditRecordDraft::new(
            crate::identity::now_timestamp()?,
            AuditEventKind::OperatorSessionRevoked,
        );
        session_audit.actor_id = Some(actor_id);
        session_audit.endpoint_id = issuer.verified_endpoint().cloned();
        session_audit.device_id = issuer.verified_device().cloned();
        session_audit.operator_session_hash = hash_principal_credential(&issuer.operator_session_id().value);
        session_audit.reason_code = "operator_session_revoked".to_owned();
        self.record_audit(session_audit).await?;
        let revoked = self
            .state
            .revoke_operator_session(issuer.operator_session_id(), &issuer.binding())
            .await;
        if !revoked {
            return Err(Status::failed_precondition(
                "operator session is no longer active",
            ));
        }
        Ok(Response::new(RevokeOperatorSessionResult { revoked }))
    }

    async fn revoke_all_operator_sessions(
        &self,
        request: Request<RevokeAllOperatorSessionsRequest>,
    ) -> Result<Response<RevokeAllOperatorSessionsResult>, Status> {
        let _ = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        validate_control_surface_reason(&request.get_ref().reason_code)?;
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        self.require_operations_open(&issuer).await?;
        let request = request.into_inner();
        let actor_id = issuer
            .verified_actor()
            .cloned()
            .ok_or_else(|| Status::internal("verified issuer lost its actor"))?;
        let generation = self.state.current_operator_session_generation(&actor_id).await;
        let revocation = OperatorSessionRevocation {
            authority_domain_id: Some(self.authority_domain_id.clone()),
            operator_actor_id: Some(actor_id.clone()),
            invalidated_through_generation: Some(generation),
            verified_revoker: Some(issuer_to_endpoint_ref(&issuer)),
            occurred_at: Some(crate::identity::now_timestamp()?),
            reason_code: request.reason_code,
        };
        let (result, revoked_session_count) = self
            .state
            .ingest_operator_session_revocation(
                &self.storage,
                &self.authority_domain_id,
                revocation,
            )
            .await
            .map_err(map_operator_error_to_status)?;
        Ok(Response::new(RevokeAllOperatorSessionsResult {
            revoked_session_count,
            invalidated_through_generation: Some(generation),
            revocation_event_id: Some(result.event_id),
        }))
    }

    async fn revoke_control_surface_principal(
        &self,
        request: Request<RevokeControlSurfacePrincipalRequest>,
    ) -> Result<Response<RevokeControlSurfaceResult>, Status> {
        let _ = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        let principal_id = request.get_ref().principal_id.clone();
        validate_control_surface_reason(&request.get_ref().reason_code)?;
        if principal_id.is_empty() {
            return Err(Status::invalid_argument("principal_id must not be empty"));
        }
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        self.require_operations_open(&issuer).await?;
        let target = patchbay_core::authority::ControlSurfaceRevocationTarget::Principal(
            principal_id.clone(),
        );
        let Some(record) = self.state.principal_record(&principal_id).await else {
            self.audit_control_surface_denied(
                &issuer,
                Some(&target),
                "control_surface_principal_not_found",
            )
            .await?;
            return Err(Status::not_found("control-surface principal was not found"));
        };
        if record.operator_actor_id.as_ref() != issuer.verified_actor() {
            self.audit_control_surface_denied(
                &issuer,
                Some(&target),
                "control_surface_principal_authorization_denied",
            )
            .await?;
            return Err(Status::not_found("control-surface principal was not found"));
        }
        let principal_count = self.state.count_matching_revocation_target(&target).await;
        let request = request.into_inner();
        let revocation = ControlSurfaceRevocation {
            authority_domain_id: Some(self.authority_domain_id.clone()),
            verified_revoker: Some(issuer_to_endpoint_ref(&issuer)),
            occurred_at: Some(crate::identity::now_timestamp()?),
            reason_code: request.reason_code,
            target: Some(
                patchbay_contracts::patchbay::control_surface_revocation::Target::PrincipalId(
                    principal_id,
                ),
            ),
        };
        let (result, _target, revoked_session_count) = self
            .state
            .ingest_control_surface_revocation(
                &self.storage,
                &self.authority_domain_id,
                revocation,
            )
            .await
            .map_err(map_operator_error_to_status)?;
        Ok(Response::new(RevokeControlSurfaceResult {
            newly_revoked: result.newly_revoked,
            revoked_principal_count: if result.newly_revoked { principal_count } else { 0 },
            revoked_session_count,
            revocation_event_id: Some(result.event_id),
        }))
    }

    async fn revoke_control_surface_endpoint(
        &self,
        request: Request<RevokeControlSurfaceEndpointRequest>,
    ) -> Result<Response<RevokeControlSurfaceResult>, Status> {
        let _ = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        validate_control_surface_reason(&request.get_ref().reason_code)?;
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        self.require_operations_open(&issuer).await?;
        let target = match request.get_ref().target.clone() {
            Some(patchbay_contracts::patchbay::revoke_control_surface_endpoint_request::Target::EndpointId(endpoint_id)) => {
                let target = patchbay_core::authority::ControlSurfaceRevocationTarget::Endpoint(endpoint_id.clone());
                if endpoint_id.value.is_empty() || !self.state.has_endpoint(&endpoint_id).await {
                    self.audit_control_surface_denied(
                        &issuer,
                        Some(&target),
                        "control_surface_endpoint_not_found",
                    )
                    .await?;
                    return Err(Status::not_found("control-surface endpoint was not found"));
                }
                target
            }
            Some(patchbay_contracts::patchbay::revoke_control_surface_endpoint_request::Target::DeviceId(device_id)) => {
                let target = patchbay_core::authority::ControlSurfaceRevocationTarget::Device(device_id.clone());
                if device_id.value.is_empty() || !self.state.has_device(&device_id).await {
                    self.audit_control_surface_denied(
                        &issuer,
                        Some(&target),
                        "control_surface_device_not_found",
                    )
                    .await?;
                    return Err(Status::not_found("control-surface device was not found"));
                }
                target
            }
            None => {
                self.audit_control_surface_denied(
                    &issuer,
                    None,
                    "control_surface_revocation_target_missing",
                )
                .await?;
                return Err(Status::invalid_argument("exactly one endpoint_id or device_id is required"));
            }
        };
        let revoked_principal_count = self.state.count_matching_revocation_target(&target).await;
        let target_wire = match &target {
            patchbay_core::authority::ControlSurfaceRevocationTarget::Endpoint(endpoint_id) => {
                patchbay_contracts::patchbay::control_surface_revocation::Target::EndpointId(endpoint_id.clone())
            }
            patchbay_core::authority::ControlSurfaceRevocationTarget::Device(device_id) => {
                patchbay_contracts::patchbay::control_surface_revocation::Target::DeviceId(device_id.clone())
            }
            patchbay_core::authority::ControlSurfaceRevocationTarget::Principal(_) => unreachable!(),
        };
        let revocation = ControlSurfaceRevocation {
            authority_domain_id: Some(self.authority_domain_id.clone()),
            verified_revoker: Some(issuer_to_endpoint_ref(&issuer)),
            occurred_at: Some(crate::identity::now_timestamp()?),
            reason_code: request.get_ref().reason_code.clone(),
            target: Some(target_wire),
        };
        let (result, _target, revoked_session_count) = self
            .state
            .ingest_control_surface_revocation(
                &self.storage,
                &self.authority_domain_id,
                revocation,
            )
            .await
            .map_err(map_operator_error_to_status)?;
        Ok(Response::new(RevokeControlSurfaceResult {
            newly_revoked: result.newly_revoked,
            revoked_principal_count: if result.newly_revoked { revoked_principal_count } else { 0 },
            revoked_session_count,
            revocation_event_id: Some(result.event_id),
        }))
    }

    async fn enroll_control_surface_principal(
        &self,
        request: Request<EnrollControlSurfacePrincipalRequest>,
    ) -> Result<Response<EnrollControlSurfacePrincipalResult>, Status> {
        let _ = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        let enrollment = request
            .get_ref()
            .principal
            .clone()
            .ok_or_else(|| Status::invalid_argument("principal enrollment is required"))?;
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        self.require_operations_open(&issuer).await?;
        let actor_id = issuer
            .verified_actor()
            .cloned()
            .ok_or_else(|| Status::internal("verified issuer lost its actor"))?;
        let (record, credential) =
            issue_principal(actor_id, enrollment, self.authority_domain_id.clone())?;
        self.state
            .ingest_principal(&self.storage, &self.authority_domain_id, record)
            .await
            .map_err(map_operator_error_to_status)?;
        self.state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        Ok(Response::new(EnrollControlSurfacePrincipalResult {
            principal: Some(credential),
        }))
    }

    async fn record_control_surface_audit(
        &self,
        request: Request<RecordControlSurfaceAuditRequest>,
    ) -> Result<Response<RecordControlSurfaceAuditResponse>, Status> {
        let _ = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        let kind = patchbay_contracts::patchbay::AuditEventKind::try_from(request.get_ref().kind)
            .map_err(|_| Status::invalid_argument("unknown control-surface audit kind"))?;
        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, self.authority_domain_id.clone())
            .await?;
        let request = request.into_inner();
        if !matches!(
            kind,
            AuditEventKind::CsrfCheckFailed
                | AuditEventKind::OriginCheckFailed
                | AuditEventKind::FetchMetadataCheckFailed
                | AuditEventKind::Logout
                | AuditEventKind::OperatorSessionCreated
                | AuditEventKind::OperatorSessionRenewed
                | AuditEventKind::OperatorSessionExpired
                | AuditEventKind::OperatorSessionRevoked
        ) {
            return Err(Status::invalid_argument(
                "audit kind is not permitted for control-surface ingress",
            ));
        }
        if request.reason_code.is_empty()
            || request.reason_code.len() > 64
            || !request
                .reason_code
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(Status::invalid_argument("reason_code must match [a-z0-9_]{1,64}"));
        }
        let mut draft = AuditRecordDraft::new(crate::identity::now_timestamp()?, kind);
        draft.actor_id = issuer.verified_actor().cloned();
        draft.endpoint_id = issuer.verified_endpoint().cloned();
        draft.device_id = issuer.verified_device().cloned();
        draft.operator_session_hash = hash_principal_credential(&issuer.operator_session_id().value);
        draft.reason_code = request.reason_code;
        let receipt = self
            .audit
            .record(draft)
            .await
            .map_err(|error| Status::unavailable(error.to_string()))?;
        let AuditReceipt::Durable(audit_event_id) = receipt else {
            return Err(Status::internal("control-surface audit was not durable"));
        };
        Ok(Response::new(RecordControlSurfaceAuditResponse {
            audit_event_id: Some(audit_event_id),
        }))
    }

    async fn query_diagnostics(
        &self,
        request: Request<QueryDiagnosticsRequest>,
    ) -> Result<Response<QueryDiagnosticsResponse>, Status> {
        let operation = request
            .get_ref()
            .operation
            .clone()
            .ok_or_else(|| Status::invalid_argument("query diagnostics request is missing operation"))?;
        let authority_domain_id = operation
            .authority_domain_id
            .clone()
            .ok_or_else(|| Status::invalid_argument("query operation is missing authority domain"))?;
        self.require_configured_domain(&authority_domain_id)?;
        let issuer = self
            .issuer_from_request(&request, authority_domain_id.clone())
            .await?;

        // Keep the shared operation envelope/time boundary ahead of the
        // stateful gate. Query payload decoding is deliberately later: while
        // lockdown is active the canonical posture denial must win over a
        // malformed typed diagnostics payload.
        let evaluated_at = self.clock.now();
        if let Err(submission) = acceptance::validate_operation_boundary(&operation, &evaluated_at) {
            let submission = *submission;
            self.audit_submission_rejection(&issuer, &operation, &submission).await?;
            return Ok(Response::new(QueryDiagnosticsResponse {
                submission: Some(submission),
                ..QueryDiagnosticsResponse::default()
            }));
        }

        let _decision_guard = self.decision_gate.acquire().await;
        self.state
            .catch_up(&self.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let issuer = self
            .issuer_from_request(&request, authority_domain_id.clone())
            .await?;
        if let Err(OperationPostureDenied::SecurityLockdown { reason_code, .. }) = self
            .state
            .operation_posture()
            .check(&authority_domain_id)
            .await
        {
            let submission = rejected_query_lockdown_submission(&operation, reason_code);
            self.audit_submission_rejection(&issuer, &operation, &submission).await?;
            return Ok(Response::new(QueryDiagnosticsResponse {
                submission: Some(submission),
                ..QueryDiagnosticsResponse::default()
            }));
        }

        // Decode and validate the typed query only after catch-up and posture
        // enforcement. A malformed query is still a normal pre-acceptance
        // rejection and never touches the durable command log.
        let current_lsn = self.state.current_lsn().await;
        let validated = match diagnostics::validate_query(&operation, current_lsn) {
            Ok(validated) => validated,
            Err(error) => {
                let submission = rejected_query_submission(&operation, error.to_string());
                self.audit_submission_rejection(&issuer, &operation, &submission).await?;
                return Ok(Response::new(QueryDiagnosticsResponse {
                    submission: Some(submission),
                    ..QueryDiagnosticsResponse::default()
                }));
            }
        };

        let submission = match acceptance::submit_with_clock_and_posture(
            &self.storage,
            self.state.grant_check(),
            &AuthorityDomainTargetResolver,
            self.state.state_lookup(),
            self.state.elicitation_contract_lookup(),
            self.state.operation_posture(),
            &issuer,
            operation.clone(),
            self.clock.as_ref(),
        )
        .await {
            Ok(result) => result,
            Err(error) => {
                self.audit_submission_unknown(&issuer, &operation).await?;
                return Err(map_acceptance_error_to_status(error));
            }
        };
        if submission.outcome != SubmissionOutcome::Accepted as i32 {
            self.audit_submission_rejection(&issuer, &operation, &submission)
                .await?;
            return Ok(Response::new(QueryDiagnosticsResponse {
                submission: Some(submission),
                ..QueryDiagnosticsResponse::default()
            }));
        }
        let command_id = submission
            .command_id
            .clone()
            .ok_or_else(|| Status::internal("accepted query has no command id"))?;

        if !submission.deduplicated {
            self.state
                .catch_up(&self.storage, &authority_domain_id)
                .await
                .map_err(map_storage_error_to_status)?;
        }
        let mut current = self
            .state
            .state_lookup()
            .current_state(&command_id)
            .await
            .ok_or_else(|| Status::internal("accepted query is missing from the command projection"))?;

        // Every checkpoint is reconciled under the submit gate. A retry that
        // arrives after any durable prefix resumes the missing suffix instead
        // of returning UNAVAILABLE merely because the first handler stopped.
        let delivered_event_id = if current.state == OperationState::Accepted {
            let event_id = append_query_transition(
                &self.storage,
                &authority_domain_id,
                &command_id,
                OperationState::Accepted,
                OperationState::Delivered,
            )
            .await
            .map_err(map_storage_error_to_status)?;
            self.state
                .catch_up(&self.storage, &authority_domain_id)
                .await
                .map_err(map_storage_error_to_status)?;
            current = self
                .state
                .state_lookup()
                .current_state(&command_id)
                .await
                .ok_or_else(|| Status::internal("delivered query is missing from the command projection"))?;
            Some(event_id)
        } else {
            None
        };
        if !matches!(current.state, OperationState::Delivered) {
            if current.state == OperationState::Completed {
                if let Some((result_event_id, result, response_result)) =
                    find_diagnostics_result(&self.storage, &authority_domain_id, &command_id).await?
                {
                    let mut completed = submission;
                    completed.operation_state = OperationState::Completed as i32;
                    return Ok(Response::new(QueryDiagnosticsResponse {
                        submission: Some(completed),
                        result_event_id: Some(result_event_id),
                        as_of_lsn: result.as_of_lsn,
                        result: Some(response_result),
                    }));
                }
            }
            if current.state != OperationState::Delivered {
                let mut terminal = submission;
                terminal.operation_state = current.state as i32;
                terminal.failure_code = if current.state == OperationState::Failed {
                    FailureCode::ExecutionFailed as i32
                } else {
                    FailureCode::Unspecified as i32
                };
                return Ok(Response::new(QueryDiagnosticsResponse {
                    submission: Some(terminal),
                    ..QueryDiagnosticsResponse::default()
                }));
            }
        }

        if let Some((result_event_id, result, response_result)) =
            find_diagnostics_result(&self.storage, &authority_domain_id, &command_id).await?
        {
            // The result Observation is durable but the completion checkpoint
            // may not be. Reconcile that checkpoint without rematerializing.
            if current.state == OperationState::Delivered {
                append_query_transition(
                    &self.storage,
                    &authority_domain_id,
                    &command_id,
                    OperationState::Delivered,
                    OperationState::Completed,
                )
                .await
                .map_err(map_storage_error_to_status)?;
                self.state
                    .catch_up(&self.storage, &authority_domain_id)
                    .await
                    .map_err(map_storage_error_to_status)?;
            }
            let mut completed = submission;
            completed.operation_state = OperationState::Completed as i32;
            return Ok(Response::new(QueryDiagnosticsResponse {
                submission: Some(completed),
                result_event_id: Some(result_event_id),
                as_of_lsn: result.as_of_lsn,
                result: Some(response_result),
            }));
        }

        let as_of_lsn = find_delivered_checkpoint(&self.storage, &authority_domain_id, &command_id)
            .await?
            .or(delivered_event_id)
            .ok_or_else(|| Status::internal("delivered query has no durable checkpoint"))?;
        let as_of_lsn = as_of_lsn
            .lsn
            .ok_or_else(|| Status::internal("delivered query has no LSN"))?;
        let (result, response_result) = match materialize_diagnostics_result(
            &self.storage,
            &self.state,
            &authority_domain_id,
            validated,
            as_of_lsn.value,
        )
        .await {
            Ok(result) => result,
            Err(_) => {
                let failed = append_query_failure(
                    &self.storage,
                    &authority_domain_id,
                    &command_id,
                )
                .await
                .map_err(map_storage_error_to_status)?;
                self.state
                    .catch_up(&self.storage, &authority_domain_id)
                    .await
                    .map_err(map_storage_error_to_status)?;
                let mut failed_submission = submission;
                failed_submission.operation_state = OperationState::Failed as i32;
                failed_submission.failure_code = FailureCode::ExecutionFailed as i32;
                let _ = failed;
                return Ok(Response::new(QueryDiagnosticsResponse {
                    submission: Some(failed_submission),
                    ..QueryDiagnosticsResponse::default()
                }));
            }
        };
        let result_event_id = self
            .storage
            .append(
                &authority_domain_id,
                patchbay_contracts::patchbay::StoredEventPayload {
                    kind: StoredEventKind::Observation as i32,
                    payload: Observation {
                        authority_domain_id: Some(authority_domain_id.clone()),
                        kind: ObservationKind::Result as i32,
                        correlations: vec![TypedCorrelation {
                            r#ref: Some(typed_correlation::Ref::CommandId(command_id.clone())),
                        }],
                        payload: Some(patchbay_contracts::patchbay::PayloadEnvelope {
                            payload: result.encode_to_vec(),
                            content_type: PayloadContentType::Protobuf as i32,
                            schema_ref: "patchbay.DiagnosticsResult".to_owned(),
                        }),
                        ..Observation::default()
                    }
                    .encode_to_vec(),
                },
            )
            .await
            .map_err(map_storage_error_to_status)?;
        append_query_transition(
            &self.storage,
            &authority_domain_id,
            &command_id,
            OperationState::Delivered,
            OperationState::Completed,
        )
        .await
        .map_err(map_storage_error_to_status)?;
        self.state
            .catch_up(&self.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;

        let mut final_submission = submission;
        final_submission.operation_state = OperationState::Completed as i32;
        Ok(Response::new(QueryDiagnosticsResponse {
            submission: Some(final_submission),
            result_event_id: Some(result_event_id),
            as_of_lsn: Some(as_of_lsn),
            result: Some(response_result),
        }))
    }
}

impl<S> ControlServiceImpl<S>
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    async fn issuer_from_request<T>(
        &self,
        request: &Request<T>,
        authority_domain_id: AuthorityDomainId,
    ) -> Result<MetadataIssuerContext, Status> {
        match MetadataIssuerContext::from_request(request, authority_domain_id, &self.state).await {
            Ok(issuer) => Ok(issuer),
            Err(error) => {
                self.audit_authentication_failure().await?;
                Err(error)
            }
        }
    }

    async fn require_operations_open(
        &self,
        issuer: &dyn IssuerContext,
    ) -> Result<(), Status> {
        if self.state.lockdown_state().await.active {
            let mut audit = AuditRecordDraft::new(
                crate::identity::now_timestamp()?,
                AuditEventKind::AuthorizationFailed,
            );
            audit.actor_id = issuer.verified_actor().cloned();
            audit.endpoint_id = issuer.verified_endpoint().cloned();
            audit.device_id = issuer.verified_device().cloned();
            audit.target_scope = Some(authority_domain_scope());
            audit.failure_code = Some(FailureCode::AuthorizationDenied);
            audit.reason_code = "security_lockdown_active".to_owned();
            self.record_audit(audit).await?;
            return Err(Status::failed_precondition(
                "authorization_denied/security_lockdown_active",
            ));
        }
        Ok(())
    }

    async fn audit_authentication_failure(&self) -> Result<(), Status> {
        let mut draft = AuditRecordDraft::new(
            crate::identity::now_timestamp()?,
            AuditEventKind::LoginFailed,
        );
        draft.reason_code = "transport_principal_authentication_failed".to_owned();
        self.record_audit(draft).await
    }

    async fn audit_control_surface_denied(
        &self,
        issuer: &dyn IssuerContext,
        target: Option<&patchbay_core::authority::ControlSurfaceRevocationTarget>,
        reason_code: &str,
    ) -> Result<(), Status> {
        let mut draft = AuditRecordDraft::new(
            crate::identity::now_timestamp()?,
            AuditEventKind::AuthorizationFailed,
        );
        draft.actor_id = issuer.verified_actor().cloned();
        draft.endpoint_id = issuer.verified_endpoint().cloned();
        draft.device_id = issuer.verified_device().cloned();
        draft.target_scope = target.map(revocation_target_scope);
        draft.failure_code = Some(FailureCode::AuthorizationDenied);
        draft.reason_code = reason_code.to_owned();
        self.record_audit(draft).await
    }

    async fn audit_revocation_denied(&self, issuer: &dyn IssuerContext) -> Result<(), Status> {
        let mut draft = AuditRecordDraft::new(
            crate::identity::now_timestamp()?,
            AuditEventKind::AuthorizationFailed,
        );
        // Deliberately omit grant_id: missing, foreign, and endpoint-mismatched
        // requests must be indistinguishable to both the caller and audit
        // consumers that do not already know the grant.
        draft.actor_id = issuer.verified_actor().cloned();
        draft.endpoint_id = issuer.verified_endpoint().cloned();
        draft.device_id = issuer.verified_device().cloned();
        draft.failure_code = Some(FailureCode::AuthorizationDenied);
        draft.reason_code = "grant_revocation_authorization_denied".to_owned();
        self.record_audit(draft).await
    }

    async fn audit_submission_unknown(
        &self,
        issuer: &dyn IssuerContext,
        operation: &patchbay_contracts::patchbay::Operation,
    ) -> Result<(), Status> {
        let mut draft = AuditRecordDraft::new(
            crate::identity::now_timestamp()?,
            AuditEventKind::CommandSubmissionUnknown,
        );
        draft.actor_id = issuer.verified_actor().cloned();
        draft.endpoint_id = issuer.verified_endpoint().cloned();
        draft.device_id = issuer.verified_device().cloned();
        draft.command_id = operation.command_id.clone();
        draft.target_scope = operation.target_scope.clone();
        draft.reason_code = "submission_outcome_unknown".to_owned();
        self.audit
            .record(draft)
            .await
            .map_err(|error| Status::unavailable(error.to_string()))?;
        Ok(())
    }

    async fn audit_submission_rejection(
        &self,
        issuer: &dyn IssuerContext,
        operation: &patchbay_contracts::patchbay::Operation,
        result: &SubmissionResult,
    ) -> Result<(), Status> {
        let failure = FailureCode::try_from(result.failure_code).ok();
        let kind = match result.reason_code.as_str() {
            "grant_expired" => AuditEventKind::GrantExpired,
            "grant_revoked" => AuditEventKind::AuthorizationFailed,
            _ if failure == Some(FailureCode::AuthorizationDenied) => AuditEventKind::AuthorizationFailed,
            _ if failure == Some(FailureCode::TargetNotFound)
                && operation.target_scope.as_ref().and_then(|target| target.session_generation.as_ref()).is_some() => AuditEventKind::TargetGenerationMismatch,
            _ => AuditEventKind::CommandSubmissionRejected,
        };
        let mut draft = AuditRecordDraft::new(crate::identity::now_timestamp()?, kind);
        draft.actor_id = issuer.verified_actor().cloned();
        draft.endpoint_id = issuer.verified_endpoint().cloned();
        draft.device_id = issuer.verified_device().cloned();
        draft.command_id = operation.command_id.clone();
        draft.grant_id = result.decision_grant_id.clone();
        draft.target_scope = operation.target_scope.clone();
        draft.failure_code = failure.filter(|code| *code != FailureCode::Unspecified);
        draft.reason_code = if result.reason_code.is_empty() {
            "submission_rejected".to_owned()
        } else {
            result.reason_code.clone()
        };
        self.audit
            .record(draft)
            .await
            .map_err(|error| Status::unavailable(error.to_string()))?;
        Ok(())
    }

    pub(crate) async fn record_audit(
        &self,
        mut draft: AuditRecordDraft,
    ) -> Result<(), Status> {
        if draft.occurred_at.seconds == 0 && draft.occurred_at.nanos == 0 {
            draft.occurred_at = crate::identity::now_timestamp()?;
        }
        self.audit
            .record(draft)
            .await
            .map_err(|error| Status::unavailable(error.to_string()))?;
        Ok(())
    }

    async fn audit_login(
        &self,
        actor_id: &ActorId,
        network_address: &str,
        outcome: LoginAuditOutcome,
        reason: &'static str,
        blocked_dimensions: Vec<LoginLimitDimension>,
    ) -> Result<(), Status> {
        let kind = match outcome {
            LoginAuditOutcome::Success => AuditEventKind::LoginSucceeded,
            LoginAuditOutcome::Failure => AuditEventKind::LoginFailed,
        };
        let mut draft = AuditRecordDraft::new(crate::identity::now_timestamp()?, kind);
        draft.actor_id = Some(actor_id.clone());
        if network_address != "unknown" {
            draft.source_network = network_address.to_owned();
        }
        draft.reason_code = reason.to_owned();
        self.audit
            .record(draft)
            .await
            .map_err(|error| Status::unavailable(error.to_string()))?;
        self.login_audit.record(LoginAuditEvent {
            operator_actor_id: actor_id.value.clone(),
            direct_socket_address: network_address.to_owned(),
            outcome,
            reason,
            blocked_dimensions,
        });
        Ok(())
    }

    pub(crate) fn require_configured_domain(
        &self,
        actual: &AuthorityDomainId,
    ) -> Result<(), Status> {
        if actual != &self.authority_domain_id {
            return Err(Status::invalid_argument(
                "request authority domain does not match this core",
            ));
        }
        Ok(())
    }
}

fn validate_revocation_reason(reason: &str) -> Result<(), Status> {
    if reason.is_empty() || reason.len() > 128 || !reason.bytes().all(|byte| byte.is_ascii_graphic() && byte != b'=' ) {
        return Err(Status::invalid_argument("reason must be 1..128 safe ASCII characters"));
    }
    Ok(())
}

fn revocation_effects(
    grant: &GrantRecord,
    records: &[CommandRecord],
) -> Vec<GrantRevocationEffect> {
    records
        .iter()
        .filter_map(|record| {
            let (to_state, failure_code) = match grant.revocation_policy {
                GrantRevocationPolicy::Continue => return None,
                GrantRevocationPolicy::Cancel if matches!(record.state, OperationState::Accepted | OperationState::Delivered | OperationState::Running) => (OperationState::Cancelled, FailureCode::Cancelled),
                GrantRevocationPolicy::RequireReauthorization if record.state == OperationState::Accepted => (OperationState::Rejected, FailureCode::AuthorizationDenied),
                _ => return None,
            };
            Some(GrantRevocationEffect {
                command_id: Some(record.command_id.clone()),
                from_state: record.state as i32,
                to_state: to_state as i32,
                failure_code: failure_code as i32,
            })
        })
        .collect()
}

async fn append_query_transition<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    command_id: &patchbay_contracts::patchbay::CommandId,
    from: OperationState,
    to: OperationState,
) -> Result<EventId, StorageError> {
    let kind = match to {
        OperationState::Delivered => AuditEventKind::CommandDelivered,
        OperationState::Completed => AuditEventKind::CommandCompleted,
        OperationState::Failed => AuditEventKind::CommandFailed,
        _ => AuditEventKind::CommandSubmissionFailed,
    };
    let mut audit = AuditRecordDraft::new(crate::identity::now_timestamp().map_err(|error| StorageError::InvalidAuditRecord(error.to_string()))?, kind);
    audit.command_id = Some(command_id.clone());
    audit.reason_code = "query_state_transition".to_owned();
    storage
        .append_decision(
            authority_domain_id,
            patchbay_contracts::patchbay::StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: CommandTransition {
                    command_id: Some(command_id.clone()),
                    from_state: from as i32,
                    to_state: to as i32,
                    failure_code: FailureCode::Unspecified as i32,
                    ..CommandTransition::default()
                }
                .encode_to_vec(),
            },
            audit,
        )
        .await
}

async fn append_query_failure<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    command_id: &patchbay_contracts::patchbay::CommandId,
) -> Result<EventId, StorageError> {
    let mut audit = AuditRecordDraft::new(
        crate::identity::now_timestamp().map_err(|error| StorageError::InvalidAuditRecord(error.to_string()))?,
        AuditEventKind::CommandFailed,
    );
    audit.command_id = Some(command_id.clone());
    // `ExecutionFailed` is the canonical failure vocabulary for an accepted
    // operation whose diagnostic read could not be materialized. Keep the
    // more specific materialization reason in the bounded reason-code field;
    // never copy backend/error display text into an audit field.
    audit.failure_code = Some(FailureCode::ExecutionFailed);
    audit.reason_code = "diagnostics_materialization_failed".to_owned();
    storage
        .append_decision(
            authority_domain_id,
            patchbay_contracts::patchbay::StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: CommandTransition {
                    command_id: Some(command_id.clone()),
                    from_state: OperationState::Delivered as i32,
                    to_state: OperationState::Failed as i32,
                    failure_code: FailureCode::ExecutionFailed as i32,
                    ..CommandTransition::default()
                }
                .encode_to_vec(),
            },
            audit,
        )
        .await
}

async fn materialize_diagnostics_result<S: Storage>(
    storage: &S,
    state: &ProjectionState,
    authority_domain_id: &AuthorityDomainId,
    query: ValidatedDiagnosticsQuery,
    as_of_lsn: u64,
) -> Result<(
    DiagnosticsResult,
    patchbay_contracts::patchbay::query_diagnostics_response::Result,
), diagnostics::DiagnosticsError> {
    let as_of = Some(Lsn { value: as_of_lsn });
    match query {
        ValidatedDiagnosticsQuery::Audit(spec) => {
            let page = storage
                .query_audit_through(authority_domain_id, spec, Lsn { value: as_of_lsn })
                .await?;
            Ok((
                DiagnosticsResult {
                    as_of_lsn: as_of,
                    result: Some(patchbay_contracts::patchbay::diagnostics_result::Result::Audit(page.clone())),
                },
                patchbay_contracts::patchbay::query_diagnostics_response::Result::Audit(page),
            ))
        }
        ValidatedDiagnosticsQuery::Command(query) => {
            let command_id = query.command_id.clone().expect("validated command id");
            let projection = state.diagnostics_at(storage, authority_domain_id, as_of_lsn).await?;
            let mut inspection = projection.result_for_query(&command_id).unwrap_or(
                patchbay_contracts::patchbay::CommandInspectionResult { found: false, inspection: None },
            );
            if let Some(command) = inspection.inspection.as_mut() {
                let before_lsn = query.audit_before_event_id.as_ref().and_then(|id| id.lsn.as_ref()).map(|lsn| lsn.value);
                let page = storage.query_audit_through(authority_domain_id, AuditPageSpec {
                    kinds: Vec::new(), actor_id: None, endpoint_id: None,
                    command_id: Some(command_id), grant_id: None, target: None, failure_codes: Vec::new(),
                    reason_codes: Vec::new(), occurred_from: None, occurred_before: None,
                    before_lsn,
                    limit: query.audit_limit.map_or(diagnostics::COMMAND_DEFAULT_LIMIT, |value| value as u16),
                }, Lsn { value: as_of_lsn }).await?;
                command.audit = Some(page);
            }
            Ok((
                DiagnosticsResult {
                    as_of_lsn: as_of,
                    result: Some(patchbay_contracts::patchbay::diagnostics_result::Result::Command(inspection.clone())),
                },
                patchbay_contracts::patchbay::query_diagnostics_response::Result::Command(inspection),
            ))
        }
        ValidatedDiagnosticsQuery::Adapters(query) => {
            let projection = state.diagnostics_at(storage, authority_domain_id, as_of_lsn).await?;
            let page = projection.adapter_page(&query, as_of_lsn)?;
            Ok((
                DiagnosticsResult {
                    as_of_lsn: as_of,
                    result: Some(patchbay_contracts::patchbay::diagnostics_result::Result::Adapters(page.clone())),
                },
                patchbay_contracts::patchbay::query_diagnostics_response::Result::Adapters(page),
            ))
        }
    }
}

fn rejected_query_lockdown_submission(
    operation: &patchbay_contracts::patchbay::Operation,
    reason_code: String,
) -> SubmissionResult {
    SubmissionResult {
        outcome: SubmissionOutcome::Rejected as i32,
        command_id: operation.command_id.clone(),
        operation_state: OperationState::Unspecified as i32,
        failure_code: FailureCode::AuthorizationDenied as i32,
        diagnostic_message: format!("security lockdown is active: {reason_code}"),
        accepted_lsn: None,
        deduplicated: false,
        decision_grant_id: None,
        reason_code: "security_lockdown_active".to_owned(),
    }
}

fn rejected_query_submission(
    operation: &patchbay_contracts::patchbay::Operation,
    diagnostic_message: String,
) -> SubmissionResult {
    SubmissionResult {
        outcome: SubmissionOutcome::Rejected as i32,
        command_id: operation.command_id.clone(),
        operation_state: OperationState::Unspecified as i32,
        failure_code: FailureCode::ValidationFailed as i32,
        diagnostic_message,
        accepted_lsn: None,
        deduplicated: false,
        decision_grant_id: None,
        reason_code: "validation_failed".to_owned(),
    }
}

async fn read_validated_replay_prefix<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<Vec<RecordedEvent>, Status> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await
        .map_err(map_storage_error_to_status)?;
    let mut previous_lsn = 0;
    for event in &events {
        previous_lsn = validate_next_replay_event(authority_domain_id, previous_lsn, event)
            .map_err(|error| Status::internal(error.to_string()))?
            .lsn;
    }
    Ok(events)
}

async fn find_delivered_checkpoint<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    command_id: &patchbay_contracts::patchbay::CommandId,
) -> Result<Option<EventId>, Status> {
    let events = read_validated_replay_prefix(storage, authority_domain_id).await?;
    let mut transition_checkpoint = None;
    let mut audit_checkpoint = None;
    for event in events {
        match StoredEventKind::try_from(event.payload.kind).ok() {
            Some(StoredEventKind::CommandTransition) => {
                let transition = CommandTransition::decode(event.payload.payload.as_slice())
                    .map_err(|error| {
                        Status::internal(format!("cannot decode query transition: {error}"))
                    })?;
                if transition.command_id.as_ref() == Some(command_id)
                    && OperationState::try_from(transition.to_state).ok()
                        == Some(OperationState::Delivered)
                {
                    transition_checkpoint = Some(event.event_id);
                }
            }
            Some(StoredEventKind::AuditRecord) => {
                let record =
                    AuditRecord::decode(event.payload.payload.as_slice()).map_err(|error| {
                        Status::internal(format!("cannot decode query audit: {error}"))
                    })?;
                if transition_checkpoint
                    .as_ref()
                    .is_some_and(|checkpoint| record.source_event_id.as_ref() == Some(checkpoint))
                {
                    audit_checkpoint = Some(event.event_id);
                }
            }
            _ => {}
        }
    }
    Ok(audit_checkpoint.or(transition_checkpoint))
}

async fn find_diagnostics_result<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    command_id: &patchbay_contracts::patchbay::CommandId,
) -> Result<
    Option<(
        EventId,
        DiagnosticsResult,
        patchbay_contracts::patchbay::query_diagnostics_response::Result,
    )>,
    Status,
> {
    let events = read_validated_replay_prefix(storage, authority_domain_id).await?;
    for event in events {
        if StoredEventKind::try_from(event.payload.kind).ok() != Some(StoredEventKind::Observation)
        {
            continue;
        }
        let observation = Observation::decode(event.payload.payload.as_slice())
            .map_err(|error| Status::internal(format!("cannot decode observation: {error}")))?;
        if observation
            .payload
            .as_ref()
            .is_none_or(|payload| payload.schema_ref != "patchbay.DiagnosticsResult")
            || !observation.correlations.iter().any(|correlation| {
                matches!(
                    correlation.r#ref.as_ref(),
                    Some(typed_correlation::Ref::CommandId(id)) if id == command_id
                )
            })
        {
            continue;
        }
        let payload = observation.payload.expect("checked above");
        let result = DiagnosticsResult::decode(payload.payload.as_slice()).map_err(|error| {
            Status::internal(format!("cannot decode diagnostics result: {error}"))
        })?;
        let response_result = match result
            .result
            .clone()
            .ok_or_else(|| Status::internal("diagnostics result has no result"))?
        {
            patchbay_contracts::patchbay::diagnostics_result::Result::Audit(page) => {
                patchbay_contracts::patchbay::query_diagnostics_response::Result::Audit(page)
            }
            patchbay_contracts::patchbay::diagnostics_result::Result::Command(result) => {
                patchbay_contracts::patchbay::query_diagnostics_response::Result::Command(result)
            }
            patchbay_contracts::patchbay::diagnostics_result::Result::Adapters(page) => {
                patchbay_contracts::patchbay::query_diagnostics_response::Result::Adapters(page)
            }
        };
        return Ok(Some((event.event_id, result, response_result)));
    }
    Ok(None)
}

fn subscription_scope() -> patchbay_contracts::patchbay::TargetScope {
    patchbay_contracts::patchbay::TargetScope {
        kind: patchbay_contracts::patchbay::TargetScopeKind::AuthorityDomain as i32,
        ..patchbay_contracts::patchbay::TargetScope::default()
    }
}

fn operator_facing_subscribe_event(event: RecordedEvent) -> Option<SubscribeEvent> {
    let kind = StoredEventKind::try_from(event.payload.kind).ok()?;
    matches!(
        kind,
        StoredEventKind::Operation
            | StoredEventKind::Observation
            | StoredEventKind::Elicitation
            | StoredEventKind::SessionState
            | StoredEventKind::ResourceState
            | StoredEventKind::CommandTransition
            | StoredEventKind::SecurityLockdown
    )
    .then_some(SubscribeEvent {
        event_id: Some(event.event_id),
        payload: Some(event.payload),
    })
}

fn issuer_to_endpoint_ref(issuer: &MetadataIssuerContext) -> ActorEndpointRef {
    let binding = issuer.binding();
    ActorEndpointRef {
        actor_id: Some(binding.actor_id),
        endpoint_id: Some(binding.endpoint_id),
        device_id: Some(binding.device_id),
        endpoint_generation: Some(binding.endpoint_generation),
    }
}

fn validate_control_surface_reason(reason_code: &str) -> Result<(), Status> {
    if reason_code.is_empty()
        || reason_code.len() > 64
        || !reason_code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(Status::invalid_argument(
            "reason_code must match [a-z0-9_]{1,64}",
        ));
    }
    Ok(())
}

fn revocation_target_scope(
    target: &patchbay_core::authority::ControlSurfaceRevocationTarget,
) -> patchbay_contracts::patchbay::TargetScope {
    let legacy_audit_resource_id = match target {
        patchbay_core::authority::ControlSurfaceRevocationTarget::Principal(id) => id.clone(),
        patchbay_core::authority::ControlSurfaceRevocationTarget::Endpoint(id) => id.value.clone(),
        patchbay_core::authority::ControlSurfaceRevocationTarget::Device(id) => id.value.clone(),
    };
    patchbay_contracts::patchbay::TargetScope {
        kind: patchbay_contracts::patchbay::TargetScopeKind::Resource as i32,
        legacy_audit_resource_id,
        ..patchbay_contracts::patchbay::TargetScope::default()
    }
}

fn caller_network_address<T>(request: &Request<T>) -> String {
    request
        .remote_addr()
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn required_actor(actor: Option<ActorId>) -> Result<ActorId, Status> {
    let actor = actor.ok_or_else(|| Status::invalid_argument("missing operator_actor_id"))?;
    if actor.value.is_empty() {
        return Err(Status::invalid_argument(
            "operator_actor_id must not be empty",
        ));
    }
    Ok(actor)
}

fn authority_domain_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::AuthorityDomain as i32,
        ..TargetScope::default()
    }
}

fn required_domain(domain: Option<AuthorityDomainId>) -> Result<AuthorityDomainId, Status> {
    let domain = domain.ok_or_else(|| Status::invalid_argument("missing authority_domain_id"))?;
    if domain.value.is_empty() {
        return Err(Status::invalid_argument(
            "authority_domain_id must not be empty",
        ));
    }
    Ok(domain)
}

pub fn map_acceptance_error_to_status(error: AcceptanceError) -> Status {
    match error {
        AcceptanceError::Storage(error) => map_storage_error_to_status(error),
        AcceptanceError::InvalidTargetScope(message) => Status::invalid_argument(message),
        AcceptanceError::AlreadyTerminal(message) => Status::failed_precondition(message),
        AcceptanceError::CorruptRecord(message) | AcceptanceError::CorruptLog(message) => {
            Status::internal(message)
        }
    }
}

pub fn map_authority_error_to_status(error: patchbay_core::authority::AuthorityError) -> Status {
    match error {
        patchbay_core::authority::AuthorityError::GrantNotFound(_) => Status::permission_denied("grant revocation is not authorized"),
        patchbay_core::authority::AuthorityError::InvalidGrant(message) => Status::invalid_argument(message),
        patchbay_core::authority::AuthorityError::CorruptRecord(message)
        | patchbay_core::authority::AuthorityError::CorruptLog(message) => Status::internal(message),
        patchbay_core::authority::AuthorityError::Storage(error) => map_storage_error_to_status(error),
    }
}

pub fn map_operator_error_to_status(error: OperatorError) -> Status {
    match error {
        OperatorError::AlreadyBootstrapped => {
            Status::already_exists("operator bootstrap has already completed")
        }
        OperatorError::OperatorNotFound => {
            Status::failed_precondition("operator bootstrap is required")
        }
        OperatorError::PrincipalNotFound | OperatorError::EndpointNotFound => {
            Status::not_found("control-surface target was not found")
        }
        OperatorError::RevokedIdentity(message) => Status::permission_denied(message),
        OperatorError::InvalidRecord(message) => Status::invalid_argument(message),
        OperatorError::CorruptRecord(message) | OperatorError::CorruptLog(message) => {
            Status::internal(message)
        }
        OperatorError::Storage(error) => map_storage_error_to_status(error),
    }
}

pub fn map_storage_error_to_status(error: StorageError) -> Status {
    match error {
        StorageError::Unavailable(message) => retryable_unavailable(message),
        StorageError::WriteFailed {
            message,
            retryable: true,
        }
        | StorageError::ReadFailed {
            message,
            retryable: true,
        } => retryable_unavailable(message),
        StorageError::IdempotencyConflict => {
            Status::failed_precondition("idempotency key conflicts with the existing operation")
        }
        StorageError::GrantIdentityConflict {
            grant_id,
            existing_lsn,
        } => Status::failed_precondition(format!(
            "grant identity {grant_id} conflicts with source LSN {existing_lsn}"
        )),
        StorageError::CorruptRecord(message) => Status::internal(message),
        StorageError::WriteFailed { message, .. } | StorageError::ReadFailed { message, .. } => {
            Status::internal(message)
        }
        StorageError::SnapshotStale(lsn) => {
            Status::failed_precondition(format!("snapshot LSN {lsn} is stale"))
        }
        StorageError::SnapshotWrongDomain => {
            Status::failed_precondition("snapshot belongs to another authority domain")
        }
        StorageError::InvalidSnapshotLsn(lsn) => {
            Status::invalid_argument(format!("snapshot LSN {lsn} is not committed"))
        }
        StorageError::InvalidEventKind => Status::internal("stored event kind is invalid"),
        StorageError::InvalidAuditRecord(message) | StorageError::InvalidAuditCursor(message) => {
            Status::invalid_argument(message)
        }
        StorageError::UnsupportedSchemaVersion(version) => {
            Status::failed_precondition(format!("database schema version {version} is unsupported"))
        }
        StorageError::MalformedSchema(message) => Status::internal(message),
        StorageError::InvalidCoreGeneration(value) => {
            Status::internal(format!("invalid core generation {value}"))
        }
        StorageError::UnsupportedOperation => Status::unimplemented("storage operation is unsupported"),
    }
}

fn retryable_unavailable(message: String) -> Status {
    Status::with_error_details(
        Code::Unavailable,
        message,
        ErrorDetails::with_retry_info(Some(Duration::from_secs(1))),
    )
}

#[cfg(test)]
mod tests {
    use super::{find_delivered_checkpoint, find_diagnostics_result, revocation_effects};
    use patchbay_contracts::patchbay::{
        diagnostics_result, typed_correlation, ActorId, AuditRecord, AuthorityDomainId, CommandId,
        CommandTransition, DiagnosticsResult, EventId, GrantId, GrantRevocationPolicy, Lsn,
        Observation, ObservationKind, Operation, OperationKind, OperationState, PayloadContentType,
        PayloadEnvelope, StoredEventKind, StoredEventPayload, TargetScope, TypedCorrelation,
    };
    use patchbay_core::{
        acceptance::CommandRecord,
        authority::{GrantProvenanceKind, GrantRecord},
        storage::{DedupOutcome, RecordedEvent, Storage, StorageError, StoredSnapshot, TargetKey},
    };
    use prost::Message;

    fn grant(policy: GrantRevocationPolicy) -> GrantRecord {
        GrantRecord {
            grant_id: GrantId { value: "grant".to_owned() },
            authority_domain_id: AuthorityDomainId { value: "domain".to_owned() },
            subject_actor_id: ActorId { value: "actor".to_owned() },
            subject_endpoint_id: None,
            subject_endpoint_class: String::new(),
            target_scope: TargetScope::default(),
            allowed_operation_kinds: vec![OperationKind::Instruct],
            created_at: None,
            expires_at: None,
            revocation_generation: None,
            revoked_at: None,
            revocation_policy: policy,
            revoked_by: None,
            revocation_reason: String::new(),
            revocation_audit_id: None,
            is_descendant: false,
            provenance: GrantProvenanceKind::Operator {
                created_by: None,
                created_by_operation_id: None,
                audit_id: None,
                reason: "test".to_owned(),
            },
        }
    }

    fn record(state: OperationState) -> CommandRecord {
        let mut record = CommandRecord::new(
            Operation {
                command_id: Some(CommandId { value: "command".to_owned() }),
                ..Operation::default()
            },
            1,
        )
        .expect("test command has an id");
        record.state = state;
        record
    }

    #[derive(Clone)]
    struct ScriptedReplayStorage {
        events: Vec<RecordedEvent>,
    }

    impl Storage for ScriptedReplayStorage {
        async fn append(
            &self,
            _authority_domain_id: &AuthorityDomainId,
            _payload: StoredEventPayload,
        ) -> Result<EventId, StorageError> {
            Err(StorageError::UnsupportedOperation)
        }

        async fn append_dedup(
            &self,
            _authority_domain_id: &AuthorityDomainId,
            _key: &patchbay_contracts::patchbay::IdempotencyKey,
            _target: &TargetKey,
            _payload: StoredEventPayload,
        ) -> Result<DedupOutcome, StorageError> {
            Err(StorageError::UnsupportedOperation)
        }

        async fn read_after(
            &self,
            _authority_domain_id: &AuthorityDomainId,
            _cursor: Lsn,
        ) -> Result<Vec<RecordedEvent>, StorageError> {
            Ok(self.events.clone())
        }

        async fn write_snapshot(
            &self,
            _authority_domain_id: &AuthorityDomainId,
            _snapshot_lsn: Lsn,
            _snapshot_payload: Vec<u8>,
        ) -> Result<(), StorageError> {
            Err(StorageError::UnsupportedOperation)
        }

        async fn load_latest_snapshot(
            &self,
            _authority_domain_id: &AuthorityDomainId,
            _at_or_before: Option<Lsn>,
        ) -> Result<Option<StoredSnapshot>, StorageError> {
            Ok(None)
        }
    }

    fn stored_event(
        authority_domain_id: &AuthorityDomainId,
        lsn: u64,
        kind: StoredEventKind,
        payload: Vec<u8>,
    ) -> RecordedEvent {
        RecordedEvent {
            event_id: EventId {
                authority_domain_id: Some(authority_domain_id.clone()),
                lsn: Some(Lsn { value: lsn }),
            },
            payload: StoredEventPayload {
                kind: kind as i32,
                payload,
            },
        }
    }

    fn delivered_transition(
        authority_domain_id: &AuthorityDomainId,
        command_id: &CommandId,
    ) -> RecordedEvent {
        stored_event(
            authority_domain_id,
            1,
            StoredEventKind::CommandTransition,
            CommandTransition {
                command_id: Some(command_id.clone()),
                to_state: OperationState::Delivered as i32,
                ..CommandTransition::default()
            }
            .encode_to_vec(),
        )
    }

    fn diagnostics_result(
        authority_domain_id: &AuthorityDomainId,
        command_id: &CommandId,
    ) -> RecordedEvent {
        let result = DiagnosticsResult {
            as_of_lsn: Some(Lsn { value: 1 }),
            result: Some(diagnostics_result::Result::Audit(Default::default())),
        };
        stored_event(
            authority_domain_id,
            1,
            StoredEventKind::Observation,
            Observation {
                authority_domain_id: Some(authority_domain_id.clone()),
                kind: ObservationKind::Result as i32,
                correlations: vec![TypedCorrelation {
                    r#ref: Some(typed_correlation::Ref::CommandId(command_id.clone())),
                }],
                payload: Some(PayloadEnvelope {
                    payload: result.encode_to_vec(),
                    content_type: PayloadContentType::Protobuf as i32,
                    schema_ref: "patchbay.DiagnosticsResult".to_owned(),
                }),
                ..Observation::default()
            }
            .encode_to_vec(),
        )
    }

    fn corrupt_tails(authority_domain_id: &AuthorityDomainId) -> [RecordedEvent; 2] {
        [
            stored_event(
                authority_domain_id,
                3,
                StoredEventKind::Observation,
                Observation::default().encode_to_vec(),
            ),
            stored_event(
                authority_domain_id,
                2,
                StoredEventKind::Unspecified,
                Vec::new(),
            ),
        ]
    }

    #[tokio::test]
    async fn delivered_checkpoint_search_validates_the_complete_replay_prefix() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let command_id = CommandId {
            value: "query-command".to_owned(),
        };

        for corrupt_tail in corrupt_tails(&authority_domain_id) {
            let storage = ScriptedReplayStorage {
                events: vec![
                    delivered_transition(&authority_domain_id, &command_id),
                    corrupt_tail,
                ],
            };
            let error = find_delivered_checkpoint(&storage, &authority_domain_id, &command_id)
                .await
                .expect_err("a gap or unspecified kind must reject before checkpoint search");
            assert_eq!(error.code(), tonic::Code::Internal);
            assert!(error.message().contains("corrupt replay"));
        }
    }

    #[tokio::test]
    async fn diagnostics_result_search_validates_the_complete_replay_prefix() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let command_id = CommandId {
            value: "query-command".to_owned(),
        };

        for corrupt_tail in corrupt_tails(&authority_domain_id) {
            let storage = ScriptedReplayStorage {
                events: vec![
                    diagnostics_result(&authority_domain_id, &command_id),
                    corrupt_tail,
                ],
            };
            let error = find_diagnostics_result(&storage, &authority_domain_id, &command_id)
                .await
                .expect_err("a gap or unspecified kind must reject before result search");
            assert_eq!(error.code(), tonic::Code::Internal);
            assert!(error.message().contains("corrupt replay"));
        }
    }

    #[tokio::test]
    async fn delivered_checkpoint_requires_an_exact_present_audit_source() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let command_id = CommandId {
            value: "query-command".to_owned(),
        };
        let source_less_audit = stored_event(
            &authority_domain_id,
            1,
            StoredEventKind::AuditRecord,
            AuditRecord::default().encode_to_vec(),
        );
        let source_less = ScriptedReplayStorage {
            events: vec![source_less_audit],
        };
        assert_eq!(
            find_delivered_checkpoint(&source_less, &authority_domain_id, &command_id)
                .await
                .unwrap(),
            None,
            "None == None must not turn a source-less audit into a checkpoint"
        );

        let transition = delivered_transition(&authority_domain_id, &command_id);
        let matching_audit = stored_event(
            &authority_domain_id,
            2,
            StoredEventKind::AuditRecord,
            AuditRecord {
                source_event_id: Some(transition.event_id.clone()),
                ..AuditRecord::default()
            }
            .encode_to_vec(),
        );
        let expected = matching_audit.event_id.clone();
        let exact_match = ScriptedReplayStorage {
            events: vec![transition, matching_audit],
        };
        assert_eq!(
            find_delivered_checkpoint(&exact_match, &authority_domain_id, &command_id)
                .await
                .unwrap(),
            Some(expected)
        );
    }

    #[test]
    fn revocation_policy_state_matrix_has_only_designed_effects() {
        let states = [
            OperationState::Accepted,
            OperationState::Delivered,
            OperationState::Running,
            OperationState::Completed,
        ];
        for policy in [
            GrantRevocationPolicy::Continue,
            GrantRevocationPolicy::Cancel,
            GrantRevocationPolicy::RequireReauthorization,
        ] {
            for state in states {
                let effects = revocation_effects(&grant(policy), &[record(state)]);
                let expected = match (policy, state) {
                    (GrantRevocationPolicy::Cancel, OperationState::Accepted | OperationState::Delivered | OperationState::Running) => Some((OperationState::Cancelled, patchbay_contracts::patchbay::FailureCode::Cancelled)),
                    (GrantRevocationPolicy::RequireReauthorization, OperationState::Accepted) => Some((OperationState::Rejected, patchbay_contracts::patchbay::FailureCode::AuthorizationDenied)),
                    _ => None,
                };
                assert_eq!(effects.len(), usize::from(expected.is_some()), "policy={policy:?}, state={state:?}");
                if let Some((to_state, failure_code)) = expected {
                    assert_eq!(OperationState::try_from(effects[0].to_state), Ok(to_state));
                    assert_eq!(patchbay_contracts::patchbay::FailureCode::try_from(effects[0].failure_code), Ok(failure_code));
                }
            }
        }
    }
}
