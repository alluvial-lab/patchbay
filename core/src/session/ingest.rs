//! Adapter-to-core session report ingestion.
//!
//! A report describes the adapter's current view of one session. The durable
//! event log remains authoritative: this writer compares the report with the
//! hot session projection, validates the implied mutation, and appends exactly
//! one schema-owned session delta when state changed.

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, EventId, Generation, RuntimeSessionId, SessionActivityChanged,
    SessionActivityState, SessionConnectivityChanged, SessionConnectivityState,
    SessionGenerationBumped, SessionRegistered, SessionRelabeled, SessionState,
};

use crate::storage::Storage;

use super::{
    allowed_activity_transition, allowed_connectivity_transition, events, SessionError,
    SessionRecord, SessionRegistry,
};

/// An adapter-reported session observation.
///
/// The adapter reports the current identity tuple, state axes, and metadata.
/// The core derives the durable delta, including generation supersession.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReport {
    pub authority_domain_id: AuthorityDomainId,
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub runtime_session_id: RuntimeSessionId,
    pub session_generation: Generation,
    pub connectivity: SessionConnectivityState,
    pub activity: SessionActivityState,
    pub project: String,
    pub cwd: String,
    pub name: String,
}

/// The durable outcome of ingesting one session report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestResult {
    /// The session slot was not previously known and was registered.
    Registered { event_id: EventId },
    /// A newer generation superseded the prior live generation.
    ///
    /// One durable event performs both actions, so both event identifiers are
    /// intentionally the same canonical event id.
    GenerationBumped {
        tombstone_event_id: EventId,
        new_generation_event_id: EventId,
        from_generation: Generation,
        to_generation: Generation,
    },
    /// The connectivity axis changed.
    ConnectivityChanged {
        event_id: EventId,
        from: SessionConnectivityState,
        to: SessionConnectivityState,
    },
    /// The activity axis changed.
    ActivityChanged {
        event_id: EventId,
        from: SessionActivityState,
        to: SessionActivityState,
    },
    /// Session metadata changed without changing session identity.
    Relabeled { event_id: EventId },
    /// The report exactly matched the current projection.
    NoChange,
}

/// Read access to the live session projection used by ingestion.
///
/// The durable event log remains authoritative. This port exposes only the
/// hot-path state needed to derive the next delta and uses static dispatch,
/// matching acceptance's `CommandStateLookup`.
pub trait SessionLookup: Send + Sync {
    fn current_session(
        &self,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
    ) -> impl std::future::Future<Output = Option<SessionRecord>> + Send;
}

impl SessionLookup for SessionRegistry {
    async fn current_session(
        &self,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
    ) -> Option<SessionRecord> {
        self.get_live_session(adapter_id, deployment_scope, runtime_session_id)
            .cloned()
    }
}

