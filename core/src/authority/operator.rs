//! Durable operator records, control-surface principals, and revocation fences.
//!
//! The authority-domain event log is the source of truth. Operator sessions keep
//! opaque bearer ids process-local, while their generation floor and all
//! principal/endpoint/device fences are replayed from typed source events.

use std::collections::HashMap;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use patchbay_contracts::patchbay::{
    control_surface_revocation, ActorEndpointRef, ActorId, AuthorityDomainId,
    ControlSurfacePrincipalRecord, ControlSurfaceRevocation, DeviceId, EndpointId, EventId,
    Generation, Lsn, OperatorRecord, OperatorSessionRevocation, StoredEventKind,
    StoredEventPayload, TargetScope, TargetScopeKind,
};
use prost::Message;
use prost_types::Timestamp;
use scrypt::{scrypt, Params};
use sha2::{Digest, Sha256};

use crate::storage::{AuditRecordDraft, AuditedDecisionAppend, RecordedEvent, Storage};

const PASSWORD_HASH_BYTES: usize = 64;
const PRINCIPAL_CREDENTIAL_HASH_BYTES: usize = 32;
const SCRYPT_LOG_N: u8 = 14;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum OperatorError {
    #[error("invalid operator record: {0}")]
    InvalidRecord(String),
    #[error("operator bootstrap has already completed")]
    AlreadyBootstrapped,
    #[error("operator record was not found")]
    OperatorNotFound,
    #[error("control-surface principal was not found")]
    PrincipalNotFound,
    #[error("control-surface endpoint or device was not found")]
    EndpointNotFound,
    #[error("control-surface identity is revoked: {0}")]
    RevokedIdentity(String),
    #[error("corrupt operator record: {0}")]
    CorruptRecord(String),
    #[error("corrupt operator log: {0}")]
    CorruptLog(String),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControlSurfaceRevocationTarget {
    Principal(String),
    Endpoint(EndpointId),
    Device(DeviceId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedControlSurfaceRevocation {
    pub event_id: EventId,
    pub authority_domain_id: AuthorityDomainId,
    pub target: ControlSurfaceRevocationTarget,
    pub verified_revoker: Option<ActorEndpointRef>,
    pub occurred_at: Option<Timestamp>,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedOperatorSessionRevocation {
    pub event_id: EventId,
    pub authority_domain_id: AuthorityDomainId,
    pub operator_actor_id: ActorId,
    pub invalidated_through_generation: Generation,
    pub verified_revoker: Option<ActorEndpointRef>,
    pub occurred_at: Option<Timestamp>,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationIngestResult {
    pub event_id: EventId,
    pub newly_revoked: bool,
}

/// Deterministic projection of the current operator, enrolled principals, and
/// durable control-surface fences.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OperatorRegistry {
    operator: Option<OperatorRecord>,
    principals: HashMap<String, ControlSurfacePrincipalRecord>,
    active_principal_by_endpoint: HashMap<String, String>,
    principal_revocations: HashMap<String, RecordedControlSurfaceRevocation>,
    endpoint_revocations: HashMap<String, RecordedControlSurfaceRevocation>,
    device_revocations: HashMap<String, RecordedControlSurfaceRevocation>,
    session_revocations: HashMap<String, RecordedOperatorSessionRevocation>,
}

impl OperatorRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn operator_record(&self) -> Option<&OperatorRecord> {
        self.operator.as_ref()
    }

    #[must_use]
    pub fn principal(&self, principal_id: &str) -> Option<&ControlSurfacePrincipalRecord> {
        let record = self.principals.get(principal_id)?;
        let endpoint_id = record.endpoint_id.as_ref()?;
        (self
            .active_principal_by_endpoint
            .get(&endpoint_id.value)
            .map(String::as_str)
            == Some(principal_id))
            .then_some(record)
    }

    #[must_use]
    pub fn principal_record(&self, principal_id: &str) -> Option<&ControlSurfacePrincipalRecord> {
        self.principals.get(principal_id)
    }

    /// Verify a password against the core-owned scrypt record.
    pub fn verify_password(
        &self,
        actor_id: &ActorId,
        password: &str,
    ) -> Result<bool, OperatorError> {
        let record = self
            .operator
            .as_ref()
            .ok_or(OperatorError::OperatorNotFound)?;
        if record.actor_id.as_ref() != Some(actor_id) {
            return Ok(false);
        }
        verify_password_hash(password, &record.password_hash)
    }

    /// Verify a high-entropy control-surface credential and return its bound
    /// identity. The actor/endpoint/device/generation all come from this
    /// record, never from request claims.
    #[must_use]
    pub fn verify_principal(
        &self,
        principal_id: &str,
        credential: &str,
    ) -> Option<ControlSurfacePrincipalRecord> {
        let record = self.principal(principal_id)?;
        let endpoint_id = record.endpoint_id.as_ref()?;
        let device_id = record.device_id.as_ref()?;
        if self.principal_revocations.contains_key(principal_id)
            || self.endpoint_revocations.contains_key(&endpoint_id.value)
            || self.device_revocations.contains_key(&device_id.value)
        {
            return None;
        }
        let actual = hash_principal_credential(credential);
        constant_time_eq(&actual, &record.credential_hash).then(|| record.clone())
    }

    #[must_use]
    pub fn revocation_for_principal(
        &self,
        principal_id: &str,
    ) -> Option<&RecordedControlSurfaceRevocation> {
        self.principal_revocations.get(principal_id)
    }

    #[must_use]
    pub fn revocation_for_endpoint(
        &self,
        endpoint_id: &EndpointId,
    ) -> Option<&RecordedControlSurfaceRevocation> {
        self.endpoint_revocations.get(&endpoint_id.value)
    }

    #[must_use]
    pub fn revocation_for_device(
        &self,
        device_id: &DeviceId,
    ) -> Option<&RecordedControlSurfaceRevocation> {
        self.device_revocations.get(&device_id.value)
    }

    #[must_use]
    pub fn session_revocation_for_actor(
        &self,
        actor_id: &ActorId,
    ) -> Option<&RecordedOperatorSessionRevocation> {
        self.session_revocations.get(&actor_id.value)
    }

    #[must_use]
    pub fn has_endpoint(&self, endpoint_id: &EndpointId) -> bool {
        self.principals.values().any(|principal| {
            principal.endpoint_id.as_ref() == Some(endpoint_id)
        })
    }

    #[must_use]
    pub fn has_device(&self, device_id: &DeviceId) -> bool {
        self.principals.values().any(|principal| {
            principal.device_id.as_ref() == Some(device_id)
        })
    }

    #[must_use]
    pub fn count_matching(&self, target: &ControlSurfaceRevocationTarget) -> u32 {
        self.principals
            .values()
            .filter(|principal| match target {
                ControlSurfaceRevocationTarget::Principal(id) => principal.principal_id == *id,
                ControlSurfaceRevocationTarget::Endpoint(endpoint) => {
                    principal.endpoint_id.as_ref() == Some(endpoint)
                }
                ControlSurfaceRevocationTarget::Device(device) => {
                    principal.device_id.as_ref() == Some(device)
                }
            })
            .count() as u32
    }

    /// Fold one committed event. Other projection families are ignored.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), OperatorError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            OperatorError::CorruptRecord(format!(
                "unknown stored event kind {}",
                event.payload.kind
            ))
        })?;
        match kind {
            StoredEventKind::OperatorRecord => self.observe_operator(event),
            StoredEventKind::ControlSurfacePrincipal => self.observe_principal(event),
            StoredEventKind::OperatorSessionRevocation => self.observe_session_revocation(event),
            StoredEventKind::ControlSurfaceRevocation => self.observe_control_surface_revocation(event),
            StoredEventKind::Operation
            | StoredEventKind::Observation
            | StoredEventKind::Elicitation
            | StoredEventKind::Grant
            | StoredEventKind::DescendantGrant
            | StoredEventKind::Revocation
            | StoredEventKind::SessionState
            | StoredEventKind::CommandTransition
            | StoredEventKind::AuditRecord
            | StoredEventKind::Unspecified => Ok(()),
        }
    }

    fn observe_operator(&mut self, event: &RecordedEvent) -> Result<(), OperatorError> {
        let (event_domain, lsn) = event_identity(event)?;
        let record = OperatorRecord::decode(event.payload.payload.as_slice()).map_err(|error| {
            OperatorError::CorruptRecord(format!(
                "cannot decode operator record at LSN {lsn}: {error}"
            ))
        })?;
        validate_operator_record(&record, event_domain)?;
        match self.operator.as_ref() {
            None => {
                self.operator = Some(record);
                Ok(())
            }
            Some(existing) if existing == &record => Ok(()),
            Some(_) => Err(OperatorError::CorruptLog(format!(
                "authority domain has conflicting operator records at LSN {lsn}"
            ))),
        }
    }

    fn observe_principal(&mut self, event: &RecordedEvent) -> Result<(), OperatorError> {
        let (event_domain, lsn) = event_identity(event)?;
        let record = ControlSurfacePrincipalRecord::decode(event.payload.payload.as_slice())
            .map_err(|error| {
                OperatorError::CorruptRecord(format!(
                    "cannot decode control-surface principal at LSN {lsn}: {error}"
                ))
            })?;
        validate_principal_record(&record, event_domain)?;
        let operator = self.operator.as_ref().ok_or_else(|| {
            OperatorError::CorruptLog(format!(
                "control-surface principal at LSN {lsn} precedes the operator record"
            ))
        })?;
        if record.operator_actor_id != operator.actor_id {
            return Err(OperatorError::CorruptLog(format!(
                "control-surface principal at LSN {lsn} is bound to another operator"
            )));
        }
        let endpoint_id = record
            .endpoint_id
            .as_ref()
            .expect("validated principal has endpoint");
        let device_id = record
            .device_id
            .as_ref()
            .expect("validated principal has device");
        if self.endpoint_revocations.contains_key(&endpoint_id.value) {
            return Err(OperatorError::CorruptLog(format!(
                "principal {} was enrolled at revoked endpoint {}",
                record.principal_id, endpoint_id.value
            )));
        }
        if self.device_revocations.contains_key(&device_id.value) {
            return Err(OperatorError::CorruptLog(format!(
                "principal {} was enrolled on revoked device {}",
                record.principal_id, device_id.value
            )));
        }
        if let Some(existing) = self.principals.get(&record.principal_id) {
            return if existing == &record {
                Ok(())
            } else {
                Err(OperatorError::CorruptLog(format!(
                    "principal {} has conflicting records at LSN {lsn}",
                    record.principal_id
                )))
            };
        }
        self.active_principal_by_endpoint
            .insert(endpoint_id.value.clone(), record.principal_id.clone());
        self.principals.insert(record.principal_id.clone(), record);
        Ok(())
    }

    fn observe_session_revocation(&mut self, event: &RecordedEvent) -> Result<(), OperatorError> {
        let (event_domain, lsn) = event_identity(event)?;
        let revocation = OperatorSessionRevocation::decode(event.payload.payload.as_slice())
            .map_err(|error| OperatorError::CorruptRecord(format!("cannot decode operator-session revocation at LSN {lsn}: {error}")))?;
        validate_session_revocation(&revocation, event_domain)?;
        let actor = revocation
            .operator_actor_id
            .clone()
            .expect("validated session revocation has actor");
        let generation = revocation
            .invalidated_through_generation
            .expect("validated session revocation has generation");
        let recorded = RecordedOperatorSessionRevocation {
            event_id: event.event_id.clone(),
            authority_domain_id: event_domain.clone(),
            operator_actor_id: actor.clone(),
            invalidated_through_generation: generation,
            verified_revoker: revocation.verified_revoker,
            occurred_at: revocation.occurred_at,
            reason_code: revocation.reason_code,
        };
        if let Some(existing) = self.session_revocations.get(&actor.value) {
            if existing.invalidated_through_generation.value > recorded.invalidated_through_generation.value {
                return Ok(());
            }
            if existing.invalidated_through_generation == recorded.invalidated_through_generation {
                return Ok(());
            }
        }
        self.session_revocations.insert(actor.value, recorded);
        Ok(())
    }

    fn observe_control_surface_revocation(
        &mut self,
        event: &RecordedEvent,
    ) -> Result<(), OperatorError> {
        let (event_domain, lsn) = event_identity(event)?;
        let revocation = ControlSurfaceRevocation::decode(event.payload.payload.as_slice())
            .map_err(|error| OperatorError::CorruptRecord(format!("cannot decode control-surface revocation at LSN {lsn}: {error}")))?;
        validate_control_surface_revocation(&revocation, event_domain)?;
        let target = match revocation.target.clone().expect("validated target") {
            control_surface_revocation::Target::PrincipalId(id) => {
                ControlSurfaceRevocationTarget::Principal(id)
            }
            control_surface_revocation::Target::EndpointId(id) => {
                ControlSurfaceRevocationTarget::Endpoint(id)
            }
            control_surface_revocation::Target::DeviceId(id) => {
                ControlSurfaceRevocationTarget::Device(id)
            }
        };
        let recorded = RecordedControlSurfaceRevocation {
            event_id: event.event_id.clone(),
            authority_domain_id: event_domain.clone(),
            target: target.clone(),
            verified_revoker: revocation.verified_revoker,
            occurred_at: revocation.occurred_at,
            reason_code: revocation.reason_code,
        };
        let destination = match &target {
            ControlSurfaceRevocationTarget::Principal(id) => self.principal_revocations.get_mut(id),
            ControlSurfaceRevocationTarget::Endpoint(id) => self.endpoint_revocations.get_mut(&id.value),
            ControlSurfaceRevocationTarget::Device(id) => self.device_revocations.get_mut(&id.value),
        };
        if let Some(existing) = destination {
            if existing == &recorded {
                return Ok(());
            }
            return Err(OperatorError::CorruptLog(format!(
                "control-surface revocation target was revoked twice with conflicting events at LSN {lsn}"
            )));
        }
        match target {
            ControlSurfaceRevocationTarget::Principal(id) => {
                self.principal_revocations.insert(id, recorded);
            }
            ControlSurfaceRevocationTarget::Endpoint(id) => {
                self.endpoint_revocations.insert(id.value, recorded);
            }
            ControlSurfaceRevocationTarget::Device(id) => {
                self.device_revocations.insert(id.value, recorded);
            }
        }
        Ok(())
    }
}

