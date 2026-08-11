use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    AuthorityDomainId, Generation, SessionActivityState, SessionCheckpointTombstone,
    SessionConnectivityState, SessionSnapshot, StoredSessionCheckpoint, TargetScopeKind,
};
use patchbay_core::{
    session::{SessionError, SessionIdentity, SessionRecord, SessionRegistry, SessionTombstone},
    storage::{recover, Storage, StoredSnapshot},
};
use prost::Message;

const CHECKPOINT_MAGIC: &[u8] = b"\x89PATCHBAY-CHECKPOINT\r\n\x1a\n";
const CHECKPOINT_FORMAT_VERSION: u32 = 2;
const CHECKPOINT_VERSION_BYTES: usize = std::mem::size_of::<u32>();
const CHECKPOINT_KIND_BYTES: usize = std::mem::size_of::<u8>();
const CHECKPOINT_HEADER_BYTES: usize =
    CHECKPOINT_MAGIC.len() + CHECKPOINT_VERSION_BYTES + CHECKPOINT_KIND_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CheckpointKind {
    Session = 1,
    #[cfg(test)]
    Resource = 2,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionCheckpointRejection {
    Undiscriminated,
    UnsupportedVersion,
    WrongType,
    Decode,
    AuthorityDomain,
    CoreGeneration,
    Lsn,
    Semantic,
}

impl std::fmt::Display for SessionCheckpointRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Undiscriminated => {
                "session checkpoint is missing the typed checkpoint discriminator"
            }
            Self::UnsupportedVersion => "session checkpoint envelope version is unsupported",
            Self::WrongType => "checkpoint envelope does not contain a session checkpoint",
            Self::Decode => "session checkpoint payload is not decodable",
            Self::AuthorityDomain => "session checkpoint has an invalid authority-domain anchor",
            Self::CoreGeneration => "session checkpoint has an invalid core-generation anchor",
            Self::Lsn => "session checkpoint LSN does not match its storage anchor",
            Self::Semantic => "session checkpoint projection state is inconsistent",
        })
    }
}

