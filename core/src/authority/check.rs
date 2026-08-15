//! Acceptance grant checks backed by the durable authority projection.

use patchbay_contracts::patchbay::{
    AuthorityDomainId, ContinuationAuthorityProvenance, OperationKind, TargetScope, TargetScopeKind,
};
use prost_types::Timestamp;

use crate::{
    acceptance::{Authorized, GrantCheck, GrantDenied, ResolvedGrantCheck, TargetBinding},
    time::{Clock, SystemClock},
};

use super::{
    grant_authorizes_at, grant_matches_request, AuthorityRegistry, GrantLiveness, IssuerContext,
    IssuerRef,
};

/// Adapt the authority registry to acceptance's grant-check port.
///
/// The registry evaluates only verified issuer identity supplied by ingress;
/// the operation's self-asserted sender remains audit data and is never used
/// as authority. Every path is deny-by-default.
impl GrantCheck for AuthorityRegistry {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        self.check_at(
            authority_domain_id,
            issuer,
            operation_kind,
            target_scope,
            &SystemClock.now(),
        )
        .await
    }

    async fn check_at(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
        evaluated_at: &Timestamp,
    ) -> Result<Authorized, GrantDenied> {
        select_authorization(
            self,
            authority_domain_id,
            issuer,
            operation_kind,
            target_scope,
            evaluated_at,
            false,
        )
    }

    async fn check_resolved_at(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        request: ResolvedGrantCheck<'_>,
    ) -> Result<Authorized, GrantDenied> {
        let ResolvedGrantCheck {
            operation_kind,
            target_scope,
            target_binding,
            authorization,
            evaluated_at,
        } = request;
        let TargetBinding::SpawnAdapter {
            claim,
            continuation_authority,
            ..
        } = target_binding
        else {
            return Ok(authorization);
        };
        if operation_kind != OperationKind::Spawn || continuation_authority.is_some() {
            return Err(no_grant(
                "invalid_spawn_decision",
                operation_kind,
                target_scope,
            ));
        }

        // Recompute the normal spawn selection at the same sampled instant.
        // This makes the adapter-spawn half independently load-bearing instead
        // of trusting an unverified Grant id handed in by another caller.
        let selected_spawn = select_authorization(
            self,
            authority_domain_id,
            issuer,
            OperationKind::Spawn,
            target_scope,
            evaluated_at,
            false,
        )?;
        if selected_spawn.grant_id != authorization.grant_id {
            return Err(no_grant(
                "spawn_grant_selection_changed",
                operation_kind,
                target_scope,
            ));
        }

        let Some(prior) = claim.expected_prior.as_ref() else {
            return Ok(selected_spawn);
        };
        let external = prior.external_runtime.as_ref().ok_or_else(|| {
            no_grant(
                "malformed_exact_prior",
                OperationKind::SessionManagement,
                target_scope,
            )
        })?;
        let exact_prior_scope = TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: external.adapter_id.clone(),
            runtime_session_id: external.runtime_session_id.clone(),
            session_generation: external.generation,
            deployment_scope: external.deployment_scope.clone(),
            ..TargetScope::default()
        };
        let replacement = select_authorization(
            self,
            authority_domain_id,
            issuer,
            OperationKind::SessionManagement,
            &exact_prior_scope,
            evaluated_at,
            true,
        )?;
        let spawning_grant_id = selected_spawn
            .grant_id
            .as_ref()
            .ok_or_else(|| no_grant("missing_spawn_grant", OperationKind::Spawn, target_scope))?;
        let replacement_grant_id = replacement.grant_id.ok_or_else(|| {
            no_grant(
                "missing_replacement_grant",
                OperationKind::SessionManagement,
                &exact_prior_scope,
            )
        })?;
        if &replacement_grant_id == spawning_grant_id {
            return Err(no_grant(
                "continuation_requires_two_distinct_grants",
                OperationKind::SessionManagement,
                &exact_prior_scope,
            ));
        }

        Ok(Authorized {
            grant_id: Some(spawning_grant_id.clone()),
            continuation_authority: Some(ContinuationAuthorityProvenance {
                exact_prior: Some(prior.clone()),
                replacement_grant_id: Some(replacement_grant_id),
                replacement_authority_kind: OperationKind::SessionManagement as i32,
            }),
        })
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the authority decision keeps verified issuer, target, time, and exact-scope policy explicit"
)]
fn select_authorization(
    registry: &AuthorityRegistry,
    authority_domain_id: &AuthorityDomainId,
    issuer: &dyn IssuerContext,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
    evaluated_at: &Timestamp,
    require_exact_scope: bool,
) -> Result<Authorized, GrantDenied> {
    let Some(actor) = issuer.verified_actor() else {
        return Err(no_grant("unauthenticated", operation_kind, target_scope));
    };
    if issuer.authority_domain_id() != authority_domain_id {
        return Err(no_grant(
            &format!("{actor:?}"),
            operation_kind,
            target_scope,
        ));
    }

    let issuer_ref = IssuerRef {
        actor,
        endpoint: issuer.verified_endpoint(),
        authority_domain_id,
    };
    let mut candidates: Vec<_> = registry
        .grants()
        .filter(|grant| {
            grant_matches_request(grant, &issuer_ref, operation_kind, target_scope)
                && (!require_exact_scope || grant.target_scope == *target_scope)
        })
        .collect();
    // Canonical decision provenance: exact UTF-8 grant-id bytes order candidates
    // within each class. `liveness_at` classifies revocation before expiry; the
    // searches below select the resulting classes as live > expired > revoked.
    candidates.sort_unstable_by(|left, right| left.grant_id.value.cmp(&right.grant_id.value));

    if let Some(grant) = candidates.iter().copied().find(|grant| {
        grant_authorizes_at(
            grant,
            &issuer_ref,
            operation_kind,
            target_scope,
            evaluated_at,
        )
    }) {
        return Ok(Authorized {
            grant_id: Some(grant.grant_id.clone()),
            continuation_authority: None,
        });
    }
    if let Some(grant) = candidates
        .iter()
        .copied()
        .find(|grant| grant.liveness_at(evaluated_at) == GrantLiveness::Expired)
    {
        return Err(no_grant(
            &format!("grant_expired:{}", grant.grant_id.value),
            operation_kind,
            target_scope,
        ));
    }
    if let Some(grant) = candidates
        .iter()
        .copied()
        .find(|grant| grant.liveness_at(evaluated_at) == GrantLiveness::Revoked)
    {
        return Err(no_grant(
            &format!("grant_revoked:{}", grant.grant_id.value),
            operation_kind,
            target_scope,
        ));
    }
    Err(no_grant(
        &format!("{actor:?}"),
        operation_kind,
        target_scope,
    ))
}

fn no_grant(actor: &str, kind: OperationKind, target: &TargetScope) -> GrantDenied {
    GrantDenied::NoGrant {
        actor: actor.to_owned(),
        kind,
        target: format!("{target:?}"),
    }
}