pub async fn ingest_operator_record<S: Storage>(
    storage: &S,
    projection: &mut OperatorRegistry,
    authority_domain_id: &AuthorityDomainId,
    mut record: OperatorRecord,
) -> Result<EventId, OperatorError> {
    if projection.operator_record().is_some() {
        return Err(OperatorError::AlreadyBootstrapped);
    }
    record.authority_domain_id = Some(authority_domain_id.clone());
    validate_operator_record(&record, authority_domain_id)?;
    append_and_warm(
        storage,
        projection,
        authority_domain_id,
        StoredEventPayload {
            kind: StoredEventKind::OperatorRecord as i32,
            payload: record.encode_to_vec(),
        },
    )
    .await
}

pub async fn ingest_control_surface_principal<S: Storage>(
    storage: &S,
    projection: &mut OperatorRegistry,
    authority_domain_id: &AuthorityDomainId,
    mut record: ControlSurfacePrincipalRecord,
) -> Result<EventId, OperatorError> {
    if projection.operator_record().is_none() {
        return Err(OperatorError::OperatorNotFound);
    }
    record.authority_domain_id = Some(authority_domain_id.clone());
    validate_principal_record(&record, authority_domain_id)?;
    let endpoint_id = record.endpoint_id.as_ref().expect("validated endpoint");
    let device_id = record.device_id.as_ref().expect("validated device");
    if projection.revocation_for_endpoint(endpoint_id).is_some() {
        return Err(OperatorError::RevokedIdentity(format!("endpoint {}", endpoint_id.value)));
    }
    if projection.revocation_for_device(device_id).is_some() {
        return Err(OperatorError::RevokedIdentity(format!("device {}", device_id.value)));
    }
    append_and_warm(
        storage,
        projection,
        authority_domain_id,
        StoredEventPayload {
            kind: StoredEventKind::ControlSurfacePrincipal as i32,
            payload: record.encode_to_vec(),
        },
    )
    .await
}

