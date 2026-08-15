use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    AuthorityDomainId, Generation, LogicalTargetId, SessionActivityState,
    SessionCheckpointTombstone, SessionConnectivityState, SessionSnapshot, StoredSessionCheckpoint,
    TargetScopeKind,
};
use patchbay_core::{
    session::{
        ManagedLineageCheckpoint, SessionError, SessionIdentity, SessionRecord, SessionRegistry,
        SessionTombstone,
    },
    storage::{recover, Storage, StoredSnapshot},
};
use prost::Message;

const CHECKPOINT_MAGIC: &[u8] = b"\x89PATCHBAY-CHECKPOINT\r\n\x1a\n";
const LEGACY_CHECKPOINT_FORMAT_VERSION: u32 = 2;
const CHECKPOINT_FORMAT_VERSION: u32 = 3;
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

#[derive(Clone, PartialEq, Message)]
struct SessionCheckpointPayloadV3 {
    #[prost(message, optional, tag = "1")]
    checkpoint: Option<StoredSessionCheckpoint>,
    #[prost(message, repeated, tag = "2")]
    managed_lineages: Vec<ManagedLineageMarkerV3>,
}

#[derive(Clone, PartialEq, Message)]
struct ManagedLineageMarkerV3 {
    #[prost(message, optional, tag = "1")]
    logical_target_id: Option<LogicalTargetId>,
    #[prost(message, repeated, tag = "2")]
    tombstones: Vec<SessionCheckpointTombstone>,
}

/// Complete private checkpoint materialized by the session projection writer.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterializedSessionCheckpoint {
    pub checkpoint: StoredSessionCheckpoint,
    pub managed_lineages: Vec<ManagedLineageCheckpoint>,
}

impl MaterializedSessionCheckpoint {
    #[must_use]
    pub fn new(
        checkpoint: StoredSessionCheckpoint,
        managed_lineages: Vec<ManagedLineageCheckpoint>,
    ) -> Self {
        Self {
            checkpoint,
            managed_lineages,
        }
    }
}

impl std::ops::Deref for MaterializedSessionCheckpoint {
    type Target = StoredSessionCheckpoint;

    fn deref(&self) -> &Self::Target {
        &self.checkpoint
    }
}

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

/// Encode an unmarked private checkpoint fixture as the current format.
///
/// Production writers use [`encode_materialized_session_checkpoint`] so
/// managed provenance cannot be inferred or silently omitted.
#[must_use]
pub fn encode_stored_session_checkpoint(checkpoint: &StoredSessionCheckpoint) -> Vec<u8> {
    encode_session_checkpoint_v3(checkpoint, &[])
}

/// Encode the production writer's complete private checkpoint payload.
#[must_use]
pub fn encode_materialized_session_checkpoint(
    checkpoint: &MaterializedSessionCheckpoint,
) -> Vec<u8> {
    encode_session_checkpoint_v3(&checkpoint.checkpoint, &checkpoint.managed_lineages)
}

