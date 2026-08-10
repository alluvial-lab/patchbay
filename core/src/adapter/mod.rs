//! Adapter registration port and durable projection.
//!
//! Adapter attachment is recorded as a source-authenticated audit Observation
//! whose protobuf payload is a redacted `AdapterRegistration`. This reuses the
//! existing durable event family without inventing a second writer or storing
//! secret-bearing attachment descriptors.

use std::collections::HashMap;

pub mod capability;

pub use capability::{
    CapabilityValidationContext, CapabilityValidationError, ValidatedAdapterCapability,
    ValidatedProjectionContract, ValidatedResourceCapability, ValidatedSchemaDescriptor,
};

use patchbay_contracts::patchbay::{
    typed_correlation, AdapterId, AdapterRegistration, AuthorityDomainId, CommandId,
    CommandTransition, EventId, FailureCode, Observation, ObservationKind, OperationState,
    PayloadContentType, PayloadEnvelope, StoredEventKind, StoredEventPayload, TargetScope,
    TargetScopeKind,
};
use prost::Message;

use crate::{
    acceptance::{Clock, CommandIndex},
    resource::{ResourceIdentity, ResourceRegistry},
    storage::{validate_next_replay_event, RecordedEvent, Storage},
    target::target_adapter_id,
};

const REGISTRATION_SCHEMA: &str = "patchbay.AdapterRegistration";
pub const DELIVERY_ACKNOWLEDGEMENT_SCHEMA: &str = "patchbay.adapter.DeliveryAcknowledgement.v1";

#[derive(Debug, Clone, PartialEq)]
pub struct AdapterRecord {
    pub registration: AdapterRegistration,
    pub validated_capability: ValidatedAdapterCapability,
    pub attach_event_id: EventId,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdapterRegistry {
    records: HashMap<AdapterId, AdapterRecord>,
}

impl AdapterRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, adapter_id: &AdapterId) -> Option<&AdapterRecord> {
        self.records.get(adapter_id)
    }

    pub fn preflight(&self, registration: &AdapterRegistration) -> Result<(), AdapterError> {
        validate_registration(registration, CapabilityValidationContext::Attach)?;
        let adapter_id = registration
            .adapter_id
            .as_ref()
            .expect("validated adapter id");
        if let Some(current) = self.records.get(adapter_id) {
            let current_generation = current
                .registration
                .adapter_generation
                .as_ref()
                .expect("validated generation")
                .value;
            let reported_generation = registration
                .adapter_generation
                .as_ref()
                .expect("validated generation")
                .value;
            if reported_generation < current_generation {
                return Err(AdapterError::StaleGeneration {
                    live: current_generation,
                    reported: reported_generation,
                });
            }
        }
        Ok(())
    }

    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AdapterError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            AdapterError::CorruptRecord(format!(
                "unknown stored event kind {}",
                event.payload.kind
            ))
        })?;
        if kind == StoredEventKind::Unspecified {
            return Err(AdapterError::CorruptLog(
                "adapter replay event kind is unspecified".to_owned(),
            ));
        }
        if kind != StoredEventKind::Observation {
            return Ok(());
        }
        let observation =
            Observation::decode(event.payload.payload.as_slice()).map_err(|error| {
                AdapterError::CorruptRecord(format!("cannot decode observation: {error}"))
            })?;
        let Some(payload) = observation.payload.as_ref() else {
            return Ok(());
        };
        if payload.schema_ref != REGISTRATION_SCHEMA {
            return Ok(());
        }
        let registration =
            AdapterRegistration::decode(payload.payload.as_slice()).map_err(|error| {
                AdapterError::CorruptRecord(format!("cannot decode adapter registration: {error}"))
            })?;
        let validated_capability =
            validate_registration(&registration, CapabilityValidationContext::Replay)?;
        let event_domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
            AdapterError::CorruptRecord("attach event has no authority domain".into())
        })?;
        if registration.authority_domain_id.as_ref() != Some(event_domain) {
            return Err(AdapterError::CorruptRecord(
                "adapter registration domain does not match attach event".into(),
            ));
        }
        let adapter_id = registration
            .adapter_id
            .clone()
            .expect("validated adapter id");
        if let Some(current) = self.records.get(&adapter_id) {
            let current_generation = current
                .registration
                .adapter_generation
                .as_ref()
                .expect("validated generation")
                .value;
            let next_generation = registration
                .adapter_generation
                .as_ref()
                .expect("validated generation")
                .value;
            if next_generation < current_generation {
                return Err(AdapterError::StaleGeneration {
                    live: current_generation,
                    reported: next_generation,
                });
            }
            let current_lsn = current
                .attach_event_id
                .lsn
                .as_ref()
                .map_or(0, |lsn| lsn.value);
            let event_lsn = event.event_id.lsn.as_ref().map_or(0, |lsn| lsn.value);
            if event_lsn <= current_lsn {
                return Ok(());
            }
        }
        self.records.insert(
            adapter_id,
            AdapterRecord {
                registration,
                validated_capability,
                attach_event_id: event.event_id.clone(),
            },
        );
        Ok(())
    }

    #[must_use]
    pub fn resource_capability(
        &self,
        identity: &ResourceIdentity,
    ) -> Option<&ValidatedResourceCapability> {
        self.records
            .get(identity.adapter_id())?
            .validated_capability
            .resource(identity.resource_kind())
    }

    pub fn validate_resource_projection<'a>(
        &'a self,
        identity: &ResourceIdentity,
        payload: &PayloadEnvelope,
        projection: &PayloadEnvelope,
    ) -> Result<&'a ValidatedResourceCapability, CapabilityValidationError> {
        let capability = self
            .resource_capability(identity)
            .ok_or(CapabilityValidationError::UndeclaredResource)?;
        capability::validate_projection_envelopes(capability, payload, projection)?;
        Ok(capability)
    }
}

pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<AdapterRegistry, AdapterError> {
    let mut registry = AdapterRegistry::new();
    let mut previous_lsn = 0;
    for event in storage
        .read_after(
            authority_domain_id,
            patchbay_contracts::patchbay::Lsn { value: 0 },
        )
        .await?
    {
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, &event)
            .map_err(|error| {
                error.map(AdapterError::CorruptRecord, AdapterError::CorruptLog)
            })?;
        registry.observe(&event)?;
        previous_lsn = validated.lsn;
    }
    Ok(registry)
}

pub async fn ingest_registration<S: Storage>(
    storage: &S,
    registry: &mut AdapterRegistry,
    registration: AdapterRegistration,
) -> Result<EventId, AdapterError> {
    registry.preflight(&registration)?;
    let prepared = prepare_registration(registration);
    let event_id = storage
        .append_decision(
            &prepared.authority_domain_id,
            prepared.source.clone(),
            prepared.audit,
        )
        .await?;
    registry.observe(&RecordedEvent {
        event_id: event_id.clone(),
        payload: prepared.source,
    })?;
    Ok(event_id)
}

/// Atomically append registration and any resource-state degradation required
/// by a capability or attachment-generation redeclaration.
pub async fn ingest_registration_with_resources<S: Storage>(
    storage: &S,
    registry: &mut AdapterRegistry,
    resources: &mut ResourceRegistry,
    registration: AdapterRegistration,
) -> Result<EventId, RegistrationIngestError> {
    registry
        .preflight(&registration)
        .map_err(RegistrationIngestError::PreCommit)?;
    let adapter_id = registration.adapter_id.clone().expect("validated adapter id");
    let incoming_generation = registration
        .adapter_generation
        .expect("validated adapter generation");
    let incoming_capability = validate_registration(
        &registration,
        CapabilityValidationContext::Attach,
    )
    .map_err(RegistrationIngestError::PreCommit)?;
    let current = registry.get(&adapter_id).cloned();
    let prepared = prepare_registration(registration);
    let degradation = current
        .as_ref()
        .map(|current| {
            crate::resource::adapter_redeclaration_event(
                resources,
                &crate::resource::AdapterResourceRedeclaration {
                    authority_domain_id: &prepared.authority_domain_id,
                    adapter_id: &adapter_id,
                    previous_generation: current
                        .registration
                        .adapter_generation
                        .expect("validated current adapter generation"),
                    incoming_generation,
                    previous_capability: &current.validated_capability,
                    incoming_capability: &incoming_capability,
                    observed_at: crate::acceptance::SystemClock.now(),
                },
            )
        })
        .transpose()
        .map_err(AdapterError::from)
        .map_err(RegistrationIngestError::PreCommit)?
        .flatten();
    let Some(degradation) = degradation else {
        let event_id = storage
            .append_decision(
                &prepared.authority_domain_id,
                prepared.source.clone(),
                prepared.audit,
            )
            .await
            .map_err(AdapterError::from)
            .map_err(RegistrationIngestError::PreCommit)?;
        registry
            .observe(&RecordedEvent {
                event_id: event_id.clone(),
                payload: prepared.source,
            })
            .map_err(RegistrationIngestError::PostCommitProjection)?;
        return Ok(event_id);
    };

    let appended = storage
        .append_batch_audited(
            &prepared.authority_domain_id,
            vec![degradation.clone(), prepared.source.clone()],
            prepared.audit,
        )
        .await
        .map_err(AdapterError::from)
        .map_err(RegistrationIngestError::PreCommit)?;
    let [degradation_event_id, registration_event_id] = appended.source_event_ids.as_slice() else {
        return Err(RegistrationIngestError::PostCommitProjection(
            AdapterError::CorruptRecord(
                "registration degradation batch returned the wrong number of source events".into(),
            ),
        ));
    };
    let mut next_resources = resources.clone();
    let mut next_registry = registry.clone();
    let fold_result = next_resources
        .observe(&RecordedEvent {
            event_id: degradation_event_id.clone(),
            payload: degradation,
        })
        .map_err(AdapterError::from)
        .and_then(|()| {
            next_registry.observe(&RecordedEvent {
                event_id: registration_event_id.clone(),
                payload: prepared.source,
            })
        });
    if let Err(error) = fold_result {
        let rebuilt_registry = rebuild_from_log(storage, &prepared.authority_domain_id)
            .await
            .map_err(RegistrationIngestError::PostCommitProjection)?;
        let rebuilt_resources =
            crate::resource::rebuild_from_log(storage, &prepared.authority_domain_id)
                .await
                .map_err(AdapterError::from)
                .map_err(RegistrationIngestError::PostCommitProjection)?;
        *registry = rebuilt_registry;
        *resources = rebuilt_resources;
        return Err(RegistrationIngestError::PostCommitProjection(error));
    }
    *registry = next_registry;
    *resources = next_resources;
    Ok(registration_event_id.clone())
}

