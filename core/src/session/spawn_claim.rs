//! Durable spawn-generation claim and pending-replacement projection.
//!
//! The authority-domain log is the only source of claim state. Command
//! terminality is deliberately a sibling projection concern: failed,
//! cancelled, expired, and every other `CommandState` leave claims unchanged.
//! Only the schema-closed, durably referenced evidence carried by a
//! `SpawnClaimEvent` may release, poison, promote, or abandon a claim.

use std::collections::{BTreeMap, HashMap, HashSet};

use patchbay_contracts::patchbay::{
    no_external_effect_proof, session_state_event, spawn_claim_disposition_changed,
    spawn_claim_event, AcceptedOperation, AdapterId, AuthorityDomainId, CommandId,
    ContinuationAuthorityProvenance, EventId, ExternalEffectDisposition, FailureCode, Lsn,
    Observation, ObservationKind, OperationKind, OperationState, RuntimeGenerationRef,
    SessionConnectivityState, SessionStateEvent, SpawnClaimAccepted, SpawnClaimDisposition,
    SpawnClaimDispositionChanged, SpawnClaimEvent, SpawnExecutionEvidence,
    SpawnExecutionEvidenceProducer, SpawnExecutionPhase, SpawnGenerationClaim,
    SpawnPendingReplacementFence, SpawnPriorWorkDisposition, SpawnPriorWorkEffect,
    SpawnPromotionCommitted, SpawnSuccessorEvidenceStaged, StoredEventKind, StoredEventPayload,
    TargetScope, TargetScopeKind,
};
use prost::Message;

use crate::{
    acceptance::{
        exact_command_correlation, validate_spawn_authority_carriage,
        validate_spawn_operation_payload,
    },
    adapter::AdapterRegistry,
    storage::{validate_next_replay_event, RecordedEvent, Storage},
};

