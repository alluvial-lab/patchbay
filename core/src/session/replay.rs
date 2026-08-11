//! Recovery of the in-memory session projection from durable log events.

use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};

use crate::storage::{validate_next_replay_event, Storage};

use super::{SessionError, SessionRegistry};

/// Rebuild a session registry by replaying one authority-domain log.
///
/// This is the strict full-replay oracle and fallback for disposable or
/// inconsistent session checkpoints. Production server consumers select the
/// private typed format-2 checkpoint through `recover_session_registry`, while
/// sibling projections independently replay their owned state from LSN 0.
pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<SessionRegistry, SessionError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut registry = SessionRegistry::new(authority_domain_id.clone())?;
    let mut previous_lsn = 0u64;

    for event in events {
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, &event)
            .map_err(|error| {
                error.map(SessionError::CorruptRecord, SessionError::CorruptLog)
            })?;
        registry.observe(&event)?;
        previous_lsn = validated.lsn;
    }

    Ok(registry)
}
