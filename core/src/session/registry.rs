//! In-memory session projection derived from durable session-state events.
//!
//! The event log is authoritative. [`SessionRegistry`] is a deterministic hot
//! lookup path that callers rebuild and keep current by feeding committed
//! [`RecordedEvent`] values through [`SessionRegistry::observe`].

use std::collections::{BTreeMap, HashMap};

use patchbay_contracts::patchbay::{
    security_lockdown_event, session_state_event, AdapterId, AuthorityDomainId, ExternalRuntimeRef,
    Generation, LogicalTargetCandidateReleased, LogicalTargetCandidateReserved,
    LogicalTargetCreated, LogicalTargetId, LogicalTargetInitialCurrentAssigned,
    LogicalTargetProjectionRecord, RuntimeSessionId, SecurityLockdownEvent, SessionActivityChanged,
    SessionActivityState, SessionConnectivityChanged, SessionConnectivityState,
    SessionGenerationBumped, SessionModelChanged, SessionRegistered, SessionRelabeled,
    SessionReportApplied, SessionReportSourceCursor, SessionState, SpawnPromotionCommitted,
    SpawnSuccessorEvidenceStaged, StoredEventKind, StoredEventPayload,
};
use prost::Message;

use crate::storage::RecordedEvent;

use super::{
    allowed_activity_transition, allowed_connectivity_transition,
    ingest::{source_cursor_strictly_after, validate_report, validate_source_cursor},
    ExternalRuntimeOwnership, LogicalTargetRegistry, SessionError, SessionIdentity,
    SessionStateEvent,
};

/// The current in-memory state of one live session generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub identity: SessionIdentity,
    pub state: SessionState,
    pub project: String,
    pub cwd: String,
    pub name: String,
    pub model: String,
    pub last_source_cursor: Option<SessionReportSourceCursor>,
    pub last_authoritative_lsn: Option<u64>,
    pub tombstoned: bool,
    pub superseded_at_lsn: Option<u64>,
}

/// The indefinitely retained audit fact for one superseded generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTombstone {
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub runtime_session_id: RuntimeSessionId,
    pub superseded_generation: Generation,
    pub superseded_at_lsn: u64,
}

/// The in-memory session projection for one authority-domain log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRegistry {
    authority_domain_id: AuthorityDomainId,
    covered_through_lsn: Option<u64>,
    applied_events: BTreeMap<u64, StoredEventPayload>,
    sessions: HashMap<SessionLiveKey, SessionRecord>,
    tombstones: HashMap<SessionTombstoneKey, SessionTombstone>,
    logical_targets: LogicalTargetRegistry,
    lockdown_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayDisposition {
    New,
    Exact,
}

/// Identity minus generation: one live generation occupies each key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionLiveKey {
    adapter_id: AdapterId,
    deployment_scope: String,
    runtime_session_id: RuntimeSessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionTombstoneKey {
    adapter_id: AdapterId,
    deployment_scope: String,
    runtime_session_id: RuntimeSessionId,
    generation: Generation,
}

impl SessionRegistry {
    /// Construct an empty session projection bound to one authority domain.
    pub fn new(authority_domain_id: AuthorityDomainId) -> Result<Self, SessionError> {
        if authority_domain_id.value.is_empty() {
            return Err(SessionError::EmptyAuthorityDomain);
        }
        let logical_targets = LogicalTargetRegistry::new(authority_domain_id.clone())?;
        Ok(Self {
            authority_domain_id,
            covered_through_lsn: None,
            applied_events: BTreeMap::new(),
            sessions: HashMap::new(),
            tombstones: HashMap::new(),
            logical_targets,
            lockdown_active: false,
        })
    }

    /// Restore a complete session projection from a validated checkpoint.
    ///
    /// The covered prefix is compact: prior event bytes are not retained, so
    /// any direct re-feed at or below the anchor fails closed. Exact envelope
    /// redelivery remains available for tail events observed after hydration.
    pub fn from_checkpoint(
        authority_domain_id: AuthorityDomainId,
        checkpoint_lsn: u64,
        live_records: Vec<SessionRecord>,
        tombstones: Vec<SessionTombstone>,
        lockdown_active: bool,
    ) -> Result<Self, SessionError> {
        Self::from_checkpoint_with_logical_targets(
            authority_domain_id,
            checkpoint_lsn,
            live_records,
            tombstones,
            Vec::new(),
            lockdown_active,
        )
    }

    /// Restore sessions and the complete logical-target/reverse-index state.
    pub fn from_checkpoint_with_logical_targets(
        authority_domain_id: AuthorityDomainId,
        checkpoint_lsn: u64,
        live_records: Vec<SessionRecord>,
        tombstones: Vec<SessionTombstone>,
        logical_targets: Vec<LogicalTargetProjectionRecord>,
        lockdown_active: bool,
    ) -> Result<Self, SessionError> {
        if authority_domain_id.value.is_empty() {
            return Err(SessionError::EmptyAuthorityDomain);
        }
        if checkpoint_lsn == 0 {
            return Err(SessionError::CorruptRecord(
                "session checkpoint has a zero LSN".to_owned(),
            ));
        }

        let mut sessions = HashMap::new();
        for record in live_records {
            validate_checkpoint_record(checkpoint_lsn, &record)?;
            let key = live_key(&record.identity);
            if sessions.insert(key, record).is_some() {
                return Err(SessionError::CorruptRecord(
                    "session checkpoint contains duplicate live session identities".to_owned(),
                ));
            }
        }

        let logical_targets = LogicalTargetRegistry::from_checkpoint(
            authority_domain_id.clone(),
            checkpoint_lsn,
            logical_targets,
        )?;

        let mut retained = HashMap::new();
        for tombstone in tombstones {
            validate_checkpoint_tombstone(checkpoint_lsn, &tombstone)?;
            let key = SessionTombstoneKey {
                adapter_id: tombstone.adapter_id.clone(),
                deployment_scope: tombstone.deployment_scope.clone(),
                runtime_session_id: tombstone.runtime_session_id.clone(),
                generation: tombstone.superseded_generation,
            };
            if !checkpoint_tombstone_has_current_successor(&sessions, &logical_targets, &tombstone)
            {
                return Err(SessionError::CorruptRecord(
                    "session checkpoint tombstone has no later current generation in its legacy runtime slot or managed logical-target lineage"
                        .to_owned(),
                ));
            }
            if retained.insert(key, tombstone).is_some() {
                return Err(SessionError::CorruptRecord(
                    "session checkpoint contains duplicate tombstone identities".to_owned(),
                ));
            }
        }
        let mut lineages: HashMap<SessionLiveKey, Vec<(u64, u64)>> = HashMap::new();
        for tombstone in retained.values() {
            lineages
                .entry(SessionLiveKey {
                    adapter_id: tombstone.adapter_id.clone(),
                    deployment_scope: tombstone.deployment_scope.clone(),
                    runtime_session_id: tombstone.runtime_session_id.clone(),
                })
                .or_default()
                .push((
                    tombstone.superseded_generation.value,
                    tombstone.superseded_at_lsn,
                ));
        }
        for lineage in lineages.values_mut() {
            lineage.sort_unstable();
            if lineage.windows(2).any(|pair| pair[0].1 >= pair[1].1) {
                return Err(SessionError::CorruptRecord(
                    "session checkpoint tombstone lineage has non-increasing event LSNs".to_owned(),
                ));
            }
        }
        if !checkpoint_logical_tombstones_have_session_counterparts(&logical_targets, &retained) {
            return Err(SessionError::CorruptRecord(
                "managed logical-target checkpoint tombstone has no exact session tombstone counterpart"
                    .to_owned(),
            ));
        }

        if lockdown_active
            && sessions
                .values()
                .any(|record| record.state.connectivity == SessionConnectivityState::Live as i32)
        {
            return Err(SessionError::CorruptRecord(
                "active-lockdown checkpoint contains a live session".to_owned(),
            ));
        }

        Ok(Self {
            authority_domain_id,
            covered_through_lsn: Some(checkpoint_lsn),
            applied_events: BTreeMap::new(),
            sessions,
            tombstones: retained,
            logical_targets,
            lockdown_active,
        })
    }

