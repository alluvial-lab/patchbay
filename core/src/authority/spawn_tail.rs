//! Durable descendant-grant completion derived from committed spawn facts.
//!
//! The fold never treats an in-memory latch as durable progress. It observes
//! accepted spawn Operations, successful result evidence, correlated session
//! registration/replacement, completion audit records, descendant grants, and
//! terminal transitions. [`SpawnDescendantTail::next_action`] then returns the
//! next missing durable step in audit → grant → completion order.

use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, AcceptedOperation, ActorId, AuditEventKind,
    AuditRecord, AuthorityDomainId, CommandId, CommandTransition, DescendantGrant, DeviceId,
    EndpointId, EventId, FailureCode, GrantId, GrantRevocationPolicy, Observation, ObservationKind,
    OperationKind, OperationState, Revocation, SessionGenerationBumped, SessionRegistered,
    SessionStateEvent, StoredEventKind, TargetScope, TargetScopeKind, TypedCorrelation,
};
use prost::Message;
use prost_types::Timestamp;

use crate::{
    acceptance::{exact_command_correlation, CommandIndex},
    storage::RecordedEvent,
};

use super::{
    grant_matches_request, AuthorityError, AuthorityRegistry, IssuerRef,
    DESCENDANT_GRANT_ALLOWED_KINDS,
};

const SPAWN_COMPLETION_REASON: &str = "spawn_completion";

type SpawnKey = (AuthorityDomainId, CommandId);

