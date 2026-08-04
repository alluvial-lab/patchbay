use std::{sync::Arc, time::Duration};

use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, CommandId, ControlSurfacePrincipalRecord, ElicitationId, EventId,
    Grant, GrantId, Lsn, OperationKind, OperatorRecord, Session, SessionSnapshot, TargetScope,
    Generation, OperatorSessionRevocation, ControlSurfaceRevocation,
    TargetScopeKind, ViewRevision,
};
use patchbay_core::{
    acceptance::{
        ActiveElicitation, Authorized, CommandIndex, CommandRecord, CommandSnapshot, CommandStateLookup,
        ElicitationContractLookup, ElicitationSlotLayer, GrantCheck, GrantDenied, OperationPosture,
        OperationPostureDenied, TargetBinding,
        TargetNotFound, TargetResolver,
    },
    authority::{
        ingest_control_surface_principal, ingest_control_surface_revocation,
        ingest_grant as ingest_authority_grant, ingest_operator_record,
        ingest_operator_session_revocation, ingest_revocation as ingest_authority_revocation,
        AuthorityError, AuthorityRegistry, ControlSurfaceRevocationTarget, GrantRecord,
        IssuerContext, OperatorError, OperatorRegistry, RevocationIngestResult,
    },
    diagnostics::DiagnosticsProjection,
    security::SecurityPostureProjection,
    resource::ResourceRegistry,
    session::SessionRegistry,
    storage::{RecordedEvent, Storage, StorageError},
    target::TargetRegistry,
};
use tokio::sync::{Mutex, MutexGuard};

use crate::{
    decision_gate::CoreDecisionGate,
    operator_session::{OperatorSessionRegistry, DEFAULT_OPERATOR_SESSION_TTL},
};

/// Server-owned concurrency boundary around core projections.
///
/// The canonical acquisition order is storage -> grant check -> target
/// resolver -> command-state lookup, matching the parameter order at the
/// acceptance boundary. Projection locks are short-lived and never nested in
/// this implementation: each port releases its lock before the next port is
/// called. `submit_guard` serializes submission plus projection catch-up, and is
/// backed by the composition-root `CoreDecisionGate` shared with adapter
/// transitions. This can be replaced by a server-local actor without changing
/// the core library or the wire contract.
#[derive(Clone)]
pub struct ProjectionState {
    grant_check: LockedGrantCheck,
    target_resolver: LockedTargetResolver,
    state_lookup: LockedCommandStateLookup,
    elicitation_slots: LockedElicitationContractLookup,
    diagnostics: Arc<Mutex<DiagnosticsProjection>>,
    security_posture: LockedSecurityPosture,
    operators: Arc<Mutex<OperatorRegistry>>,
    pub(crate) operator_sessions: OperatorSessionRegistry,
    last_applied_lsn: Arc<Mutex<u64>>,
    decision_gate: CoreDecisionGate,
}

impl ProjectionState {
    pub async fn rebuild<S: Storage>(
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
    ) -> Result<Self, String> {
        Self::rebuild_with_session_ttl(storage, authority_domain_id, DEFAULT_OPERATOR_SESSION_TTL)
            .await
    }

    pub async fn rebuild_with_session_ttl<S: Storage>(
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        operator_session_ttl: Duration,
    ) -> Result<Self, String> {
        Self::rebuild_with_session_ttl_and_gate(
            storage,
            authority_domain_id,
            operator_session_ttl,
            CoreDecisionGate::default(),
        )
        .await
    }

