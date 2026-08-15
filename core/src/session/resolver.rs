//! Acceptance target resolution backed by the session projection.

use patchbay_contracts::patchbay::{
    AuthorityDomainId, Operation, SpawnRequest, TargetScope, TargetScopeKind,
};

use crate::acceptance::{TargetBinding, TargetNotFound, TargetResolver};

use super::SessionRegistry;

/// Adapt the session registry to acceptance's target-resolution port.
///
/// Resolution validates identity existence and generation freshness only.
/// Connectivity is deliberately a delivery concern: an offline or failed
/// session remains a valid target for an Operation that may be queued or
/// retried when delivery becomes possible.
impl TargetResolver for SessionRegistry {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        operation: &Operation,
        _spawn_request: Option<&SpawnRequest>,
    ) -> Result<TargetBinding, TargetNotFound> {
        let target_scope =
            operation
                .target_scope
                .as_ref()
                .ok_or_else(|| TargetNotFound::NotFound {
                    target: "operation is missing target_scope".to_owned(),
                })?;
        self.require_authority_domain(authority_domain_id)
            .map_err(|_| not_found(target_scope, "session registry authority domain mismatch"))?;
        if TargetScopeKind::try_from(target_scope.kind).ok()
            != Some(TargetScopeKind::RuntimeSession)
        {
            return Err(not_found(target_scope, "target is not a runtime session"));
        }
        let adapter_id = target_scope
            .adapter_id
            .as_ref()
            .ok_or_else(|| not_found(target_scope, "target is missing adapter_id"))?;
        let runtime_session_id = target_scope
            .runtime_session_id
            .as_ref()
            .ok_or_else(|| not_found(target_scope, "target is missing runtime_session_id"))?;

        if let Some(requested_generation) = target_scope.session_generation.as_ref() {
            if self.is_tombstoned(
                adapter_id,
                &target_scope.deployment_scope,
                runtime_session_id,
                requested_generation,
            ) {
                return Err(not_found(
                    target_scope,
                    "requested session generation is tombstoned",
                ));
            }
        }

        let record = self
            .get_live_session(
                adapter_id,
                &target_scope.deployment_scope,
                runtime_session_id,
            )
            .ok_or_else(|| not_found(target_scope, "session is not live in the registry"))?;

        if target_scope
            .session_generation
            .as_ref()
            .is_some_and(|requested| requested != &record.identity.session_generation)
        {
            return Err(not_found(
                target_scope,
                "requested session generation is not the live generation",
            ));
        }

        Ok(TargetBinding::RuntimeSession {
            adapter_id: record.identity.adapter_id.clone(),
            deployment_scope: record.identity.deployment_scope.clone(),
            runtime_session_id: record.identity.runtime_session_id.clone(),
            session_generation: record.identity.session_generation,
        })
    }
}

fn not_found(target_scope: &TargetScope, reason: &str) -> TargetNotFound {
    TargetNotFound::NotFound {
        target: format!("{reason}: {target_scope:?}"),
    }
}
