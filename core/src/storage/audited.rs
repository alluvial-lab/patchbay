//! Production storage decorator that couples security-relevant source events
//! to their typed audit records in the underlying writer transaction.
//!
//! The domain writers intentionally keep their narrow storage APIs. The
//! composition root installs this decorator for the running core, so every
//! ordinary source append made by acceptance, authority, session, and adapter
//! code is upgraded to `append_audited`/`append_dedup_audited` without a
//! second, non-atomic audit write path.

use patchbay_contracts::patchbay::{
    AcceptedOperation, AuditEventKind, AuthorityDomainId, CommandTransition, FailureCode, Grant,
    Observation, ObservationKind, OperationKind, OperatorRecord, Revocation,
    SecurityLockdownEvent, StoredEventKind, StoredEventPayload,
};
use prost::Message;

use super::{
    AuditPageSpec, AuditRecordDraft, AuditedAppend, AuditedDecisionAppend, AuditedDedupOutcome, DedupOutcome,
    RecordedEvent, Storage, StorageError, StoredSnapshot, TargetKey,
};
use crate::time::{Clock, SystemClock};

/// A cloneable production storage view with mandatory source/audit coupling.
#[derive(Clone)]
pub struct AuditedStorage<S> {
    inner: S,
}

impl<S> AuditedStorage<S> {
    #[must_use]
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    #[must_use]
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

/// Construct the allowlisted audit draft for a source event.
///
/// The source payload is the only attribution input here. Operation sender
/// identity has already been replaced with the verified ingress identity by
/// the acceptance boundary; no caller-provided labels or payload metadata are
/// consulted.
pub fn audit_draft_for_source(
    payload: &StoredEventPayload,
) -> Result<AuditRecordDraft, StorageError> {
    let kind = StoredEventKind::try_from(payload.kind).map_err(|_| StorageError::InvalidEventKind)?;
    let now = SystemClock.now();
    let mut draft = AuditRecordDraft::new(now, AuditEventKind::CommandSubmissionAccepted);

    match kind {
        StoredEventKind::Operation => {
            let accepted = AcceptedOperation::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode accepted operation for audit: {error}"))
            })?;
            let operation = accepted.operation.ok_or_else(|| {
                StorageError::CorruptRecord("accepted operation is missing operation".to_owned())
            })?;
            draft.grant_id = accepted.authorizing_grant_id;
            draft.actor_id = operation.sender.as_ref().and_then(|sender| sender.actor_id.clone());
            draft.endpoint_id = operation.sender.as_ref().and_then(|sender| sender.endpoint_id.clone());
            draft.device_id = operation.sender.as_ref().and_then(|sender| sender.device_id.clone());
            draft.command_id = operation.command_id.clone();
            draft.target_scope = operation.target_scope.clone();
            draft.reason_code = operation_kind_reason(operation.kind);
        }
        StoredEventKind::CommandTransition => {
            let transition = CommandTransition::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode command transition for audit: {error}"))
            })?;
            draft.command_id = transition.command_id.clone();
            draft.failure_code = FailureCode::try_from(transition.failure_code)
                .ok()
                .filter(|code| *code != FailureCode::Unspecified);
            draft.kind = match patchbay_contracts::patchbay::OperationState::try_from(transition.to_state).ok() {
                Some(patchbay_contracts::patchbay::OperationState::Delivered) => {
                    AuditEventKind::CommandDelivered
                }
                Some(patchbay_contracts::patchbay::OperationState::Running) => {
                    AuditEventKind::CommandRunning
                }
                Some(patchbay_contracts::patchbay::OperationState::Completed) => {
                    AuditEventKind::CommandCompleted
                }
                Some(patchbay_contracts::patchbay::OperationState::Rejected) => {
                    AuditEventKind::CommandRejected
                }
                Some(patchbay_contracts::patchbay::OperationState::Failed) => {
                    AuditEventKind::CommandFailed
                }
                Some(patchbay_contracts::patchbay::OperationState::Expired) => {
                    AuditEventKind::CommandExpired
                }
                Some(patchbay_contracts::patchbay::OperationState::Cancelled) => {
                    AuditEventKind::CommandCancelled
                }
                Some(patchbay_contracts::patchbay::OperationState::Superseded) => {
                    AuditEventKind::CommandSuperseded
                }
                _ => AuditEventKind::CommandSubmissionFailed,
            };
            draft.reason_code = "state_transition".to_owned();
        }
        StoredEventKind::Observation => {
            let observation = Observation::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode observation for audit: {error}"))
            })?;
            draft.actor_id = observation.sender.as_ref().and_then(|sender| sender.actor_id.clone());
            draft.endpoint_id = observation.sender.as_ref().and_then(|sender| sender.endpoint_id.clone());
            draft.device_id = observation.sender.as_ref().and_then(|sender| sender.device_id.clone());
            draft.target_scope = observation.target_scope.clone();
            draft.command_id = observation.correlations.iter().find_map(|correlation| {
                match correlation.r#ref.as_ref() {
                    Some(patchbay_contracts::patchbay::typed_correlation::Ref::CommandId(id)) => {
                        Some(id.clone())
                    }
                    _ => None,
                }
            });
            let schema = observation.payload.as_ref().map(|payload| payload.schema_ref.as_str());
            draft.kind = if schema == Some("patchbay.AdapterRegistration") {
                AuditEventKind::AdapterAttached
            } else if schema == Some("patchbay.adapter.DeliveryAcknowledgement.v1") {
                AuditEventKind::CommandDelivered
            } else {
                match ObservationKind::try_from(observation.kind).ok() {
                    Some(ObservationKind::Status) => AuditEventKind::CommandRunning,
                    Some(ObservationKind::Result) if observation.failure_code == FailureCode::Unspecified as i32 => {
                        AuditEventKind::CommandCompleted
                    }
                    Some(ObservationKind::Result) => AuditEventKind::CommandFailed,
                    _ => AuditEventKind::StaleEventIgnored,
                }
            };
            draft.failure_code = FailureCode::try_from(observation.failure_code)
                .ok()
                .filter(|code| *code != FailureCode::Unspecified);
        }
        StoredEventKind::Grant => {
            let grant = Grant::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode grant for audit: {error}"))
            })?;
            draft.actor_id = grant.subject_actor_id;
            draft.target_scope = grant.target_scope;
            draft.kind = AuditEventKind::GrantCreated;
            draft.reason_code = "grant_created".to_owned();
        }
        StoredEventKind::DescendantGrant => {
            let grant = patchbay_contracts::patchbay::DescendantGrant::decode(payload.payload.as_slice())
                .map_err(|error| StorageError::CorruptRecord(format!("cannot decode descendant grant for audit: {error}")))?;
            draft.actor_id = grant.subject_actor_id;
            draft.target_scope = grant.target_scope;
            draft.kind = AuditEventKind::GrantCreated;
            draft.reason_code = "descendant_grant_created".to_owned();
        }
        StoredEventKind::Revocation => {
            let revocation = Revocation::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode revocation for audit: {error}"))
            })?;
            draft.kind = AuditEventKind::GrantRevoked;
            draft.reason_code = "grant_revoked".to_owned();
            draft.failure_code = None;
            let _ = revocation;
        }
        StoredEventKind::OperatorRecord => {
            let record = OperatorRecord::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode operator record for audit: {error}"))
            })?;
            draft.actor_id = record.actor_id;
            draft.kind = AuditEventKind::BootstrapCompleted;
            draft.reason_code = "bootstrap_completed".to_owned();
        }
        StoredEventKind::ControlSurfacePrincipal => {
            let record = patchbay_contracts::patchbay::ControlSurfacePrincipalRecord::decode(
                payload.payload.as_slice(),
            )
            .map_err(|error| StorageError::CorruptRecord(format!("cannot decode principal for audit: {error}")))?;
            draft.actor_id = record.operator_actor_id;
            draft.endpoint_id = record.endpoint_id;
            draft.device_id = record.device_id;
            draft.kind = AuditEventKind::OperatorSessionCreated;
            draft.reason_code = "control_surface_enrolled".to_owned();
        }
        StoredEventKind::OperatorSessionRevocation => {
            let revocation = patchbay_contracts::patchbay::OperatorSessionRevocation::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode operator-session revocation for audit: {error}"))
            })?;
            draft.actor_id = revocation.operator_actor_id;
            draft.endpoint_id = revocation.verified_revoker.as_ref().and_then(|value| value.endpoint_id.clone());
            draft.device_id = revocation.verified_revoker.as_ref().and_then(|value| value.device_id.clone());
            draft.kind = AuditEventKind::OperatorSessionRevoked;
            draft.reason_code = revocation.reason_code;
        }
        StoredEventKind::ControlSurfaceRevocation => {
            let revocation = patchbay_contracts::patchbay::ControlSurfaceRevocation::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode control-surface revocation for audit: {error}"))
            })?;
            let revoker = revocation.verified_revoker.as_ref();
            draft.actor_id = revoker.and_then(|value| value.actor_id.clone());
            draft.endpoint_id = revoker.and_then(|value| value.endpoint_id.clone());
            draft.device_id = revoker.and_then(|value| value.device_id.clone());
            draft.kind = match revocation.target.as_ref() {
                Some(patchbay_contracts::patchbay::control_surface_revocation::Target::PrincipalId(id)) => {
                    draft.target_scope = Some(patchbay_contracts::patchbay::TargetScope {
                        kind: patchbay_contracts::patchbay::TargetScopeKind::Resource as i32,
                        resource_id: id.clone(),
                        ..Default::default()
                    });
                    AuditEventKind::ControlSurfacePrincipalRevoked
                }
                Some(patchbay_contracts::patchbay::control_surface_revocation::Target::EndpointId(id)) => {
                    draft.target_scope = Some(patchbay_contracts::patchbay::TargetScope {
                        kind: patchbay_contracts::patchbay::TargetScopeKind::Resource as i32,
                        resource_id: id.value.clone(),
                        ..Default::default()
                    });
                    AuditEventKind::ControlSurfaceEndpointRevoked
                }
                Some(patchbay_contracts::patchbay::control_surface_revocation::Target::DeviceId(id)) => {
                    draft.target_scope = Some(patchbay_contracts::patchbay::TargetScope {
                        kind: patchbay_contracts::patchbay::TargetScopeKind::Resource as i32,
                        resource_id: id.value.clone(),
                        ..Default::default()
                    });
                    AuditEventKind::ControlSurfaceDeviceRevoked
                }
                None => return Err(StorageError::CorruptRecord("control-surface revocation has no target".to_owned())),
            };
            draft.reason_code = revocation.reason_code;
        }
        StoredEventKind::SecurityLockdown => {
            let source = SecurityLockdownEvent::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode security lockdown for audit: {error}"))
            })?;
            match source.transition {
                Some(patchbay_contracts::patchbay::security_lockdown_event::Transition::Entered(entered)) => {
                    draft.kind = AuditEventKind::LockdownEntered;
                    draft.actor_id = entered.entered_by.as_ref().and_then(|value| value.actor_id.clone());
                    draft.endpoint_id = entered.entered_by.as_ref().and_then(|value| value.endpoint_id.clone());
                    draft.device_id = entered.entered_by.as_ref().and_then(|value| value.device_id.clone());
                    draft.reason_code = entered.reason_code;
                }
                Some(patchbay_contracts::patchbay::security_lockdown_event::Transition::Exited(exited)) => {
                    draft.kind = AuditEventKind::LockdownExited;
                    draft.reason_code = exited.reason_code;
                }
                None => return Err(StorageError::CorruptRecord("security lockdown has no transition".to_owned())),
            }
        }
        StoredEventKind::SessionState | StoredEventKind::Elicitation => {
            return Err(StorageError::UnsupportedOperation);
        }
        StoredEventKind::AuditRecord | StoredEventKind::Unspecified => {
            return Err(StorageError::InvalidEventKind);
        }
    }
    Ok(draft)
}