pub async fn ingest_operator_session_revocation<S: Storage>(
    storage: &S,
    projection: &mut OperatorRegistry,
    authority_domain_id: &AuthorityDomainId,
    revocation: OperatorSessionRevocation,
) -> Result<RevocationIngestResult, OperatorError> {
    validate_session_revocation(&revocation, authority_domain_id)?;
    let actor = revocation.operator_actor_id.as_ref().expect("validated actor");
    let generation = revocation
        .invalidated_through_generation
        .as_ref()
        .expect("validated generation");
    if let Some(existing) = projection.session_revocation_for_actor(actor) {
        if existing.invalidated_through_generation.value >= generation.value {
            let mut audit = session_revocation_audit(&revocation);
            audit.source_event_id = Some(existing.event_id.clone());
            storage.append_audit(authority_domain_id, audit).await?;
            return Ok(RevocationIngestResult {
                event_id: existing.event_id.clone(),
                newly_revoked: false,
            });
        }
    }
    let payload = StoredEventPayload {
        kind: StoredEventKind::OperatorSessionRevocation as i32,
        payload: revocation.encode_to_vec(),
    };
    let audit = session_revocation_audit(&revocation);
    let result = append_and_warm_audited(storage, projection, authority_domain_id, payload, audit).await?;
    Ok(RevocationIngestResult {
        event_id: result.source_event_id,
        newly_revoked: true,
    })
}