    pub async fn rebuild_with_session_ttl_and_gate<S: Storage>(
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        operator_session_ttl: Duration,
        decision_gate: CoreDecisionGate,
    ) -> Result<Self, String> {
        let events = storage
            .read_after(authority_domain_id, Lsn { value: 0 })
            .await
            .map_err(|error| error.to_string())?;

        let mut authority = AuthorityRegistry::new();
        let mut sessions = SessionRegistry::new();
        let mut commands = CommandIndex::new();
        let mut elicitation_slots = ElicitationSlotLayer::new();
        let mut diagnostics = DiagnosticsProjection::new();
        let mut security_posture = SecurityPostureProjection::new();
        let mut operators = OperatorRegistry::new();
        let operator_sessions = OperatorSessionRegistry::new(operator_session_ttl)?;
        let mut last_applied_lsn = 0;
        for event in &events {
            last_applied_lsn = validate_next_event(event, authority_domain_id, last_applied_lsn)?;
            authority
                .observe(event)
                .map_err(|error| error.to_string())?;
            sessions.observe(event).map_err(|error| error.to_string())?;
            commands.apply(event).map_err(|error| error.to_string())?;
            elicitation_slots
                .observe(event)
                .map_err(|error| error.to_string())?;
            diagnostics
                .observe(event)
                .map_err(|error| error.to_string())?;
            security_posture
                .observe(event)
                .map_err(|error| error.to_string())?;
            operators
                .observe(event)
                .map_err(|error| error.to_string())?;
            operator_sessions
                .observe(event)
                .await
                .map_err(|error| error.to_string())?;
        }
        diagnostics.reset_adapter_liveness();

        Ok(Self {
            grant_check: LockedGrantCheck::new(authority),
            target_resolver: LockedTargetResolver::new(TargetRegistry::new(
                sessions,
                ResourceRegistry::new(),
            )),
            state_lookup: LockedCommandStateLookup::new(commands),
            elicitation_slots: LockedElicitationContractLookup::from_layer(elicitation_slots),
            diagnostics: Arc::new(Mutex::new(diagnostics)),
            security_posture: LockedSecurityPosture::new(security_posture),
            operators: Arc::new(Mutex::new(operators)),
            operator_sessions,
            last_applied_lsn: Arc::new(Mutex::new(last_applied_lsn)),
            decision_gate,
        })
    }

    #[must_use]
    pub fn grant_check(&self) -> &LockedGrantCheck {
        &self.grant_check
    }

    #[must_use]
    pub fn target_resolver(&self) -> &LockedTargetResolver {
        &self.target_resolver
    }

    #[must_use]
    pub fn state_lookup(&self) -> &LockedCommandStateLookup {
        &self.state_lookup
    }

    #[must_use]
    pub fn elicitation_contract_lookup(&self) -> &LockedElicitationContractLookup {
        &self.elicitation_slots
    }

    #[must_use]
    pub fn operation_posture(&self) -> &LockedSecurityPosture {
        &self.security_posture
    }

    pub async fn lockdown_state(&self) -> patchbay_contracts::patchbay::SecurityLockdownState {
        self.security_posture.state().await
    }

    pub async fn diagnostics_command_result(
        &self,
        command_id: &CommandId,
    ) -> Option<patchbay_contracts::patchbay::CommandInspectionResult> {
        self.diagnostics.lock().await.result_for_query(command_id)
    }

