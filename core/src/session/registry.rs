//! In-memory session projection derived from durable session-state events.
//!
//! The event log is authoritative. [`SessionRegistry`] is a deterministic hot
//! lookup path that callers rebuild and keep current by feeding committed
//! [`RecordedEvent`] values through [`SessionRegistry::observe`].

use std::collections::{BTreeMap, HashMap, HashSet};

use patchbay_contracts::patchbay::{
    security_lockdown_event, session_state_event, spawn_claim_event, AdapterId, AuthorityDomainId,
    ExternalRuntimeRef, Generation, LogicalTargetCandidateReleased, LogicalTargetCandidateReserved,
    LogicalTargetCreated, LogicalTargetId, LogicalTargetInitialCurrentAssigned,
    LogicalTargetProjectionRecord, RuntimeSessionId, SecurityLockdownEvent, SessionActivityChanged,
    SessionActivityState, SessionConnectivityChanged, SessionConnectivityState,
    SessionGenerationBumped, SessionModelChanged, SessionRegistered, SessionRelabeled,
    SessionReportApplied, SessionReportSourceCursor, SessionState, SpawnClaimEvent,
    SpawnPromotionCommitted, SpawnSuccessorEvidenceStaged, StoredEventKind, StoredEventPayload,
};
use prost::Message;

use crate::storage::RecordedEvent;

