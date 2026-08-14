//! Recovery of security posture from the authority-domain event log.

use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};

use crate::storage::{validate_next_replay_event, RecordedEvent, Storage};

use super::{SecurityError, SecurityPostureProjection};

pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<SecurityPostureProjection, SecurityError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut projection = SecurityPostureProjection::new();
    let mut previous_lsn = 0;
    for event in events {
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, &event)
            .map_err(|error| error.map(SecurityError::CorruptRecord, SecurityError::CorruptLog))?;
        projection.observe(&event)?;
        previous_lsn = validated.lsn;
    }
    Ok(projection)
}

#[allow(dead_code)]
fn _recorded_event_type_is_explicit(_: &RecordedEvent) {}
