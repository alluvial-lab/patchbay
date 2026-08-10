use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use patchbay_contracts::patchbay::{
    ActorEndpointRef, ActorId, BootstrapRequest, BootstrapResult, Grant, GrantId, GrantProvenance,
    GrantRevocationPolicy, OperatorRecord, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::COMMITTED_OPERATION_KINDS,
    authority::{validate_operator_record, AuthorityError},
    storage::{CoreGenerationStore, Storage},
    security,
};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{
    identity::{issue_principal, now_timestamp, random_token},
    rpc::admin_service_server::AdminService,
    service::{map_operator_error_to_status, map_storage_error_to_status, ControlServiceImpl},
};

#[derive(Clone)]
pub struct SetupSecret {
    state: Arc<Mutex<SetupSecretState>>,
}

struct SetupSecretState {
    expected: Vec<u8>,
    expires_at: Instant,
    consumed: bool,
}

impl SetupSecret {
    #[must_use]
    pub fn generate(ttl: Duration) -> (Self, String) {
        let secret = random_token();
        (Self::new(secret.clone(), ttl), secret)
    }

    #[must_use]
    pub fn new(secret: String, ttl: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(SetupSecretState {
                expected: secret.into_bytes(),
                expires_at: Instant::now() + ttl,
                consumed: false,
            })),
        }
    }
}

#[derive(Clone)]
pub struct AdminServiceImpl<S> {
    control: ControlServiceImpl<S>,
    setup_secret: SetupSecret,
}

impl<S> AdminServiceImpl<S> {
    #[must_use]
    pub fn new(control: ControlServiceImpl<S>, setup_secret: SetupSecret) -> Self {
        Self {
            control,
            setup_secret,
        }
    }
}

