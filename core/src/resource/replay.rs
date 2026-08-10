use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};

use crate::storage::{RecordedEvent, Storage};

use super::{ResourceError, ResourceRegistry};

/// Rebuild the operational-resource projection from one authority-domain log.
///
/// The shared checkpoint namespace remains session-only, so resource recovery
/// always folds from LSN zero.
pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<ResourceRegistry, ResourceError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    rebuild_from_events(authority_domain_id, &events)
}

pub fn rebuild_from_events(
    authority_domain_id: &AuthorityDomainId,
    events: &[RecordedEvent],
) -> Result<ResourceRegistry, ResourceError> {
    if authority_domain_id.value.is_empty() {
        return Err(ResourceError::CorruptRecord(
            "resource replay requires a non-empty authority domain".into(),
        ));
    }

    let mut registry = ResourceRegistry::new();
    let mut previous_lsn = 0u64;
    for event in events {
        let domain = event
            .event_id
            .authority_domain_id
            .as_ref()
            .ok_or_else(|| ResourceError::CorruptRecord("resource replay event has no authority domain".into()))?;
        if domain != authority_domain_id {
            return Err(ResourceError::CorruptLog(format!(
                "resource replay event belongs to authority domain {:?}, expected {:?}",
                domain, authority_domain_id
            )));
        }
        let lsn = event
            .event_id
            .lsn
            .as_ref()
            .ok_or_else(|| ResourceError::CorruptRecord("resource replay event has no LSN".into()))?
            .value;
        let expected_lsn = previous_lsn.checked_add(1).ok_or_else(|| {
            ResourceError::CorruptLog("resource replay LSN overflow".into())
        })?;
        if lsn != expected_lsn {
            return Err(ResourceError::CorruptLog(format!(
                "resource replay event LSN {lsn} is not the next LSN {expected_lsn}"
            )));
        }
        registry.observe(event)?;
        previous_lsn = lsn;
    }
    Ok(registry)
}

/// Bring an existing resource projection through the current durable tail.
pub(crate) async fn catch_up_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    registry: &mut ResourceRegistry,
) -> Result<(), ResourceError> {
    registry.require_authority_domain(authority_domain_id)?;
    let events = storage
        .read_after(
            authority_domain_id,
            Lsn {
                value: registry.applied_lsn(),
            },
        )
        .await?;
    let mut next = registry.clone();
    for event in &events {
        next.observe(event)?;
    }
    *registry = next;
    Ok(())
}