fn encode_session_checkpoint_v3(
    checkpoint: &StoredSessionCheckpoint,
    managed_lineages: &[ManagedLineageCheckpoint],
) -> Vec<u8> {
    let payload = SessionCheckpointPayloadV3 {
        checkpoint: Some(checkpoint.clone()),
        managed_lineages: managed_lineages
            .iter()
            .map(managed_lineage_to_wire)
            .collect(),
    };
    encode_checkpoint(CheckpointKind::Session, &payload.encode_to_vec())
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
    let (format_version, payload) = decode_session_checkpoint_envelope(&stored.payload)?;
    let (checkpoint, managed_lineages) = if format_version == LEGACY_CHECKPOINT_FORMAT_VERSION {
        (
            StoredSessionCheckpoint::decode(payload)
                .map_err(|_| SessionCheckpointRejection::Decode)?,
            Vec::new(),
        )
    } else {
        let payload = SessionCheckpointPayloadV3::decode(payload)
            .map_err(|_| SessionCheckpointRejection::Decode)?;
        let checkpoint = payload
            .checkpoint
            .ok_or(SessionCheckpointRejection::Decode)?;
        let managed_lineages = payload
            .managed_lineages
            .into_iter()
            .map(managed_lineage_from_wire)
            .collect::<Result<Vec<_>, _>>()?;
        (checkpoint, managed_lineages)
    };
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
    let registry = SessionRegistry::from_checkpoint_with_managed_lineages(
        expected_domain.clone(),
        stored_lsn.value,
        live_records,
        tombstones,
        checkpoint.logical_targets,
        managed_lineages,
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
    let checkpoint_was_rejected = recovery.checkpoint_rejected;
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
                checkpoint_rejected: checkpoint_was_rejected,
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

fn managed_lineage_to_wire(record: &ManagedLineageCheckpoint) -> ManagedLineageMarkerV3 {
    ManagedLineageMarkerV3 {
        logical_target_id: Some(record.logical_target_id.clone()),
        tombstones: record
            .tombstones
            .iter()
            .map(|tombstone| SessionCheckpointTombstone {
                adapter_id: Some(tombstone.adapter_id.clone()),
                deployment_scope: tombstone.deployment_scope.clone(),
                runtime_session_id: Some(tombstone.runtime_session_id.clone()),
                generation: Some(tombstone.superseded_generation),
                superseded_at_lsn: Some(patchbay_contracts::patchbay::Lsn {
                    value: tombstone.superseded_at_lsn,
                }),
            })
            .collect(),
    }
}

fn managed_lineage_from_wire(
    record: ManagedLineageMarkerV3,
) -> Result<ManagedLineageCheckpoint, SessionCheckpointRejection> {
    Ok(ManagedLineageCheckpoint {
        logical_target_id: record
            .logical_target_id
            .filter(|id| !id.value.is_empty())
            .ok_or(SessionCheckpointRejection::Semantic)?,
        tombstones: record
            .tombstones
            .into_iter()
            .map(checkpoint_tombstone_to_domain)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn encode_checkpoint(kind: CheckpointKind, payload: &[u8]) -> Vec<u8> {
    encode_checkpoint_version(CHECKPOINT_FORMAT_VERSION, kind, payload)
}

fn encode_checkpoint_version(version: u32, kind: CheckpointKind, payload: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(CHECKPOINT_HEADER_BYTES + payload.len());
    encoded.extend_from_slice(CHECKPOINT_MAGIC);
    encoded.extend_from_slice(&version.to_be_bytes());
    encoded.push(kind as u8);
    encoded.extend_from_slice(payload);
    encoded
}

fn decode_session_checkpoint_envelope(
    encoded: &[u8],
) -> Result<(u32, &[u8]), SessionCheckpointRejection> {
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
    if !matches!(
        version,
        LEGACY_CHECKPOINT_FORMAT_VERSION | CHECKPOINT_FORMAT_VERSION
    ) {
        return Err(SessionCheckpointRejection::UnsupportedVersion);
    }
    if encoded[version_end] != CheckpointKind::Session as u8 {
        return Err(SessionCheckpointRejection::WrongType);
    }
    Ok((version, &encoded[CHECKPOINT_HEADER_BYTES..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{
        AdapterId, EventId, ExternalRuntimeRef, LogicalTargetId, Lsn, ResourceSnapshot,
        RuntimeGenerationRef, RuntimeSessionId, SecurityLockdownState, Session,
        SessionReportSourceCursor, SessionState, TargetScope, ViewRevision,
    };
    use patchbay_core::session::{
        ExternalRuntimeOwnership, LogicalTargetError, LogicalTargetRegistry,
    };

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
                logical_targets: Vec::new(),
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
    fn compatible_checkpoint_restores_logical_target_reverse_index() {
        let logical_target_id = LogicalTargetId {
            value: "target-a".to_owned(),
        };
        let adapter_id = AdapterId {
            value: "pi".to_owned(),
        };
        let external = ExternalRuntimeRef {
            adapter_id: Some(adapter_id.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "runtime-a".to_owned(),
            }),
            generation: Some(Generation { value: 1 }),
        };
        let mut logical_targets = LogicalTargetRegistry::new(domain("main")).unwrap();
        logical_targets
            .create(
                logical_target_id.clone(),
                adapter_id,
                "machine-a".to_owned(),
            )
            .unwrap();
        logical_targets
            .reserve_candidate(&logical_target_id, external.clone())
            .unwrap();
        let mut stored = checkpoint(valid_snapshot());
        stored.payload = encode_stored_session_checkpoint(&StoredSessionCheckpoint {
            snapshot: Some(valid_snapshot()),
            tombstones: Vec::new(),
            logical_targets: logical_targets.checkpoint_records(),
        });

        let decoded = decode_compatible_session_checkpoint(
            &stored,
            &domain("main"),
            &Generation { value: 11 },
        )
        .unwrap();
        assert_eq!(
            decoded.registry.logical_targets().owner_of(&external),
            Some(&logical_target_id)
        );
    }

    #[test]
    fn promotion_checkpoint_retains_changed_runtime_tombstone_and_reverse_reservation() {
        let logical_target_id = LogicalTargetId {
            value: "target-a".to_owned(),
        };
        let other_target_id = LogicalTargetId {
            value: "target-b".to_owned(),
        };
        let adapter_id = AdapterId {
            value: "pi".to_owned(),
        };
        let runtime = |id: &str, generation: u64| ExternalRuntimeRef {
            adapter_id: Some(adapter_id.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: id.to_owned(),
            }),
            generation: Some(Generation { value: generation }),
        };
        let prior = runtime("runtime-a", 1);
        let successor = runtime("runtime-b", 2);
        let prior_ref = RuntimeGenerationRef {
            logical_target_id: Some(logical_target_id.clone()),
            external_runtime: Some(prior.clone()),
        };
        let mut logical_targets = LogicalTargetRegistry::new(domain("main")).unwrap();
        logical_targets
            .create(
                logical_target_id.clone(),
                adapter_id.clone(),
                "machine-a".to_owned(),
            )
            .unwrap();
        logical_targets
            .assign_initial_current(&logical_target_id, prior.clone())
            .unwrap();
        logical_targets
            .reserve_candidate(&logical_target_id, successor.clone())
            .unwrap();
        logical_targets
            .commit_reserved_candidate(&logical_target_id, Some(&prior_ref), &successor, 6)
            .unwrap();

        let successor_id = successor.runtime_session_id.clone().unwrap();
        let successor_generation = successor.generation.unwrap();
        let target_scope = TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(adapter_id.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(successor_id.clone()),
            session_generation: Some(successor_generation),
            ..TargetScope::default()
        };
        let snapshot = SessionSnapshot {
            sessions: vec![Session {
                authority_domain_id: Some(domain("main")),
                adapter_id: Some(adapter_id.clone()),
                deployment_scope: "machine-a".to_owned(),
                runtime_session_id: Some(successor_id),
                session_generation: Some(successor_generation),
                state: Some(SessionState {
                    connectivity: SessionConnectivityState::Live as i32,
                    activity: SessionActivityState::Idle as i32,
                }),
                last_authoritative_lsn: Some(Lsn { value: 6 }),
                last_source_cursor: Some(SessionReportSourceCursor {
                    adapter_generation: Some(Generation { value: 3 }),
                    revision: 1,
                }),
                ..Session::default()
            }],
            view_revisions: vec![ViewRevision {
                target_scope: Some(target_scope),
                revision_lsn: Some(Lsn { value: 6 }),
            }],
            ..valid_snapshot()
        };
        let complete = StoredSessionCheckpoint {
            snapshot: Some(snapshot),
            tombstones: vec![SessionCheckpointTombstone {
                adapter_id: Some(adapter_id.clone()),
                deployment_scope: "machine-a".to_owned(),
                runtime_session_id: prior.runtime_session_id.clone(),
                generation: prior.generation,
                superseded_at_lsn: Some(Lsn { value: 6 }),
            }],
            logical_targets: logical_targets.checkpoint_records(),
        };
        let managed_lineages = vec![ManagedLineageCheckpoint {
            logical_target_id: logical_target_id.clone(),
            tombstones: vec![
                checkpoint_tombstone_to_domain(complete.tombstones[0].clone())
                    .expect("managed tombstone fixture"),
            ],
        }];
        let stored = |checkpoint: &StoredSessionCheckpoint| StoredSnapshot {
            event_id: EventId {
                authority_domain_id: Some(domain("main")),
                lsn: Some(Lsn { value: 7 }),
            },
            payload: encode_materialized_session_checkpoint(&MaterializedSessionCheckpoint::new(
                checkpoint.clone(),
                managed_lineages.clone(),
            )),
        };
        let mut missing_session_tombstone = complete.clone();
        missing_session_tombstone.tombstones.clear();
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored(&missing_session_tombstone),
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::Semantic),
            "managed logical-target and session tombstones must hydrate symmetrically",
        );

        let mut decoded = decode_compatible_session_checkpoint(
            &stored(&complete),
            &domain("main"),
            &Generation { value: 11 },
        )
        .expect("a managed continuation may change native runtime id across a checkpoint");
        assert_eq!(decoded.registry.tombstones().count(), 1);
        assert_eq!(
            decoded.registry.logical_targets().owner_of(&prior),
            Some(&logical_target_id)
        );
        assert_eq!(
            decoded.registry.logical_targets().owner_of(&successor),
            Some(&logical_target_id)
        );
        decoded
            .registry
            .logical_targets_mut()
            .create(other_target_id.clone(), adapter_id, "machine-a".to_owned())
            .unwrap();
        assert!(matches!(
            decoded
                .registry
                .logical_targets_mut()
                .reserve_candidate(&other_target_id, prior),
            Err(LogicalTargetError::DuplicateNativeReference { .. })
        ));
    }

    #[test]
    fn semantic_mutations_are_disposable_without_seeding_session_state() {
        let adapter_id = AdapterId {
            value: "pi".to_owned(),
        };
        let runtime_session_id = RuntimeSessionId {
            value: "session-1".to_owned(),
        };
        let target = TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(adapter_id.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime_session_id.clone()),
            session_generation: Some(Generation { value: 2 }),
            ..TargetScope::default()
        };
        let session = Session {
            authority_domain_id: Some(domain("main")),
            adapter_id: Some(adapter_id.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime_session_id.clone()),
            session_generation: Some(Generation { value: 2 }),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "main".to_owned(),
            state: Some(SessionState {
                connectivity: SessionConnectivityState::Live as i32,
                activity: SessionActivityState::Idle as i32,
            }),
            last_authoritative_lsn: Some(Lsn { value: 7 }),
            observed_at: None,
            tombstoned: false,
            superseded_at_lsn: None,
            model: "provider/model".to_owned(),
            last_source_cursor: Some(SessionReportSourceCursor {
                adapter_generation: Some(Generation { value: 1 }),
                revision: 2,
            }),
        };
        let tombstone = SessionCheckpointTombstone {
            adapter_id: Some(adapter_id),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime_session_id),
            generation: Some(Generation { value: 1 }),
            superseded_at_lsn: Some(Lsn { value: 4 }),
        };
        let complete = StoredSessionCheckpoint {
            snapshot: Some(SessionSnapshot {
                sessions: vec![session],
                view_revisions: vec![ViewRevision {
                    target_scope: Some(target),
                    revision_lsn: Some(Lsn { value: 7 }),
                }],
                ..valid_snapshot()
            }),
            tombstones: vec![tombstone],
            logical_targets: Vec::new(),
        };
        let stored = |candidate: StoredSessionCheckpoint| StoredSnapshot {
            event_id: EventId {
                authority_domain_id: Some(domain("main")),
                lsn: Some(Lsn { value: 7 }),
            },
            payload: encode_stored_session_checkpoint(&candidate),
        };
        assert!(decode_compatible_session_checkpoint(
            &stored(complete.clone()),
            &domain("main"),
            &Generation { value: 11 },
        )
        .is_ok());

        let mut mutations = Vec::new();
        let mut candidate = complete.clone();
        let duplicate_session = candidate.snapshot.as_ref().unwrap().sessions[0].clone();
        candidate
            .snapshot
            .as_mut()
            .unwrap()
            .sessions
            .push(duplicate_session);
        mutations.push(candidate);
        let mut candidate = complete.clone();
        let duplicate_revision = candidate.snapshot.as_ref().unwrap().view_revisions[0].clone();
        candidate
            .snapshot
            .as_mut()
            .unwrap()
            .view_revisions
            .push(duplicate_revision);
        mutations.push(candidate);
        let mut candidate = complete.clone();
        candidate.snapshot.as_mut().unwrap().sessions[0].last_authoritative_lsn =
            Some(Lsn { value: 8 });
        mutations.push(candidate);
        let mut candidate = complete.clone();
        candidate.snapshot.as_mut().unwrap().sessions[0]
            .last_source_cursor
            .as_mut()
            .unwrap()
            .adapter_generation = Some(Generation { value: 0 });
        mutations.push(candidate);
        let mut candidate = complete.clone();
        candidate.tombstones[0].generation = Some(Generation { value: 2 });
        mutations.push(candidate);
        let mut candidate = complete.clone();
        candidate.tombstones[0].superseded_at_lsn = Some(Lsn { value: 8 });
        mutations.push(candidate);
        let mut candidate = complete;
        candidate
            .snapshot
            .as_mut()
            .unwrap()
            .lockdown
            .as_mut()
            .unwrap()
            .reason_code = "inactive_but_populated".to_owned();
        mutations.push(candidate);

        for candidate in mutations {
            assert_eq!(
                decode_compatible_session_checkpoint(
                    &stored(candidate),
                    &domain("main"),
                    &Generation { value: 11 },
                ),
                Err(SessionCheckpointRejection::Semantic),
            );
        }
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

        let legacy_v2 = StoredSnapshot {
            event_id: EventId {
                authority_domain_id: Some(domain("main")),
                lsn: Some(Lsn { value: 7 }),
            },
            payload: encode_checkpoint_version(
                LEGACY_CHECKPOINT_FORMAT_VERSION,
                CheckpointKind::Session,
                &StoredSessionCheckpoint {
                    snapshot: Some(valid_snapshot()),
                    tombstones: Vec::new(),
                    logical_targets: Vec::new(),
                }
                .encode_to_vec(),
            ),
        };
        assert!(decode_compatible_session_checkpoint(
            &legacy_v2,
            &domain("main"),
            &Generation { value: 11 },
        )
        .is_ok());
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
