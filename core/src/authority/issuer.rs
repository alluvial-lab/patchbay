//! Verified issuer identity supplied by an authenticated ingress boundary.

use patchbay_contracts::patchbay::{ActorId, AuthorityDomainId, DeviceId, EndpointId, Generation};

/// Verified issuer identity supplied by the authenticated ingress boundary.
///
/// This identity is not self-asserted. The operator actor and transport
/// endpoint come from verified connection/session evidence (see
/// `docs/SECURITY.md` "Compound issuer"). v0.1.0 tests supply a test context;
/// the real operator-session and transport-principal verifier lands with the
/// protocol seam and web server.
pub trait IssuerContext: Send + Sync {
    fn verified_actor(&self) -> Option<&ActorId>;
    fn verified_endpoint(&self) -> Option<&EndpointId>;
    fn verified_device(&self) -> Option<&DeviceId>;
    fn endpoint_generation(&self) -> Option<Generation>;
    fn authority_domain_id(&self) -> &AuthorityDomainId;
}
