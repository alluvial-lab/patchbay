//! Property tests for the stated-normative authority obligations.
//!
//! Seven authority properties have executable implementation oracles here. The
//! eighth, `ElicitationResponderAuthority`, remains an explicit coverage gap
//! because authority does not yet receive the Elicitation responder evidence.
//!
//! The mutation tests are load-bearing non-vacuity evidence: the compound-
//! issuer oracle rejects a checker that trusts the payload actor, and the
//! spawn-revocation oracle rejects a registry that cascades parent revocation.

use std::collections::HashSet;

use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, ActorEndpointRef, ActorId, AdapterId,
    AuthorityDomainId, CommandId, CommandTransition, DescendantGrant, DescendantGrantProvenance,
    DeviceId, EndpointId, EventId, FailureCode, Generation, Grant, GrantId, GrantProvenance,
    GrantRevocationPolicy, Lsn, Operation, OperationKind, OperationState, Revocation,
    RuntimeSessionId, SessionRegistered, SessionStateEvent, StoredEventKind, StoredEventPayload,
    SubmissionOutcome, TargetScope, TargetScopeKind, TypedCorrelation,
};
use patchbay_core::{
    acceptance::{
        submit, ActiveElicitation, Authorized, CommandSnapshot, CommandStateLookup,
        ElicitationContractLookup, GrantCheck, GrantDenied, TargetBinding, TargetNotFound,
        TargetResolver,
    },
    authority::{
        ingest_descendant_grant, ingest_grant, ingest_revocation, rebuild_from_log, AuthorityError,
        AuthorityRegistry, GrantLookup, GrantProjection, GrantProvenanceKind, GrantRecord,
        IssuerContext, SpawnDescendantTail, DESCENDANT_GRANT_ALLOWED_KINDS,
    },
    storage::{RecordedEvent, RusqliteStorage},
};
use proptest::prelude::*;
use prost::Message;

const ACCEPTED_OPERATION_KINDS: [OperationKind; 10] = [
    OperationKind::Spawn,
    OperationKind::Attach,
    OperationKind::Instruct,
    OperationKind::Cancel,
    OperationKind::Interrupt,
    OperationKind::Query,
    OperationKind::ApprovalResponse,
    OperationKind::ElicitationResponse,
    OperationKind::Reconfigure,
    OperationKind::SessionManagement,
];

fn any_domain() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "authority-alpha".to_owned(),
        "authority-beta".to_owned(),
        "authority-gamma".to_owned(),
    ])
}

fn any_actor() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "operator-alice".to_owned(),
        "operator-bob".to_owned(),
        "operator-carol".to_owned(),
    ])
}

fn any_operation_kind() -> impl Strategy<Value = OperationKind> {
    prop::sample::select(ACCEPTED_OPERATION_KINDS.to_vec())
}

fn any_target_scope_kind() -> impl Strategy<Value = TargetScopeKind> {
    prop::sample::select(vec![
        TargetScopeKind::Actor,
        TargetScopeKind::Adapter,
        TargetScopeKind::RuntimeSession,
        TargetScopeKind::ProjectSessionGroup,
        TargetScopeKind::FleetSupervisor,
        TargetScopeKind::AuthorityDomain,
        TargetScopeKind::Resource,
    ])
}

fn any_adapter() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "adapter-pi".to_owned(),
        "adapter-shell".to_owned(),
        "adapter-job".to_owned(),
        "adapter-test".to_owned(),
    ])
}

fn any_distinct_actors() -> impl Strategy<Value = (String, String)> {
    (any_actor(), any_actor()).prop_filter("actors must differ", |(left, right)| left != right)
}

fn any_distinct_adapters() -> impl Strategy<Value = (String, String)> {
    (any_adapter(), any_adapter())
        .prop_filter("adapters must differ", |(left, right)| left != right)
}

fn any_kind_subset() -> impl Strategy<Value = Vec<OperationKind>> {
    prop::collection::vec(any::<bool>(), ACCEPTED_OPERATION_KINDS.len())
        .prop_filter("subset must be non-empty and proper", |selected| {
            selected.iter().any(|included| *included) && selected.iter().any(|included| !*included)
        })
        .prop_map(|selected| {
            ACCEPTED_OPERATION_KINDS
                .iter()
                .copied()
                .zip(selected)
                .filter_map(|(kind, included)| included.then_some(kind))
                .collect()
        })
}

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId {
        value: value.to_owned(),
    }
}

fn actor(value: &str) -> ActorId {
    ActorId {
        value: value.to_owned(),
    }
}

fn endpoint(value: &str) -> EndpointId {
    EndpointId {
        value: value.to_owned(),
    }
}

fn grant_id(value: &str) -> GrantId {
    GrantId {
        value: value.to_owned(),
    }
}

fn adapter(value: &str) -> AdapterId {
    AdapterId {
        value: value.to_owned(),
    }
}

fn runtime_session(value: &str) -> RuntimeSessionId {
    RuntimeSessionId {
        value: value.to_owned(),
    }
}