    /// Return the authority domain whose log this projection folds.
    #[must_use]
    pub fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.authority_domain_id
    }

    /// Return the compact checkpoint prefix, when this registry was hydrated.
    #[must_use]
    pub fn covered_through_lsn(&self) -> Option<u64> {
        self.covered_through_lsn
    }

    pub(crate) fn require_authority_domain(
        &self,
        actual: &AuthorityDomainId,
    ) -> Result<(), SessionError> {
        if actual == &self.authority_domain_id {
            Ok(())
        } else {
            Err(SessionError::AuthorityDomainMismatch {
                expected: self.authority_domain_id.clone(),
                actual: actual.clone(),
            })
        }
    }

    /// Fold one committed event into the projection.
    ///
    /// Known events outside the session/security families are projection
    /// no-ops. Exact re-delivery of an already applied owned envelope is inert;
    /// reusing its domain-local LSN for different content is corrupt history.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), SessionError> {
        if let Some(covered_through_lsn) = self.covered_through_lsn {
            let (event_domain, event_lsn) = event_identity(event)?;
            self.require_authority_domain(event_domain)?;
            if event_lsn <= covered_through_lsn {
                return Err(SessionError::CorruptLog(format!(
                    "session checkpoint covers LSN {event_lsn}; compacted event bytes cannot be authenticated for redelivery through {covered_through_lsn}"
                )));
            }
        }
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            SessionError::CorruptRecord(format!("unknown stored event kind {}", event.payload.kind))
        })?;
        if kind == StoredEventKind::Unspecified {
            return Err(SessionError::CorruptLog(
                "session replay event kind is unspecified".to_owned(),
            ));
        }
        if !matches!(
            kind,
            StoredEventKind::SessionState
                | StoredEventKind::SecurityLockdown
                | StoredEventKind::SpawnSuccessorEvidenceStaged
                | StoredEventKind::SpawnPromotionCommitted
        ) {
            // Sibling kinds remain projection no-ops, including when their
            // framing is malformed. The one exception is an identity already
            // retained as an applied owned event: changing that durable
            // envelope's kind is conflicting redelivery, not a sibling event.
            if event.event_id.authority_domain_id.as_ref() == Some(&self.authority_domain_id) {
                if let Some(lsn) = event.event_id.lsn.as_ref() {
                    if self.applied_events.contains_key(&lsn.value) {
                        return self
                            .classify_redelivery(lsn.value, &event.payload)
                            .map(|_| ());
                    }
                }
            }
            return Ok(());
        }

        let (event_domain, event_lsn) = event_identity(event)?;
        self.require_authority_domain(event_domain)?;
        match self.classify_redelivery(event_lsn, &event.payload)? {
            ReplayDisposition::Exact => return Ok(()),
            ReplayDisposition::New => {}
        }

        if kind == StoredEventKind::SecurityLockdown {
            self.observe_security_lockdown(event)?;
        } else if kind == StoredEventKind::SpawnSuccessorEvidenceStaged {
            let staged = SpawnSuccessorEvidenceStaged::decode(event.payload.payload.as_slice())
                .map_err(|error| {
                    SessionError::CorruptRecord(format!(
                        "cannot decode staged successor at LSN {event_lsn}: {error}"
                    ))
                })?;
            self.observe_staged_successor(&staged)?;
        } else if kind == StoredEventKind::SpawnPromotionCommitted {
            let promotion = SpawnPromotionCommitted::decode(event.payload.payload.as_slice())
                .map_err(|error| {
                    SessionError::CorruptRecord(format!(
                        "cannot decode spawn promotion at LSN {event_lsn}: {error}"
                    ))
                })?;
            self.observe_spawn_promotion(&promotion, &event.event_id, event_lsn)?;
        } else {
            let state_event =
                SessionStateEvent::decode(event.payload.payload.as_slice()).map_err(|error| {
                    SessionError::CorruptRecord(format!(
                        "cannot decode session state event at LSN {event_lsn}: {error}"
                    ))
                })?;
            let state_domain = state_event.authority_domain_id.as_ref().ok_or_else(|| {
                SessionError::CorruptRecord(format!(
                    "session state event at LSN {event_lsn} is missing authority_domain_id"
                ))
            })?;
            if state_domain.value.is_empty() {
                return Err(SessionError::CorruptRecord(format!(
                    "session state event at LSN {event_lsn} has an empty authority_domain_id"
                )));
            }
            if state_domain != event_domain {
                return Err(SessionError::CorruptLog(format!(
                    "session state authority domain {:?} does not match event authority domain {:?} at LSN {event_lsn}",
                    state_domain, event_domain
                )));
            }

            match state_event.mutation.as_ref().ok_or_else(|| {
                SessionError::CorruptRecord(format!(
                    "session state event at LSN {event_lsn} is missing mutation"
                ))
            })? {
                session_state_event::Mutation::Registered(mutation) => {
                    self.observe_registered(mutation, event_lsn)?;
                }
                session_state_event::Mutation::GenerationBumped(mutation) => {
                    self.observe_generation_bumped(mutation, event_lsn)?;
                }
                session_state_event::Mutation::ConnectivityChanged(mutation) => {
                    self.observe_connectivity_changed(mutation, event_lsn)?;
                }
                session_state_event::Mutation::ActivityChanged(mutation) => {
                    self.observe_activity_changed(mutation, event_lsn)?;
                }
                session_state_event::Mutation::Relabeled(mutation) => {
                    self.observe_relabeled(mutation, event_lsn)?;
                }
                session_state_event::Mutation::ModelChanged(mutation) => {
                    self.observe_model_changed(mutation, event_lsn)?;
                }
                session_state_event::Mutation::ReportApplied(mutation) => {
                    self.observe_report_applied(mutation, event_lsn)?;
                }
                session_state_event::Mutation::LogicalTargetCreated(mutation) => {
                    self.observe_logical_target_created(mutation)?;
                }
                session_state_event::Mutation::LogicalTargetInitialCurrentAssigned(mutation) => {
                    self.observe_logical_target_initial_current_assigned(mutation)?;
                }
                session_state_event::Mutation::LogicalTargetCandidateReserved(mutation) => {
                    self.observe_logical_target_candidate_reserved(mutation)?;
                }
                session_state_event::Mutation::LogicalTargetCandidateReleased(mutation) => {
                    self.observe_logical_target_candidate_released(mutation)?;
                }
            }
        }

        self.applied_events.insert(event_lsn, event.payload.clone());
        Ok(())
    }

    fn classify_redelivery(
        &self,
        event_lsn: u64,
        payload: &StoredEventPayload,
    ) -> Result<ReplayDisposition, SessionError> {
        if let Some(applied) = self.applied_events.get(&event_lsn) {
            return if applied == payload {
                Ok(ReplayDisposition::Exact)
            } else {
                Err(SessionError::CorruptLog(format!(
                    "session event identity ({:?}, {event_lsn}) has conflicting durable envelopes",
                    self.authority_domain_id
                )))
            };
        }
        if let Some((&greatest_lsn, _)) = self.applied_events.last_key_value() {
            if event_lsn < greatest_lsn {
                return Err(SessionError::CorruptLog(format!(
                    "unseen session event LSN {event_lsn} precedes applied owned-event high-water mark {greatest_lsn}"
                )));
            }
        }
        Ok(ReplayDisposition::New)
    }

    /// Stable logical-target projection, including exact reverse ownership.
    #[must_use]
    pub fn logical_targets(&self) -> &LogicalTargetRegistry {
        &self.logical_targets
    }

    /// Mutable identity projection for later core-owned lifecycle folds.
    pub fn logical_targets_mut(&mut self) -> &mut LogicalTargetRegistry {
        &mut self.logical_targets
    }

    fn observe_logical_target_created(
        &mut self,
        mutation: &LogicalTargetCreated,
    ) -> Result<(), SessionError> {
        self.logical_targets.create(
            required_logical_target_id(mutation.logical_target_id.as_ref(), "creation")?,
            mutation.adapter_id.clone().ok_or_else(|| {
                SessionError::CorruptRecord(
                    "logical-target creation is missing adapter_id".to_owned(),
                )
            })?,
            mutation.deployment_scope.clone(),
        )?;
        Ok(())
    }

    fn observe_logical_target_initial_current_assigned(
        &mut self,
        mutation: &LogicalTargetInitialCurrentAssigned,
    ) -> Result<(), SessionError> {
        let logical_target_id =
            required_logical_target_id(mutation.logical_target_id.as_ref(), "initial current")?;
        let external = mutation.external_runtime_ref.clone().ok_or_else(|| {
            SessionError::CorruptRecord(
                "logical-target initial current assignment is missing external_runtime_ref"
                    .to_owned(),
            )
        })?;
        self.logical_targets
            .assign_initial_current(&logical_target_id, external)?;
        Ok(())
    }

    fn observe_logical_target_candidate_reserved(
        &mut self,
        mutation: &LogicalTargetCandidateReserved,
    ) -> Result<(), SessionError> {
        let logical_target_id =
            required_logical_target_id(mutation.logical_target_id.as_ref(), "candidate reserve")?;
        let external = mutation.external_runtime_ref.clone().ok_or_else(|| {
            SessionError::CorruptRecord(
                "logical-target candidate reservation is missing external_runtime_ref".to_owned(),
            )
        })?;
        self.logical_targets
            .reserve_candidate(&logical_target_id, external)?;
        Ok(())
    }

    fn observe_logical_target_candidate_released(
        &mut self,
        mutation: &LogicalTargetCandidateReleased,
    ) -> Result<(), SessionError> {
        let logical_target_id =
            required_logical_target_id(mutation.logical_target_id.as_ref(), "candidate release")?;
        let external = mutation.external_runtime_ref.as_ref().ok_or_else(|| {
            SessionError::CorruptRecord(
                "logical-target candidate release is missing external_runtime_ref".to_owned(),
            )
        })?;
        self.logical_targets
            .release_candidate(&logical_target_id, external)?;
        Ok(())
    }

    fn observe_staged_successor(
        &mut self,
        staged: &SpawnSuccessorEvidenceStaged,
    ) -> Result<(), SessionError> {
        super::validate_staged_successor(staged)
            .map_err(|error| SessionError::CorruptLog(error.to_string()))?;
        let target = staged
            .classified_target
            .as_ref()
            .expect("staged successor validated");
        let logical_target_id = target
            .logical_target_id
            .as_ref()
            .expect("staged successor validated");
        // Fresh staging may create the stable target and then discover that
        // another target already owns the candidate runtime. Fold through a
        // private identity projection so that rejection cannot leak the empty
        // target into a hot projection.
        let mut logical_targets = self.logical_targets.clone();
        if logical_targets.get(logical_target_id).is_none() {
            let claim = staged
                .exact_claim
                .as_ref()
                .expect("staged successor validated");
            if claim.expected_prior.is_some() {
                return Err(SessionError::CorruptLog(
                    "continuation staged successor has no existing logical target".to_owned(),
                ));
            }
            let report = staged.report.as_ref().expect("staged successor validated");
            logical_targets.create(
                logical_target_id.clone(),
                report
                    .adapter_id
                    .clone()
                    .expect("staged successor validated"),
                report.deployment_scope.clone(),
            )?;
        }
        logical_targets.reserve_candidate(
            logical_target_id,
            staged
                .external_runtime_reservation
                .clone()
                .expect("staged successor validated"),
        )?;
        self.logical_targets = logical_targets;
        Ok(())
    }

    fn observe_spawn_promotion(
        &mut self,
        promotion: &SpawnPromotionCommitted,
        event_id: &patchbay_contracts::patchbay::EventId,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        super::validate_spawn_promotion_result_order(promotion)
            .map_err(|error| SessionError::CorruptLog(error.to_string()))?;
        super::validate_spawn_promotion_envelope(promotion, event_id)
            .map_err(|error| SessionError::CorruptLog(error.to_string()))?;
        let accepted = promotion
            .accepted_claim
            .as_ref()
            .expect("promotion validated");
        let claim = accepted.claim.as_ref().expect("promotion validated");
        let promoted = promotion
            .promoted_runtime
            .as_ref()
            .expect("promotion validated");
        let logical_target_id = promoted
            .logical_target_id
            .as_ref()
            .expect("promotion validated");
        let staged = promotion
            .staged_successor
            .as_ref()
            .and_then(|evidence| evidence.staged.as_ref())
            .expect("promotion validated");
        let report = staged.report.as_ref().expect("promotion validated");
        let validated = validate_report(report)?;
        if self.lockdown_active && validated.connectivity == SessionConnectivityState::Live {
            return Err(SessionError::CorruptLog(format!(
                "spawn promotion at LSN {event_lsn} would publish live during lockdown"
            )));
        }
        let projected_target = self.logical_targets.get(logical_target_id).ok_or_else(|| {
            SessionError::CorruptLog(format!(
                "spawn promotion at LSN {event_lsn} references unknown logical target"
            ))
        })?;
        if projected_target.current.as_ref() != claim.expected_prior.as_ref()
            || projected_target.reserved_candidate.as_ref()
                != promotion.external_runtime_reservation.as_ref()
        {
            return Err(SessionError::CorruptLog(format!(
                "spawn promotion at LSN {event_lsn} does not match logical-target current/reservation pre-state"
            )));
        }

        let prior_record = if let Some(prior) = claim.expected_prior.as_ref() {
            let external = prior
                .external_runtime
                .as_ref()
                .expect("promotion validated prior");
            let prior_identity = SessionIdentity {
                adapter_id: external
                    .adapter_id
                    .clone()
                    .expect("promotion validated prior"),
                deployment_scope: external.deployment_scope.clone(),
                runtime_session_id: external
                    .runtime_session_id
                    .clone()
                    .expect("promotion validated prior"),
                session_generation: external.generation.expect("promotion validated prior"),
            };
            let record = self.get_session(&prior_identity).cloned().ok_or_else(|| {
                SessionError::CorruptLog(format!(
                    "spawn promotion at LSN {event_lsn} exact prior is not the current session"
                ))
            })?;
            Some(record)
        } else {
            None
        };
        let candidate_key = live_key(&validated.identity);
        if self.sessions.contains_key(&candidate_key)
            && prior_record
                .as_ref()
                .is_none_or(|prior| live_key(&prior.identity) != candidate_key)
        {
            return Err(SessionError::CorruptLog(format!(
                "spawn promotion at LSN {event_lsn} collides with another live session"
            )));
        }

        self.logical_targets.commit_reserved_candidate(
            logical_target_id,
            claim.expected_prior.as_ref(),
            promotion
                .external_runtime_reservation
                .as_ref()
                .expect("promotion validated"),
            event_lsn,
        )?;
        if let Some(prior) = prior_record {
            let prior_key = live_key(&prior.identity);
            self.sessions.remove(&prior_key);
            self.tombstones.insert(
                SessionTombstoneKey {
                    adapter_id: prior.identity.adapter_id.clone(),
                    deployment_scope: prior.identity.deployment_scope.clone(),
                    runtime_session_id: prior.identity.runtime_session_id.clone(),
                    generation: prior.identity.session_generation,
                },
                SessionTombstone {
                    adapter_id: prior.identity.adapter_id,
                    deployment_scope: prior.identity.deployment_scope,
                    runtime_session_id: prior.identity.runtime_session_id,
                    superseded_generation: prior.identity.session_generation,
                    superseded_at_lsn: event_lsn,
                },
            );
        }
        self.sessions.insert(
            candidate_key,
            SessionRecord {
                identity: validated.identity,
                state: SessionState {
                    connectivity: validated.connectivity as i32,
                    activity: validated.activity as i32,
                },
                project: report.project.clone(),
                cwd: report.cwd.clone(),
                name: report.name.clone(),
                model: report.model.clone(),
                last_source_cursor: Some(validated.source_cursor),
                last_authoritative_lsn: Some(event_lsn),
                tombstoned: false,
                superseded_at_lsn: None,
            },
        );
        Ok(())
    }

    /// Whether the replayed security posture currently clamps reports.
    #[must_use]
    pub fn lockdown_active(&self) -> bool {
        self.lockdown_active
    }

    fn observe_security_lockdown(&mut self, event: &RecordedEvent) -> Result<(), SessionError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        let source =
            SecurityLockdownEvent::decode(event.payload.payload.as_slice()).map_err(|error| {
                SessionError::CorruptRecord(format!(
                    "cannot decode security event at LSN {event_lsn}: {error}"
                ))
            })?;
        let source_domain = source.authority_domain_id.as_ref().ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "security event at LSN {event_lsn} has no authority domain"
            ))
        })?;
        if source_domain.value.is_empty() {
            return Err(SessionError::CorruptRecord(format!(
                "security event at LSN {event_lsn} has an empty authority domain"
            )));
        }
        if source_domain != event_domain {
            return Err(SessionError::CorruptLog(format!(
                "security event domain {:?} does not match {:?} at LSN {event_lsn}",
                source_domain, event_domain
            )));
        }
        match source.transition.ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "security event at LSN {event_lsn} has no transition"
            ))
        })? {
            security_lockdown_event::Transition::Entered(entered) => {
                let expected = entered.affected_runtime_session_count as usize;
                if expected != self.sessions.len() {
                    return Err(SessionError::CorruptLog(format!(
                        "lockdown entry at LSN {event_lsn} reports {expected} sessions, projection has {}",
                        self.sessions.len()
                    )));
                }
                for record in self.sessions.values_mut() {
                    record.state.connectivity = SessionConnectivityState::Stale as i32;
                    record.last_authoritative_lsn = Some(event_lsn);
                }
                self.lockdown_active = true;
            }
            security_lockdown_event::Transition::Exited(_) => {
                if !self.lockdown_active {
                    return Err(SessionError::CorruptLog(format!(
                        "lockdown exit at LSN {event_lsn} without active clamp"
                    )));
                }
                // Exit only clears the report clamp. It deliberately does not
                // synthesize a live signal; a later adapter report must do so.
                self.lockdown_active = false;
            }
        }
        Ok(())
    }

    /// Iterate over the authoritative live-session projection.
    ///
    /// Callers that serialize the records must impose a stable order because
    /// the registry's hash-map layout is intentionally not protocol state.
    pub fn sessions(&self) -> impl Iterator<Item = &SessionRecord> {
        self.sessions.values()
    }

    /// Look up the live record matching all four identity fields.
    #[must_use]
    pub fn get_session(&self, identity: &SessionIdentity) -> Option<&SessionRecord> {
        self.get_live_session(
            &identity.adapter_id,
            &identity.deployment_scope,
            &identity.runtime_session_id,
        )
        .filter(|record| record.identity.session_generation == identity.session_generation)
    }

    /// Look up the currently live generation for one runtime-session slot.
    #[must_use]
    pub fn get_live_session(
        &self,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
    ) -> Option<&SessionRecord> {
        self.sessions.get(&SessionLiveKey {
            adapter_id: adapter_id.clone(),
            deployment_scope: deployment_scope.to_owned(),
            runtime_session_id: runtime_session_id.clone(),
        })
    }

    /// Iterate over retained superseded-generation facts.
    pub fn tombstones(&self) -> impl Iterator<Item = &SessionTombstone> {
        self.tombstones.values()
    }

    /// Look up the retained tombstone for one session identity and generation.
    #[must_use]
    pub fn get_tombstone(
        &self,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
        generation: &Generation,
    ) -> Option<&SessionTombstone> {
        self.tombstones.get(&SessionTombstoneKey {
            adapter_id: adapter_id.clone(),
            deployment_scope: deployment_scope.to_owned(),
            runtime_session_id: runtime_session_id.clone(),
            generation: *generation,
        })
    }

    /// Return whether a session generation has been superseded.
    #[must_use]
    pub fn is_tombstoned(
        &self,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
        generation: &Generation,
    ) -> bool {
        self.get_tombstone(adapter_id, deployment_scope, runtime_session_id, generation)
            .is_some()
    }

    fn observe_registered(
        &mut self,
        mutation: &SessionRegistered,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        let identity = mutation_identity(
            mutation.adapter_id.as_ref(),
            &mutation.deployment_scope,
            mutation.runtime_session_id.as_ref(),
            mutation.session_generation.as_ref(),
            "registration",
            event_lsn,
        )?;
        let initial_state = mutation.initial_state.ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "session registration at LSN {event_lsn} is missing initial_state"
            ))
        })?;
        validate_state(&initial_state, "session registration", event_lsn)?;
        if let Some(source_cursor) = mutation.source_cursor.as_ref() {
            validate_source_cursor(source_cursor, "session registration")?;
        }
        if self.lockdown_active
            && initial_state.connectivity == SessionConnectivityState::Live as i32
        {
            return Err(SessionError::CorruptLog(format!(
                "session registration at LSN {event_lsn} would restore live connectivity during lockdown"
            )));
        }
        let key = live_key(&identity);

        if self.sessions.contains_key(&key) {
            return Err(SessionError::CorruptLog(format!(
                "session registration at LSN {event_lsn} duplicates an existing live slot"
            )));
        }

        self.sessions.insert(
            key,
            SessionRecord {
                identity,
                state: initial_state,
                project: mutation.project.clone(),
                cwd: mutation.cwd.clone(),
                name: mutation.name.clone(),
                model: mutation.model.clone(),
                last_source_cursor: mutation.source_cursor,
                last_authoritative_lsn: Some(event_lsn),
                tombstoned: false,
                superseded_at_lsn: None,
            },
        );
        Ok(())
    }

    fn observe_generation_bumped(
        &mut self,
        mutation: &SessionGenerationBumped,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        let from_identity = mutation_identity(
            mutation.adapter_id.as_ref(),
            &mutation.deployment_scope,
            mutation.runtime_session_id.as_ref(),
            mutation.from_generation.as_ref(),
            "generation bump",
            event_lsn,
        )?;
        let to_generation = mutation.to_generation.ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "session generation bump at LSN {event_lsn} is missing to_generation"
            ))
        })?;
        if to_generation.value <= from_identity.session_generation.value {
            return Err(SessionError::CorruptLog(format!(
                "session generation bump at LSN {event_lsn} is not strictly increasing: {} -> {}",
                from_identity.session_generation.value, to_generation.value
            )));
        }
        let initial_state = mutation.initial_state.ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "session generation bump at LSN {event_lsn} is missing initial_state"
            ))
        })?;
        validate_state(&initial_state, "session generation bump", event_lsn)?;
        if let Some(source_cursor) = mutation.source_cursor.as_ref() {
            validate_source_cursor(source_cursor, "session generation bump")?;
        }
        if self.lockdown_active
            && initial_state.connectivity == SessionConnectivityState::Live as i32
        {
            return Err(SessionError::CorruptLog(format!(
                "session generation bump at LSN {event_lsn} would restore live connectivity during lockdown"
            )));
        }

        if self
            .get_tombstone(
                &from_identity.adapter_id,
                &from_identity.deployment_scope,
                &from_identity.runtime_session_id,
                &from_identity.session_generation,
            )
            .is_some()
        {
            return Err(SessionError::CorruptLog(format!(
                "generation {:?} for runtime session {:?} was already superseded before LSN {event_lsn}",
                from_identity.session_generation, from_identity.runtime_session_id
            )));
        }

        let key = live_key(&from_identity);
        let current = self.sessions.get(&key).ok_or_else(|| {
            SessionError::CorruptLog(format!(
                "generation bump at LSN {event_lsn} references an unknown live session {:?}",
                from_identity.runtime_session_id
            ))
        })?;
        if current.identity != from_identity {
            return Err(SessionError::CorruptLog(format!(
                "generation bump at LSN {event_lsn} expects generation {}, but live generation is {}",
                from_identity.session_generation.value,
                current.identity.session_generation.value
            )));
        }

        let tombstone = SessionTombstone {
            adapter_id: from_identity.adapter_id.clone(),
            deployment_scope: from_identity.deployment_scope.clone(),
            runtime_session_id: from_identity.runtime_session_id.clone(),
            superseded_generation: from_identity.session_generation,
            superseded_at_lsn: event_lsn,
        };
        let mut next = current.clone();
        next.identity.session_generation = to_generation;
        next.state = initial_state;
        next.project.clone_from(&mutation.project);
        next.cwd.clone_from(&mutation.cwd);
        next.name.clone_from(&mutation.name);
        next.model.clone_from(&mutation.model);
        next.last_source_cursor = mutation.source_cursor;
        next.last_authoritative_lsn = Some(event_lsn);
        next.tombstoned = false;
        next.superseded_at_lsn = None;

        self.tombstones.insert(
            SessionTombstoneKey {
                adapter_id: tombstone.adapter_id.clone(),
                deployment_scope: tombstone.deployment_scope.clone(),
                runtime_session_id: tombstone.runtime_session_id.clone(),
                generation: tombstone.superseded_generation,
            },
            tombstone,
        );
        self.sessions.insert(key, next);
        Ok(())
    }

    fn observe_report_applied(
        &mut self,
        mutation: &SessionReportApplied,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        let report = mutation.report.as_ref().ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "session report application at LSN {event_lsn} is missing report"
            ))
        })?;
        let validated = validate_report(report)?;
        let key = live_key(&validated.identity);
        let current = self.sessions.get(&key).ok_or_else(|| {
            SessionError::CorruptLog(format!(
                "session report application at LSN {event_lsn} references an unknown live session {:?}",
                validated.identity.runtime_session_id
            ))
        })?;
        if current.identity != validated.identity {
            return Err(SessionError::CorruptLog(format!(
                "session report application at LSN {event_lsn} targets generation {}, but live generation is {}",
                validated.identity.session_generation.value,
                current.identity.session_generation.value
            )));
        }
        if current.last_source_cursor != mutation.previous_source_cursor {
            return Err(SessionError::CorruptLog(format!(
                "session report application at LSN {event_lsn} has previous source cursor {:?}, but projected cursor is {:?}",
                mutation.previous_source_cursor, current.last_source_cursor
            )));
        }
        if let Some(previous) = current.last_source_cursor {
            if !source_cursor_strictly_after(&validated.source_cursor, &previous) {
                return Err(SessionError::CorruptLog(format!(
                    "session report application at LSN {event_lsn} does not strictly advance source cursor: {:?} -> {:?}",
                    previous, validated.source_cursor
                )));
            }
        }
        if self.lockdown_active && validated.connectivity == SessionConnectivityState::Live {
            return Err(SessionError::CorruptLog(format!(
                "session report application at LSN {event_lsn} would restore live connectivity during lockdown"
            )));
        }
        let current_connectivity =
            connectivity_state(current.state.connectivity, "projected", event_lsn)?;
        if validated.connectivity != current_connectivity
            && !allowed_connectivity_transition(current_connectivity, validated.connectivity)
        {
            return Err(SessionError::CorruptLog(format!(
                "disallowed connectivity transition {current_connectivity:?} -> {:?} at LSN {event_lsn}",
                validated.connectivity
            )));
        }
        let current_activity = activity_state(current.state.activity, "projected", event_lsn)?;
        if validated.activity != current_activity
            && !allowed_activity_transition(current_activity, validated.activity)
        {
            return Err(SessionError::CorruptLog(format!(
                "disallowed activity transition {current_activity:?} -> {:?} at LSN {event_lsn}",
                validated.activity
            )));
        }

        // Validate the complete pre-state before installing any field. A failed
        // event therefore remains exactly non-mutating and does not claim its
        // replay identity in `applied_events`.
        let mut next = current.clone();
        next.state = SessionState {
            connectivity: validated.connectivity as i32,
            activity: validated.activity as i32,
        };
        next.project.clone_from(&report.project);
        next.cwd.clone_from(&report.cwd);
        next.name.clone_from(&report.name);
        next.model.clone_from(&report.model);
        next.last_source_cursor = Some(validated.source_cursor);
        next.last_authoritative_lsn = Some(event_lsn);
        self.sessions.insert(key, next);
        Ok(())
    }

    fn observe_connectivity_changed(
        &mut self,
        mutation: &SessionConnectivityChanged,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        let identity = mutation_identity(
            mutation.adapter_id.as_ref(),
            &mutation.deployment_scope,
            mutation.runtime_session_id.as_ref(),
            mutation.session_generation.as_ref(),
            "connectivity change",
            event_lsn,
        )?;
        let from = connectivity_state(mutation.from, "from", event_lsn)?;
        let to = connectivity_state(mutation.to, "to", event_lsn)?;
        if self.lockdown_active && to == SessionConnectivityState::Live {
            return Err(SessionError::CorruptLog(format!(
                "connectivity change at LSN {event_lsn} would restore live state during lockdown"
            )));
        }
        if !allowed_connectivity_transition(from, to) {
            return Err(SessionError::CorruptLog(format!(
                "disallowed connectivity transition {from:?} -> {to:?} at LSN {event_lsn}"
            )));
        }
        let record = self.live_record_mut(&identity, "connectivity change", event_lsn)?;
        let current = connectivity_state(record.state.connectivity, "projected", event_lsn)?;
        if current != from {
            return Err(SessionError::CorruptLog(format!(
                "connectivity change at LSN {event_lsn} expects {from:?}, but projected state is {current:?}"
            )));
        }
        record.state.connectivity = to as i32;
        record.last_authoritative_lsn = Some(event_lsn);
        Ok(())
    }

    fn observe_activity_changed(
        &mut self,
        mutation: &SessionActivityChanged,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        let identity = mutation_identity(
            mutation.adapter_id.as_ref(),
            &mutation.deployment_scope,
            mutation.runtime_session_id.as_ref(),
            mutation.session_generation.as_ref(),
            "activity change",
            event_lsn,
        )?;
        let from = activity_state(mutation.from, "from", event_lsn)?;
        let to = activity_state(mutation.to, "to", event_lsn)?;
        if !allowed_activity_transition(from, to) {
            return Err(SessionError::CorruptLog(format!(
                "disallowed activity transition {from:?} -> {to:?} at LSN {event_lsn}"
            )));
        }
        let record = self.live_record_mut(&identity, "activity change", event_lsn)?;
        let current = activity_state(record.state.activity, "projected", event_lsn)?;
        if current != from {
            return Err(SessionError::CorruptLog(format!(
                "activity change at LSN {event_lsn} expects {from:?}, but projected state is {current:?}"
            )));
        }
        record.state.activity = to as i32;
        record.last_authoritative_lsn = Some(event_lsn);
        Ok(())
    }

    fn observe_relabeled(
        &mut self,
        mutation: &SessionRelabeled,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        let identity = mutation_identity(
            mutation.adapter_id.as_ref(),
            &mutation.deployment_scope,
            mutation.runtime_session_id.as_ref(),
            mutation.session_generation.as_ref(),
            "relabel",
            event_lsn,
        )?;
        let record = self.live_record_mut(&identity, "relabel", event_lsn)?;
        record.project.clone_from(&mutation.project);
        record.cwd.clone_from(&mutation.cwd);
        record.name.clone_from(&mutation.name);
        record.last_authoritative_lsn = Some(event_lsn);
        Ok(())
    }

    fn observe_model_changed(
        &mut self,
        mutation: &SessionModelChanged,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        let identity = mutation_identity(
            mutation.adapter_id.as_ref(),
            &mutation.deployment_scope,
            mutation.runtime_session_id.as_ref(),
            mutation.session_generation.as_ref(),
            "model change",
            event_lsn,
        )?;
        let record = self.live_record_mut(&identity, "model change", event_lsn)?;
        if record.model != mutation.from {
            return Err(SessionError::CorruptLog(format!(
                "model change at LSN {event_lsn} expects prior model {:?}, but projected model is {:?}",
                mutation.from, record.model
            )));
        }
        record.model.clone_from(&mutation.to);
        record.last_authoritative_lsn = Some(event_lsn);
        Ok(())
    }

    fn live_record_mut(
        &mut self,
        identity: &SessionIdentity,
        mutation_name: &str,
        event_lsn: u64,
    ) -> Result<&mut SessionRecord, SessionError> {
        let record = self.sessions.get_mut(&live_key(identity)).ok_or_else(|| {
            SessionError::CorruptLog(format!(
                "session {mutation_name} at LSN {event_lsn} references an unknown live session {:?}",
                identity.runtime_session_id
            ))
        })?;
        if record.identity != *identity {
            return Err(SessionError::CorruptLog(format!(
                "session {mutation_name} at LSN {event_lsn} targets generation {}, but live generation is {}",
                identity.session_generation.value, record.identity.session_generation.value
            )));
        }
        Ok(record)
    }
}