#[tonic::async_trait]
impl<S> AdminService for AdminServiceImpl<S>
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    async fn bootstrap_operator(
        &self,
        request: Request<BootstrapRequest>,
    ) -> Result<Response<BootstrapResult>, Status> {
        let request = request.into_inner();
        let mut setup = self.setup_secret.state.lock().await;
        if let Err(error) = authorize_setup_secret(&setup, &request.setup_secret) {
            let kind = if error.message().contains("expired") {
                patchbay_contracts::patchbay::AuditEventKind::BootstrapExpired
            } else {
                patchbay_contracts::patchbay::AuditEventKind::BootstrapStarted
            };
            let mut draft = patchbay_core::storage::AuditRecordDraft::new(
                now_timestamp()?,
                kind,
            );
            draft.reason_code = if kind == patchbay_contracts::patchbay::AuditEventKind::BootstrapExpired {
                "bootstrap_expired"
            } else {
                "bootstrap_setup_secret_rejected"
            }
            .to_owned();
            self.control.record_audit(draft).await?;
            return Err(error);
        }
        if self.control.state.operator_exists().await {
            return Err(Status::already_exists(
                "operator bootstrap has already completed",
            ));
        }

        let actor_id = required_actor(request.operator_actor_id)?;
        let mut started = patchbay_core::storage::AuditRecordDraft::new(
            now_timestamp()?,
            patchbay_contracts::patchbay::AuditEventKind::BootstrapStarted,
        );
        started.actor_id = Some(actor_id.clone());
        started.reason_code = "bootstrap_started".to_owned();
        self.control.record_audit(started).await?;
        let enrollment = request
            .principal
            .ok_or_else(|| Status::invalid_argument("principal enrollment is required"))?;
        let created_at = now_timestamp()?;
        // Validate and materialize the principal before the first durable write
        // so malformed enrollment cannot leave a completed operator bootstrap
        // without returning usable credentials.
        let (principal_record, principal_credential) = issue_principal(
            actor_id.clone(),
            enrollment.clone(),
            self.control.authority_domain_id.clone(),
        )?;
        let operator = OperatorRecord {
            actor_id: Some(actor_id.clone()),
            password_hash: request.password_hash,
            created_at: Some(created_at),
            authority_domain_id: Some(self.control.authority_domain_id.clone()),
        };
        validate_operator_record(&operator, &self.control.authority_domain_id)
            .map_err(map_operator_error_to_status)?;
        let grant_id = GrantId {
            value: format!("bootstrap-operator-{}", actor_id.value),
        };
        let grant = bootstrap_grant(
            grant_id.clone(),
            actor_id.clone(),
            &enrollment,
            created_at,
            self.control.authority_domain_id.clone(),
        );

        // One gate serializes bootstrap against submissions and routine
        // enrollment. The grant uses a deterministic id so a retry after a
        // partial storage failure can reuse an already-committed grant.
        let _submit_guard = self.control.state.submit_guard().await;
        if let Some(existing) = self.control.state.grant(&grant_id).await {
            let expected_kinds = COMMITTED_OPERATION_KINDS.to_vec();
            if existing.subject_actor_id != actor_id
                || existing.authority_domain_id != self.control.authority_domain_id
                || existing.allowed_operation_kinds != expected_kinds
                || existing.target_scope.kind != TargetScopeKind::AuthorityDomain as i32
                || !existing.is_live()
            {
                return Err(Status::internal(
                    "bootstrap grant id conflicts with another authority record",
                ));
            }
        } else {
            self.control
                .state
                .ingest_grant(
                    &self.control.storage,
                    &self.control.authority_domain_id,
                    grant,
                )
                .await
                .map_err(map_authority_error_to_status)?;
        }

        self.control
            .state
            .ingest_operator(
                &self.control.storage,
                &self.control.authority_domain_id,
                operator,
            )
            .await
            .map_err(map_operator_error_to_status)?;

        self.control
            .state
            .ingest_principal(
                &self.control.storage,
                &self.control.authority_domain_id,
                principal_record,
            )
            .await
            .map_err(map_operator_error_to_status)?;
        self.control
            .state
            .catch_up(&self.control.storage, &self.control.authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;

        let session_binding = crate::operator_session::OperatorSessionBinding {
            actor_id: actor_id.clone(),
            endpoint_id: principal_credential
                .endpoint_id
                .clone()
                .ok_or_else(|| Status::internal("issued principal has no endpoint"))?,
            device_id: principal_credential
                .device_id
                .clone()
                .ok_or_else(|| Status::internal("issued principal has no device"))?,
            endpoint_generation: principal_credential
                .endpoint_generation
                .ok_or_else(|| Status::internal("issued principal has no endpoint generation"))?,
        };
        let session = self.control.state.issue_operator_session(session_binding).await;
        let mut session_audit = patchbay_core::storage::AuditRecordDraft::new(
            now_timestamp()?,
            patchbay_contracts::patchbay::AuditEventKind::OperatorSessionCreated,
        );
        session_audit.actor_id = Some(actor_id);
        session_audit.reason_code = "operator_session_created".to_owned();
        self.control.record_audit(session_audit).await?;
        setup.consumed = true;
        Ok(Response::new(BootstrapResult {
            grant_id: Some(grant_id),
            session_id: Some(session.id),
            principal: Some(principal_credential),
            operator_session_generation: Some(session.session_generation),
        }))
    }

    async fn exit_security_lockdown(
        &self,
        request: Request<patchbay_contracts::patchbay::ExitSecurityLockdownRequest>,
    ) -> Result<Response<patchbay_contracts::patchbay::ExitSecurityLockdownResult>, Status> {
        let request = request.into_inner();
        let authority_domain_id = request
            .authority_domain_id
            .ok_or_else(|| Status::invalid_argument("authority_domain_id is required"))?;
        if authority_domain_id.value.is_empty() {
            return Err(Status::invalid_argument("authority_domain_id must not be empty"));
        }
        self.control.require_configured_domain(&authority_domain_id)?;
        if !request.reason_code.is_empty()
            && (request.reason_code.len() > 64
                || !request.reason_code.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                }))
        {
            return Err(Status::invalid_argument("reason_code must match [a-z0-9_]{1,64}"));
        }

        // This is the bootstrap trust boundary: no operator credential or
        // routine ControlService issuer is read here. The admin listener's
        // loopback-only binding is the configured authorization boundary.
        let _decision_guard = self.control.state.submit_guard().await;
        self.control
            .state
            .catch_up(&self.control.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let current = self.control.state.lockdown_state().await;
        if !current.active {
            let reason_code = if request.reason_code.is_empty() {
                "bootstrap_exit_already_inactive".to_owned()
            } else {
                request.reason_code.clone()
            };
            let mut audit = patchbay_core::storage::AuditRecordDraft::new(
                now_timestamp()?,
                patchbay_contracts::patchbay::AuditEventKind::LockdownExited,
            );
            audit.target_scope = Some(patchbay_contracts::patchbay::TargetScope {
                kind: patchbay_contracts::patchbay::TargetScopeKind::AuthorityDomain as i32,
                ..patchbay_contracts::patchbay::TargetScope::default()
            });
            audit.reason_code = reason_code;
            self.control.record_audit(audit).await?;
            return Ok(Response::new(
                patchbay_contracts::patchbay::ExitSecurityLockdownResult {
                    lockdown: Some(current),
                    lockdown_event_id: None,
                    already_inactive: true,
                    entered_event_id: None,
                },
            ));
        }
        let entered_event_id = current
            .entered_event_id
            .clone()
            .ok_or_else(|| Status::internal("active lockdown has no entry event"))?;
        let reason_code = if request.reason_code.is_empty() {
            "bootstrap_exit".to_owned()
        } else {
            request.reason_code
        };
        let occurred_at = now_timestamp()?;
        let source = security::events::exited(
            authority_domain_id.clone(),
            patchbay_contracts::patchbay::SecurityLockdownExited {
                reason_code: reason_code.clone(),
                occurred_at: Some(occurred_at),
                entered_event_id: Some(entered_event_id.clone()),
                bootstrap_channel: patchbay_contracts::patchbay::BootstrapChannelKind::LoopbackAdmin as i32,
            },
        );
        let mut audit = patchbay_core::storage::AuditRecordDraft::new(
            occurred_at,
            patchbay_contracts::patchbay::AuditEventKind::LockdownExited,
        );
        audit.target_scope = Some(patchbay_contracts::patchbay::TargetScope {
            kind: patchbay_contracts::patchbay::TargetScopeKind::AuthorityDomain as i32,
            ..patchbay_contracts::patchbay::TargetScope::default()
        });
        audit.reason_code = reason_code;
        let event_id = self
            .control
            .storage
            .append_decision(&authority_domain_id, security::events::encode(&source), audit)
            .await
            .map_err(map_storage_error_to_status)?;
        self.control
            .state
            .catch_up(&self.control.storage, &authority_domain_id)
            .await
            .map_err(map_storage_error_to_status)?;
        let lockdown = self.control.state.lockdown_state().await;
        if lockdown.active {
            return Err(Status::internal("committed lockdown exit did not clear posture"));
        }
        Ok(Response::new(
            patchbay_contracts::patchbay::ExitSecurityLockdownResult {
                lockdown: Some(lockdown),
                lockdown_event_id: Some(event_id),
                already_inactive: false,
                entered_event_id: Some(entered_event_id),
            },
        ))
    }
}