pub async fn ingest_control_surface_revocation<S: Storage>(
    storage: &S,
    projection: &mut OperatorRegistry,
    authority_domain_id: &AuthorityDomainId,
    revocation: ControlSurfaceRevocation,
) -> Result<(RevocationIngestResult, ControlSurfaceRevocationTarget), OperatorError> {
    validate_control_surface_revocation(&revocation, authority_domain_id)?;
    let target = control_surface_target(&revocation)?;
    match &target {
        ControlSurfaceRevocationTarget::Principal(id) if projection.principal_record(id).is_none() => {
            return Err(OperatorError::PrincipalNotFound);
        }
        ControlSurfaceRevocationTarget::Endpoint(id) if !projection.has_endpoint(id) => {
            return Err(OperatorError::EndpointNotFound);
        }
        ControlSurfaceRevocationTarget::Device(id) if !projection.has_device(id) => {
            return Err(OperatorError::EndpointNotFound);
        }
        _ => {}
    }
    let existing = match &target {
        ControlSurfaceRevocationTarget::Principal(id) => projection.revocation_for_principal(id),
        ControlSurfaceRevocationTarget::Endpoint(id) => projection.revocation_for_endpoint(id),
        ControlSurfaceRevocationTarget::Device(id) => projection.revocation_for_device(id),
    };
    if let Some(existing) = existing {
        let mut audit = control_surface_revocation_audit(&revocation, &target);
        audit.source_event_id = Some(existing.event_id.clone());
        storage.append_audit(authority_domain_id, audit).await?;
        return Ok((
            RevocationIngestResult {
                event_id: existing.event_id.clone(),
                newly_revoked: false,
            },
            target,
        ));
    }
    let payload = StoredEventPayload {
        kind: StoredEventKind::ControlSurfaceRevocation as i32,
        payload: revocation.encode_to_vec(),
    };
    let audit = control_surface_revocation_audit(&revocation, &target);
    let result = append_and_warm_audited(storage, projection, authority_domain_id, payload, audit).await?;
    Ok((
        RevocationIngestResult {
            event_id: result.source_event_id,
            newly_revoked: true,
        },
        target,
    ))
}

