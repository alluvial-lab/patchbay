//! Durable spawn-generation claim and pending-replacement projection.
//!
//! The authority-domain log is the only source of claim state. Command
//! terminality is deliberately a sibling projection concern: failed,
//! cancelled, expired, and every other `CommandState` leave claims unchanged.
//! Only the schema-closed, durably referenced evidence carried by a
//! `SpawnClaimEvent` may release, poison, promote, or abandon a claim.

use std::collections::{BTreeMap, HashMap, HashSet};

use patchbay_contracts::patchbay::{
    no_external_effect_proof, spawn_claim_disposition_changed, spawn_claim_event,
    AcceptedOperation, AuthorityDomainId, CommandId, ContinuationAuthorityProvenance, EventId,
    FailureCode, Lsn, NoExternalEffectProof, OperationKind, OperationState, RuntimeGenerationRef,
    SpawnClaimAccepted, SpawnClaimCheckpoint, SpawnClaimCheckpointRecord, SpawnClaimDisposition,
    SpawnClaimDispositionChanged, SpawnClaimEvent, SpawnGenerationClaim,
    SpawnPendingReplacementFence, SpawnPriorWorkDisposition, SpawnPriorWorkEffect, StoredEventKind,
    StoredEventPayload,
};
use prost::Message;

use crate::storage::{validate_next_replay_event, RecordedEvent, Storage};

use super::{external_runtime_key, LogicalTargetError};

pub const REPLACEMENT_PENDING_REASON: &str = "replacement_pending";

/// Exclusive authority-domain + logical-target + expected-prior-generation key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpawnClaimKey {
    pub authority_domain_id: String,
    pub logical_target_id: String,
    pub expected_prior_generation: Option<u64>,
}

/// Durable claim state reconstructed from accepted and disposition events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnClaimRecord {
    pub claim: SpawnGenerationClaim,
    pub accepted_lsn: u64,
    pub compound_authority: Option<ContinuationAuthorityProvenance>,
    pub disposition: SpawnClaimDisposition,
    pub pending_replacement: Option<RuntimeGenerationRef>,
}

/// Canonical delivery-fence answer for work bound to an exact runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnDeliveryFence {
    Open,
    ReplacementPending {
        claim_operation_id: CommandId,
        failure_code: FailureCode,
        reason_code: &'static str,
    },
}

