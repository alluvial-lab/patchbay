//! Fail-fast validation for generated spawn payload and compound-authority carriage.
//!
//! This leaf validates structure only. Grant selection, exact target resolution,
//! and same-subject/endpoint/domain checks remain owned by the downstream
//! operation-aware authority decision.

use patchbay_contracts::patchbay::{
    spawn_request, ContinuationAuthorityProvenance, ExternalRuntimeRef, GrantId, Operation,
    OperationKind, PayloadContentType, RuntimeGenerationRef, SpawnRequest, SpawnTargetSpec,
};
use prost::Message;

pub const SPAWN_REQUEST_SCHEMA: &str = "patchbay.SpawnRequest";

const MAX_SHAPE_BYTES: usize = 128;
const MAX_DEPLOYMENT_AUTHORITY_REF_BYTES: usize = 256;
const MAX_ADAPTER_PAYLOAD_BYTES: usize = 1024 * 1024;
const MAX_SCHEMA_REF_BYTES: usize = 256;
const MAX_DEPLOYMENT_SCOPE_BYTES: usize = 256;

/// Structural errors rejected before grant selection, target resolution, or durability.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpawnValidationError {
    #[error("spawn Operation is missing its payload")]
    MissingPayload,
    #[error("spawn Operation payload must use the exact patchbay.SpawnRequest protobuf schema")]
    WrongPayloadContract,
    #[error("cannot decode SpawnRequest: {0}")]
    MalformedPayload(String),
    #[error("SpawnRequest must select exactly one generated intent")]
    MissingIntent,
    #[error("SpawnRequest wire payload carries both fresh and continuation intents")]
    MixedIntent,
    #[error("SpawnRequest is missing target_spec")]
    MissingTargetSpec,
    #[error("spawn target shape must be 1..={MAX_SHAPE_BYTES} printable ASCII bytes")]
    MalformedTargetShape,
    #[error("spawn target adapter_payload exceeds {MAX_ADAPTER_PAYLOAD_BYTES} bytes")]
    AdapterPayloadTooLarge,
    #[error("spawn target adapter_payload has an unknown or unspecified content type")]
    InvalidAdapterPayloadContentType,
    #[error("spawn target adapter_payload schema_ref must be 1..={MAX_SCHEMA_REF_BYTES} bytes without ASCII control or whitespace")]
    InvalidAdapterPayloadSchema,
    #[error("deployment_authority_ref must be at most {MAX_DEPLOYMENT_AUTHORITY_REF_BYTES} printable ASCII bytes")]
    MalformedDeploymentAuthorityRef,
    #[error("continuation is missing its exact prior runtime generation")]
    MissingExactPrior,
    #[error("continuation logical_target_id must not be empty")]
    EmptyLogicalTargetId,
    #[error("continuation exact prior is missing external_runtime")]
    MissingExternalRuntime,
    #[error("continuation exact prior adapter_id must not be empty")]
    EmptyAdapterId,
    #[error("continuation exact prior deployment_scope must be 1..={MAX_DEPLOYMENT_SCOPE_BYTES} printable ASCII bytes")]
    MalformedDeploymentScope,
    #[error("continuation exact prior runtime_session_id must not be empty")]
    EmptyRuntimeSessionId,
    #[error("continuation exact prior generation must be positive")]
    NonPositiveGeneration,
    #[error("continuation exact prior generation cannot be incremented")]
    GenerationOverflow,
    #[error("compound spawn authority is missing the adapter-scoped spawning Grant id")]
    MissingSpawningGrant,
    #[error("fresh spawn must not carry continuation authority")]
    UnexpectedContinuationAuthority,
    #[error("continuation requires exact-prior replacement authority provenance")]
    MissingReplacementAuthority,
    #[error("continuation authority exact_prior differs from the request prior")]
    ReplacementPriorMismatch,
    #[error("continuation authority is missing the replacement Grant id")]
    MissingReplacementGrant,
    #[error("continuation authority must carry two distinct Grant ids")]
    ReusedSpawningGrant,
    #[error("continuation replacement authority kind must be session-management")]
    WrongReplacementAuthorityKind,
}

