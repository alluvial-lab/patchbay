//! Recovery of security posture from the authority-domain event log.

use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};

use crate::storage::{RecordedEvent, Storage};

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
        let domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
            SecurityError::CorruptRecord("recovery event has no authority domain".to_owned())
        })?;
        let lsn = event
            .event_id
            .lsn
            .as_ref()
            .ok_or_else(|| SecurityError::CorruptRecord("recovery event has no LSN".to_owned()))?
            .value;
        if domain != authority_domain_id || lsn <= previous_lsn {
            return Err(SecurityError::CorruptLog(format!(
                "security recovery expected domain {:?} and LSN after {}, got {:?} at {}",
                authority_domain_id, previous_lsn, domain, lsn
            )));
        }
        projection.observe(&event)?;
        previous_lsn = lsn;
    }
    Ok(projection)
}

#[allow(dead_code)]
fn _recorded_event_type_is_explicit(_: &RecordedEvent) {}