/// Acceptance-time claim reconciliation without creating a second record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnClaimability<'a> {
    Available,
    ExactRetry(&'a SpawnClaimRecord),
    Conflict(&'a SpawnClaimRecord),
    Invalid,
}

/// Read-only claim and exact-generation fence queries consumed by later units.
pub trait SpawnClaimQuery {
    fn claim_for_operation(&self, command_id: &CommandId) -> Option<&SpawnClaimRecord>;
    fn classify_claim(&self, claim: &SpawnGenerationClaim) -> SpawnClaimability<'_>;
    fn delivery_fence(&self, runtime: &RuntimeGenerationRef) -> SpawnDeliveryFence;
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnClaimError {
    #[error("spawn-claim projection requires a non-empty authority domain")]
    EmptyAuthorityDomain,
    #[error("spawn-claim record is malformed: {0}")]
    CorruptRecord(String),
    #[error("spawn-claim log is corrupt: {0}")]
    CorruptLog(String),
    #[error("spawn generation is already consumed by command {0:?}")]
    GenerationAlreadyClaimed(CommandId),
    #[error("spawn claim references unknown command {0:?}")]
    UnknownClaim(CommandId),
    #[error("illegal spawn-claim disposition transition {from:?} -> {to:?}")]
    IllegalDispositionTransition {
        from: SpawnClaimDisposition,
        to: SpawnClaimDisposition,
    },
    #[error(transparent)]
    LogicalTarget(#[from] LogicalTargetError),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}

/// One authority-domain claim projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnClaimRegistry {
    authority_domain_id: AuthorityDomainId,
    applied_through_lsn: u64,
    applied_events: BTreeMap<u64, StoredEventPayload>,
    records: HashMap<CommandId, SpawnClaimRecord>,
    exclusive_claims: HashMap<SpawnClaimKey, CommandId>,
    prior_work_effects: HashMap<CommandId, Vec<SpawnPriorWorkEffect>>,
}

impl SpawnClaimRegistry {
    pub fn new(authority_domain_id: AuthorityDomainId) -> Result<Self, SpawnClaimError> {
        validate_domain(&authority_domain_id)?;
        Ok(Self {
            authority_domain_id,
            applied_through_lsn: 0,
            applied_events: BTreeMap::new(),
            records: HashMap::new(),
            exclusive_claims: HashMap::new(),
            prior_work_effects: HashMap::new(),
        })
    }

    /// Restore the complete claim projection from a domain/LSN-bound checkpoint.
    pub fn from_checkpoint(checkpoint: SpawnClaimCheckpoint) -> Result<Self, SpawnClaimError> {
        let authority_domain_id = checkpoint
            .authority_domain_id
            .ok_or_else(|| corrupt_record("checkpoint is missing authority_domain_id"))?;
        validate_domain(&authority_domain_id)?;
        let snapshot_lsn = checkpoint
            .snapshot_lsn
            .filter(|lsn| lsn.value > 0)
            .ok_or_else(|| corrupt_record("checkpoint is missing a positive snapshot_lsn"))?
            .value;
        let mut registry = Self::new(authority_domain_id)?;
        registry.applied_through_lsn = snapshot_lsn;

        for wire in checkpoint.records {
            let (record, prior_work_effects) =
                decode_checkpoint_record(&registry.authority_domain_id, snapshot_lsn, wire)?;
            let command_id = required_command_id(&record.claim)?.clone();
            if registry.records.contains_key(&command_id) {
                return Err(corrupt_record(
                    "checkpoint contains duplicate claim_operation_id",
                ));
            }
            if disposition_consumes_generation(record.disposition) {
                let key = claim_key(&record.claim)?;
                if let Some(owner) = registry.exclusive_claims.insert(key, command_id.clone()) {
                    return Err(SpawnClaimError::CorruptLog(format!(
                        "checkpoint gives one generation to commands {:?} and {:?}",
                        owner, command_id
                    )));
                }
            }
            registry
                .prior_work_effects
                .insert(command_id.clone(), prior_work_effects);
            registry.records.insert(command_id, record);
        }
        Ok(registry)
    }

    #[must_use]
    pub fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.authority_domain_id
    }

    #[must_use]
    pub fn applied_through_lsn(&self) -> u64 {
        self.applied_through_lsn
    }

    pub fn records(&self) -> impl Iterator<Item = &SpawnClaimRecord> {
        self.records.values()
    }

    #[must_use]
    pub fn prior_work_effects(&self, command_id: &CommandId) -> &[SpawnPriorWorkEffect] {
        self.prior_work_effects
            .get(command_id)
            .map_or(&[], Vec::as_slice)
    }

    /// Fold one exact durable-log successor. Failed validation is atomic.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), SpawnClaimError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            corrupt_record(format!("unknown stored event kind {}", event.payload.kind))
        })?;
        if kind == StoredEventKind::Unspecified {
            return Err(corrupt_log("stored event kind is unspecified"));
        }
        let (event_domain, event_lsn) = event_identity(event)?;
        if event_domain != &self.authority_domain_id {
            return Err(corrupt_log(format!(
                "event authority domain {:?} does not match claim projection {:?}",
                event_domain, self.authority_domain_id
            )));
        }

        if event_lsn <= self.applied_through_lsn {
            return match self.applied_events.get(&event_lsn) {
                Some(payload) if payload == &event.payload => Ok(()),
                Some(_) => Err(corrupt_log(format!(
                    "event identity ({:?}, {event_lsn}) has conflicting envelopes",
                    self.authority_domain_id
                ))),
                None => Err(corrupt_log(format!(
                    "claim checkpoint covers LSN {event_lsn}; compacted event bytes cannot be authenticated"
                ))),
            };
        }
        let expected = self
            .applied_through_lsn
            .checked_add(1)
            .ok_or_else(|| corrupt_log("claim applied LSN overflow"))?;
        if event_lsn != expected {
            return Err(corrupt_log(format!(
                "event LSN {event_lsn} leaves a gap after {}",
                self.applied_through_lsn
            )));
        }

        let mut next = self.clone();
        if kind == StoredEventKind::SpawnClaim {
            let claim_event =
                SpawnClaimEvent::decode(event.payload.payload.as_slice()).map_err(|error| {
                    corrupt_record(format!(
                        "cannot decode spawn-claim event at LSN {event_lsn}: {error}"
                    ))
                })?;
            next.apply_claim_event(claim_event, event_lsn)?;
        }
        next.applied_through_lsn = event_lsn;
        next.applied_events.insert(event_lsn, event.payload.clone());
        *self = next;
        Ok(())
    }

    /// Encode a deterministic private checkpoint at the currently applied LSN.
    pub fn checkpoint(&self) -> Result<SpawnClaimCheckpoint, SpawnClaimError> {
        if self.applied_through_lsn == 0 {
            return Err(corrupt_record("cannot checkpoint an empty claim prefix"));
        }
        let mut records: Vec<_> = self.records.values().collect();
        records.sort_by(|left, right| {
            required_command_id(&left.claim)
                .expect("stored claim was validated")
                .value
                .cmp(
                    &required_command_id(&right.claim)
                        .expect("stored claim was validated")
                        .value,
                )
        });
        Ok(SpawnClaimCheckpoint {
            authority_domain_id: Some(self.authority_domain_id.clone()),
            snapshot_lsn: Some(Lsn {
                value: self.applied_through_lsn,
            }),
            records: records
                .into_iter()
                .map(|record| SpawnClaimCheckpointRecord {
                    claim: Some(record.claim.clone()),
                    accepted_lsn: Some(Lsn {
                        value: record.accepted_lsn,
                    }),
                    compound_authority: record.compound_authority.clone(),
                    disposition: record.disposition as i32,
                    pending_replacement: record.pending_replacement.clone(),
                    prior_work_effects: self
                        .prior_work_effects
                        .get(
                            required_command_id(&record.claim).expect("stored claim was validated"),
                        )
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect(),
        })
    }

    fn apply_claim_event(
        &mut self,
        event: SpawnClaimEvent,
        event_lsn: u64,
    ) -> Result<(), SpawnClaimError> {
        let domain = event
            .authority_domain_id
            .as_ref()
            .ok_or_else(|| corrupt_record("spawn-claim event is missing authority_domain_id"))?;
        if domain != &self.authority_domain_id {
            return Err(corrupt_log(format!(
                "spawn-claim payload domain {:?} does not match envelope domain {:?}",
                domain, self.authority_domain_id
            )));
        }
        match event
            .mutation
            .ok_or_else(|| corrupt_record("spawn-claim event is missing mutation"))?
        {
            spawn_claim_event::Mutation::Accepted(accepted) => {
                self.apply_accepted(accepted, event_lsn)
            }
            spawn_claim_event::Mutation::DispositionChanged(change) => {
                self.apply_disposition_change(change, event_lsn)
            }
        }
    }

    fn apply_accepted(
        &mut self,
        accepted: SpawnClaimAccepted,
        event_lsn: u64,
    ) -> Result<(), SpawnClaimError> {
        validate_accepted_decision(&self.authority_domain_id, &accepted)?;
        let claim = accepted.claim.clone().expect("validated claim");
        let command_id = required_command_id(&claim)?.clone();
        if self.records.contains_key(&command_id) {
            return Err(corrupt_log(format!(
                "duplicate durable claim acceptance for command {:?}",
                command_id
            )));
        }
        let key = claim_key(&claim)?;
        if let Some(owner) = self.exclusive_claims.get(&key) {
            return Err(SpawnClaimError::GenerationAlreadyClaimed(owner.clone()));
        }
        let pending_replacement = accepted
            .pending_replacement
            .as_ref()
            .and_then(|fence| fence.exact_prior.clone());
        self.exclusive_claims.insert(key, command_id.clone());
        self.prior_work_effects
            .insert(command_id.clone(), accepted.prior_work_effects);
        self.records.insert(
            command_id,
            SpawnClaimRecord {
                claim,
                accepted_lsn: event_lsn,
                compound_authority: accepted.compound_authority,
                disposition: SpawnClaimDisposition::Active,
                pending_replacement,
            },
        );
        Ok(())
    }

    fn apply_disposition_change(
        &mut self,
        change: SpawnClaimDispositionChanged,
        event_lsn: u64,
    ) -> Result<(), SpawnClaimError> {
        let command_id = change
            .claim_operation_id
            .clone()
            .filter(|id| !id.value.is_empty())
            .ok_or_else(|| corrupt_record("disposition change is missing claim_operation_id"))?;
        let from = required_disposition(change.from_disposition, "from")?;
        let to = required_disposition(change.to_disposition, "to")?;
        let projected = self
            .records
            .get(&command_id)
            .ok_or_else(|| SpawnClaimError::UnknownClaim(command_id.clone()))?;
        if projected.disposition != from {
            return Err(corrupt_log(format!(
                "disposition change for {:?} expects {:?}, projected {:?}",
                command_id, from, projected.disposition
            )));
        }
        if !allowed_spawn_claim_transition(from, to) {
            return Err(SpawnClaimError::IllegalDispositionTransition { from, to });
        }
        validate_transition_evidence(
            &self.authority_domain_id,
            projected,
            to,
            change.evidence.as_ref(),
            event_lsn,
        )?;

        let key = claim_key(&projected.claim)?;
        let record = self
            .records
            .get_mut(&command_id)
            .expect("claim existence validated above");
        record.disposition = to;
        if matches!(
            to,
            SpawnClaimDisposition::ReleasedNoExternalEffect
                | SpawnClaimDisposition::Promoted
                | SpawnClaimDisposition::TargetAbandoned
        ) {
            record.pending_replacement = None;
        }
        if to == SpawnClaimDisposition::ReleasedNoExternalEffect {
            if self.exclusive_claims.get(&key) != Some(&command_id) {
                return Err(corrupt_log("released claim did not own its exclusive key"));
            }
            self.exclusive_claims.remove(&key);
        }
        Ok(())
    }
}

impl SpawnClaimQuery for SpawnClaimRegistry {
    fn claim_for_operation(&self, command_id: &CommandId) -> Option<&SpawnClaimRecord> {
        self.records.get(command_id)
    }

    fn classify_claim(&self, claim: &SpawnGenerationClaim) -> SpawnClaimability<'_> {
        let Some(command_id) = claim
            .claim_operation_id
            .as_ref()
            .filter(|command_id| !command_id.value.is_empty())
        else {
            return SpawnClaimability::Invalid;
        };
        if let Some(existing) = self.records.get(command_id) {
            return if existing.claim == *claim {
                SpawnClaimability::ExactRetry(existing)
            } else {
                SpawnClaimability::Conflict(existing)
            };
        }
        if validate_claim(&self.authority_domain_id, claim).is_err() {
            return SpawnClaimability::Invalid;
        }
        let Ok(key) = claim_key(claim) else {
            return SpawnClaimability::Invalid;
        };
        self.exclusive_claims
            .get(&key)
            .and_then(|owner| self.records.get(owner))
            .map_or(SpawnClaimability::Available, SpawnClaimability::Conflict)
    }

    fn delivery_fence(&self, runtime: &RuntimeGenerationRef) -> SpawnDeliveryFence {
        let Some((command_id, _)) = self.records.iter().find(|(command_id, record)| {
            record.pending_replacement.as_ref() == Some(runtime)
                && matches!(
                    record.disposition,
                    SpawnClaimDisposition::Active
                        | SpawnClaimDisposition::PoisonedPendingReconciliation
                )
                && self
                    .exclusive_claims
                    .get(&claim_key(&record.claim).expect("stored claim was validated"))
                    == Some(*command_id)
        }) else {
            return SpawnDeliveryFence::Open;
        };
        SpawnDeliveryFence::ReplacementPending {
            claim_operation_id: command_id.clone(),
            failure_code: FailureCode::Superseded,
            reason_code: REPLACEMENT_PENDING_REASON,
        }
    }
}