fn required_logical_target_id(
    logical_target_id: Option<&LogicalTargetId>,
    transition: &str,
) -> Result<LogicalTargetId, SessionError> {
    logical_target_id.cloned().ok_or_else(|| {
        SessionError::CorruptRecord(format!(
            "logical-target {transition} is missing logical_target_id"
        ))
    })
}

fn validate_checkpoint_record(
    checkpoint_lsn: u64,
    record: &SessionRecord,
) -> Result<(), SessionError> {
    if record.identity.adapter_id.value.is_empty()
        || record.identity.deployment_scope.is_empty()
        || record.identity.runtime_session_id.value.is_empty()
        || record.identity.session_generation.value == 0
    {
        return Err(SessionError::CorruptRecord(
            "session checkpoint contains malformed live identity".to_owned(),
        ));
    }
    validate_state(&record.state, "session checkpoint", checkpoint_lsn)?;
    if record.state.connectivity == SessionConnectivityState::Unspecified as i32
        || record.state.activity == SessionActivityState::Unspecified as i32
        || record.tombstoned
        || record.superseded_at_lsn.is_some()
    {
        return Err(SessionError::CorruptRecord(
            "session checkpoint contains invalid live record state".to_owned(),
        ));
    }
    let revision = record.last_authoritative_lsn.ok_or_else(|| {
        SessionError::CorruptRecord(
            "session checkpoint live record has no authoritative revision".to_owned(),
        )
    })?;
    if revision == 0 || revision > checkpoint_lsn {
        return Err(SessionError::CorruptRecord(format!(
            "session checkpoint live revision {revision} is outside 1..={checkpoint_lsn}"
        )));
    }
    if let Some(cursor) = record.last_source_cursor.as_ref() {
        validate_source_cursor(cursor, "session checkpoint")?;
    }
    Ok(())
}

