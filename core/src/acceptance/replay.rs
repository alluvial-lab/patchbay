//! Recovery of the in-memory command projection from snapshots and log events.
// v0.1.0: the storage snapshot slot has a typed session envelope, not a
// composite whole-core checkpoint. Command-index checkpointing is therefore
// deferred; the serialization code below is retained for the future
// namespaced/composite integration, hence the dead-code allowance.
#![allow(dead_code)]

use patchbay_contracts::patchbay::{AuthorityDomainId, FailureCode, Operation, OperationState};
use prost::Message;

use crate::storage::{validate_next_replay_event, Storage};

use super::{is_terminal, AcceptanceError, CommandIndex, CommandRecord};

const COMMAND_INDEX_SNAPSHOT_VERSION: u32 = 1;

/// Rebuild the command index from the durable event log.
///
/// v0.1.0 always replays from LSN 0. Snapshot checkpointing of the command
/// index is **deferred** because the current typed storage slot contains only a
/// session projection. A command-only snapshot in that slot would hide
/// pre-checkpoint events from sibling projections (authority, sessions,
/// elicitation) that share the same authority domain. When the snapshot
/// namespace carries a composite whole-core checkpoint or independent
/// per-projection cursors, this function can validate its own typed checkpoint
/// and replay only the tail.
///
/// For now this function reads the full log directly. The snapshot
/// serialization code (`encode_snapshot`/`decode_snapshot`) is retained for
/// the future namespaced/composite integration.
pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<CommandIndex, AcceptanceError> {
    // v0.1.0: ignore the session-only checkpoint slot. Read from LSN 0.
    let mut index = CommandIndex::new();
    let mut previous_lsn = 0u64;

    // Read all events from LSN 0. This projection has no compatible typed
    // checkpoint to offer the validator-aware recovery helper yet.
    let events = storage
        .read_after(
            authority_domain_id,
            patchbay_contracts::patchbay::Lsn { value: 0 },
        )
        .await?;

    for event in events {
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, &event)
            .map_err(|error| {
                error.map(AcceptanceError::CorruptRecord, AcceptanceError::CorruptLog)
            })?;
        index.apply(&event)?;
        previous_lsn = validated.lsn;
    }

    Ok(index)
}

impl CommandIndex {
    /// Snapshot checkpointing is **deferred for v0.1.0**.
    ///
    /// The current typed storage slot contains a session checkpoint, not a
    /// composite whole-core checkpoint. A command-only snapshot in that slot
    /// would hide pre-checkpoint events from sibling projections (authority,
    /// sessions, elicitation). When the namespace carries a composite
    /// checkpoint or independent per-projection cursors, this method can
    /// serialize the index and call `storage.write_snapshot`. The
    /// serialization code is retained below.
    #[allow(dead_code)]
    pub async fn snapshot_checkpoint<S: Storage>(
        &self,
        _storage: &S,
        _authority_domain_id: &AuthorityDomainId,
        _at_lsn: patchbay_contracts::patchbay::Lsn,
    ) -> Result<(), AcceptanceError> {
        // Deferred — see doc comment.
        Ok(())
    }
}

#[derive(Clone, PartialEq, Message)]
struct CommandIndexSnapshot {
    #[prost(uint32, tag = "1")]
    format_version: u32,
    #[prost(message, repeated, tag = "2")]
    commands: Vec<SnapshotCommandRecord>,
}

#[derive(Clone, PartialEq, Message)]
struct SnapshotCommandRecord {
    #[prost(message, optional, tag = "1")]
    operation: Option<Operation>,
    #[prost(enumeration = "OperationState", tag = "2")]
    state: i32,
    #[prost(uint64, optional, tag = "3")]
    terminal_lsn: Option<u64>,
    #[prost(enumeration = "FailureCode", tag = "4")]
    failure_code: i32,
}

fn encode_snapshot(index: &CommandIndex) -> Vec<u8> {
    let mut records: Vec<_> = index.records().collect();
    records.sort_unstable_by(|left, right| left.command_id.value.cmp(&right.command_id.value));

    CommandIndexSnapshot {
        format_version: COMMAND_INDEX_SNAPSHOT_VERSION,
        commands: records
            .into_iter()
            .map(|record| SnapshotCommandRecord {
                operation: Some(record.operation.clone()),
                state: record.state as i32,
                terminal_lsn: record.terminal_lsn,
                failure_code: record.failure_code.unwrap_or(FailureCode::Unspecified) as i32,
            })
            .collect(),
    }
    .encode_to_vec()
}

