use std::{pin::Pin, sync::Arc, time::Duration};

use patchbay_contracts::patchbay::{
    ActorEndpointRef, ActorId, AuditEventKind, AuditRecord, AuthorityDomainId, CommandTransition,
    DiagnosticsResult, GrantRevocationEffect, GrantRevocationPolicy, Generation, Revocation,
    EnrollControlSurfacePrincipalRequest, EnrollControlSurfacePrincipalResult, EventId,
    LoadSnapshotRequest, LoadSnapshotResponse, Lsn,
    QueryDiagnosticsRequest, QueryDiagnosticsResponse, RecordControlSurfaceAuditRequest,
    RevokeGrantRequest, RevokeGrantResult,
    RecordControlSurfaceAuditResponse, RevokeOperatorSessionRequest, RevokeOperatorSessionResult,
    StoredEventKind, SubmissionOutcome, SubmissionResult, SubmitRequest,
    SubscribeEvent, SubscribeRequest, Observation, ObservationKind, PayloadContentType,
    TypedCorrelation, typed_correlation, OperationState, FailureCode,
    VerifyOperatorPasswordRequest, VerifyOperatorPasswordResult,
};
use patchbay_core::{
    acceptance::{self, AcceptanceError, CommandRecord, CommandStateLookup},
    time::{Clock, SystemClock},
    audit::{AuditReceipt, AuditSink, DurableAuditSink, RequiredAuditFanout, StderrAuditSink},
    authority::{authorize_self_revocation_at, hash_principal_credential, GrantAdministrationDenied, GrantRecord, IssuerContext, IssuerRef, OperatorError},
    diagnostics::{self, AuthorityDomainTargetResolver, ValidatedDiagnosticsQuery},
    storage::{AuditPageSpec, AuditRecordDraft, RecordedEvent, Storage, StorageError},
};
use prost::Message;
use tokio_stream::{self as stream, Stream};
use tonic::{service::Interceptor, Code, Request, Response, Status};
use tonic_types::{ErrorDetails, StatusExt};

use crate::{
    identity::issue_principal,
    issuer::MetadataIssuerContext,
    login_security::{
        LoginAuditEvent, LoginAuditOutcome, LoginAuditSink, LoginLimitDimension, LoginLimiter,
        StderrLoginAuditSink,
    },
    operator_session::DEFAULT_OPERATOR_SESSION_TTL,
    rpc::control_service_server::ControlService,
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
}

impl<S> ControlServiceImpl<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    pub async fn new(storage: S, authority_domain_id: AuthorityDomainId) -> Result<Self, String> {
        Self::new_with_security(
            storage,
            authority_domain_id,
            DEFAULT_OPERATOR_SESSION_TTL,
            LoginLimiter::default(),
            Arc::new(StderrLoginAuditSink),
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
        Self::new_with_clock_security(
            storage,
            authority_domain_id,
            operator_session_ttl,
            login_limiter,
            login_audit,
            Arc::new(SystemClock),
        )
        .await
    }

    pub async fn new_with_clock(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, String> {
        Self::new_with_clock_security(
            storage,
            authority_domain_id,
            DEFAULT_OPERATOR_SESSION_TTL,
            LoginLimiter::default(),
            Arc::new(StderrLoginAuditSink),
            clock,
        )
        .await
    }

    async fn new_with_clock_security(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        operator_session_ttl: Duration,
        login_limiter: LoginLimiter,
        login_audit: Arc<dyn LoginAuditSink>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, String> {
        if authority_domain_id.value.is_empty() {
            return Err("authority domain id must not be empty".to_owned());
        }
        let state = ProjectionState::rebuild_with_session_ttl(
            &storage,
            &authority_domain_id,
            operator_session_ttl,
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
        })
    }

    pub async fn is_bootstrapped(&self) -> bool {
        self.state.operator_exists().await
    }
}

type SubscribeStream = Pin<Box<dyn Stream<Item = Result<SubscribeEvent, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl<S> ControlService for ControlServiceImpl<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    async fn submit(
        &self,
        request: Request<SubmitRequest>,
    ) -> Result<Response<SubmissionResult>, Status> {
        let operation = request
            .get_ref()
            .operation
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("submit request is missing operation"))?;
        let authority_domain_id = operation
            .authority_domain_id
            .clone()
            .ok_or_else(|| Status::invalid_argument("operation is missing authority_domain_id"))?;
        self.require_configured_domain(&authority_domain_id)?;
        let issuer =
            MetadataIssuerContext::from_request(&request, authority_domain_id.clone(), &self.state)
                .await?;
        let operation = request.into_inner().operation.ok_or_else(|| {
            Status::invalid_argument("submit request lost its validated operation")
        })?;