    /// Rebuild diagnostics from the one durable prefix selected by the query.
    /// This prevents interleaved later events from leaking into a result.
    pub async fn diagnostics_at<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        as_of_lsn: u64,
    ) -> Result<DiagnosticsProjection, StorageError> {
        let live_adapters: Vec<_> = self.diagnostics.lock().await.live_adapter_ids().cloned().collect();
        let events = storage
            .read_through(authority_domain_id, Lsn { value: 0 }, Lsn { value: as_of_lsn })
            .await?;
        let mut projection = DiagnosticsProjection::new();
        for event in events {
            projection
                .observe(&event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        }
        // Historical registration is not proof of current liveness. The
        // caller's live projection is the only source for fresh attachment
        // evidence; keep UNKNOWN for a rebuilt prefix after restart.
        projection.reset_adapter_liveness();
        for adapter_id in live_adapters {
            let attach_lsn = projection
                .adapter_page(
                    &patchbay_contracts::patchbay::AdapterStatusQuery::default(),
                    as_of_lsn,
                )
                .ok()
                .and_then(|page| page.adapters.into_iter().find(|status| status.adapter_id.as_ref() == Some(&adapter_id)))
                .and_then(|status| status.attach_event_id)
                .and_then(|event_id| event_id.lsn)
                .map(|lsn| lsn.value);
            if attach_lsn.is_some_and(|lsn| lsn <= as_of_lsn) {
                projection.mark_adapter_live(adapter_id);
            }
        }
        Ok(projection)
    }

    pub async fn diagnostics_adapter_page(
        &self,
        query: &patchbay_contracts::patchbay::AdapterStatusQuery,
        as_of: u64,
    ) -> Result<patchbay_contracts::patchbay::AdapterStatusPage, patchbay_core::diagnostics::DiagnosticsError> {
        self.diagnostics.lock().await.adapter_page(query, as_of)
    }

    pub async fn current_lsn(&self) -> u64 {
        *self.last_applied_lsn.lock().await
    }

    pub async fn current_runtime_session_count(&self) -> u32 {
        self.target_resolver.inner.lock().await.sessions().sessions().count() as u32
    }

    pub async fn submit_guard(&self) -> MutexGuard<'_, ()> {
        self.decision_gate.acquire().await
    }

    /// Materialize the authoritative live-session projection at its applied LSN.
    ///
    /// The cursor lock is acquired before the session lock, matching
    /// `catch_up`, so the returned records and `snapshot_lsn` describe one
    /// consistent projection prefix. Durable snapshot checkpointing remains a
    /// separate, deferred concern.
    pub async fn materialize_session_snapshot(
        &self,
        authority_domain_id: AuthorityDomainId,
        materialized_at: prost_types::Timestamp,
    ) -> SessionSnapshot {
        let cursor = self.last_applied_lsn.lock().await;
        let registry = self.target_resolver.inner.lock().await;
        let mut sessions: Vec<_> = registry.sessions().sessions().cloned().collect();
        sessions.sort_by(|left, right| {
            (
                &left.identity.adapter_id.value,
                &left.identity.deployment_scope,
                &left.identity.runtime_session_id.value,
                left.identity.session_generation.value,
            )
                .cmp(&(
                    &right.identity.adapter_id.value,
                    &right.identity.deployment_scope,
                    &right.identity.runtime_session_id.value,
                    right.identity.session_generation.value,
                ))
        });
        let sessions: Vec<Session> = sessions
            .into_iter()
            .map(|record| Session {
                authority_domain_id: Some(authority_domain_id.clone()),
                adapter_id: Some(record.identity.adapter_id),
                deployment_scope: record.identity.deployment_scope,
                runtime_session_id: Some(record.identity.runtime_session_id),
                session_generation: Some(record.identity.session_generation),
                project: record.project,
                cwd: record.cwd,
                name: record.name,
                state: Some(record.state),
                last_authoritative_lsn: record.last_authoritative_lsn.map(|value| Lsn { value }),
                observed_at: None,
                tombstoned: record.tombstoned,
                superseded_at_lsn: record.superseded_at_lsn.map(|value| Lsn { value }),
                model: record.model,
            })
            .collect();
        let view_revisions = sessions
            .iter()
            .map(|session| ViewRevision {
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::RuntimeSession as i32,
                    adapter_id: session.adapter_id.clone(),
                    deployment_scope: session.deployment_scope.clone(),
                    runtime_session_id: session.runtime_session_id.clone(),
                    session_generation: session.session_generation,
                    ..TargetScope::default()
                }),
                revision_lsn: session.last_authoritative_lsn,
            })
            .collect();

        let lockdown = self.security_posture.state().await;
        SessionSnapshot {
            authority_domain_id: Some(authority_domain_id),
            snapshot_lsn: Some(Lsn { value: *cursor }),
            // Core-generation persistence is a reserved seam in the current
            // executable slice; do not fabricate one on this read path.
            core_generation: None,
            sessions,
            view_revisions,
            materialized_at: Some(materialized_at),
            lockdown: Some(lockdown),
        }
    }

    pub async fn materialize_security_snapshot(
        &self,
        authority_domain_id: AuthorityDomainId,
    ) -> patchbay_contracts::patchbay::SecuritySnapshot {
        let snapshot_lsn = self.current_lsn().await;
        let lockdown = self.security_posture.state().await;
        let operator_sessions = self.operator_sessions.summaries().await;
        let (control_surfaces, grants) = {
            let operators = self.operators.lock().await;
            let control_surfaces = operators
                .principals()
                .map(|principal| patchbay_contracts::patchbay::ControlSurfaceSummary {
                    principal_id: principal.principal_id.clone(),
                    endpoint_id: principal.endpoint_id.clone(),
                    device_id: principal.device_id.clone(),
                    endpoint_generation: principal.endpoint_generation,
                    revoked: operators.is_principal_revoked(&principal.principal_id),
                })
                .collect();
            let grants = self
                .grant_check
                .inner
                .lock()
                .await
                .grants()
                .map(|grant| patchbay_contracts::patchbay::GrantSummary {
                    grant_id: Some(grant.grant_id.clone()),
                    subject_actor_id: Some(grant.subject_actor_id.clone()),
                    target_scope: Some(grant.target_scope.clone()),
                    allowed_operation_kinds: grant
                        .allowed_operation_kinds
                        .iter()
                        .map(|kind| *kind as i32)
                        .collect(),
                    expires_at: grant.expires_at,
                    revoked: grant.is_revoked(),
                    revocation_policy: grant.revocation_policy as i32,
                })
                .collect();
            (control_surfaces, grants)
        };
        patchbay_contracts::patchbay::SecuritySnapshot {
            authority_domain_id: Some(authority_domain_id),
            snapshot_lsn: Some(Lsn { value: snapshot_lsn }),
            lockdown: Some(lockdown),
            operator_sessions,
            control_surfaces,
            grants,
        }
    }

    /// Fold newly committed events into every server-owned projection.
    pub async fn catch_up<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
    ) -> Result<(), StorageError> {
        let mut cursor = self.last_applied_lsn.lock().await;
        let events = storage
            .read_after(authority_domain_id, Lsn { value: *cursor })
            .await?;

        for event in events {
            let next_lsn = validate_next_event(&event, authority_domain_id, *cursor)
                .map_err(StorageError::CorruptRecord)?;
            self.grant_check
                .observe(&event)
                .await
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            self.target_resolver
                .observe(&event)
                .await
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            self.state_lookup
                .apply(&event)
                .await
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            self.elicitation_slots
                .observe(&event)
                .await
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            self.diagnostics
                .lock()
                .await
                .observe(&event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            self.security_posture
                .observe(&event)
                .await
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            self.operators
                .lock()
                .await
                .observe(&event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            if event.payload.kind == patchbay_contracts::patchbay::StoredEventKind::ControlSurfaceRevocation as i32 {
                let revocation: ControlSurfaceRevocation = prost::Message::decode(event.payload.payload.as_slice())
                    .map_err(|error| StorageError::CorruptRecord(format!("cannot decode control-surface revocation: {error}")))?;
                let target = match revocation.target.ok_or_else(|| StorageError::CorruptRecord("control-surface revocation has no target".to_owned()))? {
                    patchbay_contracts::patchbay::control_surface_revocation::Target::PrincipalId(id) => ControlSurfaceRevocationTarget::Principal(id),
                    patchbay_contracts::patchbay::control_surface_revocation::Target::EndpointId(id) => ControlSurfaceRevocationTarget::Endpoint(id),
                    patchbay_contracts::patchbay::control_surface_revocation::Target::DeviceId(id) => ControlSurfaceRevocationTarget::Device(id),
                };
                self.revoke_sessions_for_target(&target).await;
            }
            self.operator_sessions
                .observe(&event)
                .await
                .map_err(StorageError::CorruptRecord)?;
            *cursor = next_lsn;
        }
        Ok(())
    }

    pub async fn operator_exists(&self) -> bool {
        self.operators.lock().await.operator_record().is_some()
    }

    pub async fn principal_record(
        &self,
        principal_id: &str,
    ) -> Option<ControlSurfacePrincipalRecord> {
        self.operators
            .lock()
            .await
            .principal_record(principal_id)
            .cloned()
    }

    pub async fn has_endpoint(&self, endpoint_id: &patchbay_contracts::patchbay::EndpointId) -> bool {
        self.operators.lock().await.has_endpoint(endpoint_id)
    }

    pub async fn has_device(&self, device_id: &patchbay_contracts::patchbay::DeviceId) -> bool {
        self.operators.lock().await.has_device(device_id)
    }

    pub async fn count_matching_revocation_target(
        &self,
        target: &ControlSurfaceRevocationTarget,
    ) -> u32 {
        self.operators.lock().await.count_matching(target)
    }

    pub async fn verify_password(
        &self,
        actor_id: &ActorId,
        password: &str,
    ) -> Result<bool, OperatorError> {
        self.operators
            .lock()
            .await
            .verify_password(actor_id, password)
    }

    pub async fn verify_principal(
        &self,
        principal_id: &str,
        credential: &str,
    ) -> Option<ControlSurfacePrincipalRecord> {
        self.operators
            .lock()
            .await
            .verify_principal(principal_id, credential)
    }

    pub async fn issue_operator_session(
        &self,
        binding: crate::operator_session::OperatorSessionBinding,
    ) -> crate::operator_session::IssuedOperatorSession {
        self.operator_sessions.issue(binding).await
    }

    pub async fn verify_operator_session(
        &self,
        session_id: &patchbay_contracts::patchbay::OperatorSessionId,
        binding: &crate::operator_session::OperatorSessionBinding,
    ) -> bool {
        self.operator_sessions.verify(session_id, binding).await
    }

    pub async fn revoke_operator_session(
        &self,
        session_id: &patchbay_contracts::patchbay::OperatorSessionId,
        binding: &crate::operator_session::OperatorSessionBinding,
    ) -> bool {
        self.operator_sessions.revoke_current(session_id, binding).await
    }

    pub async fn current_operator_session_generation(&self, actor_id: &ActorId) -> Generation {
        self.operator_sessions.current_generation(actor_id).await
    }

    pub async fn revoke_all_operator_sessions(
        &self,
        actor_id: &ActorId,
        through: &Generation,
    ) -> u32 {
        self.operator_sessions.revoke_all_for_actor(actor_id, through).await
    }

    pub async fn revoke_sessions_for_target(
        &self,
        target: &ControlSurfaceRevocationTarget,
    ) -> u32 {
        match target {
            ControlSurfaceRevocationTarget::Principal(principal_id) => {
                let principal = self
                    .operators
                    .lock()
                    .await
                    .principal_record(principal_id)
                    .cloned();
                self.operator_sessions
                    .revoke_matching_principal(|binding| {
                        principal.as_ref().is_some_and(|record| {
                            record.operator_actor_id.as_ref() == Some(&binding.actor_id)
                                && record.endpoint_id.as_ref() == Some(&binding.endpoint_id)
                                && record.device_id.as_ref() == Some(&binding.device_id)
                                && record.endpoint_generation.as_ref() == Some(&binding.endpoint_generation)
                        })
                    })
                    .await
            }
            ControlSurfaceRevocationTarget::Endpoint(endpoint_id) => {
                self.operator_sessions
                    .revoke_matching_principal(|binding| &binding.endpoint_id == endpoint_id)
                    .await
            }
            ControlSurfaceRevocationTarget::Device(device_id) => {
                self.operator_sessions
                    .revoke_matching_principal(|binding| &binding.device_id == device_id)
                    .await
            }
        }
    }

    pub async fn ingest_operator_session_revocation<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        revocation: OperatorSessionRevocation,
    ) -> Result<(RevocationIngestResult, u32), OperatorError> {
        let actor = revocation
            .operator_actor_id
            .clone()
            .ok_or_else(|| OperatorError::InvalidRecord("session revocation has no actor".to_owned()))?;
        let generation = revocation
            .invalidated_through_generation
            .ok_or_else(|| OperatorError::InvalidRecord("session revocation has no generation".to_owned()))?;
        let result = ingest_operator_session_revocation(
            storage,
            &mut *self.operators.lock().await,
            authority_domain_id,
            revocation,
        )
        .await?;
        let revoked_session_count = self
            .operator_sessions
            .revoke_all_for_actor(&actor, &generation)
            .await;
        Ok((result, revoked_session_count))
    }

    pub async fn ingest_control_surface_revocation<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        revocation: ControlSurfaceRevocation,
    ) -> Result<(RevocationIngestResult, ControlSurfaceRevocationTarget, u32), OperatorError> {
        let result = ingest_control_surface_revocation(
            storage,
            &mut *self.operators.lock().await,
            authority_domain_id,
            revocation,
        )
        .await?;
        let revoked_session_count = self.revoke_sessions_for_target(&result.1).await;
        Ok((result.0, result.1, revoked_session_count))
    }

    pub async fn commands_for_grant(&self, grant_id: &patchbay_contracts::patchbay::GrantId) -> Vec<CommandRecord> {
        self.state_lookup.records_for_grant(grant_id).await
    }

    pub async fn grant(&self, grant_id: &GrantId) -> Option<GrantRecord> {
        self.grant_check
            .inner
            .lock()
            .await
            .get_grant(grant_id)
            .cloned()
    }

    pub async fn recovery_capable_authority_domain_grant_count(
        &self,
        now: &prost_types::Timestamp,
    ) -> usize {
        self.grant_check
            .inner
            .lock()
            .await
            .grants()
            .filter(|grant| {
                grant.is_live_at(now) && grant.is_recovery_capable_authority_domain()
            })
            .count()
    }

    pub async fn ingest_grant<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        grant: Grant,
    ) -> Result<EventId, AuthorityError> {
        ingest_authority_grant(
            storage,
            &mut *self.grant_check.inner.lock().await,
            authority_domain_id,
            grant,
        )
        .await
    }

    pub async fn ingest_revocation<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        revocation: patchbay_contracts::patchbay::Revocation,
    ) -> Result<EventId, AuthorityError> {
        ingest_authority_revocation(
            storage,
            &mut *self.grant_check.inner.lock().await,
            authority_domain_id,
            revocation,
        ).await
    }

    pub async fn ingest_operator<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        record: OperatorRecord,
    ) -> Result<EventId, OperatorError> {
        ingest_operator_record(
            storage,
            &mut *self.operators.lock().await,
            authority_domain_id,
            record,
        )
        .await
    }

    pub async fn ingest_principal<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        record: ControlSurfacePrincipalRecord,
    ) -> Result<EventId, OperatorError> {
        ingest_control_surface_principal(
            storage,
            &mut *self.operators.lock().await,
            authority_domain_id,
            record,
        )
        .await
    }
}