/// Decode and validate the generated payload for a spawn Operation.
///
/// Shape support remains adapter-owned. This boundary validates only the
/// generated envelope, bounded opaque target fields, and exact continuation
/// identity needed before later authority and target decisions.
pub fn validate_spawn_operation_payload(
    operation: &Operation,
) -> Result<SpawnRequest, SpawnValidationError> {
    let envelope = operation
        .payload
        .as_ref()
        .ok_or(SpawnValidationError::MissingPayload)?;
    if envelope.content_type != PayloadContentType::Protobuf as i32
        || envelope.schema_ref != SPAWN_REQUEST_SCHEMA
    {
        return Err(SpawnValidationError::WrongPayloadContract);
    }

    validate_disjoint_intent_tags(&envelope.payload)?;
    let request = SpawnRequest::decode(envelope.payload.as_slice())
        .map_err(|error| SpawnValidationError::MalformedPayload(error.to_string()))?;
    validate_spawn_request(&request)?;
    Ok(request)
}

/// Validate a decoded generated spawn request without resolving any target.
pub fn validate_spawn_request(request: &SpawnRequest) -> Result<(), SpawnValidationError> {
    validate_target_spec(
        request
            .target_spec
            .as_ref()
            .ok_or(SpawnValidationError::MissingTargetSpec)?,
    )?;

    match request.intent.as_ref() {
        Some(spawn_request::Intent::Fresh(_)) => Ok(()),
        Some(spawn_request::Intent::Continuation(continuation)) => validate_runtime_generation_ref(
            continuation
                .prior
                .as_ref()
                .ok_or(SpawnValidationError::MissingExactPrior)?,
        ),
        None => Err(SpawnValidationError::MissingIntent),
    }
}

/// Validate durable two-Grant carriage after downstream grant selection.
///
/// The spawning Grant is the existing accepted-operation `authorizing_grant_id`.
/// Continuation additionally requires the generated exact-prior replacement
/// provenance. This function deliberately does not look up or select either
/// Grant; the decision owner must still prove both Grants have the same verified
/// subject, endpoint, and authority domain and that the replacement Grant
/// exactly contains the prior runtime generation.
pub fn validate_spawn_authority_carriage(
    request: &SpawnRequest,
    spawning_grant_id: Option<&GrantId>,
    continuation_authority: Option<&ContinuationAuthorityProvenance>,
) -> Result<(), SpawnValidationError> {
    validate_spawn_request(request)?;
    let spawning_grant_id = spawning_grant_id
        .filter(|grant_id| !grant_id.value.is_empty())
        .ok_or(SpawnValidationError::MissingSpawningGrant)?;

    match request.intent.as_ref() {
        Some(spawn_request::Intent::Fresh(_)) => {
            if continuation_authority.is_some() {
                return Err(SpawnValidationError::UnexpectedContinuationAuthority);
            }
            Ok(())
        }
        Some(spawn_request::Intent::Continuation(continuation)) => {
            let prior = continuation
                .prior
                .as_ref()
                .ok_or(SpawnValidationError::MissingExactPrior)?;
            let provenance =
                continuation_authority.ok_or(SpawnValidationError::MissingReplacementAuthority)?;
            validate_continuation_authority_provenance(spawning_grant_id, provenance)?;
            if provenance.exact_prior.as_ref() != Some(prior) {
                return Err(SpawnValidationError::ReplacementPriorMismatch);
            }
            Ok(())
        }
        None => Err(SpawnValidationError::MissingIntent),
    }
}

/// Validate the self-contained descendant continuation provenance shape.
///
/// Exact equality with the accepted request is checked by
/// [`validate_spawn_authority_carriage`]; this narrower function is also used
/// when replaying a descendant Grant from durable storage.
pub fn validate_continuation_authority_provenance(
    spawning_grant_id: &GrantId,
    provenance: &ContinuationAuthorityProvenance,
) -> Result<(), SpawnValidationError> {
    if spawning_grant_id.value.is_empty() {
        return Err(SpawnValidationError::MissingSpawningGrant);
    }
    let exact_prior = provenance
        .exact_prior
        .as_ref()
        .ok_or(SpawnValidationError::MissingExactPrior)?;
    validate_runtime_generation_ref(exact_prior)?;
    let replacement_grant_id = provenance
        .replacement_grant_id
        .as_ref()
        .filter(|grant_id| !grant_id.value.is_empty())
        .ok_or(SpawnValidationError::MissingReplacementGrant)?;
    if replacement_grant_id == spawning_grant_id {
        return Err(SpawnValidationError::ReusedSpawningGrant);
    }
    if OperationKind::try_from(provenance.replacement_authority_kind).ok()
        != Some(OperationKind::SessionManagement)
    {
        return Err(SpawnValidationError::WrongReplacementAuthorityKind);
    }
    Ok(())
}