fn valid_target_scope(kind: TargetScopeKind, suffix: &str) -> TargetScope {
    match kind {
        TargetScopeKind::Actor => TargetScope {
            kind: kind as i32,
            actor_id: Some(actor(&format!("target-actor-{suffix}"))),
            ..TargetScope::default()
        },
        TargetScopeKind::Adapter => adapter_scope(&format!("adapter-{suffix}")),
        TargetScopeKind::RuntimeSession => session_scope(
            &format!("adapter-{suffix}"),
            &format!("session-{suffix}"),
            1,
        ),
        TargetScopeKind::ProjectSessionGroup => TargetScope {
            kind: kind as i32,
            project_or_group: format!("project-{suffix}"),
            ..TargetScope::default()
        },
        TargetScopeKind::FleetSupervisor | TargetScopeKind::AuthorityDomain => TargetScope {
            kind: kind as i32,
            ..TargetScope::default()
        },
        TargetScopeKind::Resource => TargetScope {
            kind: kind as i32,
            resource_id: format!("resource-{suffix}"),
            ..TargetScope::default()
        },
        TargetScopeKind::Unspecified => {
            panic!("the target-scope strategy never generates Unspecified")
        }
    }
}

fn adapter_scope(adapter_id: &str) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(adapter(adapter_id)),
        ..TargetScope::default()
    }
}

fn fleet_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::FleetSupervisor as i32,
        ..TargetScope::default()
    }
}

fn session_scope(adapter_id: &str, session_id: &str, generation: u64) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(adapter(adapter_id)),
        runtime_session_id: Some(runtime_session(session_id)),
        session_generation: Some(Generation { value: generation }),
        deployment_scope: "local".to_owned(),
        ..TargetScope::default()
    }
}