fn operation_kind_reason(kind: i32) -> String {
    OperationKind::try_from(kind)
        .ok()
        .map(|kind| format!("operation_{:?}", kind).to_ascii_lowercase())
        .unwrap_or_else(|| "operation".to_owned())
}

impl<S> Storage for AuditedStorage<S>
where
    S: Storage + Clone,
{
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        // A generic Observation envelope cannot determine whether a result is
        // a completion, a stale candidate, or merely evidence. SessionState is
        // different: it is already a domain-owned lifecycle mutation and must
        // never bypass the paired writer boundary.
        let kind = StoredEventKind::try_from(payload.kind).map_err(|_| StorageError::InvalidEventKind)?;
        if kind == StoredEventKind::SessionState {
            let mut audit = AuditRecordDraft::new(
                SystemClock.now(),
                AuditEventKind::AdapterAttached,
            );
            audit.reason_code = "session_state_changed".to_owned();
            return self
                .inner
                .append_audited(authority_domain_id, payload, audit)
                .await
                .map(|result| result.source_event_id);
        }
        // Operation acceptance and authority/bootstrap records have a
        // decision fixed by their domain writer, so their typed allowlisted
        // drafts are safe here. Observations and transitions never enter this
        // inference path.
        if matches!(
            kind,
            StoredEventKind::Operation
                | StoredEventKind::Grant
                | StoredEventKind::DescendantGrant
                | StoredEventKind::Revocation
                | StoredEventKind::OperatorRecord
                | StoredEventKind::ControlSurfacePrincipal
                | StoredEventKind::SecurityLockdown
        ) {
            let audit = audit_draft_for_source(&payload)?;
            return self
                .inner
                .append_audited(authority_domain_id, payload, audit)
                .await
                .map(|result| result.source_event_id);
        }
        self.inner.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &patchbay_contracts::patchbay::IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.append_dedup_with_payload(authority_domain_id, key, target, payload.clone(), payload.encode_to_vec()).await
    }

    async fn append_dedup_with_payload(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &patchbay_contracts::patchbay::IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
        logical_payload: Vec<u8>,
    ) -> Result<DedupOutcome, StorageError> {
        let audit = audit_draft_for_source(&payload)?;
        match self
            .inner
            .append_dedup_audited_with_payload(authority_domain_id, key, target, payload, audit, logical_payload)
            .await?
        {
            AuditedDedupOutcome::Appended(result) => Ok(DedupOutcome::Appended(result.source_event_id)),
            AuditedDedupOutcome::Duplicate { source_event_id, .. } => Ok(DedupOutcome::Duplicate(source_event_id)),
        }
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: patchbay_contracts::patchbay::Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.inner.read_after(authority_domain_id, cursor).await
    }

    async fn read_through(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: patchbay_contracts::patchbay::Lsn,
        as_of_lsn: patchbay_contracts::patchbay::Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.inner.read_through(authority_domain_id, cursor, as_of_lsn).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: patchbay_contracts::patchbay::Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.inner.write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload).await
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<patchbay_contracts::patchbay::Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        self.inner.load_latest_snapshot(authority_domain_id, at_or_before).await
    }

    async fn append_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        audit: AuditRecordDraft,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        self.inner.append_audit(authority_domain_id, audit).await
    }

    async fn append_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<AuditedAppend, StorageError> {
        self.inner.append_audited(authority_domain_id, source, audit).await
    }

    async fn append_decision(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        self.inner
            .append_audited(authority_domain_id, source, audit)
            .await
            .map(|result| result.source_event_id)
    }

    async fn append_decision_audited_many(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audits: Vec<AuditRecordDraft>,
    ) -> Result<AuditedDecisionAppend, StorageError> {
        self.inner.append_decision_audited_many(authority_domain_id, source, audits).await
    }

    async fn append_dedup_audited_with_payload(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &patchbay_contracts::patchbay::IdempotencyKey,
        target: &TargetKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
        logical_payload: Vec<u8>,
    ) -> Result<AuditedDedupOutcome, StorageError> {
        self.inner.append_dedup_audited_with_payload(authority_domain_id, key, target, source, audit, logical_payload).await
    }

    async fn append_batch_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        sources: Vec<StoredEventPayload>,
        audit: AuditRecordDraft,
    ) -> Result<super::AuditedBatchAppend, StorageError> {
        self.inner.append_batch_audited(authority_domain_id, sources, audit).await
    }

    async fn query_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        spec: AuditPageSpec,
    ) -> Result<patchbay_contracts::patchbay::AuditPage, StorageError> {
        self.inner.query_audit(authority_domain_id, spec).await
    }

    async fn query_audit_through(
        &self,
        authority_domain_id: &AuthorityDomainId,
        spec: AuditPageSpec,
        as_of_lsn: patchbay_contracts::patchbay::Lsn,
    ) -> Result<patchbay_contracts::patchbay::AuditPage, StorageError> {
        self.inner.query_audit_through(authority_domain_id, spec, as_of_lsn).await
    }
}
