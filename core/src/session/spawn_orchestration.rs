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

/// Adapter-reported logical-context outcome. This is deliberately not a
/// process-state claim and never becomes a protocol lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuationContextStatus {
    Resumed,
    NewContext,
    Unknown,
}

impl ContinuationContextStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resumed => "resumed",
            Self::NewContext => "new_context",
            Self::Unknown => "unknown",
        }
    }
}

impl TryFrom<&str> for ContinuationContextStatus {
    type Error = SpawnOrchestrationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "resumed" => Ok(Self::Resumed),
            "new_context" => Ok(Self::NewContext),
            "unknown" => Ok(Self::Unknown),
            _ => Err(SpawnOrchestrationError::UnknownContinuationContextStatus(
                value.to_owned(),
            )),
        }
    }
}

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
    #[error("unknown continuation context status {0:?}")]
    UnknownContinuationContextStatus(String),
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
/// the Operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SpawnCompletionPhaseRecord {
    external_identity_known: Vec<(u64, RuntimeGenerationRef)>,
    handshake_reconciling: Vec<(u64, RuntimeGenerationRef)>,
    staged: Option<(u64, RuntimeGenerationRef)>,
    success_evidence_reported: Vec<(u64, RuntimeGenerationRef)>,
}

impl SpawnCompletionPhaseRecord {
    pub(crate) fn observe_progress(
        &mut self,
        phase: SpawnExecutionPhase,
        runtime: RuntimeGenerationRef,
        lsn: u64,
    ) {
        match phase {
            SpawnExecutionPhase::ExternalIdentityKnown => {
                self.external_identity_known.push((lsn, runtime));
            }
            SpawnExecutionPhase::HandshakeReconciling => {
                self.handshake_reconciling.push((lsn, runtime));
            }
            SpawnExecutionPhase::SuccessEvidenceReported => {
                self.success_evidence_reported.push((lsn, runtime));
            }
            SpawnExecutionPhase::Unspecified
            | SpawnExecutionPhase::AcceptedNotOffered
            | SpawnExecutionPhase::Offered
            | SpawnExecutionPhase::QuiescingPrior
            | SpawnExecutionPhase::PriorTerminated
            | SpawnExecutionPhase::LaunchAttempted => {}
        }
    }

    pub(crate) fn observe_staged(&mut self, runtime: RuntimeGenerationRef, lsn: u64) {
        self.staged = Some((lsn, runtime));
    }

    /// Initial readiness is `identity → handshake → stage → success`. After an
    /// ambiguity poisons an already staged claim, explicit reconciliation may
    /// reuse that exact reservation, but must report a new handshake and
    /// success after the poison decision. An old success can therefore never
    /// auto-promote after stream loss.
    pub(crate) fn is_ready(
        &self,
        runtime: &RuntimeGenerationRef,
        latest_disposition_lsn: u64,
    ) -> bool {
        let Some((staged_lsn, staged_runtime)) = self.staged.as_ref() else {
            return false;
        };
        if staged_runtime != runtime {
            return false;
        }
        self.external_identity_known
            .iter()
            .filter(|(_, candidate)| candidate == runtime)
            .any(|(identity_lsn, _)| {
                self.handshake_reconciling
                    .iter()
                    .filter(|(_, candidate)| candidate == runtime)
                    .any(|(handshake_lsn, _)| {
                        *handshake_lsn > *identity_lsn
                            && *handshake_lsn > latest_disposition_lsn
                            && self
                                .success_evidence_reported
                                .iter()
                                .filter(|(_, candidate)| candidate == runtime)
                                .any(|(success_lsn, _)| {
                                    *success_lsn > *handshake_lsn
                                        && *success_lsn > *staged_lsn
                                        && *success_lsn > latest_disposition_lsn
                                })
                    })
            })
    }
}