struct PreparedRegistration {
    authority_domain_id: AuthorityDomainId,
    source: StoredEventPayload,
    audit: crate::storage::AuditRecordDraft,
}

fn prepare_registration(registration: AdapterRegistration) -> PreparedRegistration {
    let authority_domain_id = registration
        .authority_domain_id
        .clone()
        .expect("validated authority domain");
    let redacted = redact_registration(registration);
    let adapter_id = redacted.adapter_id.clone().expect("validated adapter id");
    let observation = Observation {
        authority_domain_id: Some(authority_domain_id.clone()),
        sender: redacted.endpoint_id.clone().map(|endpoint_id| {
            patchbay_contracts::patchbay::ActorEndpointRef {
                actor_id: Some(patchbay_contracts::patchbay::ActorId {
                    value: adapter_id.value.clone(),
                }),
                endpoint_id: Some(endpoint_id),
                ..Default::default()
            }
        }),
        kind: ObservationKind::Event as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Adapter as i32,
            adapter_id: Some(adapter_id.clone()),
            ..Default::default()
        }),
        payload: Some(PayloadEnvelope {
            payload: redacted.encode_to_vec(),
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: REGISTRATION_SCHEMA.to_owned(),
        }),
        ..Default::default()
    };
    let source = StoredEventPayload {
        kind: StoredEventKind::Observation as i32,
        payload: observation.encode_to_vec(),
    };
    let mut audit = crate::storage::AuditRecordDraft::new(
        crate::acceptance::SystemClock.now(),
        patchbay_contracts::patchbay::AuditEventKind::AdapterAttached,
    );
    audit.actor_id = Some(patchbay_contracts::patchbay::ActorId {
        value: adapter_id.value.clone(),
    });
    audit.endpoint_id = redacted.endpoint_id.clone();
    audit.target_scope = Some(TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(adapter_id),
        ..Default::default()
    });
    audit.reason_code = "adapter_attached".to_owned();
    PreparedRegistration {
        authority_domain_id,
        source,
        audit,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeliveryAcknowledgementResult {
    pub observation_event_id: EventId,
    /// Absent when a redelivered command is idempotently re-acknowledged in
    /// the already-delivered state.
    pub transition_event_id: Option<EventId>,
}

#[must_use]
pub fn is_delivery_acknowledgement(observation: &Observation) -> bool {
    observation
        .payload
        .as_ref()
        .is_some_and(|payload| payload.schema_ref == DELIVERY_ACKNOWLEDGEMENT_SCHEMA)
}

/// Terminalize commands whose execution outcome became unknowable when an adapter disconnected.
///
/// Accepted and delivered commands remain untouched so the existing bounded redelivery policy can
/// recover them. Callers serialize this append batch with adapter observation ingestion; replay's
/// first-terminal rule remains the final authority if a terminal candidate is already durable.
pub async fn fail_running_commands_for_adapter<S: Storage>(
    storage: &S,
    commands: &CommandIndex,
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
) -> Result<Vec<EventId>, AdapterError> {
    let candidates: Vec<_> = commands
        .records()
        .filter(|record| {
            record.state == OperationState::Running
                && record.operation.authority_domain_id.as_ref() == Some(authority_domain_id)
                && record
                    .operation
                    .target_scope
                    .as_ref()
                    .and_then(target_adapter_id)
                    == Some(adapter_id)
        })
        .map(|record| {
            (
                record.command_id.clone(),
                record.operation.correlations.clone(),
            )
        })
        .collect();

    let mut event_ids = Vec::with_capacity(candidates.len());
    for (command_id, correlations) in candidates {
        let command_id_for_audit = command_id.clone();
        let mut audit = crate::storage::AuditRecordDraft::new(
            crate::acceptance::SystemClock.now(),
            patchbay_contracts::patchbay::AuditEventKind::CommandFailed,
        );
        audit.command_id = Some(command_id_for_audit);
        audit.failure_code = Some(FailureCode::ExecutionOutcomeUnknown);
        audit.reason_code = "adapter_disconnect".to_owned();
        let event_id = storage
            .append_decision(
                authority_domain_id,
                StoredEventPayload {
                    kind: StoredEventKind::CommandTransition as i32,
                    payload: CommandTransition {
                        command_id: Some(command_id),
                        from_state: OperationState::Running as i32,
                        to_state: OperationState::Failed as i32,
                        failure_code: FailureCode::ExecutionOutcomeUnknown as i32,
                        correlations,
                        ..Default::default()
                    }
                    .encode_to_vec(),
                },
                audit,
            )
            .await?;
        event_ids.push(event_id);
    }
    Ok(event_ids)
}

/// Durably acknowledge that an attached adapter accepted one Operation for delivery.
///
/// The first acknowledgement commits the canonical `accepted -> delivered` transition and
/// records the adapter's audit Observation. A delivered command that has not advanced to
/// running or terminal remains eligible for re-delivery; its repeated acknowledgement records
/// audit evidence without appending a second lifecycle transition.
pub async fn ingest_delivery_acknowledgement<S: Storage>(
    storage: &S,
    commands: &CommandIndex,
    observation: Observation,
) -> Result<DeliveryAcknowledgementResult, AdapterError> {
    if ObservationKind::try_from(observation.kind).ok() != Some(ObservationKind::Event)
        || !is_delivery_acknowledgement(&observation)
    {
        return Err(AdapterError::InvalidDeliveryAcknowledgement(
            "delivery acknowledgement must be an event with the canonical schema".into(),
        ));
    }
    if FailureCode::try_from(observation.failure_code).ok() != Some(FailureCode::Unspecified) {
        return Err(AdapterError::InvalidDeliveryAcknowledgement(
            "delivery acknowledgement cannot carry a failure code".into(),
        ));
    }
    let authority_domain_id = observation.authority_domain_id.as_ref().ok_or_else(|| {
        AdapterError::InvalidDeliveryAcknowledgement("missing authority_domain_id".into())
    })?;
    let command_id = correlated_command_id(&observation)?;
    let record = commands.get_command(&command_id).ok_or_else(|| {
        AdapterError::InvalidDeliveryAcknowledgement(format!(
            "acknowledgement references unknown command {:?}",
            command_id
        ))
    })?;
    if !matches!(
        record.state,
        OperationState::Accepted | OperationState::Delivered
    ) {
        return Err(AdapterError::InvalidDeliveryAcknowledgement(format!(
            "command {:?} is {:?}, not accepted or delivered",
            command_id, record.state
        )));
    }
    if record.operation.authority_domain_id.as_ref() != Some(authority_domain_id) {
        return Err(AdapterError::InvalidDeliveryAcknowledgement(
            "acknowledgement authority domain does not match the command".into(),
        ));
    }
    if observation.target_scope != record.operation.target_scope {
        return Err(AdapterError::InvalidDeliveryAcknowledgement(
            "acknowledgement target does not match the command".into(),
        ));
    }

    // Commit the lifecycle checkpoint first. If the following audit Observation
    // append fails, recovery still sees `delivered`; the delivery filter then
    // re-offers that non-running command so the adapter can re-acknowledge it
    // and begin execution. A re-ack is a no-op, never a delivered -> delivered
    // transition.
    let transition_event_id = if record.state == OperationState::Accepted {
        let mut audit = crate::storage::AuditRecordDraft::new(
            crate::acceptance::SystemClock.now(),
            patchbay_contracts::patchbay::AuditEventKind::CommandDelivered,
        );
        audit.command_id = Some(command_id.clone());
        audit.reason_code = "delivery_acknowledged".to_owned();
        Some(
            storage
                .append_decision(
                    authority_domain_id,
                    StoredEventPayload {
                        kind: StoredEventKind::CommandTransition as i32,
                        payload: CommandTransition {
                            command_id: Some(command_id),
                            from_state: OperationState::Accepted as i32,
                            to_state: OperationState::Delivered as i32,
                            failure_code: FailureCode::Unspecified as i32,
                            correlations: observation.correlations.clone(),
                            ..Default::default()
                        }
                        .encode_to_vec(),
                    },
                    audit,
                )
                .await?,
        )
    } else {
        None
    };
    let observation_event_id = storage
        .append(
            authority_domain_id,
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: observation.encode_to_vec(),
            },
        )
        .await?;
    Ok(DeliveryAcknowledgementResult {
        observation_event_id,
        transition_event_id,
    })
}