/// Exact legal disposition adjacency. Terminal dispositions never reactivate.
#[must_use]
pub const fn allowed_spawn_claim_transition(
    from: SpawnClaimDisposition,
    to: SpawnClaimDisposition,
) -> bool {
    use SpawnClaimDisposition::{
        Active, PoisonedPendingReconciliation, Promoted, ReleasedNoExternalEffect, TargetAbandoned,
        Unspecified,
    };
    match from {
        Active => matches!(
            to,
            ReleasedNoExternalEffect | PoisonedPendingReconciliation | Promoted | TargetAbandoned
        ),
        PoisonedPendingReconciliation => {
            matches!(to, ReleasedNoExternalEffect | Promoted | TargetAbandoned)
        }
        Unspecified | ReleasedNoExternalEffect | Promoted | TargetAbandoned => false,
    }
}

/// Rebuild claims from the exact gap-free authority-domain log.
pub async fn rebuild_spawn_claims_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<SpawnClaimRegistry, SpawnClaimError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut registry = SpawnClaimRegistry::new(authority_domain_id.clone())?;
    let mut previous_lsn = 0;
    for event in events {
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, &event)
            .map_err(|error| {
                error.map(SpawnClaimError::CorruptRecord, SpawnClaimError::CorruptLog)
            })?;
        registry.observe(&event)?;
        previous_lsn = validated.lsn;
    }
    Ok(registry)
}

