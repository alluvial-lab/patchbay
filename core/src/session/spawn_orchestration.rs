//! Generic managed-spawn continuation orchestration.
//!
//! This module composes the durable claim, execution-evidence, staged-successor,
//! and promotion contracts without owning an adapter process. Adapter-specific
//! supervisors report the generated evidence; the core owns the phase order,
//! prior-runtime availability requirements, and the point at which completion
//! becomes eligible for the sole promotion driver.

use patchbay_contracts::patchbay::{
    ExternalEffectDisposition, FailureCode, RuntimeGenerationRef, SessionActivityState,
    SessionConnectivityState, SpawnExecutionPhase, SpawnGenerationClaim,
};

use super::{SessionIdentity, SessionRegistry};

/// Prior-generation presentation required by one failure/progress cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorRuntimeOutcome {
    Unchanged,
    FencedUnknownActivity,
    OfflineUnknownActivity,
    UnavailableUnknownActivity,
    Tombstoned,
    Retired,
}

/// Candidate visibility at one durable phase. None of the pre-promotion values
/// grants ordinary delivery or publishes a live session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateOutcome {
    Absent,
    UnpublishableUnknown,
    IdentifiedOrStaged,
    StagedReady,
    CurrentAfterPromotion,
    AuditOnly,
}

/// Claim/fence consequence independent from CommandState.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimFenceOutcome {
    None,
    Active,
    ReleaseEligible,
    ActivePendingPriorLiveness,
    Poisoned,
    Promoted,
    TargetAbandoned,
}

/// One typed cell in the parent phase/connectivity/fence table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnPhaseOutcome {
    pub prior: PriorRuntimeOutcome,
    pub candidate: CandidateOutcome,
    pub claim: ClaimFenceOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpawnOrchestrationError {
    #[error("spawn phase/effect/failure combination is not in the closed orchestration table")]
    InvalidPhaseOutcome,
    #[error("continuation exact prior is malformed")]
    MalformedExactPrior,
    #[error("continuation prior generation is not the exact current session")]
    PriorNotCurrent,
    #[error("continuation prior must have unknown activity before successor staging/promotion")]
    PriorActivityNotQuiesced,
    #[error("continuation prior connectivity is not offline, stale, or failed")]
    PriorConnectivityNotUnavailable,
}

/// Single source of truth for phase → connectivity/candidate/claim semantics.
///
/// The generated phase/effect registry remains the wire authority. This table
/// adds the generic orchestration consequence and is consumed by the claim
/// fold instead of maintaining another poisoning list.
pub fn phase_outcome(
    phase: SpawnExecutionPhase,
    effect: ExternalEffectDisposition,
    failure: FailureCode,
    continuation: bool,
) -> Result<SpawnPhaseOutcome, SpawnOrchestrationError> {
    use CandidateOutcome::{Absent, IdentifiedOrStaged, StagedReady, UnpublishableUnknown};
    use ClaimFenceOutcome::{Active, ActivePendingPriorLiveness, Poisoned, ReleaseEligible};
    use ExternalEffectDisposition::{Identified, MayExist, ProvedNone};
    use PriorRuntimeOutcome::{
        FencedUnknownActivity, OfflineUnknownActivity, UnavailableUnknownActivity, Unchanged,
    };
    use SpawnExecutionPhase::{
        AcceptedNotOffered, ExternalIdentityKnown, HandshakeReconciling, LaunchAttempted, Offered,
        PriorTerminated, QuiescingPrior, SuccessEvidenceReported,
    };

    let ambiguity = matches!(
        failure,
        FailureCode::AdapterUnavailable
            | FailureCode::TransportTimeout
            | FailureCode::ExecutionFailed
            | FailureCode::Expired
            | FailureCode::Cancelled
            | FailureCode::ExecutionOutcomeUnknown
    );
    let failed = failure != FailureCode::Unspecified;
    let outcome = match (phase, effect) {
        (AcceptedNotOffered, ProvedNone) if failed => SpawnPhaseOutcome {
            prior: Unchanged,
            candidate: Absent,
            claim: ReleaseEligible,
        },
        (Offered, ProvedNone) if failed => SpawnPhaseOutcome {
            prior: Unchanged,
            candidate: Absent,
            claim: ReleaseEligible,
        },
        (Offered, MayExist) if ambiguity => SpawnPhaseOutcome {
            prior: FencedUnknownActivity,
            candidate: UnpublishableUnknown,
            claim: Poisoned,
        },
        (QuiescingPrior, ProvedNone) if continuation && failed => SpawnPhaseOutcome {
            prior: FencedUnknownActivity,
            candidate: Absent,
            claim: ActivePendingPriorLiveness,
        },
        (QuiescingPrior, MayExist) if continuation && ambiguity => SpawnPhaseOutcome {
            prior: FencedUnknownActivity,
            candidate: UnpublishableUnknown,
            claim: Poisoned,
        },
        (PriorTerminated, ProvedNone) if continuation && failed => SpawnPhaseOutcome {
            prior: OfflineUnknownActivity,
            candidate: Absent,
            claim: ActivePendingPriorLiveness,
        },
        (PriorTerminated, MayExist) if continuation && ambiguity => SpawnPhaseOutcome {
            prior: OfflineUnknownActivity,
            candidate: UnpublishableUnknown,
            claim: Poisoned,
        },
        (LaunchAttempted, MayExist) if ambiguity => SpawnPhaseOutcome {
            prior: UnavailableUnknownActivity,
            candidate: UnpublishableUnknown,
            claim: Poisoned,
        },
        (LaunchAttempted, Identified) => SpawnPhaseOutcome {
            prior: UnavailableUnknownActivity,
            candidate: IdentifiedOrStaged,
            // Identity at launch does not remove launch ambiguity.
            claim: Poisoned,
        },
        (ExternalIdentityKnown | HandshakeReconciling, Identified) => SpawnPhaseOutcome {
            prior: UnavailableUnknownActivity,
            candidate: IdentifiedOrStaged,
            claim: if failed { Poisoned } else { Active },
        },
        (SuccessEvidenceReported, Identified) => SpawnPhaseOutcome {
            prior: UnavailableUnknownActivity,
            candidate: StagedReady,
            claim: if failed { Poisoned } else { Active },
        },
        _ => return Err(SpawnOrchestrationError::InvalidPhaseOutcome),
    };
    Ok(outcome)
}

