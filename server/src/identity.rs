use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, ControlSurfacePrincipalRecord, OperatorSessionId,
    PrincipalCredential, PrincipalEnrollment,
};
use patchbay_core::authority::hash_principal_credential;
use prost_types::Timestamp;
use rand::{rngs::OsRng, RngCore};
use tonic::Status;

const TOKEN_BYTES: usize = 32;

pub fn issue_principal(
    operator_actor_id: ActorId,
    enrollment: PrincipalEnrollment,
    authority_domain_id: AuthorityDomainId,
) -> Result<(ControlSurfacePrincipalRecord, PrincipalCredential), Status> {
    let endpoint_id = enrollment
        .endpoint_id
        .filter(|value| !value.value.is_empty())
        .ok_or_else(|| Status::invalid_argument("principal endpoint id must not be empty"))?;
    let device_id = enrollment
        .device_id
        .filter(|value| !value.value.is_empty())
        .ok_or_else(|| Status::invalid_argument("principal device id must not be empty"))?;
    let endpoint_generation = enrollment
        .endpoint_generation
        .filter(|value| value.value > 0)
        .ok_or_else(|| {
            Status::invalid_argument("principal endpoint generation must be positive")
        })?;
    let principal_id = format!("principal-{}", random_token());
    let secret = random_token();
    let record = ControlSurfacePrincipalRecord {
        principal_id: principal_id.clone(),
        operator_actor_id: Some(operator_actor_id.clone()),
        endpoint_id: Some(endpoint_id.clone()),
        device_id: Some(device_id.clone()),
        endpoint_generation: Some(endpoint_generation),
        credential_hash: hash_principal_credential(&secret),
        created_at: Some(now_timestamp()?),
        authority_domain_id: Some(authority_domain_id),
    };
    let credential = PrincipalCredential {
        principal_id,
        secret,
        operator_actor_id: Some(operator_actor_id),
        endpoint_id: Some(endpoint_id),
        device_id: Some(device_id),
        endpoint_generation: Some(endpoint_generation),
    };
    Ok((record, credential))
}

#[must_use]
pub fn issue_operator_session_id() -> OperatorSessionId {
    OperatorSessionId {
        value: format!("operator-session-{}", random_token()),
    }
}

pub fn now_timestamp() -> Result<Timestamp, Status> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Status::internal("system clock is before the Unix epoch"))?;
    Ok(Timestamp {
        seconds: duration.as_secs().try_into().map_err(|_| {
            Status::internal("system clock cannot be represented as a protobuf timestamp")
        })?,
        nanos: duration.subsec_nanos() as i32,
    })
}

#[must_use]
pub fn random_token() -> String {
    let mut bytes = [0_u8; TOKEN_BYTES];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
