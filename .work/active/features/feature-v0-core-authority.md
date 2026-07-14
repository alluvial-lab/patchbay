---
id: feature-v0-core-authority
kind: feature
stage: implementing
tags: [security, protocol, foundation]
parent: epic-v0-core
depends_on: [feature-v0-core-persistence]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-14
---

# Feature: Authority, grants, and audit

## Brief

Build the authority layer: grants, revocation, spawn authority, descendant-grant creation, and audit. A grant authorizes a subject (an actor, optionally narrowed to an endpoint or endpoint class) to perform a set of OperationKinds against a target scope. Grants are explicit, revocable, and evaluated inside one authority domain. The authority feature implements the grant-check port that the acceptance pipeline calls before accepting an operation.

v0.1.0 is single-operator, so the authority model is simple in practice — the operator can do everything — but the model keeps actor, endpoint, grant, and audit concepts explicit so future multi-human coordination is possible without rework. Fleet-level spawn authority is in v0.1.0 scope (single-operator, single-core, not HA/multi-core). Non-cascading spawn-grant revocation and descendant-grant creation are stated-normative obligations.

This feature has the weakest formal backing: all `authority.qnt` properties are stated-normative (draft). The one promoted property that touches authority (`RevokedSessionCannotCommand`) lives in `csrf_browser.qnt` and models the browser/CSRF boundary — it is web-server-facing, not core-internal. The authority feature's obligations are real but not yet checked.

## Epic context

- Parent epic: `epic-v0-core`
- Position in epic: depends on persistence (grants and audit records are durable). Implements the grant-check port that acceptance calls; acceptance and authority can proceed in parallel after persistence lands because the port interface decouples them.

## Formal-model backing

- All `authority.qnt` properties are stated-normative (draft) — obligations the feature must satisfy but that do not yet have checked formulas. The v1 formal gate owns the real authority properties. Four were actively demoted during `epic-public-product-contract-verification-claim-correction` with documented demotion reasons (trace-fidelity defects: invariants inspect state written by the accepting action rather than independent attempted evidence; non-cascade formula not mutation-survivable).
- `RevokedSessionCannotCommand` (promoted, `csrf_browser.qnt`) — models the browser/CSRF boundary, NOT core-internal authority. Listed here only to clarify the boundary; it belongs to `feature-v0-web-server`.

## Foundation references

- `docs/PROTOCOL.md` — Authority grants; Spawn authority; Security and trust boundary
- `docs/SECURITY.md` — threat model, grants, revocation, audit, descendant grants, v0.1.0 authority domain
- `docs/ARCHITECTURE.md` — Authority and identity plane
- `docs/VERIFICATION.md` — stated-normative authority obligations
- `contracts/proto/patchbay/authority.proto` — `Grant`, `GrantProvenance`, `GrantRevocationPolicy`, `DescendantGrant`, `Revocation`
- `contracts/proto/patchbay/common.proto` — `ActorId`, `EndpointId`, `AuthorityDomainId`, `GrantId`, `TargetScope`, `ActorEndpointRef`, `StoredEventKind` (`GRANT=4`, `DESCENDANT_GRANT=5`, `REVOCATION=6`)
- `specs/seed/authority.qnt` — stated-normative authority obligations (7 properties, all draft)

## Design decisions (feature-design, 2026-07-13)

Resolved interactively with the operator after unpacking each option's trade-offs. These answers came AFTER a deliberate decision NOT to do a formal-backing pass first: `authority.qnt` has zero promoted properties (4 actively demoted for trace-fidelity / non-mutation-survivable defects), and the verification-claim-correction epic settled that the real formal uplift belongs to the v1 formal gate (`epic-public-product-contract-executable-release-assurance`), not v0.1.0. The design therefore treats the 7 stated-normative semantics as binding obligations the implementation must satisfy + property-testable oracles (mirroring how sessions shipped with one promoted property and the rest stated-normative), without over-claiming verification status.

