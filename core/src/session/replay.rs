//! Recovery of the in-memory session projection from durable log events.

use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};

use crate::storage::{validate_next_replay_event, Storage};

use super::{SessionError, SessionRegistry};

/// Rebuild a session registry by replaying one authority-domain log.
///
/// v0.1.0 replays from LSN 0 because the shared snapshot slot has no
/// projection discriminator. This matches command-index and Elicitation-slot
/// recovery and prevents one projection's snapshot from hiding another
/// projection's earlier events.
pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<SessionRegistry, SessionError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut registry = SessionRegistry::new();
    let mut previous_lsn = 0u64;

    for event in events {
        previous_lsn = validate_next_replay_event(&event, authority_domain_id, previous_lsn)
            .map_err(|error| SessionError::CorruptLog(error.to_string()))?;
        registry.observe(&event)?;
    }

    Ok(registry)
}
