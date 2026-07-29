//! Operation submission and durable acceptance.

use patchbay_contracts::patchbay::{
    AcceptedOperation, ActorEndpointRef, AuthorityDomainId, CommandId, EventId, FailureCode,
    GrantId, IdempotencyKey, Lsn, Operation, OperationKind, OperationState, StoredEventKind,
    StoredEventPayload,
    SubmissionOutcome, SubmissionResult, TargetScope, TargetScopeKind, TimeWindow,
};
use prost::Message;
use prost_types::Timestamp;

use crate::{
    authority::IssuerContext,
    storage::{DedupOutcome, Storage, StorageError, TargetKey},
};

use super::{
    validate_response_payload, AcceptanceError, AllowOperations, Clock, CommandStateLookup,
    ElicitationContractLookup, GrantCheck, OperationPosture, OperationPostureDenied, SystemClock,
    TargetResolver,
};

/// The committed v0.1.0 operation kinds. Reserved wire values deliberately do
/// not appear here, so adding another generated enum variant remains fail
/// closed until its protocol disposition is explicitly promoted.
pub const COMMITTED_OPERATION_KINDS: [OperationKind; 10] = [
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
    operation_kind: OperationKind,
    target_scope: &'a TargetScope,
    idempotency_key: IdempotencyKey,
    target_key: TargetKey,
}

struct ValidationRejection {
    failure_code: FailureCode,
    reason_code: String,
    diagnostic: String,
    decision_grant_id: Option<GrantId>,
}

impl ValidationRejection {
    fn validation_failed(diagnostic: impl Into<String>) -> Self {
        Self {
            failure_code: FailureCode::ValidationFailed,
            reason_code: "validation_failed".to_owned(),
            diagnostic: diagnostic.into(),
            decision_grant_id: None,
        }
    }

    fn expired(diagnostic: impl Into<String>) -> Self {
        Self {
            failure_code: FailureCode::Expired,
            reason_code: "operation_expired".to_owned(),
            diagnostic: diagnostic.into(),
            decision_grant_id: None,
        }
    }
}

/// Submit an operation for acceptance using the production wall clock.
///
/// The ordering is protocol-significant: boundary validation, authority check,
/// target binding, and only then the atomic deduplicating durable append.
/// Every rejection before that append leaves the command log untouched.
pub async fn submit<S, G, R, L, E>(
    storage: &S,
    grant_check: &G,
    target_resolver: &R,
    state_lookup: &L,
    contract_lookup: &E,
    issuer: &dyn IssuerContext,
    operation: Operation,
) -> Result<SubmissionResult, AcceptanceError>
where
    S: Storage,
    G: GrantCheck,
    R: TargetResolver,
    L: CommandStateLookup,
    E: ElicitationContractLookup,
{
    submit_with_clock_and_posture(
        storage,
        grant_check,
        target_resolver,
        state_lookup,
        contract_lookup,
        &AllowOperations,
        issuer,
        operation,
        &SystemClock,
    )
    .await
}

/// Submit an operation using an injected clock.
///
/// The ordering is protocol-significant: boundary and validity-window
/// validation, authority check, target binding, and only then the atomic
/// deduplicating durable append. Every rejection before that append leaves the
/// command log untouched.
#[allow(
    clippy::too_many_arguments,
    reason = "the injected clock extends the existing acceptance boundary without bundling unrelated ports"
)]
pub async fn submit_with_clock<S, G, R, L, E, C>(
    storage: &S,
    grant_check: &G,
    target_resolver: &R,
    state_lookup: &L,
    contract_lookup: &E,
    issuer: &dyn IssuerContext,
    operation: Operation,
    clock: &C,
) -> Result<SubmissionResult, AcceptanceError>
where
    S: Storage,
    G: GrantCheck,
    R: TargetResolver,
    L: CommandStateLookup,
    E: ElicitationContractLookup,
    C: Clock + ?Sized,
{
    submit_with_clock_and_posture(
        storage,
        grant_check,
        target_resolver,
        state_lookup,
        contract_lookup,
        &AllowOperations,
        issuer,
        operation,
        clock,
    )
    .await
}