async fn append_and_warm<S: Storage>(
    storage: &S,
    projection: &mut OperatorRegistry,
    authority_domain_id: &AuthorityDomainId,
    payload: StoredEventPayload,
) -> Result<EventId, OperatorError> {
    let event_id = storage.append(authority_domain_id, payload.clone()).await?;
    validate_event_id(&event_id, authority_domain_id)?;
    projection.observe(&RecordedEvent { event_id: event_id.clone(), payload })?;
    Ok(event_id)
}

async fn append_and_warm_audited<S: Storage>(
    storage: &S,
    projection: &mut OperatorRegistry,
    authority_domain_id: &AuthorityDomainId,
    payload: StoredEventPayload,
    audit: AuditRecordDraft,
) -> Result<AuditedDecisionAppend, OperatorError> {
    let result = storage
        .append_decision_audited_many(authority_domain_id, payload.clone(), vec![audit])
        .await?;
    validate_event_id(&result.source_event_id, authority_domain_id)?;
    projection.observe(&RecordedEvent {
        event_id: result.source_event_id.clone(),
        payload,
    })?;
    Ok(result)
}

fn session_revocation_audit(revocation: &OperatorSessionRevocation) -> AuditRecordDraft {
    let mut audit = AuditRecordDraft::new(
        revocation.occurred_at.unwrap_or(Timestamp { seconds: 0, nanos: 0 }),
        patchbay_contracts::patchbay::AuditEventKind::OperatorSessionRevoked,
    );
    audit.actor_id = revocation.operator_actor_id.clone();
    audit.endpoint_id = revocation
        .verified_revoker
        .as_ref()
        .and_then(|value| value.endpoint_id.clone());
    audit.device_id = revocation
        .verified_revoker
        .as_ref()
        .and_then(|value| value.device_id.clone());
    audit.reason_code = revocation.reason_code.clone();
    audit
}