        // Reconcile and submit under one gate. Pre-submit catch-up repairs the
        // projection after a prior append whose handler did not complete; the
        // post-append catch-up makes a newly durable command visible before the
        // next submit acquires the gate.
        let _submit_guard = self.state.submit_guard().await;
        self.state
            .catch_up(&self.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let result = match acceptance::submit_with_clock(
            &self.storage,
            self.state.grant_check(),
            self.state.target_resolver(),
            self.state.state_lookup(),
            self.state.elicitation_contract_lookup(),
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

    async fn revoke_grant(
        &self,
        request: Request<RevokeGrantRequest>,
    ) -> Result<Response<RevokeGrantResult>, Status> {
        let requested_domain = required_domain(request.get_ref().authority_domain_id.clone())?;
        self.require_configured_domain(&requested_domain)?;
        let issuer = MetadataIssuerContext::from_request(&request, requested_domain.clone(), &self.state).await?;
        let request = request.into_inner();
        let grant_id = request.grant_id.ok_or_else(|| Status::invalid_argument("grant_id is required"))?;
        if grant_id.value.is_empty() || grant_id.value.len() > 256 {
            return Err(Status::invalid_argument("grant_id must be non-empty and bounded"));
        }
        validate_revocation_reason(&request.reason)?;

        let _decision_guard = self.state.submit_guard().await;
        self.state.catch_up(&self.storage, &requested_domain).await.map_err(map_storage_error_to_status)?;
        let grant = self.state.grant(&grant_id).await.ok_or_else(|| Status::permission_denied("grant revocation is not authorized"))?;
        let actor = issuer.verified_actor().ok_or_else(|| Status::permission_denied("grant revocation is not authorized"))?;
        let issuer_ref = IssuerRef {
            actor,
            endpoint: issuer.verified_endpoint(),
            authority_domain_id: &requested_domain,
        };
        match authorize_self_revocation_at(&grant, &issuer_ref, &self.clock.now()) {
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
                return Err(Status::permission_denied("grant revocation is not authorized"));
            }
        }
        if grant.is_revoked() {
            return Ok(Response::new(RevokeGrantResult {
                changed: false,
                already_revoked: true,
                revocation_event_id: None,
                applied_policy: grant.revocation_policy as i32,
                command_effects: Vec::new(),
            }));
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
        MetadataIssuerContext::from_request(&request, authority_domain_id.clone(), &self.state)
            .await?;
        let request = request.into_inner();
        let cursor = request.cursor.unwrap_or(Lsn { value: 0 });
        let events = self
            .storage
            .read_after(&authority_domain_id, cursor)
            .await
            .map_err(map_storage_error_to_status)?;
        let events = events
            .into_iter()
            .filter_map(operator_facing_subscribe_event)
            .map(Ok);
        Ok(Response::new(Box::pin(stream::iter(events))))
    }

    async fn load_snapshot(
        &self,
        request: Request<LoadSnapshotRequest>,
    ) -> Result<Response<LoadSnapshotResponse>, Status> {
        let authority_domain_id = required_domain(request.get_ref().authority_domain_id.clone())?;
        self.require_configured_domain(&authority_domain_id)?;
        MetadataIssuerContext::from_request(&request, authority_domain_id.clone(), &self.state)
            .await?;
        let request = request.into_inner();

        // Snapshot reads share the submission gate and catch-up ordering so a
        // caller never observes a projection behind an already committed
        // event. If no durable checkpoint exists, serve the current rebuilt
        // session projection directly; this deliberately does not write into
        // the still-undiscriminated durable snapshot namespace.
        let _submit_guard = self.state.submit_guard().await;
        self.state
            .catch_up(&self.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        if let Some(snapshot) = self
            .storage
            .load_latest_snapshot(&authority_domain_id, request.at_or_before)
            .await
            .map_err(map_storage_error_to_status)?
        {
            return Ok(Response::new(LoadSnapshotResponse {
                present: true,
                event_id: Some(snapshot.event_id),
                snapshot_payload: snapshot.payload,
            }));
        }

        // A historical at_or_before bound cannot be reconstructed from the
        // current hot projection. Returning the newer current authoritative
        // view follows the protocol's stale-snapshot repair rule rather than
        // reporting an empty deployment.
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
        Ok(Response::new(LoadSnapshotResponse {
            present: true,
            event_id: Some(EventId {
                authority_domain_id: Some(authority_domain_id),
                lsn: Some(snapshot_lsn),
            }),
            snapshot_payload: snapshot.encode_to_vec(),
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
        let _submit_guard = self.state.submit_guard().await;
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
        let operator_session_id = self.state.issue_operator_session(actor_id.clone()).await;
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
            operator_session_id: Some(operator_session_id),
            principal: Some(credential),
        }))
    }

    async fn revoke_operator_session(
        &self,
        request: Request<RevokeOperatorSessionRequest>,
    ) -> Result<Response<RevokeOperatorSessionResult>, Status> {
        let issuer = MetadataIssuerContext::from_request(
            &request,
            self.authority_domain_id.clone(),
            &self.state,
        )
        .await?;
        let actor_id = issuer
            .verified_actor()
            .cloned()
            .ok_or_else(|| Status::internal("verified issuer lost its actor"))?;
        let revoked = self
            .state
            .revoke_operator_session(issuer.operator_session_id(), &actor_id)
            .await;
        if !revoked {
            return Err(Status::failed_precondition(
                "operator session is no longer active",
            ));
        }
        let mut session_audit = AuditRecordDraft::new(
            crate::identity::now_timestamp()?,
            AuditEventKind::OperatorSessionRevoked,
        );
        session_audit.actor_id = Some(actor_id);
        session_audit.reason_code = "operator_session_revoked".to_owned();
        self.record_audit(session_audit).await?;
        Ok(Response::new(RevokeOperatorSessionResult { revoked }))
    }

    async fn enroll_control_surface_principal(
        &self,
        request: Request<EnrollControlSurfacePrincipalRequest>,
    ) -> Result<Response<EnrollControlSurfacePrincipalResult>, Status> {
        let issuer = MetadataIssuerContext::from_request(
            &request,
            self.authority_domain_id.clone(),
            &self.state,
        )
        .await?;
        let actor_id = issuer
            .verified_actor()
            .cloned()
            .ok_or_else(|| Status::internal("verified issuer lost its actor"))?;
        let enrollment = request
            .into_inner()
            .principal
            .ok_or_else(|| Status::invalid_argument("principal enrollment is required"))?;
        let (record, credential) =
            issue_principal(actor_id, enrollment, self.authority_domain_id.clone())?;
        let _submit_guard = self.state.submit_guard().await;
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
        let issuer = MetadataIssuerContext::from_request(
            &request,
            self.authority_domain_id.clone(),
            &self.state,
        )
        .await?;
        let request = request.into_inner();
        let kind = patchbay_contracts::patchbay::AuditEventKind::try_from(request.kind)
            .map_err(|_| Status::invalid_argument("unknown control-surface audit kind"))?;
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
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("query diagnostics request is missing operation"))?;
        let authority_domain_id = operation
            .authority_domain_id
            .clone()
            .ok_or_else(|| Status::invalid_argument("query operation is missing authority domain"))?;
        self.require_configured_domain(&authority_domain_id)?;
        let issuer = MetadataIssuerContext::from_request(
            &request,
            authority_domain_id.clone(),
            &self.state,
        )
        .await?;
        let operation = request
            .into_inner()
            .operation
            .ok_or_else(|| Status::invalid_argument("query diagnostics request lost operation"))?;

        // Decode and validate the typed query before any persistence read. A
        // malformed query is a normal pre-acceptance rejection, not a gRPC
        // framing error and not an opportunity to touch the log.
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

        let _submit_guard = self.state.submit_guard().await;
        self.state
            .catch_up(&self.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;

        let submission = match acceptance::submit_with_clock(
            &self.storage,
            self.state.grant_check(),
            &AuthorityDomainTargetResolver,
            self.state.state_lookup(),
            self.state.elicitation_contract_lookup(),
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

impl<S> ControlServiceImpl<S> {
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

async fn find_delivered_checkpoint<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    command_id: &patchbay_contracts::patchbay::CommandId,
) -> Result<Option<EventId>, Status> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await
        .map_err(map_storage_error_to_status)?;
    let mut transition_checkpoint = None;
    let mut audit_checkpoint = None;
    for event in events {
        match StoredEventKind::try_from(event.payload.kind).ok() {
            Some(StoredEventKind::CommandTransition) => {
                let transition = CommandTransition::decode(event.payload.payload.as_slice())
                    .map_err(|error| Status::internal(format!("cannot decode query transition: {error}")))?;
                if transition.command_id.as_ref() == Some(command_id)
                    && OperationState::try_from(transition.to_state).ok() == Some(OperationState::Delivered)
                {
                    transition_checkpoint = Some(event.event_id);
                }
            }
            Some(StoredEventKind::AuditRecord) => {
                let record = AuditRecord::decode(event.payload.payload.as_slice())
                    .map_err(|error| Status::internal(format!("cannot decode query audit: {error}")))?;
                if record.source_event_id == transition_checkpoint {
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
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await
        .map_err(map_storage_error_to_status)?;
    for event in events {
        if StoredEventKind::try_from(event.payload.kind).ok() != Some(StoredEventKind::Observation) {
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
        let result = DiagnosticsResult::decode(payload.payload.as_slice())
            .map_err(|error| Status::internal(format!("cannot decode diagnostics result: {error}")))?;
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

fn operator_facing_subscribe_event(event: RecordedEvent) -> Option<SubscribeEvent> {
    let kind = StoredEventKind::try_from(event.payload.kind).ok()?;
    matches!(
        kind,
        StoredEventKind::Operation
            | StoredEventKind::Observation
            | StoredEventKind::Elicitation
            | StoredEventKind::SessionState
            | StoredEventKind::CommandTransition
    )
    .then_some(SubscribeEvent {
        event_id: Some(event.event_id),
        payload: Some(event.payload),
    })
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
