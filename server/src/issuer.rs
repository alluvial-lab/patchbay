use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, DeviceId, EndpointId, Generation, OperatorSessionId,
};
use patchbay_core::authority::IssuerContext;
use tonic::{Request, Status};

use crate::state::ProjectionState;

pub const OPERATOR_SESSION_HEADER: &str = "x-patchbay-operator-session-id";
pub const OPERATOR_ID_HEADER: &str = "x-patchbay-operator-id";
pub const PRINCIPAL_ID_HEADER: &str = "x-patchbay-principal-id";
pub const PRINCIPAL_SECRET_HEADER: &str = "x-patchbay-principal-secret";

/// Compound issuer derived from a durable credential-backed transport
/// principal plus the actor/session that principal vouches for. Endpoint,
/// device, generation, and actor are taken from the verified principal record,
/// never from caller-supplied identity strings.
#[derive(Debug, Clone)]
pub struct MetadataIssuerContext {
    operator_session_id: OperatorSessionId,
    verified_actor: ActorId,
    verified_endpoint: EndpointId,
    verified_device: DeviceId,
    endpoint_generation: Generation,
    authority_domain_id: AuthorityDomainId,
}

impl MetadataIssuerContext {
    pub async fn from_request<T>(
        request: &Request<T>,
        authority_domain_id: AuthorityDomainId,
        state: &ProjectionState,
    ) -> Result<Self, Status> {
        let principal_id = required_metadata(request, PRINCIPAL_ID_HEADER, "transport principal")?;
        let principal_secret = required_metadata(
            request,
            PRINCIPAL_SECRET_HEADER,
            "transport principal credential",
        )?;
        let operator_session_id = OperatorSessionId {
            value: required_metadata(
                request,
                OPERATOR_SESSION_HEADER,
                "verified operator session",
            )?,
        };
        let claimed_actor = ActorId {
            value: required_metadata(request, OPERATOR_ID_HEADER, "verified operator actor")?,
        };

        let principal = state
            .verify_principal(&principal_id, &principal_secret)
            .await
            .ok_or_else(|| Status::unauthenticated("invalid transport principal credential"))?;
        if principal.authority_domain_id.as_ref() != Some(&authority_domain_id) {
            return Err(Status::unauthenticated(
                "transport principal belongs to another authority domain",
            ));
        }
        let verified_actor = principal.operator_actor_id.ok_or_else(|| {
            Status::internal("verified transport principal has no operator actor")
        })?;
        if verified_actor != claimed_actor {
            return Err(Status::unauthenticated(
                "operator actor is not bound to the verified transport principal",
            ));
        }
        let verified_endpoint = principal
            .endpoint_id
            .ok_or_else(|| Status::internal("verified transport principal has no endpoint"))?;
        let verified_device = principal
            .device_id
            .ok_or_else(|| Status::internal("verified transport principal has no device"))?;
        let endpoint_generation = principal.endpoint_generation.ok_or_else(|| {
            Status::internal("verified transport principal has no endpoint generation")
        })?;

        Ok(Self {
            operator_session_id,
            verified_actor,
            verified_endpoint,
            verified_device,
            endpoint_generation,
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
        Some(&self.verified_device)
    }

    fn endpoint_generation(&self) -> Option<Generation> {
        Some(self.endpoint_generation)
    }

    fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.authority_domain_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{ControlSurfacePrincipalRecord, OperatorRecord};
    use patchbay_core::{authority::hash_principal_credential, storage::RusqliteStorage};
    use proptest::prelude::*;
    use prost_types::Timestamp;

    #[tokio::test]
    async fn verified_web_and_cli_principals_preserve_distinct_identity() {
        let authority_domain_id = AuthorityDomainId {
            value: "authority-main".to_owned(),
        };
        let actor_id = ActorId {
            value: "operator-primary".to_owned(),
        };
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let state = ProjectionState::rebuild(&storage, &authority_domain_id)
            .await
            .unwrap();
        state
            .ingest_operator(
                &storage,
                &authority_domain_id,
                OperatorRecord {
                    actor_id: Some(actor_id.clone()),
                    password_hash: "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g".to_owned(),
                    created_at: Some(Timestamp { seconds: 1, nanos: 0 }),
                    authority_domain_id: Some(authority_domain_id.clone()),
                },
            )
            .await
            .unwrap();
        for (principal_id, secret, endpoint, device, generation) in [
            ("web-principal", "web-secret", "web", "web-host", 3),
            ("cli-principal", "cli-secret", "cli", "cli-host", 7),
        ] {
            state
                .ingest_principal(
                    &storage,
                    &authority_domain_id,
                    ControlSurfacePrincipalRecord {
                        principal_id: principal_id.to_owned(),
                        operator_actor_id: Some(actor_id.clone()),
                        endpoint_id: Some(EndpointId {
                            value: endpoint.to_owned(),
                        }),
                        device_id: Some(DeviceId {
                            value: device.to_owned(),
                        }),
                        endpoint_generation: Some(Generation { value: generation }),
                        credential_hash: hash_principal_credential(secret),
                        created_at: Some(Timestamp {
                            seconds: 2,
                            nanos: 0,
                        }),
                        authority_domain_id: Some(authority_domain_id.clone()),
                    },
                )
                .await
                .unwrap();
        }

        let web = verified_request("web-principal", "web-secret");
        let web = MetadataIssuerContext::from_request(&web, authority_domain_id.clone(), &state)
            .await
            .unwrap();
        let cli = verified_request("cli-principal", "cli-secret");
        let cli = MetadataIssuerContext::from_request(&cli, authority_domain_id, &state)
            .await
            .unwrap();

        assert_eq!(web.verified_actor(), Some(&actor_id));
        assert_eq!(web.verified_endpoint().unwrap().value, "web");
        assert_eq!(web.verified_device().unwrap().value, "web-host");
        assert_eq!(web.endpoint_generation(), Some(Generation { value: 3 }));
        assert_eq!(cli.verified_actor(), Some(&actor_id));
        assert_eq!(cli.verified_endpoint().unwrap().value, "cli");
        assert_eq!(cli.verified_device().unwrap().value, "cli-host");
        assert_eq!(cli.endpoint_generation(), Some(Generation { value: 7 }));
    }

    fn verified_request(principal_id: &str, principal_secret: &str) -> Request<()> {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert(PRINCIPAL_ID_HEADER, principal_id.parse().unwrap());
        request
            .metadata_mut()
            .insert(PRINCIPAL_SECRET_HEADER, principal_secret.parse().unwrap());
        request
            .metadata_mut()
            .insert(OPERATOR_ID_HEADER, "operator-primary".parse().unwrap());
        request
            .metadata_mut()
            .insert(OPERATOR_SESSION_HEADER, "operator-session".parse().unwrap());
        request
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn unverified_identity_is_rejected_for_all_non_empty_claims(
            actor in "[a-z][a-z0-9-]{0,31}",
            session in "[a-z][a-z0-9-]{0,31}",
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async move {
                let authority_domain_id = AuthorityDomainId {
                    value: "authority-main".to_owned(),
                };
                let storage = RusqliteStorage::open_in_memory().unwrap();
                let state = ProjectionState::rebuild(&storage, &authority_domain_id)
                    .await
                    .unwrap();
                let mut request = Request::new(());
                request.metadata_mut().insert(
                    OPERATOR_ID_HEADER,
                    actor.parse().unwrap(),
                );
                request.metadata_mut().insert(
                    OPERATOR_SESSION_HEADER,
                    session.parse().unwrap(),
                );

                let result = MetadataIssuerContext::from_request(
                    &request,
                    authority_domain_id,
                    &state,
                )
                .await;
                prop_assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
                Ok(())
            })?;
        }
    }
}
