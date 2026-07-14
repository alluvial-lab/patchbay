//! Durable authority-event encoding helpers.
//!
//! The generated Protobuf messages own the wire shape. These helpers attach
//! the authoritative domain and wrap each message in its schema-owned durable
//! event discriminator.

use patchbay_contracts::patchbay::{
    AuthorityDomainId, DescendantGrant, Grant, Revocation, StoredEventKind, StoredEventPayload,
};
use prost::Message;

/// Encode an operator-issued grant for durable append.
#[must_use]
pub fn grant(authority_domain_id: AuthorityDomainId, mut grant: Grant) -> StoredEventPayload {
    grant.authority_domain_id = Some(authority_domain_id);
    StoredEventPayload {
        kind: StoredEventKind::Grant as i32,
        payload: grant.encode_to_vec(),
    }
}

/// Encode an auto-issued descendant grant for durable append.
#[must_use]
pub fn descendant_grant(
    authority_domain_id: AuthorityDomainId,
    mut grant: DescendantGrant,
) -> StoredEventPayload {
    grant.authority_domain_id = Some(authority_domain_id);
    StoredEventPayload {
        kind: StoredEventKind::DescendantGrant as i32,
        payload: grant.encode_to_vec(),
    }
}

/// Encode a grant revocation for durable append.
#[must_use]
pub fn revocation(
    authority_domain_id: AuthorityDomainId,
    mut revocation: Revocation,
) -> StoredEventPayload {
    revocation.authority_domain_id = Some(authority_domain_id);
    StoredEventPayload {
        kind: StoredEventKind::Revocation as i32,
        payload: revocation.encode_to_vec(),
    }
}