impl std::error::Error for SessionCheckpointRejection {}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleSessionCheckpoint {
    pub snapshot: SessionSnapshot,
    pub registry: SessionRegistry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredSessionRegistry {
    pub registry: SessionRegistry,
    pub checkpoint_lsn: u64,
    pub recovered_through_lsn: u64,
    pub replayed_event_count: usize,
    pub checkpoint_rejected: bool,
}

/// Encode a complete private checkpoint payload.
#[must_use]
pub fn encode_stored_session_checkpoint(checkpoint: &StoredSessionCheckpoint) -> Vec<u8> {
    encode_checkpoint(CheckpointKind::Session, &checkpoint.encode_to_vec())
}

pub fn decode_compatible_session_checkpoint(
    stored: &StoredSnapshot,
    expected_domain: &AuthorityDomainId,
    expected_core_generation: &Generation,
) -> Result<CompatibleSessionCheckpoint, SessionCheckpointRejection> {
    if stored.event_id.authority_domain_id.as_ref() != Some(expected_domain) {
        return Err(SessionCheckpointRejection::AuthorityDomain);
    }
    let stored_lsn = stored
        .event_id
        .lsn
        .as_ref()
        .filter(|lsn| lsn.value > 0)
        .ok_or(SessionCheckpointRejection::Lsn)?;
    let payload = decode_session_checkpoint_envelope(&stored.payload)?;
    let checkpoint =
        StoredSessionCheckpoint::decode(payload).map_err(|_| SessionCheckpointRejection::Decode)?;
    let snapshot = checkpoint
        .snapshot
        .ok_or(SessionCheckpointRejection::Decode)?;
    if snapshot.authority_domain_id.as_ref() != Some(expected_domain) {
        return Err(SessionCheckpointRejection::AuthorityDomain);
    }
    if expected_core_generation.value == 0
        || snapshot.core_generation.as_ref().is_none_or(|generation| {
            generation.value == 0 || generation != expected_core_generation
        })
    {
        return Err(SessionCheckpointRejection::CoreGeneration);
    }
    if snapshot.snapshot_lsn.as_ref() != Some(stored_lsn) {
        return Err(SessionCheckpointRejection::Lsn);
    }

    let mut revisions = HashMap::new();
    for revision in &snapshot.view_revisions {
        let target = revision
            .target_scope
            .as_ref()
            .ok_or(SessionCheckpointRejection::Semantic)?;
        if target.kind != TargetScopeKind::RuntimeSession as i32
            || target
                .adapter_id
                .as_ref()
                .is_none_or(|id| id.value.is_empty())
            || target.deployment_scope.is_empty()
            || target
                .runtime_session_id
                .as_ref()
                .is_none_or(|id| id.value.is_empty())
            || target
                .session_generation
                .as_ref()
                .is_none_or(|generation| generation.value == 0)
            || target.actor_id.is_some()
            || !target.project_or_group.is_empty()
            || !target.legacy_audit_resource_id.is_empty()
            || target.resource.is_some()
        {
            return Err(SessionCheckpointRejection::Semantic);
        }
        let key = (
            target.adapter_id.clone().expect("validated adapter id"),
            target.deployment_scope.clone(),
            target
                .runtime_session_id
                .clone()
                .expect("validated runtime id"),
            target
                .session_generation
                .expect("validated session generation"),
        );
        let lsn = revision
            .revision_lsn
            .as_ref()
            .filter(|lsn| lsn.value > 0 && lsn.value <= stored_lsn.value)
            .ok_or(SessionCheckpointRejection::Semantic)?
            .value;
        if revisions.insert(key, lsn).is_some() {
            return Err(SessionCheckpointRejection::Semantic);
        }
    }

    let mut live_records = Vec::with_capacity(snapshot.sessions.len());
    let mut live_keys = HashSet::new();
    for session in &snapshot.sessions {
        if session.authority_domain_id.as_ref() != Some(expected_domain)
            || session
                .adapter_id
                .as_ref()
                .is_none_or(|id| id.value.is_empty())
            || session.deployment_scope.is_empty()
            || session
                .runtime_session_id
                .as_ref()
                .is_none_or(|id| id.value.is_empty())
            || session
                .session_generation
                .as_ref()
                .is_none_or(|generation| generation.value == 0)
            || session.tombstoned
            || session.superseded_at_lsn.is_some()
            || session.observed_at.is_some()
        {
            return Err(SessionCheckpointRejection::Semantic);
        }
        let state = session.state.ok_or(SessionCheckpointRejection::Semantic)?;
        if SessionConnectivityState::try_from(state.connectivity).is_err()
            || SessionActivityState::try_from(state.activity).is_err()
            || state.connectivity == SessionConnectivityState::Unspecified as i32
            || state.activity == SessionActivityState::Unspecified as i32
        {
            return Err(SessionCheckpointRejection::Semantic);
        }
        let key = (
            session.adapter_id.clone().expect("validated adapter id"),
            session.deployment_scope.clone(),
            session
                .runtime_session_id
                .clone()
                .expect("validated runtime id"),
            session
                .session_generation
                .expect("validated session generation"),
        );
        if !live_keys.insert(key.clone()) {
            return Err(SessionCheckpointRejection::Semantic);
        }
        let record_lsn = session
            .last_authoritative_lsn
            .as_ref()
            .filter(|lsn| lsn.value > 0 && lsn.value <= stored_lsn.value)
            .ok_or(SessionCheckpointRejection::Semantic)?
            .value;
        if revisions.remove(&key) != Some(record_lsn) {
            return Err(SessionCheckpointRejection::Semantic);
        }
        live_records.push(SessionRecord {
            identity: SessionIdentity {
                adapter_id: key.0,
                deployment_scope: key.1,
                runtime_session_id: key.2,
                session_generation: key.3,
            },
            state,
            project: session.project.clone(),
            cwd: session.cwd.clone(),
            name: session.name.clone(),
            model: session.model.clone(),
            last_source_cursor: session.last_source_cursor,
            last_authoritative_lsn: Some(record_lsn),
            tombstoned: false,
            superseded_at_lsn: None,
        });
    }
    if !revisions.is_empty() {
        return Err(SessionCheckpointRejection::Semantic);
    }

    let tombstones = checkpoint
        .tombstones
        .into_iter()
        .map(checkpoint_tombstone_to_domain)
        .collect::<Result<Vec<_>, _>>()?;
    let lockdown = snapshot
        .lockdown
        .as_ref()
        .ok_or(SessionCheckpointRejection::Semantic)?;
    validate_checkpoint_lockdown(lockdown, expected_domain, stored_lsn.value)?;
    let lockdown_active = lockdown.active;
    let registry = SessionRegistry::from_checkpoint(
        expected_domain.clone(),
        stored_lsn.value,
        live_records,
        tombstones,
        lockdown_active,
    )
    .map_err(|_| SessionCheckpointRejection::Semantic)?;

    Ok(CompatibleSessionCheckpoint { snapshot, registry })
}

pub async fn recover_session_registry<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    core_generation: &Generation,
) -> Result<RecoveredSessionRegistry, SessionError> {
    let recovery = recover(storage, authority_domain_id, |stored| {
        decode_compatible_session_checkpoint(stored, authority_domain_id, core_generation).ok()
    })
    .await?;
    let checkpoint_lsn = recovery.start_lsn()?;
    let used_checkpoint = recovery.snapshot.is_some();
    let registry = recovery.snapshot.map_or_else(
        || SessionRegistry::new(authority_domain_id.clone()),
        |snapshot| Ok(snapshot.value.registry),
    )?;
    match fold_session_tail(registry, recovery.tail, checkpoint_lsn) {
        Ok((registry, recovered_through_lsn, replayed_event_count)) => {
            Ok(RecoveredSessionRegistry {
                registry,
                checkpoint_lsn,
                recovered_through_lsn,
                replayed_event_count,
                checkpoint_rejected: false,
            })
        }
        Err(checkpoint_error) if used_checkpoint => {
            // A checkpoint is disposable derived data. Its internal fields may
            // validate yet still disagree with the authoritative post-anchor
            // tail. Retry from LSN 0 and report corruption only if that strict
            // full replay also fails.
            let full = recover(storage, authority_domain_id, |_| {
                None::<CompatibleSessionCheckpoint>
            })
            .await?;
            let full_registry = SessionRegistry::new(authority_domain_id.clone())?;
            let (registry, recovered_through_lsn, replayed_event_count) =
                fold_session_tail(full_registry, full.tail, 0).map_err(|full_error| {
                    SessionError::CorruptLog(format!(
                        "session checkpoint tail rejected ({checkpoint_error}); full replay also rejected ({full_error})"
                    ))
                })?;
            Ok(RecoveredSessionRegistry {
                registry,
                checkpoint_lsn: 0,
                recovered_through_lsn,
                replayed_event_count,
                checkpoint_rejected: true,
            })
        }
        Err(error) => Err(error),
    }
}