#[allow(
    clippy::too_many_arguments,
    reason = "the posture port is an explicit acceptance boundary alongside the existing domain ports"
)]
pub async fn submit_with_clock_and_posture<S, G, R, L, E, P, C>(
    storage: &S,
    grant_check: &G,
    target_resolver: &R,
    state_lookup: &L,
    contract_lookup: &E,
    posture: &P,
    issuer: &dyn IssuerContext,
    operation: Operation,
    clock: &C,
) -> Result<SubmissionResult, AcceptanceError>
where
    S: Storage,
    G: GrantCheck,
    R: TargetResolver,
    L: CommandStateLookup,
    E: ElicitationContractLookup,
    P: OperationPosture,
    C: Clock + ?Sized,
{
    let evaluated_at = clock.now();
    let validated = match validate_operation(&operation, &evaluated_at) {
        Ok(validated) => validated,
        Err(rejection) => {
            return Ok(rejected_result(
                operation.command_id.clone(),
                rejection.failure_code,
                rejection.reason_code,
                rejection.decision_grant_id,
                rejection.diagnostic,
            ));
        }
    };

    let verified_sender = match sender_from_verified_issuer(issuer) {
        Some(sender) => sender,
        None => {
            return Ok(rejected_result(
                Some(validated.command_id.clone()),
                FailureCode::AuthorizationDenied,
                "authorization_denied".to_owned(),
                None,
                "operation issuer identity is incomplete".to_owned(),
            ));
        }
    };

    if let Err(OperationPostureDenied::SecurityLockdown {
        reason_code,
        entered_event_id,
    }) = posture.check(validated.authority_domain_id).await
    {
        return Ok(rejected_result(
            Some(validated.command_id.clone()),
            FailureCode::AuthorizationDenied,
            "security_lockdown_active".to_owned(),
            None,
            format!("security lockdown is active: {reason_code} (entered at {:?})", entered_event_id.lsn),
        ));
    }

    if matches!(
        validated.operation_kind,
        OperationKind::ElicitationResponse | OperationKind::ApprovalResponse
    ) {
        let elicitation_id =
            operation.correlations.iter().find_map(|correlation| {
                match correlation.r#ref.as_ref() {
                    Some(patchbay_contracts::patchbay::typed_correlation::Ref::ElicitationId(
                        id,
                    )) => Some(id),
                    _ => None,
                }
            });
        let active = match elicitation_id {
            Some(elicitation_id) => contract_lookup.active_contract(elicitation_id).await,
            None => None,
        };
        if let Err(diagnostic) = validate_response_payload(&operation, active.as_ref()) {
            return Ok(rejected_result(
                Some(validated.command_id.clone()),
                FailureCode::ValidationFailed,
                "validation_failed".to_owned(),
                None,
                diagnostic,
            ));
        }
    }

    let authorization = match grant_check
        .check_at(
            validated.authority_domain_id,
            issuer,
            validated.operation_kind,
            validated.target_scope,
            &evaluated_at,
        )
        .await
    {
        Ok(authorized) => authorized,
        Err(crate::acceptance::GrantDenied::NoGrant { actor, .. }) => {
            let (failure, reason_code, decision_grant_id, diagnostic) =
                if let Some(value) = actor.strip_prefix("grant_expired:") {
                    (FailureCode::Expired, "grant_expired", Some(GrantId { value: value.to_owned() }), "grant is expired")
                } else if let Some(value) = actor.strip_prefix("grant_revoked:") {
                    (FailureCode::AuthorizationDenied, "grant_revoked", Some(GrantId { value: value.to_owned() }), "grant is revoked")
                } else {
                    (FailureCode::AuthorizationDenied, "authorization_denied", None, "no matching live grant")
                };
            return Ok(rejected_result(
                Some(validated.command_id.clone()),
                failure,
                reason_code.to_owned(),
                decision_grant_id,
                diagnostic.to_owned(),
            ));
        }
    };
    let grant_id = authorization.grant_id.ok_or_else(|| {
        AcceptanceError::CorruptRecord("grant check authorized without grant provenance".to_owned())
    })?;

    if target_resolver
        .resolve(validated.authority_domain_id, validated.target_scope)
        .await
        .is_err()
    {
        return Ok(rejected_result(
            Some(validated.command_id.clone()),
            FailureCode::TargetNotFound,
            "target_not_found".to_owned(),
            None,
            "operation target could not be resolved".to_owned(),
        ));
    }

    // Sender is an audit attribution field, but it must never preserve a
    // caller-supplied identity claim. Persist only the identity established by
    // the authenticated ingress boundary.
    let mut durable_operation = operation.clone();
    durable_operation.sender = Some(verified_sender);
    let accepted_operation = AcceptedOperation {
        operation: Some(durable_operation),
        authorizing_grant_id: Some(grant_id.clone()),
    };
    let payload = StoredEventPayload {
        kind: StoredEventKind::Operation as i32,
        payload: accepted_operation.encode_to_vec(),
    };
    let logical_operation_bytes = operation.encode_to_vec();

    let append_result = storage
        .append_dedup_with_payload(
            validated.authority_domain_id,
            &validated.idempotency_key,
            &validated.target_key,
            payload,
            logical_operation_bytes,
        )
        .await;

    match append_result {
        Ok(DedupOutcome::Appended(event_id)) => accepted_result(
            validated.command_id.clone(),
            validated.authority_domain_id,
            event_id,
            OperationState::Accepted,
            grant_id,
            false,
        ),
        Ok(DedupOutcome::Duplicate(event_id)) => {
            // A retry returns the EXISTING command's state, not a hardcoded
            // Accepted. The command may have advanced (delivered, running,
            // or terminal) since the original accept. Look it up.
            //
            // If the lookup returns None, the command exists in the durable
            // log (storage said Duplicate) but not in the in-memory index —
            // an inconsistency. Fail fast rather than silently returning
            // Accepted (which would reproduce the original blocker).
            let snapshot = state_lookup
                .current_state(validated.command_id)
                .await
                .ok_or_else(|| {
                    AcceptanceError::CorruptRecord(format!(
                        "duplicate submission for command {:?} not found in the command index",
                        validated.command_id
                    ))
                })?;
            accepted_result(
                validated.command_id.clone(),
                validated.authority_domain_id,
                event_id,
                snapshot.state,
                grant_id,
                true,
            )
        }
        Err(StorageError::IdempotencyConflict) => Ok(rejected_result(
            Some(validated.command_id.clone()),
            FailureCode::ValidationFailed,
            "validation_failed".to_owned(),
            None,
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

fn validate_operation<'a>(
    operation: &'a Operation,
    now: &Timestamp,
) -> Result<ValidatedOperation<'a>, ValidationRejection> {
    let operation_kind = OperationKind::try_from(operation.kind)
        .ok()
        .filter(|kind| COMMITTED_OPERATION_KINDS.contains(kind))
        .ok_or_else(|| {
            ValidationRejection::validation_failed(
                "operation kind is unknown or unavailable in v0.1.0",
            )
        })?;

    let command_id = operation
        .command_id
        .as_ref()
        .ok_or_else(|| ValidationRejection::validation_failed("operation is missing command_id"))?;
    if command_id.value.is_empty() {
        return Err(ValidationRejection::validation_failed(
            "operation command_id is empty",
        ));
    }

    let authority_domain_id = operation.authority_domain_id.as_ref().ok_or_else(|| {
        ValidationRejection::validation_failed("operation is missing authority_domain_id")
    })?;
    if authority_domain_id.value.is_empty() {
        return Err(ValidationRejection::validation_failed(
            "operation authority_domain_id is empty",
        ));
    }

    operation
        .sender
        .as_ref()
        .ok_or_else(|| ValidationRejection::validation_failed("operation is missing sender"))?;
    let target_scope = operation.target_scope.as_ref().ok_or_else(|| {
        ValidationRejection::validation_failed("operation is missing target_scope")
    })?;
    let target_scope_kind = TargetScopeKind::try_from(target_scope.kind).map_err(|_| {
        ValidationRejection::validation_failed("operation target_scope has an unknown kind")
    })?;
    if target_scope_kind == TargetScopeKind::Unspecified {
        return Err(ValidationRejection::validation_failed(
            "operation target_scope kind is unspecified",
        ));
    }

    if operation.idempotency_key.is_empty() {
        return Err(ValidationRejection::validation_failed(
            "operation is missing idempotency_key",
        ));
    }

    validate_validity_window(operation, now)?;

    let target_key = target_key_for(operation)
        .map_err(|error| ValidationRejection::validation_failed(error.to_string()))?;

    Ok(ValidatedOperation {
        command_id,
        authority_domain_id,
        operation_kind,
        target_scope,
        idempotency_key: IdempotencyKey {
            value: operation.idempotency_key.clone(),
        },
        target_key,
    })
}

fn validate_validity_window(
    operation: &Operation,
    now: &Timestamp,
) -> Result<(), ValidationRejection> {
    validate_timestamp("acceptance clock", now)?;
    let TimeWindow {
        starts_at,
        expires_at,
    } = operation.validity_window.as_ref().ok_or_else(|| {
        ValidationRejection::validation_failed("operation is missing validity_window")
    })?;
    let starts_at = starts_at.as_ref().ok_or_else(|| {
        ValidationRejection::validation_failed("operation validity_window is missing starts_at")
    })?;
    let expires_at = expires_at.as_ref().ok_or_else(|| {
        ValidationRejection::validation_failed("operation validity_window is missing expires_at")
    })?;
    let submitted_at = operation.submitted_at.as_ref().ok_or_else(|| {
        ValidationRejection::validation_failed("operation is missing submitted_at")
    })?;

    validate_timestamp("operation validity_window.starts_at", starts_at)?;
    validate_timestamp("operation validity_window.expires_at", expires_at)?;
    validate_timestamp("operation submitted_at", submitted_at)?;

    if timestamp_key(starts_at) >= timestamp_key(expires_at) {
        return Err(ValidationRejection::validation_failed(
            "operation validity_window must have starts_at before expires_at",
        ));
    }
    if timestamp_key(submitted_at) < timestamp_key(starts_at)
        || timestamp_key(submitted_at) >= timestamp_key(expires_at)
    {
        return Err(ValidationRejection::validation_failed(
            "operation submitted_at is outside validity_window",
        ));
    }
    if timestamp_key(now) < timestamp_key(starts_at) {
        return Err(ValidationRejection::validation_failed(
            "operation validity_window is not active yet",
        ));
    }
    if timestamp_key(now) >= timestamp_key(expires_at) {
        return Err(ValidationRejection::expired(
            "operation validity_window has expired",
        ));
    }
    if timestamp_key(submitted_at) > timestamp_key(now) {
        return Err(ValidationRejection::validation_failed(
            "operation submitted_at is in the future",
        ));
    }

    Ok(())
}

fn validate_timestamp(field: &str, timestamp: &Timestamp) -> Result<(), ValidationRejection> {
    const MIN_SECONDS: i64 = -62_135_596_800;
    const MAX_SECONDS: i64 = 253_402_300_799;
    if !(MIN_SECONDS..=MAX_SECONDS).contains(&timestamp.seconds)
        || !(0..1_000_000_000).contains(&timestamp.nanos)
    {
        return Err(ValidationRejection::validation_failed(format!(
            "{field} is not a valid protobuf Timestamp"
        )));
    }
    Ok(())
}

fn timestamp_key(timestamp: &Timestamp) -> (i64, i32) {
    (timestamp.seconds, timestamp.nanos)
}

fn sender_from_verified_issuer(issuer: &dyn IssuerContext) -> Option<ActorEndpointRef> {
    Some(ActorEndpointRef {
        actor_id: Some(issuer.verified_actor()?.clone()),
        endpoint_id: Some(issuer.verified_endpoint()?.clone()),
        device_id: Some(issuer.verified_device()?.clone()),
        endpoint_generation: Some(issuer.endpoint_generation()?),
    })
}

fn rejected_result(
    command_id: Option<CommandId>,
    failure_code: FailureCode,
    reason_code: String,
    decision_grant_id: Option<GrantId>,
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
        decision_grant_id,
        reason_code,
    }
}

fn accepted_result(
    command_id: CommandId,
    expected_domain: &AuthorityDomainId,
    event_id: EventId,
    operation_state: OperationState,
    grant_id: GrantId,
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
        decision_grant_id: Some(grant_id),
        reason_code: "accepted".to_owned(),
    })
}