/// Ingest an adapter-reported session observation.
///
/// The ordering is protocol-significant: validate the boundary, read the live
/// projection, validate the implied transition, durably append the delta, and
/// only then return. Callers keep the in-memory registry warm by observing the
/// committed event after this function succeeds; this writer never mutates the
/// projection before durability is established.
pub async fn ingest_session_report<S, L>(
    storage: &S,
    session_lookup: &L,
    report: SessionReport,
) -> Result<IngestResult, SessionError>
where
    S: Storage,
    L: SessionLookup,
{
    validate_authority_domain(&report)?;
    let authority_domain_id = report.authority_domain_id.clone();
    let live = session_lookup
        .current_session(
            &report.adapter_id,
            &report.deployment_scope,
            &report.runtime_session_id,
        )
        .await;

    let Some(current) = live else {
        let event = events::registered(
            authority_domain_id.clone(),
            SessionRegistered {
                adapter_id: Some(report.adapter_id),
                deployment_scope: report.deployment_scope,
                runtime_session_id: Some(report.runtime_session_id),
                session_generation: Some(report.session_generation),
                initial_state: Some(SessionState {
                    connectivity: report.connectivity as i32,
                    activity: report.activity as i32,
                }),
                project: report.project,
                cwd: report.cwd,
                name: report.name,
            },
        );
        let event_id = storage
            .append(&authority_domain_id, events::encode(&event))
            .await?;
        validate_event_id(&event_id, &authority_domain_id)?;
        return Ok(IngestResult::Registered { event_id });
    };

    let live_generation = current.identity.session_generation;
    match report.session_generation.value.cmp(&live_generation.value) {
        std::cmp::Ordering::Greater => {
            let event = events::generation_bumped(
                authority_domain_id.clone(),
                SessionGenerationBumped {
                    adapter_id: Some(report.adapter_id),
                    deployment_scope: report.deployment_scope,
                    runtime_session_id: Some(report.runtime_session_id),
                    from_generation: Some(live_generation),
                    to_generation: Some(report.session_generation),
                },
            );
            let event_id = storage
                .append(&authority_domain_id, events::encode(&event))
                .await?;
            validate_event_id(&event_id, &authority_domain_id)?;

            Ok(IngestResult::GenerationBumped {
                tombstone_event_id: event_id.clone(),
                new_generation_event_id: event_id,
                from_generation: live_generation,
                to_generation: report.session_generation,
            })
        }
        std::cmp::Ordering::Equal => {
            let current_connectivity = current.state.connectivity();
            if report.connectivity != current_connectivity {
                if !allowed_connectivity_transition(current_connectivity, report.connectivity) {
                    return Err(invalid_transition(
                        current_connectivity,
                        report.connectivity,
                    ));
                }
                let event = events::connectivity_changed(
                    authority_domain_id.clone(),
                    SessionConnectivityChanged {
                        adapter_id: Some(report.adapter_id),
                        deployment_scope: report.deployment_scope,
                        runtime_session_id: Some(report.runtime_session_id),
                        session_generation: Some(report.session_generation),
                        from: current_connectivity as i32,
                        to: report.connectivity as i32,
                    },
                );
                let event_id = storage
                    .append(&authority_domain_id, events::encode(&event))
                    .await?;
                validate_event_id(&event_id, &authority_domain_id)?;
                return Ok(IngestResult::ConnectivityChanged {
                    event_id,
                    from: current_connectivity,
                    to: report.connectivity,
                });
            }

            let current_activity = current.state.activity();
            if report.activity != current_activity {
                if !allowed_activity_transition(current_activity, report.activity) {
                    return Err(invalid_transition(current_activity, report.activity));
                }
                let event = events::activity_changed(
                    authority_domain_id.clone(),
                    SessionActivityChanged {
                        adapter_id: Some(report.adapter_id),
                        deployment_scope: report.deployment_scope,
                        runtime_session_id: Some(report.runtime_session_id),
                        session_generation: Some(report.session_generation),
                        from: current_activity as i32,
                        to: report.activity as i32,
                    },
                );
                let event_id = storage
                    .append(&authority_domain_id, events::encode(&event))
                    .await?;
                validate_event_id(&event_id, &authority_domain_id)?;
                return Ok(IngestResult::ActivityChanged {
                    event_id,
                    from: current_activity,
                    to: report.activity,
                });
            }

            if metadata_changed(&current, &report) {
                let event = events::relabeled(
                    authority_domain_id.clone(),
                    SessionRelabeled {
                        adapter_id: Some(report.adapter_id),
                        deployment_scope: report.deployment_scope,
                        runtime_session_id: Some(report.runtime_session_id),
                        session_generation: Some(report.session_generation),
                        project: report.project,
                        cwd: report.cwd,
                        name: report.name,
                    },
                );
                let event_id = storage
                    .append(&authority_domain_id, events::encode(&event))
                    .await?;
                validate_event_id(&event_id, &authority_domain_id)?;
                return Ok(IngestResult::Relabeled { event_id });
            }

            Ok(IngestResult::NoChange)
        }
        std::cmp::Ordering::Less => Err(SessionError::StaleGeneration {
            live: live_generation,
            reported: report.session_generation,
        }),
    }
}

fn validate_authority_domain(report: &SessionReport) -> Result<(), SessionError> {
    if report.authority_domain_id.value.is_empty() {
        return Err(SessionError::CorruptRecord(
            "session report authority_domain_id is empty".to_owned(),
        ));
    }
    Ok(())
}

fn metadata_changed(current: &SessionRecord, report: &SessionReport) -> bool {
    current.project != report.project || current.cwd != report.cwd || current.name != report.name
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
    if event_id.lsn.is_none() {
        return Err(SessionError::CorruptRecord(
            "storage returned session state event without an LSN".to_owned(),
        ));
    }
    Ok(())
}