fn control_surface_revocation_audit(
    revocation: &ControlSurfaceRevocation,
    target: &ControlSurfaceRevocationTarget,
) -> AuditRecordDraft {
    let kind = match target {
        ControlSurfaceRevocationTarget::Principal(_) => {
            patchbay_contracts::patchbay::AuditEventKind::ControlSurfacePrincipalRevoked
        }
        ControlSurfaceRevocationTarget::Endpoint(_) => {
            patchbay_contracts::patchbay::AuditEventKind::ControlSurfaceEndpointRevoked
        }
        ControlSurfaceRevocationTarget::Device(_) => {
            patchbay_contracts::patchbay::AuditEventKind::ControlSurfaceDeviceRevoked
        }
    };
    let mut audit = AuditRecordDraft::new(
        revocation.occurred_at.unwrap_or(Timestamp { seconds: 0, nanos: 0 }),
        kind,
    );
    audit.actor_id = revocation
        .verified_revoker
        .as_ref()
        .and_then(|value| value.actor_id.clone());
    let revoker = revocation.verified_revoker.as_ref();
    audit.endpoint_id = revoker.and_then(|value| value.endpoint_id.clone());
    audit.device_id = revoker.and_then(|value| value.device_id.clone());
    audit.target_scope = Some(match target {
        ControlSurfaceRevocationTarget::Principal(principal_id) => TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource_id: principal_id.clone(),
            ..TargetScope::default()
        },
        ControlSurfaceRevocationTarget::Endpoint(endpoint_id) => TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource_id: endpoint_id.value.clone(),
            ..TargetScope::default()
        },
        ControlSurfaceRevocationTarget::Device(device_id) => TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource_id: device_id.value.clone(),
            ..TargetScope::default()
        },
    });
    audit.reason_code = revocation.reason_code.clone();
    audit
}

fn control_surface_target(
    revocation: &ControlSurfaceRevocation,
) -> Result<ControlSurfaceRevocationTarget, OperatorError> {
    match revocation.target.clone() {
        Some(control_surface_revocation::Target::PrincipalId(id)) if !id.is_empty() => {
            Ok(ControlSurfaceRevocationTarget::Principal(id))
        }
        Some(control_surface_revocation::Target::EndpointId(id)) if !id.value.is_empty() => {
            Ok(ControlSurfaceRevocationTarget::Endpoint(id))
        }
        Some(control_surface_revocation::Target::DeviceId(id)) if !id.value.is_empty() => {
            Ok(ControlSurfaceRevocationTarget::Device(id))
        }
        _ => Err(OperatorError::InvalidRecord(
            "control-surface revocation target is required".to_owned(),
        )),
    }
}