/// Encode a generated claim event in the schema-owned durable envelope.
#[must_use]
pub fn encode_spawn_claim_event(event: &SpawnClaimEvent) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::SpawnClaim as i32,
        payload: event.encode_to_vec(),
    }
}

fn validate_accepted_decision(
    domain: &AuthorityDomainId,
    accepted: &SpawnClaimAccepted,
) -> Result<(), SpawnClaimError> {
    let accepted_operation = accepted
        .accepted_operation
        .as_ref()
        .ok_or_else(|| corrupt_record("accepted claim is missing accepted_operation"))?;
    validate_accepted_operation(domain, accepted_operation)?;
    let claim = accepted
        .claim
        .as_ref()
        .ok_or_else(|| corrupt_record("accepted claim is missing claim"))?;
    validate_claim(domain, claim)?;
    let operation = accepted_operation
        .operation
        .as_ref()
        .expect("accepted operation validated");
    if operation.command_id.as_ref() != claim.claim_operation_id.as_ref() {
        return Err(corrupt_log(
            "accepted operation command_id does not match claim_operation_id",
        ));
    }

    match claim.expected_prior.as_ref() {
        None => {
            if accepted.compound_authority.is_some()
                || accepted.pending_replacement.is_some()
                || !accepted.prior_work_effects.is_empty()
            {
                return Err(corrupt_log(
                    "fresh claim carries continuation authority, fence, or prior-work effects",
                ));
            }
        }
        Some(prior) => {
            validate_compound_authority(prior, accepted.compound_authority.as_ref())?;
            validate_pending_replacement(prior, accepted.pending_replacement.as_ref())?;
            validate_prior_work_effects(
                claim
                    .claim_operation_id
                    .as_ref()
                    .expect("validated command id"),
                &accepted.prior_work_effects,
            )?;
        }
    }
    Ok(())
}

