//! Durable operator records and credential-backed control-surface principals.
//!
//! The authority-domain event log is the source of truth. Ingestion validates,
//! appends, and only then warms the projection, matching grant ingestion.

use std::collections::HashMap;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, ControlSurfacePrincipalRecord, EventId, Lsn, OperatorRecord,
    StoredEventKind, StoredEventPayload,
};
use prost::Message;
use scrypt::{scrypt, Params};
use sha2::{Digest, Sha256};

use crate::storage::{RecordedEvent, Storage};

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
    #[error("corrupt operator record: {0}")]
    CorruptRecord(String),
    #[error("corrupt operator log: {0}")]
    CorruptLog(String),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}

/// Deterministic projection of the current operator and enrolled principals.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OperatorRegistry {
    operator: Option<OperatorRecord>,
    principals: HashMap<String, ControlSurfacePrincipalRecord>,
    active_principal_by_endpoint: HashMap<String, String>,
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
        let actual = hash_principal_credential(credential);
        constant_time_eq(&actual, &record.credential_hash).then(|| record.clone())
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
            StoredEventKind::Operation
            | StoredEventKind::Observation
            | StoredEventKind::Elicitation
            | StoredEventKind::Grant
            | StoredEventKind::DescendantGrant
            | StoredEventKind::Revocation
            | StoredEventKind::SessionState
            | StoredEventKind::CommandTransition
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
        let endpoint_id = record
            .endpoint_id
            .as_ref()
            .expect("validated principal has endpoint")
            .value
            .clone();
        self.active_principal_by_endpoint
            .insert(endpoint_id, record.principal_id.clone());
        self.principals.insert(record.principal_id.clone(), record);
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

async fn append_and_warm<S: Storage>(
    storage: &S,
    projection: &mut OperatorRegistry,
    authority_domain_id: &AuthorityDomainId,
    payload: StoredEventPayload,
) -> Result<EventId, OperatorError> {
    let event_id = storage.append(authority_domain_id, payload.clone()).await?;
    validate_event_id(&event_id, authority_domain_id)?;
    projection.observe(&RecordedEvent {
        event_id: event_id.clone(),
        payload,
    })?;
    Ok(event_id)
}

pub fn validate_operator_record(
    record: &OperatorRecord,
    expected_domain: &AuthorityDomainId,
) -> Result<(), OperatorError> {
    validate_domain(record.authority_domain_id.as_ref(), expected_domain)?;
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
    validate_domain(record.authority_domain_id.as_ref(), expected_domain)?;
    required_non_empty(Some(record.principal_id.as_str()), "principal id")?;
    required_non_empty(
        record
            .operator_actor_id
            .as_ref()
            .map(|value| value.value.as_str()),
        "principal operator actor id",
    )?;
    required_non_empty(
        record
            .endpoint_id
            .as_ref()
            .map(|value| value.value.as_str()),
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
) -> Result<(), OperatorError> {
    if expected.value.is_empty() {
        return Err(OperatorError::InvalidRecord(
            "authority domain id is empty".to_owned(),
        ));
    }
    let actual = actual.ok_or_else(|| {
        OperatorError::InvalidRecord("record is missing authority_domain_id".to_owned())
    })?;
    if actual != expected {
        return Err(OperatorError::InvalidRecord(
            "record authority domain does not match the core".to_owned(),
        ));
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
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
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