fn validate_session_revocation(
    revocation: &OperatorSessionRevocation,
    expected_domain: &AuthorityDomainId,
) -> Result<(), OperatorError> {
    validate_domain(revocation.authority_domain_id.as_ref(), expected_domain, "session revocation")?;
    required_non_empty(
        revocation.operator_actor_id.as_ref().map(|value| value.value.as_str()),
        "session revocation operator actor id",
    )?;
    if revocation
        .invalidated_through_generation
        .as_ref()
        .is_none_or(|generation| generation.value == 0)
    {
        return Err(OperatorError::InvalidRecord(
            "session revocation generation must be positive".to_owned(),
        ));
    }
    validate_reason_code(&revocation.reason_code)?;
    if revocation.occurred_at.is_none() {
        return Err(OperatorError::InvalidRecord(
            "session revocation is missing occurred_at".to_owned(),
        ));
    }
    Ok(())
}

fn validate_control_surface_revocation(
    revocation: &ControlSurfaceRevocation,
    expected_domain: &AuthorityDomainId,
) -> Result<(), OperatorError> {
    validate_domain(
        revocation.authority_domain_id.as_ref(),
        expected_domain,
        "control-surface revocation",
    )?;
    control_surface_target(revocation)?;
    validate_reason_code(&revocation.reason_code)?;
    if revocation.occurred_at.is_none() {
        return Err(OperatorError::InvalidRecord(
            "control-surface revocation is missing occurred_at".to_owned(),
        ));
    }
    Ok(())
}

fn validate_reason_code(reason_code: &str) -> Result<(), OperatorError> {
    if reason_code.is_empty()
        || reason_code.len() > 64
        || !reason_code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(OperatorError::InvalidRecord(
            "reason_code must match [a-z0-9_]{1,64}".to_owned(),
        ));
    }
    Ok(())
}

pub fn validate_operator_record(
    record: &OperatorRecord,
    expected_domain: &AuthorityDomainId,
) -> Result<(), OperatorError> {
    validate_domain(record.authority_domain_id.as_ref(), expected_domain, "operator record")?;
    required_non_empty(
        record.actor_id.as_ref().map(|value| value.value.as_str()),
        "operator actor id",
    )?;
    validate_password_hash(&record.password_hash)?;
    if record.created_at.is_none() {
        return Err(OperatorError::InvalidRecord(
            "operator record is missing created_at".to_owned(),
        ));
    }
    Ok(())
}

fn validate_principal_record(
    record: &ControlSurfacePrincipalRecord,
    expected_domain: &AuthorityDomainId,
) -> Result<(), OperatorError> {
    validate_domain(record.authority_domain_id.as_ref(), expected_domain, "principal record")?;
    required_non_empty(Some(record.principal_id.as_str()), "principal id")?;
    required_non_empty(
        record.operator_actor_id.as_ref().map(|value| value.value.as_str()),
        "principal operator actor id",
    )?;
    required_non_empty(
        record.endpoint_id.as_ref().map(|value| value.value.as_str()),
        "principal endpoint id",
    )?;
    required_non_empty(
        record.device_id.as_ref().map(|value| value.value.as_str()),
        "principal device id",
    )?;
    if record
        .endpoint_generation
        .as_ref()
        .is_none_or(|generation| generation.value == 0)
    {
        return Err(OperatorError::InvalidRecord(
            "principal endpoint generation must be positive".to_owned(),
        ));
    }
    if record.credential_hash.len() != PRINCIPAL_CREDENTIAL_HASH_BYTES {
        return Err(OperatorError::InvalidRecord(format!(
            "principal credential hash must be {PRINCIPAL_CREDENTIAL_HASH_BYTES} bytes"
        )));
    }
    if record.created_at.is_none() {
        return Err(OperatorError::InvalidRecord(
            "principal record is missing created_at".to_owned(),
        ));
    }
    Ok(())
}

