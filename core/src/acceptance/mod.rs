//! Pure operation-acceptance domain state.
//!
//! This module owns the canonical command lifecycle adjacency and the fold that
//! applies durable command-transition events. Both live acceptance and replay
//! use the same functions so recovery cannot acquire different state-machine
//! semantics from the write path.

pub mod elicitation;
pub mod elicitation_response;
pub mod index;
pub mod observation;
pub mod pipeline;
pub mod ports;
pub mod replay;
pub mod spawn;
pub mod state;
pub mod transitions;

pub use elicitation::{
    fence_runtime_elicitation_mutation, ingest_runtime_elicitation,
    is_runtime_elicitation_observation, prepare_runtime_elicitation_mutation,
    rebuild_slots_from_log, ElicitationRecord, ElicitationSlotLayer,
    PreparedRuntimeElicitationMutation, ELICITATION_SCHEMA,
};
pub use elicitation_response::{validate_response_payload, validate_response_responder};
pub use index::{CommandIndex, MANAGED_SPAWN_OFFERED_REASON};
pub use observation::{
    derive_transition, exact_command_correlation, fence_runtime_observation, ingest_observation,
    CommandSnapshot, CommandStateLookup, IngestResult, TransitionCandidate,
};
pub use pipeline::{
    submit, submit_with_clock, submit_with_clock_and_posture, target_key_for,
    validate_operation_boundary, COMMITTED_OPERATION_KINDS,
};
pub use ports::{
    fence_runtime_candidate, ActiveElicitation, AllowOperations, Authorized, Clock,
    ElicitationContractLookup, FencedRuntimeEvidence, GrantCheck, GrantDenied, OperationPosture,
    OperationPostureDenied, ResolvedGrantCheck, RuntimeEvidenceCandidate, RuntimeGenerationFence,
    SystemClock, TargetBinding, TargetNotFound, TargetResolver,
};
pub use replay::rebuild_from_log;
pub use spawn::{
    validate_spawn_authority_carriage, validate_spawn_operation_payload, validate_spawn_request,
    SpawnValidationError, SPAWN_REQUEST_SCHEMA,
};
pub use state::{is_terminal, CommandRecord, OperationStateExt};
pub use transitions::{
    allowed_transition, apply_grant_revocation_effect, apply_transition, AcceptanceError,
};
