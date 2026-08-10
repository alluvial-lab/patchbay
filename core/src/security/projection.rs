//! Replay projection for the active security lockdown posture.

use patchbay_contracts::patchbay::{
    security_lockdown_event, AuthorityDomainId, BootstrapChannelKind, EventId, Generation,
    SecurityLockdownState, StoredEventKind,
};
use prost::Message;
use prost_types::Timestamp;

use crate::{
    acceptance::{OperationPosture, OperationPostureDenied},
    storage::RecordedEvent,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSecurityLockdown {
    pub authority_domain_id: AuthorityDomainId,
    pub reason_code: String,
    pub entered_at: Timestamp,
    pub entered_by: patchbay_contracts::patchbay::ActorEndpointRef,
    pub entered_event_id: EventId,
    pub invalidated_through_operator_session_generation: Generation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecurityPostureProjection {
    active: Option<ActiveSecurityLockdown>,
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("corrupt security record: {0}")]
    CorruptRecord(String),
    #[error("corrupt security log: {0}")]
    CorruptLog(String),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}

impl SecurityPostureProjection {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), SecurityError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            SecurityError::CorruptRecord(format!("unknown stored event kind {}", event.payload.kind))
        })?;
        if kind == StoredEventKind::Unspecified {
            return Err(SecurityError::CorruptLog(
                "security replay event kind is unspecified".to_owned(),
            ));
        }
        if kind != StoredEventKind::SecurityLockdown {
            return Ok(());
        }
        let event_domain = event
            .event_id
            .authority_domain_id
            .as_ref()
            .ok_or_else(|| SecurityError::CorruptRecord("security event has no authority domain".to_owned()))?;
        let event_lsn = event
            .event_id
            .lsn
            .as_ref()
            .ok_or_else(|| SecurityError::CorruptRecord("security event has no LSN".to_owned()))?
            .value;
        if event_domain.value.is_empty() {
            return Err(SecurityError::CorruptRecord("security event has empty authority domain".to_owned()));
        }
        let source = patchbay_contracts::patchbay::SecurityLockdownEvent::decode(
            event.payload.payload.as_slice(),
        )
        .map_err(|error| SecurityError::CorruptRecord(format!("cannot decode security event at LSN {event_lsn}: {error}")))?;
        let source_domain = source.authority_domain_id.as_ref().ok_or_else(|| {
            SecurityError::CorruptRecord(format!("security event at LSN {event_lsn} has no source domain"))
        })?;
        if source_domain != event_domain || source_domain.value.is_empty() {
            return Err(SecurityError::CorruptLog(format!(
                "security event domain {:?} does not match {:?} at LSN {event_lsn}",
                source_domain, event_domain
            )));
        }
        match source.transition.ok_or_else(|| {
            SecurityError::CorruptRecord(format!("security event at LSN {event_lsn} has no transition"))
        })? {
            security_lockdown_event::Transition::Entered(entered) => {
                let occurred_at = entered.occurred_at.ok_or_else(|| {
                    SecurityError::CorruptRecord(format!("lockdown entry at LSN {event_lsn} has no occurred_at"))
                })?;
                validate_timestamp(&occurred_at)?;
                validate_reason(&entered.reason_code)?;
                let entered_by = entered.entered_by.ok_or_else(|| {
                    SecurityError::CorruptRecord(format!("lockdown entry at LSN {event_lsn} has no verified issuer"))
                })?;
                let generation = entered.invalidated_through_operator_session_generation.ok_or_else(|| {
                    SecurityError::CorruptRecord(format!("lockdown entry at LSN {event_lsn} has no session generation floor"))
                })?;
                if generation.value == 0 || entered.affected_runtime_session_count > 1_000_000 {
                    return Err(SecurityError::CorruptRecord(format!("invalid lockdown entry bounds at LSN {event_lsn}")));
                }
                if let Some(active) = &self.active {
                    if active.authority_domain_id == *event_domain
                        && active.reason_code == entered.reason_code
                        && active.entered_event_id == event.event_id
                    {
                        return Ok(());
                    }
                    return Err(SecurityError::CorruptLog(format!(
                        "second lockdown entry at LSN {event_lsn} while posture is active"
                    )));
                }
                self.active = Some(ActiveSecurityLockdown {
                    authority_domain_id: event_domain.clone(),
                    reason_code: entered.reason_code,
                    entered_at: occurred_at,
                    entered_by,
                    entered_event_id: event.event_id.clone(),
                    invalidated_through_operator_session_generation: generation,
                });
            }
            security_lockdown_event::Transition::Exited(exited) => {
                validate_reason(&exited.reason_code)?;
                let channel = BootstrapChannelKind::try_from(exited.bootstrap_channel).map_err(|_| {
                    SecurityError::CorruptRecord(format!("unknown bootstrap channel at LSN {event_lsn}"))
                })?;
                if channel != BootstrapChannelKind::LoopbackAdmin {
                    return Err(SecurityError::CorruptLog(format!("unsupported bootstrap exit channel at LSN {event_lsn}")));
                }
                let entered_event_id = exited.entered_event_id.ok_or_else(|| {
                    SecurityError::CorruptRecord(format!("lockdown exit at LSN {event_lsn} has no entered event"))
                })?;
                let Some(active) = self.active.as_ref() else {
                    return Err(SecurityError::CorruptLog(format!("lockdown exit at LSN {event_lsn} without active posture")));
                };
                if active.entered_event_id != entered_event_id {
                    return Err(SecurityError::CorruptLog(format!("lockdown exit at LSN {event_lsn} references the wrong entry")));
                }
                self.active = None;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn active(&self) -> Option<&ActiveSecurityLockdown> {
        self.active.as_ref()
    }

    #[must_use]
    pub fn state(&self) -> SecurityLockdownState {
        self.active.as_ref().map_or_else(SecurityLockdownState::default, |active| {
            SecurityLockdownState {
                active: true,
                reason_code: active.reason_code.clone(),
                entered_at: Some(active.entered_at),
                entered_by: Some(active.entered_by.clone()),
                entered_event_id: Some(active.entered_event_id.clone()),
            }
        })
    }
}

impl OperationPosture for SecurityPostureProjection {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
    ) -> Result<(), OperationPostureDenied> {
        let Some(active) = &self.active else {
            return Ok(());
        };
        if &active.authority_domain_id != authority_domain_id {
            return Err(OperationPostureDenied::SecurityLockdown {
                reason_code: active.reason_code.clone(),
                entered_event_id: active.entered_event_id.clone(),
            });
        }
        Err(OperationPostureDenied::SecurityLockdown {
            reason_code: active.reason_code.clone(),
            entered_event_id: active.entered_event_id.clone(),
        })
    }
}

pub(crate) fn validate_reason(reason: &str) -> Result<(), SecurityError> {
    if reason.is_empty()
        || reason.len() > 64
        || !reason
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(SecurityError::CorruptRecord(
            "reason_code must match [a-z0-9_]{1,64}".to_owned(),
        ));
    }
    Ok(())
}

fn validate_timestamp(value: &Timestamp) -> Result<(), SecurityError> {
    const MIN_SECONDS: i64 = -62_135_596_800;
    const MAX_SECONDS: i64 = 253_402_300_799;
    if !(MIN_SECONDS..=MAX_SECONDS).contains(&value.seconds)
        || !(0..1_000_000_000).contains(&value.nanos)
    {
        return Err(SecurityError::CorruptRecord("invalid protobuf timestamp".to_owned()));
    }
    Ok(())
}