fn validate_next_event(
    event: &RecordedEvent,
    authority_domain_id: &AuthorityDomainId,
    previous_lsn: u64,
) -> Result<u64, String> {
    let domain = event
        .event_id
        .authority_domain_id
        .as_ref()
        .ok_or_else(|| "replay event has no authority domain".to_owned())?;
    if domain != authority_domain_id {
        return Err(format!(
            "replay event belongs to authority domain {:?}, expected {:?}",
            domain, authority_domain_id
        ));
    }
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| "replay event has no LSN".to_owned())?
        .value;
    if lsn <= previous_lsn {
        return Err(format!(
            "replay event LSN {lsn} is not after previous LSN {previous_lsn}"
        ));
    }
    Ok(lsn)
}

#[derive(Clone)]
pub struct LockedSecurityPosture {
    inner: Arc<Mutex<SecurityPostureProjection>>,
}

impl LockedSecurityPosture {
    fn new(projection: SecurityPostureProjection) -> Self {
        Self {
            inner: Arc::new(Mutex::new(projection)),
        }
    }

    async fn observe(&self, event: &RecordedEvent) -> Result<(), patchbay_core::security::SecurityError> {
        self.inner.lock().await.observe(event)
    }

    async fn state(&self) -> patchbay_contracts::patchbay::SecurityLockdownState {
        self.inner.lock().await.state()
    }
}