fn operator_grant(
    id: &str,
    authority_domain_id: &AuthorityDomainId,
    subject_actor_id: &ActorId,
    target_scope: TargetScope,
    allowed_operation_kinds: &[OperationKind],
) -> Grant {
    Grant {
        grant_id: Some(grant_id(id)),
        authority_domain_id: Some(authority_domain_id.clone()),
        subject_actor_id: Some(subject_actor_id.clone()),
        subject_endpoint_class: "verified-control-surface".to_owned(),
        target_scope: Some(target_scope),
        allowed_operation_kinds: allowed_operation_kinds
            .iter()
            .map(|kind| *kind as i32)
            .collect(),
        provenance: Some(GrantProvenance {
            reason: "authority property fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    }
}

fn descendant_grant(
    id: &str,
    parent_id: &str,
    authority_domain_id: &AuthorityDomainId,
    subject_actor_id: &ActorId,
    target_scope: TargetScope,
) -> DescendantGrant {
    DescendantGrant {
        grant_id: Some(grant_id(id)),
        authority_domain_id: Some(authority_domain_id.clone()),
        subject_actor_id: Some(subject_actor_id.clone()),
        subject_endpoint_class: "verified-control-surface".to_owned(),
        target_scope: Some(target_scope),
        allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS
            .iter()
            .map(|kind| *kind as i32)
            .collect(),
        provenance: Some(DescendantGrantProvenance {
            spawning_grant_id: Some(grant_id(parent_id)),
            ..DescendantGrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..DescendantGrant::default()
    }
}

fn revocation(
    authority_domain_id: &AuthorityDomainId,
    id: &str,
    revocation_generation: u64,
) -> Revocation {
    Revocation {
        authority_domain_id: Some(authority_domain_id.clone()),
        grant_id: Some(grant_id(id)),
        revocation_generation: Some(Generation {
            value: revocation_generation,
        }),
        accepted_operation_policy: GrantRevocationPolicy::Cancel as i32,
        reason: "authority property revocation".to_owned(),
        ..Revocation::default()
    }
}

async fn ingest_live_grant<L: GrantProjection>(
    storage: &RusqliteStorage,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    id: &str,
    subject_actor_id: &ActorId,
    allowed_operation_kinds: &[OperationKind],
    target_scope: TargetScope,
) -> Result<GrantId, AuthorityError> {
    ingest_grant(
        storage,
        projection,
        authority_domain_id,
        operator_grant(
            id,
            authority_domain_id,
            subject_actor_id,
            target_scope,
            allowed_operation_kinds,
        ),
    )
    .await?;
    Ok(grant_id(id))
}

#[derive(Clone)]
struct TestIssuerContext {
    actor: Option<ActorId>,
    endpoint: Option<EndpointId>,
    device: Option<DeviceId>,
    generation: Option<Generation>,
    domain: AuthorityDomainId,
}

impl TestIssuerContext {
    fn verified(actor_id: ActorId, authority_domain_id: AuthorityDomainId) -> Self {
        Self {
            actor: Some(actor_id),
            endpoint: Some(endpoint("verified-endpoint")),
            device: Some(DeviceId {
                value: "verified-device".to_owned(),
            }),
            generation: Some(Generation { value: 1 }),
            domain: authority_domain_id,
        }
    }
}

impl IssuerContext for TestIssuerContext {
    fn verified_actor(&self) -> Option<&ActorId> {
        self.actor.as_ref()
    }

    fn verified_endpoint(&self) -> Option<&EndpointId> {
        self.endpoint.as_ref()
    }

    fn verified_device(&self) -> Option<&DeviceId> {
        self.device.as_ref()
    }

    fn endpoint_generation(&self) -> Option<Generation> {
        self.generation
    }

    fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.domain
    }
}

struct AlwaysResolvedTarget;

impl TargetResolver for AlwaysResolvedTarget {
    async fn resolve(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _target_scope: &TargetScope,
    ) -> Result<TargetBinding, TargetNotFound> {
        Ok(TargetBinding {
            runtime_session_id: runtime_session("compound-issuer-session"),
            session_generation: Generation { value: 1 },
            adapter_id: adapter("adapter-pi"),
        })
    }
}

struct AlwaysAcceptedCommandState;

impl ElicitationContractLookup for AlwaysAcceptedCommandState {
    async fn active_contract(
        &self,
        _elicitation_id: &patchbay_contracts::patchbay::ElicitationId,
    ) -> Option<ActiveElicitation> {
        None
    }
}

impl CommandStateLookup for AlwaysAcceptedCommandState {
    async fn current_state(&self, _command_id: &CommandId) -> Option<CommandSnapshot> {
        Some(CommandSnapshot {
            state: OperationState::Accepted,
            correlations: vec![],
            terminal_lsn: None,
        })
    }
}

async fn authorized_by<C: GrantCheck>(
    checker: &C,
    authority_domain_id: &AuthorityDomainId,
    issuer: &dyn IssuerContext,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
    expected_grant_id: &GrantId,
) -> bool {
    checker
        .check(authority_domain_id, issuer, operation_kind, target_scope)
        .await
        .is_ok_and(|authorized| authorized.grant_id.as_ref() == Some(expected_grant_id))
}

/// Oracle 1: an empty grant set denies every otherwise-valid authority query.
async fn no_command_without_grant_holds<C: GrantCheck>(
    checker: &C,
    authority_domain_id: &AuthorityDomainId,
    issuer: &dyn IssuerContext,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
) -> bool {
    matches!(
        checker
            .check(authority_domain_id, issuer, operation_kind, target_scope,)
            .await,
        Err(GrantDenied::NoGrant { .. })
    )
}

fn operation_with_payload_sender(
    authority_domain_id: &AuthorityDomainId,
    payload_actor: &ActorId,
    operation_kind: OperationKind,
    target_scope: TargetScope,
) -> Operation {
    Operation {
        authority_domain_id: Some(authority_domain_id.clone()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(payload_actor.clone()),
            ..ActorEndpointRef::default()
        }),
        kind: operation_kind as i32,
        target_scope: Some(target_scope),
        ..Operation::default()
    }
}

/// Oracle 2: payload identity cannot substitute for the verified issuer.
async fn compound_issuer_holds<C: GrantCheck>(
    checker: &C,
    operation: &Operation,
    verified_issuer: &dyn IssuerContext,
) -> Result<(), String> {
    let authority_domain_id = operation
        .authority_domain_id
        .as_ref()
        .ok_or_else(|| "test operation is missing authority_domain_id".to_owned())?;
    let operation_kind = OperationKind::try_from(operation.kind)
        .map_err(|_| format!("test operation has unknown kind {}", operation.kind))?;
    let target_scope = operation
        .target_scope
        .as_ref()
        .ok_or_else(|| "test operation is missing target_scope".to_owned())?;

    match checker
        .check(
            authority_domain_id,
            verified_issuer,
            operation_kind,
            target_scope,
        )
        .await
    {
        Err(GrantDenied::NoGrant { .. }) => Ok(()),
        Ok(authorized) => Err(format!(
            "mismatched verified issuer was authorized by {:?}",
            authorized.grant_id
        )),
    }
}

/// Oracle 3: the canonical kind membership in a grant is both necessary and sufficient.
async fn grant_authority_is_command_kinds_holds<C: GrantCheck>(
    checker: &C,
    authority_domain_id: &AuthorityDomainId,
    issuer: &dyn IssuerContext,
    target_scope: &TargetScope,
    grant_id: &GrantId,
    allowed_operation_kinds: &[OperationKind],
) -> Result<(), String> {
    for operation_kind in ACCEPTED_OPERATION_KINDS {
        let authorized = authorized_by(
            checker,
            authority_domain_id,
            issuer,
            operation_kind,
            target_scope,
            grant_id,
        )
        .await;
        let expected = allowed_operation_kinds.contains(&operation_kind);
        if authorized != expected {
            return Err(format!(
                "kind {operation_kind:?}: authorized={authorized}, expected={expected}"
            ));
        }
    }
    Ok(())
}

/// Oracle 4: a named revocation removes that grant from future authorization.
async fn revocation_prevents_future_holds<C: GrantCheck>(
    checker: &C,
    authority_domain_id: &AuthorityDomainId,
    issuer: &dyn IssuerContext,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
) -> bool {
    matches!(
        checker
            .check(authority_domain_id, issuer, operation_kind, target_scope,)
            .await,
        Err(GrantDenied::NoGrant { .. })
    )
}

struct FleetSpawnOracle<'a> {
    authority_domain_id: &'a AuthorityDomainId,
    issuer: &'a dyn IssuerContext,
    fleet_grant_id: &'a GrantId,
    adapter_grant_id: &'a GrantId,
    scoped_adapter: &'a str,
    other_adapter: &'a str,
}

/// Oracle 5: fleet, adapter, and existing-session spawn scopes retain distinct containment.
async fn fleet_authority_for_spawn_holds<C: GrantCheck>(
    fleet_checker: &C,
    adapter_checker: &C,
    session_checker: &C,
    oracle: &FleetSpawnOracle<'_>,
) -> Result<(), String> {
    for adapter_id in [oracle.scoped_adapter, oracle.other_adapter, "adapter-third"] {
        if !authorized_by(
            fleet_checker,
            oracle.authority_domain_id,
            oracle.issuer,
            OperationKind::Spawn,
            &adapter_scope(adapter_id),
            oracle.fleet_grant_id,
        )
        .await
        {
            return Err(format!(
                "fleet grant did not authorize spawn on adapter {adapter_id}"
            ));
        }
    }

    if !authorized_by(
        adapter_checker,
        oracle.authority_domain_id,
        oracle.issuer,
        OperationKind::Spawn,
        &adapter_scope(oracle.scoped_adapter),
        oracle.adapter_grant_id,
    )
    .await
    {
        return Err("adapter grant did not authorize its own adapter".to_owned());
    }
    if !matches!(
        adapter_checker
            .check(
                oracle.authority_domain_id,
                oracle.issuer,
                OperationKind::Spawn,
                &adapter_scope(oracle.other_adapter),
            )
            .await,
        Err(GrantDenied::NoGrant { .. })
    ) {
        return Err("adapter grant authorized a different adapter".to_owned());
    }

    let nonexistent_session = session_scope(oracle.scoped_adapter, "not-yet-existing", 2);
    if !matches!(
        session_checker
            .check(
                oracle.authority_domain_id,
                oracle.issuer,
                OperationKind::Spawn,
                &nonexistent_session,
            )
            .await,
        Err(GrantDenied::NoGrant { .. })
    ) {
        return Err("runtime-session grant authorized a different future session".to_owned());
    }

    Ok(())
}

fn recorded<M: Message>(
    lsn: u64,
    authority_domain_id: &AuthorityDomainId,
    kind: StoredEventKind,
    message: &M,
) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(authority_domain_id.clone()),
            lsn: Some(Lsn { value: lsn }),
        },
        payload: StoredEventPayload {
            kind: kind as i32,
            payload: message.encode_to_vec(),
        },
    }
}

