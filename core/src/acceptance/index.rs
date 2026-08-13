//! In-memory projection of accepted commands.
//!
//! The durable event log is authoritative. [`CommandIndex`] is the derived hot
//! path used for command and idempotency-key lookups; recovery reconstructs it
//! by applying the same ordered event sequence.

use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    AcceptedOperation, AuthorityDomainId, CommandId, CommandTransition, FailureCode, Observation,
    ObservationKind, Operation, OperationKind, OperationState, Revocation, StoredEventKind,
};
use prost::Message;

use crate::storage::{RecordedEvent, TargetKey};

use super::{
    apply_grant_revocation_effect, apply_transition, exact_command_correlation, target_key_for,
    AcceptanceError, CommandRecord,
};

type DedupKey = (String, String, String);

/// The in-memory command index rebuilt from the durable event log.
///
/// Both maps provide average O(1) lookup. `key_to_command` mirrors the durable
/// acceptance boundary's `(authority domain, idempotency key, target)` scope so
/// retries can resolve to the existing command projection.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommandIndex {
    commands: HashMap<CommandId, CommandRecord>,
    key_to_command: HashMap<DedupKey, CommandId>,
    /// Successful spawn result evidence that intentionally did not terminalize
    /// the command. Delivery reconstruction uses this durable checkpoint to
    /// avoid re-executing a non-idempotent spawn while completion waits for its
    /// correlated session fact.
    deferred_spawn_successes: HashSet<CommandId>,
}