impl OperationPosture for LockedSecurityPosture {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
    ) -> Result<(), OperationPostureDenied> {
        let projection = self.inner.lock().await;
        OperationPosture::check(&*projection, authority_domain_id).await
    }
}

#[derive(Clone)]
pub struct LockedGrantCheck {
    inner: Arc<Mutex<AuthorityRegistry>>,
}

impl LockedGrantCheck {
    fn new(registry: AuthorityRegistry) -> Self {
        Self {
            inner: Arc::new(Mutex::new(registry)),
        }
    }

    async fn observe(
        &self,
        event: &RecordedEvent,
    ) -> Result<(), patchbay_core::authority::AuthorityError> {
        self.inner.lock().await.observe(event)
    }
}

impl GrantCheck for LockedGrantCheck {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        let registry = self.inner.lock().await;
        GrantCheck::check(&*registry, authority_domain_id, issuer, operation_kind, target_scope).await
    }

    async fn check_at(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
        evaluated_at: &prost_types::Timestamp,
    ) -> Result<Authorized, GrantDenied> {
        let registry = self.inner.lock().await;
        GrantCheck::check_at(&*registry, authority_domain_id, issuer, operation_kind, target_scope, evaluated_at).await
    }
}

#[derive(Clone)]
pub struct LockedTargetResolver {
    inner: Arc<Mutex<TargetRegistry>>,
}

