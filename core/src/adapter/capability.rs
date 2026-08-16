use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    adapter_assurance_manifest, AdapterAssuranceManifest, AdapterAssuranceManifestV1,
    AdapterCapability, AdapterReconciliationStrength, AdapterSnapshotSupport,
    AdapterTargetCategory, FailureCode, IdempotencyStrength, OperationKind, PayloadContentType,
    PayloadEnvelope, ReconciliationAction, ResourceCapability, ResourceKind,
    ResourceProjectionContract, SchemaDescriptor,
};

const MAX_RESOURCE_CAPABILITIES: usize = 128;
const MAX_SCHEMA_REF_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityValidationContext {
    Attach,
    Replay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedAdapterCapability {
    assurance: ValidatedAdapterAssurance,
    target_categories: HashSet<AdapterTargetCategory>,
    resources: HashMap<ResourceKind, ValidatedResourceCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedAdapterAssurance {
    deduplication_strength: IdempotencyStrength,
    continuation_proof_support: bool,
    cursor_support: bool,
    generation_fence_support: bool,
    reconciliation_strength: AdapterReconciliationStrength,
    unproven_outcome_action: ReconciliationAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedResourceCapability {
    resource_kind: ResourceKind,
    snapshot_support: AdapterSnapshotSupport,
    projection_contract: ValidatedProjectionContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedProjectionContract {
    target_category: AdapterTargetCategory,
    payload_schema: ValidatedSchemaDescriptor,
    projection_schema: ValidatedSchemaDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSchemaDescriptor {
    schema_ref: String,
    content_type: PayloadContentType,
}

impl ValidatedAdapterCapability {
    pub fn try_from_wire(
        capability: &AdapterCapability,
        context: CapabilityValidationContext,
    ) -> Result<Self, CapabilityValidationError> {
        let assurance = ValidatedAdapterAssurance::try_from_wire(capability, context)?;
        validate_operation_kinds(&capability.supported_operation_kinds)?;
        validate_known_failure_modes(&capability.known_failure_modes)?;
        let session_snapshot = AdapterSnapshotSupport::try_from(
            capability.session_snapshot_support,
        )
        .map_err(|_| {
            CapabilityValidationError::UnknownSessionSnapshotSupport(
                capability.session_snapshot_support,
            )
        })?;

        let legacy_session_only = context == CapabilityValidationContext::Replay
            && capability.assurance.is_none()
            && capability.target_categories.is_empty()
            && capability.resource_capabilities.is_empty();

        let mut target_categories = HashSet::new();
        if legacy_session_only {
            target_categories.insert(AdapterTargetCategory::RuntimeSession);
        } else {
            if capability.target_categories.is_empty() {
                return Err(CapabilityValidationError::MissingTargetCategories);
            }
            for value in &capability.target_categories {
                let category = AdapterTargetCategory::try_from(*value)
                    .map_err(|_| CapabilityValidationError::UnknownTargetCategory(*value))?;
                if !matches!(
                    category,
                    AdapterTargetCategory::RuntimeSession
                        | AdapterTargetCategory::OperationalResource
                ) {
                    return Err(CapabilityValidationError::UnsupportedTargetCategory(
                        category,
                    ));
                }
                if !target_categories.insert(category) {
                    return Err(CapabilityValidationError::DuplicateTargetCategory(category));
                }
            }
        }

        if !legacy_session_only {
            let targets_sessions =
                target_categories.contains(&AdapterTargetCategory::RuntimeSession);
            if targets_sessions == (session_snapshot == AdapterSnapshotSupport::Unspecified) {
                return Err(CapabilityValidationError::SessionSnapshotCategoryMismatch);
            }
        }

        if capability.resource_capabilities.len() > MAX_RESOURCE_CAPABILITIES {
            return Err(CapabilityValidationError::TooManyResourceCapabilities);
        }
        let targets_resources =
            target_categories.contains(&AdapterTargetCategory::OperationalResource);
        if targets_resources == capability.resource_capabilities.is_empty() {
            return Err(CapabilityValidationError::ResourceCategoryMismatch);
        }

        let mut resources = HashMap::new();
        for resource in &capability.resource_capabilities {
            let validated = ValidatedResourceCapability::try_from_wire(resource)?;
            let kind = validated.resource_kind.clone();
            if resources.insert(kind.clone(), validated).is_some() {
                return Err(CapabilityValidationError::DuplicateResourceKind(kind.value));
            }
        }

        Ok(Self {
            assurance,
            target_categories,
            resources,
        })
    }

    #[must_use]
    pub fn assurance(&self) -> ValidatedAdapterAssurance {
        self.assurance
    }

    #[must_use]
    pub fn targets(&self, category: AdapterTargetCategory) -> bool {
        self.target_categories.contains(&category)
    }

    #[must_use]
    pub fn resource(&self, kind: &ResourceKind) -> Option<&ValidatedResourceCapability> {
        self.resources.get(kind)
    }
}

fn validate_operation_kinds(values: &[i32]) -> Result<(), CapabilityValidationError> {
    let mut seen = HashSet::new();
    for value in values {
        let kind = OperationKind::try_from(*value)
            .map_err(|_| CapabilityValidationError::UnknownSupportedOperationKind(*value))?;
        if kind == OperationKind::Unspecified {
            return Err(CapabilityValidationError::UnspecifiedSupportedOperationKind);
        }
        if !seen.insert(kind) {
            return Err(CapabilityValidationError::DuplicateSupportedOperationKind(
                kind,
            ));
        }
    }
    Ok(())
}

fn validate_known_failure_modes(values: &[i32]) -> Result<(), CapabilityValidationError> {
    let mut seen = HashSet::new();
    for value in values {
        let failure = FailureCode::try_from(*value)
            .map_err(|_| CapabilityValidationError::UnknownKnownFailureMode(*value))?;
        if failure == FailureCode::Unspecified {
            return Err(CapabilityValidationError::UnspecifiedKnownFailureMode);
        }
        if !seen.insert(failure) {
            return Err(CapabilityValidationError::DuplicateKnownFailureMode(
                failure,
            ));
        }
    }
    Ok(())
}

impl ValidatedAdapterAssurance {
    #[allow(deprecated)]
    pub fn try_from_wire(
        capability: &AdapterCapability,
        context: CapabilityValidationContext,
    ) -> Result<Self, CapabilityValidationError> {
        let Some(manifest) = capability.assurance.as_ref() else {
            return if context == CapabilityValidationContext::Replay {
                Ok(Self::from_legacy_replay(capability.idempotency_strength))
            } else {
                Err(CapabilityValidationError::MissingAssuranceManifest)
            };
        };
        let contract = manifest
            .contract
            .as_ref()
            .ok_or(CapabilityValidationError::MissingAssuranceContractVersion)?;
        match contract {
            adapter_assurance_manifest::Contract::V1(v1) => {
                let legacy = IdempotencyStrength::try_from(capability.idempotency_strength)
                    .map_err(|_| {
                        CapabilityValidationError::UnknownLegacyDeduplicationStrength(
                            capability.idempotency_strength,
                        )
                    })?;
                if legacy != IdempotencyStrength::Unspecified {
                    return Err(CapabilityValidationError::DualDeduplicationDeclaration);
                }
                Self::try_from_v1(v1)
            }
        }
    }

    fn try_from_v1(
        manifest: &AdapterAssuranceManifestV1,
    ) -> Result<Self, CapabilityValidationError> {
        let deduplication_strength = IdempotencyStrength::try_from(manifest.deduplication_strength)
            .map_err(|_| {
                CapabilityValidationError::UnknownDeduplicationStrength(
                    manifest.deduplication_strength,
                )
            })?;
        if deduplication_strength == IdempotencyStrength::Unspecified {
            return Err(CapabilityValidationError::UnspecifiedDeduplicationStrength);
        }
        let continuation_proof_support = manifest
            .continuation_proof_support
            .ok_or(CapabilityValidationError::MissingContinuationProofSupport)?;
        let cursor_support = manifest
            .cursor_support
            .ok_or(CapabilityValidationError::MissingCursorSupport)?;
        let generation_fence_support = manifest
            .generation_fence_support
            .ok_or(CapabilityValidationError::MissingGenerationFenceSupport)?;
        let reconciliation_strength = AdapterReconciliationStrength::try_from(
            manifest.reconciliation_strength,
        )
        .map_err(|_| {
            CapabilityValidationError::UnknownReconciliationStrength(
                manifest.reconciliation_strength,
            )
        })?;
        if reconciliation_strength == AdapterReconciliationStrength::Unspecified {
            return Err(CapabilityValidationError::UnspecifiedReconciliationStrength);
        }
        let unproven_outcome_action =
            ReconciliationAction::try_from(manifest.unproven_outcome_action).map_err(|_| {
                CapabilityValidationError::UnknownUnprovenOutcomeAction(
                    manifest.unproven_outcome_action,
                )
            })?;
        if unproven_outcome_action == ReconciliationAction::Unspecified {
            return Err(CapabilityValidationError::UnspecifiedUnprovenOutcomeAction);
        }
        Ok(Self {
            deduplication_strength,
            continuation_proof_support,
            cursor_support,
            generation_fence_support,
            reconciliation_strength,
            unproven_outcome_action,
        })
    }

    fn from_legacy_replay(value: i32) -> Self {
        let deduplication_strength = match IdempotencyStrength::try_from(value) {
            Ok(IdempotencyStrength::None) => IdempotencyStrength::None,
            Ok(IdempotencyStrength::AtPatchbayBoundary) => IdempotencyStrength::AtPatchbayBoundary,
            Ok(IdempotencyStrength::EndToEnd) => IdempotencyStrength::EndToEnd,
            Ok(IdempotencyStrength::Unspecified) | Err(_) => IdempotencyStrength::None,
        };
        Self {
            deduplication_strength,
            continuation_proof_support: false,
            cursor_support: false,
            generation_fence_support: false,
            reconciliation_strength: AdapterReconciliationStrength::None,
            unproven_outcome_action: ReconciliationAction::None,
        }
    }

    #[must_use]
    pub fn deduplication_strength(self) -> IdempotencyStrength {
        self.deduplication_strength
    }

    #[must_use]
    pub fn continuation_proof_support(self) -> bool {
        self.continuation_proof_support
    }

    #[must_use]
    pub fn cursor_support(self) -> bool {
        self.cursor_support
    }

    #[must_use]
    pub fn generation_fence_support(self) -> bool {
        self.generation_fence_support
    }

    #[must_use]
    pub fn reconciliation_strength(self) -> AdapterReconciliationStrength {
        self.reconciliation_strength
    }

    #[must_use]
    pub fn unproven_outcome_action(self) -> ReconciliationAction {
        self.unproven_outcome_action
    }

    #[must_use]
    pub fn to_wire_v1(self) -> AdapterAssuranceManifest {
        AdapterAssuranceManifest {
            contract: Some(adapter_assurance_manifest::Contract::V1(
                AdapterAssuranceManifestV1 {
                    deduplication_strength: self.deduplication_strength as i32,
                    continuation_proof_support: Some(self.continuation_proof_support),
                    cursor_support: Some(self.cursor_support),
                    generation_fence_support: Some(self.generation_fence_support),
                    reconciliation_strength: self.reconciliation_strength as i32,
                    unproven_outcome_action: self.unproven_outcome_action as i32,
                },
            )),
        }
    }
}

impl ValidatedResourceCapability {
    fn try_from_wire(capability: &ResourceCapability) -> Result<Self, CapabilityValidationError> {
        let resource_kind = capability
            .resource_kind
            .clone()
            .filter(|kind| !kind.value.is_empty())
            .ok_or(CapabilityValidationError::MissingResourceKind)?;
        let snapshot_support = AdapterSnapshotSupport::try_from(capability.snapshot_support)
            .map_err(|_| {
                CapabilityValidationError::UnknownResourceSnapshotSupport(
                    capability.snapshot_support,
                )
            })?;
        if snapshot_support == AdapterSnapshotSupport::Unspecified {
            return Err(CapabilityValidationError::UnspecifiedResourceSnapshotSupport);
        }
        let projection_contract = capability
            .projection_contract
            .as_ref()
            .ok_or(CapabilityValidationError::MissingProjectionContract)
            .and_then(ValidatedProjectionContract::try_from_wire)?;
        Ok(Self {
            resource_kind,
            snapshot_support,
            projection_contract,
        })
    }

    #[must_use]
    pub fn resource_kind(&self) -> &ResourceKind {
        &self.resource_kind
    }

    #[must_use]
    pub fn snapshot_support(&self) -> AdapterSnapshotSupport {
        self.snapshot_support
    }

    #[must_use]
    pub fn projection_contract(&self) -> &ValidatedProjectionContract {
        &self.projection_contract
    }
}

impl ValidatedProjectionContract {
    fn try_from_wire(
        contract: &ResourceProjectionContract,
    ) -> Result<Self, CapabilityValidationError> {
        let target_category =
            AdapterTargetCategory::try_from(contract.target_category).map_err(|_| {
                CapabilityValidationError::UnknownProjectionTargetCategory(contract.target_category)
            })?;
        if target_category != AdapterTargetCategory::OperationalResource {
            return Err(CapabilityValidationError::ProjectionTargetCategoryMismatch);
        }
        let payload_schema = contract
            .payload_schema
            .as_ref()
            .ok_or(CapabilityValidationError::MissingSchemaDescriptor(
                "payload",
            ))
            .and_then(|schema| ValidatedSchemaDescriptor::try_from_wire("payload", schema))?;
        let projection_schema = contract
            .projection_schema
            .as_ref()
            .ok_or(CapabilityValidationError::MissingSchemaDescriptor(
                "projection",
            ))
            .and_then(|schema| ValidatedSchemaDescriptor::try_from_wire("projection", schema))?;
        Ok(Self {
            target_category,
            payload_schema,
            projection_schema,
        })
    }

    #[must_use]
    pub fn target_category(&self) -> AdapterTargetCategory {
        self.target_category
    }

    #[must_use]
    pub fn payload_schema(&self) -> &ValidatedSchemaDescriptor {
        &self.payload_schema
    }

    #[must_use]
    pub fn projection_schema(&self) -> &ValidatedSchemaDescriptor {
        &self.projection_schema
    }
}

impl ValidatedSchemaDescriptor {
    fn try_from_wire(
        role: &'static str,
        descriptor: &SchemaDescriptor,
    ) -> Result<Self, CapabilityValidationError> {
        if descriptor.schema_ref.is_empty()
            || descriptor.schema_ref.len() > MAX_SCHEMA_REF_BYTES
            || descriptor
                .schema_ref
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        {
            return Err(CapabilityValidationError::InvalidSchemaRef(role));
        }
        let content_type = PayloadContentType::try_from(descriptor.content_type).map_err(|_| {
            CapabilityValidationError::UnknownSchemaContentType {
                role,
                value: descriptor.content_type,
            }
        })?;
        if content_type == PayloadContentType::Unspecified {
            return Err(CapabilityValidationError::UnspecifiedSchemaContentType(
                role,
            ));
        }
        Ok(Self {
            schema_ref: descriptor.schema_ref.clone(),
            content_type,
        })
    }

    #[must_use]
    pub fn schema_ref(&self) -> &str {
        &self.schema_ref
    }

    #[must_use]
    pub fn content_type(&self) -> PayloadContentType {
        self.content_type
    }

    fn matches(&self, envelope: &PayloadEnvelope) -> bool {
        self.schema_ref == envelope.schema_ref && self.content_type as i32 == envelope.content_type
    }
}

pub(crate) fn validate_projection_envelopes(
    capability: &ValidatedResourceCapability,
    payload: &PayloadEnvelope,
    projection: &PayloadEnvelope,
) -> Result<(), CapabilityValidationError> {
    if !capability
        .projection_contract
        .payload_schema
        .matches(payload)
    {
        return Err(CapabilityValidationError::PayloadSchemaMismatch);
    }
    if !capability
        .projection_contract
        .projection_schema
        .matches(projection)
    {
        return Err(CapabilityValidationError::ProjectionSchemaMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityValidationError {
    #[error("manifest is missing the required assurance declaration")]
    MissingAssuranceManifest,
    #[error("assurance declaration is missing a supported contract version")]
    MissingAssuranceContractVersion,
    #[error("legacy and current deduplication declarations cannot be combined")]
    DualDeduplicationDeclaration,
    #[error("legacy deduplication declaration contains unknown value {0}")]
    UnknownLegacyDeduplicationStrength(i32),
    #[error("assurance declaration contains unknown deduplication strength {0}")]
    UnknownDeduplicationStrength(i32),
    #[error("assurance declaration deduplication strength is unspecified")]
    UnspecifiedDeduplicationStrength,
    #[error("assurance declaration is missing continuation_proof_support")]
    MissingContinuationProofSupport,
    #[error("assurance declaration is missing cursor_support")]
    MissingCursorSupport,
    #[error("assurance declaration is missing generation_fence_support")]
    MissingGenerationFenceSupport,
    #[error("assurance declaration contains unknown reconciliation strength {0}")]
    UnknownReconciliationStrength(i32),
    #[error("assurance declaration reconciliation strength is unspecified")]
    UnspecifiedReconciliationStrength,
    #[error("assurance declaration contains unknown unproven-outcome action {0}")]
    UnknownUnprovenOutcomeAction(i32),
    #[error("assurance declaration unproven-outcome action is unspecified")]
    UnspecifiedUnprovenOutcomeAction,
    #[error("manifest contains unknown supported OperationKind {0}")]
    UnknownSupportedOperationKind(i32),
    #[error("manifest supported OperationKind is unspecified")]
    UnspecifiedSupportedOperationKind,
    #[error("manifest contains duplicate supported OperationKind {0:?}")]
    DuplicateSupportedOperationKind(OperationKind),
    #[error("manifest contains unknown known failure mode {0}")]
    UnknownKnownFailureMode(i32),
    #[error("manifest known failure mode is unspecified")]
    UnspecifiedKnownFailureMode,
    #[error("manifest contains duplicate known failure mode {0:?}")]
    DuplicateKnownFailureMode(FailureCode),
    #[error("manifest requires at least one explicit target category")]
    MissingTargetCategories,
    #[error("manifest contains unknown target category {0}")]
    UnknownTargetCategory(i32),
    #[error("manifest target category {0:?} is reserved or unspecified")]
    UnsupportedTargetCategory(AdapterTargetCategory),
    #[error("manifest contains duplicate target category {0:?}")]
    DuplicateTargetCategory(AdapterTargetCategory),
    #[error("session snapshot support must be specified exactly when runtime-session is targeted")]
    SessionSnapshotCategoryMismatch,
    #[error("manifest contains unknown session snapshot support {0}")]
    UnknownSessionSnapshotSupport(i32),
    #[error(
        "operational-resource target category and resource declarations must be present together"
    )]
    ResourceCategoryMismatch,
    #[error("manifest contains more than 128 resource capabilities")]
    TooManyResourceCapabilities,
    #[error("resource capability is missing resource_kind")]
    MissingResourceKind,
    #[error("manifest contains duplicate resource kind {0}")]
    DuplicateResourceKind(String),
    #[error("resource capability contains unknown snapshot support {0}")]
    UnknownResourceSnapshotSupport(i32),
    #[error("resource capability snapshot support is unspecified")]
    UnspecifiedResourceSnapshotSupport,
    #[error("resource capability is missing projection contract")]
    MissingProjectionContract,
    #[error("projection contract contains unknown target category {0}")]
    UnknownProjectionTargetCategory(i32),
    #[error("projection contract must target operational-resource")]
    ProjectionTargetCategoryMismatch,
    #[error("projection contract is missing {0} schema descriptor")]
    MissingSchemaDescriptor(&'static str),
    #[error("{0} schema_ref must be 1..=256 bytes without ASCII control or whitespace")]
    InvalidSchemaRef(&'static str),
    #[error("{role} schema descriptor contains unknown content type {value}")]
    UnknownSchemaContentType { role: &'static str, value: i32 },
    #[error("{0} schema descriptor content type is unspecified")]
    UnspecifiedSchemaContentType(&'static str),
    #[error("resource kind is not declared by the authenticated adapter")]
    UndeclaredResource,
    #[error("resource payload schema descriptor does not match the manifest")]
    PayloadSchemaMismatch,
    #[error("resource projection schema descriptor does not match the manifest")]
    ProjectionSchemaMismatch,
}
