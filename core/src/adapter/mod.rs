//! Adapter registration port and durable projection.
//!
//! Adapter attachment is recorded as a source-authenticated audit Observation
//! whose protobuf payload is a redacted `AdapterRegistration`. This reuses the
//! existing durable event family without inventing a second writer or storing
//! secret-bearing attachment descriptors.

use std::collections::HashMap;

use patchbay_contracts::patchbay::{
    AdapterId, AdapterRegistration, AuthorityDomainId, EventId, Observation, ObservationKind,
    PayloadContentType, PayloadEnvelope, StoredEventKind, StoredEventPayload, TargetScope,
    TargetScopeKind,
};
use prost::Message;

use crate::storage::{RecordedEvent, Storage};

const REGISTRATION_SCHEMA: &str = "patchbay.AdapterRegistration";

#[derive(Debug, Clone, PartialEq)]
pub struct AdapterRecord {
    pub registration: AdapterRegistration,
    pub attach_event_id: EventId,
}

#[derive(Debug, Clone, Default)]
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

    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AdapterError> {
        if StoredEventKind::try_from(event.payload.kind).ok() != Some(StoredEventKind::Observation)
        {
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
        validate_registration(&registration)?;
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
                attach_event_id: event.event_id.clone(),
            },
        );
        Ok(())
    }
}

pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<AdapterRegistry, AdapterError> {
    let mut registry = AdapterRegistry::new();
    for event in storage
        .read_after(
            authority_domain_id,
            patchbay_contracts::patchbay::Lsn { value: 0 },
        )
        .await?
    {
        registry.observe(&event)?;
    }
    Ok(registry)
}

pub async fn ingest_registration<S: Storage>(
    storage: &S,
    registry: &mut AdapterRegistry,
    registration: AdapterRegistration,
) -> Result<EventId, AdapterError> {
    validate_registration(&registration)?;
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
            adapter_id: Some(adapter_id),
            ..Default::default()
        }),
        payload: Some(PayloadEnvelope {
            payload: redacted.encode_to_vec(),
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: REGISTRATION_SCHEMA.to_owned(),
        }),
        ..Default::default()
    };
    let payload = StoredEventPayload {
        kind: StoredEventKind::Observation as i32,
        payload: observation.encode_to_vec(),
    };
    let event_id = storage
        .append(&authority_domain_id, payload.clone())
        .await?;
    registry.observe(&RecordedEvent {
        event_id: event_id.clone(),
        payload,
    })?;
    Ok(event_id)
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

fn validate_registration(registration: &AdapterRegistration) -> Result<(), AdapterError> {
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
    if registration.capability.is_none() {
        return Err(AdapterError::InvalidRegistration(
            "missing capability".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("invalid adapter registration: {0}")]
    InvalidRegistration(String),
    #[error("stale adapter generation: live={live}, reported={reported}")]
    StaleGeneration { live: u64, reported: u64 },
    #[error("corrupt adapter record: {0}")]
    CorruptRecord(String),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
