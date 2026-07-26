//! Authenticated, allowlisted adapter diagnostic reports.
//!
//! This module owns the adapter-report boundary. It deliberately constructs the
//! durable Observation and its audit draft from the verified attachment and a
//! small generated payload, rather than copying caller-supplied identity or
//! content into the log.

use patchbay_contracts::patchbay::{
    typed_correlation, AdapterDiagnosticDetail, AdapterDiagnosticPayload,
    AdapterDiagnosticReport, AdapterDiagnosticSeverity, AdapterId, AdapterRegistration,
    ActorEndpointRef, AuditEventKind, AuthorityDomainId, FailureCode, Observation,
    ObservationKind, OperationKind, PayloadContentType, PayloadEnvelope, StoredEventKind,
    StoredEventPayload, TargetScope, TargetScopeKind,
};
use prost::Message;
use prost_types::Timestamp;

use crate::storage::{AuditRecordDraft, Storage, StorageError};

pub const ADAPTER_DIAGNOSTIC_SCHEMA: &str = "patchbay.AdapterDiagnosticPayload";

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedAdapterDiagnostic {
    pub observation: Observation,
    pub audit: AuditRecordDraft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterDiagnosticReceipt {
    pub observation_event_id: patchbay_contracts::patchbay::EventId,
    pub audit_event_id: patchbay_contracts::patchbay::EventId,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AdapterDiagnosticRejection {
    #[error("validation_failed: {0}")]
    ValidationFailed(String),
}

pub fn validate_adapter_diagnostic_report(
    report: AdapterDiagnosticReport,
    authenticated_adapter: &AdapterId,
    registration: &AdapterRegistration,
    received_at: Timestamp,
) -> Result<ValidatedAdapterDiagnostic, AdapterDiagnosticRejection> {
    let domain = registration
        .authority_domain_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| invalid("registration has no authority domain"))?;
    let registered_adapter = registration
        .adapter_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| invalid("registration has no adapter id"))?;
    if registered_adapter != authenticated_adapter {
        return Err(invalid("authenticated adapter does not match registration"));
    }
    let endpoint = registration
        .endpoint_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| invalid("registration has no endpoint id"))?;
    let adapter_generation = registration
        .adapter_generation
        .as_ref()
        .ok_or_else(|| invalid("registration has no adapter generation"))?;

    let report_domain = report
        .authority_domain_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| invalid("report has no authority domain"))?;
    if report_domain != domain {
        return Err(invalid("report authority domain does not match attachment"));
    }
    validate_timestamp(&received_at).map_err(invalid)?;
    let observed_at = report
        .observed_at
        .ok_or_else(|| invalid("report has no observed_at"))?;
    validate_timestamp(&observed_at).map_err(invalid)?;

    let target = report
        .target_scope
        .clone()
        .ok_or_else(|| invalid("report has no target scope"))?;
    validate_target(&target, authenticated_adapter).map_err(invalid)?;

    let mut command_id = None;
    for correlation in &report.correlations {
        match correlation.r#ref.as_ref() {
            Some(typed_correlation::Ref::CommandId(id)) if !id.value.is_empty() && command_id.is_none() => {
                command_id = Some(id.clone());
            }
            Some(typed_correlation::Ref::CommandId(_)) => {
                return Err(invalid("report must contain zero or one non-empty command correlation"));
            }
            Some(_) | None => return Err(invalid("report correlation must be a command id")),
        }
    }

    let envelope = report
        .payload
        .clone()
        .ok_or_else(|| invalid("report has no payload"))?;
    if envelope.content_type != PayloadContentType::Protobuf as i32
        || envelope.schema_ref != ADAPTER_DIAGNOSTIC_SCHEMA
    {
        return Err(invalid("report payload must be the exact protobuf diagnostic schema"));
    }
    let payload = AdapterDiagnosticPayload::decode(envelope.payload.as_slice())
        .map_err(|error| invalid(format!("diagnostic payload is malformed: {error}")))?;
    validate_code(&payload.code).map_err(invalid)?;
    if !(1..=1000).contains(&payload.count) {
        return Err(invalid("diagnostic count must be between 1 and 1000"));
    }
    if payload.adapter_generation.as_ref() != Some(adapter_generation) {
        return Err(invalid("diagnostic payload generation does not match attachment"));
    }
    let severity = AdapterDiagnosticSeverity::try_from(payload.severity)
        .map_err(|_| invalid("diagnostic severity is unknown"))?;
    if severity == AdapterDiagnosticSeverity::Unspecified {
        return Err(invalid("diagnostic severity is unspecified"));
    }
    let operation_kind = OperationKind::try_from(payload.operation_kind)
        .map_err(|_| invalid("diagnostic operation kind is unknown"))?;
    if !matches!(
        operation_kind,
        OperationKind::Unspecified
            | OperationKind::Spawn
            | OperationKind::Attach
            | OperationKind::Instruct
            | OperationKind::Cancel
            | OperationKind::Interrupt
            | OperationKind::Query
            | OperationKind::ApprovalResponse
            | OperationKind::ElicitationResponse
            | OperationKind::Reconfigure
            | OperationKind::SessionManagement
    ) {
        return Err(invalid("diagnostic operation kind is reserved or unavailable"));
    }
    if operation_kind == OperationKind::Unspecified && command_id.is_some() {
        return Err(invalid("a command-correlated diagnostic must name an operation kind"));
    }
    let failure_code = FailureCode::try_from(report.failure_code)
        .map_err(|_| invalid("diagnostic failure code is unknown"))?;
    if matches!(severity, AdapterDiagnosticSeverity::Warning | AdapterDiagnosticSeverity::Error)
        && failure_code == FailureCode::Unspecified
    {
        return Err(invalid("warning and error diagnostics require a canonical failure code"));
    }

    let payload = AdapterDiagnosticPayload {
        code: payload.code.clone(),
        severity: severity as i32,
        adapter_generation: Some(*adapter_generation),
        operation_kind: operation_kind as i32,
        count: payload.count,
    };
    let payload_envelope = PayloadEnvelope {
        payload: payload.encode_to_vec(),
        content_type: PayloadContentType::Protobuf as i32,
        schema_ref: ADAPTER_DIAGNOSTIC_SCHEMA.to_owned(),
    };
    let sender = ActorEndpointRef {
        actor_id: Some(patchbay_contracts::patchbay::ActorId {
            value: authenticated_adapter.value.clone(),
        }),
        endpoint_id: Some(endpoint.clone()),
        ..ActorEndpointRef::default()
    };
    let observation = Observation {
        authority_domain_id: Some(domain.clone()),
        sender: Some(sender.clone()),
        kind: ObservationKind::Event as i32,
        correlations: report.correlations.clone(),
        target_scope: Some(target.clone()),
        payload: Some(payload_envelope),
        observed_at: Some(observed_at),
        failure_code: failure_code as i32,
        ..Observation::default()
    };

    let mut audit = AuditRecordDraft::new(received_at, AuditEventKind::AdapterDiagnosticReported);
    audit.actor_id = Some(patchbay_contracts::patchbay::ActorId {
        value: authenticated_adapter.value.clone(),
    });
    audit.endpoint_id = Some(endpoint.clone());
    audit.target_scope = Some(target);
    audit.command_id = command_id;
    audit.failure_code = (failure_code != FailureCode::Unspecified).then_some(failure_code);
    audit.reason_code = payload.code.clone();
    audit.adapter_diagnostic = Some(AdapterDiagnosticDetail {
        adapter_id: Some(authenticated_adapter.clone()),
        adapter_generation: Some(*adapter_generation),
        severity: severity as i32,
        operation_kind: operation_kind as i32,
        count: payload.count,
        adapter_observed_at: Some(observed_at),
    });

    Ok(ValidatedAdapterDiagnostic { observation, audit })
}