#[derive(Debug, Clone, PartialEq, Eq)]
struct AcceptedSpawn {
    accepted_lsn: u64,
    target_scope: TargetScope,
    spawning_grant_id: GrantId,
    subject_actor_id: ActorId,
    subject_endpoint_id: Option<EndpointId>,
    subject_device_id: Option<DeviceId>,
    correlations: Vec<TypedCorrelation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletionEvidence {
    event_id: EventId,
    target_scope: TargetScope,
    correlations: Vec<TypedCorrelation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionFact {
    event_lsn: u64,
    target_scope: TargetScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletionAuditFact {
    event: RecordedEvent,
    record: AuditRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TerminalFact {
    event_id: EventId,
    state: OperationState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DescendantGrantFact {
    event_lsn: u64,
    grant: DescendantGrant,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SpawnProgress {
    accepted: Option<AcceptedSpawn>,
    successful_result: Option<CompletionEvidence>,
    session: Option<SessionFact>,
    audit: Option<CompletionAuditFact>,
    descendant_grant: Option<DescendantGrantFact>,
    terminal: Option<(u64, TerminalFact)>,
}

/// The next durable action required to finish one spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnCompletionAction {
    RecordAudit(SpawnCompletionAudit),
    IssueDescendantGrant(DescendantGrantIssuance),
    CommitCompleted(SpawnCompletionCommit),
}

/// Verified fields for the durable spawn-completion audit producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCompletionAudit {
    pub authority_domain_id: AuthorityDomainId,
    pub spawn_operation_id: CommandId,
    pub completion_source_event_id: EventId,
    pub spawning_grant_id: GrantId,
    pub subject_actor_id: ActorId,
    pub subject_endpoint_id: Option<EndpointId>,
    pub subject_device_id: Option<DeviceId>,
    pub spawned_session_scope: TargetScope,
}

/// A descendant grant that should be durably persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescendantGrantIssuance {
    pub spawn_operation_id: CommandId,
    pub spawning_grant_id: GrantId,
    pub spawned_session_scope: TargetScope,
    pub subject_actor_id: ActorId,
    pub subject_endpoint_id: Option<EndpointId>,
    pub authority_domain_id: AuthorityDomainId,
    pub allowed_operation_kinds: Vec<OperationKind>,
    pub descendant_grant_id: GrantId,
    pub created_at: Timestamp,
    pub audit_id: EventId,
}

/// The final lifecycle transition, emitted only after authority is durable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCompletionCommit {
    pub spawn_operation_id: CommandId,
    pub from_state: OperationState,
    pub correlations: Vec<TypedCorrelation>,
}

/// Pure durable-action fold for descendant-grant completion.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpawnDescendantTail {
    authority_domain_id: Option<AuthorityDomainId>,
    spawns: HashMap<SpawnKey, SpawnProgress>,
    /// Canonical projections folded beside the completion facts. They make
    /// accepted-spawn authority and lifecycle eligibility depend on the prior
    /// durable LSN prefix rather than on self-consistent later records.
    authority: AuthorityRegistry,
    commands: CommandIndex,
}

impl SpawnDescendantTail {
    /// Construct an empty spawn-completion fold.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one committed event.
    ///
    /// Exact redelivery is idempotent. Conflicting durable facts for the same
    /// spawn are corrupt history and fail closed.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        let (event_domain, event_lsn) = event_identity(event, "spawn-tail event")?;
        self.bind_domain(event_domain)?;
        let event_domain = event_domain.clone();
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            AuthorityError::CorruptRecord(format!(
                "unknown stored event kind {} at LSN {event_lsn}",
                event.payload.kind
            ))
        })?;

        // Fold the same prior prefix through the canonical authority and
        // command projections before interpreting completion-specific facts.
        // CommandIndex applies both explicit transitions and
        // Revocation.command_effects through the shared lifecycle validator.
        self.authority.observe(event)?;
        self.commands.apply(event).map_err(|error| {
            AuthorityError::CorruptLog(format!(
                "spawn-tail command fold failed at LSN {event_lsn}: {error}"
            ))
        })?;

        match kind {
            StoredEventKind::Operation => self.observe_operation(event, &event_domain, event_lsn),
            StoredEventKind::Observation => {
                self.observe_observation(event, &event_domain, event_lsn)
            }
            StoredEventKind::SessionState => {
                self.observe_session_state(event, &event_domain, event_lsn)
            }
            StoredEventKind::AuditRecord => self.observe_audit(event, &event_domain, event_lsn),
            StoredEventKind::DescendantGrant => {
                self.observe_descendant_grant(event, &event_domain, event_lsn)
            }
            StoredEventKind::CommandTransition => {
                self.observe_command_transition(event, &event_domain, event_lsn)
            }
            StoredEventKind::Revocation => self.observe_revocation(event, &event_domain, event_lsn),
            StoredEventKind::ResourceState
            | StoredEventKind::Elicitation
            | StoredEventKind::Grant
            | StoredEventKind::OperatorRecord
            | StoredEventKind::ControlSurfacePrincipal
            | StoredEventKind::OperatorSessionRevocation
            | StoredEventKind::ControlSurfaceRevocation
            | StoredEventKind::SecurityLockdown => Ok(()),
            StoredEventKind::Unspecified => Err(AuthorityError::CorruptLog(format!(
                "spawn-tail event at LSN {event_lsn} has unspecified kind"
            ))),
        }
    }

    /// Return the deterministic next missing durable action.
    ///
    /// Across ready spawns the successful-completion evidence LSN (or legacy
    /// completed-transition LSN during repair) orders work, followed by the
    /// exact command-id UTF-8 value.
    pub fn next_action(&self) -> Result<Option<SpawnCompletionAction>, AuthorityError> {
        let mut ready = Vec::new();
        for ((domain, command_id), progress) in &self.spawns {
            let Some(accepted) = progress.accepted.as_ref() else {
                continue;
            };
            let Some(session) = progress.session.as_ref() else {
                continue;
            };
            let record = self.commands.get_command(command_id).ok_or_else(|| {
                AuthorityError::CorruptLog(format!(
                    "spawn completion lost accepted command {command_id:?}"
                ))
            })?;
            if matches!(
                record.state,
                OperationState::Accepted
                    | OperationState::Rejected
                    | OperationState::Failed
                    | OperationState::Expired
                    | OperationState::Cancelled
                    | OperationState::Superseded
                    | OperationState::Unspecified
            ) {
                // Accepted alone is not completion authority. Any non-completed
                // terminal—including Revocation.command_effects—wins over a
                // staged audit or result and suppresses all later actions.
                continue;
            }

            let source_event_id = if let Some(audit) = progress.audit.as_ref() {
                // The durable audit fixes whether this was a normal completion
                // sourced by successful Result evidence or a historical repair
                // sourced by the already-completed transition.
                audit.record.source_event_id.clone().ok_or_else(|| {
                    AuthorityError::CorruptRecord(format!(
                        "spawn-completion audit for {command_id:?} has no source_event_id"
                    ))
                })?
            } else {
                match record.state {
                    OperationState::Delivered | OperationState::Running => {
                        let Some(result) = progress.successful_result.as_ref() else {
                            continue;
                        };
                        result.event_id.clone()
                    }
                    OperationState::Completed => {
                        let Some((terminal_lsn, terminal)) = progress.terminal.as_ref() else {
                            return Err(AuthorityError::CorruptLog(format!(
                                "completed spawn {command_id:?} has no terminal source fact"
                            )));
                        };
                        if record.terminal_lsn != Some(*terminal_lsn)
                            || terminal.state != OperationState::Completed
                        {
                            return Err(AuthorityError::CorruptLog(format!(
                                "completed spawn {command_id:?} has inconsistent terminal facts"
                            )));
                        }
                        terminal.event_id.clone()
                    }
                    OperationState::Rejected
                    | OperationState::Failed
                    | OperationState::Expired
                    | OperationState::Cancelled
                    | OperationState::Superseded => {
                        // A transition or Revocation.command_effect at the
                        // earlier durable LSN won. Never publish or issue
                        // descendant authority from success evidence that lost.
                        continue;
                    }
                    OperationState::Accepted | OperationState::Unspecified => continue,
                }
            };
            let source_lsn = required_event_lsn(&source_event_id, "spawn completion source")?;
            if source_lsn <= accepted.accepted_lsn {
                return Err(AuthorityError::CorruptLog(format!(
                    "spawn completion source for {command_id:?} precedes accepted authority"
                )));
            }

            let action = match progress.audit.as_ref() {
                None => SpawnCompletionAction::RecordAudit(SpawnCompletionAudit {
                    authority_domain_id: domain.clone(),
                    spawn_operation_id: command_id.clone(),
                    completion_source_event_id: source_event_id,
                    spawning_grant_id: accepted.spawning_grant_id.clone(),
                    subject_actor_id: accepted.subject_actor_id.clone(),
                    subject_endpoint_id: accepted.subject_endpoint_id.clone(),
                    subject_device_id: accepted.subject_device_id.clone(),
                    spawned_session_scope: session.target_scope.clone(),
                }),
                Some(audit) => {
                    validate_completion_audit(
                        audit,
                        domain,
                        command_id,
                        accepted,
                        session,
                        &source_event_id,
                    )?;
                    if let Some(descendant_grant) = progress.descendant_grant.as_ref() {
                        validate_observed_descendant_grant(
                            descendant_grant,
                            domain,
                            command_id,
                            accepted,
                            session,
                            audit,
                        )?;
                        if record.state == OperationState::Completed {
                            // Historical completed history is repaired by audit
                            // and grant only; never append a second terminal.
                            continue;
                        }
                        let mut correlations = accepted.correlations.clone();
                        if let Some(result) = progress.successful_result.as_ref() {
                            merge_correlations(&mut correlations, &result.correlations);
                        }
                        SpawnCompletionAction::CommitCompleted(SpawnCompletionCommit {
                            spawn_operation_id: command_id.clone(),
                            from_state: record.state,
                            correlations,
                        })
                    } else {
                        let audit_id = audit.record.audit_event_id.clone().ok_or_else(|| {
                            AuthorityError::CorruptRecord(format!(
                                "spawn-completion audit for {command_id:?} has no audit_event_id"
                            ))
                        })?;
                        let created_at = audit.record.occurred_at.ok_or_else(|| {
                            AuthorityError::CorruptRecord(format!(
                                "spawn-completion audit for {command_id:?} has no occurred_at"
                            ))
                        })?;
                        SpawnCompletionAction::IssueDescendantGrant(DescendantGrantIssuance {
                            spawn_operation_id: command_id.clone(),
                            spawning_grant_id: accepted.spawning_grant_id.clone(),
                            spawned_session_scope: session.target_scope.clone(),
                            subject_actor_id: accepted.subject_actor_id.clone(),
                            subject_endpoint_id: accepted.subject_endpoint_id.clone(),
                            authority_domain_id: domain.clone(),
                            allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS.to_vec(),
                            descendant_grant_id: descendant_grant_id(domain, command_id),
                            created_at,
                            audit_id,
                        })
                    }
                }
            };
            ready.push((source_lsn, command_id.value.clone(), action));
        }

        ready
            .sort_by(|left, right| (left.0, left.1.as_bytes()).cmp(&(right.0, right.1.as_bytes())));
        Ok(ready.into_iter().next().map(|(_, _, action)| action))
    }

    /// Return the issuance action for one exact spawn without letting an
    /// unrelated earlier ready spawn hide its validation result. The live
    /// driver still uses [`Self::next_action`] for deterministic global order;
    /// descendant ingress uses this scoped view to validate its own candidate.
    pub(crate) fn descendant_issuance_for(
        &self,
        authority_domain_id: &AuthorityDomainId,
        command_id: &CommandId,
    ) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
        let key = (authority_domain_id.clone(), command_id.clone());
        if !self.spawns.contains_key(&key) {
            return Ok(None);
        }
        let mut scoped = self.clone();
        scoped.spawns.retain(|candidate, _| candidate == &key);
        if let Some(SpawnCompletionAction::IssueDescendantGrant(issuance)) = scoped.next_action()? {
            return Ok(Some(issuance));
        }

        // An exact writer retry reaches this path after the descendant source
        // already committed. `next_action` has validated that source against
        // the durable spawn context and now returns CommitCompleted (or no
        // action for an already-completed historical prefix). Reconstruct the
        // same canonical issuance from prior facts so ingress can compare the
        // retry through the immutable storage identity transaction instead of
        // rejecting it before idempotency is consulted.
        let progress = scoped
            .spawns
            .get(&key)
            .expect("scoped spawn remains present");
        if progress.descendant_grant.is_none() {
            return Ok(None);
        }
        let record = scoped.commands.get_command(command_id).ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn completion lost accepted command {command_id:?}"
            ))
        })?;
        if !matches!(
            record.state,
            OperationState::Delivered | OperationState::Running | OperationState::Completed
        ) {
            return Ok(None);
        }
        let Some(accepted) = progress.accepted.as_ref() else {
            return Ok(None);
        };
        let Some(session) = progress.session.as_ref() else {
            return Ok(None);
        };
        let Some(audit) = progress.audit.as_ref() else {
            return Ok(None);
        };
        let source_event_id = audit.record.source_event_id.as_ref().ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "spawn-completion audit for {command_id:?} has no source_event_id"
            ))
        })?;
        validate_completion_audit(
            audit,
            authority_domain_id,
            command_id,
            accepted,
            session,
            source_event_id,
        )?;
        let audit_id = audit.record.audit_event_id.clone().ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "spawn-completion audit for {command_id:?} has no audit_event_id"
            ))
        })?;
        let created_at = audit.record.occurred_at.ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "spawn-completion audit for {command_id:?} has no occurred_at"
            ))
        })?;
        Ok(Some(DescendantGrantIssuance {
            spawn_operation_id: command_id.clone(),
            spawning_grant_id: accepted.spawning_grant_id.clone(),
            spawned_session_scope: session.target_scope.clone(),
            subject_actor_id: accepted.subject_actor_id.clone(),
            subject_endpoint_id: accepted.subject_endpoint_id.clone(),
            authority_domain_id: authority_domain_id.clone(),
            allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS.to_vec(),
            descendant_grant_id: descendant_grant_id(authority_domain_id, command_id),
            created_at,
            audit_id,
        }))
    }

    fn observe_operation(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<(), AuthorityError> {
        let accepted =
            AcceptedOperation::decode(event.payload.payload.as_slice()).map_err(|error| {
                AuthorityError::CorruptRecord(format!(
                    "cannot decode accepted operation at LSN {event_lsn}: {error}"
                ))
            })?;
        let operation = accepted.operation.ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "accepted operation at LSN {event_lsn} has no operation"
            ))
        })?;
        validate_message_domain(
            operation.authority_domain_id.as_ref(),
            event_domain,
            "operation",
            event_lsn,
        )?;
        let kind = OperationKind::try_from(operation.kind).map_err(|_| {
            AuthorityError::CorruptRecord(format!(
                "operation at LSN {event_lsn} has unknown kind {}",
                operation.kind
            ))
        })?;
        if kind != OperationKind::Spawn {
            return Ok(());
        }

        let command_id = required_command_id(operation.command_id, "spawn operation", event_lsn)?;
        let sender = operation.sender.ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "spawn operation at LSN {event_lsn} has no verified sender"
            ))
        })?;
        let subject_actor_id = required_actor(sender.actor_id, "spawn sender", event_lsn)?;
        validate_optional_endpoint(sender.endpoint_id.as_ref(), "spawn sender", event_lsn)?;
        validate_optional_device(sender.device_id.as_ref(), "spawn sender", event_lsn)?;
        let spawning_grant_id =
            required_grant_id(accepted.authorizing_grant_id, "accepted spawn", event_lsn)?;
        let target_scope = operation.target_scope.ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "accepted spawn at LSN {event_lsn} has no target_scope"
            ))
        })?;
        let parent_grant = self
            .authority
            .get_grant(&spawning_grant_id)
            .ok_or_else(|| {
                AuthorityError::CorruptLog(format!(
                    "accepted spawn at LSN {event_lsn} references missing prior grant {spawning_grant_id:?}"
                ))
            })?;
        let issuer = IssuerRef {
            actor: &subject_actor_id,
            endpoint: sender.endpoint_id.as_ref(),
            authority_domain_id: event_domain,
        };
        if parent_grant.is_revoked()
            || !grant_matches_request(parent_grant, &issuer, OperationKind::Spawn, &target_scope)
        {
            return Err(AuthorityError::CorruptLog(format!(
                "accepted spawn at LSN {event_lsn} is not anchored to its exact authorizing grant"
            )));
        }
        let incoming = AcceptedSpawn {
            accepted_lsn: event_lsn,
            target_scope,
            spawning_grant_id,
            subject_actor_id,
            subject_endpoint_id: sender.endpoint_id,
            subject_device_id: sender.device_id,
            correlations: operation.correlations,
        };
        let key = (event_domain.clone(), command_id);
        let progress = self.spawns.entry(key.clone()).or_default();
        insert_consistent_option(
            &mut progress.accepted,
            incoming,
            "accepted spawn",
            &key,
            event_lsn,
        )?;
        validate_known_target(progress, &key)
    }

    fn observe_observation(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<(), AuthorityError> {
        let observation =
            Observation::decode(event.payload.payload.as_slice()).map_err(|error| {
                AuthorityError::CorruptRecord(format!(
                    "cannot decode observation at LSN {event_lsn}: {error}"
                ))
            })?;
        validate_message_domain(
            observation.authority_domain_id.as_ref(),
            event_domain,
            "observation",
            event_lsn,
        )?;
        let kind = ObservationKind::try_from(observation.kind).map_err(|_| {
            AuthorityError::CorruptRecord(format!(
                "observation at LSN {event_lsn} has unknown kind {}",
                observation.kind
            ))
        })?;
        if kind != ObservationKind::Result {
            return Ok(());
        }
        let failure = FailureCode::try_from(observation.failure_code).map_err(|_| {
            AuthorityError::CorruptRecord(format!(
                "result observation at LSN {event_lsn} has unknown failure code {}",
                observation.failure_code
            ))
        })?;
        if failure != FailureCode::Unspecified {
            return Ok(());
        }
        let Some(command_id) = exact_command_correlation(&observation.correlations) else {
            // A successful result for another command shape is not spawn
            // completion evidence. The shared qualifier accepts identical
            // duplicate command references but keeps empty/conflicting ids
            // inert so unrelated durable Observations cannot arm authority.
            return Ok(());
        };
        let Some(record) = self.commands.get_command(&command_id) else {
            // Pre-acceptance evidence is durable but inert for authority. It
            // cannot arm a later command that reuses the correlation value.
            return Ok(());
        };
        if record.operation.kind != OperationKind::Spawn as i32
            || !matches!(
                record.state,
                OperationState::Delivered | OperationState::Running
            )
        {
            return Ok(());
        }
        let key = (event_domain.clone(), command_id);
        let Some(progress) = self.spawns.get_mut(&key) else {
            return Ok(());
        };
        let accepted = progress.accepted.as_ref().ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "successful result at LSN {event_lsn} has no accepted spawn context"
            ))
        })?;
        let target_scope = observation.target_scope.ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "successful result observation at LSN {event_lsn} has no target_scope"
            ))
        })?;
        if target_scope != accepted.target_scope {
            return Err(AuthorityError::CorruptLog(format!(
                "successful result target conflicts with accepted spawn for key {key:?}"
            )));
        }
        let incoming = CompletionEvidence {
            event_id: event.event_id.clone(),
            target_scope,
            correlations: observation.correlations,
        };
        match progress.successful_result.as_ref() {
            Some(existing) if existing.target_scope != incoming.target_scope => {
                return Err(AuthorityError::CorruptLog(format!(
                    "successful result has conflicting targets for key {key:?} at LSN {event_lsn}"
                )));
            }
            Some(existing)
                if required_event_lsn(&existing.event_id, "successful result")? <= event_lsn => {}
            _ => progress.successful_result = Some(incoming),
        }
        validate_known_target(progress, &key)
    }

    fn observe_session_state(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<(), AuthorityError> {
        let state_event =
            SessionStateEvent::decode(event.payload.payload.as_slice()).map_err(|error| {
                AuthorityError::CorruptRecord(format!(
                    "cannot decode session-state event at LSN {event_lsn}: {error}"
                ))
            })?;
        validate_message_domain(
            state_event.authority_domain_id.as_ref(),
            event_domain,
            "session-state event",
            event_lsn,
        )?;

        let correlated = match state_event.mutation {
            Some(session_state_event::Mutation::Registered(registered)) => {
                session_from_registration(registered, event_lsn)?
            }
            Some(session_state_event::Mutation::GenerationBumped(bumped)) => {
                session_from_generation_bump(bumped, event_lsn)?
            }
            _ => None,
        };
        let Some((spawn_command_id, target_scope)) = correlated else {
            return Ok(());
        };
        let key = (event_domain.clone(), spawn_command_id);
        let Some(progress) = self.spawns.get_mut(&key) else {
            // A correlation cannot pre-seed authority before the accepted
            // spawn appears in the durable LSN prefix.
            return Ok(());
        };
        let accepted = progress.accepted.as_ref().ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn-correlated session fact at LSN {event_lsn} has no accepted spawn"
            ))
        })?;
        if event_lsn <= accepted.accepted_lsn
            || !spawn_scope_contains_session(&accepted.target_scope, &target_scope)
        {
            return Err(AuthorityError::CorruptLog(format!(
                "spawn-correlated session target is outside accepted spawn scope for key {key:?} at LSN {event_lsn}"
            )));
        }
        insert_consistent_option(
            &mut progress.session,
            SessionFact {
                event_lsn,
                target_scope,
            },
            "spawn-correlated session fact",
            &key,
            event_lsn,
        )
    }

    fn observe_audit(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<(), AuthorityError> {
        let record = AuditRecord::decode(event.payload.payload.as_slice()).map_err(|error| {
            AuthorityError::CorruptRecord(format!(
                "cannot decode audit record at LSN {event_lsn}: {error}"
            ))
        })?;
        if record.audit_event_id.as_ref() != Some(&event.event_id) {
            return Err(AuthorityError::CorruptLog(format!(
                "audit record identity does not match event at LSN {event_lsn}"
            )));
        }
        if record.reason_code != SPAWN_COMPLETION_REASON {
            return Ok(());
        }
        let command_id = required_command_id(
            record.command_id.clone(),
            "spawn-completion audit",
            event_lsn,
        )?;
        let key = (event_domain.clone(), command_id.clone());
        let progress = self.spawns.get_mut(&key).ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} has no accepted spawn context"
            ))
        })?;
        let accepted = progress.accepted.as_ref().ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} has no accepted spawn"
            ))
        })?;
        let session = progress.session.as_ref().ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} precedes its session fact"
            ))
        })?;
        let command = self.commands.get_command(&command_id).ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} references an unknown command"
            ))
        })?;
        let source_event_id = completion_source_for(progress, command.state)?.ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} precedes valid delivered/running success"
            ))
        })?;
        let fact = CompletionAuditFact {
            event: event.clone(),
            record,
        };
        validate_completion_audit(
            &fact,
            event_domain,
            &command_id,
            accepted,
            session,
            &source_event_id,
        )?;
        insert_consistent_option(
            &mut progress.audit,
            fact,
            "spawn-completion audit",
            &key,
            event_lsn,
        )
    }

    fn observe_descendant_grant(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<(), AuthorityError> {
        let grant = DescendantGrant::decode(event.payload.payload.as_slice()).map_err(|error| {
            AuthorityError::CorruptRecord(format!(
                "cannot decode descendant grant at LSN {event_lsn}: {error}"
            ))
        })?;
        validate_message_domain(
            grant.authority_domain_id.as_ref(),
            event_domain,
            "descendant grant",
            event_lsn,
        )?;
        let provenance = grant.provenance.as_ref().ok_or_else(|| {
            AuthorityError::InvalidGrant(format!(
                "descendant grant at LSN {event_lsn} is missing provenance"
            ))
        })?;
        let command_id = required_command_id(
            provenance.spawn_operation_id.clone(),
            "descendant grant provenance",
            event_lsn,
        )?;
        required_grant_id(
            provenance.spawning_grant_id.clone(),
            "descendant grant provenance",
            event_lsn,
        )?;
        let key = (event_domain.clone(), command_id.clone());
        let progress = self.spawns.get_mut(&key).ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "descendant grant at LSN {event_lsn} has no accepted spawn context"
            ))
        })?;
        let accepted = progress.accepted.as_ref().ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "descendant grant at LSN {event_lsn} has no accepted spawn"
            ))
        })?;
        let session = progress.session.as_ref().ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "descendant grant at LSN {event_lsn} precedes its session fact"
            ))
        })?;
        let audit = progress.audit.as_ref().ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "descendant grant at LSN {event_lsn} precedes its completion audit"
            ))
        })?;
        let command = self.commands.get_command(&command_id).ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "descendant grant at LSN {event_lsn} references an unknown command"
            ))
        })?;
        if !matches!(
            command.state,
            OperationState::Delivered | OperationState::Running | OperationState::Completed
        ) {
            return Err(AuthorityError::CorruptLog(format!(
                "descendant grant at LSN {event_lsn} has no eligible lifecycle"
            )));
        }
        let fact = DescendantGrantFact { event_lsn, grant };
        validate_observed_descendant_grant(
            &fact,
            event_domain,
            &command_id,
            accepted,
            session,
            audit,
        )?;
        insert_consistent_option(
            &mut progress.descendant_grant,
            fact,
            "descendant grant",
            &key,
            event_lsn,
        )
    }

    fn observe_command_transition(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<(), AuthorityError> {
        let transition =
            CommandTransition::decode(event.payload.payload.as_slice()).map_err(|error| {
                AuthorityError::CorruptRecord(format!(
                    "cannot decode command transition at LSN {event_lsn}: {error}"
                ))
            })?;
        let command_id =
            required_command_id(transition.command_id, "command transition", event_lsn)?;
        let to_state = OperationState::try_from(transition.to_state).map_err(|_| {
            AuthorityError::CorruptLog(format!(
                "command transition at LSN {event_lsn} has unknown to_state {}",
                transition.to_state
            ))
        })?;
        if to_state == OperationState::Unspecified {
            return Err(AuthorityError::CorruptLog(format!(
                "command transition at LSN {event_lsn} has unspecified to_state"
            )));
        }
        let key = (event_domain.clone(), command_id.clone());
        let Some(progress) = self.spawns.get_mut(&key) else {
            return Ok(());
        };
        if is_terminal(to_state) {
            let record = self.commands.get_command(&command_id).ok_or_else(|| {
                AuthorityError::CorruptLog(format!(
                    "terminal transition at LSN {event_lsn} references an unknown command"
                ))
            })?;
            if record.terminal_lsn == Some(event_lsn) {
                insert_terminal_fact(
                    progress,
                    event_lsn,
                    TerminalFact {
                        event_id: event.event_id.clone(),
                        state: record.state,
                    },
                )?;
            }
        }
        Ok(())
    }

    fn observe_revocation(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<(), AuthorityError> {
        let revocation = Revocation::decode(event.payload.payload.as_slice()).map_err(|error| {
            AuthorityError::CorruptRecord(format!(
                "cannot decode revocation at LSN {event_lsn}: {error}"
            ))
        })?;
        validate_message_domain(
            revocation.authority_domain_id.as_ref(),
            event_domain,
            "revocation",
            event_lsn,
        )?;
        for effect in revocation.command_effects {
            let command_id =
                required_command_id(effect.command_id, "revocation command effect", event_lsn)?;
            let key = (event_domain.clone(), command_id.clone());
            let Some(progress) = self.spawns.get_mut(&key) else {
                continue;
            };
            let record = self.commands.get_command(&command_id).ok_or_else(|| {
                AuthorityError::CorruptLog(format!(
                    "revocation effect at LSN {event_lsn} references an unknown command"
                ))
            })?;
            if record.terminal_lsn == Some(event_lsn) {
                insert_terminal_fact(
                    progress,
                    event_lsn,
                    TerminalFact {
                        event_id: event.event_id.clone(),
                        state: record.state,
                    },
                )?;
            }
        }
        Ok(())
    }

    fn bind_domain(&mut self, event_domain: &AuthorityDomainId) -> Result<(), AuthorityError> {
        if let Some(bound_domain) = &self.authority_domain_id {
            if bound_domain != event_domain {
                return Err(AuthorityError::CorruptLog(format!(
                    "spawn tail is bound to authority domain {:?}, but observed event for {:?}",
                    bound_domain, event_domain
                )));
            }
        } else {
            self.authority_domain_id = Some(event_domain.clone());
        }
        Ok(())
    }
}

fn session_from_registration(
    registered: SessionRegistered,
    event_lsn: u64,
) -> Result<Option<(CommandId, TargetScope)>, AuthorityError> {
    let Some(spawn_command_id) = spawn_command_id(registered.spawn_origin, event_lsn)? else {
        return Ok(None);
    };
    Ok(Some((
        spawn_command_id,
        runtime_session_scope(
            registered.adapter_id,
            registered.deployment_scope,
            registered.runtime_session_id,
            registered.session_generation,
            "session registration",
            event_lsn,
        )?,
    )))
}

fn session_from_generation_bump(
    bumped: SessionGenerationBumped,
    event_lsn: u64,
) -> Result<Option<(CommandId, TargetScope)>, AuthorityError> {
    let Some(spawn_command_id) = spawn_command_id(bumped.spawn_origin, event_lsn)? else {
        return Ok(None);
    };
    Ok(Some((
        spawn_command_id,
        runtime_session_scope(
            bumped.adapter_id,
            bumped.deployment_scope,
            bumped.runtime_session_id,
            bumped.to_generation,
            "session generation bump",
            event_lsn,
        )?,
    )))
}

fn spawn_command_id(
    origin: Option<TypedCorrelation>,
    event_lsn: u64,
) -> Result<Option<CommandId>, AuthorityError> {
    let Some(origin) = origin else {
        return Ok(None);
    };
    match origin.r#ref {
        Some(typed_correlation::Ref::CommandId(command_id)) if !command_id.value.is_empty() => {
            Ok(Some(command_id))
        }
        Some(typed_correlation::Ref::CommandId(_)) => Err(AuthorityError::CorruptRecord(format!(
            "session spawn_origin at LSN {event_lsn} has an empty command id"
        ))),
        Some(_) => Err(AuthorityError::CorruptRecord(format!(
            "session spawn_origin at LSN {event_lsn} is not a command correlation"
        ))),
        None => Err(AuthorityError::CorruptRecord(format!(
            "session spawn_origin at LSN {event_lsn} has no typed reference"
        ))),
    }
}