fn decode_snapshot(
    snapshot: &crate::storage::StoredSnapshot,
    authority_domain_id: &AuthorityDomainId,
) -> Result<CommandIndex, AcceptanceError> {
    let snapshot_domain = snapshot
        .event_id
        .authority_domain_id
        .as_ref()
        .ok_or_else(|| AcceptanceError::CorruptRecord("snapshot has no authority domain".into()))?;
    if snapshot_domain != authority_domain_id {
        return Err(AcceptanceError::CorruptLog(format!(
            "snapshot belongs to authority domain {:?}, expected {:?}",
            snapshot_domain, authority_domain_id
        )));
    }
    let snapshot_lsn = snapshot
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| AcceptanceError::CorruptRecord("snapshot has no LSN".into()))?
        .value;

    let decoded = CommandIndexSnapshot::decode(snapshot.payload.as_slice()).map_err(|error| {
        AcceptanceError::CorruptRecord(format!(
            "cannot decode command-index snapshot at LSN {snapshot_lsn}: {error}"
        ))
    })?;
    if decoded.format_version != COMMAND_INDEX_SNAPSHOT_VERSION {
        return Err(AcceptanceError::CorruptRecord(format!(
            "unsupported command-index snapshot version {} at LSN {snapshot_lsn}",
            decoded.format_version
        )));
    }

    let mut index = CommandIndex::new();
    for encoded_record in decoded.commands {
        let operation = encoded_record.operation.ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "command-index snapshot at LSN {snapshot_lsn} contains a record without an operation"
            ))
        })?;
        let operation_domain = operation.authority_domain_id.as_ref().ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "command-index snapshot at LSN {snapshot_lsn} contains an operation without an authority domain"
            ))
        })?;
        if operation_domain != authority_domain_id {
            return Err(AcceptanceError::CorruptLog(format!(
                "snapshot operation belongs to authority domain {:?}, expected {:?}",
                operation_domain, authority_domain_id
            )));
        }

        let state = OperationState::try_from(encoded_record.state).map_err(|_| {
            AcceptanceError::CorruptRecord(format!(
                "snapshot command has unknown state {} at LSN {snapshot_lsn}",
                encoded_record.state
            ))
        })?;
        if state == OperationState::Unspecified {
            return Err(AcceptanceError::CorruptRecord(format!(
                "snapshot command has unspecified state at LSN {snapshot_lsn}"
            )));
        }
        let failure_code = FailureCode::try_from(encoded_record.failure_code).map_err(|_| {
            AcceptanceError::CorruptRecord(format!(
                "snapshot command has unknown failure code {} at LSN {snapshot_lsn}",
                encoded_record.failure_code
            ))
        })?;

        validate_snapshot_metadata(
            state,
            encoded_record.terminal_lsn,
            failure_code,
            snapshot_lsn,
        )?;

        let mut record = CommandRecord::new(operation, snapshot_lsn)?;
        record.state = state;
        record.terminal_lsn = encoded_record.terminal_lsn;
        record.failure_code = (failure_code != FailureCode::Unspecified).then_some(failure_code);
        index.insert_recovered_record(record)?;
    }

    Ok(index)
}

fn validate_snapshot_metadata(
    state: OperationState,
    terminal_lsn: Option<u64>,
    failure_code: FailureCode,
    snapshot_lsn: u64,
) -> Result<(), AcceptanceError> {
    if is_terminal(state) {
        let terminal_lsn = terminal_lsn.ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "terminal snapshot command in state {state:?} has no terminal LSN"
            ))
        })?;
        if terminal_lsn == 0 || terminal_lsn > snapshot_lsn {
            return Err(AcceptanceError::CorruptRecord(format!(
                "terminal snapshot command has LSN {terminal_lsn} outside snapshot prefix 1..={snapshot_lsn}"
            )));
        }
    } else {
        if terminal_lsn.is_some() {
            return Err(AcceptanceError::CorruptRecord(format!(
                "non-terminal snapshot command in state {state:?} has a terminal LSN"
            )));
        }
        if failure_code != FailureCode::Unspecified {
            return Err(AcceptanceError::CorruptRecord(format!(
                "non-terminal snapshot command in state {state:?} has failure code {failure_code:?}"
            )));
        }
    }
    Ok(())
}
