use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};

use crate::storage::{validate_next_replay_event, RecordedEvent, Storage};

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
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, event)
            .map_err(|error| error.map(ResourceError::CorruptRecord, ResourceError::CorruptLog))?;
        registry.observe(event)?;
        previous_lsn = validated.lsn;
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
    *registry = fold_contiguous_suffix(registry, authority_domain_id, &events)?;
    Ok(())
}

/// Install the exact stored suffix ending at a just-committed event.
///
/// Sibling writers may allocate LSNs between pre-append catch-up and the
/// report append. Reading through the returned event identity makes those
/// records part of the same atomic projection install instead of treating the
/// report as a false gap. Missing, reordered, corrupt, or substituted suffixes
/// remain fail-closed.
pub(crate) async fn catch_up_through_event<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    registry: &mut ResourceRegistry,
    committed: &RecordedEvent,
) -> Result<(), ResourceError> {
    registry.require_authority_domain(authority_domain_id)?;
    let committed_lsn = suffix_event_lsn(committed, authority_domain_id)?;
    let applied_lsn = registry.applied_lsn();
    if committed_lsn <= applied_lsn {
        return Err(ResourceError::CorruptLog(format!(
            "committed resource event LSN {committed_lsn} does not follow applied LSN {applied_lsn}"
        )));
    }

    let events = storage
        .read_through(
            authority_domain_id,
            Lsn { value: applied_lsn },
            Lsn {
                value: committed_lsn,
            },
        )
        .await?;
    if events.last() != Some(committed) {
        return Err(ResourceError::CorruptLog(format!(
            "durable resource suffix does not end with the exact committed report at LSN {committed_lsn}"
        )));
    }

    *registry = fold_contiguous_suffix(registry, authority_domain_id, &events)?;
    Ok(())
}

fn fold_contiguous_suffix(
    registry: &ResourceRegistry,
    authority_domain_id: &AuthorityDomainId,
    events: &[RecordedEvent],
) -> Result<ResourceRegistry, ResourceError> {
    let mut next = registry.clone();
    let mut previous_lsn = registry.applied_lsn();
    for event in events {
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, event)
            .map_err(|error| error.map(ResourceError::CorruptRecord, ResourceError::CorruptLog))?;
        next.observe(event)?;
        previous_lsn = validated.lsn;
    }
    Ok(next)
}

fn suffix_event_lsn(
    event: &RecordedEvent,
    authority_domain_id: &AuthorityDomainId,
) -> Result<u64, ResourceError> {
    let domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        ResourceError::CorruptRecord("resource durable suffix event has no authority domain".into())
    })?;
    if domain != authority_domain_id {
        return Err(ResourceError::CorruptLog(format!(
            "resource durable suffix event belongs to authority domain {:?}, expected {:?}",
            domain, authority_domain_id
        )));
    }
    let lsn = event.event_id.lsn.as_ref().ok_or_else(|| {
        ResourceError::CorruptRecord("resource durable suffix event has no LSN".into())
    })?;
    if lsn.value == 0 {
        return Err(ResourceError::CorruptRecord(
            "resource durable suffix event has zero LSN".into(),
        ));
    }
    Ok(lsn.value)
}
