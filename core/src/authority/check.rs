//! Acceptance grant checks backed by the durable authority projection.

use patchbay_contracts::patchbay::{AuthorityDomainId, OperationKind, TargetScope};

use crate::acceptance::{Authorized, GrantCheck, GrantDenied};

use super::{grant_authorizes, grant_matches_request, AuthorityRegistry, IssuerContext, IssuerRef};

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
        if let Some(grant) = self
            .grants()
            .find(|grant| grant.is_expired() && grant_matches_request(grant, &issuer_ref, operation_kind, target_scope))
        {
            return Err(no_grant(&format!("expired grant {:?}", grant.grant_id), operation_kind, target_scope));
        }
        self.live_grants()
            .find(|grant| grant_authorizes(grant, &issuer_ref, operation_kind, target_scope))
            .map(|grant| Authorized {
                grant_id: Some(grant.grant_id.clone()),
            })
            .ok_or_else(|| no_grant(&format!("{actor:?}"), operation_kind, target_scope))
    }
}

fn no_grant(actor: &str, kind: OperationKind, target: &TargetScope) -> GrantDenied {
    GrantDenied::NoGrant {
        actor: actor.to_owned(),
        kind,
        target: format!("{target:?}"),
    }
}
