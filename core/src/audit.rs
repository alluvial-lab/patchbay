//! Typed, redacted audit production.
//!
//! Durable auditing is a required core side effect. Stderr is an observer only
//! and can never stand in for the durable sink.

use std::sync::Arc;

use async_trait::async_trait;
use patchbay_contracts::patchbay::{AuditEventKind, EventId};

use crate::storage::{AuditRecordDraft, Storage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditReceipt {
    Durable(EventId),
    DiagnosticOnly,
}

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("durable audit append failed: {0}")]
    Storage(String),
    #[error("audit sink did not return a durable receipt")]
    NotDurable,
}

#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn record(&self, draft: AuditRecordDraft) -> Result<AuditReceipt, AuditError>;
}

pub struct DurableAuditSink<S: Storage> {
    storage: S,
    domain: patchbay_contracts::patchbay::AuthorityDomainId,
}

impl<S: Storage> DurableAuditSink<S> {
    #[must_use]
    pub fn new(storage: S, domain: patchbay_contracts::patchbay::AuthorityDomainId) -> Self {
        Self { storage, domain }
    }
}

#[async_trait]
impl<S: Storage> AuditSink for DurableAuditSink<S> {
    async fn record(&self, draft: AuditRecordDraft) -> Result<AuditReceipt, AuditError> {
        self.storage
            .append_audit(&self.domain, draft)
            .await
            .map(AuditReceipt::Durable)
            .map_err(|error| AuditError::Storage(error.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct StderrAuditSink;

#[async_trait]
impl AuditSink for StderrAuditSink {
    async fn record(&self, draft: AuditRecordDraft) -> Result<AuditReceipt, AuditError> {
        eprintln!("{}", redacted_line(&draft));
        Ok(AuditReceipt::DiagnosticOnly)
    }
}

/// Fan out after durability. Diagnostic observers are intentionally best
/// effort; a diagnostic failure cannot veto a committed audit record.
pub struct RequiredAuditFanout {
    durable: Arc<dyn AuditSink>,
    diagnostics: Vec<Arc<dyn AuditSink>>,
}

impl RequiredAuditFanout {
    /// Construct the production composition from a typed durable sink. Keeping
    /// this constructor typed prevents a stderr-only production setup by
    /// construction; recording sinks can still be supplied directly in tests.
    #[must_use]
    pub fn new<S: Storage + 'static>(
        durable: Arc<DurableAuditSink<S>>,
        diagnostics: Vec<Arc<dyn AuditSink>>,
    ) -> Self {
        Self {
            durable,
            diagnostics,
        }
    }

    #[must_use]
    pub fn from_verified_durable(
        durable: Arc<dyn AuditSink>,
        diagnostics: Vec<Arc<dyn AuditSink>>,
    ) -> Self {
        Self {
            durable,
            diagnostics,
        }
    }
}

#[async_trait]
impl AuditSink for RequiredAuditFanout {
    async fn record(&self, draft: AuditRecordDraft) -> Result<AuditReceipt, AuditError> {
        let receipt = self.durable.record(draft.clone()).await?;
        if !matches!(receipt, AuditReceipt::Durable(_)) {
            return Err(AuditError::NotDurable);
        }
        for diagnostic in &self.diagnostics {
            let _ = diagnostic.record(draft.clone()).await;
        }
        Ok(receipt)
    }
}

fn safe_log_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_graphic() && character != '=' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn redacted_line(draft: &AuditRecordDraft) -> String {
    let kind = AuditEventKind::try_from(draft.kind as i32)
        .map(|kind| {
            kind.as_str_name()
                .trim_start_matches("AUDIT_EVENT_KIND_")
                .to_ascii_lowercase()
        })
        .unwrap_or_else(|_| "unknown".to_owned());
    let actor_id = draft
        .actor_id
        .as_ref()
        .map_or_else(String::new, |id| safe_log_value(&id.value));
    let endpoint_id = draft
        .endpoint_id
        .as_ref()
        .map_or_else(String::new, |id| safe_log_value(&id.value));
    let command_id = draft
        .command_id
        .as_ref()
        .map_or_else(String::new, |id| safe_log_value(&id.value));
    format!(
        "audit_event_kind={kind} actor_id={actor_id} endpoint_id={endpoint_id} command_id={command_id} reason_code={} source_network={}",
        safe_log_value(&draft.reason_code),
        safe_log_value(&draft.source_network),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::AuthorityDomainId;
    use prost_types::Timestamp;

    #[tokio::test]
    async fn stderr_sink_is_explicitly_diagnostic_only() {
        let draft = AuditRecordDraft::new(
            Timestamp {
                seconds: 1,
                nanos: 0,
            },
            AuditEventKind::LoginFailed,
        );
        assert_eq!(
            StderrAuditSink.record(draft).await.unwrap(),
            AuditReceipt::DiagnosticOnly
        );
    }

    #[test]
    fn line_contains_only_allowlisted_fields() {
        let draft = AuditRecordDraft::new(
            Timestamp {
                seconds: 1,
                nanos: 0,
            },
            AuditEventKind::LoginSucceeded,
        );
        let _ = AuthorityDomainId {
            value: "main".to_owned(),
        };
        let line = redacted_line(&draft);
        assert!(!line.contains("payload"));
        assert!(!line.contains("token"));
    }
}
