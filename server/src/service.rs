use std::{pin::Pin, sync::Arc, time::Duration};

use patchbay_contracts::patchbay::{
    ActorId, AuditEventKind, AuthorityDomainId, EnrollControlSurfacePrincipalRequest,
    EnrollControlSurfacePrincipalResult, EventId, LoadSnapshotRequest, LoadSnapshotResponse, Lsn,
    QueryDiagnosticsRequest, QueryDiagnosticsResponse, RevokeOperatorSessionRequest,
    RevokeOperatorSessionResult, StoredEventKind, SubmissionOutcome, SubmissionResult, SubmitRequest,
    SubscribeEvent, SubscribeRequest,
    VerifyOperatorPasswordRequest, VerifyOperatorPasswordResult,
};
use patchbay_core::{
    acceptance::{self, AcceptanceError},
    audit::{AuditSink, DurableAuditSink, RequiredAuditFanout, StderrAuditSink},
    authority::{IssuerContext, OperatorError},
    storage::{AuditRecordDraft, RecordedEvent, Storage, StorageError},
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
        let result = acceptance::submit(
            &self.storage,
            self.state.grant_check(),
            self.state.target_resolver(),
            self.state.state_lookup(),
            self.state.elicitation_contract_lookup(),
            &issuer,
            operation,
        )
        .await
        .map_err(map_acceptance_error_to_status)?;
        if result.outcome == SubmissionOutcome::Accepted as i32 && !result.deduplicated {
            self.state
                .catch_up(&self.storage, &authority_domain_id)
                .await
                .map_err(map_storage_error_to_status)?;
        }

        Ok(Response::new(result))
    }

    type SubscribeStream = SubscribeStream;

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

    async fn query_diagnostics(
        &self,
        _request: Request<QueryDiagnosticsRequest>,
    ) -> Result<Response<QueryDiagnosticsResponse>, Status> {
        Err(Status::unimplemented("diagnostics query surface is not wired yet"))
    }
}

impl<S> ControlServiceImpl<S> {
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
