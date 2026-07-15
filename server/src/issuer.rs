use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, DeviceId, EndpointId, Generation, OperatorSessionId,
};
use patchbay_core::authority::IssuerContext;
use tonic::{Request, Status};

pub const OPERATOR_SESSION_HEADER: &str = "x-patchbay-operator-session-id";
const WEB_SERVER_ENDPOINT_ID: &str = "patchbay-web-server";

/// Compound-issuer evidence extracted after the transport interceptor has
/// authenticated the web-server principal.
#[derive(Debug, Clone)]
pub struct MetadataIssuerContext {
    operator_session_id: OperatorSessionId,
    verified_actor: ActorId,
    verified_endpoint: EndpointId,
    authority_domain_id: AuthorityDomainId,
}

impl MetadataIssuerContext {
    pub fn from_request<T>(
        request: &Request<T>,
        authority_domain_id: AuthorityDomainId,
    ) -> Result<Self, Status> {
        let value = request
            .metadata()
            .get(OPERATOR_SESSION_HEADER)
            .ok_or_else(|| Status::unauthenticated("missing verified operator session"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid verified operator session"))?;
        if value.is_empty() {
            return Err(Status::unauthenticated(
                "verified operator session must not be empty",
            ));
        }

        let operator_session_id = OperatorSessionId {
            value: value.to_owned(),
        };
        // v0.1.0 has one operator and the authenticated web ingress vouches for
        // this server-side session id. The actor projection is intentionally
        // deterministic so grants can use the same stable boundary identity.
        let verified_actor = ActorId {
            value: operator_session_id.value.clone(),
        };

        Ok(Self {
            operator_session_id,
            verified_actor,
            verified_endpoint: EndpointId {
                value: WEB_SERVER_ENDPOINT_ID.to_owned(),
            },
            authority_domain_id,
        })
    }

    #[must_use]
    pub fn operator_session_id(&self) -> &OperatorSessionId {
        &self.operator_session_id
    }
}

impl IssuerContext for MetadataIssuerContext {
    fn verified_actor(&self) -> Option<&ActorId> {
        Some(&self.verified_actor)
    }

    fn verified_endpoint(&self) -> Option<&EndpointId> {
        Some(&self.verified_endpoint)
    }

    fn verified_device(&self) -> Option<&DeviceId> {
        None
    }

    fn endpoint_generation(&self) -> Option<Generation> {
        None
    }

    fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.authority_domain_id
    }
}