pub async fn ingest_adapter_diagnostic<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    diagnostic: ValidatedAdapterDiagnostic,
) -> Result<AdapterDiagnosticReceipt, StorageError> {
    if diagnostic.observation.authority_domain_id.as_ref() != Some(authority_domain_id) {
        return Err(StorageError::InvalidAuditRecord(
            "diagnostic observation has the wrong authority domain".to_owned(),
        ));
    }
    let result = storage
        .append_audited(
            authority_domain_id,
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: diagnostic.observation.encode_to_vec(),
            },
            diagnostic.audit,
        )
        .await?;
    Ok(AdapterDiagnosticReceipt {
        observation_event_id: result.source_event_id,
        audit_event_id: result.audit_event_id,
    })
}

fn invalid(message: impl Into<String>) -> AdapterDiagnosticRejection {
    AdapterDiagnosticRejection::ValidationFailed(message.into())
}

fn validate_target(target: &TargetScope, adapter_id: &AdapterId) -> Result<(), String> {
    let kind = TargetScopeKind::try_from(target.kind)
        .map_err(|_| "target scope kind is unknown".to_owned())?;
    if target.adapter_id.as_ref() != Some(adapter_id) {
        return Err("target adapter does not match authenticated adapter".to_owned());
    }
    match kind {
        TargetScopeKind::Adapter => {
            if target.runtime_session_id.is_some()
                || target.session_generation.is_some()
                || !target.deployment_scope.is_empty()
            {
                return Err("adapter target contains runtime-session fields".to_owned());
            }
        }
        TargetScopeKind::RuntimeSession => {
            if target
                .runtime_session_id
                .as_ref()
                .is_none_or(|id| id.value.is_empty())
                || target.deployment_scope.is_empty()
                || target
                    .session_generation
                    .as_ref()
                    .is_none_or(|generation| generation.value == 0)
            {
                return Err("runtime-session target is incomplete".to_owned());
            }
        }
        _ => return Err("diagnostic target must be an adapter or runtime session".to_owned()),
    }
    Ok(())
}