fn spawn_facts(
    authority_domain_id: &AuthorityDomainId,
    actor_id: &ActorId,
    command_id: &CommandId,
    adapter_id: &str,
    session_id: &str,
    generation: u64,
) -> [RecordedEvent; 3] {
    let spawn = Operation {
        command_id: Some(command_id.clone()),
        authority_domain_id: Some(authority_domain_id.clone()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(actor_id.clone()),
            ..ActorEndpointRef::default()
        }),
        kind: OperationKind::Spawn as i32,
        target_scope: Some(fleet_scope()),
        ..Operation::default()
    };
    let completed = CommandTransition {
        command_id: Some(command_id.clone()),
        from_state: OperationState::Running as i32,
        to_state: OperationState::Completed as i32,
        ..CommandTransition::default()
    };
    let registered = SessionStateEvent {
        authority_domain_id: Some(authority_domain_id.clone()),
        mutation: Some(session_state_event::Mutation::Registered(
            SessionRegistered {
                adapter_id: Some(adapter(adapter_id)),
                deployment_scope: "local".to_owned(),
                runtime_session_id: Some(runtime_session(session_id)),
                session_generation: Some(Generation { value: generation }),
                spawn_origin: Some(TypedCorrelation {
                    r#ref: Some(typed_correlation::Ref::CommandId(command_id.clone())),
                }),
                ..SessionRegistered::default()
            },
        )),
    };

    [
        recorded(1, authority_domain_id, StoredEventKind::Operation, &spawn),
        recorded(
            2,
            authority_domain_id,
            StoredEventKind::CommandTransition,
            &completed,
        ),
        recorded(
            3,
            authority_domain_id,
            StoredEventKind::SessionState,
            &registered,
        ),
    ]
}

/// Oracle 6: the three durable spawn facts produce one deterministic descendant issuance.
fn spawn_creates_descendant_grant_holds(
    events: &[RecordedEvent; 3],
    order: [usize; 3],
    authority_domain_id: &AuthorityDomainId,
    command_id: &CommandId,
    expected_actor: &ActorId,
) -> Result<(), String> {
    fn collect(
        events: &[RecordedEvent; 3],
        order: [usize; 3],
    ) -> Result<Vec<patchbay_core::authority::DescendantGrantIssuance>, String> {
        let mut tail = SpawnDescendantTail::new();
        order
            .into_iter()
            .filter_map(|index| match tail.observe(&events[index]) {
                Ok(Some(issuance)) => Some(Ok(issuance)),
                Ok(None) => None,
                Err(error) => Some(Err(error.to_string())),
            })
            .collect()
    }

    let issuances = collect(events, order)?;
    if issuances.len() != 1 {
        return Err(format!(
            "expected exactly one issuance, found {}",
            issuances.len()
        ));
    }
    let issuance = &issuances[0];
    let expected_id = grant_id(&format!(
        "desc:{}:{}",
        authority_domain_id.value, command_id.value
    ));
    if issuance.spawn_operation_id != *command_id
        || issuance.authority_domain_id != *authority_domain_id
        || issuance.subject_actor_id != *expected_actor
        || issuance.allowed_operation_kinds != DESCENDANT_GRANT_ALLOWED_KINDS
        || issuance.descendant_grant_id != expected_id
    {
        return Err(format!("incorrect descendant issuance: {issuance:?}"));
    }

    let replayed = collect(events, order)?;
    if replayed.len() != 1 || replayed[0].descendant_grant_id != issuance.descendant_grant_id {
        return Err("fresh replay produced a different deterministic grant id".to_owned());
    }
    Ok(())
}

