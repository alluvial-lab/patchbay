//! Order-independent descendant-grant issuance derived from durable spawn facts.
//!
//! This module is a pure log fold. It does not write grants or own a live
//! consumer loop: callers feed committed events through [`SpawnDescendantTail::observe`]
//! and decide how to persist the returned issuance.

use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, ActorId, AuthorityDomainId, CommandId,
    CommandTransition, EventId, GrantId, Operation, OperationKind, OperationState,
    SessionStateEvent, StoredEventKind, TargetScope, TargetScopeKind,
};
use prost::Message;

use crate::storage::RecordedEvent;

use super::{AuthorityError, DESCENDANT_GRANT_ALLOWED_KINDS};

type SpawnKey = (AuthorityDomainId, CommandId);

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpawnOpInfo {
    spawner_actor: Option<ActorId>,
    // The durable Operation wire shape does not carry the authorizing grant.
    spawning_grant_id: Option<GrantId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistrationInfo {
    spawned_session_scope: TargetScope,
    authority_domain_id: AuthorityDomainId,
}

/// A descendant grant that should be persisted for one completed spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescendantGrantIssuance {
    pub spawn_operation_id: CommandId,
    /// `None` in v0.1.0 because durable Operations do not retain the grant that
    /// authorized acceptance.
    pub spawning_grant_id: Option<GrantId>,
    pub spawned_session_scope: TargetScope,
    pub subject_actor_id: ActorId,
    pub authority_domain_id: AuthorityDomainId,
    pub allowed_operation_kinds: Vec<OperationKind>,
    pub descendant_grant_id: GrantId,
    /// `None` in v0.1.0; the spawn-completion audit producer is deferred.
    pub audit_id: Option<EventId>,
}

/// Pure, order-independent fold joining spawn, completion, and registration.
///
/// A tail instance is bound to the authority domain of its first event. Every
/// later event must belong to that same domain. Issuance occurs exactly once
/// after all three facts have arrived, regardless of their arrival order.
#[derive(Debug, Clone, Default)]
pub struct SpawnDescendantTail {
    authority_domain_id: Option<AuthorityDomainId>,
    spawn_ops: HashMap<SpawnKey, SpawnOpInfo>,
    completed: HashSet<SpawnKey>,
    registrations: HashMap<SpawnKey, RegistrationInfo>,
    issued: HashSet<SpawnKey>,
}

impl SpawnDescendantTail {
    /// Construct an empty spawn-tail fold.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one committed event and return a newly-complete issuance, if any.
    ///
    /// Exact redelivery is idempotent. Conflicting spawn or registration facts
    /// for the same `(authority_domain_id, command_id)` fail as corrupt log
    /// history rather than silently selecting one.
    pub fn observe(
        &mut self,
        event: &RecordedEvent,
    ) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        self.bind_domain(event_domain)?;
        let event_domain = event_domain.clone();

        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            AuthorityError::CorruptRecord(format!(
                "unknown stored event kind {} at LSN {event_lsn}",
                event.payload.kind
            ))
        })?;

        match kind {
            StoredEventKind::Operation => self.observe_operation(event, &event_domain, event_lsn),
            StoredEventKind::CommandTransition => {
                self.observe_command_transition(event, &event_domain, event_lsn)
            }
            StoredEventKind::SessionState => {
                self.observe_session_state(event, &event_domain, event_lsn)
            }
            StoredEventKind::Observation
            | StoredEventKind::Elicitation
            | StoredEventKind::Grant
            | StoredEventKind::DescendantGrant
            | StoredEventKind::Revocation
            | StoredEventKind::OperatorRecord
            | StoredEventKind::ControlSurfacePrincipal
            | StoredEventKind::Unspecified => Ok(None),
        }
    }

    fn observe_operation(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
        let operation = Operation::decode(event.payload.payload.as_slice()).map_err(|error| {
            AuthorityError::CorruptRecord(format!(
                "cannot decode operation at LSN {event_lsn}: {error}"
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
            return Ok(None);
        }

        let command_id = required_command_id(operation.command_id, "spawn operation", event_lsn)?;
        let key = (event_domain.clone(), command_id);
        let incoming = SpawnOpInfo {
            spawner_actor: operation.sender.and_then(|sender| sender.actor_id),
            // The Operation proto has no grant_id; provenance enrichment is a
            // follow-on once durable acceptance metadata exists.
            spawning_grant_id: None,
        };
        insert_consistent(
            &mut self.spawn_ops,
            key.clone(),
            incoming,
            "spawn operation",
            event_lsn,
        )?;
        self.try_issue(&key)
    }

    fn observe_command_transition(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
        let transition =
            CommandTransition::decode(event.payload.payload.as_slice()).map_err(|error| {
                AuthorityError::CorruptRecord(format!(
                    "cannot decode command transition at LSN {event_lsn}: {error}"
                ))
            })?;
        let to_state = OperationState::try_from(transition.to_state).map_err(|_| {
            AuthorityError::CorruptLog(format!(
                "command transition at LSN {event_lsn} has unknown to_state {}",
                transition.to_state
            ))
        })?;
        if to_state != OperationState::Completed {
            return Ok(None);
        }

        let command_id =
            required_command_id(transition.command_id, "command transition", event_lsn)?;
        let key = (event_domain.clone(), command_id);
        self.completed.insert(key.clone());
        self.try_issue(&key)
    }

    fn observe_session_state(
        &mut self,
        event: &RecordedEvent,
        event_domain: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
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

        let Some(session_state_event::Mutation::Registered(registered)) = state_event.mutation
        else {
            return Ok(None);
        };
        let Some(spawn_origin) = registered.spawn_origin else {
            return Ok(None);
        };
        let Some(typed_correlation::Ref::CommandId(spawn_command_id)) = spawn_origin.r#ref else {
            return Ok(None);
        };
        if spawn_command_id.value.is_empty() {
            return Err(AuthorityError::CorruptRecord(format!(
                "session registration at LSN {event_lsn} has an empty spawn command id"
            )));
        }

        let spawned_session_scope = TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(required_identifier(
                registered.adapter_id,
                "adapter_id",
                event_lsn,
            )?),
            runtime_session_id: Some(required_identifier(
                registered.runtime_session_id,
                "runtime_session_id",
                event_lsn,
            )?),
            session_generation: Some(registered.session_generation.ok_or_else(|| {
                AuthorityError::CorruptRecord(format!(
                    "session registration at LSN {event_lsn} is missing session_generation"
                ))
            })?),
            deployment_scope: required_string(
                registered.deployment_scope,
                "deployment_scope",
                event_lsn,
            )?,
            ..TargetScope::default()
        };
        let key = (event_domain.clone(), spawn_command_id);
        let incoming = RegistrationInfo {
            spawned_session_scope,
            authority_domain_id: event_domain.clone(),
        };
        insert_consistent(
            &mut self.registrations,
            key.clone(),
            incoming,
            "spawn-correlated session registration",
            event_lsn,
        )?;
        self.try_issue(&key)
    }

    fn try_issue(
        &mut self,
        key: &SpawnKey,
    ) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
        if self.issued.contains(key) || !self.completed.contains(key) {
            return Ok(None);
        }
        let Some(spawn_op) = self.spawn_ops.get(key) else {
            return Ok(None);
        };
        let Some(registration) = self.registrations.get(key) else {
            return Ok(None);
        };
        let subject_actor_id = spawn_op.spawner_actor.clone().ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "completed spawn operation {:?} in authority domain {:?} has no sender actor",
                key.1, key.0
            ))
        })?;

        let issuance = DescendantGrantIssuance {
            spawn_operation_id: key.1.clone(),
            spawning_grant_id: spawn_op.spawning_grant_id.clone(),
            spawned_session_scope: registration.spawned_session_scope.clone(),
            subject_actor_id,
            authority_domain_id: registration.authority_domain_id.clone(),
            allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS.to_vec(),
            descendant_grant_id: descendant_grant_id(&key.0, &key.1),
            audit_id: None,
        };
        self.issued.insert(key.clone());
        Ok(Some(issuance))
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