impl LockedTargetResolver {
    fn new(registry: TargetRegistry) -> Self {
        Self {
            inner: Arc::new(Mutex::new(registry)),
        }
    }

    async fn observe(
        &self,
        event: &RecordedEvent,
    ) -> Result<(), patchbay_core::session::SessionError> {
        self.inner.lock().await.observe_session_event(event)
    }
}

impl TargetResolver for LockedTargetResolver {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> Result<TargetBinding, TargetNotFound> {
        let registry = self.inner.lock().await;
        TargetResolver::resolve(&*registry, authority_domain_id, target_scope).await
    }
}

#[derive(Clone, Default)]
pub struct LockedElicitationContractLookup {
    inner: Arc<Mutex<ElicitationSlotLayer>>,
}

impl LockedElicitationContractLookup {
    pub fn new() -> Self {
        Self::from_layer(ElicitationSlotLayer::new())
    }

    fn from_layer(layer: ElicitationSlotLayer) -> Self {
        Self {
            inner: Arc::new(Mutex::new(layer)),
        }
    }

    async fn observe(
        &self,
        event: &RecordedEvent,
    ) -> Result<(), patchbay_core::acceptance::AcceptanceError> {
        self.inner.lock().await.observe(event)
    }
}

impl ElicitationContractLookup for LockedElicitationContractLookup {
    async fn active_contract(&self, elicitation_id: &ElicitationId) -> Option<ActiveElicitation> {
        let layer = self.inner.lock().await;
        let record = layer.get_slot(elicitation_id)?;
        Some(ActiveElicitation {
            contract: record.contract.clone()?,
            is_terminal: patchbay_core::acceptance::elicitation::is_terminal_state(record.state),
            winning_response: record.winning_response.clone(),
        })
    }
}

