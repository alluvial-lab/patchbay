//! Operation submission and durable acceptance.

use patchbay_contracts::patchbay::{
    ActorEndpointRef, AuthorityDomainId, CommandId, EventId, FailureCode, IdempotencyKey, Lsn,
    Operation, OperationKind, OperationState, StoredEventKind, StoredEventPayload,
    SubmissionOutcome, SubmissionResult, TargetScope, TargetScopeKind,
};
use prost::Message;

use crate::storage::{DedupOutcome, Storage, StorageError, TargetKey};

use super::{AcceptanceError, CommandStateLookup, GrantCheck, TargetResolver};

/// The committed v0.1.0 operation kinds. Reserved wire values deliberately do
/// not appear here, so adding another generated enum variant remains fail
/// closed until its protocol disposition is explicitly promoted.
const ACCEPTED_OPERATION_KINDS: [OperationKind; 10] = [
    OperationKind::Spawn,
    OperationKind::Attach,
    OperationKind::Instruct,
    OperationKind::Cancel,
    OperationKind::Interrupt,
    OperationKind::Query,
    OperationKind::ApprovalResponse,
    OperationKind::ElicitationResponse,
    OperationKind::Reconfigure,
    OperationKind::SessionManagement,
];

struct ValidatedOperation<'a> {
    command_id: &'a CommandId,
    authority_domain_id: &'a AuthorityDomainId,
    sender: &'a ActorEndpointRef,
    operation_kind: OperationKind,
    target_scope: &'a TargetScope,
    idempotency_key: IdempotencyKey,
    target_key: TargetKey,
}

/// Submit an operation for acceptance.
///
/// The ordering is protocol-significant: boundary validation, authority check,
/// target binding, and only then the atomic deduplicating durable append.
/// Every rejection before that append leaves the command log untouched.
pub async fn submit<S, G, R, L>(
    storage: &S,
    grant_check: &G,
    target_resolver: &R,
    state_lookup: &L,
    operation: Operation,
) -> Result<SubmissionResult, AcceptanceError>
where
    S: Storage,
    G: GrantCheck,
    R: TargetResolver,
    L: CommandStateLookup,
{
    let validated = match validate_operation(&operation) {
        Ok(validated) => validated,
        Err(diagnostic) => {
            return Ok(rejected_result(
                operation.command_id.clone(),
                FailureCode::ValidationFailed,
                diagnostic,
            ));
        }
    };

    if grant_check
        .check(
            validated.authority_domain_id,
            validated.sender,
            validated.operation_kind,
            validated.target_scope,
        )
        .await
        .is_err()
    {
        return Ok(rejected_result(
            Some(validated.command_id.clone()),
            FailureCode::AuthorizationDenied,
            "operation is not authorized for this actor and target".to_owned(),
        ));
    }

    if target_resolver
        .resolve(validated.authority_domain_id, validated.target_scope)
        .await
        .is_err()
    {
        return Ok(rejected_result(
            Some(validated.command_id.clone()),
            FailureCode::TargetNotFound,
            "operation target could not be resolved".to_owned(),
        ));
    }

    let payload = StoredEventPayload {
        kind: StoredEventKind::Operation as i32,
        payload: operation.encode_to_vec(),
    };

    let append_result = storage
        .append_dedup(
            validated.authority_domain_id,
            &validated.idempotency_key,
            &validated.target_key,
            payload,
        )
        .await;

    match append_result {
        Ok(DedupOutcome::Appended(event_id)) => accepted_result(
            validated.command_id.clone(),
            validated.authority_domain_id,
            event_id,
            OperationState::Accepted,
            false,
        ),
        Ok(DedupOutcome::Duplicate(event_id)) => {
            // A retry returns the EXISTING command's state, not a hardcoded
            // Accepted. The command may have advanced (delivered, running,
            // or terminal) since the original accept. Look it up.
            let existing_state = state_lookup
                .current_state(validated.command_id)
                .await
                .map(|snapshot| snapshot.state)
                .unwrap_or(OperationState::Accepted);
            accepted_result(
                validated.command_id.clone(),
                validated.authority_domain_id,
                event_id,
                existing_state,
                true,
            )
        }
        Err(StorageError::IdempotencyConflict) => Ok(rejected_result(
            Some(validated.command_id.clone()),
            FailureCode::ValidationFailed,
            "idempotency key was already used with a different operation payload".to_owned(),
        )),
        Err(error) => Err(AcceptanceError::Storage(error)),
    }
}