fn fold_session_tail(
    mut registry: SessionRegistry,
    tail: Vec<patchbay_core::storage::RecordedEvent>,
    start_lsn: u64,
) -> Result<(SessionRegistry, u64, usize), SessionError> {
    let replayed_event_count = tail.len();
    let mut recovered_through_lsn = start_lsn;
    for event in tail {
        registry.observe(&event)?;
        recovered_through_lsn = event
            .event_id
            .lsn
            .as_ref()
            .expect("generic recovery validated every tail event LSN")
            .value;
    }
    Ok((registry, recovered_through_lsn, replayed_event_count))
}

fn validate_checkpoint_lockdown(
    lockdown: &patchbay_contracts::patchbay::SecurityLockdownState,
    expected_domain: &AuthorityDomainId,
    checkpoint_lsn: u64,
) -> Result<(), SessionCheckpointRejection> {
    if !lockdown.active {
        return if lockdown.reason_code.is_empty()
            && lockdown.entered_at.is_none()
            && lockdown.entered_by.is_none()
            && lockdown.entered_event_id.is_none()
        {
            Ok(())
        } else {
            Err(SessionCheckpointRejection::Semantic)
        };
    }
    if lockdown.reason_code.len() > 64
        || lockdown.reason_code.is_empty()
        || !lockdown
            .reason_code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        || lockdown.entered_at.as_ref().is_none_or(|timestamp| {
            !(-62_135_596_800..=253_402_300_799).contains(&timestamp.seconds)
                || !(0..1_000_000_000).contains(&timestamp.nanos)
        })
        || lockdown.entered_by.is_none()
        || lockdown.entered_event_id.as_ref().is_none_or(|event_id| {
            event_id.authority_domain_id.as_ref() != Some(expected_domain)
                || event_id
                    .lsn
                    .as_ref()
                    .is_none_or(|lsn| lsn.value == 0 || lsn.value > checkpoint_lsn)
        })
    {
        return Err(SessionCheckpointRejection::Semantic);
    }
    Ok(())
}

fn checkpoint_tombstone_to_domain(
    tombstone: SessionCheckpointTombstone,
) -> Result<SessionTombstone, SessionCheckpointRejection> {
    Ok(SessionTombstone {
        adapter_id: tombstone
            .adapter_id
            .filter(|id| !id.value.is_empty())
            .ok_or(SessionCheckpointRejection::Semantic)?,
        deployment_scope: if tombstone.deployment_scope.is_empty() {
            return Err(SessionCheckpointRejection::Semantic);
        } else {
            tombstone.deployment_scope
        },
        runtime_session_id: tombstone
            .runtime_session_id
            .filter(|id| !id.value.is_empty())
            .ok_or(SessionCheckpointRejection::Semantic)?,
        superseded_generation: tombstone
            .generation
            .filter(|generation| generation.value > 0)
            .ok_or(SessionCheckpointRejection::Semantic)?,
        superseded_at_lsn: tombstone
            .superseded_at_lsn
            .filter(|lsn| lsn.value > 0)
            .ok_or(SessionCheckpointRejection::Semantic)?
            .value,
    })
}

fn encode_checkpoint(kind: CheckpointKind, payload: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(CHECKPOINT_HEADER_BYTES + payload.len());
    encoded.extend_from_slice(CHECKPOINT_MAGIC);
    encoded.extend_from_slice(&CHECKPOINT_FORMAT_VERSION.to_be_bytes());
    encoded.push(kind as u8);
    encoded.extend_from_slice(payload);
    encoded
}

