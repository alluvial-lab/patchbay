//! Acceptance grant checks backed by the durable authority projection.

use patchbay_contracts::patchbay::{AuthorityDomainId, OperationKind, TargetScope};
use prost_types::Timestamp;

use crate::{
    acceptance::{Authorized, GrantCheck, GrantDenied},
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
        let mut candidates: Vec<_> = self
            .grants()
            .filter(|grant| grant_matches_request(grant, &issuer_ref, operation_kind, target_scope))
            .collect();
        // Canonical decision provenance: exact UTF-8 grant-id bytes order candidates
        // within a liveness class; the searches below preserve live > expired > revoked.
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
}

fn no_grant(actor: &str, kind: OperationKind, target: &TargetScope) -> GrantDenied {
    GrantDenied::NoGrant {
        actor: actor.to_owned(),
        kind,
        target: format!("{target:?}"),
    }
}
