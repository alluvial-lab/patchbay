//! Neutral validation for generated contracts shared across core domains.
//!
//! This module depends only on generated boundary types. Acceptance and
//! authority replay both consume it without importing validation through one
//! another.

use patchbay_contracts::patchbay::{
    ContinuationAuthorityProvenance, ExternalRuntimeRef, GrantId, OperationKind,
    RuntimeGenerationRef,
};

const MAX_DEPLOYMENT_SCOPE_BYTES: usize = 256;

/// Structural failures in exact-prior continuation authority provenance.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContinuationProvenanceError {
    #[error("compound spawn authority is missing the adapter-scoped spawning Grant id")]
    MissingSpawningGrant,
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
    #[error("continuation authority is missing the replacement Grant id")]
    MissingReplacementGrant,
    #[error("continuation authority must carry two distinct Grant ids")]
    ReusedSpawningGrant,
    #[error("continuation replacement authority kind must be session-management")]
    WrongReplacementAuthorityKind,
}

/// Validate self-contained descendant continuation provenance.
///
/// This does not look up or select either Grant. The operation-aware authority
/// decision remains responsible for proving liveness, exact scope, and common
/// verified subject, endpoint, and authority domain.
pub fn validate_continuation_authority_provenance(
    spawning_grant_id: &GrantId,
    provenance: &ContinuationAuthorityProvenance,
) -> Result<(), ContinuationProvenanceError> {
    if spawning_grant_id.value.is_empty() {
        return Err(ContinuationProvenanceError::MissingSpawningGrant);
    }
    let exact_prior = provenance
        .exact_prior
        .as_ref()
        .ok_or(ContinuationProvenanceError::MissingExactPrior)?;
    validate_runtime_generation_ref(exact_prior)?;
    let replacement_grant_id = provenance
        .replacement_grant_id
        .as_ref()
        .filter(|grant_id| !grant_id.value.is_empty())
        .ok_or(ContinuationProvenanceError::MissingReplacementGrant)?;
    if replacement_grant_id == spawning_grant_id {
        return Err(ContinuationProvenanceError::ReusedSpawningGrant);
    }
    if provenance.replacement_authority_kind != OperationKind::SessionManagement as i32 {
        return Err(ContinuationProvenanceError::WrongReplacementAuthorityKind);
    }
    Ok(())
}

/// Validate the exact logical/runtime generation shape shared by a continuation
/// request and its durable authority provenance.
pub fn validate_runtime_generation_ref(
    prior: &RuntimeGenerationRef,
) -> Result<(), ContinuationProvenanceError> {
    if prior
        .logical_target_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(ContinuationProvenanceError::EmptyLogicalTargetId);
    }
    let external = prior
        .external_runtime
        .as_ref()
        .ok_or(ContinuationProvenanceError::MissingExternalRuntime)?;
    validate_external_runtime_ref(external)
}

fn validate_external_runtime_ref(
    external: &ExternalRuntimeRef,
) -> Result<(), ContinuationProvenanceError> {
    if external
        .adapter_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(ContinuationProvenanceError::EmptyAdapterId);
    }
    if external.deployment_scope.is_empty()
        || external.deployment_scope.len() > MAX_DEPLOYMENT_SCOPE_BYTES
        || !external
            .deployment_scope
            .bytes()
            .all(|byte| byte.is_ascii_graphic())
    {
        return Err(ContinuationProvenanceError::MalformedDeploymentScope);
    }
    if external
        .runtime_session_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
    {
        return Err(ContinuationProvenanceError::EmptyRuntimeSessionId);
    }
    let generation = external
        .generation
        .as_ref()
        .map_or(0, |generation| generation.value);
    if generation == 0 {
        return Err(ContinuationProvenanceError::NonPositiveGeneration);
    }
    if generation == u64::MAX {
        return Err(ContinuationProvenanceError::GenerationOverflow);
    }
    Ok(())
}
