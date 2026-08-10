//! Durable session-state delta event helpers.
//!
//! The Protobuf schema owns the event and mutation variant set. This module
//! supplies the small construction and storage-envelope helpers used by the
//! session writer without duplicating the generated contract.

use patchbay_contracts::patchbay::{
    session_state_event, AuthorityDomainId, SessionActivityChanged, SessionConnectivityChanged,
    SessionGenerationBumped, SessionModelChanged, SessionRegistered, SessionRelabeled,
    SessionReportApplied, StoredEventKind, StoredEventPayload,
};
use prost::Message;

pub use patchbay_contracts::patchbay::SessionStateEvent;

/// Construct a session registration delta.
#[must_use]
pub fn registered(
    authority_domain_id: AuthorityDomainId,
    mutation: SessionRegistered,
) -> SessionStateEvent {
    event(
        authority_domain_id,
        session_state_event::Mutation::Registered(mutation),
    )
}

/// Construct a session-generation supersession delta.
#[must_use]
pub fn generation_bumped(
    authority_domain_id: AuthorityDomainId,
    mutation: SessionGenerationBumped,
) -> SessionStateEvent {
    event(
        authority_domain_id,
        session_state_event::Mutation::GenerationBumped(mutation),
    )
}

/// Construct one atomic equal-generation full-report mutation.
#[must_use]
pub fn report_applied(
    authority_domain_id: AuthorityDomainId,
    mutation: SessionReportApplied,
) -> SessionStateEvent {
    event(
        authority_domain_id,
        session_state_event::Mutation::ReportApplied(mutation),
    )
}

/// Construct a connectivity-axis delta.
#[must_use]
pub fn connectivity_changed(
    authority_domain_id: AuthorityDomainId,
    mutation: SessionConnectivityChanged,
) -> SessionStateEvent {
    event(
        authority_domain_id,
        session_state_event::Mutation::ConnectivityChanged(mutation),
    )
}

/// Construct an activity-axis delta.
#[must_use]
pub fn activity_changed(
    authority_domain_id: AuthorityDomainId,
    mutation: SessionActivityChanged,
) -> SessionStateEvent {
    event(
        authority_domain_id,
        session_state_event::Mutation::ActivityChanged(mutation),
    )
}

/// Construct a metadata-only relabel delta.
#[must_use]
pub fn relabeled(
    authority_domain_id: AuthorityDomainId,
    mutation: SessionRelabeled,
) -> SessionStateEvent {
    event(
        authority_domain_id,
        session_state_event::Mutation::Relabeled(mutation),
    )
}

/// Construct a current-model delta.
#[must_use]
pub fn model_changed(
    authority_domain_id: AuthorityDomainId,
    mutation: SessionModelChanged,
) -> SessionStateEvent {
    event(
        authority_domain_id,
        session_state_event::Mutation::ModelChanged(mutation),
    )
}

/// Encode a session delta in the schema-owned durable event envelope.
#[must_use]
pub fn encode(event: &SessionStateEvent) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::SessionState as i32,
        payload: event.encode_to_vec(),
    }
}

fn event(
    authority_domain_id: AuthorityDomainId,
    mutation: session_state_event::Mutation,
) -> SessionStateEvent {
    SessionStateEvent {
        authority_domain_id: Some(authority_domain_id),
        mutation: Some(mutation),
    }
}