use super::{
    allowed_activity_transition, allowed_connectivity_transition,
    ingest::{source_cursor_strictly_after, validate_report, validate_source_cursor},
    LogicalTargetRegistry, SessionError, SessionIdentity, SessionStateEvent,
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

/// Private checkpoint provenance for one explicitly managed logical lineage.
///
/// Absence means legacy, never "infer managed from the current checkpoint
/// shape". Tombstones are repeated here deliberately: the writer records the
/// promotion provenance independently from both checkpoint projections so
/// hydration can require exact three-way agreement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedLineageCheckpoint {
    pub logical_target_id: LogicalTargetId,
    pub tombstones: Vec<SessionTombstone>,
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
    managed_lineages: HashSet<LogicalTargetId>,
    managed_tombstone_owners: HashMap<SessionTombstoneKey, LogicalTargetId>,
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
            managed_lineages: HashSet::new(),
            managed_tombstone_owners: HashMap::new(),
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
        Self::from_checkpoint_with_managed_lineages(
            authority_domain_id,
            checkpoint_lsn,
            live_records,
            tombstones,
            logical_targets,
            Vec::new(),
            lockdown_active,
        )
    }

    /// Restore sessions plus explicit managed-lineage checkpoint provenance.
    pub fn from_checkpoint_with_managed_lineages(
        authority_domain_id: AuthorityDomainId,
        checkpoint_lsn: u64,
        live_records: Vec<SessionRecord>,
        tombstones: Vec<SessionTombstone>,
        logical_targets: Vec<LogicalTargetProjectionRecord>,
        managed_lineages: Vec<ManagedLineageCheckpoint>,
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
        let (managed_lineages, managed_tombstone_owners) = validate_managed_lineage_checkpoint(
            checkpoint_lsn,
            managed_lineages,
            &logical_targets,
            &retained,
        )?;
        for (key, tombstone) in &retained {
            let valid = if let Some(logical_target_id) = managed_tombstone_owners.get(key) {
                checkpoint_managed_tombstone_has_current_successor(
                    &sessions,
                    &logical_targets,
                    logical_target_id,
                    tombstone,
                )
            } else {
                checkpoint_legacy_tombstone_has_current_successor(&sessions, tombstone)
            };
            if !valid {
                return Err(SessionError::CorruptRecord(
                    "session checkpoint tombstone has no later current generation in its explicit managed lineage or legacy runtime slot"
                        .to_owned(),
                ));
            }
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
            managed_lineages,
            managed_tombstone_owners,
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
                | StoredEventKind::SpawnClaim
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
        } else if kind == StoredEventKind::SpawnClaim {
            self.observe_spawn_claim(event, event_lsn)?;
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

    /// Deterministically encode explicit managed-lineage checkpoint provenance.
    #[must_use]
    pub fn managed_lineage_checkpoint_records(&self) -> Vec<ManagedLineageCheckpoint> {
        let mut logical_target_ids: Vec<_> = self.managed_lineages.iter().cloned().collect();
        logical_target_ids.sort_by(|left, right| left.value.cmp(&right.value));
        logical_target_ids
            .into_iter()
            .map(|logical_target_id| {
                let mut tombstones: Vec<_> = self
                    .managed_tombstone_owners
                    .iter()
                    .filter(|(_, owner)| *owner == &logical_target_id)
                    .map(|(key, _)| {
                        self.tombstones
                            .get(key)
                            .expect("managed tombstone provenance has a session tombstone")
                            .clone()
                    })
                    .collect();
                sort_tombstones(&mut tombstones);
                ManagedLineageCheckpoint {
                    logical_target_id,
                    tombstones,
                }
            })
            .collect()
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

    fn observe_spawn_claim(
        &mut self,
        event: &RecordedEvent,
        event_lsn: u64,
    ) -> Result<(), SessionError> {
        let claim_event =
            SpawnClaimEvent::decode(event.payload.payload.as_slice()).map_err(|error| {
                SessionError::CorruptRecord(format!(
                    "cannot decode spawn claim at LSN {event_lsn}: {error}"
                ))
            })?;
        if claim_event.authority_domain_id.as_ref() != Some(&self.authority_domain_id) {
            return Err(SessionError::CorruptLog(format!(
                "spawn claim at LSN {event_lsn} belongs to another authority domain"
            )));
        }
        match claim_event.mutation.as_ref().ok_or_else(|| {
            SessionError::CorruptRecord(format!(
                "spawn claim at LSN {event_lsn} is missing mutation"
            ))
        })? {
            spawn_claim_event::Mutation::Accepted(accepted) => {
                super::validate_spawn_claim_accepted(&self.authority_domain_id, accepted)
                    .map_err(|error| SessionError::CorruptLog(error.to_string()))?;
                let logical_target_id = accepted
                    .claim
                    .as_ref()
                    .and_then(|claim| claim.logical_target_id.clone())
                    .expect("accepted spawn claim validated");
                self.managed_lineages.insert(logical_target_id);
            }
            spawn_claim_event::Mutation::DispositionChanged(_) => {}
        }
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
        self.managed_lineages.insert(logical_target_id.clone());
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

        let managed_tombstone_key = prior_record.as_ref().map(|prior| SessionTombstoneKey {
            adapter_id: prior.identity.adapter_id.clone(),
            deployment_scope: prior.identity.deployment_scope.clone(),
            runtime_session_id: prior.identity.runtime_session_id.clone(),
            generation: prior.identity.session_generation,
        });
        if managed_tombstone_key
            .as_ref()
            .is_some_and(|key| self.managed_tombstone_owners.contains_key(key))
        {
            return Err(SessionError::CorruptLog(format!(
                "spawn promotion at LSN {event_lsn} duplicates managed tombstone provenance"
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
        self.managed_lineages.insert(logical_target_id.clone());
        if let Some(prior) = prior_record {
            let prior_key = live_key(&prior.identity);
            self.sessions.remove(&prior_key);
            let tombstone_key = managed_tombstone_key.expect("prior has a tombstone key");
            self.tombstones.insert(
                tombstone_key.clone(),
                SessionTombstone {
                    adapter_id: prior.identity.adapter_id,
                    deployment_scope: prior.identity.deployment_scope,
                    runtime_session_id: prior.identity.runtime_session_id,
                    superseded_generation: prior.identity.session_generation,
                    superseded_at_lsn: event_lsn,
                },
            );
            self.managed_tombstone_owners
                .insert(tombstone_key, logical_target_id.clone());
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

fn validate_managed_lineage_checkpoint(
    checkpoint_lsn: u64,
    records: Vec<ManagedLineageCheckpoint>,
    logical_targets: &LogicalTargetRegistry,
    session_tombstones: &HashMap<SessionTombstoneKey, SessionTombstone>,
) -> Result<
    (
        HashSet<LogicalTargetId>,
        HashMap<SessionTombstoneKey, LogicalTargetId>,
    ),
    SessionError,
> {
    let mut managed_lineages = HashSet::new();
    let mut managed_tombstone_owners = HashMap::new();
    for record in records {
        if record.logical_target_id.value.is_empty() {
            return Err(SessionError::CorruptRecord(
                "managed-lineage checkpoint marker has an empty logical_target_id".to_owned(),
            ));
        }
        if !managed_lineages.insert(record.logical_target_id.clone()) {
            return Err(SessionError::CorruptRecord(
                "session checkpoint contains duplicate managed-lineage markers".to_owned(),
            ));
        }
        for tombstone in record.tombstones {
            validate_checkpoint_tombstone(checkpoint_lsn, &tombstone)?;
            let key = tombstone_key(&tombstone);
            if session_tombstones.get(&key) != Some(&tombstone) {
                return Err(SessionError::CorruptRecord(
                    "managed-lineage marker has no exact session tombstone counterpart".to_owned(),
                ));
            }
            if managed_tombstone_owners
                .insert(key, record.logical_target_id.clone())
                .is_some()
            {
                return Err(SessionError::CorruptRecord(
                    "managed-lineage marker repeats a tombstone identity".to_owned(),
                ));
            }
            if !logical_target_has_exact_tombstone(
                logical_targets,
                &record.logical_target_id,
                &tombstone,
            ) {
                return Err(SessionError::CorruptRecord(
                    "managed-lineage marker has no exact logical-target tombstone counterpart"
                        .to_owned(),
                ));
            }
        }
    }

    for target in logical_targets.records() {
        for logical_tombstone in target.tombstones.values() {
            let external = &logical_tombstone.external_runtime_ref;
            let (Some(adapter_id), Some(runtime_session_id), Some(generation)) = (
                external.adapter_id.as_ref(),
                external.runtime_session_id.as_ref(),
                external.generation,
            ) else {
                return Err(SessionError::CorruptRecord(
                    "logical-target checkpoint tombstone has malformed identity".to_owned(),
                ));
            };
            let key = SessionTombstoneKey {
                adapter_id: adapter_id.clone(),
                deployment_scope: external.deployment_scope.clone(),
                runtime_session_id: runtime_session_id.clone(),
                generation,
            };
            if managed_tombstone_owners.get(&key) != Some(&target.logical_target_id) {
                return Err(SessionError::CorruptRecord(
                    "logical-target tombstone has no explicit managed-lineage marker".to_owned(),
                ));
            }
            if session_tombstones
                .get(&key)
                .is_none_or(|session_tombstone| {
                    session_tombstone.superseded_at_lsn != logical_tombstone.superseded_at_lsn
                })
            {
                return Err(SessionError::CorruptRecord(
                    "managed logical-target tombstone has no exact session tombstone counterpart"
                        .to_owned(),
                ));
            }
        }
    }

    Ok((managed_lineages, managed_tombstone_owners))
}

fn checkpoint_managed_tombstone_has_current_successor(
    sessions: &HashMap<SessionLiveKey, SessionRecord>,
    logical_targets: &LogicalTargetRegistry,
    logical_target_id: &LogicalTargetId,
    tombstone: &SessionTombstone,
) -> bool {
    if !logical_target_has_exact_tombstone(logical_targets, logical_target_id, tombstone) {
        return false;
    }
    let Some(current) = logical_targets
        .get(logical_target_id)
        .and_then(|target| target.current.as_ref())
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
    sessions.get(&current_key).is_some_and(|live| {
        live.identity.session_generation == generation
            && checkpoint_tombstone_precedes_live(tombstone, live)
    })
}

fn checkpoint_legacy_tombstone_has_current_successor(
    sessions: &HashMap<SessionLiveKey, SessionRecord>,
    tombstone: &SessionTombstone,
) -> bool {
    sessions
        .get(&SessionLiveKey {
            adapter_id: tombstone.adapter_id.clone(),
            deployment_scope: tombstone.deployment_scope.clone(),
            runtime_session_id: tombstone.runtime_session_id.clone(),
        })
        .is_some_and(|live| checkpoint_tombstone_precedes_live(tombstone, live))
}

fn checkpoint_tombstone_precedes_live(tombstone: &SessionTombstone, live: &SessionRecord) -> bool {
    tombstone.superseded_generation.value < live.identity.session_generation.value
        && tombstone.superseded_at_lsn
            <= live
                .last_authoritative_lsn
                .expect("checkpoint live records are validated before tombstones")
}

fn logical_target_has_exact_tombstone(
    logical_targets: &LogicalTargetRegistry,
    logical_target_id: &LogicalTargetId,
    tombstone: &SessionTombstone,
) -> bool {
    let superseded = ExternalRuntimeRef {
        adapter_id: Some(tombstone.adapter_id.clone()),
        deployment_scope: tombstone.deployment_scope.clone(),
        runtime_session_id: Some(tombstone.runtime_session_id.clone()),
        generation: Some(tombstone.superseded_generation),
    };
    logical_targets
        .get(logical_target_id)
        .is_some_and(|target| {
            target.tombstones.values().any(|retained| {
                retained.external_runtime_ref == superseded
                    && retained.superseded_at_lsn == tombstone.superseded_at_lsn
            })
        })
}

fn tombstone_key(tombstone: &SessionTombstone) -> SessionTombstoneKey {
    SessionTombstoneKey {
        adapter_id: tombstone.adapter_id.clone(),
        deployment_scope: tombstone.deployment_scope.clone(),
        runtime_session_id: tombstone.runtime_session_id.clone(),
        generation: tombstone.superseded_generation,
    }
}

fn sort_tombstones(tombstones: &mut [SessionTombstone]) {
    tombstones.sort_by(|left, right| {
        (
            &left.adapter_id.value,
            &left.deployment_scope,
            &left.runtime_session_id.value,
            left.superseded_generation.value,
        )
            .cmp(&(
                &right.adapter_id.value,
                &right.deployment_scope,
                &right.runtime_session_id.value,
                right.superseded_generation.value,
            ))
    });
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

#[cfg(test)]
mod tests {
    use patchbay_contracts::patchbay::RuntimeGenerationRef;

    use super::*;
    use crate::session::ExternalRuntimeOwnership;

    fn same_runtime_managed_checkpoint() -> (
        AuthorityDomainId,
        Vec<SessionRecord>,
        Vec<SessionTombstone>,
        Vec<LogicalTargetProjectionRecord>,
        Vec<ManagedLineageCheckpoint>,
    ) {
        let authority_domain_id = AuthorityDomainId {
            value: "main".to_owned(),
        };
        let adapter_id = AdapterId {
            value: "pi".to_owned(),
        };
        let runtime_session_id = RuntimeSessionId {
            value: "runtime-a".to_owned(),
        };
        let logical_target_id = LogicalTargetId {
            value: "target-a".to_owned(),
        };
        let external = |generation| ExternalRuntimeRef {
            adapter_id: Some(adapter_id.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime_session_id.clone()),
            generation: Some(Generation { value: generation }),
        };
        let prior = external(1);
        let successor = external(2);
        let prior_ref = RuntimeGenerationRef {
            logical_target_id: Some(logical_target_id.clone()),
            external_runtime: Some(prior.clone()),
        };
        let mut logical_targets = LogicalTargetRegistry::new(authority_domain_id.clone()).unwrap();
        logical_targets
            .create(
                logical_target_id.clone(),
                adapter_id.clone(),
                "machine-a".to_owned(),
            )
            .unwrap();
        logical_targets
            .assign_initial_current(&logical_target_id, prior)
            .unwrap();
        logical_targets
            .reserve_candidate(&logical_target_id, successor.clone())
            .unwrap();
        logical_targets
            .commit_reserved_candidate(&logical_target_id, Some(&prior_ref), &successor, 6)
            .unwrap();

        let live = SessionRecord {
            identity: SessionIdentity {
                adapter_id: adapter_id.clone(),
                deployment_scope: "machine-a".to_owned(),
                runtime_session_id: runtime_session_id.clone(),
                session_generation: Generation { value: 2 },
            },
            state: SessionState {
                connectivity: SessionConnectivityState::Live as i32,
                activity: SessionActivityState::Idle as i32,
            },
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "managed".to_owned(),
            model: "provider/model".to_owned(),
            last_source_cursor: None,
            last_authoritative_lsn: Some(6),
            tombstoned: false,
            superseded_at_lsn: None,
        };
        let tombstone = SessionTombstone {
            adapter_id,
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id,
            superseded_generation: Generation { value: 1 },
            superseded_at_lsn: 6,
        };
        (
            authority_domain_id,
            vec![live],
            vec![tombstone.clone()],
            logical_targets.checkpoint_records(),
            vec![ManagedLineageCheckpoint {
                logical_target_id,
                tombstones: vec![tombstone],
            }],
        )
    }

    #[test]
    fn same_runtime_managed_checkpoint_rejects_missing_logical_tombstone() {
        let (domain, live, tombstones, logical_targets, managed_lineages) =
            same_runtime_managed_checkpoint();
        let complete = SessionRegistry::from_checkpoint_with_managed_lineages(
            domain.clone(),
            6,
            live.clone(),
            tombstones.clone(),
            logical_targets.clone(),
            managed_lineages.clone(),
            false,
        )
        .expect("the complete same-runtime managed lineage is compatible");
        let external = |generation| ExternalRuntimeRef {
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "runtime-a".to_owned(),
            }),
            generation: Some(Generation { value: generation }),
        };
        let logical_target_id = LogicalTargetId {
            value: "target-a".to_owned(),
        };
        assert_eq!(
            complete.logical_targets().owner_of(&external(1)),
            Some(&logical_target_id)
        );
        assert_eq!(
            complete.logical_targets().owner_of(&external(2)),
            Some(&logical_target_id)
        );

        let mut asymmetric = logical_targets;
        asymmetric[0].tombstones.clear();
        assert!(matches!(
            SessionRegistry::from_checkpoint_with_managed_lineages(
                domain,
                6,
                live,
                tombstones,
                asymmetric,
                managed_lineages,
                false,
            ),
            Err(SessionError::CorruptRecord(message))
                if message.contains("no exact logical-target tombstone counterpart")
        ));
    }

    #[test]
    fn same_runtime_managed_checkpoint_rejects_missing_session_tombstone_but_legacy_hydrates() {
        let (domain, live, tombstones, logical_targets, managed_lineages) =
            same_runtime_managed_checkpoint();
        assert!(matches!(
            SessionRegistry::from_checkpoint_with_managed_lineages(
                domain.clone(),
                6,
                live.clone(),
                Vec::new(),
                logical_targets,
                managed_lineages,
                false,
            ),
            Err(SessionError::CorruptRecord(message))
                if message.contains("no exact session tombstone counterpart")
        ));

        let legacy = SessionRegistry::from_checkpoint_with_logical_targets(
            domain,
            6,
            live,
            tombstones,
            Vec::new(),
            false,
        )
        .expect("a session-only lineage without managed runtime markers remains compatible");
        assert_eq!(legacy.tombstones().count(), 1);
    }

    #[test]
    fn unmarked_legacy_adoption_hydrates_after_session_generation_history() {
        let (domain, live, tombstones, _, _) = same_runtime_managed_checkpoint();
        let logical_target_id = LogicalTargetId {
            value: "legacy-adopted".to_owned(),
        };
        let current = ExternalRuntimeRef {
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "runtime-a".to_owned(),
            }),
            generation: Some(Generation { value: 2 }),
        };
        let mut adopted = LogicalTargetRegistry::new(domain.clone()).unwrap();
        adopted
            .create(
                logical_target_id.clone(),
                AdapterId {
                    value: "pi".to_owned(),
                },
                "machine-a".to_owned(),
            )
            .unwrap();
        adopted
            .assign_initial_current(&logical_target_id, current)
            .unwrap();

        let hydrated = SessionRegistry::from_checkpoint_with_logical_targets(
            domain,
            6,
            live,
            tombstones,
            adopted.checkpoint_records(),
            false,
        )
        .expect("explicit initial-current adoption does not convert legacy history to managed");
        assert_eq!(hydrated.tombstones().count(), 1);
    }

    #[test]
    fn marked_changed_runtime_rejects_missing_logical_tombstone_despite_old_slot_reuse() {
        let domain = AuthorityDomainId {
            value: "main".to_owned(),
        };
        let adapter_id = AdapterId {
            value: "pi".to_owned(),
        };
        let logical_target_id = LogicalTargetId {
            value: "target-a".to_owned(),
        };
        let runtime = |id: &str, generation: u64| ExternalRuntimeRef {
            adapter_id: Some(adapter_id.clone()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: id.to_owned(),
            }),
            generation: Some(Generation { value: generation }),
        };
        let prior = runtime("runtime-a", 1);
        let successor = runtime("runtime-b", 2);
        let prior_ref = RuntimeGenerationRef {
            logical_target_id: Some(logical_target_id.clone()),
            external_runtime: Some(prior.clone()),
        };
        let mut logical_targets = LogicalTargetRegistry::new(domain.clone()).unwrap();
        logical_targets
            .create(
                logical_target_id.clone(),
                adapter_id.clone(),
                "machine-a".to_owned(),
            )
            .unwrap();
        logical_targets
            .assign_initial_current(&logical_target_id, prior.clone())
            .unwrap();
        logical_targets
            .reserve_candidate(&logical_target_id, successor.clone())
            .unwrap();
        logical_targets
            .commit_reserved_candidate(&logical_target_id, Some(&prior_ref), &successor, 6)
            .unwrap();

        let session = |id: &str, generation: u64| SessionRecord {
            identity: SessionIdentity {
                adapter_id: adapter_id.clone(),
                deployment_scope: "machine-a".to_owned(),
                runtime_session_id: RuntimeSessionId {
                    value: id.to_owned(),
                },
                session_generation: Generation { value: generation },
            },
            state: SessionState {
                connectivity: SessionConnectivityState::Live as i32,
                activity: SessionActivityState::Idle as i32,
            },
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: id.to_owned(),
            model: "provider/model".to_owned(),
            last_source_cursor: None,
            last_authoritative_lsn: Some(7),
            tombstoned: false,
            superseded_at_lsn: None,
        };
        let tombstone = SessionTombstone {
            adapter_id: adapter_id.clone(),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: RuntimeSessionId {
                value: "runtime-a".to_owned(),
            },
            superseded_generation: Generation { value: 1 },
            superseded_at_lsn: 6,
        };
        let managed_lineages = vec![ManagedLineageCheckpoint {
            logical_target_id: logical_target_id.clone(),
            tombstones: vec![tombstone.clone()],
        }];
        let complete_records = logical_targets.checkpoint_records();
        let complete = SessionRegistry::from_checkpoint_with_managed_lineages(
            domain.clone(),
            7,
            vec![session("runtime-b", 2), session("runtime-a", 2)],
            vec![tombstone.clone()],
            complete_records.clone(),
            managed_lineages.clone(),
            false,
        )
        .expect("complete marked changed-runtime history hydrates");
        assert_eq!(
            complete.logical_targets().owner_of(&prior),
            Some(&logical_target_id)
        );

        let mut asymmetric = complete_records;
        asymmetric[0].tombstones.clear();
        assert!(matches!(
            SessionRegistry::from_checkpoint_with_managed_lineages(
                domain,
                7,
                vec![session("runtime-b", 2), session("runtime-a", 2)],
                vec![tombstone],
                asymmetric,
                managed_lineages,
                false,
            ),
            Err(SessionError::CorruptRecord(message))
                if message.contains("no exact logical-target tombstone counterpart")
        ));
    }

    #[test]
    fn unmarked_logical_tombstone_is_a_cross_version_disposable_shape() {
        let (domain, live, tombstones, logical_targets, _) = same_runtime_managed_checkpoint();
        assert!(matches!(
            SessionRegistry::from_checkpoint_with_logical_targets(
                domain,
                6,
                live,
                tombstones,
                logical_targets,
                false,
            ),
            Err(SessionError::CorruptRecord(message))
                if message.contains("no explicit managed-lineage marker")
        ));
    }
}