fn validate_disjoint_intent_tags(payload: &[u8]) -> Result<(), SpawnValidationError> {
    let mut cursor = 0;
    let mut saw_fresh = false;
    let mut saw_continuation = false;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor).ok_or_else(malformed_framing)?;
        let field = key >> 3;
        let wire_type = (key & 0x07) as u8;
        if field == 0 {
            return Err(malformed_framing());
        }
        match field {
            1 if wire_type == 2 => saw_fresh = true,
            2 if wire_type == 2 => saw_continuation = true,
            1 | 2 => return Err(malformed_framing()),
            _ => {}
        }
        skip_wire_value(payload, &mut cursor, wire_type).ok_or_else(malformed_framing)?;
    }
    if saw_fresh && saw_continuation {
        return Err(SpawnValidationError::MixedIntent);
    }
    Ok(())
}

fn read_varint(payload: &[u8], cursor: &mut usize) -> Option<u64> {
    let mut value = 0_u64;
    for shift in (0..=63).step_by(7) {
        let byte = *payload.get(*cursor)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return None;
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

fn skip_wire_value(payload: &[u8], cursor: &mut usize, wire_type: u8) -> Option<()> {
    let bytes = match wire_type {
        0 => {
            read_varint(payload, cursor)?;
            return Some(());
        }
        1 => 8,
        2 => usize::try_from(read_varint(payload, cursor)?).ok()?,
        5 => 4,
        _ => return None,
    };
    *cursor = cursor.checked_add(bytes)?;
    (*cursor <= payload.len()).then_some(())
}

fn malformed_framing() -> SpawnValidationError {
    SpawnValidationError::MalformedPayload("invalid protobuf framing".to_owned())
}

fn validate_target_spec(target: &SpawnTargetSpec) -> Result<(), SpawnValidationError> {
    if !bounded_graphic(&target.shape, MAX_SHAPE_BYTES, false) {
        return Err(SpawnValidationError::MalformedTargetShape);
    }
    if !bounded_graphic(
        &target.deployment_authority_ref,
        MAX_DEPLOYMENT_AUTHORITY_REF_BYTES,
        true,
    ) {
        return Err(SpawnValidationError::MalformedDeploymentAuthorityRef);
    }

    if let Some(adapter_payload) = target.adapter_payload.as_ref() {
        if adapter_payload.payload.len() > MAX_ADAPTER_PAYLOAD_BYTES {
            return Err(SpawnValidationError::AdapterPayloadTooLarge);
        }
        let content_type = PayloadContentType::try_from(adapter_payload.content_type)
            .map_err(|_| SpawnValidationError::InvalidAdapterPayloadContentType)?;
        if content_type == PayloadContentType::Unspecified {
            return Err(SpawnValidationError::InvalidAdapterPayloadContentType);
        }
        if adapter_payload.schema_ref.is_empty()
            || adapter_payload.schema_ref.len() > MAX_SCHEMA_REF_BYTES
            || adapter_payload
                .schema_ref
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        {
            return Err(SpawnValidationError::InvalidAdapterPayloadSchema);
        }
    }
    Ok(())
}

fn validate_runtime_generation_ref(
    prior: &RuntimeGenerationRef,
) -> Result<(), SpawnValidationError> {
    if prior
        .logical_target_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(SpawnValidationError::EmptyLogicalTargetId);
    }
    let external = prior
        .external_runtime
        .as_ref()
        .ok_or(SpawnValidationError::MissingExternalRuntime)?;
    validate_external_runtime_ref(external)
}

fn validate_external_runtime_ref(
    external: &ExternalRuntimeRef,
) -> Result<(), SpawnValidationError> {
    if external
        .adapter_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(SpawnValidationError::EmptyAdapterId);
    }
    if !bounded_graphic(
        &external.deployment_scope,
        MAX_DEPLOYMENT_SCOPE_BYTES,
        false,
    ) {
        return Err(SpawnValidationError::MalformedDeploymentScope);
    }
    if external
        .runtime_session_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(SpawnValidationError::EmptyRuntimeSessionId);
    }
    let generation = external
        .generation
        .as_ref()
        .map_or(0, |generation| generation.value);
    if generation == 0 {
        return Err(SpawnValidationError::NonPositiveGeneration);
    }
    if generation == u64::MAX {
        return Err(SpawnValidationError::GenerationOverflow);
    }
    Ok(())
}

fn bounded_graphic(value: &str, max_bytes: usize, allow_empty: bool) -> bool {
    (allow_empty || !value.is_empty())
        && value.len() <= max_bytes
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}