/// Oracle 7: parent and descendant revocation are independent authority levers.
async fn spawn_revocation_does_not_cascade_holds<L>(
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    subject_actor_id: &ActorId,
    adapter_id: &str,
    session_id: &str,
    generation: u64,
) -> Result<(), String>
where
    L: GrantProjection + GrantCheck,
{
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let parent_id = ingest_live_grant(
        &storage,
        projection,
        authority_domain_id,
        "spawn-parent",
        subject_actor_id,
        &[OperationKind::Spawn],
        fleet_scope(),
    )
    .await
    .map_err(|error| error.to_string())?;
    let descendant_id = grant_id("spawn-descendant");
    let descendant_target = session_scope(adapter_id, session_id, generation);
    ingest_descendant_grant(
        &storage,
        projection,
        authority_domain_id,
        descendant_grant(
            &descendant_id.value,
            &parent_id.value,
            authority_domain_id,
            subject_actor_id,
            descendant_target.clone(),
        ),
    )
    .await
    .map_err(|error| error.to_string())?;

    let issuer = TestIssuerContext::verified(subject_actor_id.clone(), authority_domain_id.clone());
    if !authorized_by(
        projection,
        authority_domain_id,
        &issuer,
        OperationKind::Spawn,
        &adapter_scope(adapter_id),
        &parent_id,
    )
    .await
        || !authorized_by(
            projection,
            authority_domain_id,
            &issuer,
            OperationKind::Instruct,
            &descendant_target,
            &descendant_id,
        )
        .await
    {
        return Err("fixture grants were not live before revocation".to_owned());
    }

    ingest_revocation(
        &storage,
        projection,
        authority_domain_id,
        revocation(authority_domain_id, &parent_id.value, 1),
    )
    .await
    .map_err(|error| error.to_string())?;

    let parent_denied = matches!(
        projection
            .check(
                authority_domain_id,
                &issuer,
                OperationKind::Spawn,
                &adapter_scope(adapter_id),
            )
            .await,
        Err(GrantDenied::NoGrant { .. })
    );
    let descendant_still_authorizes = authorized_by(
        projection,
        authority_domain_id,
        &issuer,
        OperationKind::Instruct,
        &descendant_target,
        &descendant_id,
    )
    .await;
    if !parent_denied || !descendant_still_authorizes {
        return Err(format!(
            "lever 1 failed: parent_denied={parent_denied}, descendant_authorizes={descendant_still_authorizes}"
        ));
    }

    ingest_revocation(
        &storage,
        projection,
        authority_domain_id,
        revocation(authority_domain_id, &descendant_id.value, 1),
    )
    .await
    .map_err(|error| error.to_string())?;
    if !matches!(
        projection
            .check(
                authority_domain_id,
                &issuer,
                OperationKind::Instruct,
                &descendant_target,
            )
            .await,
        Err(GrantDenied::NoGrant { .. })
    ) {
        return Err("lever 2 failed: explicitly revoked descendant still authorizes".to_owned());
    }

    Ok(())
}

#[derive(Clone, Debug)]
enum ReplayGrantPlan {
    Operator {
        actor: String,
        operation_kind: OperationKind,
        target_kind: TargetScopeKind,
        revoke: bool,
    },
    Descendant {
        actor: String,
        revoke: bool,
    },
}

fn any_replay_grant_plan() -> impl Strategy<Value = ReplayGrantPlan> {
    prop_oneof![
        (
            any_actor(),
            any_operation_kind(),
            any_target_scope_kind(),
            any::<bool>(),
        )
            .prop_map(|(actor, operation_kind, target_kind, revoke)| {
                ReplayGrantPlan::Operator {
                    actor,
                    operation_kind,
                    target_kind,
                    revoke,
                }
            }),
        (any_actor(), any::<bool>())
            .prop_map(|(actor, revoke)| ReplayGrantPlan::Descendant { actor, revoke }),
    ]
}