#[derive(Clone)]
pub struct LockedCommandStateLookup {
    inner: Arc<Mutex<CommandIndex>>,
}

impl LockedCommandStateLookup {
    fn new(index: CommandIndex) -> Self {
        Self {
            inner: Arc::new(Mutex::new(index)),
        }
    }

    async fn apply(
        &self,
        event: &RecordedEvent,
    ) -> Result<(), patchbay_core::acceptance::AcceptanceError> {
        self.inner.lock().await.apply(event)
    }
}

impl LockedCommandStateLookup {
    pub async fn records_for_grant(&self, grant_id: &patchbay_contracts::patchbay::GrantId) -> Vec<CommandRecord> {
        self.inner.lock().await.records().filter(|record| record.grant_id.as_ref() == Some(grant_id)).cloned().collect()
    }
}

impl CommandStateLookup for LockedCommandStateLookup {
    async fn current_state(&self, command_id: &CommandId) -> Option<CommandSnapshot> {
        let index = self.inner.lock().await;
        CommandStateLookup::current_state(&*index, command_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{
        response_contract, AuthorityDomainId, Elicitation, ElicitationState, QuestionContract,
        ResponseContract, ResponseContractKind, ResponseOption, StoredEventKind,
        StoredEventPayload,
    };
    use prost::Message;

    #[tokio::test]
    async fn fold_lag_invariant_exposes_contract_only_after_storage_catch_up() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let elicitation_id = ElicitationId {
            value: "elicitation-fold-lag".to_owned(),
        };
        let contract = ResponseContract {
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
        };
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let state = ProjectionState::rebuild(&storage, &authority_domain_id)
            .await
            .unwrap();

        assert!(state
            .elicitation_contract_lookup()
            .active_contract(&elicitation_id)
            .await
            .is_none());

        storage
            .append(
                &authority_domain_id,
                StoredEventPayload {
                    kind: StoredEventKind::Elicitation as i32,
                    payload: Elicitation {
                        elicitation_id: Some(elicitation_id.clone()),
                        authority_domain_id: Some(authority_domain_id.clone()),
                        response_contract: Some(contract.clone()),
                        state: ElicitationState::Opened as i32,
                        ..Elicitation::default()
                    }
                    .encode_to_vec(),
                },
            )
            .await
            .unwrap();

        // A future Elicitation-opening producer (the pi adapter) must share
        // the CoreDecisionGate so an append-after-read race cannot bypass catch_up.
        state
            .catch_up(&storage, &authority_domain_id)
            .await
            .unwrap();

        let active = state
            .elicitation_contract_lookup()
            .active_contract(&elicitation_id)
            .await
            .expect("storage-backed catch_up exposes the active contract");
        assert_eq!(active.contract, contract);
        assert!(!active.is_terminal);
    }
}