fn validate_accepted_operation(
    domain: &AuthorityDomainId,
    accepted: &AcceptedOperation,
) -> Result<(), SpawnClaimError> {
    let operation = accepted
        .operation
        .as_ref()
        .ok_or_else(|| corrupt_record("accepted claim is missing operation"))?;
    if operation.authority_domain_id.as_ref() != Some(domain) {
        return Err(corrupt_log(
            "accepted claim operation belongs to another authority domain",
        ));
    }
    if OperationKind::try_from(operation.kind).ok() != Some(OperationKind::Spawn) {
        return Err(corrupt_log("spawn claim wraps a non-spawn operation"));
    }
    if operation
        .command_id
        .as_ref()
        .is_none_or(|command_id| command_id.value.is_empty())
    {
        return Err(corrupt_record("accepted spawn operation has no command_id"));
    }
    if accepted
        .authorizing_grant_id
        .as_ref()
        .is_none_or(|grant_id| grant_id.value.is_empty())
    {
        return Err(corrupt_record(
            "accepted spawn operation has no authorizing_grant_id",
        ));
    }
    Ok(())
}

fn validate_claim(
    domain: &AuthorityDomainId,
    claim: &SpawnGenerationClaim,
) -> Result<(), SpawnClaimError> {
    if claim.authority_domain_id.as_ref() != Some(domain) {
        return Err(corrupt_log("claim belongs to another authority domain"));
    }
    let command_id = required_command_id(claim)?;
    if command_id.value.is_empty() {
        return Err(corrupt_record("claim_operation_id is empty"));
    }
    let logical_target_id = claim
        .logical_target_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| corrupt_record("claim is missing logical_target_id"))?;
    let claimed_generation = claim
        .claimed_generation
        .filter(|generation| generation.value > 0)
        .ok_or_else(|| corrupt_record("claim is missing a positive claimed_generation"))?;

    match claim.expected_prior.as_ref() {
        None if claimed_generation.value == 1 => Ok(()),
        None => Err(corrupt_log("fresh claim must consume generation 1")),
        Some(prior) => {
            if prior.logical_target_id.as_ref() != Some(logical_target_id) {
                return Err(corrupt_log(
                    "continuation prior names another logical target",
                ));
            }
            let external = prior
                .external_runtime
                .as_ref()
                .ok_or_else(|| corrupt_record("continuation prior has no external runtime"))?;
            let prior_generation = external
                .generation
                .filter(|generation| generation.value > 0)
                .ok_or_else(|| corrupt_record("continuation prior has no positive generation"))?;
            external_runtime_key(domain, external)?;
            let expected = prior_generation
                .value
                .checked_add(1)
                .ok_or_else(|| corrupt_log("continuation generation overflows"))?;
            if claimed_generation.value != expected {
                return Err(corrupt_log(format!(
                    "continuation must claim exact N+1: prior {}, claimed {}",
                    prior_generation.value, claimed_generation.value
                )));
            }
            Ok(())
        }
    }
}

fn validate_compound_authority(
    prior: &RuntimeGenerationRef,
    authority: Option<&ContinuationAuthorityProvenance>,
) -> Result<(), SpawnClaimError> {
    let authority = authority
        .ok_or_else(|| corrupt_record("continuation claim is missing compound authority"))?;
    if authority.exact_prior.as_ref() != Some(prior)
        || authority
            .replacement_grant_id
            .as_ref()
            .is_none_or(|grant_id| grant_id.value.is_empty())
        || OperationKind::try_from(authority.replacement_authority_kind).ok()
            != Some(OperationKind::SessionManagement)
    {
        return Err(corrupt_log(
            "continuation compound authority is not exact-prior session-management authority",
        ));
    }
    Ok(())
}