/// Require the exact continuation prior to remain current but unavailable and
/// explicitly quiesced before a candidate may stage or promote.
///
/// `stale` supports stream-loss reconciliation and `failed` supports explicit
/// runtime failure. Neither value publishes N+1; only the promotion driver can
/// tombstone N and install the reserved successor.
pub fn validate_continuation_prior_quiesced(
    sessions: &SessionRegistry,
    claim: &SpawnGenerationClaim,
) -> Result<(), SpawnOrchestrationError> {
    let Some(prior) = claim.expected_prior.as_ref() else {
        return Ok(());
    };
    let external = prior
        .external_runtime
        .as_ref()
        .ok_or(SpawnOrchestrationError::MalformedExactPrior)?;
    if prior.logical_target_id != claim.logical_target_id {
        return Err(SpawnOrchestrationError::MalformedExactPrior);
    }
    let identity = SessionIdentity {
        adapter_id: external
            .adapter_id
            .clone()
            .ok_or(SpawnOrchestrationError::MalformedExactPrior)?,
        deployment_scope: external.deployment_scope.clone(),
        runtime_session_id: external
            .runtime_session_id
            .clone()
            .ok_or(SpawnOrchestrationError::MalformedExactPrior)?,
        session_generation: external
            .generation
            .ok_or(SpawnOrchestrationError::MalformedExactPrior)?,
    };
    let record = sessions
        .get_session(&identity)
        .filter(|_| {
            prior
                .logical_target_id
                .as_ref()
                .is_some_and(|logical_target_id| {
                    sessions
                        .logical_targets()
                        .get(logical_target_id)
                        .is_some_and(|target| target.current.as_ref() == Some(prior))
                })
        })
        .ok_or(SpawnOrchestrationError::PriorNotCurrent)?;
    if record.state.activity() != SessionActivityState::Unknown {
        return Err(SpawnOrchestrationError::PriorActivityNotQuiesced);
    }
    if !matches!(
        record.state.connectivity(),
        SessionConnectivityState::Offline
            | SessionConnectivityState::Stale
            | SessionConnectivityState::Failed
    ) {
        return Err(SpawnOrchestrationError::PriorConnectivityNotUnavailable);
    }
    Ok(())
}

/// Helper for tests and callers that need to compare a staged/promoted runtime
/// with the exact claim without inventing another identity rule.
#[must_use]
pub fn runtime_matches_claim(runtime: &RuntimeGenerationRef, claim: &SpawnGenerationClaim) -> bool {
    runtime.logical_target_id == claim.logical_target_id
        && runtime
            .external_runtime
            .as_ref()
            .zip(claim.claimed_generation.as_ref())
            .is_some_and(|(external, generation)| external.generation.as_ref() == Some(generation))
}

/// Durable phase checkpoints retained inside the claim projection. They are
/// evidence for ordering only; they do not publish the candidate or complete
/// the Operation. Continuations retain the complete delivery-to-staging spine
/// so readiness never depends on a caller's in-memory sequencing flags.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SpawnCompletionPhaseRecord {
    delivered: Vec<u64>,
    quiescing_prior: Vec<u64>,
    prior_terminated: Vec<u64>,
    launch_attempted: Vec<u64>,
    external_identity_known: Vec<(u64, RuntimeGenerationRef)>,
    handshake_reconciling: Vec<(u64, RuntimeGenerationRef)>,
    staged: Option<(u64, RuntimeGenerationRef)>,
    success_evidence_reported: Vec<(u64, RuntimeGenerationRef)>,
}

