//! In-memory session projection derived from durable session-state events.
//!
//! The event log is authoritative. [`SessionRegistry`] is a deterministic hot
//! lookup path that callers rebuild and keep current by feeding committed
//! [`RecordedEvent`] values through [`SessionRegistry::observe`].

use std::collections::HashMap;

use patchbay_contracts::patchbay::{
    session_state_event, AdapterId, AuthorityDomainId, Generation, RuntimeSessionId,
    SessionActivityChanged, SessionActivityState, SessionConnectivityChanged,
    SessionConnectivityState, SessionGenerationBumped, SessionRegistered, SessionRelabeled,
    SessionState, StoredEventKind, TargetScope,
};
use prost::Message;

use crate::{acceptance::TargetBinding, storage::RecordedEvent};

use super::{
    allowed_activity_transition, allowed_connectivity_transition, SessionError, SessionIdentity,
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionRegistry {
    sessions: HashMap<SessionLiveKey, SessionRecord>,
    tombstones: HashMap<SessionTombstoneKey, SessionTombstone>,
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
    /// Construct an empty session projection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one committed event into the projection.
    ///
    /// Events outside the `SessionState` family are ignored. Re-delivery of a
    /// previously folded session event is a no-op; malformed payloads and
    /// impossible state transitions fail immediately.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), SessionError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            SessionError::CorruptRecord(format!("unknown stored event kind {}", event.payload.kind))
        })?;
        if kind != StoredEventKind::SessionState {
            return Ok(());
        }

        let (event_domain, event_lsn) = event_identity(event)?;
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
                self.observe_registered(mutation, event_lsn)
            }
            session_state_event::Mutation::GenerationBumped(mutation) => {
                self.observe_generation_bumped(mutation, event_lsn)
            }
            session_state_event::Mutation::ConnectivityChanged(mutation) => {
                self.observe_connectivity_changed(mutation, event_lsn)
            }
            session_state_event::Mutation::ActivityChanged(mutation) => {
                self.observe_activity_changed(mutation, event_lsn)
            }
            session_state_event::Mutation::Relabeled(mutation) => {
                self.observe_relabeled(mutation, event_lsn)
            }
        }
    }

    /// Resolve a protocol target to the live delivery identity.
    ///
    /// A specifically requested tombstoned or non-live generation does not
    /// resolve. If no generation is supplied, the current live generation is
    /// selected. Connectivity is deliberately not a resolution criterion.
    #[must_use]
    pub fn resolve(&self, target_scope: &TargetScope) -> Option<TargetBinding> {
        let adapter_id = target_scope.adapter_id.as_ref()?;
        let runtime_session_id = target_scope.runtime_session_id.as_ref()?;

        if let Some(generation) = target_scope.session_generation.as_ref() {
            if self.is_tombstoned(
                adapter_id,
                &target_scope.deployment_scope,
                runtime_session_id,
                generation,
            ) {
                return None;
            }
        }

        let record = self.get_live_session(
            adapter_id,
            &target_scope.deployment_scope,
            runtime_session_id,
        )?;
        if target_scope
            .session_generation
            .as_ref()
            .is_some_and(|generation| generation != &record.identity.session_generation)
        {
            return None;
        }

        Some(TargetBinding {
            runtime_session_id: record.identity.runtime_session_id.clone(),
            session_generation: record.identity.session_generation,
            adapter_id: record.identity.adapter_id.clone(),
        })
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
        let key = live_key(&identity);

        // First-write-wins. A replayed registration must never reset a later
        // generation, state-axis change, or relabel.
        if self.sessions.contains_key(&key) {
            return Ok(());
        }

        self.sessions.insert(
            key,
            SessionRecord {
                identity,
                state: initial_state,
                project: mutation.project.clone(),
                cwd: mutation.cwd.clone(),
                name: mutation.name.clone(),
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

        if let Some(existing) = self.get_tombstone(
            &from_identity.adapter_id,
            &from_identity.deployment_scope,
            &from_identity.runtime_session_id,
            &from_identity.session_generation,
        ) {
            if existing.adapter_id == from_identity.adapter_id
                && existing.deployment_scope == from_identity.deployment_scope
                && existing.superseded_at_lsn == event_lsn
            {
                return Ok(());
            }
            return Err(SessionError::CorruptLog(format!(
                "generation {:?} for runtime session {:?} has conflicting tombstones at LSN {event_lsn}",
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
        if !allowed_connectivity_transition(from, to) {
            return Err(SessionError::CorruptLog(format!(
                "disallowed connectivity transition {from:?} -> {to:?} at LSN {event_lsn}"
            )));
        }
        if self.is_stale_replay(&identity, event_lsn)? {
            return Ok(());
        }

        let record = self.live_record_mut(&identity, "connectivity change", event_lsn)?;
        if record
            .last_authoritative_lsn
            .is_some_and(|last_lsn| event_lsn <= last_lsn)
        {
            return Ok(());
        }
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
        if self.is_stale_replay(&identity, event_lsn)? {
            return Ok(());
        }

        let record = self.live_record_mut(&identity, "activity change", event_lsn)?;
        if record
            .last_authoritative_lsn
            .is_some_and(|last_lsn| event_lsn <= last_lsn)
        {
            return Ok(());
        }
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
        if self.is_stale_replay(&identity, event_lsn)? {
            return Ok(());
        }

        let record = self.live_record_mut(&identity, "relabel", event_lsn)?;
        if record
            .last_authoritative_lsn
            .is_some_and(|last_lsn| event_lsn <= last_lsn)
        {
            return Ok(());
        }
        record.project.clone_from(&mutation.project);
        record.cwd.clone_from(&mutation.cwd);
        record.name.clone_from(&mutation.name);
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

    /// A redelivered pre-supersession event is inert. A newly committed event
    /// aimed at a tombstoned generation is log corruption: stale external
    /// evidence belongs in an audit/Observation record, not a SessionState
    /// mutation.
    fn is_stale_replay(
        &self,
        identity: &SessionIdentity,
        event_lsn: u64,
    ) -> Result<bool, SessionError> {
        let Some(tombstone) = self.get_tombstone(
            &identity.adapter_id,
            &identity.deployment_scope,
            &identity.runtime_session_id,
            &identity.session_generation,
        ) else {
            return Ok(false);
        };
        if tombstone.adapter_id != identity.adapter_id
            || tombstone.deployment_scope != identity.deployment_scope
        {
            return Err(SessionError::CorruptLog(format!(
                "tombstone identity collision for runtime session {:?}, generation {:?}",
                identity.runtime_session_id, identity.session_generation
            )));
        }
        if event_lsn <= tombstone.superseded_at_lsn {
            Ok(true)
        } else {
            Err(SessionError::CorruptLog(format!(
                "session state event at LSN {event_lsn} targets tombstoned generation {} superseded at LSN {}",
                identity.session_generation.value, tombstone.superseded_at_lsn
            )))
        }
    }
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
    let session_generation = generation.copied().ok_or_else(|| {
        SessionError::CorruptRecord(format!(
            "session {mutation_name} at LSN {event_lsn} is missing session_generation"
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
    Ok((authority_domain_id, lsn.value))
}