fn correlated_command_id(observation: &Observation) -> Result<CommandId, AdapterError> {
    let mut found: Option<&CommandId> = None;
    for correlation in &observation.correlations {
        let Some(typed_correlation::Ref::CommandId(candidate)) = correlation.r#ref.as_ref() else {
            continue;
        };
        if candidate.value.is_empty() || found.is_some_and(|existing| existing != candidate) {
            return Err(AdapterError::InvalidDeliveryAcknowledgement(
                "acknowledgement must carry one unambiguous, non-empty command correlation".into(),
            ));
        }
        found = Some(candidate);
    }
    found.cloned().ok_or_else(|| {
        AdapterError::InvalidDeliveryAcknowledgement(
            "acknowledgement is missing a command correlation".into(),
        )
    })
}

fn redact_registration(mut registration: AdapterRegistration) -> AdapterRegistration {
    registration.attach_lsn = None;
    if let Some(capability) = registration.capability.as_mut() {
        if let Some(method) = capability.attachment_method.as_mut() {
            method.descriptor.clear();
        }
    }
    registration
}

fn validate_registration(
    registration: &AdapterRegistration,
    context: CapabilityValidationContext,
) -> Result<ValidatedAdapterCapability, AdapterError> {
    if registration
        .adapter_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(AdapterError::InvalidRegistration(
            "missing adapter_id".into(),
        ));
    }
    if registration
        .endpoint_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(AdapterError::InvalidRegistration(
            "missing endpoint_id".into(),
        ));
    }
    if registration
        .authority_domain_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(AdapterError::InvalidRegistration(
            "missing authority_domain_id".into(),
        ));
    }
    if registration.adapter_generation.is_none() {
        return Err(AdapterError::InvalidRegistration(
            "missing adapter_generation".into(),
        ));
    }
    let capability = registration
        .capability
        .as_ref()
        .ok_or_else(|| AdapterError::InvalidRegistration("missing capability".into()))?;
    let validated_capability = ValidatedAdapterCapability::try_from_wire(capability, context)
        .map_err(|error| AdapterError::InvalidRegistration(error.to_string()))?;
    if let Some(reporting) = capability.diagnostic_reporting.as_ref() {
        if reporting.diagnostic_codes.len() > 128
            || reporting.diagnostic_codes.iter().any(|code| {
                code.is_empty()
                    || code.len() > 64
                    || !code.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    })
            })
        {
            return Err(AdapterError::InvalidRegistration(
                "diagnostic_codes must contain at most 128 values matching [a-z0-9_]{1,64}".into(),
            ));
        }
    }
    Ok(validated_capability)
}