impl SpawnCompletionPhaseRecord {
    pub(crate) fn observe_delivered(&mut self, lsn: u64) {
        self.delivered.push(lsn);
    }

    pub(crate) fn observe_progress(
        &mut self,
        phase: SpawnExecutionPhase,
        runtime: Option<RuntimeGenerationRef>,
        failure: FailureCode,
        lsn: u64,
    ) {
        match phase {
            SpawnExecutionPhase::QuiescingPrior => self.quiescing_prior.push(lsn),
            SpawnExecutionPhase::PriorTerminated => self.prior_terminated.push(lsn),
            SpawnExecutionPhase::LaunchAttempted => self.launch_attempted.push(lsn),
            SpawnExecutionPhase::ExternalIdentityKnown if failure == FailureCode::Unspecified => {
                if let Some(runtime) = runtime {
                    self.external_identity_known.push((lsn, runtime));
                }
            }
            SpawnExecutionPhase::HandshakeReconciling if failure == FailureCode::Unspecified => {
                if let Some(runtime) = runtime {
                    self.handshake_reconciling.push((lsn, runtime));
                }
            }
            SpawnExecutionPhase::SuccessEvidenceReported if failure == FailureCode::Unspecified => {
                if let Some(runtime) = runtime {
                    self.success_evidence_reported.push((lsn, runtime));
                }
            }
            SpawnExecutionPhase::Unspecified
            | SpawnExecutionPhase::AcceptedNotOffered
            | SpawnExecutionPhase::Offered
            | SpawnExecutionPhase::ExternalIdentityKnown
            | SpawnExecutionPhase::HandshakeReconciling
            | SpawnExecutionPhase::SuccessEvidenceReported => {}
        }
    }

    pub(crate) fn may_stage(&self, runtime: &RuntimeGenerationRef, continuation: bool) -> bool {
        self.has_initial_handshake_before(runtime, continuation, None)
    }

    pub(crate) fn observe_staged(&mut self, runtime: RuntimeGenerationRef, lsn: u64) {
        self.staged = Some((lsn, runtime));
    }

    /// Initial readiness is the durable `delivery → quiesce → old runtime →
    /// launch → identity → handshake → stage → success` chain for a
    /// continuation (`identity → handshake → stage → success` for fresh
    /// spawn). If ambiguity poisons an already staged claim, reconciliation may
    /// reuse that reservation, but a later handshake and success are required.
    pub(crate) fn is_ready(
        &self,
        runtime: &RuntimeGenerationRef,
        continuation: bool,
        accepted_lsn: u64,
        latest_disposition_lsn: u64,
    ) -> bool {
        let Some((staged_lsn, staged_runtime)) = self.staged.as_ref() else {
            return false;
        };
        if staged_runtime != runtime
            || !self.has_initial_handshake_before(runtime, continuation, Some(*staged_lsn))
        {
            return false;
        }

        if latest_disposition_lsn > accepted_lsn {
            self.handshake_reconciling
                .iter()
                .filter(|(_, candidate)| candidate == runtime)
                .any(|(handshake_lsn, _)| {
                    *handshake_lsn > latest_disposition_lsn
                        && self.success_after(
                            runtime,
                            (*staged_lsn)
                                .max(*handshake_lsn)
                                .max(latest_disposition_lsn),
                        )
                })
        } else {
            self.success_after(runtime, *staged_lsn)
        }
    }

    fn has_initial_handshake_before(
        &self,
        runtime: &RuntimeGenerationRef,
        continuation: bool,
        before_lsn: Option<u64>,
    ) -> bool {
        self.external_identity_known
            .iter()
            .filter(|(_, candidate)| candidate == runtime)
            .any(|(identity_lsn, _)| {
                (!continuation || self.has_continuation_prefix(*identity_lsn))
                    && self
                        .handshake_reconciling
                        .iter()
                        .filter(|(_, candidate)| candidate == runtime)
                        .any(|(handshake_lsn, _)| {
                            *handshake_lsn > *identity_lsn
                                && before_lsn.is_none_or(|limit| *handshake_lsn < limit)
                        })
            })
    }

    fn has_continuation_prefix(&self, identity_lsn: u64) -> bool {
        self.delivered.iter().any(|delivered_lsn| {
            self.quiescing_prior
                .iter()
                .filter(|quiesce_lsn| **quiesce_lsn > *delivered_lsn)
                .any(|quiesce_lsn| {
                    self.prior_terminated
                        .iter()
                        .filter(|terminated_lsn| **terminated_lsn > *quiesce_lsn)
                        .any(|terminated_lsn| {
                            self.launch_attempted.iter().any(|launch_lsn| {
                                *launch_lsn > *terminated_lsn && *launch_lsn < identity_lsn
                            })
                        })
                })
        })
    }

    fn success_after(&self, runtime: &RuntimeGenerationRef, after_lsn: u64) -> bool {
        self.success_evidence_reported
            .iter()
            .filter(|(_, candidate)| candidate == runtime)
            .any(|(success_lsn, _)| *success_lsn > after_lsn)
    }
}