fn validate_pending_replacement(
    prior: &RuntimeGenerationRef,
    fence: Option<&SpawnPendingReplacementFence>,
) -> Result<(), SpawnClaimError> {
    let fence = fence
        .ok_or_else(|| corrupt_record("continuation claim is missing pending-replacement fence"))?;
    if fence.exact_prior.as_ref() != Some(prior)
        || FailureCode::try_from(fence.failure_code).ok() != Some(FailureCode::Superseded)
        || fence.reason_code != REPLACEMENT_PENDING_REASON
    {
        return Err(corrupt_log(
            "continuation fence is not exact-prior superseded/replacement_pending",
        ));
    }
    Ok(())
}

fn validate_prior_work_effects(
    claim_operation_id: &CommandId,
    effects: &[SpawnPriorWorkEffect],
) -> Result<(), SpawnClaimError> {
    let mut commands = HashSet::new();
    for effect in effects {
        let command_id = effect
            .command_id
            .as_ref()
            .filter(|id| !id.value.is_empty())
            .ok_or_else(|| corrupt_record("prior-work effect is missing command_id"))?;
        if command_id == claim_operation_id || !commands.insert(command_id.clone()) {
            return Err(corrupt_log(
                "prior-work effects contain the claim command or a duplicate command",
            ));
        }
        if effect.reason_code != REPLACEMENT_PENDING_REASON {
            return Err(corrupt_log(
                "prior-work effect has a non-canonical reason code",
            ));
        }
        let state = required_operation_state(effect.prior_state, "prior-work state")?;
        let disposition = SpawnPriorWorkDisposition::try_from(effect.disposition)
            .map_err(|_| corrupt_record("prior-work effect has unknown disposition"))?;
        let failure = FailureCode::try_from(effect.failure_code)
            .map_err(|_| corrupt_record("prior-work effect has unknown failure code"))?;
        match disposition {
            SpawnPriorWorkDisposition::SupersededBeforeOffer
                if state == OperationState::Accepted && failure == FailureCode::Superseded => {}
            SpawnPriorWorkDisposition::QuiesceOutcomeReconciliation
                if matches!(state, OperationState::Delivered | OperationState::Running)
                    && failure == FailureCode::Unspecified => {}
            SpawnPriorWorkDisposition::Unspecified => {
                return Err(corrupt_record(
                    "prior-work effect has unspecified disposition",
                ));
            }
            _ => {
                return Err(corrupt_log(
                    "prior-work effect does not match accepted-vs-offered lifecycle",
                ));
            }
        }
    }
    Ok(())
}

fn validate_transition_evidence(
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    to: SpawnClaimDisposition,
    evidence: Option<&spawn_claim_disposition_changed::Evidence>,
    event_lsn: u64,
) -> Result<(), SpawnClaimError> {
    match (to, evidence) {
        (
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            Some(spawn_claim_disposition_changed::Evidence::NoExternalEffectRelease(release)),
        ) => validate_no_effect_release(domain, record, release, event_lsn),
        (
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            Some(spawn_claim_disposition_changed::Evidence::AmbiguousExternalEffect(ambiguity)),
        ) => validate_prior_event(
            domain,
            ambiguity.evidence_event_id.as_ref(),
            event_lsn,
            "ambiguous external-effect evidence",
        ),
        (
            SpawnClaimDisposition::Promoted,
            Some(spawn_claim_disposition_changed::Evidence::Promotion(promotion)),
        ) => {
            validate_prior_event(
                domain,
                promotion.promotion_event_id.as_ref(),
                event_lsn,
                "promotion evidence",
            )?;
            let runtime = promotion
                .promoted_runtime
                .as_ref()
                .ok_or_else(|| corrupt_record("promotion evidence has no promoted runtime"))?;
            validate_promoted_runtime(domain, &record.claim, runtime)
        }
        (
            SpawnClaimDisposition::TargetAbandoned,
            Some(spawn_claim_disposition_changed::Evidence::TargetAbandonment(abandonment)),
        ) => validate_prior_event(
            domain,
            abandonment.abandonment_event_id.as_ref(),
            event_lsn,
            "target-abandonment evidence",
        ),
        _ => Err(corrupt_log(
            "claim disposition is not paired with its exact closed-vocabulary evidence",
        )),
    }
}

fn validate_no_effect_release(
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    release: &patchbay_contracts::patchbay::SpawnClaimNoEffectRelease,
    event_lsn: u64,
) -> Result<(), SpawnClaimError> {
    validate_no_effect_proof(
        domain,
        release
            .proof
            .as_ref()
            .ok_or_else(|| corrupt_record("release is missing no-external-effect proof"))?,
        event_lsn,
    )?;
    match record.claim.expected_prior.as_ref() {
        Some(prior) => {
            if release.exact_prior_liveness.as_ref() != Some(prior) {
                return Err(corrupt_log(
                    "continuation release lacks exact prior-N liveness",
                ));
            }
            validate_prior_event(
                domain,
                release.prior_liveness_event_id.as_ref(),
                event_lsn,
                "prior-N liveness evidence",
            )
        }
        None if release.exact_prior_liveness.is_none()
            && release.prior_liveness_event_id.is_none() =>
        {
            Ok(())
        }
        None => Err(corrupt_log(
            "fresh claim release carries unrelated prior-N liveness evidence",
        )),
    }
}