/// Canonical deterministic descendant-grant id, namespaced from operator grants.
fn descendant_grant_id(domain: &AuthorityDomainId, spawn_op: &CommandId) -> GrantId {
    GrantId {
        value: format!("desc:{}:{}", domain.value, spawn_op.value),
    }
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), AuthorityError> {
    let domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        AuthorityError::CorruptRecord("spawn-tail event has no authority domain".to_owned())
    })?;
    if domain.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(
            "spawn-tail event has an empty authority domain".to_owned(),
        ));
    }
    let lsn =
        event.event_id.lsn.as_ref().ok_or_else(|| {
            AuthorityError::CorruptRecord("spawn-tail event has no LSN".to_owned())
        })?;
    Ok((domain, lsn.value))
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
    let command_id = command_id.ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} is missing command_id"
        ))
    })?;
    if command_id.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} has an empty command_id"
        )));
    }
    Ok(command_id)
}

trait NonEmptyIdentifier {
    fn is_empty(&self) -> bool;
    fn field_name() -> &'static str;
}

impl NonEmptyIdentifier for patchbay_contracts::patchbay::AdapterId {
    fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    fn field_name() -> &'static str {
        "adapter_id"
    }
}

impl NonEmptyIdentifier for patchbay_contracts::patchbay::RuntimeSessionId {
    fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    fn field_name() -> &'static str {
        "runtime_session_id"
    }
}

fn required_identifier<T: NonEmptyIdentifier>(
    identifier: Option<T>,
    field_name: &str,
    event_lsn: u64,
) -> Result<T, AuthorityError> {
    let identifier = identifier.ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "session registration at LSN {event_lsn} is missing {field_name}"
        ))
    })?;
    if identifier.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "session registration at LSN {event_lsn} has an empty {}",
            T::field_name()
        )));
    }
    Ok(identifier)
}

fn required_string(
    value: String,
    field_name: &str,
    event_lsn: u64,
) -> Result<String, AuthorityError> {
    if value.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "session registration at LSN {event_lsn} has an empty {field_name}"
        )));
    }
    Ok(value)
}

fn insert_consistent<K, V>(
    map: &mut HashMap<K, V>,
    key: K,
    incoming: V,
    record_name: &str,
    event_lsn: u64,
) -> Result<(), AuthorityError>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
    V: PartialEq,
{
    if let Some(existing) = map.get(&key) {
        if existing == &incoming {
            return Ok(());
        }
        return Err(AuthorityError::CorruptLog(format!(
            "{record_name} has conflicting records for key {key:?} at LSN {event_lsn}"
        )));
    }
    map.insert(key, incoming);
    Ok(())
}
