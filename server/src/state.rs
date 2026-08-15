use std::{sync::Arc, time::Duration};

use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, CommandId, ControlSurfacePrincipalRecord, ControlSurfaceRevocation,
    ElicitationId, EventId, Generation, Grant, GrantId, Lsn, OperationKind, OperatorRecord,
    OperatorSessionRevocation, Resource, ResourceSnapshot, ResourceViewRevision, Session,
    SessionCheckpointTombstone, SessionSnapshot, StoredEventKind, StoredSessionCheckpoint,
    TargetScope, TargetScopeKind, ViewRevision,
};
use patchbay_core::{
    acceptance::{
        ActiveElicitation, Authorized, CommandIndex, CommandRecord, CommandSnapshot,
        CommandStateLookup, ElicitationContractLookup, ElicitationSlotLayer, GrantCheck,
        GrantDenied, OperationPosture, OperationPostureDenied, ResolvedGrantCheck, TargetBinding,
        TargetNotFound, TargetResolver,
    },
    adapter::AdapterRegistry,
    authority::{
        ingest_control_surface_principal, ingest_control_surface_revocation,
        ingest_grant as ingest_authority_grant, ingest_operator_record,
        ingest_operator_session_revocation, ingest_revocation as ingest_authority_revocation,
        AuthorityError, AuthorityRegistry, ControlSurfaceRevocationTarget, GrantRecord,
        IssuerContext, OperatorError, OperatorRegistry, RevocationIngestResult,
    },
    diagnostics::DiagnosticsProjection,
    resource::ResourceRegistry,
    security::SecurityPostureProjection,
    session::{
        fold_spawn_promotion_ordered, SessionRegistry, SpawnClaimQuery, SpawnClaimRegistry,
        SpawnClaimability,
    },
    storage::{validate_next_replay_event, CoreGenerationStore, Storage, StorageError},
    target::TargetRegistry,
};
use tokio::sync::{Mutex, MutexGuard};

use crate::{
    decision_gate::CoreDecisionGate,
    identity::random_core_generation,
    operator_session::{OperatorSessionRegistry, DEFAULT_OPERATOR_SESSION_TTL},
    snapshot::{recover_session_registry, MaterializedSessionCheckpoint},
};

/// Server-owned concurrency boundary around core projections.
///
/// Ordinary port calls keep projection locks short-lived. Aggregate catch-up is
/// the deliberate exception: after staging, publication holds the cursor and
/// acquires operators -> authority -> targets -> spawn claims -> security ->
/// commands -> Elicitations -> diagnostics before its first live mutation. Those relative
/// orders preserve the existing operators -> authority and cursor -> targets ->
/// security reader paths. `submit_guard` serializes submission plus projection
/// catch-up, and is backed by the composition-root `CoreDecisionGate` shared
/// with adapter transitions. This can be replaced by a server-local actor
/// without changing the core library or the wire contract.
#[derive(Clone)]
pub struct ProjectionState {
    grant_check: LockedGrantCheck,
    target_resolver: LockedTargetResolver,
    state_lookup: LockedCommandStateLookup,
    spawn_claims: Arc<Mutex<SpawnClaimRegistry>>,
    elicitation_slots: LockedElicitationContractLookup,
    diagnostics: Arc<Mutex<DiagnosticsProjection>>,
    security_posture: LockedSecurityPosture,
    operators: Arc<Mutex<OperatorRegistry>>,
    pub(crate) operator_sessions: OperatorSessionRegistry,
    core_generation: Generation,
    last_applied_lsn: Arc<Mutex<u64>>,
    session_recovery_checkpoint_lsn: u64,
    session_replayed_event_count: usize,
    session_checkpoint_was_rejected: bool,
    decision_gate: CoreDecisionGate,
}

impl ProjectionState {
    pub async fn rebuild<S: Storage + CoreGenerationStore>(
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
    ) -> Result<Self, String> {
        Self::rebuild_with_session_ttl(storage, authority_domain_id, DEFAULT_OPERATOR_SESSION_TTL)
            .await
    }