fn runtime_session_scope(
    adapter_id: Option<patchbay_contracts::patchbay::AdapterId>,
    deployment_scope: String,
    runtime_session_id: Option<patchbay_contracts::patchbay::RuntimeSessionId>,
    generation: Option<patchbay_contracts::patchbay::Generation>,
    record_name: &str,
    event_lsn: u64,
) -> Result<TargetScope, AuthorityError> {
    let adapter_id = adapter_id
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "{record_name} at LSN {event_lsn} is missing adapter_id"
            ))
        })?;
    if deployment_scope.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} has an empty deployment_scope"
        )));
    }
    let runtime_session_id = runtime_session_id
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "{record_name} at LSN {event_lsn} is missing runtime_session_id"
            ))
        })?;
    let generation = generation.ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} is missing session generation"
        ))
    })?;
    Ok(TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(adapter_id),
        deployment_scope,
        runtime_session_id: Some(runtime_session_id),
        session_generation: Some(generation),
        ..TargetScope::default()
    })
}

fn completion_source_for(
    progress: &SpawnProgress,
    state: OperationState,
) -> Result<Option<EventId>, AuthorityError> {
    match state {
        OperationState::Delivered | OperationState::Running => Ok(progress
            .successful_result
            .as_ref()
            .map(|result| result.event_id.clone())),
        OperationState::Completed => {
            let Some((_, terminal)) = progress.terminal.as_ref() else {
                return Err(AuthorityError::CorruptLog(
                    "completed spawn has no terminal completion source".to_owned(),
                ));
            };
            if terminal.state != OperationState::Completed {
                return Err(AuthorityError::CorruptLog(
                    "completed spawn has a non-completed terminal source".to_owned(),
                ));
            }
            Ok(Some(terminal.event_id.clone()))
        }
        _ => Ok(None),
    }
}

