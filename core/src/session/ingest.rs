//! Adapter-to-core session report ingestion.
//!
//! A report describes the adapter's current view of one session. The durable
//! event log remains authoritative: this writer validates the generated report,
//! fences it by adapter source order, and appends one schema-owned session event.

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, ContinuationContextStatus, EventId, Generation, RuntimeSessionId,
    SessionActivityState, SessionConnectivityChanged, SessionConnectivityState,
    SessionGenerationBumped, SessionRegistered, SessionReport, SessionReportApplied,
    SessionReportSourceCursor, SessionState,
};

use crate::{
    acceptance::Clock,
    storage::{RecordedEvent, Storage},
};

use super::{
    allowed_activity_transition, allowed_connectivity_transition, events, SessionError,
    SessionIdentity, SessionRecord, SessionRegistry,
};

/// The durable outcome of ingesting one session report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestResult {
    /// The session slot was not previously known and was registered.
    Registered { event_id: EventId },
    /// A newer runtime-session generation superseded the prior live generation.
    GenerationBumped {
        event_id: EventId,
        from_generation: Generation,
        to_generation: Generation,
    },
    /// One equal-generation full report was durably applied.
    ReportApplied { event_id: EventId },
}

/// Read access to the live session projection used by ingestion.
///
/// The durable event log remains authoritative. This port exposes the hot-path
/// state needed to derive the next atomic event and uses static dispatch,
/// matching acceptance's `CommandStateLookup`.
pub trait SessionLookup: Send + Sync {
    fn current_session(
        &self,
        authority_domain_id: &AuthorityDomainId,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
    ) -> impl std::future::Future<Output = Result<Option<SessionRecord>, SessionError>> + Send;
}

/// A session projection that can fold a committed event.
///
/// The writer performs one append and one fold for every accepted report. A
/// security projection may additionally clamp incoming adapter evidence.
pub trait SessionProjection: SessionLookup {
    fn observe(&mut self, event: &RecordedEvent) -> Result<(), SessionError>;

    fn lockdown_active(&self) -> bool {
        false
    }
}

impl SessionLookup for SessionRegistry {
    async fn current_session(
        &self,
        authority_domain_id: &AuthorityDomainId,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
    ) -> Result<Option<SessionRecord>, SessionError> {
        self.require_authority_domain(authority_domain_id)?;
        Ok(self
            .get_live_session(adapter_id, deployment_scope, runtime_session_id)
            .cloned())
    }
}

impl SessionProjection for SessionRegistry {
    fn observe(&mut self, event: &RecordedEvent) -> Result<(), SessionError> {
        SessionRegistry::observe(self, event)
    }