fn validate_checkpoint_tombstone(
    checkpoint_lsn: u64,
    tombstone: &SessionTombstone,
) -> Result<(), SessionError> {
    if tombstone.adapter_id.value.is_empty()
        || tombstone.deployment_scope.is_empty()
        || tombstone.runtime_session_id.value.is_empty()
        || tombstone.superseded_generation.value == 0
        || tombstone.superseded_at_lsn == 0
        || tombstone.superseded_at_lsn > checkpoint_lsn
    {
        return Err(SessionError::CorruptRecord(
            "session checkpoint contains a malformed tombstone".to_owned(),
        ));
    }
    Ok(())
}

fn checkpoint_tombstone_has_current_successor(
    sessions: &HashMap<SessionLiveKey, SessionRecord>,
    logical_targets: &LogicalTargetRegistry,
    tombstone: &SessionTombstone,
) -> bool {
    let valid_successor = |live: &SessionRecord| {
        tombstone.superseded_generation.value < live.identity.session_generation.value
            && tombstone.superseded_at_lsn
                <= live
                    .last_authoritative_lsn
                    .expect("checkpoint live records are validated before tombstones")
    };
    let superseded = ExternalRuntimeRef {
        adapter_id: Some(tombstone.adapter_id.clone()),
        deployment_scope: tombstone.deployment_scope.clone(),
        runtime_session_id: Some(tombstone.runtime_session_id.clone()),
        generation: Some(tombstone.superseded_generation),
    };
    if let Some(logical_target_id) = logical_targets.owner_of(&superseded) {
        let Some(target) = logical_targets.get(logical_target_id) else {
            return false;
        };
        if !target.tombstones.values().any(|retained| {
            retained.external_runtime_ref == superseded
                && retained.superseded_at_lsn == tombstone.superseded_at_lsn
        }) {
            return false;
        }
        let Some(current) = target
            .current
            .as_ref()
            .and_then(|runtime| runtime.external_runtime.as_ref())
        else {
            return false;
        };
        let (Some(adapter_id), Some(runtime_session_id), Some(generation)) = (
            current.adapter_id.as_ref(),
            current.runtime_session_id.as_ref(),
            current.generation,
        ) else {
            return false;
        };
        let current_key = SessionLiveKey {
            adapter_id: adapter_id.clone(),
            deployment_scope: current.deployment_scope.clone(),
            runtime_session_id: runtime_session_id.clone(),
        };
        return sessions.get(&current_key).is_some_and(|live| {
            live.identity.session_generation == generation && valid_successor(live)
        });
    }

    let legacy_key = SessionLiveKey {
        adapter_id: tombstone.adapter_id.clone(),
        deployment_scope: tombstone.deployment_scope.clone(),
        runtime_session_id: tombstone.runtime_session_id.clone(),
    };
    sessions.get(&legacy_key).is_some_and(valid_successor)
}