impl CommandIndex {
    /// Construct an empty command index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one durable event into this projection.
    ///
    /// Applying the same valid event sequence to an empty index always yields
    /// the same index. Malformed payloads and impossible event orderings fail
    /// immediately rather than leaving a silently incorrect projection.
    pub fn apply(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            AcceptanceError::CorruptRecord(format!(
                "unknown stored event kind {}",
                event.payload.kind
            ))
        })?;

        match kind {
            StoredEventKind::Operation => self.apply_operation(event),
            StoredEventKind::CommandTransition => self.apply_command_transition(event),
            StoredEventKind::Revocation => self.apply_revocation(event),
            // Observations never mutate CommandState. The one auxiliary fact
            // retained here is a successful spawn result whose terminalization
            // is deliberately deferred to the descendant-completion owner.
            StoredEventKind::Observation => self.apply_observation(event),
            StoredEventKind::Elicitation
            | StoredEventKind::Grant
            | StoredEventKind::DescendantGrant
            | StoredEventKind::SessionState
            | StoredEventKind::ResourceState
            | StoredEventKind::SpawnClaim
            | StoredEventKind::OperatorRecord
            | StoredEventKind::ControlSurfacePrincipal
            | StoredEventKind::OperatorSessionRevocation
            | StoredEventKind::ControlSurfaceRevocation
            | StoredEventKind::SecurityLockdown
            | StoredEventKind::AuditRecord => Ok(()),
            StoredEventKind::Unspecified => Err(AcceptanceError::CorruptLog(
                "command replay event kind is unspecified".to_owned(),
            )),
        }
    }

    /// Return a command by id using the primary hash index.
    #[must_use]
    pub fn get_command(&self, command_id: &CommandId) -> Option<&CommandRecord> {
        self.commands.get(command_id)
    }

    /// Resolve a retry's durable deduplication scope to its command projection.
    #[must_use]
    pub fn get_by_idempotency_key(
        &self,
        authority_domain_id: &AuthorityDomainId,
        idempotency_key: &str,
        target_key: &TargetKey,
    ) -> Option<&CommandRecord> {
        let key = (
            authority_domain_id.value.clone(),
            idempotency_key.to_owned(),
            target_key.as_str().to_owned(),
        );
        self.key_to_command
            .get(&key)
            .and_then(|command_id| self.commands.get(command_id))
    }

    /// Whether durable successful-result evidence suppresses redelivery while
    /// this spawn waits for registration and descendant completion.
    #[must_use]
    pub fn has_deferred_spawn_success(&self, command_id: &CommandId) -> bool {
        self.deferred_spawn_successes.contains(command_id)
    }

    /// Number of accepted commands in the projection.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the projection contains no accepted commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Iterate over every projected command record.
    ///
    /// The durable log remains authoritative; this view supports focused
    /// reconciliation policies such as adapter-loss terminalization.
    pub fn records(&self) -> impl Iterator<Item = &CommandRecord> {
        self.commands.values()
    }

    pub(super) fn insert_recovered_record(
        &mut self,
        record: CommandRecord,
    ) -> Result<(), AcceptanceError> {
        let command_id = record.command_id.clone();
        if command_id.value.is_empty() {
            return Err(AcceptanceError::CorruptRecord(
                "accepted operation has an empty command_id".to_owned(),
            ));
        }
        if self.commands.contains_key(&command_id) {
            return Err(AcceptanceError::CorruptLog(format!(
                "duplicate operation for command {:?}",
                command_id
            )));
        }

        let dedup_key = dedup_key_for(&record.operation)?;
        if let Some(existing_command_id) = self.key_to_command.get(&dedup_key) {
            return Err(AcceptanceError::CorruptLog(format!(
                "idempotency scope for command {:?} is already bound to command {:?}",
                command_id, existing_command_id
            )));
        }

        self.key_to_command.insert(dedup_key, command_id.clone());
        self.commands.insert(command_id, record);
        Ok(())
    }

    fn apply_operation(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        let accepted =
            AcceptedOperation::decode(event.payload.payload.as_slice()).map_err(|error| {
                AcceptanceError::CorruptRecord(format!(
                    "cannot decode accepted operation at LSN {event_lsn}: {error}"
                ))
            })?;
        let grant_id = accepted.authorizing_grant_id.ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "accepted operation at LSN {event_lsn} is missing authorizing_grant_id"
            ))
        })?;
        if grant_id.value.is_empty() {
            return Err(AcceptanceError::CorruptRecord(format!(
                "accepted operation at LSN {event_lsn} has an empty authorizing_grant_id"
            )));
        }
        let operation = accepted.operation.ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "accepted operation at LSN {event_lsn} is missing operation"
            ))
        })?;

        let operation_domain = operation.authority_domain_id.as_ref().ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "accepted operation at LSN {event_lsn} is missing authority_domain_id"
            ))
        })?;
        if operation_domain != event_domain {
            return Err(AcceptanceError::CorruptLog(format!(
                "operation authority domain {:?} does not match event authority domain {:?} at LSN {event_lsn}",
                operation_domain, event_domain
            )));
        }

        let record = CommandRecord::new_accepted(operation, grant_id, event_lsn)?;
        self.insert_recovered_record(record)
    }

    fn apply_observation(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        let (_, event_lsn) = event_identity(event)?;
        let observation =
            Observation::decode(event.payload.payload.as_slice()).map_err(|error| {
                AcceptanceError::CorruptRecord(format!(
                    "cannot decode observation at LSN {event_lsn}: {error}"
                ))
            })?;
        if ObservationKind::try_from(observation.kind).ok() != Some(ObservationKind::Result)
            || FailureCode::try_from(observation.failure_code).ok()
                != Some(FailureCode::Unspecified)
        {
            return Ok(());
        }
        let Some(command_id) = exact_command_correlation(&observation.correlations) else {
            return Ok(());
        };
        let Some(record) = self.commands.get(&command_id) else {
            // Unknown/pre-acceptance evidence is non-qualifying. It must not
            // become delivery or descendant authority if a matching command
            // appears later in the log.
            return Ok(());
        };
        if record.operation.kind != OperationKind::Spawn as i32 {
            return Ok(());
        }
        if observation.target_scope != record.operation.target_scope {
            return Err(AcceptanceError::CorruptLog(format!(
                "successful spawn result target does not match command {:?} at LSN {event_lsn}",
                command_id
            )));
        }
        if matches!(
            record.state,
            OperationState::Delivered | OperationState::Running
        ) {
            self.deferred_spawn_successes.insert(command_id);
        }
        Ok(())
    }

    fn apply_revocation(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        // One revocation may carry several command effects. Stage the complete
        // event so a later invalid effect cannot leave earlier commands
        // terminalized in an otherwise rejected projection update.
        let mut staged = self.clone();
        staged.apply_revocation_in_place(event)?;
        *self = staged;
        Ok(())
    }

    fn apply_revocation_in_place(
        &mut self,
        event: &RecordedEvent,
    ) -> Result<(), AcceptanceError> {
        let (_, event_lsn) = event_identity(event)?;
        let revocation = Revocation::decode(event.payload.payload.as_slice()).map_err(|error| {
            AcceptanceError::CorruptRecord(format!(
                "cannot decode revocation at LSN {event_lsn}: {error}"
            ))
        })?;
        let grant_id = revocation.grant_id.ok_or_else(|| {
            AcceptanceError::CorruptLog(format!(
                "revocation at LSN {event_lsn} is missing grant_id"
            ))
        })?;
        for effect in revocation.command_effects {
            let command_id = effect.command_id.as_ref().ok_or_else(|| {
                AcceptanceError::CorruptLog(format!(
                    "revocation at LSN {event_lsn} has effect without command_id"
                ))
            })?;
            let record = self.commands.get_mut(command_id).ok_or_else(|| {
                AcceptanceError::CorruptLog(format!(
                    "revocation at LSN {event_lsn} references unknown command {:?}",
                    command_id
                ))
            })?;
            if record.grant_id.as_ref() != Some(&grant_id) {
                return Err(AcceptanceError::CorruptLog(format!(
                    "revocation at LSN {event_lsn} effect command {:?} was accepted under another grant",
                    command_id
                )));
            }
            let _ = apply_grant_revocation_effect(record, &effect, event_lsn)?;
        }
        Ok(())
    }

    fn apply_command_transition(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        let transition =
            CommandTransition::decode(event.payload.payload.as_slice()).map_err(|error| {
                AcceptanceError::CorruptRecord(format!(
                    "cannot decode command transition at LSN {event_lsn}: {error}"
                ))
            })?;
        let command_id = transition.command_id.as_ref().ok_or_else(|| {
            AcceptanceError::CorruptLog(format!(
                "command transition at LSN {event_lsn} is missing command_id"
            ))
        })?;
        let record = self.commands.get_mut(command_id).ok_or_else(|| {
            AcceptanceError::CorruptLog(format!(
                "transition for unknown command {:?} at LSN {event_lsn}",
                command_id
            ))
        })?;

        let operation_domain = record
            .operation
            .authority_domain_id
            .as_ref()
            .ok_or_else(|| {
                AcceptanceError::CorruptRecord(format!(
                    "indexed command {:?} is missing authority_domain_id",
                    record.command_id
                ))
            })?;
        if operation_domain != event_domain {
            return Err(AcceptanceError::CorruptLog(format!(
                "transition event authority domain {:?} does not match command authority domain {:?} at LSN {event_lsn}",
                event_domain, operation_domain
            )));
        }

        apply_transition(record, &transition, event_lsn).or_else(|err| match err {
            // A duplicate terminal transition from a race-produced TOCTOU
            // window. The first terminal wins (TerminalFinality); the
            // second is a stale candidate, not corruption. Skip it.
            AcceptanceError::AlreadyTerminal(_) => Ok(()),
            other => Err(other),
        })
    }
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), AcceptanceError> {
    let authority_domain_id = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        AcceptanceError::CorruptRecord("event has no authority domain".to_owned())
    })?;
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| AcceptanceError::CorruptRecord("event has no LSN".to_owned()))?;
    Ok((authority_domain_id, lsn.value))
}