use super::{
    external_runtime_key,
    spawn_orchestration::{phase_outcome, SpawnCompletionPhaseRecord},
    LogicalTargetError,
};

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
    /// Latest durable claim-disposition decision whose effect evidence must be
    /// superseded by any later no-effect release proof.
    pub latest_disposition_lsn: u64,
    /// Canonical adapter target from the accepted Operation. Evidence from any
    /// other adapter can never authorize this claim.
    pub adapter_id: AdapterId,
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
    fn claim_for_external_runtime(
        &self,
        runtime: &RuntimeGenerationRef,
    ) -> Option<&SpawnClaimRecord>;
    fn identified_runtime_for_operation(
        &self,
        command_id: &CommandId,
    ) -> Option<&RuntimeGenerationRef>;
    fn classify_claim(&self, claim: &SpawnGenerationClaim) -> SpawnClaimability<'_>;
    fn delivery_fence(&self, runtime: &RuntimeGenerationRef) -> SpawnDeliveryFence;
    fn delivery_fence_for_target_scope(&self, scope: &TargetScope) -> SpawnDeliveryFence;
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
    #[error("spawn claim {command_id:?} already identifies another external runtime")]
    ClaimRuntimeConflict { command_id: CommandId },
    #[error(
        "external runtime identified by claim {attempted:?} is already owned by claim {owner:?}"
    )]
    ExternalRuntimeOwnershipConflict {
        owner: CommandId,
        attempted: CommandId,
    },
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
///
/// Raw claim checkpoints are deliberately not a recovery authority. Recovery
/// rebuilds this projection from the durable authority-domain log until a
/// continuity-epoch and exact-row-anchored checkpoint loader exists.
///
/// ```compile_fail
/// use patchbay_contracts::patchbay::SpawnClaimCheckpoint;
/// use patchbay_core::session::SpawnClaimRegistry;
///
/// let hostile = SpawnClaimCheckpoint::default();
/// let _ = SpawnClaimRegistry::from_checkpoint(hostile);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnClaimRegistry {
    authority_domain_id: AuthorityDomainId,
    applied_through_lsn: u64,
    applied_events: BTreeMap<u64, StoredEventPayload>,
    records: HashMap<CommandId, SpawnClaimRecord>,
    exclusive_claims: HashMap<SpawnClaimKey, CommandId>,
    identified_runtime_by_claim: HashMap<CommandId, RuntimeGenerationRef>,
    external_runtime_claims: HashMap<super::ExternalRuntimeKey, CommandId>,
    prior_work_effects: HashMap<CommandId, Vec<SpawnPriorWorkEffect>>,
    completion_phases: HashMap<CommandId, SpawnCompletionPhaseRecord>,
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
            identified_runtime_by_claim: HashMap::new(),
            external_runtime_claims: HashMap::new(),
            prior_work_effects: HashMap::new(),
            completion_phases: HashMap::new(),
        })
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

    /// Pre-append boundary validation for a candidate typed evidence event.
    /// The later disposition fold revalidates the same bytes and current
    /// attachment, so an evidence append alone never mutates a claim.
    pub fn validate_execution_evidence_candidate(
        &self,
        evidence: &SpawnExecutionEvidence,
    ) -> Result<(), SpawnClaimError> {
        let command_id = evidence
            .exact_claim
            .as_ref()
            .and_then(|claim| claim.claim_operation_id.as_ref())
            .filter(|id| !id.value.is_empty())
            .ok_or_else(|| corrupt_record("spawn execution evidence has no claim command id"))?;
        let record = self
            .records
            .get(command_id)
            .ok_or_else(|| SpawnClaimError::UnknownClaim(command_id.clone()))?;
        let evidence_lsn = self
            .applied_through_lsn
            .checked_add(1)
            .ok_or_else(|| corrupt_log("spawn evidence LSN overflow"))?;
        validate_execution_evidence_contract(
            &self.applied_events,
            &self.authority_domain_id,
            record,
            evidence,
            evidence_lsn,
        )
    }

    #[must_use]
    pub fn prior_work_effects(&self, command_id: &CommandId) -> &[SpawnPriorWorkEffect] {
        self.prior_work_effects
            .get(command_id)
            .map_or(&[], Vec::as_slice)
    }

    /// Completion readiness requires the complete generic phase spine and the
    /// exact staged runtime. Result/lifecycle/authority remain separate
    /// prerequisites owned by the promotion producer and completion driver.
    #[must_use]
    pub fn completion_phases_ready(
        &self,
        command_id: &CommandId,
        runtime: &RuntimeGenerationRef,
    ) -> bool {
        self.records.get(command_id).is_some_and(|record| {
            self.completion_phases
                .get(command_id)
                .is_some_and(|phases| phases.is_ready(runtime, record.latest_disposition_lsn))
        })
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
        } else if kind == StoredEventKind::SpawnExecutionEvidence {
            let evidence = SpawnExecutionEvidence::decode(event.payload.payload.as_slice())
                .map_err(|error| {
                    corrupt_record(format!(
                        "cannot decode spawn execution evidence at LSN {event_lsn}: {error}"
                    ))
                })?;
            next.apply_execution_evidence(evidence, event_lsn)?;
        } else if kind == StoredEventKind::SpawnSuccessorEvidenceStaged {
            let staged = SpawnSuccessorEvidenceStaged::decode(event.payload.payload.as_slice())
                .map_err(|error| {
                    corrupt_record(format!(
                        "cannot decode staged spawn successor at LSN {event_lsn}: {error}"
                    ))
                })?;
            next.apply_staged_successor(staged, event_lsn)?;
        } else if kind == StoredEventKind::SpawnPromotionCommitted {
            let promotion = SpawnPromotionCommitted::decode(event.payload.payload.as_slice())
                .map_err(|error| {
                    corrupt_record(format!(
                        "cannot decode spawn promotion at LSN {event_lsn}: {error}"
                    ))
                })?;
            next.apply_promotion(promotion, &event.event_id, event_lsn)?;
        }
        next.applied_through_lsn = event_lsn;
        next.applied_events.insert(event_lsn, event.payload.clone());
        *self = next;
        Ok(())
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
        validate_spawn_claim_accepted(&self.authority_domain_id, &accepted)?;
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
        let adapter_id = accepted_adapter_id(
            accepted
                .accepted_operation
                .as_ref()
                .expect("accepted operation validated"),
        )?
        .clone();
        let pending_replacement = accepted
            .pending_replacement
            .as_ref()
            .and_then(|fence| fence.exact_prior.clone());
        self.exclusive_claims.insert(key, command_id.clone());
        self.prior_work_effects
            .insert(command_id.clone(), accepted.prior_work_effects);
        self.completion_phases
            .insert(command_id.clone(), SpawnCompletionPhaseRecord::default());
        self.records.insert(
            command_id,
            SpawnClaimRecord {
                claim,
                accepted_lsn: event_lsn,
                compound_authority: accepted.compound_authority,
                disposition: SpawnClaimDisposition::Active,
                pending_replacement,
                latest_disposition_lsn: event_lsn,
                adapter_id,
            },
        );
        Ok(())
    }

    fn apply_execution_evidence(
        &mut self,
        evidence: SpawnExecutionEvidence,
        event_lsn: u64,
    ) -> Result<(), SpawnClaimError> {
        let command_id = evidence
            .exact_claim
            .as_ref()
            .and_then(|claim| claim.claim_operation_id.as_ref())
            .filter(|id| !id.value.is_empty())
            .ok_or_else(|| corrupt_record("spawn execution evidence has no claim command id"))?
            .clone();
        let Some(record) = self.records.get(&command_id).cloned() else {
            // Pre-acceptance or another claim's evidence is durable diagnostic
            // input only. It cannot become disposition or runtime-ownership
            // authority if a matching claim appears later.
            return Ok(());
        };
        if !matches!(
            record.disposition,
            SpawnClaimDisposition::Active | SpawnClaimDisposition::PoisonedPendingReconciliation
        ) {
            // Exact retries are reconciled before append. Historical evidence
            // after a terminal claim remains inert rather than reactivating it.
            return Ok(());
        }
        if validate_execution_evidence_contract(
            &self.applied_events,
            &self.authority_domain_id,
            &record,
            &evidence,
            event_lsn,
        )
        .is_err()
        {
            // Only a later disposition event can consume evidence. Keeping an
            // invalid candidate inert preserves the Leaf-5 diagnostic contract
            // while the authenticated ingress rejects it before new durability.
            return Ok(());
        }
        let effect = required_external_effect_disposition(evidence.external_effect_disposition)?;
        if effect == ExternalEffectDisposition::Identified {
            let runtime = evidence
                .external_runtime
                .as_ref()
                .expect("identified evidence contract validated");
            self.reserve_identified_runtime(&command_id, runtime)?;
            let phase = required_execution_phase(evidence.phase)?;
            let failure = FailureCode::try_from(evidence.failure_code)
                .expect("execution evidence contract validated failure");
            if failure == FailureCode::Unspecified
                && matches!(
                    phase,
                    SpawnExecutionPhase::ExternalIdentityKnown
                        | SpawnExecutionPhase::HandshakeReconciling
                        | SpawnExecutionPhase::SuccessEvidenceReported
                )
            {
                self.completion_phases
                    .get_mut(&command_id)
                    .expect("accepted claim initialized completion phases")
                    .observe_progress(phase, runtime.clone(), event_lsn);
            }
        }
        Ok(())
    }

    fn apply_staged_successor(
        &mut self,
        staged: SpawnSuccessorEvidenceStaged,
        event_lsn: u64,
    ) -> Result<(), SpawnClaimError> {
        super::validate_staged_successor(&staged)
            .map_err(|error| corrupt_log(error.to_string()))?;
        let claim = staged
            .exact_claim
            .as_ref()
            .expect("staged successor validated");
        let command_id = claim
            .claim_operation_id
            .as_ref()
            .expect("staged successor validated");
        let record = self
            .records
            .get(command_id)
            .ok_or_else(|| SpawnClaimError::UnknownClaim(command_id.clone()))?;
        if record.claim != *claim
            || !matches!(
                record.disposition,
                SpawnClaimDisposition::Active
                    | SpawnClaimDisposition::PoisonedPendingReconciliation
            )
        {
            return Err(corrupt_log(
                "staged successor does not belong to its exact active/poisoned claim",
            ));
        }
        let runtime = staged
            .classified_target
            .as_ref()
            .expect("staged successor validated")
            .clone();
        self.reserve_identified_runtime(command_id, &runtime)?;
        self.completion_phases
            .get_mut(command_id)
            .expect("accepted claim initialized completion phases")
            .observe_staged(runtime, event_lsn);
        Ok(())
    }

    fn reserve_identified_runtime(
        &mut self,
        command_id: &CommandId,
        runtime: &RuntimeGenerationRef,
    ) -> Result<(), SpawnClaimError> {
        let record = self
            .records
            .get(command_id)
            .ok_or_else(|| SpawnClaimError::UnknownClaim(command_id.clone()))?;
        validate_promoted_runtime(&self.authority_domain_id, &record.claim, runtime)?;
        if let Some(existing) = self.identified_runtime_by_claim.get(command_id) {
            return if existing == runtime {
                Ok(())
            } else {
                Err(SpawnClaimError::ClaimRuntimeConflict {
                    command_id: command_id.clone(),
                })
            };
        }
        let external = runtime
            .external_runtime
            .as_ref()
            .expect("identified runtime validated");
        let key = external_runtime_key(&self.authority_domain_id, external)?;
        if let Some(owner) = self.external_runtime_claims.get(&key) {
            if owner != command_id {
                return Err(SpawnClaimError::ExternalRuntimeOwnershipConflict {
                    owner: owner.clone(),
                    attempted: command_id.clone(),
                });
            }
        }
        self.external_runtime_claims.insert(key, command_id.clone());
        self.identified_runtime_by_claim
            .insert(command_id.clone(), runtime.clone());
        Ok(())
    }

    fn apply_promotion(
        &mut self,
        promotion: SpawnPromotionCommitted,
        event_id: &EventId,
        event_lsn: u64,
    ) -> Result<(), SpawnClaimError> {
        super::validate_spawn_promotion_result_order(&promotion)
            .map_err(|error| corrupt_log(error.to_string()))?;
        super::validate_spawn_promotion_envelope(&promotion, event_id)
            .map_err(|error| corrupt_log(error.to_string()))?;
        let accepted = promotion
            .accepted_claim
            .as_ref()
            .expect("promotion envelope validated");
        let claim = accepted
            .claim
            .as_ref()
            .expect("promotion envelope validated");
        let command_id = claim
            .claim_operation_id
            .as_ref()
            .expect("promotion envelope validated")
            .clone();
        let record = self
            .records
            .get(&command_id)
            .ok_or_else(|| SpawnClaimError::UnknownClaim(command_id.clone()))?;
        if record.claim != *claim
            || !matches!(
                record.disposition,
                SpawnClaimDisposition::Active
                    | SpawnClaimDisposition::PoisonedPendingReconciliation
            )
        {
            return Err(corrupt_log(
                "promotion does not consume the exact active/poisoned claim pre-state",
            ));
        }
        validate_embedded_promotion_history(
            &self.applied_events,
            &self.authority_domain_id,
            record,
            &promotion,
            event_lsn,
        )?;
        let promoted_runtime = promotion
            .promoted_runtime
            .as_ref()
            .expect("promotion envelope validated");
        validate_promoted_runtime(&self.authority_domain_id, &record.claim, promoted_runtime)?;
        if !self.completion_phases_ready(&command_id, promoted_runtime) {
            return Err(corrupt_log(
                "promotion omits the ordered identity/handshake/staged/success phase spine",
            ));
        }
        let key = claim_key(&record.claim)?;
        if self.exclusive_claims.get(&key) != Some(&command_id) {
            return Err(corrupt_log("promoted claim did not own its exclusive key"));
        }
        let record = self.records.get_mut(&command_id).expect("claim validated");
        record.disposition = SpawnClaimDisposition::Promoted;
        record.pending_replacement = None;
        record.latest_disposition_lsn = event_lsn;
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
            &self.applied_events,
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
        record.latest_disposition_lsn = event_lsn;
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
            if let Some(runtime) = self.identified_runtime_by_claim.remove(&command_id) {
                let external = runtime
                    .external_runtime
                    .as_ref()
                    .expect("stored identified runtime was validated");
                let runtime_key = external_runtime_key(&self.authority_domain_id, external)?;
                if self.external_runtime_claims.get(&runtime_key) != Some(&command_id) {
                    return Err(corrupt_log(
                        "released claim did not own its identified external runtime",
                    ));
                }
                self.external_runtime_claims.remove(&runtime_key);
            }
        }
        Ok(())
    }
}