fn validate_domain(
    actual: Option<&AuthorityDomainId>,
    expected: &AuthorityDomainId,
    record_name: &str,
) -> Result<(), OperatorError> {
    if expected.value.is_empty() {
        return Err(OperatorError::InvalidRecord(
            "authority domain id is empty".to_owned(),
        ));
    }
    let actual = actual.ok_or_else(|| {
        OperatorError::InvalidRecord(format!("{record_name} is missing authority_domain_id"))
    })?;
    if actual != expected {
        return Err(OperatorError::InvalidRecord(format!(
            "{record_name} authority domain does not match the core"
        )));
    }
    Ok(())
}

fn required_non_empty(value: Option<&str>, field: &str) -> Result<(), OperatorError> {
    if value.is_none_or(str::is_empty) {
        return Err(OperatorError::InvalidRecord(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn validate_password_hash(password_hash: &str) -> Result<(), OperatorError> {
    parse_password_hash(password_hash).map(|_| ())
}

fn verify_password_hash(password: &str, password_hash: &str) -> Result<bool, OperatorError> {
    let (salt, expected) = parse_password_hash(password_hash)?;
    let params = Params::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, PASSWORD_HASH_BYTES)
        .map_err(|error| OperatorError::CorruptRecord(format!("invalid scrypt params: {error}")))?;
    let mut actual = vec![0_u8; PASSWORD_HASH_BYTES];
    scrypt(password.as_bytes(), &salt, &params, &mut actual)
        .map_err(|error| OperatorError::CorruptRecord(format!("scrypt failed: {error}")))?;
    Ok(constant_time_eq(&actual, &expected))
}

fn parse_password_hash(password_hash: &str) -> Result<(Vec<u8>, Vec<u8>), OperatorError> {
    let mut parts = password_hash.split('$');
    let algorithm = parts.next();
    let encoded_salt = parts.next();
    let encoded_hash = parts.next();
    if algorithm != Some("scrypt")
        || encoded_salt.is_none_or(str::is_empty)
        || encoded_hash.is_none_or(str::is_empty)
        || parts.next().is_some()
    {
        return Err(OperatorError::InvalidRecord(
            "password hash must use scrypt$<salt>$<hash>".to_owned(),
        ));
    }
    let salt = URL_SAFE_NO_PAD
        .decode(encoded_salt.expect("checked salt"))
        .map_err(|_| OperatorError::InvalidRecord("password hash salt is invalid".to_owned()))?;
    let expected = URL_SAFE_NO_PAD
        .decode(encoded_hash.expect("checked hash"))
        .map_err(|_| OperatorError::InvalidRecord("password hash bytes are invalid".to_owned()))?;
    if salt.len() < 16 || expected.len() != PASSWORD_HASH_BYTES {
        return Err(OperatorError::InvalidRecord(
            "password hash has an invalid salt or derived-key length".to_owned(),
        ));
    }
    Ok((salt, expected))
}

#[must_use]
pub fn hash_principal_credential(credential: &str) -> Vec<u8> {
    Sha256::digest(credential.as_bytes()).to_vec()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), OperatorError> {
    let authority_domain_id = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        OperatorError::CorruptRecord("operator event has no authority domain".to_owned())
    })?;
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| OperatorError::CorruptRecord("operator event has no LSN".to_owned()))?;
    Ok((authority_domain_id, lsn.value))
}

fn validate_event_id(
    event_id: &EventId,
    expected_domain: &AuthorityDomainId,
) -> Result<(), OperatorError> {
    if event_id.authority_domain_id.as_ref() != Some(expected_domain) {
        return Err(OperatorError::CorruptRecord(
            "storage returned operator event for another authority domain".to_owned(),
        ));
    }
    if event_id.lsn.is_none() {
        return Err(OperatorError::CorruptRecord(
            "storage returned operator event without an LSN".to_owned(),
        ));
    }
    Ok(())
}

/// Rebuild the operator projection from the authoritative event log.
pub async fn rebuild_operator_registry<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<OperatorRegistry, OperatorError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut registry = OperatorRegistry::new();
    for event in &events {
        registry.observe(event)?;
    }
    Ok(registry)
}