async fn replay_matches_live_holds(
    authority_domain_id: &AuthorityDomainId,
    plans: &[ReplayGrantPlan],
) -> Result<(), String> {
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let mut live = AuthorityRegistry::new();
    let mut grant_ids = Vec::with_capacity(plans.len());

    for (index, plan) in plans.iter().enumerate() {
        let id = format!("replay-grant-{index}");
        match plan {
            ReplayGrantPlan::Operator {
                actor,
                operation_kind,
                target_kind,
                revoke,
            } => {
                ingest_live_grant(
                    &storage,
                    &mut live,
                    authority_domain_id,
                    &id,
                    &self::actor(actor),
                    &[*operation_kind],
                    valid_target_scope(*target_kind, &index.to_string()),
                )
                .await
                .map_err(|error| error.to_string())?;
                if *revoke {
                    ingest_revocation(
                        &storage,
                        &mut live,
                        authority_domain_id,
                        revocation(authority_domain_id, &id, 1),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                }
            }
            ReplayGrantPlan::Descendant { actor, revoke } => {
                ingest_descendant_grant(
                    &storage,
                    &mut live,
                    authority_domain_id,
                    descendant_grant(
                        &id,
                        &format!("replay-parent-{index}"),
                        authority_domain_id,
                        &self::actor(actor),
                        session_scope(
                            &format!("replay-adapter-{index}"),
                            &format!("replay-session-{index}"),
                            index as u64 + 1,
                        ),
                    ),
                )
                .await
                .map_err(|error| error.to_string())?;
                if *revoke {
                    ingest_revocation(
                        &storage,
                        &mut live,
                        authority_domain_id,
                        revocation(authority_domain_id, &id, 1),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                }
            }
        }
        grant_ids.push(grant_id(&id));
    }

    let rebuilt = rebuild_from_log(&storage, authority_domain_id)
        .await
        .map_err(|error| format!("authority replay failed: {error}"))?;
    if rebuilt != live {
        return Err("replayed registry differs from the live projection".to_owned());
    }

    let live_ids: HashSet<_> = live
        .live_grants()
        .map(|grant| grant.grant_id.clone())
        .collect();
    let rebuilt_ids: HashSet<_> = rebuilt
        .live_grants()
        .map(|grant| grant.grant_id.clone())
        .collect();
    if rebuilt_ids != live_ids {
        return Err("replayed live-grant set differs from the live projection".to_owned());
    }
    for id in grant_ids {
        if rebuilt.get_grant(&id) != live.get_grant(&id) {
            return Err(format!("replayed grant record differs for {id:?}"));
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        ..ProptestConfig::default()
    })]

    /// 1. NoCommandWithoutGrant: authority is deny-by-default.
    #[test]
    fn no_command_without_grant(
        domain_value in any_domain(),
        actor_value in any_actor(),
        operation_kind in any_operation_kind(),
        target_kind in any_target_scope_kind(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let authority_domain_id = domain(&domain_value);
            let issuer = TestIssuerContext::verified(actor(&actor_value), authority_domain_id.clone());
            let registry = AuthorityRegistry::new();
            prop_assert!(no_command_without_grant_holds(
                &registry,
                &authority_domain_id,
                &issuer,
                operation_kind,
                &valid_target_scope(target_kind, "deny-default"),
            ).await);
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// 2. CompoundIssuer: the verified issuer wins over Operation.sender.
    #[test]
    fn compound_issuer(
        domain_value in any_domain(),
        (payload_actor, verified_actor) in any_distinct_actors(),
        operation_kind in any_operation_kind(),
        target_kind in any_target_scope_kind(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let authority_domain_id = domain(&domain_value);
            let target_scope = valid_target_scope(target_kind, "compound");
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = AuthorityRegistry::new();
            ingest_live_grant(
                &storage,
                &mut registry,
                &authority_domain_id,
                "payload-actor-grant",
                &actor(&payload_actor),
                &[operation_kind],
                target_scope.clone(),
            ).await.unwrap();
            let operation = operation_with_payload_sender(
                &authority_domain_id,
                &actor(&payload_actor),
                operation_kind,
                target_scope,
            );
            let issuer = TestIssuerContext::verified(actor(&verified_actor), authority_domain_id);
            compound_issuer_holds(&registry, &operation, &issuer)
                .await
                .map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// 3. GrantAuthorityIsCommandKinds: only canonical kinds listed by the grant authorize.
    #[test]
    fn grant_authority_is_command_kinds(
        domain_value in any_domain(),
        actor_value in any_actor(),
        target_kind in any_target_scope_kind(),
        allowed_operation_kinds in any_kind_subset(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let authority_domain_id = domain(&domain_value);
            let subject = actor(&actor_value);
            let target_scope = valid_target_scope(target_kind, "kind-membership");
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = AuthorityRegistry::new();
            let id = ingest_live_grant(
                &storage,
                &mut registry,
                &authority_domain_id,
                "kind-grant",
                &subject,
                &allowed_operation_kinds,
                target_scope.clone(),
            ).await.unwrap();
            let issuer = TestIssuerContext::verified(subject, authority_domain_id.clone());
            grant_authority_is_command_kinds_holds(
                &registry,
                &authority_domain_id,
                &issuer,
                &target_scope,
                &id,
                &allowed_operation_kinds,
            ).await.map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// 4. RevocationPreventsFuture: a revoked grant cannot authorize a later check.
    #[test]
    fn revocation_prevents_future(
        domain_value in any_domain(),
        actor_value in any_actor(),
        operation_kind in any_operation_kind(),
        target_kind in any_target_scope_kind(),
        revocation_generation in 1u64..=4,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let authority_domain_id = domain(&domain_value);
            let subject = actor(&actor_value);
            let target_scope = valid_target_scope(target_kind, "revoked");
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = AuthorityRegistry::new();
            ingest_live_grant(
                &storage,
                &mut registry,
                &authority_domain_id,
                "revoked-grant",
                &subject,
                &[operation_kind],
                target_scope.clone(),
            ).await.unwrap();
            ingest_revocation(
                &storage,
                &mut registry,
                &authority_domain_id,
                revocation(&authority_domain_id, "revoked-grant", revocation_generation),
            ).await.unwrap();
            let issuer = TestIssuerContext::verified(subject, authority_domain_id.clone());
            prop_assert!(revocation_prevents_future_holds(
                &registry,
                &authority_domain_id,
                &issuer,
                operation_kind,
                &target_scope,
            ).await);
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// 5. FleetAuthorityForSpawn: fleet is broad, adapter is narrow, session is exact.
    #[test]
    fn fleet_authority_for_spawn(
        domain_value in any_domain(),
        actor_value in any_actor(),
        (scoped_adapter, other_adapter) in any_distinct_adapters(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let authority_domain_id = domain(&domain_value);
            let subject = actor(&actor_value);
            let issuer = TestIssuerContext::verified(subject.clone(), authority_domain_id.clone());

            let fleet_storage = RusqliteStorage::open_in_memory().unwrap();
            let mut fleet_registry = AuthorityRegistry::new();
            let fleet_id = ingest_live_grant(
                &fleet_storage,
                &mut fleet_registry,
                &authority_domain_id,
                "fleet-spawn",
                &subject,
                &[OperationKind::Spawn],
                fleet_scope(),
            ).await.unwrap();

            let adapter_storage = RusqliteStorage::open_in_memory().unwrap();
            let mut adapter_registry = AuthorityRegistry::new();
            let adapter_id = ingest_live_grant(
                &adapter_storage,
                &mut adapter_registry,
                &authority_domain_id,
                "adapter-spawn",
                &subject,
                &[OperationKind::Spawn],
                adapter_scope(&scoped_adapter),
            ).await.unwrap();

            let session_storage = RusqliteStorage::open_in_memory().unwrap();
            let mut session_registry = AuthorityRegistry::new();
            ingest_live_grant(
                &session_storage,
                &mut session_registry,
                &authority_domain_id,
                "session-spawn",
                &subject,
                &[OperationKind::Spawn],
                session_scope(&scoped_adapter, "existing-session", 1),
            ).await.unwrap();

            let oracle = FleetSpawnOracle {
                authority_domain_id: &authority_domain_id,
                issuer: &issuer,
                fleet_grant_id: &fleet_id,
                adapter_grant_id: &adapter_id,
                scoped_adapter: &scoped_adapter,
                other_adapter: &other_adapter,
            };
            fleet_authority_for_spawn_holds(
                &fleet_registry,
                &adapter_registry,
                &session_registry,
                &oracle,
            ).await.map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// 6. SpawnCreatesDescendantGrant: completed spawn facts join in any order.
    #[test]
    fn spawn_creates_descendant_grant(
        domain_value in any_domain(),
        actor_value in any_actor(),
        command_suffix in "[a-z0-9]{1,12}",
        adapter_value in any_adapter(),
        session_suffix in "[a-z0-9]{1,12}",
        generation in 1u64..=4,
        order in prop::sample::select(vec![
            [0, 1, 2], [0, 2, 1], [1, 0, 2],
            [1, 2, 0], [2, 0, 1], [2, 1, 0],
        ]),
    ) {
        let authority_domain_id = domain(&domain_value);
        let subject = actor(&actor_value);
        let command_id = CommandId { value: format!("spawn-{command_suffix}") };
        let events = spawn_facts(
            &authority_domain_id,
            &subject,
            &command_id,
            &adapter_value,
            &format!("session-{session_suffix}"),
            generation,
        );
        spawn_creates_descendant_grant_holds(
            &events,
            order,
            &authority_domain_id,
            &command_id,
            &subject,
        ).map_err(TestCaseError::fail)?;
    }

    /// 7. SpawnRevocationDoesNotCascade: parent and descendant are separate levers.
    #[test]
    fn spawn_revocation_does_not_cascade(
        domain_value in any_domain(),
        actor_value in any_actor(),
        adapter_value in any_adapter(),
        session_suffix in "[a-z0-9]{1,12}",
        generation in 1u64..=4,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut registry = AuthorityRegistry::new();
            spawn_revocation_does_not_cascade_holds(
                &mut registry,
                &domain(&domain_value),
                &actor(&actor_value),
                &adapter_value,
                &format!("session-{session_suffix}"),
                generation,
            ).await.map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Supplementary IdempotentLogReplay: replay and the warm projection agree exactly.
    #[test]
    fn replay_matches_live(
        domain_value in any_domain(),
        plans in prop::collection::vec(any_replay_grant_plan(), 1..=8),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            replay_matches_live_holds(&domain(&domain_value), &plans)
                .await
                .map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }
}

// 8. ElicitationResponderAuthority: NOT TESTED HERE.
// Authority does not receive an Elicitation's expected_responder_actor and
// therefore cannot enforce response-Operation responder matching. This is the
// documented rev3 R6 gap, owned by the future acceptance responder-validation
// feature tracked in `.work/backlog/backlog-elicitation-responder-authority.md`.
// There is deliberately no vacuous stand-in assertion.

// ===== Mutation discipline =====

/// Mutant: acceptance derives the issuer actor from self-asserted
/// `Operation.sender` while retaining the verified transport context.
struct PayloadTrustingGrantCheck {
    registry: AuthorityRegistry,
    payload_actor: ActorId,
}

impl GrantCheck for PayloadTrustingGrantCheck {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        let payload_issuer = TestIssuerContext {
            actor: Some(self.payload_actor.clone()),
            endpoint: issuer.verified_endpoint().cloned(),
            device: issuer.verified_device().cloned(),
            generation: issuer.endpoint_generation(),
            domain: issuer.authority_domain_id().clone(),
        };
        self.registry
            .check(
                authority_domain_id,
                &payload_issuer,
                operation_kind,
                target_scope,
            )
            .await
    }
}

/// Mutant: revoking a parent adds every provenance-linked descendant to an
/// implicit cascade set, causing future checks for those grants to deny.
#[derive(Default)]
struct CascadingRegistry {
    inner: AuthorityRegistry,
    cascaded_grants: HashSet<GrantId>,
}

impl GrantLookup for CascadingRegistry {
    async fn current_grant(&self, grant_id: &GrantId) -> Option<GrantRecord> {
        self.inner.get_grant(grant_id).cloned()
    }
}

impl GrantProjection for CascadingRegistry {
    fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        if StoredEventKind::try_from(event.payload.kind).ok() == Some(StoredEventKind::Revocation) {
            let revocation =
                Revocation::decode(event.payload.payload.as_slice()).map_err(|error| {
                    AuthorityError::CorruptRecord(format!(
                        "cascade mutant cannot decode revocation: {error}"
                    ))
                })?;
            if let Some(parent_id) = revocation.grant_id {
                let mut frontier = HashSet::from([parent_id]);
                loop {
                    let discovered: Vec<_> = self
                        .inner
                        .live_grants()
                        .filter_map(|grant| match &grant.provenance {
                            GrantProvenanceKind::Descendant {
                                spawning_grant_id: Some(spawning_grant_id),
                                ..
                            } if frontier.contains(spawning_grant_id)
                                && !self.cascaded_grants.contains(&grant.grant_id) =>
                            {
                                Some(grant.grant_id.clone())
                            }
                            _ => None,
                        })
                        .collect();
                    if discovered.is_empty() {
                        break;
                    }
                    frontier = discovered.iter().cloned().collect();
                    self.cascaded_grants.extend(discovered);
                }
            }
        }
        self.inner.observe(event)
    }
}

impl GrantCheck for CascadingRegistry {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        let authorized = self
            .inner
            .check(authority_domain_id, issuer, operation_kind, target_scope)
            .await?;
        if authorized
            .grant_id
            .as_ref()
            .is_some_and(|id| self.cascaded_grants.contains(id))
        {
            return Err(GrantDenied::NoGrant {
                actor: format!("{:?}", issuer.verified_actor()),
                kind: operation_kind,
                target: format!("{target_scope:?}"),
            });
        }
        Ok(authorized)
    }
}

#[test]
fn payload_actor_trust_catches_injected_bug() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let authority_domain_id = domain("authority-mutation");
        let payload_actor = actor("payload-operator");
        let verified_actor = actor("verified-other-operator");
        let target_scope = adapter_scope("adapter-pi");
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let mut registry = AuthorityRegistry::new();
        ingest_live_grant(
            &storage,
            &mut registry,
            &authority_domain_id,
            "payload-grant",
            &payload_actor,
            &[OperationKind::Instruct],
            target_scope.clone(),
        )
        .await
        .unwrap();
        let operation = operation_with_payload_sender(
            &authority_domain_id,
            &payload_actor,
            OperationKind::Instruct,
            target_scope,
        );
        let issuer = TestIssuerContext::verified(verified_actor, authority_domain_id);

        assert!(
            compound_issuer_holds(&registry, &operation, &issuer)
                .await
                .is_ok(),
            "the production registry must use verified issuer identity"
        );

        let mutant = PayloadTrustingGrantCheck {
            registry,
            payload_actor,
        };
        assert!(
            compound_issuer_holds(&mutant, &operation, &issuer)
                .await
                .is_err(),
            "the CompoundIssuer oracle did not catch payload-actor trust"
        );
    });
}

#[test]
fn compound_issuer_integration_denies_payload_actor_mismatch_through_submit() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let authority_domain_id = domain("authority-submit-integration");
        let verified_actor = actor("verified-operator");
        let payload_actor = actor("self-asserted-payload-operator");
        let target_scope = adapter_scope("adapter-pi");
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let mut registry = AuthorityRegistry::new();

        // The only live grant belongs to the self-asserted payload actor. The
        // independently verified actor has no grant, so trusting the issuer
        // argument must deny while deriving authority from the payload would
        // authorize.
        ingest_live_grant(
            &storage,
            &mut registry,
            &authority_domain_id,
            "payload-actor-submit-grant",
            &payload_actor,
            &[OperationKind::Instruct],
            target_scope.clone(),
        )
        .await
        .unwrap();

        let operation = Operation {
            command_id: Some(CommandId {
                value: "compound-issuer-submit".to_owned(),
            }),
            authority_domain_id: Some(authority_domain_id.clone()),
            sender: Some(ActorEndpointRef {
                actor_id: Some(payload_actor.clone()),
                ..ActorEndpointRef::default()
            }),
            kind: OperationKind::Instruct as i32,
            target_scope: Some(target_scope),
            idempotency_key: "compound-issuer-submit-key".to_owned(),
            ..Operation::default()
        };

        let verified_issuer =
            TestIssuerContext::verified(verified_actor, authority_domain_id.clone());
        let verified_result = submit(
            &storage,
            &registry,
            &AlwaysResolvedTarget,
            &AlwaysAcceptedCommandState,
            &AlwaysAcceptedCommandState,
            &verified_issuer,
            operation.clone(),
        )
        .await
        .expect("authority denial is a successful submission response");

        assert_eq!(
            verified_result.outcome,
            SubmissionOutcome::Rejected as i32,
            "submit must reject when the verified actor lacks the payload actor's grant"
        );
        assert_eq!(
            verified_result.failure_code,
            FailureCode::AuthorizationDenied as i32,
            "the verified-issuer mismatch must fail at the authority boundary"
        );

        let payload_derived_issuer =
            TestIssuerContext::verified(payload_actor, authority_domain_id);
        let payload_derived_result = submit(
            &storage,
            &registry,
            &AlwaysResolvedTarget,
            &AlwaysAcceptedCommandState,
            &AlwaysAcceptedCommandState,
            &payload_derived_issuer,
            operation,
        )
        .await
        .expect("the payload-derived mutation reaches a submission outcome");

        assert_eq!(
            payload_derived_result.outcome,
            SubmissionOutcome::Accepted as i32,
            "deriving issuer identity from Operation.sender would wrongly authorize the payload actor"
        );
        assert_eq!(
            payload_derived_result.failure_code,
            FailureCode::Unspecified as i32,
            "the mutation demonstration must pass the real submit authority check"
        );
    });
}

#[test]
fn cascade_revocation_catches_injected_bug() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let authority_domain_id = domain("authority-mutation");
        let subject = actor("operator");

        let mut real = AuthorityRegistry::new();
        assert!(
            spawn_revocation_does_not_cascade_holds(
                &mut real,
                &authority_domain_id,
                &subject,
                "adapter-pi",
                "session-real",
                1,
            )
            .await
            .is_ok(),
            "the production registry must satisfy both revocation levers"
        );

        let mut mutant = CascadingRegistry::default();
        assert!(
            spawn_revocation_does_not_cascade_holds(
                &mut mutant,
                &authority_domain_id,
                &subject,
                "adapter-pi",
                "session-mutant",
                1,
            )
            .await
            .is_err(),
            "the non-cascade oracle did not catch cascading parent revocation"
        );
    });
}