fn spawn_scope_contains_session(spawn_scope: &TargetScope, session: &TargetScope) -> bool {
    if TargetScopeKind::try_from(session.kind).ok() != Some(TargetScopeKind::RuntimeSession) {
        return false;
    }
    match TargetScopeKind::try_from(spawn_scope.kind).ok() {
        Some(TargetScopeKind::FleetSupervisor | TargetScopeKind::AuthorityDomain) => true,
        Some(TargetScopeKind::Adapter) => matches!(
            (spawn_scope.adapter_id.as_ref(), session.adapter_id.as_ref()),
            (Some(spawn_adapter), Some(session_adapter))
                if !spawn_adapter.value.is_empty() && spawn_adapter == session_adapter
        ),
        _ => false,
    }
}

fn insert_terminal_fact(
    progress: &mut SpawnProgress,
    event_lsn: u64,
    incoming: TerminalFact,
) -> Result<(), AuthorityError> {
    match &progress.terminal {
        Some((existing_lsn, _)) if *existing_lsn < event_lsn => Ok(()),
        Some((existing_lsn, existing)) if *existing_lsn == event_lsn => {
            if existing == &incoming {
                Ok(())
            } else {
                Err(AuthorityError::CorruptLog(format!(
                    "conflicting terminal lifecycle fact at LSN {event_lsn}"
                )))
            }
        }
        _ => {
            progress.terminal = Some((event_lsn, incoming));
            Ok(())
        }
    }
}