fn decode_session_checkpoint_envelope(encoded: &[u8]) -> Result<&[u8], SessionCheckpointRejection> {
    if encoded.len() < CHECKPOINT_HEADER_BYTES || !encoded.starts_with(CHECKPOINT_MAGIC) {
        return Err(SessionCheckpointRejection::Undiscriminated);
    }
    let version_offset = CHECKPOINT_MAGIC.len();
    let version_end = version_offset + CHECKPOINT_VERSION_BYTES;
    let version = u32::from_be_bytes(
        encoded[version_offset..version_end]
            .try_into()
            .expect("checkpoint version slice has a fixed width"),
    );
    if version != CHECKPOINT_FORMAT_VERSION {
        return Err(SessionCheckpointRejection::UnsupportedVersion);
    }
    if encoded[version_end] != CheckpointKind::Session as u8 {
        return Err(SessionCheckpointRejection::WrongType);
    }
    Ok(&encoded[CHECKPOINT_HEADER_BYTES..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{EventId, Lsn, ResourceSnapshot, SecurityLockdownState};

    fn domain(value: &str) -> AuthorityDomainId {
        AuthorityDomainId {
            value: value.to_owned(),
        }
    }

    fn checkpoint(snapshot: SessionSnapshot) -> StoredSnapshot {
        StoredSnapshot {
            event_id: EventId {
                authority_domain_id: Some(domain("main")),
                lsn: Some(Lsn { value: 7 }),
            },
            payload: encode_stored_session_checkpoint(&StoredSessionCheckpoint {
                snapshot: Some(snapshot),
                tombstones: Vec::new(),
            }),
        }
    }

    fn valid_snapshot() -> SessionSnapshot {
        SessionSnapshot {
            authority_domain_id: Some(domain("main")),
            snapshot_lsn: Some(Lsn { value: 7 }),
            core_generation: Some(Generation { value: 11 }),
            lockdown: Some(SecurityLockdownState::default()),
            ..SessionSnapshot::default()
        }
    }

    #[test]
    fn compatible_checkpoint_requires_exact_domain_generation_and_lsn() {
        let decoded = decode_compatible_session_checkpoint(
            &checkpoint(valid_snapshot()),
            &domain("main"),
            &Generation { value: 11 },
        )
        .unwrap();
        assert_eq!(decoded.snapshot, valid_snapshot());
        assert_eq!(decoded.registry.covered_through_lsn(), Some(7));
    }

    #[test]
    fn legacy_undiscriminated_and_format_one_are_disposable() {
        let mut stored = checkpoint(valid_snapshot());
        stored.payload = valid_snapshot().encode_to_vec();
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 }
            ),
            Err(SessionCheckpointRejection::Undiscriminated)
        );
        stored = checkpoint(valid_snapshot());
        let offset = CHECKPOINT_MAGIC.len();
        stored.payload[offset..offset + CHECKPOINT_VERSION_BYTES]
            .copy_from_slice(&1u32.to_be_bytes());
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 }
            ),
            Err(SessionCheckpointRejection::UnsupportedVersion)
        );
    }

    #[test]
    fn resource_and_corrupt_payloads_cannot_seed_sessions() {
        let resource = ResourceSnapshot::default();
        let mut stored = checkpoint(valid_snapshot());
        stored.payload = encode_checkpoint(CheckpointKind::Resource, &resource.encode_to_vec());
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 }
            ),
            Err(SessionCheckpointRejection::WrongType)
        );
        stored.payload = encode_checkpoint(CheckpointKind::Session, &[0xff]);
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 }
            ),
            Err(SessionCheckpointRejection::Decode)
        );
    }

    #[test]
    fn wrong_anchor_dimensions_are_rejected() {
        let mut stored = checkpoint(valid_snapshot());
        stored.event_id.authority_domain_id = Some(domain("other"));
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 }
            ),
            Err(SessionCheckpointRejection::AuthorityDomain)
        );
        let mut snapshot = valid_snapshot();
        snapshot.core_generation = Some(Generation { value: 12 });
        assert_eq!(
            decode_compatible_session_checkpoint(
                &checkpoint(snapshot),
                &domain("main"),
                &Generation { value: 11 }
            ),
            Err(SessionCheckpointRejection::CoreGeneration)
        );
        let mut snapshot = valid_snapshot();
        snapshot.snapshot_lsn = Some(Lsn { value: 6 });
        assert_eq!(
            decode_compatible_session_checkpoint(
                &checkpoint(snapshot),
                &domain("main"),
                &Generation { value: 11 }
            ),
            Err(SessionCheckpointRejection::Lsn)
        );
    }
}