fn validate_no_effect_proof(
    domain: &AuthorityDomainId,
    proof: &NoExternalEffectProof,
    event_lsn: u64,
) -> Result<(), SpawnClaimError> {
    match proof
        .proof
        .as_ref()
        .ok_or_else(|| corrupt_record("no-external-effect proof has no variant"))?
    {
        no_external_effect_proof::Proof::CorePreDeliveryTerminal(core) => validate_prior_event(
            domain,
            core.decision_event_id.as_ref(),
            event_lsn,
            "core pre-delivery terminal proof",
        ),
        no_external_effect_proof::Proof::AuthenticatedAdapterRefusalBeforeDelivery(adapter) => {
            validate_adapter_proof(
                domain,
                adapter.evidence_event_id.as_ref(),
                adapter.adapter_id.as_ref(),
                adapter.adapter_generation.as_ref(),
                event_lsn,
                "authenticated adapter refusal-before-delivery proof",
            )
        }
        no_external_effect_proof::Proof::ExactSupervisorPreLaunchFailure(supervisor) => {
            validate_adapter_proof(
                domain,
                supervisor.evidence_event_id.as_ref(),
                supervisor.adapter_id.as_ref(),
                supervisor.adapter_generation.as_ref(),
                event_lsn,
                "exact supervisor pre-launch failure proof",
            )
        }
    }
}

fn validate_adapter_proof(
    domain: &AuthorityDomainId,
    event_id: Option<&EventId>,
    adapter_id: Option<&patchbay_contracts::patchbay::AdapterId>,
    adapter_generation: Option<&patchbay_contracts::patchbay::Generation>,
    event_lsn: u64,
    name: &str,
) -> Result<(), SpawnClaimError> {
    validate_prior_event(domain, event_id, event_lsn, name)?;
    if adapter_id.is_none_or(|adapter| adapter.value.is_empty())
        || adapter_generation.is_none_or(|generation| generation.value == 0)
    {
        return Err(corrupt_record(format!(
            "{name} lacks authenticated adapter identity/generation"
        )));
    }
    Ok(())
}

fn validate_promoted_runtime(
    domain: &AuthorityDomainId,
    claim: &SpawnGenerationClaim,
    runtime: &RuntimeGenerationRef,
) -> Result<(), SpawnClaimError> {
    if runtime.logical_target_id != claim.logical_target_id {
        return Err(corrupt_log("promoted runtime names another logical target"));
    }
    let external = runtime
        .external_runtime
        .as_ref()
        .ok_or_else(|| corrupt_record("promoted runtime has no external reference"))?;
    external_runtime_key(domain, external)?;
    if external.generation != claim.claimed_generation {
        return Err(corrupt_log(
            "promoted runtime generation does not equal the claimed generation",
        ));
    }
    Ok(())
}

fn validate_prior_event(
    domain: &AuthorityDomainId,
    event_id: Option<&EventId>,
    current_lsn: u64,
    name: &str,
) -> Result<(), SpawnClaimError> {
    let event_id = event_id.ok_or_else(|| corrupt_record(format!("{name} has no event id")))?;
    if event_id.authority_domain_id.as_ref() != Some(domain)
        || event_id
            .lsn
            .as_ref()
            .is_none_or(|lsn| lsn.value == 0 || lsn.value >= current_lsn)
    {
        return Err(corrupt_log(format!(
            "{name} does not reference a prior durable event in the claim authority domain"
        )));
    }
    Ok(())
}

