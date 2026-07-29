//! Constructors for the schema-owned security posture event family.

use patchbay_contracts::patchbay::{
    security_lockdown_event, AuthorityDomainId, SecurityLockdownEntered,
    SecurityLockdownExited, StoredEventKind, StoredEventPayload,
};
use prost::Message;

pub use patchbay_contracts::patchbay::SecurityLockdownEvent;

#[must_use]
pub fn entered(
    authority_domain_id: AuthorityDomainId,
    mutation: SecurityLockdownEntered,
) -> SecurityLockdownEvent {
    event(
        authority_domain_id,
        security_lockdown_event::Transition::Entered(mutation),
    )
}

#[must_use]
pub fn exited(
    authority_domain_id: AuthorityDomainId,
    mutation: SecurityLockdownExited,
) -> SecurityLockdownEvent {
    event(
        authority_domain_id,
        security_lockdown_event::Transition::Exited(mutation),
    )
}

#[must_use]
pub fn encode(event: &SecurityLockdownEvent) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::SecurityLockdown as i32,
        payload: event.encode_to_vec(),
    }
}

fn event(
    authority_domain_id: AuthorityDomainId,
    transition: security_lockdown_event::Transition,
) -> SecurityLockdownEvent {
    SecurityLockdownEvent {
        authority_domain_id: Some(authority_domain_id),
        transition: Some(transition),
    }
}