fn checkpoint_logical_tombstones_have_session_counterparts(
    logical_targets: &LogicalTargetRegistry,
    session_tombstones: &HashMap<SessionTombstoneKey, SessionTombstone>,
) -> bool {
    logical_targets.records().all(|target| {
        target.tombstones.values().all(|logical_tombstone| {
            let external = &logical_tombstone.external_runtime_ref;
            let (Some(adapter_id), Some(runtime_session_id), Some(generation)) = (
                external.adapter_id.as_ref(),
                external.runtime_session_id.as_ref(),
                external.generation,
            ) else {
                return false;
            };
            session_tombstones
                .get(&SessionTombstoneKey {
                    adapter_id: adapter_id.clone(),
                    deployment_scope: external.deployment_scope.clone(),
                    runtime_session_id: runtime_session_id.clone(),
                    generation,
                })
                .is_some_and(|session_tombstone| {
                    session_tombstone.superseded_at_lsn == logical_tombstone.superseded_at_lsn
                })
        })
    })
}

fn mutation_identity(
    adapter_id: Option<&AdapterId>,
    deployment_scope: &str,
    runtime_session_id: Option<&RuntimeSessionId>,
    generation: Option<&Generation>,
    mutation_name: &str,
    event_lsn: u64,
) -> Result<SessionIdentity, SessionError> {
    let adapter_id = adapter_id.cloned().ok_or_else(|| {
        SessionError::CorruptRecord(format!(
            "session {mutation_name} at LSN {event_lsn} is missing adapter_id"
        ))
    })?;
    if adapter_id.value.is_empty() {
        return Err(SessionError::CorruptRecord(format!(
            "session {mutation_name} at LSN {event_lsn} has an empty adapter_id"
        )));
    }
    if deployment_scope.is_empty() {
        return Err(SessionError::CorruptRecord(format!(
            "session {mutation_name} at LSN {event_lsn} has an empty deployment_scope"
        )));
    }
    let runtime_session_id = runtime_session_id.cloned().ok_or_else(|| {
        SessionError::CorruptRecord(format!(
            "session {mutation_name} at LSN {event_lsn} is missing runtime_session_id"
        ))
    })?;
    if runtime_session_id.value.is_empty() {
        return Err(SessionError::CorruptRecord(format!(
            "session {mutation_name} at LSN {event_lsn} has an empty runtime_session_id"
        )));
    }
    let session_generation = generation
        .copied()
        .filter(|generation| generation.value > 0)
        .ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "session {mutation_name} at LSN {event_lsn} is missing a positive session_generation"
            ))
        })?;

    Ok(SessionIdentity {
        adapter_id,
        deployment_scope: deployment_scope.to_owned(),
        runtime_session_id,
        session_generation,
    })
}