    pub async fn rebuild_with_session_ttl<S: Storage + CoreGenerationStore>(
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

    pub async fn rebuild_with_session_ttl_and_gate<S: Storage + CoreGenerationStore>(
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        operator_session_ttl: Duration,
        decision_gate: CoreDecisionGate,
    ) -> Result<Self, String> {
        let core_generation = storage
            .load_or_create_core_generation(authority_domain_id, random_core_generation())
            .await
            .map_err(|error| error.to_string())?;
        let events = storage
            .read_after(authority_domain_id, Lsn { value: 0 })
            .await
            .map_err(|error| error.to_string())?;

        let recovered_sessions =
            recover_session_registry(storage, authority_domain_id, &core_generation)
                .await
                .map_err(|error| error.to_string())?;
        let session_recovery_checkpoint_lsn = recovered_sessions.checkpoint_lsn;
        let session_replayed_event_count = recovered_sessions.replayed_event_count;
        let session_checkpoint_was_rejected = recovered_sessions.checkpoint_rejected;

        let mut authority = AuthorityRegistry::new();
        // Promotion is a cross-projection replay unit, so the authoritative
        // aggregate rebuild folds sessions from the same full prefix as
        // authority/claim/command. The separately recovered checkpoint remains
        // available to diagnostics and checkpoint-health evidence only.
        let sessions =
            SessionRegistry::new(authority_domain_id.clone()).map_err(|error| error.to_string())?;
        let resources = ResourceRegistry::new();
        let adapters = AdapterRegistry::new();
        let mut targets = TargetRegistry::with_adapters(sessions, resources, adapters);
        let mut claims = SpawnClaimRegistry::new(authority_domain_id.clone())
            .map_err(|error| error.to_string())?;
        let mut commands = CommandIndex::new();
        let mut elicitation_slots = ElicitationSlotLayer::new();
        let mut diagnostics =
            DiagnosticsProjection::with_session_registry(recovered_sessions.registry);
        let mut security_posture = SecurityPostureProjection::new();
        let mut operators = OperatorRegistry::new();
        let operator_sessions = OperatorSessionRegistry::new(operator_session_ttl)?;
        let mut last_applied_lsn = 0;
        for event in &events {
            let validated =
                validate_next_replay_event(authority_domain_id, last_applied_lsn, event)
                    .map_err(|error| error.to_string())?;
            if validated.kind == StoredEventKind::SpawnPromotionCommitted {
                fold_spawn_promotion_ordered(
                    &mut authority,
                    &mut targets,
                    &mut claims,
                    &mut commands,
                    event,
                )
                .map_err(|error| error.to_string())?;
            } else {
                authority
                    .observe(event)
                    .map_err(|error| error.to_string())?;
                targets
                    .observe_event(event)
                    .map_err(|error| error.to_string())?;
                claims.observe(event).map_err(|error| error.to_string())?;
                commands.apply(event).map_err(|error| error.to_string())?;
            }
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
            last_applied_lsn = validated.lsn;
        }
        diagnostics.reset_adapter_liveness();

        let spawn_claims = Arc::new(Mutex::new(claims));
        Ok(Self {
            grant_check: LockedGrantCheck::new(authority),
            target_resolver: LockedTargetResolver::new(targets, Arc::clone(&spawn_claims)),
            state_lookup: LockedCommandStateLookup::new(commands),
            spawn_claims,
            elicitation_slots: LockedElicitationContractLookup::from_layer(elicitation_slots),
            diagnostics: Arc::new(Mutex::new(diagnostics)),
            security_posture: LockedSecurityPosture::new(security_posture),
            operators: Arc::new(Mutex::new(operators)),
            operator_sessions,
            core_generation,
            last_applied_lsn: Arc::new(Mutex::new(last_applied_lsn)),
            session_recovery_checkpoint_lsn,
            session_replayed_event_count,
            session_checkpoint_was_rejected,
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
        let live_adapters: Vec<_> = self
            .diagnostics
            .lock()
            .await
            .live_adapter_ids()
            .cloned()
            .collect();
        let events = storage
            .read_through(
                authority_domain_id,
                Lsn { value: 0 },
                Lsn { value: as_of_lsn },
            )
            .await?;
        let mut projection = DiagnosticsProjection::new(authority_domain_id.clone())
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        let mut previous_lsn = 0;
        for event in events {
            let event_lsn = event.event_id.lsn.as_ref().ok_or_else(|| {
                StorageError::CorruptRecord(
                    "bounded diagnostics replay returned an event with no LSN".to_owned(),
                )
            })?;
            if event_lsn.value > as_of_lsn {
                return Err(StorageError::CorruptRecord(format!(
                    "bounded diagnostics replay returned LSN {} beyond requested LSN {as_of_lsn}",
                    event_lsn.value
                )));
            }
            let validated = validate_next_replay_event(authority_domain_id, previous_lsn, &event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            projection
                .observe(&event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            previous_lsn = validated.lsn;
        }
        if previous_lsn != as_of_lsn {
            return Err(StorageError::CorruptRecord(format!(
                "bounded diagnostics replay ended at LSN {previous_lsn}, expected exact LSN {as_of_lsn}"
            )));
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
                .and_then(|page| {
                    page.adapters
                        .into_iter()
                        .find(|status| status.adapter_id.as_ref() == Some(&adapter_id))
                })
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
    ) -> Result<
        patchbay_contracts::patchbay::AdapterStatusPage,
        patchbay_core::diagnostics::DiagnosticsError,
    > {
        self.diagnostics.lock().await.adapter_page(query, as_of)
    }

    #[must_use]
    pub fn core_generation(&self) -> &Generation {
        &self.core_generation
    }

    pub async fn current_lsn(&self) -> u64 {
        *self.last_applied_lsn.lock().await
    }

    #[must_use]
    pub fn session_recovery_checkpoint_lsn(&self) -> u64 {
        self.session_recovery_checkpoint_lsn
    }

    #[must_use]
    pub fn session_replayed_event_count(&self) -> usize {
        self.session_replayed_event_count
    }

    #[must_use]
    pub fn session_checkpoint_was_rejected(&self) -> bool {
        self.session_checkpoint_was_rejected
    }

    pub async fn current_runtime_session_count(&self) -> u32 {
        self.target_resolver
            .inner
            .lock()
            .await
            .sessions()
            .sessions()
            .count() as u32
    }

    #[cfg(test)]
    pub async fn conformance_session_registry(&self) -> patchbay_core::session::SessionRegistry {
        self.target_resolver.inner.lock().await.sessions().clone()
    }

    #[cfg(test)]
    pub async fn conformance_spawn_claim_registry(&self) -> SpawnClaimRegistry {
        self.spawn_claims.lock().await.clone()
    }

    pub async fn submit_guard(&self) -> MutexGuard<'_, ()> {
        self.decision_gate.acquire().await
    }

    /// Materialize the authoritative live-session projection at its applied LSN.
    ///
    /// The cursor lock is acquired before the session lock, matching
    /// `catch_up`, so the returned records and `snapshot_lsn` describe one
    /// consistent projection prefix. The production private checkpoint
    /// materializer adds retained tombstones to this public snapshot shape;
    /// callers that need recovery bytes use `materialize_session_checkpoint`.
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
                last_source_cursor: record.last_source_cursor,
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
            core_generation: Some(self.core_generation),
            sessions,
            view_revisions,
            materialized_at: Some(materialized_at),
            lockdown: Some(lockdown),
        }
    }

    /// Materialize the complete private session checkpoint payload.
    ///
    /// The caller must hold the shared decision gate across this call. That
    /// makes the separately encoded snapshot and tombstone collection one
    /// consistent applied prefix without holding the gate during storage I/O.
    pub async fn materialize_session_checkpoint(
        &self,
        authority_domain_id: AuthorityDomainId,
        materialized_at: prost_types::Timestamp,
    ) -> MaterializedSessionCheckpoint {
        let snapshot = self
            .materialize_session_snapshot(authority_domain_id, materialized_at)
            .await;
        let registry = self.target_resolver.inner.lock().await;
        let mut tombstones: Vec<_> = registry.sessions().tombstones().cloned().collect();
        tombstones.sort_by(|left, right| {
            (
                &left.adapter_id.value,
                &left.deployment_scope,
                &left.runtime_session_id.value,
                left.superseded_generation.value,
            )
                .cmp(&(
                    &right.adapter_id.value,
                    &right.deployment_scope,
                    &right.runtime_session_id.value,
                    right.superseded_generation.value,
                ))
        });
        let managed_lineages = registry.sessions().managed_lineage_checkpoint_records();
        MaterializedSessionCheckpoint::new(
            StoredSessionCheckpoint {
                snapshot: Some(snapshot),
                tombstones: tombstones
                    .into_iter()
                    .map(|tombstone| SessionCheckpointTombstone {
                        adapter_id: Some(tombstone.adapter_id),
                        deployment_scope: tombstone.deployment_scope,
                        runtime_session_id: Some(tombstone.runtime_session_id),
                        generation: Some(tombstone.superseded_generation),
                        superseded_at_lsn: Some(Lsn {
                            value: tombstone.superseded_at_lsn,
                        }),
                    })
                    .collect(),
                logical_targets: registry.sessions().logical_targets().checkpoint_records(),
            },
            managed_lineages,
        )
    }

    /// Materialize the complete operational-resource projection at one applied
    /// durable prefix. Active and tombstoned records are retained; consumers
    /// use freshness and tombstone fields rather than inferring liveness.
    pub async fn materialize_resource_snapshot(
        &self,
        authority_domain_id: AuthorityDomainId,
        materialized_at: prost_types::Timestamp,
    ) -> ResourceSnapshot {
        let cursor = self.last_applied_lsn.lock().await;
        let registry = self.target_resolver.inner.lock().await;
        let mut records: Vec<_> = registry.resources().resources().cloned().collect();
        records.sort_by(|left, right| {
            (
                &left.identity.adapter_id().value,
                &left.identity.resource_kind().value,
                &left.identity.resource_id().value,
            )
                .cmp(&(
                    &right.identity.adapter_id().value,
                    &right.identity.resource_kind().value,
                    &right.identity.resource_id().value,
                ))
        });
        let resources = records
            .into_iter()
            .map(|record| {
                let tombstoned = record.tombstoned();
                Resource {
                    authority_domain_id: Some(authority_domain_id.clone()),
                    identity: record.identity.to_scope().resource,
                    resource_payload: record.resource_payload,
                    projection_payload: record.projection_payload,
                    freshness: record.freshness as i32,
                    source_adapter_generation: Some(record.source_adapter_generation),
                    revision_lsn: Some(Lsn {
                        value: record.revision_lsn,
                    }),
                    observed_at: Some(record.observed_at),
                    tombstoned,
                    tombstoned_at_lsn: record.tombstoned_at_lsn.map(|value| Lsn { value }),
                    replaced_by: record
                        .replaced_by
                        .map(|identity| identity.to_scope().resource.expect("canonical resource")),
                }
            })
            .collect();
        let mut views: Vec<_> = registry.resources().views().cloned().collect();
        views.sort_by(|left, right| {
            (&left.key.adapter_id.value, &left.key.resource_kind.value)
                .cmp(&(&right.key.adapter_id.value, &right.key.resource_kind.value))
        });
        let view_revisions = views
            .into_iter()
            .map(|view| ResourceViewRevision {
                adapter_id: Some(view.key.adapter_id),
                resource_kind: Some(view.key.resource_kind),
                completeness: view.completeness as i32,
                source_adapter_generation: Some(view.source_adapter_generation),
                revision_lsn: Some(Lsn {
                    value: view.revision_lsn,
                }),
                observed_at: Some(view.observed_at),
            })
            .collect();
        ResourceSnapshot {
            authority_domain_id: Some(authority_domain_id),
            snapshot_lsn: Some(Lsn { value: *cursor }),
            core_generation: Some(self.core_generation),
            resources,
            view_revisions,
            materialized_at: Some(materialized_at),
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
                .map(
                    |principal| patchbay_contracts::patchbay::ControlSurfaceSummary {
                        principal_id: principal.principal_id.clone(),
                        endpoint_id: principal.endpoint_id.clone(),
                        device_id: principal.device_id.clone(),
                        endpoint_generation: principal.endpoint_generation,
                        revoked: operators.is_principal_revoked(&principal.principal_id),
                    },
                )
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
            snapshot_lsn: Some(Lsn {
                value: snapshot_lsn,
            }),
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
        self.catch_up_with_before_publish(storage, authority_domain_id, std::future::ready(()))
            .await
    }

    async fn catch_up_with_before_publish<S: Storage, F: std::future::Future<Output = ()>>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        before_publish: F,
    ) -> Result<(), StorageError> {
        let mut cursor = self.last_applied_lsn.lock().await;
        let events = storage
            .read_after(authority_domain_id, Lsn { value: *cursor })
            .await?;
        if events.is_empty() {
            return Ok(());
        }

        // A catch-up tail is one aggregate projection transaction. Clone every
        // affected view, fold and validate the complete returned suffix, then
        // install all views and the cursor only after the final event succeeds.
        let mut staged_authority = self.grant_check.inner.lock().await.clone();
        let mut staged_targets = self.target_resolver.inner.lock().await.clone();
        let mut staged_claims = self.spawn_claims.lock().await.clone();
        let mut staged_commands = self.state_lookup.inner.lock().await.clone();
        let mut staged_elicitations = self.elicitation_slots.inner.lock().await.clone();
        let mut staged_diagnostics = self.diagnostics.lock().await.clone();
        let mut staged_security = self.security_posture.inner.lock().await.clone();
        let mut staged_operators = self.operators.lock().await.clone();
        let _operator_session_guard = self.operator_sessions.replay_guard().await;
        let staged_operator_sessions = self.operator_sessions.staged_clone_unlocked().await;
        let mut staged_cursor = *cursor;

        for event in &events {
            let validated = validate_next_replay_event(authority_domain_id, staged_cursor, event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            if validated.kind == StoredEventKind::SpawnPromotionCommitted {
                fold_spawn_promotion_ordered(
                    &mut staged_authority,
                    &mut staged_targets,
                    &mut staged_claims,
                    &mut staged_commands,
                    event,
                )
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            } else {
                staged_authority
                    .observe(event)
                    .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
                staged_targets
                    .observe_event(event)
                    .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
                staged_claims
                    .observe(event)
                    .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
                staged_commands
                    .apply(event)
                    .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            }
            staged_elicitations
                .observe(event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            staged_diagnostics
                .observe(event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            staged_security
                .observe(event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            staged_operators
                .observe(event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            if validated.kind == StoredEventKind::ControlSurfaceRevocation {
                let revocation: ControlSurfaceRevocation =
                    prost::Message::decode(event.payload.payload.as_slice()).map_err(|error| {
                        StorageError::CorruptRecord(format!(
                            "cannot decode control-surface revocation: {error}"
                        ))
                    })?;
                let target = match revocation.target.ok_or_else(|| {
                    StorageError::CorruptRecord(
                        "control-surface revocation has no target".to_owned(),
                    )
                })? {
                    patchbay_contracts::patchbay::control_surface_revocation::Target::PrincipalId(id) => ControlSurfaceRevocationTarget::Principal(id),
                    patchbay_contracts::patchbay::control_surface_revocation::Target::EndpointId(id) => ControlSurfaceRevocationTarget::Endpoint(id),
                    patchbay_contracts::patchbay::control_surface_revocation::Target::DeviceId(id) => ControlSurfaceRevocationTarget::Device(id),
                };
                let principal = if let ControlSurfaceRevocationTarget::Principal(id) = &target {
                    staged_operators.principal_record(id)
                } else {
                    None
                };
                revoke_operator_sessions_for_target(&staged_operator_sessions, &target, principal)
                    .await;
            }
            staged_operator_sessions
                .observe(event)
                .await
                .map_err(StorageError::CorruptRecord)?;
            staged_cursor = validated.lsn;
        }
        let prepared_operator_sessions = staged_operator_sessions.prepare_install_unlocked().await;

        before_publish.await;

        // Acquire every live publication guard before the first assignment.
        // Cancellation while any lock is contended therefore drops only staged
        // values. Once assignment begins, this function has no suspension point.
        let mut live_operators = self.operators.lock().await;
        let mut live_authority = self.grant_check.inner.lock().await;
        let mut live_targets = self.target_resolver.inner.lock().await;
        let mut live_claims = self.spawn_claims.lock().await;
        let mut live_security = self.security_posture.inner.lock().await;
        let mut live_commands = self.state_lookup.inner.lock().await;
        let mut live_elicitations = self.elicitation_slots.inner.lock().await;
        let mut live_diagnostics = self.diagnostics.lock().await;
        let mut live_operator_sessions = self.operator_sessions.install_guards_unlocked().await;

        live_operator_sessions.install(prepared_operator_sessions);
        *live_operators = staged_operators;
        *live_authority = staged_authority;
        *live_targets = staged_targets;
        *live_claims = staged_claims;
        *live_security = staged_security;
        *live_commands = staged_commands;
        *live_elicitations = staged_elicitations;
        *live_diagnostics = staged_diagnostics;
        *cursor = staged_cursor;
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

    pub async fn has_endpoint(
        &self,
        endpoint_id: &patchbay_contracts::patchbay::EndpointId,
    ) -> bool {
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
        self.operator_sessions
            .revoke_current(session_id, binding)
            .await
    }

    pub async fn current_operator_session_generation(&self, actor_id: &ActorId) -> Generation {
        self.operator_sessions.current_generation(actor_id).await
    }

    pub async fn revoke_all_operator_sessions(
        &self,
        actor_id: &ActorId,
        through: &Generation,
    ) -> u32 {
        self.operator_sessions
            .revoke_all_for_actor(actor_id, through)
            .await
    }

    pub async fn revoke_sessions_for_target(&self, target: &ControlSurfaceRevocationTarget) -> u32 {
        let principal = if let ControlSurfaceRevocationTarget::Principal(principal_id) = target {
            self.operators
                .lock()
                .await
                .principal_record(principal_id)
                .cloned()
        } else {
            None
        };
        revoke_operator_sessions_for_target(&self.operator_sessions, target, principal.as_ref())
            .await
    }

    pub async fn ingest_operator_session_revocation<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        revocation: OperatorSessionRevocation,
    ) -> Result<(RevocationIngestResult, u32), OperatorError> {
        let actor = revocation.operator_actor_id.clone().ok_or_else(|| {
            OperatorError::InvalidRecord("session revocation has no actor".to_owned())
        })?;
        let generation = revocation.invalidated_through_generation.ok_or_else(|| {
            OperatorError::InvalidRecord("session revocation has no generation".to_owned())
        })?;
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

    pub async fn commands_for_grant(
        &self,
        grant_id: &patchbay_contracts::patchbay::GrantId,
    ) -> Vec<CommandRecord> {
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
            .filter(|grant| grant.is_live_at(now) && grant.is_recovery_capable_authority_domain())
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
        )
        .await
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

async fn revoke_operator_sessions_for_target(
    operator_sessions: &OperatorSessionRegistry,
    target: &ControlSurfaceRevocationTarget,
    principal: Option<&ControlSurfacePrincipalRecord>,
) -> u32 {
    match target {
        ControlSurfaceRevocationTarget::Principal(_) => {
            operator_sessions
                .revoke_matching_principal(|binding| {
                    principal.is_some_and(|record| {
                        record.operator_actor_id.as_ref() == Some(&binding.actor_id)
                            && record.endpoint_id.as_ref() == Some(&binding.endpoint_id)
                            && record.device_id.as_ref() == Some(&binding.device_id)
                            && record.endpoint_generation.as_ref()
                                == Some(&binding.endpoint_generation)
                    })
                })
                .await
        }
        ControlSurfaceRevocationTarget::Endpoint(endpoint_id) => {
            operator_sessions
                .revoke_matching_principal(|binding| &binding.endpoint_id == endpoint_id)
                .await
        }
        ControlSurfaceRevocationTarget::Device(device_id) => {
            operator_sessions
                .revoke_matching_principal(|binding| &binding.device_id == device_id)
                .await
        }
    }
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
        GrantCheck::check(
            &*registry,
            authority_domain_id,
            issuer,
            operation_kind,
            target_scope,
        )
        .await
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
        GrantCheck::check_at(
            &*registry,
            authority_domain_id,
            issuer,
            operation_kind,
            target_scope,
            evaluated_at,
        )
        .await
    }

    async fn check_resolved_at(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        request: ResolvedGrantCheck<'_>,
    ) -> Result<Authorized, GrantDenied> {
        let registry = self.inner.lock().await;
        GrantCheck::check_resolved_at(&*registry, authority_domain_id, issuer, request).await
    }
}

#[derive(Clone)]
pub struct LockedTargetResolver {
    inner: Arc<Mutex<TargetRegistry>>,
    spawn_claims: Arc<Mutex<SpawnClaimRegistry>>,
}

impl LockedTargetResolver {
    fn new(registry: TargetRegistry, spawn_claims: Arc<Mutex<SpawnClaimRegistry>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(registry)),
            spawn_claims,
        }
    }
}

impl TargetResolver for LockedTargetResolver {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        operation: &patchbay_contracts::patchbay::Operation,
        spawn_request: Option<&patchbay_contracts::patchbay::SpawnRequest>,
    ) -> Result<TargetBinding, TargetNotFound> {
        let binding = {
            let registry = self.inner.lock().await;
            TargetResolver::resolve(&*registry, authority_domain_id, operation, spawn_request)
                .await?
        };
        if let TargetBinding::SpawnAdapter { claim, .. } = &binding {
            match self.spawn_claims.lock().await.classify_claim(claim) {
                SpawnClaimability::Available | SpawnClaimability::ExactRetry(_) => {}
                SpawnClaimability::Conflict(record) => {
                    let command_id = record.claim.claim_operation_id.clone().ok_or_else(|| {
                        TargetNotFound::NotFound {
                            target: "durable spawn claim has no operation id".to_owned(),
                        }
                    })?;
                    return Err(TargetNotFound::ReplacementPending { command_id });
                }
                SpawnClaimability::Invalid => {
                    return Err(TargetNotFound::NotFound {
                        target: "spawn generation claim is malformed".to_owned(),
                    });
                }
            }
        }
        Ok(binding)
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
}

impl ElicitationContractLookup for LockedElicitationContractLookup {
    async fn active_contract(&self, elicitation_id: &ElicitationId) -> Option<ActiveElicitation> {
        let layer = self.inner.lock().await;
        let record = layer.get_slot(elicitation_id)?;
        Some(ActiveElicitation {
            contract: record.contract.clone()?,
            expected_responder_actor: record.expected_responder_actor.clone(),
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
}

impl LockedCommandStateLookup {
    pub async fn records_for_grant(
        &self,
        grant_id: &patchbay_contracts::patchbay::GrantId,
    ) -> Vec<CommandRecord> {
        self.inner
            .lock()
            .await
            .records()
            .filter(|record| record.grant_id.as_ref() == Some(grant_id))
            .cloned()
            .collect()
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
        response_contract, ActorId, AuthorityDomainId, Elicitation, ElicitationState,
        QuestionContract, ResponseContract, ResponseContractKind, ResponseOption, StoredEventKind,
        StoredEventPayload,
    };
    use patchbay_core::storage::RecordedEvent;
    use prost::Message;

    #[tokio::test]
    async fn session_and_resource_snapshots_share_the_persisted_anchor() {
        let domain = AuthorityDomainId {
            value: "authority-main".into(),
        };
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let state = ProjectionState::rebuild(&storage, &domain).await.unwrap();
        let materialized_at = prost_types::Timestamp {
            seconds: 1,
            nanos: 0,
        };

        let session = state
            .materialize_session_snapshot(domain.clone(), materialized_at)
            .await;
        let resource = state
            .materialize_resource_snapshot(domain.clone(), materialized_at)
            .await;

        assert_eq!(session.authority_domain_id.as_ref(), Some(&domain));
        assert_eq!(resource.authority_domain_id.as_ref(), Some(&domain));
        assert_eq!(session.snapshot_lsn, Some(Lsn { value: 0 }));
        assert_eq!(resource.snapshot_lsn, Some(Lsn { value: 0 }));
        assert_eq!(
            session.core_generation.as_ref(),
            Some(state.core_generation())
        );
        assert_eq!(resource.core_generation, session.core_generation);
        assert!(resource
            .core_generation
            .is_some_and(|generation| generation.value > 0));
    }

    #[tokio::test]
    async fn session_snapshot_publishes_the_last_source_cursor() {
        use patchbay_contracts::patchbay::{
            AdapterId, Generation, RuntimeSessionId, SessionActivityState,
            SessionConnectivityState, SessionRegistered, SessionReportSourceCursor, SessionState,
        };

        let domain = AuthorityDomainId {
            value: "authority-main".into(),
        };
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let cursor = SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 2 }),
            revision: 3,
        };
        let registration = patchbay_core::session::events::registered(
            domain.clone(),
            SessionRegistered {
                adapter_id: Some(AdapterId { value: "pi".into() }),
                deployment_scope: "machine-a".into(),
                runtime_session_id: Some(RuntimeSessionId {
                    value: "runtime-1".into(),
                }),
                session_generation: Some(Generation { value: 1 }),
                initial_state: Some(SessionState {
                    connectivity: SessionConnectivityState::Live as i32,
                    activity: SessionActivityState::Idle as i32,
                }),
                project: "patchbay".into(),
                cwd: "/work/patchbay".into(),
                name: "main".into(),
                model: "provider/model".into(),
                spawn_origin: None,
                source_cursor: Some(cursor),
            },
        );
        storage
            .append(
                &domain,
                patchbay_core::session::events::encode(&registration),
            )
            .await
            .unwrap();

        let state = ProjectionState::rebuild(&storage, &domain).await.unwrap();
        let snapshot = state
            .materialize_session_snapshot(
                domain,
                prost_types::Timestamp {
                    seconds: 1,
                    nanos: 0,
                },
            )
            .await;
        assert_eq!(snapshot.sessions.len(), 1);
        assert_eq!(snapshot.sessions[0].last_source_cursor, Some(cursor));
    }

    #[tokio::test]
    async fn spawn_promotion_catch_up_and_restart_publish_one_complete_aggregate() {
        use patchbay_contracts::patchbay::{
            runtime_generation_disposition, spawn_claim_event, spawn_request, typed_correlation,
            AcceptedOperation, ActorEndpointRef, AdapterCapability, AdapterId, AdapterRegistration,
            AdapterSnapshotSupport, AdapterTargetCategory, AuditEventKind, CommandTransition,
            DescendantGrant, DescendantGrantProvenance, EndpointId, ExternalEffectDisposition,
            ExternalRuntimeRef, FailureCode, FreshSpawn, Grant, GrantId, GrantProvenance,
            GrantRevocationPolicy, LogicalTargetCreated, LogicalTargetId, Observation,
            ObservationKind, Operation, OperationState, PayloadContentType, PayloadEnvelope,
            RuntimeEvidenceSourceAttachment, RuntimeGenerationClaimedSuccessor,
            RuntimeGenerationDisposition, RuntimeGenerationRef, RuntimeSessionId,
            SessionActivityState, SessionConnectivityState, SessionReport,
            SessionReportSourceCursor, SpawnClaimAccepted, SpawnClaimDisposition, SpawnClaimEvent,
            SpawnEvidenceAttachment, SpawnExecutionEvidence, SpawnExecutionEvidenceProducer,
            SpawnExecutionPhase, SpawnGenerationClaim, SpawnPromotionAuthorityEvidence,
            SpawnPromotionCommitted, SpawnPromotionLifecycleEvidence, SpawnPromotionResultEvidence,
            SpawnPromotionStagedEvidence, SpawnRequest, SpawnSuccessorEvidenceStaged,
            SpawnTargetSpec, TargetScope, TypedCorrelation,
        };
        use patchbay_core::{
            authority::DESCENDANT_GRANT_ALLOWED_KINDS,
            session::{encode_spawn_claim_event, encode_staged_successor, SpawnClaimQuery},
            storage::AuditRecordDraft,
        };
        use prost_types::Timestamp;

        let domain = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let adapter = AdapterId {
            value: "pi".to_owned(),
        };
        let command = CommandId {
            value: "spawn-a".to_owned(),
        };
        let logical = LogicalTargetId {
            value: "logical-a".to_owned(),
        };
        let external = ExternalRuntimeRef {
            adapter_id: Some(adapter.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "runtime-a".to_owned(),
            }),
            generation: Some(Generation { value: 1 }),
        };
        let event_id = |lsn| EventId {
            authority_domain_id: Some(domain.clone()),
            lsn: Some(Lsn { value: lsn }),
        };
        let operation = Operation {
            command_id: Some(command.clone()),
            authority_domain_id: Some(domain.clone()),
            sender: Some(ActorEndpointRef {
                actor_id: Some(ActorId {
                    value: "operator".to_owned(),
                }),
                endpoint_id: Some(EndpointId {
                    value: "web".to_owned(),
                }),
                ..ActorEndpointRef::default()
            }),
            kind: OperationKind::Spawn as i32,
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::Adapter as i32,
                adapter_id: Some(adapter.clone()),
                ..TargetScope::default()
            }),
            idempotency_key: "spawn-key".to_owned(),
            payload: Some(PayloadEnvelope {
                payload: SpawnRequest {
                    intent: Some(spawn_request::Intent::Fresh(FreshSpawn {})),
                    target_spec: Some(SpawnTargetSpec {
                        shape: "session".to_owned(),
                        ..SpawnTargetSpec::default()
                    }),
                }
                .encode_to_vec(),
                content_type: PayloadContentType::Protobuf as i32,
                schema_ref: patchbay_core::acceptance::SPAWN_REQUEST_SCHEMA.to_owned(),
            }),
            ..Operation::default()
        };
        let accepted_operation = AcceptedOperation {
            operation: Some(operation.clone()),
            authorizing_grant_id: Some(GrantId {
                value: "spawn-grant".to_owned(),
            }),
        };
        let claim = SpawnGenerationClaim {
            authority_domain_id: Some(domain.clone()),
            claim_operation_id: Some(command.clone()),
            logical_target_id: Some(logical.clone()),
            expected_prior: None,
            claimed_generation: Some(Generation { value: 1 }),
        };
        let accepted_claim = SpawnClaimAccepted {
            accepted_operation: Some(accepted_operation.clone()),
            claim: Some(claim.clone()),
            ..SpawnClaimAccepted::default()
        };
        let attachment_registration = AdapterRegistration {
            adapter_id: Some(adapter.clone()),
            endpoint_id: Some(EndpointId {
                value: "pi-endpoint".to_owned(),
            }),
            authority_domain_id: Some(domain.clone()),
            adapter_generation: Some(Generation { value: 3 }),
            capability: Some(AdapterCapability {
                session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
                target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
                ..AdapterCapability::default()
            }),
            ..AdapterRegistration::default()
        };
        let attachment = Observation {
            authority_domain_id: Some(domain.clone()),
            sender: Some(ActorEndpointRef {
                actor_id: Some(ActorId {
                    value: "pi".to_owned(),
                }),
                endpoint_id: Some(EndpointId {
                    value: "pi-endpoint".to_owned(),
                }),
                ..ActorEndpointRef::default()
            }),
            kind: ObservationKind::Event as i32,
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::Adapter as i32,
                adapter_id: Some(adapter.clone()),
                ..TargetScope::default()
            }),
            payload: Some(PayloadEnvelope {
                payload: attachment_registration.encode_to_vec(),
                content_type: PayloadContentType::Protobuf as i32,
                schema_ref: "patchbay.AdapterRegistration".to_owned(),
            }),
            ..Observation::default()
        };
        let report = SessionReport {
            adapter_id: Some(adapter.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: external.runtime_session_id.clone(),
            session_generation: external.generation,
            connectivity: SessionConnectivityState::Live as i32,
            activity: SessionActivityState::Idle as i32,
            spawn_origin: Some(TypedCorrelation {
                r#ref: Some(typed_correlation::Ref::CommandId(command.clone())),
            }),
            source_cursor: Some(SessionReportSourceCursor {
                adapter_generation: Some(Generation { value: 3 }),
                revision: 1,
            }),
            ..SessionReport::default()
        };
        let promoted = RuntimeGenerationRef {
            logical_target_id: Some(logical.clone()),
            external_runtime: Some(external.clone()),
        };
        let staged = SpawnSuccessorEvidenceStaged {
            authority_domain_id: Some(domain.clone()),
            exact_claim: Some(claim.clone()),
            report: Some(report),
            classified_target: Some(promoted.clone()),
            disposition: Some(RuntimeGenerationDisposition {
                disposition: Some(
                    runtime_generation_disposition::Disposition::ClaimedSuccessor(
                        RuntimeGenerationClaimedSuccessor {
                            claim_operation_id: Some(command.clone()),
                            expected_prior: None,
                            claimed_generation: Some(Generation { value: 1 }),
                        },
                    ),
                ),
            }),
            source_attachment: Some(RuntimeEvidenceSourceAttachment {
                adapter_id: Some(adapter.clone()),
                adapter_generation: Some(Generation { value: 3 }),
                attachment_event_id: Some(event_id(1)),
            }),
            external_runtime_reservation: Some(external.clone()),
        };
        let transition = |from, to| CommandTransition {
            command_id: Some(command.clone()),
            from_state: from as i32,
            to_state: to as i32,
            failure_code: FailureCode::Unspecified as i32,
            ..CommandTransition::default()
        };
        let result = Observation {
            authority_domain_id: Some(domain.clone()),
            kind: ObservationKind::Result as i32,
            correlations: vec![TypedCorrelation {
                r#ref: Some(typed_correlation::Ref::CommandId(command.clone())),
            }],
            target_scope: operation.target_scope.clone(),
            failure_code: FailureCode::Unspecified as i32,
            observed_at: Some(Timestamp {
                seconds: 10,
                nanos: 0,
            }),
            ..Observation::default()
        };
        let progress = |phase| StoredEventPayload {
            kind: StoredEventKind::SpawnExecutionEvidence as i32,
            payload: SpawnExecutionEvidence {
                authority_domain_id: Some(domain.clone()),
                exact_claim: Some(claim.clone()),
                phase: phase as i32,
                external_effect_disposition: ExternalEffectDisposition::Identified as i32,
                producer: SpawnExecutionEvidenceProducer::CurrentAdapter as i32,
                source_attachment: Some(SpawnEvidenceAttachment {
                    adapter_id: Some(adapter.clone()),
                    adapter_generation: Some(Generation { value: 3 }),
                    attachment_event_id: Some(event_id(1)),
                }),
                failure_code: FailureCode::Unspecified as i32,
                no_external_effect_proof: None,
                external_runtime: Some(promoted.clone()),
            }
            .encode_to_vec(),
        };
        let prefix = vec![
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: attachment.encode_to_vec(),
            },
            StoredEventPayload {
                kind: StoredEventKind::Grant as i32,
                payload: Grant {
                    grant_id: accepted_operation.authorizing_grant_id.clone(),
                    authority_domain_id: Some(domain.clone()),
                    subject_actor_id: Some(ActorId {
                        value: "operator".to_owned(),
                    }),
                    subject_endpoint_id: Some(EndpointId {
                        value: "web".to_owned(),
                    }),
                    target_scope: operation.target_scope.clone(),
                    allowed_operation_kinds: vec![OperationKind::Spawn as i32],
                    created_at: Some(Timestamp {
                        seconds: 1,
                        nanos: 0,
                    }),
                    provenance: Some(GrantProvenance {
                        reason: "server promotion fixture".to_owned(),
                        ..GrantProvenance::default()
                    }),
                    revocation_policy: GrantRevocationPolicy::Continue as i32,
                    ..Grant::default()
                }
                .encode_to_vec(),
            },
            patchbay_core::session::events::encode(
                &patchbay_core::session::events::logical_target_created(
                    domain.clone(),
                    LogicalTargetCreated {
                        logical_target_id: Some(logical.clone()),
                        adapter_id: Some(adapter.clone()),
                        deployment_scope: "machine-a".to_owned(),
                    },
                ),
            ),
            encode_spawn_claim_event(&SpawnClaimEvent {
                authority_domain_id: Some(domain.clone()),
                mutation: Some(spawn_claim_event::Mutation::Accepted(
                    accepted_claim.clone(),
                )),
            }),
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Accepted, OperationState::Delivered)
                    .encode_to_vec(),
            },
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Delivered, OperationState::Running)
                    .encode_to_vec(),
            },
            progress(SpawnExecutionPhase::ExternalIdentityKnown),
            progress(SpawnExecutionPhase::HandshakeReconciling),
            encode_staged_successor(&staged),
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: result.encode_to_vec(),
            },
            progress(SpawnExecutionPhase::SuccessEvidenceReported),
        ];
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let mut expected_lsn = 1;
        for payload in prefix {
            let source_lsn = expected_lsn;
            let appended = if payload.kind == StoredEventKind::SpawnExecutionEvidence as i32 {
                storage
                    .append_spawn_execution_evidence_reconciled(
                        &domain,
                        SpawnExecutionEvidence::decode(payload.payload.as_slice()).unwrap(),
                    )
                    .await
                    .unwrap()
                    .evidence_event_id
            } else if payload.kind == StoredEventKind::SpawnSuccessorEvidenceStaged as i32 {
                storage
                    .append_spawn_successor_staged_idempotent(
                        &domain,
                        SpawnSuccessorEvidenceStaged::decode(payload.payload.as_slice()).unwrap(),
                    )
                    .await
                    .unwrap()
            } else if payload.kind == StoredEventKind::SpawnClaim as i32 {
                let claim_event = SpawnClaimEvent::decode(payload.payload.as_slice()).unwrap();
                let spawn_claim_event::Mutation::Accepted(accepted) = claim_event.mutation.unwrap()
                else {
                    panic!("expected accepted claim fixture");
                };
                let accepted_operation = accepted.accepted_operation.as_ref().unwrap();
                let accepted_wire = accepted_operation.operation.as_ref().unwrap();
                let mut audit = AuditRecordDraft::new(
                    Timestamp {
                        seconds: 10,
                        nanos: 0,
                    },
                    AuditEventKind::CommandSubmissionAccepted,
                );
                audit.actor_id = accepted_wire
                    .sender
                    .as_ref()
                    .and_then(|sender| sender.actor_id.clone());
                audit.endpoint_id = accepted_wire
                    .sender
                    .as_ref()
                    .and_then(|sender| sender.endpoint_id.clone());
                audit.device_id = accepted_wire
                    .sender
                    .as_ref()
                    .and_then(|sender| sender.device_id.clone());
                audit.command_id = accepted_wire.command_id.clone();
                audit.grant_id = accepted_operation.authorizing_grant_id.clone();
                audit.target_scope = accepted_wire.target_scope.clone();
                audit.reason_code = "operation_spawn".to_owned();
                let key = patchbay_contracts::patchbay::IdempotencyKey {
                    value: accepted_wire.idempotency_key.clone(),
                };
                let logical_payload = accepted_wire.encode_to_vec();
                let committed = storage
                    .append_spawn_claim_accepted(
                        &domain,
                        &key,
                        &patchbay_core::storage::TargetKey::new("spawn-target".to_owned()).unwrap(),
                        accepted,
                        audit,
                        logical_payload,
                    )
                    .await
                    .unwrap();
                let patchbay_core::storage::SpawnClaimDedupOutcome::Appended(committed) = committed
                else {
                    panic!("claim fixture unexpectedly deduplicated");
                };
                assert_eq!(committed.audit_event_id, event_id(expected_lsn + 1));
                expected_lsn += 1;
                committed.source_event_id
            } else if payload.kind == StoredEventKind::Observation as i32
                && Observation::decode(payload.payload.as_slice())
                    .ok()
                    .is_some_and(|observation| observation.kind == ObservationKind::Result as i32)
            {
                let observation =
                    Observation::decode(payload.payload.as_slice()).expect("Result fixture");
                let mut audit = AuditRecordDraft::new(
                    Timestamp {
                        seconds: 10,
                        nanos: 0,
                    },
                    AuditEventKind::CommandRunning,
                );
                audit.command_id = Some(command.clone());
                audit.target_scope = observation.target_scope.clone();
                audit.reason_code = "spawn_completion_deferred".to_owned();
                let committed = storage
                    .append_spawn_result_deferred_audited(&domain, observation, audit)
                    .await
                    .unwrap();
                assert_eq!(committed.audit_event_id, event_id(expected_lsn + 1));
                expected_lsn += 1;
                committed.source_event_id
            } else {
                storage.append(&domain, payload).await.unwrap()
            };
            assert_eq!(appended, event_id(source_lsn));
            expected_lsn += 1;
        }
        let live = ProjectionState::rebuild(&storage, &domain).await.unwrap();
        assert_eq!(live.current_lsn().await, 13);
        assert_eq!(live.current_runtime_session_count().await, 0);

        let timestamp = Timestamp {
            seconds: 10,
            nanos: 0,
        };
        let promotion = SpawnPromotionCommitted {
            authority_domain_id: Some(domain.clone()),
            accepted_claim_event_id: Some(event_id(4)),
            accepted_claim: Some(accepted_claim),
            lifecycle: vec![
                SpawnPromotionLifecycleEvidence {
                    event_id: Some(event_id(6)),
                    transition: Some(transition(
                        OperationState::Accepted,
                        OperationState::Delivered,
                    )),
                },
                SpawnPromotionLifecycleEvidence {
                    event_id: Some(event_id(7)),
                    transition: Some(transition(
                        OperationState::Delivered,
                        OperationState::Running,
                    )),
                },
            ],
            successful_result: Some(SpawnPromotionResultEvidence {
                event_id: Some(event_id(11)),
                command_id: Some(command.clone()),
                target_scope: operation.target_scope,
                failure_code: FailureCode::Unspecified as i32,
                observed_at: Some(timestamp),
            }),
            staged_successor: Some(SpawnPromotionStagedEvidence {
                event_id: Some(event_id(10)),
                staged: Some(staged),
            }),
            promoted_runtime: Some(promoted),
            external_runtime_reservation: Some(external.clone()),
            authority: Some(SpawnPromotionAuthorityEvidence {
                spawning_grant_id: Some(GrantId {
                    value: "spawn-grant".to_owned(),
                }),
                continuation_authority: None,
                descendant_grant: Some(DescendantGrant {
                    grant_id: Some(GrantId {
                        value: "desc:authority-main:spawn-a".to_owned(),
                    }),
                    authority_domain_id: Some(domain.clone()),
                    subject_actor_id: Some(ActorId {
                        value: "operator".to_owned(),
                    }),
                    subject_endpoint_id: Some(EndpointId {
                        value: "web".to_owned(),
                    }),
                    target_scope: Some(TargetScope {
                        kind: TargetScopeKind::RuntimeSession as i32,
                        adapter_id: external.adapter_id,
                        deployment_scope: external.deployment_scope,
                        runtime_session_id: external.runtime_session_id,
                        session_generation: external.generation,
                        ..TargetScope::default()
                    }),
                    allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS
                        .iter()
                        .map(|kind| *kind as i32)
                        .collect(),
                    provenance: Some(DescendantGrantProvenance {
                        spawn_operation_id: Some(command.clone()),
                        spawning_grant_id: Some(GrantId {
                            value: "spawn-grant".to_owned(),
                        }),
                        continuation_authority: None,
                    }),
                    created_at: Some(timestamp),
                    revocation_policy: GrantRevocationPolicy::Continue as i32,
                    ..DescendantGrant::default()
                }),
            }),
            committed_at: Some(timestamp),
            ..SpawnPromotionCommitted::default()
        };
        let mut audit = AuditRecordDraft::new(timestamp, AuditEventKind::CommandCompleted);
        audit.command_id = Some(command.clone());
        audit.reason_code = "spawn_completion".to_owned();
        storage
            .append_spawn_promotion_audited(&domain, promotion, audit)
            .await
            .unwrap();

        live.catch_up(&storage, &domain).await.unwrap();
        assert_eq!(live.current_lsn().await, 15);
        assert_eq!(live.current_runtime_session_count().await, 1);
        assert!(live
            .grant_check
            .inner
            .lock()
            .await
            .get_grant(&GrantId {
                value: "desc:authority-main:spawn-a".to_owned(),
            })
            .is_some());
        assert_eq!(
            live.conformance_spawn_claim_registry()
                .await
                .claim_for_operation(&command)
                .unwrap()
                .disposition,
            SpawnClaimDisposition::Promoted
        );
        assert_eq!(
            live.state_lookup
                .inner
                .lock()
                .await
                .get_command(&command)
                .unwrap()
                .state,
            OperationState::Completed
        );

        let restarted = ProjectionState::rebuild(&storage, &domain).await.unwrap();
        assert_eq!(restarted.current_lsn().await, 15);
        assert_eq!(restarted.current_runtime_session_count().await, 1);
        assert!(restarted
            .grant_check
            .inner
            .lock()
            .await
            .get_grant(&GrantId {
                value: "desc:authority-main:spawn-a".to_owned(),
            })
            .is_some());
        assert_eq!(
            restarted
                .conformance_spawn_claim_registry()
                .await
                .claim_for_operation(&command)
                .unwrap()
                .disposition,
            SpawnClaimDisposition::Promoted
        );
        assert_eq!(
            restarted
                .state_lookup
                .inner
                .lock()
                .await
                .get_command(&command)
                .unwrap()
                .state,
            OperationState::Completed
        );
    }

    #[tokio::test]
    async fn resource_snapshot_is_stable_ordered_and_restart_equivalent() {
        use patchbay_contracts::patchbay::{
            resource_state_mutation, AdapterId, AdapterSnapshotSupport, Generation, Lsn,
            PayloadContentType, PayloadEnvelope, ResourceId,
            ResourceIdentity as WireResourceIdentity, ResourceKind, ResourceStateEvent,
            ResourceStateMutation, ResourceStateTombstone, ResourceStateUpsert,
            ResourceViewStateUpdate,
        };
        use prost_types::Timestamp;

        let domain = AuthorityDomainId {
            value: "authority-main".into(),
        };
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let identity = |id: &str| WireResourceIdentity {
            adapter_id: Some(AdapterId {
                value: "adapter-a".into(),
            }),
            resource_kind: Some(ResourceKind {
                value: "pool".into(),
            }),
            resource_id: Some(ResourceId { value: id.into() }),
        };
        let upsert = |id: &str, from: Option<u64>| ResourceStateMutation {
            identity: Some(identity(id)),
            from_revision_lsn: from.map(|value| Lsn { value }),
            mutation: Some(resource_state_mutation::Mutation::Upsert(
                ResourceStateUpsert {
                    resource_payload: Some(PayloadEnvelope {
                        payload: vec![1],
                        content_type: PayloadContentType::Protobuf as i32,
                        schema_ref: "pool.payload.v1".into(),
                    }),
                    projection_payload: Some(PayloadEnvelope {
                        payload: vec![2],
                        content_type: PayloadContentType::Json as i32,
                        schema_ref: "pool.projection.v1".into(),
                    }),
                },
            )),
        };
        let event = |mutations| ResourceStateEvent {
            authority_domain_id: Some(domain.clone()),
            source_adapter_id: Some(AdapterId {
                value: "adapter-a".into(),
            }),
            source_adapter_generation: Some(Generation { value: 2 }),
            views: vec![ResourceViewStateUpdate {
                resource_kind: Some(ResourceKind {
                    value: "pool".into(),
                }),
                completeness: AdapterSnapshotSupport::Authoritative as i32,
            }],
            mutations,
            observed_at: Some(Timestamp {
                seconds: 100,
                nanos: 0,
            }),
        };
        storage
            .append(
                &domain,
                patchbay_core::resource::events::encode(&event(vec![
                    upsert("z", None),
                    upsert("a", None),
                ])),
            )
            .await
            .unwrap();
        storage
            .append(
                &domain,
                patchbay_core::resource::events::encode(&event(vec![
                    ResourceStateMutation {
                        identity: Some(identity("z")),
                        from_revision_lsn: Some(Lsn { value: 1 }),
                        mutation: Some(resource_state_mutation::Mutation::Tombstone(
                            ResourceStateTombstone {
                                replaced_by: Some(identity("m")),
                            },
                        )),
                    },
                    upsert("m", None),
                ])),
            )
            .await
            .unwrap();

        let first = ProjectionState::rebuild(&storage, &domain).await.unwrap();
        let snapshot = first
            .materialize_resource_snapshot(
                domain.clone(),
                Timestamp {
                    seconds: 200,
                    nanos: 0,
                },
            )
            .await;
        let ids: Vec<_> = snapshot
            .resources
            .iter()
            .map(|resource| {
                resource
                    .identity
                    .as_ref()
                    .unwrap()
                    .resource_id
                    .as_ref()
                    .unwrap()
                    .value
                    .as_str()
            })
            .collect();
        assert_eq!(ids, ["a", "m", "z"]);
        let retired = snapshot
            .resources
            .iter()
            .find(|resource| {
                resource
                    .identity
                    .as_ref()
                    .unwrap()
                    .resource_id
                    .as_ref()
                    .unwrap()
                    .value
                    == "z"
            })
            .unwrap();
        assert!(retired.tombstoned);
        assert_eq!(retired.tombstoned_at_lsn, Some(Lsn { value: 2 }));
        assert_eq!(
            retired
                .replaced_by
                .as_ref()
                .unwrap()
                .resource_id
                .as_ref()
                .unwrap()
                .value,
            "m"
        );
        assert_eq!(snapshot.snapshot_lsn, Some(Lsn { value: 2 }));
        assert_eq!(
            snapshot.view_revisions[0].revision_lsn,
            Some(Lsn { value: 2 })
        );

        let restarted = ProjectionState::rebuild(&storage, &domain).await.unwrap();
        let after_restart = restarted
            .materialize_resource_snapshot(
                domain,
                Timestamp {
                    seconds: 201,
                    nanos: 0,
                },
            )
            .await;
        assert_eq!(snapshot.resources, after_restart.resources);
        assert_eq!(snapshot.view_revisions, after_restart.view_revisions);
        assert_eq!(snapshot.snapshot_lsn, after_restart.snapshot_lsn);
    }

    #[tokio::test]
    async fn resource_snapshot_preserves_unknown_tombstone_without_payload() {
        use patchbay_contracts::patchbay::{
            resource_state_mutation, AdapterId, AdapterSnapshotSupport, Generation, Lsn,
            ResourceFreshnessState, ResourceId, ResourceIdentity as WireResourceIdentity,
            ResourceKind, ResourceStateEvent, ResourceStateMutation, ResourceStateTombstone,
            ResourceStateUnknown, ResourceViewStateUpdate,
        };
        use prost_types::Timestamp;

        let domain = AuthorityDomainId {
            value: "authority-main".into(),
        };
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let identity = WireResourceIdentity {
            adapter_id: Some(AdapterId {
                value: "adapter-a".into(),
            }),
            resource_kind: Some(ResourceKind {
                value: "pool".into(),
            }),
            resource_id: Some(ResourceId {
                value: "unknown-pool".into(),
            }),
        };
        let event = |mutation| ResourceStateEvent {
            authority_domain_id: Some(domain.clone()),
            source_adapter_id: Some(AdapterId {
                value: "adapter-a".into(),
            }),
            source_adapter_generation: Some(Generation { value: 1 }),
            views: vec![ResourceViewStateUpdate {
                resource_kind: Some(ResourceKind {
                    value: "pool".into(),
                }),
                completeness: AdapterSnapshotSupport::Authoritative as i32,
            }],
            mutations: vec![mutation],
            observed_at: Some(Timestamp {
                seconds: 100,
                nanos: 0,
            }),
        };
        storage
            .append(
                &domain,
                patchbay_core::resource::events::encode(&event(ResourceStateMutation {
                    identity: Some(identity.clone()),
                    from_revision_lsn: None,
                    mutation: Some(resource_state_mutation::Mutation::Unknown(
                        ResourceStateUnknown {},
                    )),
                })),
            )
            .await
            .unwrap();
        storage
            .append(
                &domain,
                patchbay_core::resource::events::encode(&event(ResourceStateMutation {
                    identity: Some(identity),
                    from_revision_lsn: Some(Lsn { value: 1 }),
                    mutation: Some(resource_state_mutation::Mutation::Tombstone(
                        ResourceStateTombstone { replaced_by: None },
                    )),
                })),
            )
            .await
            .unwrap();

        let state = ProjectionState::rebuild(&storage, &domain).await.unwrap();
        let snapshot = state
            .materialize_resource_snapshot(
                domain,
                Timestamp {
                    seconds: 200,
                    nanos: 0,
                },
            )
            .await;
        let resource = snapshot
            .resources
            .first()
            .expect("unknown resource snapshot");
        assert!(resource.tombstoned);
        assert_eq!(resource.freshness, ResourceFreshnessState::Unknown as i32);
        assert!(resource.resource_payload.is_none());
        assert!(resource.projection_payload.is_none());
    }

    #[tokio::test]
    async fn fold_lag_invariant_exposes_contract_only_after_storage_catch_up() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let elicitation_id = ElicitationId {
            value: "elicitation-fold-lag".to_owned(),
        };
        let missing_responder_id = ElicitationId {
            value: "elicitation-missing-responder".to_owned(),
        };
        let expected_responder_actor = ActorId {
            value: "operator-primary".to_owned(),
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

        for id in [&elicitation_id, &missing_responder_id] {
            assert!(state
                .elicitation_contract_lookup()
                .active_contract(id)
                .await
                .is_none());
        }

        for (id, expected_actor) in [
            (
                elicitation_id.clone(),
                Some(expected_responder_actor.clone()),
            ),
            (missing_responder_id.clone(), None),
        ] {
            storage
                .append(
                    &authority_domain_id,
                    StoredEventPayload {
                        kind: StoredEventKind::Elicitation as i32,
                        payload: Elicitation {
                            elicitation_id: Some(id),
                            authority_domain_id: Some(authority_domain_id.clone()),
                            expected_responder_actor: expected_actor,
                            response_contract: Some(contract.clone()),
                            state: ElicitationState::Opened as i32,
                            ..Elicitation::default()
                        }
                        .encode_to_vec(),
                    },
                )
                .await
                .unwrap();
        }

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
        assert_eq!(
            active.expected_responder_actor,
            Some(expected_responder_actor.clone())
        );
        assert!(!active.is_terminal);
        assert_eq!(
            state
                .elicitation_contract_lookup()
                .active_contract(&missing_responder_id)
                .await
                .expect("missing responder remains explicit in active context")
                .expected_responder_actor,
            None
        );

        let restarted = ProjectionState::rebuild(&storage, &authority_domain_id)
            .await
            .unwrap();
        assert_eq!(
            restarted
                .elicitation_contract_lookup()
                .active_contract(&elicitation_id)
                .await
                .expect("restart rebuild restores active responder context")
                .expected_responder_actor,
            Some(expected_responder_actor)
        );
        assert_eq!(
            restarted
                .elicitation_contract_lookup()
                .active_contract(&missing_responder_id)
                .await
                .expect("restart rebuild preserves absent responder evidence")
                .expected_responder_actor,
            None
        );
    }

    #[derive(Debug, Clone, PartialEq)]
    struct AggregateProjectionSnapshot {
        authority: AuthorityRegistry,
        targets: TargetRegistry,
        commands: CommandIndex,
        elicitations: ElicitationSlotLayer,
        diagnostics: DiagnosticsProjection,
        security: SecurityPostureProjection,
        operators: OperatorRegistry,
        cursor: u64,
    }

    async fn aggregate_projection_snapshot(state: &ProjectionState) -> AggregateProjectionSnapshot {
        AggregateProjectionSnapshot {
            authority: state.grant_check.inner.lock().await.clone(),
            targets: state.target_resolver.inner.lock().await.clone(),
            commands: state.state_lookup.inner.lock().await.clone(),
            elicitations: state.elicitation_slots.inner.lock().await.clone(),
            diagnostics: state.diagnostics.lock().await.clone(),
            security: state.security_posture.inner.lock().await.clone(),
            operators: state.operators.lock().await.clone(),
            cursor: *state.last_applied_lsn.lock().await,
        }
    }

    #[tokio::test]
    async fn catch_up_failure_preserves_the_exact_aggregate_projection() {
        use patchbay_contracts::patchbay::{
            AcceptedOperation, AdapterId, GrantProvenance, GrantRevocationEffect,
            GrantRevocationPolicy, Operation, OperationState, Revocation, RuntimeSessionId,
        };

        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let grant_id = GrantId {
            value: "grant-atomic".to_owned(),
        };
        storage
            .append(
                &authority_domain_id,
                patchbay_core::authority::events::grant(
                    authority_domain_id.clone(),
                    Grant {
                        grant_id: Some(grant_id.clone()),
                        subject_actor_id: Some(ActorId {
                            value: "operator".to_owned(),
                        }),
                        target_scope: Some(TargetScope {
                            kind: TargetScopeKind::AuthorityDomain as i32,
                            ..TargetScope::default()
                        }),
                        allowed_operation_kinds: vec![OperationKind::Instruct as i32],
                        provenance: Some(GrantProvenance {
                            reason: "atomic replay fixture".to_owned(),
                            ..GrantProvenance::default()
                        }),
                        revocation_policy: GrantRevocationPolicy::Continue as i32,
                        ..Grant::default()
                    },
                ),
            )
            .await
            .unwrap();
        for command in ["command-1", "command-2"] {
            let operation = Operation {
                command_id: Some(CommandId {
                    value: command.to_owned(),
                }),
                authority_domain_id: Some(authority_domain_id.clone()),
                kind: OperationKind::Instruct as i32,
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::RuntimeSession as i32,
                    adapter_id: Some(AdapterId {
                        value: "pi".to_owned(),
                    }),
                    runtime_session_id: Some(RuntimeSessionId {
                        value: "session-1".to_owned(),
                    }),
                    deployment_scope: "local".to_owned(),
                    ..TargetScope::default()
                }),
                idempotency_key: format!("key-{command}"),
                ..Operation::default()
            };
            storage
                .append(
                    &authority_domain_id,
                    StoredEventPayload {
                        kind: StoredEventKind::Operation as i32,
                        payload: AcceptedOperation {
                            operation: Some(operation),
                            authorizing_grant_id: Some(grant_id.clone()),
                        }
                        .encode_to_vec(),
                    },
                )
                .await
                .unwrap();
        }

        let state = ProjectionState::rebuild(&storage, &authority_domain_id)
            .await
            .unwrap();
        let before = aggregate_projection_snapshot(&state).await;
        let operator_sessions_before = state.operator_sessions.staged_clone().await;
        assert_eq!(before.cursor, 3);

        storage
            .append(
                &authority_domain_id,
                patchbay_core::authority::events::revocation(
                    authority_domain_id.clone(),
                    Revocation {
                        grant_id: Some(grant_id),
                        revocation_generation: Some(Generation { value: 1 }),
                        accepted_operation_policy: GrantRevocationPolicy::Cancel as i32,
                        command_effects: vec![
                            GrantRevocationEffect {
                                command_id: Some(CommandId {
                                    value: "command-1".to_owned(),
                                }),
                                from_state: OperationState::Accepted as i32,
                                to_state: OperationState::Cancelled as i32,
                                failure_code: patchbay_contracts::patchbay::FailureCode::Cancelled
                                    as i32,
                            },
                            GrantRevocationEffect {
                                command_id: Some(CommandId {
                                    value: "missing-command".to_owned(),
                                }),
                                from_state: OperationState::Accepted as i32,
                                to_state: OperationState::Cancelled as i32,
                                failure_code: patchbay_contracts::patchbay::FailureCode::Cancelled
                                    as i32,
                            },
                        ],
                        ..Revocation::default()
                    },
                ),
            )
            .await
            .unwrap();

        state
            .catch_up(&storage, &authority_domain_id)
            .await
            .expect_err("a later invalid effect rejects the aggregate event");
        assert_eq!(aggregate_projection_snapshot(&state).await, before);
        assert!(
            state
                .operator_sessions
                .equivalent_to(&operator_sessions_before)
                .await
        );
    }

    #[tokio::test]
    async fn catch_up_abort_under_publication_contention_preserves_the_aggregate() {
        use patchbay_contracts::patchbay::{GrantProvenance, GrantRevocationPolicy};
        use tokio::sync::oneshot;

        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let state = ProjectionState::rebuild(&storage, &authority_domain_id)
            .await
            .unwrap();
        let before = aggregate_projection_snapshot(&state).await;
        let operator_sessions_before = state.operator_sessions.staged_clone().await;

        storage
            .append(
                &authority_domain_id,
                patchbay_core::authority::events::grant(
                    authority_domain_id.clone(),
                    Grant {
                        grant_id: Some(GrantId {
                            value: "grant-cancellation-safe".to_owned(),
                        }),
                        subject_actor_id: Some(ActorId {
                            value: "operator".to_owned(),
                        }),
                        target_scope: Some(TargetScope {
                            kind: TargetScopeKind::AuthorityDomain as i32,
                            ..TargetScope::default()
                        }),
                        allowed_operation_kinds: vec![OperationKind::Instruct as i32],
                        provenance: Some(GrantProvenance {
                            reason: "cancellation-safe publication fixture".to_owned(),
                            ..GrantProvenance::default()
                        }),
                        revocation_policy: GrantRevocationPolicy::Continue as i32,
                        ..Grant::default()
                    },
                ),
            )
            .await
            .unwrap();

        let (staged_tx, staged_rx) = oneshot::channel();
        let (publish_tx, publish_rx) = oneshot::channel();
        let task_state = state.clone();
        let task_storage = storage.clone();
        let task_domain = authority_domain_id.clone();
        let catch_up = tokio::spawn(async move {
            task_state
                .catch_up_with_before_publish(&task_storage, &task_domain, async move {
                    staged_tx.send(()).expect("test waits for staged catch-up");
                    publish_rx.await.expect("test releases publication");
                })
                .await
        });
        staged_rx.await.expect("catch-up reaches publication");

        // The target lock is later than authority in the publication order.
        // The old per-assignment locking published the staged grant and then
        // suspended here; abort left that grant visible while the cursor stayed
        // at zero. The corrected path holds authority without mutating it.
        let blocked_target = state.target_resolver.inner.lock().await;
        publish_tx
            .send(())
            .expect("catch-up still waits to publish");
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                match state.grant_check.inner.try_lock() {
                    Ok(authority) => {
                        let already_published = *authority != before.authority;
                        drop(authority);
                        if already_published {
                            break;
                        }
                    }
                    Err(_) => break,
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("catch-up reaches the contended publication boundary");

        catch_up.abort();
        let cancellation = catch_up
            .await
            .expect_err("catch-up must be cancelled while publication is blocked");
        assert!(cancellation.is_cancelled());
        drop(blocked_target);

        assert_eq!(aggregate_projection_snapshot(&state).await, before);
        assert!(
            state
                .operator_sessions
                .equivalent_to(&operator_sessions_before)
                .await
        );
    }

    #[derive(Clone, Default)]
    struct ScriptedReplayStorage {
        events: Arc<Mutex<Vec<RecordedEvent>>>,
    }

    impl ScriptedReplayStorage {
        fn new(events: Vec<RecordedEvent>) -> Self {
            Self {
                events: Arc::new(Mutex::new(events)),
            }
        }

        async fn push(&self, event: RecordedEvent) {
            self.events.lock().await.push(event);
        }
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
            _target: &patchbay_core::storage::TargetKey,
            _payload: StoredEventPayload,
        ) -> Result<patchbay_core::storage::DedupOutcome, StorageError> {
            Err(StorageError::UnsupportedOperation)
        }

        async fn read_after(
            &self,
            _authority_domain_id: &AuthorityDomainId,
            cursor: Lsn,
        ) -> Result<Vec<RecordedEvent>, StorageError> {
            Ok(self
                .events
                .lock()
                .await
                .iter()
                .filter(|event| {
                    event
                        .event_id
                        .lsn
                        .as_ref()
                        .is_none_or(|lsn| lsn.value > cursor.value)
                })
                .cloned()
                .collect())
        }

        async fn read_through(
            &self,
            _authority_domain_id: &AuthorityDomainId,
            _cursor: Lsn,
            _as_of_lsn: Lsn,
        ) -> Result<Vec<RecordedEvent>, StorageError> {
            // Deliberately faulty bounded port: diagnostics_at must enforce
            // both record framing and its trusted final bound itself.
            Ok(self.events.lock().await.clone())
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
        ) -> Result<Option<patchbay_core::storage::StoredSnapshot>, StorageError> {
            Ok(None)
        }
    }

    impl CoreGenerationStore for ScriptedReplayStorage {
        async fn load_or_create_core_generation(
            &self,
            _authority_domain_id: &AuthorityDomainId,
            _candidate: Generation,
        ) -> Result<Generation, StorageError> {
            Ok(Generation { value: 1 })
        }
    }

    fn replay_event(
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

    fn harmless_observation(authority_domain_id: &AuthorityDomainId, lsn: u64) -> RecordedEvent {
        replay_event(
            authority_domain_id,
            lsn,
            StoredEventKind::Observation,
            patchbay_contracts::patchbay::Observation::default().encode_to_vec(),
        )
    }

    fn valid_elicitation(authority_domain_id: &AuthorityDomainId, lsn: u64) -> RecordedEvent {
        replay_event(
            authority_domain_id,
            lsn,
            StoredEventKind::Elicitation,
            Elicitation {
                elicitation_id: Some(ElicitationId {
                    value: format!("elicitation-{lsn}"),
                }),
                authority_domain_id: Some(authority_domain_id.clone()),
                state: ElicitationState::Opened as i32,
                ..Elicitation::default()
            }
            .encode_to_vec(),
        )
    }

    #[tokio::test]
    async fn replay_integrity_startup_rejects_gap_and_unspecified() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        for events in [
            vec![harmless_observation(&authority_domain_id, 2)],
            vec![replay_event(
                &authority_domain_id,
                1,
                StoredEventKind::Unspecified,
                Vec::new(),
            )],
        ] {
            let error =
                ProjectionState::rebuild(&ScriptedReplayStorage::new(events), &authority_domain_id)
                    .await
                    .err()
                    .expect("corrupt aggregate replay must fail before construction");
            assert!(error.contains("corrupt replay"));
        }
    }

    #[tokio::test]
    async fn replay_integrity_catch_up_preserves_aggregate_on_validation_or_fold_failure() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let first = harmless_observation(&authority_domain_id, 1);

        for corrupt in [
            harmless_observation(&authority_domain_id, 3),
            replay_event(
                &authority_domain_id,
                2,
                StoredEventKind::Unspecified,
                Vec::new(),
            ),
            replay_event(
                &authority_domain_id,
                2,
                StoredEventKind::Elicitation,
                vec![0xff],
            ),
        ] {
            let storage = ScriptedReplayStorage::new(vec![first.clone()]);
            let state = ProjectionState::rebuild(&storage, &authority_domain_id)
                .await
                .unwrap();
            let before = aggregate_projection_snapshot(&state).await;
            let operator_sessions_before = state.operator_sessions.staged_clone().await;
            storage.push(corrupt).await;

            state
                .catch_up(&storage, &authority_domain_id)
                .await
                .expect_err("corrupt catch-up must fail closed");
            assert_eq!(aggregate_projection_snapshot(&state).await, before);
            assert!(
                state
                    .operator_sessions
                    .equivalent_to(&operator_sessions_before)
                    .await
            );
        }

        // A distinct append-only fixture proves valid catch-up without ever
        // replacing bytes at an already committed LSN.
        let storage = ScriptedReplayStorage::new(vec![first]);
        let state = ProjectionState::rebuild(&storage, &authority_domain_id)
            .await
            .unwrap();
        storage
            .push(valid_elicitation(&authority_domain_id, 2))
            .await;
        state
            .catch_up(&storage, &authority_domain_id)
            .await
            .unwrap();
        assert_eq!(state.current_lsn().await, 2);
    }

    #[tokio::test]
    async fn replay_integrity_as_of_diagnostics_rejects_gap_and_unspecified() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let first = harmless_observation(&authority_domain_id, 1);

        for (corrupt, as_of_lsn) in [
            (harmless_observation(&authority_domain_id, 3), 3),
            (
                replay_event(
                    &authority_domain_id,
                    2,
                    StoredEventKind::Unspecified,
                    Vec::new(),
                ),
                2,
            ),
        ] {
            let storage = ScriptedReplayStorage::new(vec![first.clone()]);
            let state = ProjectionState::rebuild(&storage, &authority_domain_id)
                .await
                .unwrap();
            storage.push(corrupt).await;
            let error = state
                .diagnostics_at(&storage, &authority_domain_id, as_of_lsn)
                .await
                .expect_err("as-of projection must reject corrupt complete prefix");
            assert!(error.to_string().contains("corrupt replay"));
        }
    }

    #[tokio::test]
    async fn diagnostics_at_requires_the_exact_requested_final_bound() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };

        let empty = ScriptedReplayStorage::default();
        let empty_state = ProjectionState::rebuild(&empty, &authority_domain_id)
            .await
            .unwrap();
        assert!(empty_state
            .diagnostics_at(&empty, &authority_domain_id, 1)
            .await
            .is_err());

        let truncated =
            ScriptedReplayStorage::new(vec![harmless_observation(&authority_domain_id, 1)]);
        let truncated_state = ProjectionState::rebuild(&truncated, &authority_domain_id)
            .await
            .unwrap();
        assert!(truncated_state
            .diagnostics_at(&truncated, &authority_domain_id, 2)
            .await
            .is_err());

        let over_bound = ScriptedReplayStorage::new(vec![
            harmless_observation(&authority_domain_id, 1),
            valid_elicitation(&authority_domain_id, 2),
            harmless_observation(&authority_domain_id, 3),
        ]);
        let over_bound_state = ProjectionState::rebuild(&over_bound, &authority_domain_id)
            .await
            .unwrap();
        let error = over_bound_state
            .diagnostics_at(&over_bound, &authority_domain_id, 2)
            .await
            .expect_err("a faulty port must not return rows beyond the trusted bound");
        assert!(error.to_string().contains("beyond requested LSN 2"));

        let missing_lsn = ScriptedReplayStorage::default();
        let missing_lsn_state = ProjectionState::rebuild(&missing_lsn, &authority_domain_id)
            .await
            .unwrap();
        let mut malformed = harmless_observation(&authority_domain_id, 1);
        malformed.event_id.lsn = None;
        missing_lsn.push(malformed).await;
        let error = missing_lsn_state
            .diagnostics_at(&missing_lsn, &authority_domain_id, 1)
            .await
            .expect_err("missing LSN framing must be rejected, not filtered");
        assert!(error.to_string().contains("no LSN"));

        let exact = ScriptedReplayStorage::new(vec![
            harmless_observation(&authority_domain_id, 1),
            valid_elicitation(&authority_domain_id, 2),
        ]);
        let exact_state = ProjectionState::rebuild(&exact, &authority_domain_id)
            .await
            .unwrap();
        exact_state
            .diagnostics_at(&exact, &authority_domain_id, 2)
            .await
            .expect("an exact contiguous bounded prefix remains valid");
    }

    #[tokio::test]
    async fn replay_integrity_accepts_valid_contiguous_mixed_kind_prefix() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let storage = ScriptedReplayStorage::new(vec![
            harmless_observation(&authority_domain_id, 1),
            valid_elicitation(&authority_domain_id, 2),
        ]);
        let state = ProjectionState::rebuild(&storage, &authority_domain_id)
            .await
            .expect("known sibling kinds in a complete prefix remain valid");
        assert_eq!(state.current_lsn().await, 2);
    }
}
