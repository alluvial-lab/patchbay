use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    AdapterCapability, AdapterSnapshotSupport, AdapterTargetCategory, PayloadContentType,
    PayloadEnvelope, ResourceCapability, ResourceKind, ResourceProjectionContract,
    SchemaDescriptor,
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
    target_categories: HashSet<AdapterTargetCategory>,
    resources: HashMap<ResourceKind, ValidatedResourceCapability>,
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
        let session_snapshot = AdapterSnapshotSupport::try_from(
            capability.session_snapshot_support,
        )
        .map_err(|_| {
            CapabilityValidationError::UnknownSessionSnapshotSupport(
                capability.session_snapshot_support,
            )
        })?;

        let legacy_session_only = context == CapabilityValidationContext::Replay
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
            target_categories,
            resources,
        })
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