fn validate_known_target(progress: &SpawnProgress, key: &SpawnKey) -> Result<(), AuthorityError> {
    if let (Some(accepted), Some(result)) = (&progress.accepted, &progress.successful_result) {
        if accepted.target_scope != result.target_scope {
            return Err(AuthorityError::CorruptLog(format!(
                "successful result target conflicts with accepted spawn for key {key:?}"
            )));
        }
    }
    Ok(())
}

fn validate_completion_audit(
    fact: &CompletionAuditFact,
    domain: &AuthorityDomainId,
    command_id: &CommandId,
    accepted: &AcceptedSpawn,
    session: &SessionFact,
    source_event_id: &EventId,
) -> Result<(), AuthorityError> {
    let record = &fact.record;
    let audit_lsn = required_event_lsn(&fact.event.event_id, "spawn-completion audit")?;
    let source_lsn = required_event_lsn(source_event_id, "spawn completion source")?;
    if source_lsn <= accepted.accepted_lsn
        || source_lsn >= audit_lsn
        || session.event_lsn >= audit_lsn
    {
        return Err(AuthorityError::CorruptLog(format!(
            "spawn-completion audit for {command_id:?} does not follow accepted lifecycle, source, and session facts"
        )));
    }
    if record.audit_event_id.as_ref() != Some(&fact.event.event_id)
        || record.kind != AuditEventKind::CommandCompleted as i32
        || record.reason_code != SPAWN_COMPLETION_REASON
        || record.command_id.as_ref() != Some(command_id)
        || record.actor_id.as_ref() != Some(&accepted.subject_actor_id)
        || record.endpoint_id != accepted.subject_endpoint_id
        || record.device_id != accepted.subject_device_id
        || record.grant_id.as_ref() != Some(&accepted.spawning_grant_id)
        || record.target_scope.as_ref() != Some(&session.target_scope)
        || record.source_event_id.as_ref() != Some(source_event_id)
        || record.failure_code != FailureCode::Unspecified as i32
        || record.occurred_at.is_none()
    {
        return Err(AuthorityError::CorruptLog(format!(
            "spawn-completion audit does not match verified spawn {:?}",
            command_id
        )));
    }
    validate_message_domain(
        record
            .audit_event_id
            .as_ref()
            .and_then(|id| id.authority_domain_id.as_ref()),
        domain,
        "spawn-completion audit",
        audit_lsn,
    )?;
    Ok(())
}

