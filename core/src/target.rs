//! Composite ordinary-operation target registry and routing helpers.

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, OperationKind, TargetScope, TargetScopeKind,
};

use crate::{
    acceptance::{TargetBinding, TargetNotFound, TargetResolver},
    adapter::{AdapterError, AdapterRegistry},
    resource::{resolver::resolve_resource, ResourceError, ResourceIdentity, ResourceRegistry},
    session::{SessionError, SessionRegistry},
    storage::RecordedEvent,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TargetRegistry {
    sessions: SessionRegistry,
    resources: ResourceRegistry,
    adapters: AdapterRegistry,
}

impl TargetRegistry {
    #[must_use]
    pub fn new(sessions: SessionRegistry, resources: ResourceRegistry) -> Self {
        Self::with_adapters(sessions, resources, AdapterRegistry::new())
    }

    #[must_use]
    pub fn with_adapters(
        sessions: SessionRegistry,
        resources: ResourceRegistry,
        adapters: AdapterRegistry,
    ) -> Self {
        Self {
            sessions,
            resources,
            adapters,
        }
    }

    #[must_use]
    pub fn sessions(&self) -> &SessionRegistry {
        &self.sessions
    }

    pub fn sessions_mut(&mut self) -> &mut SessionRegistry {
        &mut self.sessions
    }

    #[must_use]
    pub fn resources(&self) -> &ResourceRegistry {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut ResourceRegistry {
        &mut self.resources
    }

    #[must_use]
    pub fn adapters(&self) -> &AdapterRegistry {
        &self.adapters
    }

    pub fn observe_event(&mut self, event: &RecordedEvent) -> Result<(), TargetRegistryError> {
        self.sessions.observe(event)?;
        self.resources.observe(event)?;
        self.adapters.observe(event)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TargetRegistryError {
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Resource(#[from] ResourceError),
    #[error(transparent)]
    Adapter(#[from] AdapterError),
}

impl TargetResolver for TargetRegistry {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<TargetBinding, TargetNotFound> {
        let target_kind =
            TargetScopeKind::try_from(target_scope.kind).map_err(|_| TargetNotFound::NotFound {
                target: format!("target has an unknown scope kind: {target_scope:?}"),
            })?;
        match (operation_kind, target_kind) {
            (OperationKind::Spawn, TargetScopeKind::Adapter) => {
                resolve_spawn_adapter(&self.adapters, authority_domain_id, target_scope)
            }
            (OperationKind::Spawn, _) => Err(TargetNotFound::NotFound {
                target: format!(
                    "v0.1.0 spawn requires one attached adapter target; fleet selection and existing runtime/resource targets are unavailable: {target_scope:?}"
                ),
            }),
            (_, TargetScopeKind::RuntimeSession) => {
                TargetResolver::resolve(
                    &self.sessions,
                    authority_domain_id,
                    operation_kind,
                    target_scope,
                )
                .await
            }
            (_, TargetScopeKind::Resource) => resolve_resource(&self.resources, target_scope),
            _ => Err(TargetNotFound::NotFound {
                target: format!("unsupported operation target: {target_scope:?}"),
            }),
        }
    }
}

fn resolve_spawn_adapter(
    adapters: &AdapterRegistry,
    authority_domain_id: &AuthorityDomainId,
    target_scope: &TargetScope,
) -> Result<TargetBinding, TargetNotFound> {
    let adapter_id = target_scope
        .adapter_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| TargetNotFound::NotFound {
            target: format!("spawn adapter target is missing adapter_id: {target_scope:?}"),
        })?;
    if target_scope.actor_id.is_some()
        || target_scope.runtime_session_id.is_some()
        || target_scope.session_generation.is_some()
        || !target_scope.deployment_scope.is_empty()
        || !target_scope.project_or_group.is_empty()
        || !target_scope.legacy_audit_resource_id.is_empty()
        || target_scope.resource.is_some()
    {
        return Err(TargetNotFound::NotFound {
            target: format!("spawn adapter target is not canonical: {target_scope:?}"),
        });
    }
    let registration = adapters
        .get(adapter_id)
        .filter(|record| {
            record.registration.authority_domain_id.as_ref() == Some(authority_domain_id)
        })
        .ok_or_else(|| TargetNotFound::NotFound {
            target: format!(
                "spawn adapter is not attached in this authority domain: {target_scope:?}"
            ),
        })?;
    debug_assert_eq!(
        registration.registration.adapter_id.as_ref(),
        Some(adapter_id)
    );
    Ok(TargetBinding::Adapter {
        adapter_id: adapter_id.clone(),
    })
}

/// Return the adapter addressed by a routable adapter/session/resource scope.
///
/// Resource routing is available only after the complete canonical tuple
/// parses, preventing a partial nested adapter id from becoming delivery or
/// authenticated-ingress authority.
#[must_use]
pub fn target_adapter_id(scope: &TargetScope) -> Option<&AdapterId> {
    match TargetScopeKind::try_from(scope.kind).ok()? {
        TargetScopeKind::Adapter => scope.adapter_id.as_ref(),
        TargetScopeKind::RuntimeSession => scope
            .runtime_session_id
            .as_ref()
            .and(scope.adapter_id.as_ref()),
        TargetScopeKind::Resource => {
            ResourceIdentity::try_from_scope(scope).ok()?;
            scope.resource.as_ref()?.adapter_id.as_ref()
        }
        _ => None,
    }
}