fn decode_checkpoint_record(
    domain: &AuthorityDomainId,
    checkpoint_lsn: u64,
    wire: SpawnClaimCheckpointRecord,
) -> Result<(SpawnClaimRecord, Vec<SpawnPriorWorkEffect>), SpawnClaimError> {
    let claim = wire
        .claim
        .ok_or_else(|| corrupt_record("checkpoint claim record is missing claim"))?;
    validate_claim(domain, &claim)?;
    let accepted_lsn = wire
        .accepted_lsn
        .filter(|lsn| lsn.value > 0 && lsn.value <= checkpoint_lsn)
        .ok_or_else(|| corrupt_record("claim accepted_lsn is outside checkpoint prefix"))?
        .value;
    let disposition = required_disposition(wire.disposition, "checkpoint")?;
    let continuation = claim.expected_prior.as_ref();
    if continuation.is_some() {
        validate_prior_work_effects(required_command_id(&claim)?, &wire.prior_work_effects)?;
    } else if !wire.prior_work_effects.is_empty() {
        return Err(corrupt_log(
            "fresh checkpoint claim carries prior-work effects",
        ));
    }
    match (continuation, disposition, wire.pending_replacement.as_ref()) {
        (
            Some(prior),
            SpawnClaimDisposition::Active | SpawnClaimDisposition::PoisonedPendingReconciliation,
            Some(pending),
        ) if pending == prior => {
            validate_compound_authority(prior, wire.compound_authority.as_ref())?
        }
        (
            None,
            SpawnClaimDisposition::Active | SpawnClaimDisposition::PoisonedPendingReconciliation,
            None,
        ) if wire.compound_authority.is_none() => {}
        (
            _,
            SpawnClaimDisposition::ReleasedNoExternalEffect
            | SpawnClaimDisposition::Promoted
            | SpawnClaimDisposition::TargetAbandoned,
            None,
        ) => {
            if let Some(prior) = continuation {
                validate_compound_authority(prior, wire.compound_authority.as_ref())?;
            } else if wire.compound_authority.is_some() {
                return Err(corrupt_log(
                    "fresh checkpoint claim carries compound authority",
                ));
            }
        }
        _ => {
            return Err(corrupt_log(
                "checkpoint claim disposition and pending-replacement fence disagree",
            ));
        }
    }
    Ok((
        SpawnClaimRecord {
            claim,
            accepted_lsn,
            compound_authority: wire.compound_authority,
            disposition,
            pending_replacement: wire.pending_replacement,
        },
        wire.prior_work_effects,
    ))
}

fn claim_key(claim: &SpawnGenerationClaim) -> Result<SpawnClaimKey, SpawnClaimError> {
    let domain = claim
        .authority_domain_id
        .as_ref()
        .filter(|domain| !domain.value.is_empty())
        .ok_or_else(|| corrupt_record("claim has no authority domain"))?;
    let target = claim
        .logical_target_id
        .as_ref()
        .filter(|target| !target.value.is_empty())
        .ok_or_else(|| corrupt_record("claim has no logical target"))?;
    let expected_prior_generation = claim
        .expected_prior
        .as_ref()
        .map(|prior| {
            prior
                .external_runtime
                .as_ref()
                .and_then(|external| external.generation)
                .filter(|generation| generation.value > 0)
                .map(|generation| generation.value)
                .ok_or_else(|| corrupt_record("claim prior has no positive generation"))
        })
        .transpose()?;
    Ok(SpawnClaimKey {
        authority_domain_id: domain.value.clone(),
        logical_target_id: target.value.clone(),
        expected_prior_generation,
    })
}

fn required_command_id(claim: &SpawnGenerationClaim) -> Result<&CommandId, SpawnClaimError> {
    claim
        .claim_operation_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| corrupt_record("claim has no claim_operation_id"))
}

fn required_disposition(raw: i32, field: &str) -> Result<SpawnClaimDisposition, SpawnClaimError> {
    let disposition = SpawnClaimDisposition::try_from(raw)
        .map_err(|_| corrupt_record(format!("{field} disposition is unknown")))?;
    if disposition == SpawnClaimDisposition::Unspecified {
        return Err(corrupt_record(format!(
            "{field} disposition is unspecified"
        )));
    }
    Ok(disposition)
}

fn required_operation_state(raw: i32, field: &str) -> Result<OperationState, SpawnClaimError> {
    let state =
        OperationState::try_from(raw).map_err(|_| corrupt_record(format!("{field} is unknown")))?;
    if state == OperationState::Unspecified {
        return Err(corrupt_record(format!("{field} is unspecified")));
    }
    Ok(state)
}

const fn disposition_consumes_generation(disposition: SpawnClaimDisposition) -> bool {
    !matches!(disposition, SpawnClaimDisposition::ReleasedNoExternalEffect)
}

fn validate_domain(domain: &AuthorityDomainId) -> Result<(), SpawnClaimError> {
    if domain.value.is_empty() {
        Err(SpawnClaimError::EmptyAuthorityDomain)
    } else {
        Ok(())
    }
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), SpawnClaimError> {
    let domain = event
        .event_id
        .authority_domain_id
        .as_ref()
        .ok_or_else(|| corrupt_record("event has no authority domain"))?;
    validate_domain(domain)?;
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .filter(|lsn| lsn.value > 0)
        .ok_or_else(|| corrupt_record("event has no positive LSN"))?;
    Ok((domain, lsn.value))
}

fn corrupt_record(message: impl Into<String>) -> SpawnClaimError {
    SpawnClaimError::CorruptRecord(message.into())
}

fn corrupt_log(message: impl Into<String>) -> SpawnClaimError {
    SpawnClaimError::CorruptLog(message.into())
}