fn validate_observed_descendant_grant(
    fact: &DescendantGrantFact,
    domain: &AuthorityDomainId,
    command_id: &CommandId,
    accepted: &AcceptedSpawn,
    session: &SessionFact,
    audit: &CompletionAuditFact,
) -> Result<(), AuthorityError> {
    let grant = &fact.grant;
    let audit_lsn = required_event_lsn(&audit.event.event_id, "spawn-completion audit")?;
    if fact.event_lsn <= audit_lsn {
        return Err(AuthorityError::CorruptLog(format!(
            "descendant grant for {command_id:?} does not follow its completion audit"
        )));
    }
    let expected_id = descendant_grant_id(domain, command_id);
    let provenance = grant.provenance.as_ref().ok_or_else(|| {
        AuthorityError::InvalidGrant(format!(
            "descendant grant {:?} is missing provenance",
            grant.grant_id
        ))
    })?;
    let actual_kinds: HashSet<_> = grant.allowed_operation_kinds.iter().copied().collect();
    let expected_kinds: HashSet<_> = DESCENDANT_GRANT_ALLOWED_KINDS
        .iter()
        .map(|kind| *kind as i32)
        .collect();
    if grant.grant_id.as_ref() != Some(&expected_id)
        || grant.authority_domain_id.as_ref() != Some(domain)
        || grant.subject_actor_id.as_ref() != Some(&accepted.subject_actor_id)
        || grant.subject_endpoint_id != accepted.subject_endpoint_id
        || !grant.subject_endpoint_class.is_empty()
        || grant.target_scope.as_ref() != Some(&session.target_scope)
        || grant.allowed_operation_kinds.len() != DESCENDANT_GRANT_ALLOWED_KINDS.len()
        || actual_kinds != expected_kinds
        || provenance.spawn_operation_id.as_ref() != Some(command_id)
        || provenance.spawning_grant_id.as_ref() != Some(&accepted.spawning_grant_id)
        || grant.created_at != audit.record.occurred_at
        || grant.expires_at.is_some()
        || grant.revocation_generation.is_some()
        || grant.revoked_at.is_some()
        || grant.revocation_policy != GrantRevocationPolicy::Continue as i32
        || grant.audit_id != audit.record.audit_event_id
    {
        return Err(AuthorityError::CorruptLog(format!(
            "observed descendant grant does not match verified spawn {:?}",
            command_id
        )));
    }
    Ok(())
}