    fn lockdown_active(&self) -> bool {
        SessionRegistry::lockdown_active(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedSessionReport {
    pub identity: SessionIdentity,
    pub connectivity: SessionConnectivityState,
    pub activity: SessionActivityState,
    pub source_cursor: SessionReportSourceCursor,
}

/// Ingest one generated adapter session report.
///
/// Validation happens before projection lookup. Runtime-session generation is
/// compared before source order; for an equal generation, source order is
/// compared before any field transition is derived. Every accepted report is
/// represented by exactly one durable event and folded only after append.
pub async fn ingest_session_report<S, L>(
    storage: &S,
    session_lookup: &mut L,
    authority_domain_id: &AuthorityDomainId,
    mut report: SessionReport,
) -> Result<IngestResult, SessionError>
where
    S: Storage,
    L: SessionProjection,
{
    if authority_domain_id.value.is_empty() {
        return Err(SessionError::CorruptRecord(
            "session report authority_domain_id is empty".to_owned(),
        ));
    }
    if report.spawn_origin.is_some() {
        return Err(SessionError::CorruptRecord(
            "ordinary session report ingress rejects spawn_origin; managed reports require the staged-successor boundary"
                .to_owned(),
        ));
    }
    let mut validated = validate_ordinary_report(&report)?;
    let live = session_lookup
        .current_session(
            authority_domain_id,
            &validated.identity.adapter_id,
            &validated.identity.deployment_scope,
            &validated.identity.runtime_session_id,
        )
        .await?;

    let Some(current) = live else {
        clamp_report_for_lockdown(session_lookup, &mut report, &mut validated);
        let event = events::registered(
            authority_domain_id.clone(),
            SessionRegistered {
                adapter_id: report.adapter_id,
                deployment_scope: report.deployment_scope,
                runtime_session_id: report.runtime_session_id,
                session_generation: report.session_generation,
                initial_state: Some(SessionState {
                    connectivity: validated.connectivity as i32,
                    activity: validated.activity as i32,
                }),
                project: report.project,
                cwd: report.cwd,
                name: report.name,
                spawn_origin: report.spawn_origin,
                model: report.model,
                source_cursor: report.source_cursor,
            },
        );
        let event_id =
            append_and_apply(storage, session_lookup, authority_domain_id, event).await?;
        return Ok(IngestResult::Registered { event_id });
    };

    let live_generation = current.identity.session_generation;
    match validated
        .identity
        .session_generation
        .value
        .cmp(&live_generation.value)
    {
        std::cmp::Ordering::Greater => {
            clamp_report_for_lockdown(session_lookup, &mut report, &mut validated);
            let to_generation = validated.identity.session_generation;
            let event = events::generation_bumped(
                authority_domain_id.clone(),
                SessionGenerationBumped {
                    adapter_id: report.adapter_id,
                    deployment_scope: report.deployment_scope,
                    runtime_session_id: report.runtime_session_id,
                    from_generation: Some(live_generation),
                    to_generation: report.session_generation,
                    initial_state: Some(SessionState {
                        connectivity: validated.connectivity as i32,
                        activity: validated.activity as i32,
                    }),
                    project: report.project,
                    cwd: report.cwd,
                    name: report.name,
                    model: report.model,
                    spawn_origin: report.spawn_origin,
                    source_cursor: report.source_cursor,
                },
            );
            let event_id =
                append_and_apply(storage, session_lookup, authority_domain_id, event).await?;
            Ok(IngestResult::GenerationBumped {
                event_id,
                from_generation: live_generation,
                to_generation,
            })
        }
        std::cmp::Ordering::Equal => {
            if let Some(live_cursor) = current.last_source_cursor {
                if !source_cursor_strictly_after(&validated.source_cursor, &live_cursor) {
                    return Err(SessionError::StaleSourceCursor {
                        live: live_cursor,
                        reported: validated.source_cursor,
                    });
                }
            }

            clamp_report_for_lockdown(session_lookup, &mut report, &mut validated);
            let current_connectivity = current.state.connectivity();
            if validated.connectivity != current_connectivity
                && !allowed_connectivity_transition(current_connectivity, validated.connectivity)
            {
                return Err(invalid_transition(
                    current_connectivity,
                    validated.connectivity,
                ));
            }
            let current_activity = current.state.activity();
            if validated.activity != current_activity
                && !allowed_activity_transition(current_activity, validated.activity)
            {
                return Err(invalid_transition(current_activity, validated.activity));
            }

            let event = events::report_applied(
                authority_domain_id.clone(),
                SessionReportApplied {
                    report: Some(report),
                    previous_source_cursor: current.last_source_cursor,
                },
            );
            let event_id =
                append_and_apply(storage, session_lookup, authority_domain_id, event).await?;
            Ok(IngestResult::ReportApplied { event_id })
        }
        std::cmp::Ordering::Less => Err(SessionError::StaleGeneration {
            live: live_generation,
            reported: validated.identity.session_generation,
        }),
    }
}

fn clamp_report_for_lockdown<L: SessionProjection>(
    session_lookup: &L,
    report: &mut SessionReport,
    validated: &mut ValidatedSessionReport,
) {
    if session_lookup.lockdown_active() {
        report.connectivity = SessionConnectivityState::Stale as i32;
        validated.connectivity = SessionConnectivityState::Stale;
    }
}

/// Durably degrade every live session owned by an abnormally disconnected adapter.
///
/// Disconnect is core-authored evidence. It retains the legacy connectivity
/// delta because it must not consume or manufacture adapter source order.
pub fn adapter_stale_events(
    registry: &SessionRegistry,
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
) -> Result<Vec<patchbay_contracts::patchbay::StoredEventPayload>, SessionError> {
    registry.require_authority_domain(authority_domain_id)?;
    Ok(registry
        .sessions()
        .filter(|record| record.identity.adapter_id == *adapter_id)
        .flat_map(|record| {
            let mut degraded = Vec::with_capacity(2);
            if record.state.connectivity() != SessionConnectivityState::Stale {
                degraded.push(events::encode(&events::connectivity_changed(
                    authority_domain_id.clone(),
                    SessionConnectivityChanged {
                        adapter_id: Some(record.identity.adapter_id.clone()),
                        deployment_scope: record.identity.deployment_scope.clone(),
                        runtime_session_id: Some(record.identity.runtime_session_id.clone()),
                        session_generation: Some(record.identity.session_generation),
                        from: record.state.connectivity,
                        to: SessionConnectivityState::Stale as i32,
                    },
                )));
            }
            if record.state.activity() != SessionActivityState::Unknown {
                degraded.push(events::encode(&events::activity_changed(
                    authority_domain_id.clone(),
                    patchbay_contracts::patchbay::SessionActivityChanged {
                        adapter_id: Some(record.identity.adapter_id.clone()),
                        deployment_scope: record.identity.deployment_scope.clone(),
                        runtime_session_id: Some(record.identity.runtime_session_id.clone()),
                        session_generation: Some(record.identity.session_generation),
                        from: record.state.activity,
                        to: SessionActivityState::Unknown as i32,
                    },
                )));
            }
            degraded
        })
        .collect())
}

/// Compatibility composition for session-only callers. Production adapter
/// detach composes session and resource stale events into one audited batch.
pub async fn mark_adapter_sessions_stale<S: Storage>(
    storage: &S,
    registry: &mut SessionRegistry,
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
) -> Result<Vec<EventId>, SessionError> {
    let sources = adapter_stale_events(registry, authority_domain_id, adapter_id)?;
    let mut audit = crate::storage::AuditRecordDraft::new(
        crate::acceptance::SystemClock.now(),
        patchbay_contracts::patchbay::AuditEventKind::AdapterDetached,
    );
    audit.actor_id = Some(patchbay_contracts::patchbay::ActorId {
        value: adapter_id.value.clone(),
    });
    audit.reason_code = "adapter_detached".to_owned();
    let event_ids = if sources.is_empty() {
        storage.append_audit(authority_domain_id, audit).await?;
        Vec::new()
    } else {
        storage
            .append_batch_audited(authority_domain_id, sources, audit)
            .await?
            .source_event_ids
    };

    *registry = super::rebuild_from_log(storage, authority_domain_id).await?;
    Ok(event_ids)
}

async fn append_and_apply<S, L>(
    storage: &S,
    session_lookup: &mut L,
    authority_domain_id: &AuthorityDomainId,
    event: events::SessionStateEvent,
) -> Result<EventId, SessionError>
where
    S: Storage,
    L: SessionProjection,
{
    let payload = events::encode(&event);
    let event_id = storage.append(authority_domain_id, payload.clone()).await?;
    validate_event_id(&event_id, authority_domain_id)?;

    // The append is durable before this fold. A fold error therefore leaves a
    // committed event and an unusable hot projection; propagate the corruption
    // so callers rebuild from the authoritative log before reusing it.
    session_lookup.observe(&RecordedEvent {
        event_id: event_id.clone(),
        payload,
    })?;
    Ok(event_id)
}

pub(crate) fn validate_ordinary_report(
    report: &SessionReport,
) -> Result<ValidatedSessionReport, SessionError> {
    let validated = validate_report(report)?;
    if report.continuation_context_status != ContinuationContextStatus::Unspecified as i32 {
        return Err(SessionError::CorruptRecord(
            "ordinary session report carries continuation-only context status".to_owned(),
        ));
    }
    Ok(validated)
}

pub(crate) fn validate_report(
    report: &SessionReport,
) -> Result<ValidatedSessionReport, SessionError> {
    ContinuationContextStatus::try_from(report.continuation_context_status).map_err(|_| {
        SessionError::CorruptRecord(format!(
            "session report has unknown continuation context status {}",
            report.continuation_context_status
        ))
    })?;
    let adapter_id = report.adapter_id.clone().ok_or_else(|| {
        SessionError::CorruptRecord("session report is missing adapter_id".to_owned())
    })?;
    if adapter_id.value.is_empty() {
        return Err(SessionError::CorruptRecord(
            "session report adapter_id is empty".to_owned(),
        ));
    }
    if report.deployment_scope.is_empty() {
        return Err(SessionError::CorruptRecord(
            "session report deployment_scope is empty".to_owned(),
        ));
    }
    let runtime_session_id = report.runtime_session_id.clone().ok_or_else(|| {
        SessionError::CorruptRecord("session report is missing runtime_session_id".to_owned())
    })?;
    if runtime_session_id.value.is_empty() {
        return Err(SessionError::CorruptRecord(
            "session report runtime_session_id is empty".to_owned(),
        ));
    }
    let session_generation = report
        .session_generation
        .filter(|generation| generation.value > 0)
        .ok_or_else(|| {
            SessionError::CorruptRecord(
                "session report is missing a positive session_generation".to_owned(),
            )
        })?;
    let connectivity = SessionConnectivityState::try_from(report.connectivity).map_err(|_| {
        SessionError::CorruptRecord(format!(
            "session report has unknown connectivity state {}",
            report.connectivity
        ))
    })?;
    if connectivity == SessionConnectivityState::Unspecified {
        return Err(SessionError::CorruptRecord(
            "session report connectivity is unspecified".to_owned(),
        ));
    }
    let activity = SessionActivityState::try_from(report.activity).map_err(|_| {
        SessionError::CorruptRecord(format!(
            "session report has unknown activity state {}",
            report.activity
        ))
    })?;
    if activity == SessionActivityState::Unspecified {
        return Err(SessionError::CorruptRecord(
            "session report activity is unspecified".to_owned(),
        ));
    }
    let source_cursor = report.source_cursor.ok_or_else(|| {
        SessionError::CorruptRecord("session report is missing source_cursor".to_owned())
    })?;
    validate_source_cursor(&source_cursor, "session report")?;

    Ok(ValidatedSessionReport {
        identity: SessionIdentity {
            adapter_id,
            deployment_scope: report.deployment_scope.clone(),
            runtime_session_id,
            session_generation,
        },
        connectivity,
        activity,
        source_cursor,
    })
}

pub(crate) fn validate_source_cursor(
    cursor: &SessionReportSourceCursor,
    context: &str,
) -> Result<(), SessionError> {
    cursor
        .adapter_generation
        .filter(|generation| generation.value > 0)
        .ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "{context} source_cursor is missing a positive adapter_generation"
            ))
        })?;
    if cursor.revision == 0 {
        return Err(SessionError::CorruptRecord(format!(
            "{context} source_cursor revision is zero"
        )));
    }
    Ok(())
}