- **Q1 — v0.1.0 grant population: hybrid (implicit operator authority + durable descendant grants).** Chosen over pure implicit (leaves descendant-grant + revocation logic untested — exactly the demoted formal properties' subject matter) and full durable grant set (more than single-operator v0.1.0 needs — the operator is the only actor). `GrantCheck` returns `Authorized { grant_id: None }` for the operator actor (the `None` grant_id is already reserved for exactly this per `ports.rs`). Descendant grants from spawn are durably recorded so the spawn→descendant-grant seam, revocation, and non-cascade are real, tested, and audited, while the operator's own authority is implicit. This makes the safety-relevant parts testable without ceremony.
- **Q2 — Grant storage: durable event-sourced.** Chosen over in-memory only (loses audit + crash-recovery; PROTOCOL.md says grants are durable; SECURITY.md requires the audit record). Grants/revocations are `StoredEventKind::Grant`/`DescendantGrant`/`Revocation` events (the discriminators already exist in common.proto); an `AuthorityRegistry` projection folds them, mirroring `SessionRegistry`/`CommandIndex`/`ElicitationSlotLayer`. Replay on startup from LSN 0 (same snapshot-discriminator gap as the other projections — deferred per the sessions feature's Q2).
- **Q3 — Descendant grant issuance: authority tails the log.** Chosen over acceptance-calls-authority-hook (couples acceptance→authority — Ports & Adapters violation) and authority-owns-spawn-ingress (spawn lifecycle is acceptance's; authority shouldn't own it). The descendant grant is a *reaction* to spawn completion — authority watches for `OPERATION` events of kind `Spawn` reaching terminal `Completed` (via `COMMAND_TRANSITION` events), and reacts by writing the descendant `DescendantGrant` event. This is exactly the elicitation-slot pattern (tail the log, react to command transitions). Keeps acceptance ignorant of descendant grants. The descendant grant event is written by authority in response to seeing a Completed spawn, with provenance linking back to the spawn operation + spawning grant.
- **Q4 — Scope: full feature with child stories.** Chosen over splitting (the descendant-grant and revocation are the security-critical parts — the demoted formal properties are about these; they should be in-scope and tested, not deferred). Implement everything: grant/revocation event model + `AuthorityRegistry` projection, `GrantCheck` impl, descendant-grant-on-spawn log-tail, revocation (non-cascade two-lever), proptests. ~5-6 child stories.
- **Q5 — Spawn authority modeling depth: full protocol model.** Chosen over minimal (the explicitly-enumerated descendant allowed-kind set is in PROTOCOL.md + the proto comment; non-cascade is a stated-normative obligation worth a property test — it's one of the demoted formal properties; we can't formally check it but we can property-test it). Model: fleet spawn grants, descendant grants with the explicitly-enumerated allowed-kind set (instruct/cancel/interrupt/query/approval-response/elicitation-response/reconfigure/session-management; spawn+attach excluded), two-lever non-cascade revocation, provenance. This is the security-critical surface; minimal would ship unverified safety logic.

## Architectural choice

A hybrid authority layer mirroring the sessions feature's established shape: the event log (owned by `feature-v0-core-persistence`) is the single source of truth for grant/revocation state. Authority writes `Grant`/`DescendantGrant`/`Revocation` events through the `Storage::append` port. An in-memory `AuthorityRegistry` is the hot lookup path, rebuilt from replay on startup. Snapshot checkpointing is deferred (replay from LSN 0), matching acceptance, elicitation, and sessions.

The authority feature owns its event kinds end-to-end (writer pattern, like sessions' `ingest_session_report`), EXCEPT for descendant-grant issuance which is a pure log-tail (like the elicitation-slot layer) — because descendant grants are a *reaction* to spawn completion events that acceptance owns. This split mirrors the codebase's established rule: if a feature owns its own state transitions, it writes them; if it reacts to another feature's events, it tails.

The `GrantCheck` port (already declared in `core/src/acceptance/ports.rs`) is implemented by the `AuthorityRegistry`. v0.1.0 `GrantCheck` is a hybrid: the operator actor resolves to implicit authority (`Authorized { grant_id: None }` — deny-by-default with the single operator as the universal subject), while non-operator subjects are evaluated against the durable grant set (descendant grants). Revocation is durable and enforced (a revoked descendant grant denies future authority); non-cascade is structural (revoking a spawn grant does NOT revoke already-issued descendant grants — two independent levers).

This shape honors Ports & Adapters (authority depends on `Storage` and implements `GrantCheck`; acceptance depends on the `GrantCheck` trait, not on authority), Single Source of Truth (the event log is the only source of grant state; the in-memory registry is a pure fold), Generated Contracts (`Grant`, `DescendantGrant`, `Revocation` are generated proto messages; `StoredEventKind::Grant`/`DescendantGrant`/`Revocation` are schema-owned discriminators), and Fail Fast (invalid grants, unknown grant kinds, and log corruption are rejected at the boundary).

## Implementation Units

### Unit 1: Grant/revocation event model and the `AuthorityRegistry` projection

**File**: `core/src/authority/mod.rs`, `core/src/authority/state.rs`, `core/src/authority/events.rs`, `core/src/authority/registry.rs`

**Story**: `story-v0-core-authority-registry`

The durable event shape (already-defined proto messages — `Grant`, `DescendantGrant`, `Revocation` — encoded under the existing `StoredEventKind` discriminators) and the in-memory projection that folds them. Mirrors `SessionRegistry`/`ElicitationSlotLayer`.

```rust
// core/src/authority/state.rs
use patchbay_contracts::patchbay::{
    ActorId, EndpointId, AuthorityDomainId, GrantId, TargetScope, OperationKind, Generation,
};

/// The in-memory grant record, derived from the event log.
/// Mirrors SessionRecord: a pure fold of Grant/DescendantGrant/Revocation events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantRecord {
    pub grant_id: GrantId,
    pub authority_domain_id: AuthorityDomainId,
    pub subject_actor_id: ActorId,
    pub subject_endpoint_id: Option<EndpointId>,
    pub subject_endpoint_class: String,
    pub target_scope: TargetScope,
    pub allowed_operation_kinds: Vec<OperationKind>,
    pub revocation_generation: Option<Generation>,
    pub revoked_at: Option<::prost_types::Timestamp>,
    pub is_descendant: bool,  // distinguishes Grant vs DescendantGrant events
    pub provenance: GrantProvenance,  // created_by + audit_id (or descendant spawn_operation_id + spawning_grant_id)
}

impl GrantRecord {
    /// A grant is live if not revoked and not expired.
    /// (Expiration enforcement is a stated-normative obligation; v0.1.0
    /// checks revocation durably. Expiry needs a clock — deferred with the
    /// time-driven staleness work, same as sessions.)
    pub fn is_live(&self) -> bool { self.revocation_generation.is_none() }
}

/// The canonical descendant-grant allowed-kind set from docs/PROTOCOL.md
/// "Spawn payload and authority commitments". Single source of truth — the
/// explicitly-enumerated existing-session OperationKinds (spawn + attach
/// excluded). This is a protocol fact, not invented.
pub const DESCENDANT_GRANT_ALLOWED_KINDS: &[OperationKind] = &[
    OperationKind::Instruct,
    OperationKind::Cancel,
    OperationKind::Interrupt,
    OperationKind::Query,
    OperationKind::ApprovalResponse,
    OperationKind::ElicitationResponse,
    OperationKind::Reconfigure,
    OperationKind::SessionManagement,
];

/// Does `grant` authorize `kind` against `target_scope`? The grant-matching
/// predicate (deny-by-default). Mirrors authority.qnt's `actionGrantAuthorizes*`
/// but as an independent oracle over grant state, not action-recorded state.
#[must_use]
pub fn grant_authorizes(
    grant: &GrantRecord,
    actor: &ActorEndpointRef,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
) -> bool {
    grant.is_live()
        && grant.subject_actor_id == actor.actor_id  // subject matches
        && grant.allowed_operation_kinds.contains(&operation_kind)  // kind allowed
        && target_scope_matches(&grant.target_scope, target_scope)  // target in scope
    // endpoint narrowing: if grant has subject_endpoint_id, the actor's
    // endpoint must match (v0.1.0: operator grants are endpoint-unscoped)
}
```

```rust
// core/src/authority/registry.rs
use std::collections::HashMap;
use patchbay_contracts::patchbay::{AuthorityDomainId, GrantId, StoredEventKind};
use crate::storage::{RecordedEvent, Storage};

/// The in-memory authority projection. Rebuilt from replay on startup.
/// Mirrors SessionRegistry / ElicitationSlotLayer: HashMap-backed, observe(event) fold.
#[derive(Debug, Clone, Default)]
pub struct AuthorityRegistry {
    grants: HashMap<GrantId, GrantRecord>,
}

impl AuthorityRegistry {
    pub fn new() -> Self { Self::default() }

    /// Fold one committed event into the authority projection.
    /// Consumes Grant / DescendantGrant / Revocation events; ignores others.
    /// Idempotent for re-delivered events. Validates grant shape (Fail Fast).
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            AuthorityError::CorruptRecord(format!("unknown stored event kind {}", event.payload.kind))
        })?;
        match kind {
            StoredEventKind::Grant => self.observe_grant(event),
            StoredEventKind::DescendantGrant => self.observe_descendant_grant(event),
            StoredEventKind::Revocation => self.observe_revocation(event),
            _ => Ok(()),  // ignore non-authority events
        }
    }

    /// Look up a grant by id.
    pub fn get_grant(&self, grant_id: &GrantId) -> Option<&GrantRecord> { ... }

    /// Iterate live grants (for GrantCheck evaluation / tests).
    pub fn live_grants(&self) -> impl Iterator<Item = &GrantRecord> { ... }
}
```

**Implementation Notes**:
- `GrantRecord` is the in-memory projection of BOTH `Grant` and `DescendantGrant` proto messages (the `is_descendant` flag distinguishes them; descendant grants carry spawn provenance, regular grants carry created_by provenance). Mirrors how `SessionRecord` projects `SessionState` events.
- `DESCENDANT_GRANT_ALLOWED_KINDS` is the SSOT for the descendant allowed-kind set, copied verbatim from PROTOCOL.md "Spawn payload and authority commitments". The proto comment on `DescendantGrant.allowed_operation_kinds` already documents it; this constant is the Rust-side authority.
- `grant_authorizes` is the grant-matching predicate — deny-by-default (SECURITY.md: "Authorization is deny-by-default"). It checks: live, subject matches, kind allowed, target in scope. Endpoint narrowing is conditional (operator grants are endpoint-unscoped in v0.1.0).
- `observe` validates grant shape (non-empty grant_id, subject, target_scope; valid OperationKinds; descendant grants must have exactly the allowed-kind set) and returns `CorruptRecord`/`CorruptLog` on violation (Fail Fast, mirroring `SessionRegistry::observe`).
- `observe_revocation` marks the matching grant revoked (sets `revocation_generation`, `revoked_at`) — does NOT delete it (audit retention). A revocation for an unknown grant_id is `CorruptLog`. Idempotent for re-delivered revocations.
- First-write-wins on duplicate `Grant`/`DescendantGrant` events (idempotent replay, mirroring `observe_registered`).

**Acceptance Criteria**:
- [ ] `AuthorityRegistry::observe` folds Grant, DescendantGrant, and Revocation events correctly
- [ ] A revocation marks the grant revoked (not deleted); subsequent `is_live()` returns false
- [ ] `grant_authorizes` returns true only when live + subject matches + kind allowed + target in scope
- [ ] `DESCENDANT_GRANT_ALLOWED_KINDS` matches PROTOCOL.md exactly (8 kinds, spawn+attach excluded)
- [ ] `observe` rejects malformed grants (empty grant_id, unknown OperationKind, descendant with wrong allowed-kinds) as `CorruptRecord`
- [ ] `observe` is idempotent for re-delivered events

---

### Unit 2: `GrantCheck` impl (the acceptance seam)

**File**: `core/src/authority/check.rs`, `core/src/authority/resolver.rs`

**Story**: `story-v0-core-authority-grant-check`

Implements the `GrantCheck` port (declared in `core/src/acceptance/ports.rs`) on the `AuthorityRegistry`. v0.1.0 hybrid: operator actor → implicit authority; non-operator → durable grant evaluation.

```rust
// core/src/authority/check.rs
use patchbay_contracts::patchbay::{AuthorityDomainId, ActorEndpointRef, OperationKind, TargetScope, ActorId};
use crate::acceptance::ports::{GrantCheck, Authorized, GrantDenied};
use super::registry::AuthorityRegistry;
use super::state::grant_authorizes;

/// The v0.1.0 operator actor id. Single-operator deployment: this actor
/// has implicit authority (Authorized { grant_id: None }). The `None`
/// grant_id is reserved for exactly this (ports.rs Authorized.grant_id doc).
/// TODO(v1): replace with durable operator grants when multi-operator lands.
const OPERATOR_ACTOR_ID: &str = "operator";  // configured at deployment

impl GrantCheck for AuthorityRegistry {
    async fn check(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        actor: &ActorEndpointRef,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        // Q1 hybrid: operator actor → implicit authority.
        if is_operator(actor) {
            return Ok(Authorized { grant_id: None });
        }
        // Non-operator: evaluate against durable grants (descendant grants).
        // Deny-by-default: any live grant that authorizes this (actor, kind, target).
        for grant in self.live_grants() {
            if grant_authorizes(grant, actor, operation_kind, target_scope) {
                return Ok(Authorized { grant_id: Some(grant.grant_id.clone()) });
            }
        }
        Err(GrantDenied::NoGrant {
            actor: format!("{:?}", actor.actor_id),
            kind: operation_kind,
            target: format!("{:?}", target_scope),
        })
    }
}

fn is_operator(actor: &ActorEndpointRef) -> bool {
    actor.actor_id.as_ref().is_some_and(|a| a.value == OPERATOR_ACTOR_ID)
}
```

**Implementation Notes**:
- The `GrantCheck` port signature already exists in `ports.rs` — this story implements it, exactly mirroring how sessions implemented `TargetResolver`.
- v0.1.0 hybrid: the operator actor resolves to `Authorized { grant_id: None }` (implicit). The `OPERATOR_ACTOR_ID` constant is the single-operator assumption made explicit — a v0.1.0 deployment value, not a protocol constant. (Future multi-operator: this becomes a durable operator grant.)
- Non-operator subjects (e.g. a spawned session acting as a subject — future agent-to-agent) are evaluated against the durable grant set. In v0.1.0 the only non-operator grants are descendant grants from spawn.
- Deny-by-default: if no live grant authorizes, return `GrantDenied::NoGrant`. This is the SECURITY.md invariant.
- The acceptance pipeline currently calls `grant_check.check(...).await.is_err()` — it uses the authorization decision but discards the `Authorized` evidence. This story doesn't change that; it just provides a real impl (the existing `TestGrantCheck` in `core/tests/acceptance_pipeline.rs` can be replaced by a real `AuthorityRegistry` in integration tests).

**Acceptance Criteria**:
- [ ] `GrantCheck::check` returns `Authorized { grant_id: None }` for the operator actor
- [ ] `GrantCheck::check` returns `Authorized { grant_id: Some(...) }` for a non-operator with a live matching grant
- [ ] `GrantCheck::check` returns `GrantDenied::NoGrant` for a non-operator with no matching grant (deny-by-default)
- [ ] `GrantCheck::check` returns `GrantDenied` for a revoked grant (revocation prevents future authority)
- [ ] `GrantCheck::check` returns `GrantDenied` for a kind not in the grant's allowed set

---

### Unit 3: Grant + revocation ingestion (the writer)

**File**: `core/src/authority/ingest.rs`

**Story**: `story-v0-core-authority-ingest`

The direct ingestion writer for grants and revocations — the analog of sessions' `ingest_session_report` / acceptance's `ingest_observation`. Owns its event kinds end-to-end (writer pattern, Q2).

```rust
// core/src/authority/ingest.rs
use patchbay_contracts::patchbay::{
    AuthorityDomainId, Grant, DescendantGrant, Revocation, GrantId, EventId,
    StoredEventKind, StoredEventPayload,
};
use prost::Message;
use crate::storage::Storage;
use super::registry::AuthorityRegistry;
use super::AuthorityError;

/// Read access to the live grant projection (mirrors SessionLookup).
pub trait GrantLookup: Send + Sync {
    fn current_grant(
        &self,
        grant_id: &GrantId,
    ) -> impl std::future::Future<Output = Option<super::state::GrantRecord>> + Send;
}

/// Durably record a grant. Writes a Grant event.
pub async fn ingest_grant<S, L>(
    storage: &S,
    lookup: &L,
    grant: Grant,
) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantLookup, { ... }

/// Durably record a descendant grant (from spawn completion).
/// Writes a DescendantGrant event. Validates the allowed-kind set matches
/// DESCENDANT_GRANT_ALLOWED_KINDS exactly (Fail Fast).
pub async fn ingest_descendant_grant<S, L>(
    storage: &S,
    lookup: &L,
    grant: DescendantGrant,
) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantLookup, { ... }

/// Durably record a revocation. Writes a Revocation event.
/// Two-lever non-cascade: revoking grant G marks G revoked; it does NOT
/// revoke descendant grants issued under G (they have their own grant_ids
/// and must be revoked separately). The registry fold enforces this.
pub async fn ingest_revocation<S, L>(
    storage: &S,
    lookup: &L,
    revocation: Revocation,
) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantLookup, { ... }
```

**Implementation Notes**:
- Writer pattern mirroring `ingest_session_report` / `ingest_observation`: validate → read current (for revocation, confirm the grant exists) → write the delta event → warm the registry → return.
- `ingest_descendant_grant` validates the allowed-kind set matches `DESCENDANT_GRANT_ALLOWED_KINDS` exactly (reject if it includes spawn/attach or omits a required kind — Fail Fast, Q5).
- `ingest_revocation` is the two-lever non-cascade enforcement point: it revokes ONLY the named grant. The registry fold (`observe_revocation`) marks that one grant revoked and touches no other. There is no cascade code path. (The non-cascade is structural: there's simply no mechanism to cascade.)
- Warm-after-write mirrors sessions' pattern (post-B5 fix: warm after each successful append so retry is idempotent).
- Encoding: `grant.encode_to_vec()` under `StoredEventPayload { kind: StoredEventKind::Grant as i32, payload }`.

**Acceptance Criteria**:
- [ ] `ingest_grant` writes a Grant event and the registry reflects it
- [ ] `ingest_descendant_grant` rejects a descendant with the wrong allowed-kind set (spawn included, or a required kind missing)
- [ ] `ingest_revocation` writes a Revocation event and marks ONLY the named grant revoked
- [ ] `ingest_revocation` does NOT revoke descendant grants issued under the revoked grant (non-cascade, two-lever)
- [ ] Revoking a non-existent grant returns an error (Fail Fast)
- [ ] Warm-after-write keeps the registry consistent (retry-safe, per the sessions B5 fix pattern)

---

### Unit 4: Descendant-grant-on-spawn log-tail (the reactor)

**File**: `core/src/authority/spawn_tail.rs`

**Story**: `story-v0-core-authority-spawn-tail`

The pure log-tail that reacts to spawn completion by issuing the descendant grant. Exactly the elicitation-slot pattern (tail the log, react to command transitions).

```rust
// core/src/authority/spawn_tail.rs
use patchbay_contracts::patchbay::{AuthorityDomainId, CommandId, OperationKind, OperationState, StoredEventKind};
use crate::storage::{RecordedEvent, Storage};
use super::registry::AuthorityRegistry;
use super::state::DESCENDANT_GRANT_ALLOWED_KINDS;
use super::AuthorityError;

/// An independent event-log consumer that issues descendant grants when
/// spawn Operations reach terminal Completed.
///
/// Mirrors ElicitationSlotLayer: owns no storage writes of its own for the
/// *trigger* (it reads COMMAND_TRANSITION events acceptance wrote), but it
/// DOES write the descendant grant event in reaction. The tail is read-only
/// over the command log; the reaction is a writer call (Unit 3's
/// ingest_descendant_grant).
///
/// Because the authority-domain log is delivered in LSN order, the first
/// Completed transition for a spawn operation structurally wins (no duplicate
/// descendant grants on replay).
#[derive(Debug, Clone, Default)]
pub struct SpawnDescendantTail {
    /// command_id -> (spawn operation kind confirmed) for Spawn OPERATION events seen.
    /// Used to confirm a COMMAND_TRANSITION to Completed belongs to a spawn.
    spawn_commands: HashMap<CommandId, ()>,
    /// command_ids whose descendant grant has been issued (idempotent on replay).
    issued: HashSet<CommandId>,
}

impl SpawnDescendantTail {
    pub fn new() -> Self { Self::default() }

    /// Fold one committed event. On a Completed transition for a Spawn
    /// operation not yet issued, produce a `DescendantGrantIssuance`
    /// describing the descendant grant to write.
    pub fn observe(
        &mut self,
        event: &RecordedEvent,
    ) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
        // Track Spawn OPERATION events (command_id -> ()).
        // On COMMAND_TRANSITION to Completed for a tracked spawn command,
        // and not yet issued, return Some(issuance) with provenance
        // (spawn_operation_id, spawning_grant_id) + the target scope from
        // the spawn operation + the canonical allowed-kind set.
        // The caller (composition layer) then calls ingest_descendant_grant.
    }
}

/// Describes the descendant grant to issue for a completed spawn.
pub struct DescendantGrantIssuance {
    pub spawn_operation_id: CommandId,
    pub spawning_grant_id: Option<GrantId>,  // from the spawn operation's authorization
    pub target_scope: TargetScope,  // the spawned session
    pub subject_actor_id: ActorId,  // the spawner (operator in v0.1.0)
    pub allowed_operation_kinds: Vec<OperationKind>,  // DESCENDANT_GRANT_ALLOWED_KINDS
}
```

**Implementation Notes**:
- This is the pure-tail pattern (like `ElicitationSlotLayer`): reads `OPERATION` + `COMMAND_TRANSITION` events acceptance wrote, reacts to `Spawn → Completed`. The tail itself writes nothing; it produces an `Issuance` that the composition layer feeds to `ingest_descendant_grant` (Unit 3's writer).
- `spawn_commands` tracks which command_ids are spawn operations (from `OPERATION` events where `OperationKind::Spawn`). `issued` tracks which have already produced a descendant grant (idempotent on replay — first-Completed-wins, mirroring elicitation's first-answer-wins).
- The descendant grant's target_scope is the spawned session (from the spawn operation's target, now that the session exists). The subject is the spawner (operator in v0.1.0). The allowed-kinds are `DESCENDANT_GRANT_ALLOWED_KINDS`. Provenance links `spawn_operation_id` + `spawning_grant_id`.
- `spawning_grant_id` comes from the spawn operation's authorization evidence — but the acceptance pipeline currently discards `Authorized.grant_id`. This is a known gap: to populate provenance, either (a) the composition layer that drives the tail must correlate the spawn's authorization (requires the pipeline to retain it), or (b) the tail infers it from the grant set. **Flag this as an integration detail to resolve in the story** — it may require acceptance to retain the `Authorized` evidence on the command record (a small acceptance change), or the descendant grant provenance's `spawning_grant_id` is `None` in v0.1.0 (operator's implicit authority has no grant_id). Prefer (b) for v0.1.0: `spawning_grant_id: None` when the spawn was authorized by implicit operator authority. Document this.

**Acceptance Criteria**:
- [ ] A spawn `OPERATION` followed by a `COMMAND_TRANSITION` to `Completed` produces exactly one `DescendantGrantIssuance`
- [ ] A spawn that reaches a non-Completed terminal (Rejected/Failed/etc.) produces NO issuance
- [ ] Replay (re-observing the same events) does not produce duplicate issuances (idempotent)
- [ ] The issuance's allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly
- [ ] The issuance's provenance links the spawn_operation_id; `spawning_grant_id` is `None` for operator-authorized spawns (v0.1.0)

---

### Unit 5: Replay and module wiring

**File**: `core/src/authority/replay.rs`, `core/src/authority/mod.rs`, `core/src/lib.rs`

**Story**: `story-v0-core-authority-replay`

Rebuild the registry from the log (mirroring `rebuild_from_log` in sessions/elicitation) and wire the module into the crate.

```rust
// core/src/authority/replay.rs
use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};
use crate::storage::Storage;
use super::registry::AuthorityRegistry;
use super::AuthorityError;

/// Rebuild an authority registry by replaying one authority domain.
/// v0.1.0 replays from LSN 0 (snapshot discriminator gap — deferred, matches
/// acceptance/elicitation/sessions).
pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<AuthorityRegistry, AuthorityError> {
    // Near-exact copy of session::rebuild_from_log / elicitation::rebuild_slots_from_log:
    // read_after(domain, Lsn{0}), fold via observe, validate LSN monotonicity + domain match.
}
```

```rust
// core/src/authority/mod.rs
pub mod state;
pub mod events;
pub mod registry;
pub mod check;
pub mod ingest;
pub mod spawn_tail;
pub mod replay;

pub use state::{GrantRecord, grant_authorizes, DESCENDANT_GRANT_ALLOWED_KINDS};
pub use registry::AuthorityRegistry;
pub use check::GrantCheckImpl;  // or the impl is on AuthorityRegistry directly
pub use ingest::{ingest_grant, ingest_descendant_grant, ingest_revocation, GrantLookup};
pub use spawn_tail::{SpawnDescendantTail, DescendantGrantIssuance};
pub use replay::rebuild_from_log;

#[derive(Debug, thiserror::Error)]
pub enum AuthorityError {
    #[error("corrupt authority record: {0}")]
    CorruptRecord(String),
    #[error("corrupt authority log: {0}")]
    CorruptLog(String),
    #[error("invalid grant shape: {0}")]
    InvalidGrant(String),
    #[error("grant not found: {0}")]
    GrantNotFound(String),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
```

```rust
// core/src/lib.rs — add alongside acceptance, session, storage
pub mod acceptance;
pub mod authority;  // NEW
pub mod session;
pub mod storage;
```

**Acceptance Criteria**:
- [ ] `rebuild_from_log` reconstructs the registry identically to a live registry that observed the same events
- [ ] `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`
- [ ] `core/src/authority/` module compiles and is exported from `core/src/lib.rs`
- [ ] The existing `TestGrantCheck` in `core/tests/acceptance_pipeline.rs` can be replaced by a real `AuthorityRegistry` in an integration test

---

### Unit 6: Property tests for authority invariants

**File**: `core/tests/authority_proptest.rs`

**Story**: `story-v0-core-authority-proptests`

Property tests for the stated-normative obligations. None are formally checked (all `authority.qnt` properties are draft), but each is testable as an executable oracle — mirroring how sessions tested stated-normative obligations as properties.

```rust
// core/tests/authority_proptest.rs
proptest! {
    /// NoCommandWithoutGrant (stated-normative): a non-operator command that
    /// reaches accepted state does so only with a live matching grant.
    /// Deny-by-default: no grant -> denied.
    #[test]
    fn no_command_without_grant(/* ... */) { ... }

    /// RevocationPreventsFuture (stated-normative): after a grant is revoked,
    /// subsequent check() for that grant's subject/kind/target is denied.
    #[test]
    fn revocation_prevents_future(/* ... */) { ... }

    /// SpawnRevocationDoesNotCascade (stated-normative — one of the demoted
    /// formal properties): revoking a spawn grant does NOT revoke descendant
    /// grants issued under it. Two independent levers.
    #[test]
    fn spawn_revocation_does_not_cascade(/* ... */) { ... }

    /// DescendantGrantAllowedKindsExact (stated-normative): a descendant grant
    /// issued on spawn completion has EXACTLY the canonical allowed-kind set
    /// (spawn + attach excluded).
    #[test]
    fn descendant_grant_allowed_kinds_exact(/* ... */) { ... }

    /// Replay determinism.
    #[test]
    fn replay_matches_live(/* ... */) { ... }
}

// Mutation tests (non-vacuity): a buggy registry that cascades revocation
// MUST fail spawn_revocation_does_not_cascade. Mirrors acceptance/sessions
// mutation-test discipline.
```

**Implementation Notes**:
- All properties are stated-normative (no promoted formulas). They document + enforce the intended behavior as executable oracles, the same way sessions tested `LateGenerationInert` and `LabelsCannotOverrideIdentity`.
- `spawn_revocation_does_not_cascade` is the most valuable: it's one of the actively-demoted formal properties (demoted because the formula wasn't mutation-survivable). The property test here IS mutation-survivable by construction — a buggy registry that cascades will fail it. This is the executable stand-in for the demoted formal property.
- Mutation tests are essential (non-vacuity discipline established by acceptance/sessions proptests).

**Acceptance Criteria**:
- [ ] `no_command_without_grant` passes (deny-by-default)
- [ ] `revocation_prevents_future` passes
- [ ] `spawn_revocation_does_not_cascade` passes AND FAILS against a mutation that cascades (non-vacuous — this is the executable stand-in for the demoted formal property)
- [ ] `descendant_grant_allowed_kinds_exact` passes
- [ ] `replay_matches_live` passes

---

## Implementation Order

1. `story-v0-core-authority-registry` — grant/revocation event model + `AuthorityRegistry` projection (no deps; the SSOT for grant state + `grant_authorizes` predicate)
2. `story-v0-core-authority-grant-check` — `impl GrantCheck for AuthorityRegistry` (depends on 1)
3. `story-v0-core-authority-ingest` — grant/revocation/descendant-grant writer (depends on 1)
4. `story-v0-core-authority-spawn-tail` — descendant-grant-on-spawn log-tail reactor (depends on 1, 3)
5. `story-v0-core-authority-replay` — `rebuild_from_log` + module wiring (depends on 1, 2, 3)
6. `story-v0-core-authority-proptests` — property tests for stated-normative obligations (depends on 1-5)

Stories 1 is the foundation. 2 and 3 can proceed in parallel after 1 lands (GrantCheck impl and the writer both depend on the registry but not each other). 4 depends on 1 + 3 (the tail produces issuances the writer consumes). 5 depends on 1-3. 6 depends on all.

## Testing

### Unit Tests: `core/tests/authority_*.rs`
- `authority_registry.rs` — fold correctness, revocation marks-not-deletes, idempotent observe, malformed-grant rejection
- `authority_grant_check.rs` — operator implicit, non-operator grant match, deny-by-default, revoked grant denied, kind-mismatch denied
- `authority_ingest.rs` — grant/descendant/revocation ingestion, descendant allowed-kind validation, non-cascade revocation
- `authority_spawn_tail.rs` — spawn-Completed produces issuance, non-Completed terminal produces none, idempotent replay
- `authority_replay.rs` — replay determinism, LSN monotonicity, cross-domain rejection
- `authority_proptest.rs` — property oracles + mutation tests

### Integration Points
- **Acceptance ↔ Authority**: acceptance calls `GrantCheck::check` (implemented by `AuthorityRegistry`) before accepting an operation. The existing `TestGrantCheck` is replaced by a real `AuthorityRegistry` in integration tests.
- **Authority ↔ Sessions**: no direct coupling. Authority's `target_scope` matching uses the `TargetScope` type (sessions owns session identity; authority matches grant target scope against it).
- **Authority ↔ Storage**: authority writes Grant/DescendantGrant/Revocation events via `Storage::append` and reads via `Storage::read_after` for replay. Same `Storage` port.
- **Authority ↔ Elicitation**: no direct coupling. Both are independent log consumers (the spawn-tail reads command transitions like the elicitation-slot layer does).

## Risks

- **Weakest formal backing.** All `authority.qnt` properties are stated-normative; four were actively demoted (trace-fidelity + non-mutation-survivable). The property tests (Unit 6) are executable oracles, NOT formally checked properties. The design must not over-claim verification status. The v1 formal gate owns the real authority properties (independent attempted-evidence state for the trace-fidelity defects; mutation-survivable non-cascade oracle). This is documented in the feature body and the `@promotion` blocks.
- **Implicit operator authority is a v0.1.0 simplification.** `GrantCheck` returns `Authorized { grant_id: None }` for the operator actor. This is a documented v0.1.0 assumption (single operator), not a protocol constant. Future multi-operator work replaces it with durable operator grants — a reversal, not a gap-fill. The `OPERATOR_ACTOR_ID` constant makes the assumption explicit and locatable.
- **Descendant grant provenance `spawning_grant_id`.** For operator-authorized spawns, `spawning_grant_id` is `None` (implicit authority has no grant_id). This means descendant-grant provenance is partially populated in v0.1.0. The protocol allows this (the descendant grant is still an explicit record); the provenance seam is preserved for future durable operator grants. Documented in Unit 4.
- **Snapshot checkpointing deferred.** Same as sessions/acceptance/elicitation — replay from LSN 0. The projection-discriminator gap is a cross-cutting storage concern, tracked separately.
- **Spawn-tail reaction ordering.** The tail produces a `DescendantGrantIssuance` on observing a Completed spawn; the composition layer must call `ingest_descendant_grant` to make it durable. If the composition layer crashes between observing and writing, the descendant grant is lost — but replay re-observes the Completed transition and re-issues. This is the same idempotent-replay property the elicitation layer relies on. The composition layer (not in this feature's scope) owns the wiring.

## Extension pressure classification

- **Committed v0.1.0 behavior**: deny-by-default grant evaluation; the `GrantCheck` port (existence + matching, with implicit operator authority); durable Grant/DescendantGrant/Revocation events; revocation marks-not-deletes (audit retention); the descendant-grant allowed-kind set (8 existing-session kinds, spawn+attach excluded); two-lever non-cascade revocation (structural — no cascade mechanism); descendant-grant-on-spawn via log-tail; the `AuthorityDomainId` key shape (federation seam).
- **Reserved seam**: multi-operator authority domains + durable operator grants (replaces the implicit operator authority — a reversal, not a gap-fill); delegation lineage (`parent_grant_id` — explicitly absent per PROTOCOL.md); per-spawn-variant authority; tighter endpoint-class narrowing; expiration enforcement (needs a clock — deferred with time-driven staleness); cascade-revocation as a query (future, no schema change).
- **Explicitly rejected**: trusting self-asserted actor identity (SECURITY.md "compound issuer" — sender identity is verified, never self-asserted); adapter capability declarations as grant authority (capability ≠ authority); making acceptance write descendant grants (Ports & Adapters violation — authority owns the reaction); cascading revocation as v0.1.0 behavior (two-lever is the v0.1.0 rule).