/// Registration failures are classified around the durable commit boundary.
/// Once a replacement registration commits, callers must fence the prior
/// attachment even if refreshing the in-memory projections fails.
#[derive(Debug, thiserror::Error)]
pub enum RegistrationIngestError {
    #[error(transparent)]
    PreCommit(AdapterError),
    #[error("adapter registration committed but projection refresh failed: {0}")]
    PostCommitProjection(AdapterError),
}

impl RegistrationIngestError {
    #[must_use]
    pub const fn committed(&self) -> bool {
        matches!(self, Self::PostCommitProjection(_))
    }

    #[must_use]
    pub fn into_adapter_error(self) -> AdapterError {
        match self {
            Self::PreCommit(error) | Self::PostCommitProjection(error) => error,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("invalid adapter registration: {0}")]
    InvalidRegistration(String),
    #[error("stale adapter generation: live={live}, reported={reported}")]
    StaleGeneration { live: u64, reported: u64 },
    #[error("invalid delivery acknowledgement: {0}")]
    InvalidDeliveryAcknowledgement(String),
    #[error("corrupt adapter record: {0}")]
    CorruptRecord(String),
    #[error("corrupt adapter log: {0}")]
    CorruptLog(String),
    #[error(transparent)]
    Resource(#[from] crate::resource::ResourceError),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}

#[cfg(test)]
mod tests {
    use patchbay_contracts::patchbay::{
        AdapterCapability, AdapterSnapshotSupport, AdapterTargetCategory, EndpointId, Generation,
    };

    use super::*;
    use crate::storage::RusqliteStorage;

    #[tokio::test]
    async fn rejected_stale_attach_does_not_poison_durable_rebuild() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("adapter.sqlite3");
        let storage =
            RusqliteStorage::open(database.to_str().expect("utf8 path")).expect("storage opens");
        let domain = AuthorityDomainId {
            value: "authority-main".into(),
        };
        let mut registry = AdapterRegistry::new();

        ingest_registration(&storage, &mut registry, registration(&domain, 2))
            .await
            .expect("generation 2 attaches");
        let stale = ingest_registration(&storage, &mut registry, registration(&domain, 1))
            .await
            .expect_err("generation 1 is stale");
        assert!(matches!(
            stale,
            AdapterError::StaleGeneration {
                live: 2,
                reported: 1
            }
        ));

        let rebuilt = rebuild_from_log(&storage, &domain)
            .await
            .expect("stale rejection left the durable log replayable");
        assert_eq!(
            rebuilt
                .get(&AdapterId { value: "pi".into() })
                .expect("adapter record")
                .registration
                .adapter_generation
                .as_ref()
                .expect("generation")
                .value,
            2
        );
    }

    fn registration(domain: &AuthorityDomainId, generation: u64) -> AdapterRegistration {
        AdapterRegistration {
            adapter_id: Some(AdapterId { value: "pi".into() }),
            endpoint_id: Some(EndpointId {
                value: "pi-endpoint".into(),
            }),
            authority_domain_id: Some(domain.clone()),
            adapter_generation: Some(Generation { value: generation }),
            capability: Some(AdapterCapability {
                session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
                target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
                ..AdapterCapability::default()
            }),
            ..Default::default()
        }
    }
}