fn validate_code(code: &str) -> Result<(), String> {
    if code.is_empty()
        || code.len() > 64
        || !code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err("diagnostic code must match [a-z0-9_]{1,64}".to_owned());
    }
    Ok(())
}

fn validate_timestamp(timestamp: &Timestamp) -> Result<(), String> {
    if !(-62_135_596_800..=253_402_300_799).contains(&timestamp.seconds)
        || !(0..1_000_000_000).contains(&timestamp.nanos)
    {
        return Err("timestamp is invalid".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{
        AdapterCapability, Generation, PayloadContentType, TypedCorrelation,
    };

    fn registration() -> AdapterRegistration {
        AdapterRegistration {
            adapter_id: Some(AdapterId { value: "pi".into() }),
            endpoint_id: Some(patchbay_contracts::patchbay::EndpointId { value: "pi-endpoint".into() }),
            authority_domain_id: Some(AuthorityDomainId { value: "main".into() }),
            adapter_generation: Some(Generation { value: 1 }),
            capability: Some(AdapterCapability::default()),
            ..Default::default()
        }
    }

    fn report() -> AdapterDiagnosticReport {
        AdapterDiagnosticReport {
            authority_domain_id: Some(AuthorityDomainId { value: "main".into() }),
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::Adapter as i32,
                adapter_id: Some(AdapterId { value: "pi".into() }),
                ..Default::default()
            }),
            observed_at: Some(Timestamp { seconds: 1, nanos: 0 }),
            failure_code: FailureCode::Unspecified as i32,
            payload: Some(PayloadEnvelope {
                payload: AdapterDiagnosticPayload {
                    code: "pi_adapter_started".into(),
                    severity: AdapterDiagnosticSeverity::Info as i32,
                    adapter_generation: Some(Generation { value: 1 }),
                    operation_kind: OperationKind::Unspecified as i32,
                    count: 1,
                }
                .encode_to_vec(),
                content_type: PayloadContentType::Protobuf as i32,
                schema_ref: ADAPTER_DIAGNOSTIC_SCHEMA.into(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn builds_verified_safe_source_and_audit() {
        let validated = validate_adapter_diagnostic_report(
            report(),
            &AdapterId { value: "pi".into() },
            &registration(),
            Timestamp { seconds: 2, nanos: 0 },
        )
        .expect("valid report");
        assert_eq!(validated.audit.kind, AuditEventKind::AdapterDiagnosticReported);
        assert_eq!(validated.audit.reason_code, "pi_adapter_started");
        assert_eq!(validated.observation.sender.unwrap().endpoint_id.unwrap().value, "pi-endpoint");
    }

    #[test]
    fn rejects_warning_without_failure_and_non_command_correlation() {
        let mut value = report();
        value.payload.as_mut().unwrap().payload = AdapterDiagnosticPayload {
            code: "pi_delivery_failed".into(),
            severity: AdapterDiagnosticSeverity::Error as i32,
            adapter_generation: Some(Generation { value: 1 }),
            count: 1,
            ..Default::default()
        }
        .encode_to_vec();
        assert!(validate_adapter_diagnostic_report(
            value.clone(),
            &AdapterId { value: "pi".into() },
            &registration(),
            Timestamp { seconds: 2, nanos: 0 },
        )
        .is_err());
        value.correlations.push(TypedCorrelation::default());
        assert!(validate_adapter_diagnostic_report(
            value,
            &AdapterId { value: "pi".into() },
            &registration(),
            Timestamp { seconds: 2, nanos: 0 },
        )
        .is_err());
    }
}