/// Validate the immutable completion audit/source link carried by a descendant
/// grant. Authority replay and the live ingestion boundary share this helper.
pub(crate) fn validate_descendant_audit_link(
    grant: &DescendantGrant,
    audit_event: &RecordedEvent,
    source_event: &RecordedEvent,
) -> Result<(), AuthorityError> {
    let audit_lsn = required_event_lsn(&audit_event.event_id, "descendant audit")?;
    let source_lsn = required_event_lsn(&source_event.event_id, "completion source")?;
    let domain = audit_event
        .event_id
        .authority_domain_id
        .as_ref()
        .ok_or_else(|| AuthorityError::InvalidGrant("descendant audit has no domain".to_owned()))?;
    if source_event.event_id.authority_domain_id.as_ref() != Some(domain) || source_lsn >= audit_lsn
    {
        return Err(AuthorityError::InvalidGrant(
            "descendant completion source must be a prior same-domain event".to_owned(),
        ));
    }
    let audit = AuditRecord::decode(audit_event.payload.payload.as_slice()).map_err(|error| {
        AuthorityError::InvalidGrant(format!(
            "cannot decode descendant completion audit: {error}"
        ))
    })?;
    let provenance = grant.provenance.as_ref().ok_or_else(|| {
        AuthorityError::InvalidGrant("descendant grant is missing provenance".to_owned())
    })?;
    let command_id = provenance.spawn_operation_id.as_ref().ok_or_else(|| {
        AuthorityError::InvalidGrant(
            "descendant provenance is missing spawn_operation_id".to_owned(),
        )
    })?;
    let spawning_grant_id = provenance.spawning_grant_id.as_ref().ok_or_else(|| {
        AuthorityError::InvalidGrant(
            "descendant provenance is missing spawning_grant_id".to_owned(),
        )
    })?;
    if command_id.value.is_empty() || spawning_grant_id.value.is_empty() {
        return Err(AuthorityError::InvalidGrant(
            "descendant provenance identifiers must be non-empty".to_owned(),
        ));
    }
    if audit_event.payload.kind != StoredEventKind::AuditRecord as i32
        || audit.audit_event_id.as_ref() != Some(&audit_event.event_id)
        || grant.audit_id.as_ref() != Some(&audit_event.event_id)
        || audit.kind != AuditEventKind::CommandCompleted as i32
        || audit.reason_code != SPAWN_COMPLETION_REASON
        || audit.command_id.as_ref() != Some(command_id)
        || audit.grant_id.as_ref() != Some(spawning_grant_id)
        || audit.actor_id != grant.subject_actor_id
        || audit.endpoint_id != grant.subject_endpoint_id
        || audit.target_scope != grant.target_scope
        || audit.source_event_id.as_ref() != Some(&source_event.event_id)
        || audit.failure_code != FailureCode::Unspecified as i32
        || audit.occurred_at.is_none()
        || audit.occurred_at != grant.created_at
    {
        return Err(AuthorityError::InvalidGrant(
            "descendant grant does not match its spawn-completion audit".to_owned(),
        ));
    }

    match StoredEventKind::try_from(source_event.payload.kind).ok() {
        Some(StoredEventKind::Observation) => {
            let observation = Observation::decode(source_event.payload.payload.as_slice())
                .map_err(|error| {
                    AuthorityError::InvalidGrant(format!(
                        "cannot decode descendant completion observation: {error}"
                    ))
                })?;
            let source_command = exact_command_correlation(&observation.correlations).ok_or_else(
                || {
                    AuthorityError::CorruptRecord(format!(
                        "descendant completion observation at LSN {source_lsn} must have one exact non-empty command correlation"
                    ))
                },
            )?;
            if observation.kind != ObservationKind::Result as i32
                || observation.failure_code != FailureCode::Unspecified as i32
                || &source_command != command_id
            {
                return Err(AuthorityError::InvalidGrant(
                    "descendant audit source is not a matching successful result".to_owned(),
                ));
            }
        }
        Some(StoredEventKind::CommandTransition) => {
            let transition = CommandTransition::decode(source_event.payload.payload.as_slice())
                .map_err(|error| {
                    AuthorityError::InvalidGrant(format!(
                        "cannot decode descendant completion transition: {error}"
                    ))
                })?;
            if transition.command_id.as_ref() != Some(command_id)
                || transition.to_state != OperationState::Completed as i32
                || transition.failure_code != FailureCode::Unspecified as i32
            {
                return Err(AuthorityError::InvalidGrant(
                    "descendant audit source is not a matching completed transition".to_owned(),
                ));
            }
        }
        _ => {
            return Err(AuthorityError::InvalidGrant(
                "descendant audit source has the wrong durable event kind".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Validate an ingress candidate against the exact action derived from the
/// complete durable prefix. This is the final defense against a self-consistent
/// forged source/audit/grant chain that lacks accepted authority or lifecycle.
pub(crate) fn validate_descendant_issuance_candidate(
    grant: &DescendantGrant,
    issuance: &DescendantGrantIssuance,
) -> Result<(), AuthorityError> {
    let provenance = grant.provenance.as_ref().ok_or_else(|| {
        AuthorityError::InvalidGrant("descendant grant is missing provenance".to_owned())
    })?;
    let actual_kinds: HashSet<_> = grant.allowed_operation_kinds.iter().copied().collect();
    let expected_kinds: HashSet<_> = issuance
        .allowed_operation_kinds
        .iter()
        .map(|kind| *kind as i32)
        .collect();
    if grant.grant_id.as_ref() != Some(&issuance.descendant_grant_id)
        || grant.authority_domain_id.as_ref() != Some(&issuance.authority_domain_id)
        || grant.subject_actor_id.as_ref() != Some(&issuance.subject_actor_id)
        || grant.subject_endpoint_id != issuance.subject_endpoint_id
        || !grant.subject_endpoint_class.is_empty()
        || grant.target_scope.as_ref() != Some(&issuance.spawned_session_scope)
        || grant.allowed_operation_kinds.len() != issuance.allowed_operation_kinds.len()
        || actual_kinds != expected_kinds
        || provenance.spawn_operation_id.as_ref() != Some(&issuance.spawn_operation_id)
        || provenance.spawning_grant_id.as_ref() != Some(&issuance.spawning_grant_id)
        || grant.created_at.as_ref() != Some(&issuance.created_at)
        || grant.expires_at.is_some()
        || grant.revocation_generation.is_some()
        || grant.revoked_at.is_some()
        || grant.revocation_policy != GrantRevocationPolicy::Continue as i32
        || grant.audit_id.as_ref() != Some(&issuance.audit_id)
    {
        return Err(AuthorityError::InvalidGrant(
            "descendant grant does not match the full durable spawn completion context".to_owned(),
        ));
    }
    Ok(())
}

/// Canonical deterministic descendant-grant id, namespaced from operator grants.
pub(crate) fn descendant_grant_id(domain: &AuthorityDomainId, spawn_op: &CommandId) -> GrantId {
    GrantId {
        value: format!("desc:{}:{}", domain.value, spawn_op.value),
    }
}

fn is_terminal(state: OperationState) -> bool {
    matches!(
        state,
        OperationState::Completed
            | OperationState::Rejected
            | OperationState::Failed
            | OperationState::Expired
            | OperationState::Cancelled
            | OperationState::Superseded
    )
}

fn merge_correlations(target: &mut Vec<TypedCorrelation>, incoming: &[TypedCorrelation]) {
    for correlation in incoming {
        if !target.iter().any(|existing| existing == correlation) {
            target.push(correlation.clone());
        }
    }
}

fn insert_consistent_option<K: std::fmt::Debug, V: PartialEq>(
    slot: &mut Option<V>,
    incoming: V,
    record_name: &str,
    key: &K,
    event_lsn: u64,
) -> Result<(), AuthorityError> {
    if let Some(existing) = slot {
        if existing == &incoming {
            return Ok(());
        }
        return Err(AuthorityError::CorruptLog(format!(
            "{record_name} has conflicting records for key {key:?} at LSN {event_lsn}"
        )));
    }
    *slot = Some(incoming);
    Ok(())
}

fn event_identity<'a>(
    event: &'a RecordedEvent,
    record_name: &str,
) -> Result<(&'a AuthorityDomainId, u64), AuthorityError> {
    let domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        AuthorityError::CorruptRecord(format!("{record_name} has no authority domain"))
    })?;
    if domain.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} has an empty authority domain"
        )));
    }
    let lsn = required_event_lsn(&event.event_id, record_name)?;
    Ok((domain, lsn))
}