#[must_use]
pub(crate) fn source_cursor_strictly_after(
    reported: &SessionReportSourceCursor,
    live: &SessionReportSourceCursor,
) -> bool {
    let reported_generation = reported
        .adapter_generation
        .expect("validated reported source cursor");
    let live_generation = live
        .adapter_generation
        .expect("validated live source cursor");
    reported_generation.value > live_generation.value
        || (reported_generation == live_generation && reported.revision > live.revision)
}

fn invalid_transition<T: std::fmt::Debug>(from: T, to: T) -> SessionError {
    SessionError::InvalidTransition {
        from: format!("{from:?}"),
        to: format!("{to:?}"),
    }
}

fn validate_event_id(
    event_id: &EventId,
    expected_domain: &AuthorityDomainId,
) -> Result<(), SessionError> {
    match event_id.authority_domain_id.as_ref() {
        Some(actual_domain) if actual_domain == expected_domain => {}
        Some(actual_domain) => {
            return Err(SessionError::CorruptRecord(format!(
                "storage returned session state event for domain {:?}, expected {:?}",
                actual_domain, expected_domain
            )));
        }
        None => {
            return Err(SessionError::CorruptRecord(
                "storage returned session state event without authority_domain_id".to_owned(),
            ));
        }
    }
    match event_id.lsn.as_ref() {
        Some(lsn) if lsn.value > 0 => {}
        Some(_) => {
            return Err(SessionError::CorruptRecord(
                "storage returned session state event with zero LSN".to_owned(),
            ));
        }
        None => {
            return Err(SessionError::CorruptRecord(
                "storage returned session state event without an LSN".to_owned(),
            ));
        }
    }
    Ok(())
}
