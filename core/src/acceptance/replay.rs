//! Recovery of the in-memory command projection from snapshots and log events.

use patchbay_contracts::patchbay::{
    AuthorityDomainId, FailureCode, Lsn, Operation, OperationState,
};
use prost::Message;

use crate::storage::{recover, Storage, StoredSnapshot};

use super::{is_terminal, AcceptanceError, CommandIndex, CommandRecord};

const COMMAND_INDEX_SNAPSHOT_VERSION: u32 = 1;

/// Rebuild the command index from the latest checkpoint and its event tail.
///
/// Storage supplies a deterministic snapshot-plus-tail recovery view. This
/// function decodes the snapshot projection, validates the tail ordering and
/// authority domain, and applies each event using [`CommandIndex::apply`].
/// Thus unchanged durable contents reconstruct an equal index.
pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<CommandIndex, AcceptanceError> {
    let recovery = recover(storage, authority_domain_id).await?;
    let mut index = match recovery.snapshot.as_ref() {
        Some(snapshot) => decode_snapshot(snapshot, authority_domain_id)?,
        None => CommandIndex::new(),
    };

    let mut previous_lsn = recovery.start_lsn()?;
    for event in recovery.events() {
        let event_domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
            AcceptanceError::CorruptRecord("recovery event has no authority domain".to_owned())
        })?;
        if event_domain != authority_domain_id {
            return Err(AcceptanceError::CorruptLog(format!(
                "recovery event belongs to authority domain {:?}, expected {:?}",
                event_domain, authority_domain_id
            )));
        }

        let event_lsn = event
            .event_id
            .lsn
            .as_ref()
            .ok_or_else(|| AcceptanceError::CorruptRecord("recovery event has no LSN".to_owned()))?
            .value;
        if event_lsn <= previous_lsn {
            return Err(AcceptanceError::CorruptLog(format!(
                "recovery event LSN {event_lsn} is not after previous LSN {previous_lsn}"
            )));
        }

        index.apply(event)?;
        previous_lsn = event_lsn;
    }

    Ok(index)
}

impl CommandIndex {
    /// Persist this projection as a checkpoint at `at_lsn`.
    ///
    /// The caller must ensure this index reflects exactly the consistent log
    /// prefix ending at `at_lsn`. The storage port validates that the LSN is a
    /// committed event; this method uses a versioned, deterministically ordered
    /// payload so recovery can load the checkpoint and replay only its tail.
    pub async fn snapshot_checkpoint<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
        at_lsn: Lsn,
    ) -> Result<(), AcceptanceError> {
        let payload = encode_snapshot(self);
        storage
            .write_snapshot(authority_domain_id, at_lsn, payload)
            .await?;
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
    snapshot: &StoredSnapshot,
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
