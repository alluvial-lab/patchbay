//! Composite ordinary-operation target registry and routing helpers.

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, TargetScope, TargetScopeKind,
};

use crate::{
    acceptance::{TargetBinding, TargetNotFound, TargetResolver},
    resource::{resolver::resolve_resource, ResourceError, ResourceIdentity, ResourceRegistry},
    session::{SessionError, SessionRegistry},
    storage::RecordedEvent,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TargetRegistry {
    sessions: SessionRegistry,
    resources: ResourceRegistry,
}

impl TargetRegistry {
    #[must_use]
    pub fn new(sessions: SessionRegistry, resources: ResourceRegistry) -> Self {
        Self {
            sessions,
            resources,
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

    pub fn observe_event(&mut self, event: &RecordedEvent) -> Result<(), TargetRegistryError> {
        self.sessions.observe(event)?;
        self.resources.observe(event)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TargetRegistryError {
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Resource(#[from] ResourceError),
}

impl TargetResolver for TargetRegistry {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> Result<TargetBinding, TargetNotFound> {
        match TargetScopeKind::try_from(target_scope.kind) {
            Ok(TargetScopeKind::RuntimeSession) => {
                TargetResolver::resolve(&self.sessions, authority_domain_id, target_scope).await
            }
            Ok(TargetScopeKind::Resource) => resolve_resource(&self.resources, target_scope),
            _ => Err(TargetNotFound::NotFound {
                target: format!("unsupported ordinary target: {target_scope:?}"),
            }),
        }
    }
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