/// Produce the canonical per-target idempotency scope for an operation.
///
/// Prost emits protobuf fields in tag order and `TargetScope` contains no map
/// fields, so its ordinary encoding is deterministic. Hex makes those bytes a
/// stable, storage-safe string without relying on display labels or metadata
/// outside the generated target-scope contract.
pub fn target_key_for(operation: &Operation) -> Result<TargetKey, AcceptanceError> {
    let target_scope = operation.target_scope.as_ref().ok_or_else(|| {
        AcceptanceError::InvalidTargetScope("operation is missing target_scope".to_owned())
    })?;
    let bytes = target_scope.encode_to_vec();
    let mut canonical = String::with_capacity(bytes.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        canonical.push(HEX[(byte >> 4) as usize] as char);
        canonical.push(HEX[(byte & 0x0f) as usize] as char);
    }

    TargetKey::new(canonical).ok_or_else(|| {
        AcceptanceError::InvalidTargetScope(
            "target_scope has no canonical identity fields".to_owned(),
        )
    })
}

fn validate_operation(operation: &Operation) -> Result<ValidatedOperation<'_>, String> {
    let operation_kind = OperationKind::try_from(operation.kind)
        .ok()
        .filter(|kind| ACCEPTED_OPERATION_KINDS.contains(kind))
        .ok_or_else(|| "operation kind is unknown or unavailable in v0.1.0".to_owned())?;

    let command_id = operation
        .command_id
        .as_ref()
        .ok_or_else(|| "operation is missing command_id".to_owned())?;
    if command_id.value.is_empty() {
        return Err("operation command_id is empty".to_owned());
    }

    let authority_domain_id = operation
        .authority_domain_id
        .as_ref()
        .ok_or_else(|| "operation is missing authority_domain_id".to_owned())?;
    if authority_domain_id.value.is_empty() {
        return Err("operation authority_domain_id is empty".to_owned());
    }

    let sender = operation
        .sender
        .as_ref()
        .ok_or_else(|| "operation is missing sender".to_owned())?;
    let target_scope = operation
        .target_scope
        .as_ref()
        .ok_or_else(|| "operation is missing target_scope".to_owned())?;
    let target_scope_kind = TargetScopeKind::try_from(target_scope.kind)
        .map_err(|_| "operation target_scope has an unknown kind".to_owned())?;
    if target_scope_kind == TargetScopeKind::Unspecified {
        return Err("operation target_scope kind is unspecified".to_owned());
    }

    if operation.idempotency_key.is_empty() {
        return Err("operation is missing idempotency_key".to_owned());
    }

    let target_key = target_key_for(operation).map_err(|error| error.to_string())?;

    Ok(ValidatedOperation {
        command_id,
        authority_domain_id,
        sender,
        operation_kind,
        target_scope,
        idempotency_key: IdempotencyKey {
            value: operation.idempotency_key.clone(),
        },
        target_key,
    })
}

fn rejected_result(
    command_id: Option<CommandId>,
    failure_code: FailureCode,
    diagnostic_message: String,
) -> SubmissionResult {
    SubmissionResult {
        outcome: SubmissionOutcome::Rejected as i32,
        command_id,
        operation_state: OperationState::Unspecified as i32,
        failure_code: failure_code as i32,
        diagnostic_message,
        accepted_lsn: None,
        deduplicated: false,
    }
}

fn accepted_result(
    command_id: CommandId,
    expected_domain: &AuthorityDomainId,
    event_id: EventId,
    operation_state: OperationState,
    deduplicated: bool,
) -> Result<SubmissionResult, AcceptanceError> {
    match event_id.authority_domain_id.as_ref() {
        Some(actual_domain) if actual_domain == expected_domain => {}
        Some(actual_domain) => {
            return Err(AcceptanceError::CorruptRecord(format!(
                "storage returned acceptance event for domain {:?}, expected {:?}",
                actual_domain, expected_domain
            )));
        }
        None => {
            return Err(AcceptanceError::CorruptRecord(
                "storage returned acceptance event without authority_domain_id".to_owned(),
            ));
        }
    }
    let accepted_lsn: Lsn = event_id.lsn.ok_or_else(|| {
        AcceptanceError::CorruptRecord(
            "storage returned acceptance event without an LSN".to_owned(),
        )
    })?;

    Ok(SubmissionResult {
        outcome: SubmissionOutcome::Accepted as i32,
        command_id: Some(command_id),
        operation_state: operation_state as i32,
        failure_code: FailureCode::Unspecified as i32,
        diagnostic_message: String::new(),
        accepted_lsn: Some(accepted_lsn),
        deduplicated,
    })
}