fn dedup_key_for(operation: &Operation) -> Result<DedupKey, AcceptanceError> {
    let command_id = operation.command_id.as_ref().ok_or_else(|| {
        AcceptanceError::CorruptRecord("accepted operation is missing command_id".to_owned())
    })?;
    let authority_domain_id = operation.authority_domain_id.as_ref().ok_or_else(|| {
        AcceptanceError::CorruptRecord(format!(
            "accepted operation for command {:?} is missing authority_domain_id",
            command_id
        ))
    })?;
    if authority_domain_id.value.is_empty() {
        return Err(AcceptanceError::CorruptRecord(format!(
            "accepted operation for command {:?} has an empty authority_domain_id",
            command_id
        )));
    }
    if operation.idempotency_key.is_empty() {
        return Err(AcceptanceError::CorruptRecord(format!(
            "accepted operation for command {:?} has an empty idempotency_key",
            command_id
        )));
    }
    let target_key = target_key_for(operation).map_err(|error| {
        AcceptanceError::CorruptRecord(format!(
            "accepted operation for command {:?} has an invalid target: {error}",
            command_id
        ))
    })?;

    Ok((
        authority_domain_id.value.clone(),
        operation.idempotency_key.clone(),
        target_key.as_str().to_owned(),
    ))
}

impl crate::acceptance::CommandStateLookup for CommandIndex {
    async fn current_state(
        &self,
        command_id: &CommandId,
    ) -> Option<crate::acceptance::CommandSnapshot> {
        let record = self.commands.get(command_id)?;
        let operation_kind = OperationKind::try_from(record.operation.kind).ok()?;
        Some(crate::acceptance::CommandSnapshot {
            state: record.state,
            operation_kind,
            target_scope: record.operation.target_scope.clone(),
            correlations: record.operation.correlations.clone(),
            terminal_lsn: record.terminal_lsn,
        })
    }
}