fn required_event_lsn(event_id: &EventId, record_name: &str) -> Result<u64, AuthorityError> {
    let lsn = event_id
        .lsn
        .as_ref()
        .ok_or_else(|| AuthorityError::CorruptRecord(format!("{record_name} has no LSN")))?;
    if lsn.value == 0 {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} has zero LSN"
        )));
    }
    Ok(lsn.value)
}

fn validate_message_domain(
    message_domain: Option<&AuthorityDomainId>,
    event_domain: &AuthorityDomainId,
    record_name: &str,
    event_lsn: u64,
) -> Result<(), AuthorityError> {
    let message_domain = message_domain.ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} is missing authority_domain_id"
        ))
    })?;
    if message_domain.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} has an empty authority_domain_id"
        )));
    }
    if message_domain != event_domain {
        return Err(AuthorityError::CorruptLog(format!(
            "{record_name} authority domain {:?} does not match event authority domain {:?} at LSN {event_lsn}",
            message_domain, event_domain
        )));
    }
    Ok(())
}

fn required_command_id(
    command_id: Option<CommandId>,
    record_name: &str,
    event_lsn: u64,
) -> Result<CommandId, AuthorityError> {
    command_id.filter(|id| !id.value.is_empty()).ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} is missing a non-empty command_id"
        ))
    })
}

fn required_grant_id(
    grant_id: Option<GrantId>,
    record_name: &str,
    event_lsn: u64,
) -> Result<GrantId, AuthorityError> {
    grant_id.filter(|id| !id.value.is_empty()).ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} is missing a non-empty grant_id"
        ))
    })
}

fn required_actor(
    actor_id: Option<ActorId>,
    record_name: &str,
    event_lsn: u64,
) -> Result<ActorId, AuthorityError> {
    actor_id.filter(|id| !id.value.is_empty()).ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} is missing a non-empty actor_id"
        ))
    })
}

fn validate_optional_endpoint(
    endpoint: Option<&EndpointId>,
    record_name: &str,
    event_lsn: u64,
) -> Result<(), AuthorityError> {
    if endpoint.is_some_and(|id| id.value.is_empty()) {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} has an empty endpoint_id"
        )));
    }
    Ok(())
}

fn validate_optional_device(
    device: Option<&DeviceId>,
    record_name: &str,
    event_lsn: u64,
) -> Result<(), AuthorityError> {
    if device.is_some_and(|id| id.value.is_empty()) {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} has an empty device_id"
        )));
    }
    Ok(())
}