fn authorize_setup_secret(state: &SetupSecretState, supplied: &str) -> Result<(), Status> {
    if state.consumed {
        return Err(Status::failed_precondition(
            "the one-time setup secret has already been consumed",
        ));
    }
    if Instant::now() >= state.expires_at {
        return Err(Status::failed_precondition("the setup secret has expired"));
    }
    if !constant_time_eq(supplied.as_bytes(), &state.expected) {
        return Err(Status::permission_denied("invalid setup secret"));
    }
    Ok(())
}

fn required_actor(actor: Option<ActorId>) -> Result<ActorId, Status> {
    let actor = actor.ok_or_else(|| Status::invalid_argument("operator actor id is required"))?;
    if actor.value.is_empty() {
        return Err(Status::invalid_argument(
            "operator actor id must not be empty",
        ));
    }
    Ok(actor)
}

fn bootstrap_grant(
    grant_id: GrantId,
    actor_id: ActorId,
    enrollment: &patchbay_contracts::patchbay::PrincipalEnrollment,
    created_at: prost_types::Timestamp,
    authority_domain_id: patchbay_contracts::patchbay::AuthorityDomainId,
) -> Grant {
    Grant {
        grant_id: Some(grant_id),
        authority_domain_id: Some(authority_domain_id),
        subject_actor_id: Some(actor_id.clone()),
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::AuthorityDomain as i32,
            ..TargetScope::default()
        }),
        allowed_operation_kinds: COMMITTED_OPERATION_KINDS
            .iter()
            .map(|kind| *kind as i32)
            .collect(),
        created_at: Some(created_at),
        provenance: Some(GrantProvenance {
            created_by: Some(ActorEndpointRef {
                actor_id: Some(actor_id),
                endpoint_id: enrollment.endpoint_id.clone(),
                device_id: enrollment.device_id.clone(),
                endpoint_generation: enrollment.endpoint_generation,
            }),
            reason: "local-console operator bootstrap".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    }
}

fn map_authority_error_to_status(error: AuthorityError) -> Status {
    match error {
        AuthorityError::InvalidGrant(message) => Status::invalid_argument(message),
        AuthorityError::GrantNotFound(message) => Status::failed_precondition(message),
        AuthorityError::CorruptRecord(message) | AuthorityError::CorruptLog(message) => {
            Status::internal(message)
        }
        AuthorityError::Storage(error) => map_storage_error_to_status(error),
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