fn validate_state(
    state: &SessionState,
    record_name: &str,
    event_lsn: u64,
) -> Result<(), SessionError> {
    connectivity_state(state.connectivity, record_name, event_lsn)?;
    activity_state(state.activity, record_name, event_lsn)?;
    Ok(())
}

fn connectivity_state(
    raw: i32,
    field: &str,
    event_lsn: u64,
) -> Result<SessionConnectivityState, SessionError> {
    SessionConnectivityState::try_from(raw).map_err(|_| {
        SessionError::CorruptRecord(format!(
            "session state event at LSN {event_lsn} has unknown {field} connectivity state {raw}"
        ))
    })
}

fn activity_state(
    raw: i32,
    field: &str,
    event_lsn: u64,
) -> Result<SessionActivityState, SessionError> {
    SessionActivityState::try_from(raw).map_err(|_| {
        SessionError::CorruptRecord(format!(
            "session state event at LSN {event_lsn} has unknown {field} activity state {raw}"
        ))
    })
}

fn live_key(identity: &SessionIdentity) -> SessionLiveKey {
    SessionLiveKey {
        adapter_id: identity.adapter_id.clone(),
        deployment_scope: identity.deployment_scope.clone(),
        runtime_session_id: identity.runtime_session_id.clone(),
    }
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), SessionError> {
    let authority_domain_id = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        SessionError::CorruptRecord("session event has no authority domain".to_owned())
    })?;
    if authority_domain_id.value.is_empty() {
        return Err(SessionError::CorruptRecord(
            "session event has an empty authority domain".to_owned(),
        ));
    }
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| SessionError::CorruptRecord("session event has no LSN".to_owned()))?;
    if lsn.value == 0 {
        return Err(SessionError::CorruptRecord(
            "session event has zero LSN".to_owned(),
        ));
    }
    Ok((authority_domain_id, lsn.value))
}
