use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, DeviceId, EndpointId, Generation, OperatorSessionId,
};
use patchbay_core::authority::IssuerContext;
use tonic::{Request, Status};

pub const OPERATOR_SESSION_HEADER: &str = "x-patchbay-operator-session-id";
pub const OPERATOR_ID_HEADER: &str = "x-patchbay-operator-id";
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
        let operator_session_id = OperatorSessionId {
            value: required_metadata(
                request,
                OPERATOR_SESSION_HEADER,
                "verified operator session",
            )?,
        };
        let verified_actor = ActorId {
            value: required_metadata(request, OPERATOR_ID_HEADER, "verified operator actor")?,
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

fn required_metadata<T>(
    request: &Request<T>,
    header: &'static str,
    description: &str,
) -> Result<String, Status> {
    let value = request
        .metadata()
        .get(header)
        .ok_or_else(|| Status::unauthenticated(format!("missing {description}")))?
        .to_str()
        .map_err(|_| Status::unauthenticated(format!("invalid {description}")))?;
    if value.is_empty() {
        return Err(Status::unauthenticated(format!(
            "{description} must not be empty"
        )));
    }
    Ok(value.to_owned())
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
