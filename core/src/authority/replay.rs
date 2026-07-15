//! Recovery of the in-memory authority projection from durable log events.

use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};

use crate::storage::{RecordedEvent, Storage};

use super::{AuthorityError, AuthorityRegistry};

/// Rebuild an authority registry by replaying one authority-domain log.
///
/// v0.1.0 replays from LSN 0 because the shared snapshot slot has no
/// projection discriminator. Events owned by sibling projections are folded
/// through [`AuthorityRegistry::observe`] and ignored there.
pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<AuthorityRegistry, AuthorityError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut registry = AuthorityRegistry::new();
    let mut previous_lsn = 0u64;

    for event in events {
        let (event_domain, event_lsn) = event_identity(&event)?;
        if event_domain != authority_domain_id {
            return Err(AuthorityError::CorruptLog(format!(
                "recovery event belongs to authority domain {:?}, expected {:?}",
                event_domain, authority_domain_id
            )));
        }
        if event_lsn <= previous_lsn {
            return Err(AuthorityError::CorruptLog(format!(
                "recovery event LSN {event_lsn} is not after previous LSN {previous_lsn}"
            )));
        }

        registry.observe(&event)?;
        previous_lsn = event_lsn;
    }

    Ok(registry)
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), AuthorityError> {
    let authority_domain_id = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        AuthorityError::CorruptRecord("recovery event has no authority domain".to_owned())
    })?;
    if authority_domain_id.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(
            "recovery event has an empty authority domain".to_owned(),
        ));
    }
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| AuthorityError::CorruptRecord("recovery event has no LSN".to_owned()))?;
    Ok((authority_domain_id, lsn.value))
}
