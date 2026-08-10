//! Integrity validation for complete authority-domain replay prefixes.
//!
//! This boundary applies only to complete log reads (or complete tails after an
//! explicit cursor). It validates adjacency among returned records. An
//! open-ended read still relies on the storage port's completeness contract for
//! an unknown omitted final tail unless the caller has a trusted high-water
//! mark. Filtered subscription, audit, and adapter-specific streams may
//! legitimately omit unrelated LSNs and must not use this validator.

use patchbay_contracts::patchbay::{AuthorityDomainId, StoredEventKind};

use super::RecordedEvent;

/// Framing failures found while consuming a complete authority-domain log.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplayIntegrityError {
    /// One returned record is structurally malformed.
    #[error("corrupt replay record: {0}")]
    CorruptRecord(String),
    /// The returned records cannot be one committed contiguous log prefix.
    #[error("corrupt replay log: {0}")]
    CorruptLog(String),
}

impl ReplayIntegrityError {
    /// Preserve the record/log distinction while mapping into a domain error.
    pub fn map<T>(
        self,
        corrupt_record: impl FnOnce(String) -> T,
        corrupt_log: impl FnOnce(String) -> T,
    ) -> T {
        match self {
            Self::CorruptRecord(message) => corrupt_record(message),
            Self::CorruptLog(message) => corrupt_log(message),
        }
    }
}

/// Framing facts accepted for one replay event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedReplayEvent {
    pub lsn: u64,
    pub kind: StoredEventKind,
}

/// Validate the next record in one complete authority-domain replay.
///
/// A cold rebuild starts with `previous_lsn = 0`. A complete snapshot tail
/// starts with the snapshot's validated LSN. Empty input needs no call and is
/// valid. The caller must apply the event successfully before adopting the
/// returned LSN as its next cursor. For an open-ended suffix, successful
/// adjacency validation does not independently prove that a faulty backend
/// returned the unknown final tail; exact bounded callers must also verify the
/// final returned LSN equals their trusted bound.
pub fn validate_next_replay_event(
    authority_domain_id: &AuthorityDomainId,
    previous_lsn: u64,
    event: &RecordedEvent,
) -> Result<ValidatedReplayEvent, ReplayIntegrityError> {
    if authority_domain_id.value.is_empty() {
        return Err(ReplayIntegrityError::CorruptRecord(
            "complete replay requested an empty authority domain".to_owned(),
        ));
    }

    let event_domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        ReplayIntegrityError::CorruptRecord("replay event has no authority domain".to_owned())
    })?;
    if event_domain.value.is_empty() {
        return Err(ReplayIntegrityError::CorruptRecord(
            "replay event has an empty authority domain".to_owned(),
        ));
    }
    if event_domain != authority_domain_id {
        return Err(ReplayIntegrityError::CorruptLog(format!(
            "replay event belongs to authority domain {}, expected {}",
            event_domain.value, authority_domain_id.value
        )));
    }

    let event_lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| ReplayIntegrityError::CorruptRecord("replay event has no LSN".to_owned()))?
        .value;

    let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
        ReplayIntegrityError::CorruptRecord(format!(
            "replay event at LSN {event_lsn} has unknown stored event kind {}",
            event.payload.kind
        ))
    })?;
    if kind == StoredEventKind::Unspecified {
        return Err(ReplayIntegrityError::CorruptLog(format!(
            "replay event at LSN {event_lsn} has unspecified stored event kind"
        )));
    }

    let expected_lsn = previous_lsn.checked_add(1).ok_or_else(|| {
        ReplayIntegrityError::CorruptLog(format!("replay cannot advance beyond LSN {previous_lsn}"))
    })?;
    if event_lsn != expected_lsn {
        return Err(ReplayIntegrityError::CorruptLog(format!(
            "replay event LSN {event_lsn} is not the exact successor {expected_lsn}"
        )));
    }

    Ok(ValidatedReplayEvent {
        lsn: event_lsn,
        kind,
    })
}
