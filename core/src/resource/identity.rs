use patchbay_contracts::patchbay::{
    AdapterId, ResourceId, ResourceIdentity as WireResourceIdentity, ResourceKind, TargetScope,
    TargetScopeKind,
};

/// Canonical routable identity for an adapter-owned operational resource.
///
/// The generated wrappers preserve the three distinct wire id spaces while
/// private fields prevent unchecked domain identities from entering a
/// registry or authority decision.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceIdentity {
    adapter_id: AdapterId,
    resource_id: ResourceId,
    resource_kind: ResourceKind,
}

impl ResourceIdentity {
    pub fn new(
        adapter_id: AdapterId,
        resource_kind: ResourceKind,
        resource_id: ResourceId,
    ) -> Result<Self, ResourceIdentityError> {
        if adapter_id.value.is_empty() {
            return Err(ResourceIdentityError::Missing {
                field: "adapter_id",
            });
        }
        if resource_kind.value.is_empty() {
            return Err(ResourceIdentityError::Missing {
                field: "resource_kind",
            });
        }
        if resource_id.value.is_empty() {
            return Err(ResourceIdentityError::Missing {
                field: "resource_id",
            });
        }
        Ok(Self {
            adapter_id,
            resource_id,
            resource_kind,
        })
    }

    pub fn try_from_wire(resource: &WireResourceIdentity) -> Result<Self, ResourceIdentityError> {
        Self::new(
            resource
                .adapter_id
                .clone()
                .ok_or(ResourceIdentityError::Missing {
                    field: "adapter_id",
                })?,
            resource
                .resource_kind
                .clone()
                .ok_or(ResourceIdentityError::Missing {
                    field: "resource_kind",
                })?,
            resource
                .resource_id
                .clone()
                .ok_or(ResourceIdentityError::Missing {
                    field: "resource_id",
                })?,
        )
    }

    pub fn try_from_scope(scope: &TargetScope) -> Result<Self, ResourceIdentityError> {
        if TargetScopeKind::try_from(scope.kind).ok() != Some(TargetScopeKind::Resource) {
            return Err(ResourceIdentityError::WrongTargetKind);
        }
        if !scope.legacy_audit_resource_id.is_empty() {
            return Err(ResourceIdentityError::LegacyAuditOnly);
        }
        if scope.actor_id.is_some()
            || scope.adapter_id.is_some()
            || scope.runtime_session_id.is_some()
            || scope.session_generation.is_some()
            || !scope.deployment_scope.is_empty()
            || !scope.project_or_group.is_empty()
        {
            return Err(ResourceIdentityError::MixedTargetFields);
        }

        let resource = scope
            .resource
            .as_ref()
            .ok_or(ResourceIdentityError::Missing { field: "resource" })?;
        Self::try_from_wire(resource)
    }

    #[must_use]
    pub fn to_scope(&self) -> TargetScope {
        TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource: Some(WireResourceIdentity {
                adapter_id: Some(self.adapter_id.clone()),
                resource_id: Some(self.resource_id.clone()),
                resource_kind: Some(self.resource_kind.clone()),
            }),
            ..TargetScope::default()
        }
    }

    #[must_use]
    pub fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }

    #[must_use]
    pub fn resource_kind(&self) -> &ResourceKind {
        &self.resource_kind
    }

    #[must_use]
    pub fn resource_id(&self) -> &ResourceId {
        &self.resource_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResourceIdentityError {
    #[error("target scope is not a resource")]
    WrongTargetKind,
    #[error("resource identity is missing {field}")]
    Missing { field: &'static str },
    #[error("resource identity contains non-resource target fields")]
    MixedTargetFields,
    #[error("legacy audit resource id is not an operational resource identity")]
    LegacyAuditOnly,
}