impl SpawnClaimQuery for SpawnClaimRegistry {
    fn claim_for_operation(&self, command_id: &CommandId) -> Option<&SpawnClaimRecord> {
        self.records.get(command_id)
    }

    fn claim_for_external_runtime(
        &self,
        runtime: &RuntimeGenerationRef,
    ) -> Option<&SpawnClaimRecord> {
        let external = runtime.external_runtime.as_ref()?;
        let key = external_runtime_key(&self.authority_domain_id, external).ok()?;
        self.external_runtime_claims
            .get(&key)
            .and_then(|command_id| self.records.get(command_id))
    }

    fn identified_runtime_for_operation(
        &self,
        command_id: &CommandId,
    ) -> Option<&RuntimeGenerationRef> {
        self.identified_runtime_by_claim.get(command_id)
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
        self.delivery_fence_matching(|prior| prior == runtime)
    }

    fn delivery_fence_for_target_scope(&self, scope: &TargetScope) -> SpawnDeliveryFence {
        self.delivery_fence_matching(|prior| runtime_ref_matches_target_scope(prior, scope))
    }
}

impl SpawnClaimRegistry {
    fn delivery_fence_matching(
        &self,
        matches_prior: impl Fn(&RuntimeGenerationRef) -> bool,
    ) -> SpawnDeliveryFence {
        let Some((command_id, _)) = self.records.iter().find(|(command_id, record)| {
            record
                .pending_replacement
                .as_ref()
                .is_some_and(&matches_prior)
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

/// Whether a runtime-targeted Operation names the exact external half of a
/// logical runtime-generation reference. `TargetScope` deliberately does not
/// duplicate `LogicalTargetId`; the active claim supplies that stable identity.
/// All external identity dimensions still match exactly.
#[must_use]
pub fn runtime_ref_matches_target_scope(
    runtime: &RuntimeGenerationRef,
    scope: &TargetScope,
) -> bool {
    let Some(external) = runtime.external_runtime.as_ref() else {
        return false;
    };
    TargetScopeKind::try_from(scope.kind).ok() == Some(TargetScopeKind::RuntimeSession)
        && scope.adapter_id == external.adapter_id
        && scope.deployment_scope == external.deployment_scope
        && scope.runtime_session_id == external.runtime_session_id
        && scope.session_generation == external.generation
        && scope.actor_id.is_none()
        && scope.resource.is_none()
        && scope.project_or_group.is_empty()
        && scope.legacy_audit_resource_id.is_empty()
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

/// Encode exact claim-correlated execution evidence in its schema-owned
/// durable discriminator. The claim projection still validates it before any
/// disposition transition can consume it.
#[must_use]
pub fn encode_spawn_execution_evidence(event: &SpawnExecutionEvidence) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::SpawnExecutionEvidence as i32,
        payload: event.encode_to_vec(),
    }
}

/// Validate one complete accepted-spawn envelope at durable and delivery boundaries.
///
/// This binds the untrusted nested request intent to the core-prepared claim,
/// fence, and two-Grant authority carriage so no self-consistent claim can
/// disagree with the Operation that an adapter will execute.
pub fn validate_spawn_claim_accepted(
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
    let request = validate_spawn_operation_payload(operation).map_err(|error| {
        corrupt_log(format!(
            "accepted spawn operation payload is not canonical: {error}"
        ))
    })?;
    validate_spawn_authority_carriage(
        &request,
        accepted_operation.authorizing_grant_id.as_ref(),
        accepted.compound_authority.as_ref(),
    )
    .map_err(|error| {
        corrupt_log(format!(
            "accepted spawn authority carriage is not canonical: {error}"
        ))
    })?;
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
    accepted_adapter_id(accepted)?;
    Ok(())
}

fn accepted_adapter_id(accepted: &AcceptedOperation) -> Result<&AdapterId, SpawnClaimError> {
    let scope = accepted
        .operation
        .as_ref()
        .and_then(|operation| operation.target_scope.as_ref())
        .ok_or_else(|| corrupt_record("accepted spawn operation has no adapter target"))?;
    if TargetScopeKind::try_from(scope.kind).ok() != Some(TargetScopeKind::Adapter)
        || scope
            .adapter_id
            .as_ref()
            .is_none_or(|id| id.value.is_empty())
        || scope.actor_id.is_some()
        || scope.runtime_session_id.is_some()
        || scope.session_generation.is_some()
        || !scope.deployment_scope.is_empty()
        || !scope.project_or_group.is_empty()
        || !scope.legacy_audit_resource_id.is_empty()
        || scope.resource.is_some()
    {
        return Err(corrupt_log(
            "accepted spawn operation target is not one exact adapter",
        ));
    }
    Ok(scope.adapter_id.as_ref().expect("validated adapter target"))
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
    events: &BTreeMap<u64, StoredEventPayload>,
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
        ) => validate_no_effect_release(events, domain, record, release, event_lsn),
        (
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            Some(spawn_claim_disposition_changed::Evidence::AmbiguousExternalEffect(ambiguity)),
        ) => {
            let (_, typed) = validate_typed_execution_evidence(
                events,
                domain,
                record,
                ambiguity.evidence_event_id.as_ref(),
                event_lsn,
                "external-effect evidence",
            )?;
            let phase = required_execution_phase(typed.phase)?;
            let disposition =
                required_external_effect_disposition(typed.external_effect_disposition)?;
            let failure = FailureCode::try_from(typed.failure_code)
                .map_err(|_| corrupt_record("spawn execution evidence failure code is unknown"))?;
            if !execution_evidence_poisons_claim(phase, disposition, failure) {
                return Err(corrupt_log(
                    "claim poison requires phase-aware ambiguous or identified external-effect evidence",
                ));
            }
            Ok(())
        }
        (
            SpawnClaimDisposition::TargetAbandoned,
            Some(spawn_claim_disposition_changed::Evidence::TargetAbandonment(abandonment)),
        ) => validate_referenced_event(
            events,
            domain,
            record.accepted_lsn,
            abandonment.abandonment_event_id.as_ref(),
            event_lsn,
            "target-abandonment evidence",
        )
        .map(|_| ()),
        _ => Err(corrupt_log(
            "claim disposition is not paired with its exact closed-vocabulary evidence",
        )),
    }
}

fn validate_embedded_promotion_history(
    events: &BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    promotion: &SpawnPromotionCommitted,
    promotion_lsn: u64,
) -> Result<(), SpawnClaimError> {
    let accepted_id = promotion
        .accepted_claim_event_id
        .as_ref()
        .expect("promotion envelope validated accepted id");
    let accepted_payload = validate_referenced_event(
        events,
        domain,
        0,
        Some(accepted_id),
        promotion_lsn,
        "promotion accepted claim",
    )?;
    if StoredEventKind::try_from(accepted_payload.kind).ok() != Some(StoredEventKind::SpawnClaim) {
        return Err(corrupt_log(
            "promotion accepted-claim reference is not SpawnClaim",
        ));
    }
    let accepted_event =
        SpawnClaimEvent::decode(accepted_payload.payload.as_slice()).map_err(|error| {
            corrupt_record(format!("cannot decode promotion accepted claim: {error}"))
        })?;
    if accepted_event.authority_domain_id.as_ref() != Some(domain)
        || accepted_event.mutation
            != Some(spawn_claim_event::Mutation::Accepted(
                promotion
                    .accepted_claim
                    .clone()
                    .expect("promotion envelope validated"),
            ))
        || accepted_id
            .lsn
            .is_none_or(|lsn| lsn.value != record.accepted_lsn)
    {
        return Err(corrupt_log(
            "promotion accepted claim bytes/pre-state do not match",
        ));
    }

    let mut latest_lifecycle_lsn = 0;
    for evidence in &promotion.lifecycle {
        let id = evidence
            .event_id
            .as_ref()
            .expect("promotion envelope validated lifecycle id");
        let payload = validate_referenced_event(
            events,
            domain,
            record.accepted_lsn,
            Some(id),
            promotion_lsn,
            "promotion lifecycle evidence",
        )?;
        if StoredEventKind::try_from(payload.kind).ok() != Some(StoredEventKind::CommandTransition)
            || patchbay_contracts::patchbay::CommandTransition::decode(payload.payload.as_slice())
                .ok()
                .as_ref()
                != evidence.transition.as_ref()
        {
            return Err(corrupt_log(
                "promotion lifecycle reference bytes do not match",
            ));
        }
        latest_lifecycle_lsn = id
            .lsn
            .as_ref()
            .expect("referenced event id validated")
            .value;
    }

    let result = promotion
        .successful_result
        .as_ref()
        .expect("promotion envelope validated result");
    let result_payload = validate_referenced_event(
        events,
        domain,
        record.accepted_lsn,
        result.event_id.as_ref(),
        promotion_lsn,
        "promotion successful result",
    )?;
    if StoredEventKind::try_from(result_payload.kind).ok() != Some(StoredEventKind::Observation) {
        return Err(corrupt_log(
            "promotion result reference is not an Observation",
        ));
    }
    let observation = Observation::decode(result_payload.payload.as_slice())
        .map_err(|error| corrupt_record(format!("cannot decode promotion result: {error}")))?;
    if result
        .event_id
        .as_ref()
        .and_then(|id| id.lsn)
        .is_none_or(|lsn| lsn.value <= latest_lifecycle_lsn)
        || ObservationKind::try_from(observation.kind).ok() != Some(ObservationKind::Result)
        || FailureCode::try_from(observation.failure_code).ok() != Some(FailureCode::Unspecified)
        || observation.target_scope != result.target_scope
        || observation.observed_at != result.observed_at
        || exact_command_correlation(&observation.correlations).as_ref()
            != result.command_id.as_ref()
    {
        return Err(corrupt_log(
            "promotion result does not match exact successful Observation",
        ));
    }

    let staged = promotion
        .staged_successor
        .as_ref()
        .expect("promotion envelope validated staged");
    let staged_payload = validate_referenced_event(
        events,
        domain,
        record.accepted_lsn,
        staged.event_id.as_ref(),
        promotion_lsn,
        "promotion staged successor",
    )?;
    if StoredEventKind::try_from(staged_payload.kind).ok()
        != Some(StoredEventKind::SpawnSuccessorEvidenceStaged)
        || SpawnSuccessorEvidenceStaged::decode(staged_payload.payload.as_slice())
            .ok()
            .as_ref()
            != staged.staged.as_ref()
    {
        return Err(corrupt_log(
            "promotion staged-successor reference bytes do not match",
        ));
    }
    Ok(())
}

fn validate_no_effect_release(
    events: &BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    release: &patchbay_contracts::patchbay::SpawnClaimNoEffectRelease,
    event_lsn: u64,
) -> Result<(), SpawnClaimError> {
    let (evidence_lsn, typed) = validate_typed_execution_evidence(
        events,
        domain,
        record,
        release.evidence_event_id.as_ref(),
        event_lsn,
        "no-external-effect evidence",
    )?;
    if required_external_effect_disposition(typed.external_effect_disposition)?
        != ExternalEffectDisposition::ProvedNone
    {
        return Err(corrupt_log(
            "release evidence does not prove absence of external effect",
        ));
    }
    if evidence_lsn <= record.latest_disposition_lsn {
        return Err(corrupt_log(
            "no-effect proof does not postdate the latest claim disposition",
        ));
    }
    validate_no_effect_proof(events, domain, record, &typed, evidence_lsn)?;
    reject_later_external_effect_evidence(events, record, evidence_lsn, event_lsn)?;

    match record.claim.expected_prior.as_ref() {
        Some(prior) => {
            if release.exact_prior_liveness.as_ref() != Some(prior) {
                return Err(corrupt_log(
                    "continuation release lacks exact prior-N liveness",
                ));
            }
            validate_prior_liveness(
                events,
                domain,
                evidence_lsn,
                prior,
                release.prior_liveness_event_id.as_ref(),
                event_lsn,
            )?;
        }
        None if release.exact_prior_liveness.is_none()
            && release.prior_liveness_event_id.is_none() => {}
        None => {
            return Err(corrupt_log(
                "fresh claim release carries unrelated prior-N liveness evidence",
            ));
        }
    }
    Ok(())
}

fn validate_typed_execution_evidence(
    events: &BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    event_id: Option<&EventId>,
    current_lsn: u64,
    name: &str,
) -> Result<(u64, SpawnExecutionEvidence), SpawnClaimError> {
    let referenced = validate_referenced_event(
        events,
        domain,
        record.accepted_lsn,
        event_id,
        current_lsn,
        name,
    )?;
    if StoredEventKind::try_from(referenced.kind).ok()
        != Some(StoredEventKind::SpawnExecutionEvidence)
    {
        return Err(corrupt_log(format!(
            "{name} does not reference SpawnExecutionEvidence"
        )));
    }
    let typed = SpawnExecutionEvidence::decode(referenced.payload.as_slice())
        .map_err(|error| corrupt_record(format!("cannot decode {name}: {error}")))?;
    let evidence_lsn = event_id
        .and_then(|id| id.lsn)
        .expect("referenced event id validated")
        .value;
    validate_execution_evidence_contract(events, domain, record, &typed, evidence_lsn)?;
    Ok((evidence_lsn, typed))
}

/// Validate one typed evidence event against its exact durable claim. Adapter
/// ingress and replay use the same fail-closed contract.
pub fn validate_execution_evidence_contract(
    events: &BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    evidence: &SpawnExecutionEvidence,
    evidence_lsn: u64,
) -> Result<(), SpawnClaimError> {
    if evidence.authority_domain_id.as_ref() != Some(domain)
        || evidence.exact_claim.as_ref() != Some(&record.claim)
        || evidence_lsn < record.accepted_lsn
    {
        return Err(corrupt_log(
            "spawn execution evidence is not correlated to the exact accepted claim",
        ));
    }
    let phase = required_execution_phase(evidence.phase)?;
    let disposition = required_external_effect_disposition(evidence.external_effect_disposition)?;
    if !allowed_external_effect_disposition(phase, disposition) {
        return Err(corrupt_log(
            "spawn execution phase and external-effect disposition conflict",
        ));
    }
    if record.claim.expected_prior.is_none()
        && matches!(
            phase,
            SpawnExecutionPhase::QuiescingPrior | SpawnExecutionPhase::PriorTerminated
        )
    {
        return Err(corrupt_log(
            "fresh spawn evidence names a prior-runtime phase",
        ));
    }

    let source = evidence
        .source_attachment
        .as_ref()
        .ok_or_else(|| corrupt_record("spawn execution evidence has no attachment provenance"))?;
    let source_adapter = source
        .adapter_id
        .as_ref()
        .filter(|adapter| !adapter.value.is_empty())
        .ok_or_else(|| corrupt_record("evidence attachment has no adapter id"))?;
    let source_generation = source
        .adapter_generation
        .filter(|generation| generation.value > 0)
        .ok_or_else(|| corrupt_record("evidence attachment has no positive generation"))?;
    if source_adapter != &record.adapter_id {
        return Err(corrupt_log("evidence came from another claim adapter"));
    }
    validate_current_attachment(
        events,
        domain,
        source_adapter,
        source_generation.value,
        source.attachment_event_id.as_ref(),
    )?;

    let producer = SpawnExecutionEvidenceProducer::try_from(evidence.producer)
        .map_err(|_| corrupt_record("spawn execution evidence producer is unknown"))?;
    if producer == SpawnExecutionEvidenceProducer::Unspecified {
        return Err(corrupt_record(
            "spawn execution evidence producer is unspecified",
        ));
    }
    let failure = FailureCode::try_from(evidence.failure_code)
        .map_err(|_| corrupt_record("spawn execution evidence failure code is unknown"))?;

    match disposition {
        ExternalEffectDisposition::ProvedNone => {
            if evidence.external_runtime.is_some() || evidence.no_external_effect_proof.is_none() {
                return Err(corrupt_log(
                    "proved-none evidence must carry one proof and no external runtime",
                ));
            }
            if failure == FailureCode::Unspecified {
                return Err(corrupt_record(
                    "proved-none evidence has unspecified failure code",
                ));
            }
            if failure == FailureCode::ExecutionOutcomeUnknown {
                return Err(corrupt_log(
                    "execution_outcome_unknown can never prove absence of external effect",
                ));
            }
        }
        ExternalEffectDisposition::MayExist => {
            if evidence.no_external_effect_proof.is_some() || !poison_failure(evidence.failure_code)
            {
                return Err(corrupt_log(
                    "effect-may-exist evidence must carry an ambiguity failure and no no-effect proof",
                ));
            }
            if let Some(runtime) = evidence.external_runtime.as_ref() {
                validate_evidence_runtime(domain, record, source_adapter, runtime)?;
            }
        }
        ExternalEffectDisposition::Identified => {
            if evidence.no_external_effect_proof.is_some() {
                return Err(corrupt_log(
                    "identified external effect carries a no-effect proof",
                ));
            }
            validate_evidence_runtime(
                domain,
                record,
                source_adapter,
                evidence.external_runtime.as_ref().ok_or_else(|| {
                    corrupt_record("identified external effect has no external runtime")
                })?,
            )?;
        }
        ExternalEffectDisposition::Unspecified => unreachable!("rejected above"),
    }
    Ok(())
}

#[must_use]
pub const fn allowed_external_effect_disposition(
    phase: SpawnExecutionPhase,
    disposition: ExternalEffectDisposition,
) -> bool {
    use ExternalEffectDisposition::{Identified, MayExist, ProvedNone};
    use SpawnExecutionPhase::{
        AcceptedNotOffered, ExternalIdentityKnown, HandshakeReconciling, LaunchAttempted, Offered,
        PriorTerminated, QuiescingPrior, SuccessEvidenceReported,
    };
    match phase {
        AcceptedNotOffered => matches!(disposition, ProvedNone),
        Offered | QuiescingPrior | PriorTerminated => matches!(disposition, ProvedNone | MayExist),
        LaunchAttempted => matches!(disposition, MayExist | Identified),
        ExternalIdentityKnown | HandshakeReconciling | SuccessEvidenceReported => {
            matches!(disposition, Identified)
        }
        SpawnExecutionPhase::Unspecified => false,
    }
}

/// Whether one valid phase/disposition/failure cell requires the exact claim to
/// become poisoned. Launch-attempted identity is ambiguous even without a
/// failure code; after identity is durably known, only failure evidence poisons.
#[must_use]
pub(crate) fn execution_evidence_poisons_claim(
    phase: SpawnExecutionPhase,
    disposition: ExternalEffectDisposition,
    failure: FailureCode,
) -> bool {
    phase_outcome(
        phase,
        disposition,
        failure,
        matches!(
            phase,
            SpawnExecutionPhase::QuiescingPrior | SpawnExecutionPhase::PriorTerminated
        ),
    )
    .is_ok_and(|outcome| outcome.claim == super::spawn_orchestration::ClaimFenceOutcome::Poisoned)
}

fn validate_no_effect_proof(
    events: &BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    evidence: &SpawnExecutionEvidence,
    evidence_lsn: u64,
) -> Result<(), SpawnClaimError> {
    let phase = required_execution_phase(evidence.phase)?;
    let producer = SpawnExecutionEvidenceProducer::try_from(evidence.producer)
        .map_err(|_| corrupt_record("spawn execution evidence producer is unknown"))?;
    let source = evidence
        .source_attachment
        .as_ref()
        .expect("execution evidence contract validated source");
    let proof = evidence
        .no_external_effect_proof
        .as_ref()
        .expect("execution evidence contract validated proof");
    match proof
        .proof
        .as_ref()
        .ok_or_else(|| corrupt_record("no-external-effect proof has no variant"))?
    {
        no_external_effect_proof::Proof::CorePreDeliveryTerminal(core) => {
            if producer != SpawnExecutionEvidenceProducer::Core
                || phase != SpawnExecutionPhase::AcceptedNotOffered
            {
                return Err(corrupt_log(
                    "core no-effect proof is not an atomic accepted-before-offer terminal decision",
                ));
            }
            validate_core_pre_delivery_terminal_decision(
                events,
                domain,
                record,
                core.terminal_decision_event_id.as_ref(),
                evidence_lsn,
                FailureCode::try_from(evidence.failure_code)
                    .expect("execution evidence failure validated"),
            )?;
        }
        no_external_effect_proof::Proof::AuthenticatedAdapterRefusalBeforeDelivery(adapter) => {
            validate_adapter_proof_source(
                adapter.adapter_id.as_ref(),
                adapter.adapter_generation,
                source,
            )?;
            if producer != SpawnExecutionEvidenceProducer::CurrentAdapter
                || phase != SpawnExecutionPhase::Offered
                || !matches!(
                    FailureCode::try_from(evidence.failure_code).ok(),
                    Some(
                        FailureCode::UnsupportedCommand
                            | FailureCode::TargetOffline
                            | FailureCode::DeliveryRejected
                    )
                )
                || delivered_responsibility_exists(events, record, evidence_lsn)?
            {
                return Err(corrupt_log(
                    "adapter refusal proof is not explicitly before delivery responsibility",
                ));
            }
        }
        no_external_effect_proof::Proof::ExactSupervisorPreLaunchFailure(supervisor) => {
            validate_adapter_proof_source(
                supervisor.adapter_id.as_ref(),
                supervisor.adapter_generation,
                source,
            )?;
            if producer != SpawnExecutionEvidenceProducer::CurrentAdapter
                || !matches!(
                    phase,
                    SpawnExecutionPhase::Offered
                        | SpawnExecutionPhase::QuiescingPrior
                        | SpawnExecutionPhase::PriorTerminated
                )
                || !matches!(
                    FailureCode::try_from(evidence.failure_code).ok(),
                    Some(
                        FailureCode::ExecutionFailed
                            | FailureCode::Cancelled
                            | FailureCode::Expired
                    )
                )
            {
                return Err(corrupt_log(
                    "supervisor/journal proof does not establish exact-claim pre-launch failure",
                ));
            }
        }
    }
    Ok(())
}

fn validate_core_pre_delivery_terminal_decision(
    events: &BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    decision_event_id: Option<&EventId>,
    evidence_lsn: u64,
    evidence_failure: FailureCode,
) -> Result<(), SpawnClaimError> {
    let decision = validate_referenced_event(
        events,
        domain,
        record.accepted_lsn,
        decision_event_id,
        evidence_lsn,
        "core pre-delivery terminal decision",
    )?;
    if StoredEventKind::try_from(decision.kind).ok() != Some(StoredEventKind::CommandTransition) {
        return Err(corrupt_log(
            "core pre-delivery terminal proof does not reference a command transition",
        ));
    }
    let transition =
        patchbay_contracts::patchbay::CommandTransition::decode(decision.payload.as_slice())
            .map_err(|error| {
                corrupt_record(format!(
                    "cannot decode core pre-delivery terminal decision: {error}"
                ))
            })?;
    let to = required_operation_state(transition.to_state, "core terminal decision target")?;
    let from = required_operation_state(transition.from_state, "core terminal decision source")?;
    let failure = FailureCode::try_from(transition.failure_code)
        .map_err(|_| corrupt_record("core terminal decision failure code is unknown"))?;
    let safe_terminal = matches!(
        (to, failure),
        (
            OperationState::Rejected,
            FailureCode::TargetNotFound | FailureCode::UnsupportedCommand
        ) | (
            OperationState::Failed,
            FailureCode::TargetOffline
                | FailureCode::AdapterUnavailable
                | FailureCode::TransportTimeout
        ) | (OperationState::Expired, FailureCode::Expired)
            | (OperationState::Cancelled, FailureCode::Cancelled)
            | (OperationState::Superseded, FailureCode::Superseded)
    );
    let decision_lsn = decision_event_id
        .and_then(|id| id.lsn)
        .expect("referenced core decision id validated")
        .value;
    if transition.command_id.as_ref() != record.claim.claim_operation_id.as_ref()
        || from != OperationState::Accepted
        || failure != evidence_failure
        || !safe_terminal
        || delivered_responsibility_exists(events, record, decision_lsn)?
    {
        return Err(corrupt_log(
            "core no-effect proof does not reference the exact safe accepted-before-offer terminal decision",
        ));
    }
    Ok(())
}

fn reject_later_external_effect_evidence(
    events: &BTreeMap<u64, StoredEventPayload>,
    record: &SpawnClaimRecord,
    proof_lsn: u64,
    release_lsn: u64,
) -> Result<(), SpawnClaimError> {
    let first_later = proof_lsn
        .checked_add(1)
        .ok_or_else(|| corrupt_log("no-effect proof LSN overflow"))?;
    for (&lsn, payload) in events.range(first_later..release_lsn) {
        match StoredEventKind::try_from(payload.kind).ok() {
            Some(StoredEventKind::CommandTransition) => {
                let transition = patchbay_contracts::patchbay::CommandTransition::decode(
                    payload.payload.as_slice(),
                )
                .map_err(|error| {
                    corrupt_record(format!(
                        "cannot decode command transition at LSN {lsn} while validating release: {error}"
                    ))
                })?;
                if transition.command_id.as_ref() == record.claim.claim_operation_id.as_ref()
                    && (matches!(
                        OperationState::try_from(transition.to_state).ok(),
                        Some(
                            OperationState::Delivered
                                | OperationState::Running
                                | OperationState::Completed
                        )
                    ) || matches!(
                        FailureCode::try_from(transition.failure_code).ok(),
                        Some(FailureCode::ExecutionFailed | FailureCode::ExecutionOutcomeUnknown)
                    ))
                {
                    return Err(corrupt_log(
                        "later command delivery or execution evidence contradicts no-effect proof",
                    ));
                }
            }
            Some(StoredEventKind::SpawnExecutionEvidence) => {
                let evidence = SpawnExecutionEvidence::decode(payload.payload.as_slice())
                    .map_err(|error| {
                        corrupt_record(format!(
                            "cannot decode spawn execution evidence at LSN {lsn} while validating release: {error}"
                        ))
                    })?;
                if evidence.exact_claim.as_ref() == Some(&record.claim)
                    && (matches!(
                        SpawnExecutionPhase::try_from(evidence.phase).ok(),
                        Some(
                            SpawnExecutionPhase::LaunchAttempted
                                | SpawnExecutionPhase::ExternalIdentityKnown
                                | SpawnExecutionPhase::HandshakeReconciling
                                | SpawnExecutionPhase::SuccessEvidenceReported
                        )
                    ) || matches!(
                        ExternalEffectDisposition::try_from(evidence.external_effect_disposition)
                            .ok(),
                        Some(
                            ExternalEffectDisposition::MayExist
                                | ExternalEffectDisposition::Identified
                        )
                    ) || evidence.external_runtime.is_some()
                        || matches!(
                            FailureCode::try_from(evidence.failure_code).ok(),
                            Some(
                                FailureCode::ExecutionFailed | FailureCode::ExecutionOutcomeUnknown
                            )
                        ))
                {
                    return Err(corrupt_log(
                        "later spawn execution evidence contradicts no-effect proof",
                    ));
                }
            }
            Some(StoredEventKind::SpawnClaim) => {
                let event = SpawnClaimEvent::decode(payload.payload.as_slice()).map_err(|error| {
                    corrupt_record(format!(
                        "cannot decode spawn claim event at LSN {lsn} while validating release: {error}"
                    ))
                })?;
                if matches!(
                    event.mutation,
                    Some(spawn_claim_event::Mutation::DispositionChanged(
                        SpawnClaimDispositionChanged {
                            claim_operation_id: Some(ref command_id),
                            to_disposition,
                            ..
                        }
                    )) if Some(command_id) == record.claim.claim_operation_id.as_ref()
                        && SpawnClaimDisposition::try_from(to_disposition).ok()
                            == Some(SpawnClaimDisposition::PoisonedPendingReconciliation)
                ) {
                    return Err(corrupt_log(
                        "later poisoned claim disposition contradicts no-effect proof",
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_adapter_proof_source(
    adapter_id: Option<&AdapterId>,
    adapter_generation: Option<patchbay_contracts::patchbay::Generation>,
    source: &patchbay_contracts::patchbay::SpawnEvidenceAttachment,
) -> Result<(), SpawnClaimError> {
    if adapter_id != source.adapter_id.as_ref()
        || adapter_generation != source.adapter_generation
        || adapter_id.is_none_or(|id| id.value.is_empty())
        || adapter_generation.is_none_or(|generation| generation.value == 0)
    {
        return Err(corrupt_log(
            "no-effect proof does not match current authenticated attachment provenance",
        ));
    }
    Ok(())
}

fn validate_current_attachment(
    events: &BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    adapter_id: &AdapterId,
    adapter_generation: u64,
    attachment_event_id: Option<&EventId>,
) -> Result<(), SpawnClaimError> {
    let attachment_event_id = attachment_event_id
        .ok_or_else(|| corrupt_record("evidence source has no attachment event id"))?;
    let attachment_lsn = attachment_event_id
        .lsn
        .filter(|lsn| lsn.value > 0)
        .ok_or_else(|| corrupt_record("evidence source attachment has no positive LSN"))?
        .value;
    if attachment_event_id.authority_domain_id.as_ref() != Some(domain) {
        return Err(corrupt_log(
            "evidence source attachment belongs to another domain",
        ));
    }
    let mut adapters = AdapterRegistry::new();
    for (&lsn, payload) in events {
        adapters
            .observe(&RecordedEvent {
                event_id: EventId {
                    authority_domain_id: Some(domain.clone()),
                    lsn: Some(Lsn { value: lsn }),
                },
                payload: payload.clone(),
            })
            .map_err(|error| {
                corrupt_log(format!("cannot validate evidence attachment: {error}"))
            })?;
    }
    let current = adapters
        .get(adapter_id)
        .ok_or_else(|| corrupt_log("evidence adapter has no authenticated durable attachment"))?;
    if current
        .registration
        .adapter_generation
        .map(|generation| generation.value)
        != Some(adapter_generation)
        || current.attach_event_id.lsn.map(|lsn| lsn.value) != Some(attachment_lsn)
    {
        return Err(corrupt_log(
            "spawn execution evidence came from a stale adapter attachment",
        ));
    }
    Ok(())
}

fn validate_evidence_runtime(
    domain: &AuthorityDomainId,
    record: &SpawnClaimRecord,
    adapter_id: &AdapterId,
    runtime: &RuntimeGenerationRef,
) -> Result<(), SpawnClaimError> {
    validate_promoted_runtime(domain, &record.claim, runtime)?;
    if runtime
        .external_runtime
        .as_ref()
        .and_then(|external| external.adapter_id.as_ref())
        != Some(adapter_id)
    {
        return Err(corrupt_log(
            "external runtime is not owned by the current claim adapter",
        ));
    }
    Ok(())
}

fn delivered_responsibility_exists(
    events: &BTreeMap<u64, StoredEventPayload>,
    record: &SpawnClaimRecord,
    through_lsn: u64,
) -> Result<bool, SpawnClaimError> {
    for (&lsn, payload) in events.range(record.accepted_lsn..=through_lsn) {
        if StoredEventKind::try_from(payload.kind).ok() != Some(StoredEventKind::CommandTransition)
        {
            continue;
        }
        let transition =
            patchbay_contracts::patchbay::CommandTransition::decode(payload.payload.as_slice())
                .map_err(|error| {
                    corrupt_record(format!(
                        "cannot decode command transition at LSN {lsn}: {error}"
                    ))
                })?;
        if transition.command_id.as_ref() == record.claim.claim_operation_id.as_ref()
            && matches!(
                OperationState::try_from(transition.to_state).ok(),
                Some(
                    OperationState::Delivered | OperationState::Running | OperationState::Completed
                )
            )
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_prior_liveness(
    events: &BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    accepted_lsn: u64,
    prior: &RuntimeGenerationRef,
    event_id: Option<&EventId>,
    current_lsn: u64,
) -> Result<(), SpawnClaimError> {
    let payload = validate_referenced_event(
        events,
        domain,
        accepted_lsn,
        event_id,
        current_lsn,
        "prior-N liveness evidence",
    )?;
    if StoredEventKind::try_from(payload.kind).ok() != Some(StoredEventKind::SessionState) {
        return Err(corrupt_log(
            "prior-N liveness does not reference typed session-state evidence",
        ));
    }
    let state = SessionStateEvent::decode(payload.payload.as_slice())
        .map_err(|error| corrupt_record(format!("cannot decode prior-N liveness: {error}")))?;
    if state.authority_domain_id.as_ref() != Some(domain) {
        return Err(corrupt_log("prior-N liveness belongs to another domain"));
    }
    let external = prior
        .external_runtime
        .as_ref()
        .ok_or_else(|| corrupt_record("prior-N liveness target has no external runtime"))?;
    let is_live = match state.mutation.as_ref() {
        Some(session_state_event::Mutation::ConnectivityChanged(change)) => {
            change.adapter_id == external.adapter_id
                && change.deployment_scope == external.deployment_scope
                && change.runtime_session_id == external.runtime_session_id
                && change.session_generation == external.generation
                && SessionConnectivityState::try_from(change.to).ok()
                    == Some(SessionConnectivityState::Live)
        }
        Some(session_state_event::Mutation::ReportApplied(applied)) => {
            applied.report.as_ref().is_some_and(|report| {
                report.adapter_id == external.adapter_id
                    && report.deployment_scope == external.deployment_scope
                    && report.runtime_session_id == external.runtime_session_id
                    && report.session_generation == external.generation
                    && SessionConnectivityState::try_from(report.connectivity).ok()
                        == Some(SessionConnectivityState::Live)
            })
        }
        _ => false,
    };
    if !is_live {
        return Err(corrupt_log(
            "prior-N liveness evidence does not re-establish exact prior runtime as live",
        ));
    }
    Ok(())
}

fn poison_failure(raw: i32) -> bool {
    matches!(
        FailureCode::try_from(raw).ok(),
        Some(
            FailureCode::AdapterUnavailable
                | FailureCode::TransportTimeout
                | FailureCode::ExecutionFailed
                | FailureCode::Expired
                | FailureCode::Cancelled
                | FailureCode::ExecutionOutcomeUnknown
        )
    )
}

fn required_execution_phase(raw: i32) -> Result<SpawnExecutionPhase, SpawnClaimError> {
    let phase = SpawnExecutionPhase::try_from(raw)
        .map_err(|_| corrupt_record("spawn execution phase is unknown"))?;
    if phase == SpawnExecutionPhase::Unspecified {
        return Err(corrupt_record("spawn execution phase is unspecified"));
    }
    Ok(phase)
}

fn required_external_effect_disposition(
    raw: i32,
) -> Result<ExternalEffectDisposition, SpawnClaimError> {
    let disposition = ExternalEffectDisposition::try_from(raw)
        .map_err(|_| corrupt_record("external-effect disposition is unknown"))?;
    if disposition == ExternalEffectDisposition::Unspecified {
        return Err(corrupt_record("external-effect disposition is unspecified"));
    }
    Ok(disposition)
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

fn validate_referenced_event<'a>(
    events: &'a BTreeMap<u64, StoredEventPayload>,
    domain: &AuthorityDomainId,
    accepted_lsn: u64,
    event_id: Option<&EventId>,
    current_lsn: u64,
    name: &str,
) -> Result<&'a StoredEventPayload, SpawnClaimError> {
    let event_id = event_id.ok_or_else(|| corrupt_record(format!("{name} has no event id")))?;
    let referenced_lsn = event_id
        .lsn
        .as_ref()
        .filter(|lsn| lsn.value > 0)
        .ok_or_else(|| corrupt_record(format!("{name} has no positive event LSN")))?
        .value;
    if event_id.authority_domain_id.as_ref() != Some(domain)
        || referenced_lsn < accepted_lsn
        || referenced_lsn >= current_lsn
    {
        return Err(corrupt_log(format!(
            "{name} is outside the claim's accepted durable prefix"
        )));
    }
    events.get(&referenced_lsn).ok_or_else(|| {
        corrupt_log(format!(
            "{name} references unavailable or unauthenticated durable bytes"
        ))
    })
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
