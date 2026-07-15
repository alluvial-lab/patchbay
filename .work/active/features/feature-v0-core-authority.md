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
- `docs/SECURITY.md` — threat model, grants, revocation, audit, descendant grants, v0.1.0 authority domain, compound issuer
- `docs/ARCHITECTURE.md` — Authority and identity plane
- `docs/VERIFICATION.md` — stated-normative authority obligations (8 properties)
- `contracts/proto/patchbay/authority.proto` — `Grant`, `GrantProvenance`, `GrantRevocationPolicy`, `DescendantGrant`, `Revocation`
- `contracts/proto/patchbay/common.proto` — `ActorId`, `EndpointId`, `AuthorityDomainId`, `GrantId`, `TargetScope`, `TargetScopeKind`, `ActorEndpointRef`, `TypedCorrelation`, `StoredEventKind` (`GRANT=4`, `DESCENDANT_GRANT=5`, `REVOCATION=6`)
- `specs/seed/authority.qnt` — stated-normative authority obligations (8 properties, all draft)

## Design decisions (feature-design, revision 3, 2026-07-13)

This revision supersedes revisions 1 and 2. Revisions 1-2 aimed for a *live* vertical slice (durable bootstrap operator grants + a live descendant-grant reactor wired to a live consumer loop). Two design reviews (cross-model openai-codex/gpt-5.6-sol) found that "live" drove most of the complexity and under-specification: the bootstrap grant shape was a 50/50, `IssuerContext` couldn't be a real compound-issuer without the ingress (at `drafting`), the composition layer had no live event source to consume, and the live path was blocked anyway by fleet-target-resolution (backlog).

**Revision 3 drops the "live" framing.** v0.1.0 authority is a **component-complete, tested layer**: grants/revocation/descendant-grants/non-cascade as durable event-sourced state + property-test oracles, exercised via test doubles and replay — NOT a live end-to-end path. This matches the SPEC.md verification floor (line 70: "general authority, fleet-spawn authority, non-cascading spawn-grant revocation, descendant-grant creation... stay stated-normative until models represent their claimed failure boundaries") and how sessions shipped (one promoted property, rest stated-normative-but-property-tested). The live operator-issuing path (ingress + fleet-target-resolution + live wiring) is follow-on work.

- **R1 — Operator authority: no bootstrap grant; GrantCheck evaluates against durable grants (injected in tests).** Dropped the bootstrap operator grant (rev2 R1a) — it was a 50/50 (one grant vs two) and only mattered for a live path that doesn't exist in v0.1.0. v0.1.0 `GrantCheck` evaluates the verified issuer against whatever durable grants exist; in tests, those are injected (operator grant, descendant grants, revoked grants). The operator-auth provisioning (bootstrap grant, operator identity) is the ingress's job — deferred with `feature-v0-protocol-seam`/`feature-v0-web-server`. This removes the R1a-vs-R1b tension: there's no implicit bypass AND no ceremonial bootstrap; the machinery is exercised by injected grants in tests, and the live provisioning is follow-on.
- **R2 — `IssuerContext` port: simple verified-identity port + test double; real verifier is the ingress.** Retained from rev2 (the self-asserted-payload blocker stands), but dropped the "real compound-issuer now" ambition. The port carries verified actor/endpoint/device/generation/domain; v0.1.0 tests supply a `TestIssuerContext`; the real verifier (operator-session evidence, transport principal) lands with the ingress. **Domain-equality pinned** (rev2 finding B): `grant_authorizes` checks the issuer's verified domain against the grant's domain AND the requested domain — no payload-domain-override hole. The port is an opaque-ish trait (the ingress constructs it); acceptance takes `&dyn IssuerContext`.
- **R3 — Descendant-grant reactor: order-independent join, exercised via replay/direct observe (not a live consumer).** Retained the reactor (rev2 R3a) but dropped the live-composition-layer requirement (rev2 finding E). The reactor is a pure fold (`SpawnDescendantTail::observe`) that produces an `Issuance` on observing a completed spawn + correlated `SessionRegistered.spawn_origin`. **Order-independent** (rev2 finding D): it retains all three facts (spawn op, completion, registration) separately and runs `try_issue` after any of them, so arrival order doesn't matter. It's exercised in tests by feeding events in any order + by replay. No live consumer loop, no composition root, no cursor catch-up — those are follow-on when the ingress exists. Durable idempotency for tests = deterministic grant_id derived from `(authority_domain_id, spawn_command_id)` (the test harness uses `ingest_descendant_grant` with that ID; a re-observe produces the same ID → no-op). Depends on `story-sessions-spawn-origin-field` (the `spawn_origin` proto field).
- **R4 — Audit: minimal (grant/revocation events ARE the grant-lifecycle audit; distinct failed-authorization audit deferred).** Unchanged from rev2. The durable `Grant`/`DescendantGrant`/`Revocation` events with provenance satisfy grant-lifecycle audit. The distinct failed-authorization audit record (SECURITY.md) is a separate, cross-cutting concern — deferred, filed as backlog. The feature does NOT claim full audit.
- **R5 — Fleet target resolution: out of scope (backlog).** Unchanged. The existing `SessionRegistry`-backed `TargetResolver` rejects fleet spawn targets; that's an acceptance/sessions gap, filed as backlog. Authority's GrantCheck + grant model are testable independently.
- **R6 (new) — ElicitationResponderAuthority: narrowed; not enforced by authority.** Rev2 finding G: property #8 requires comparing the verified issuer with `Elicitation.expected_responder_actor`, but neither `GrantCheck` nor acceptance receives the Elicitation, and the elicitation projection checks only kind+correlation. Authority does NOT own response-Operation responder validation — that's an acceptance/elicitation concern. The property is listed in Unit 6 as a **documented untested gap** (not a vacuous test): authority's GrantCheck does not enforce responder matching; the obligation is real but owned by a future acceptance/elicitation responder-validation feature. This is honest — no vacuous stand-in.

### Retained from revisions 1-2 (unchanged by the reviews)
- **Durable event-sourced storage**: Grant/DescendantGrant/Revocation events under the existing `StoredEventKind` discriminators; `AuthorityRegistry` projection folds them, mirroring `SessionRegistry`/`ElicitationSlotLayer`. Replay from LSN 0.
- **Full protocol model**: fleet spawn grants, descendant grants with the explicitly-enumerated allowed-kind set (8 kinds, spawn+attach excluded), two-lever non-cascade revocation (structural), provenance. Deny-by-default.
- **Full grant-matching matrix** (rev2 finding #3, pinned): domain equality, verified actor, endpoint narrowing, kind membership, `target_scope_matches` scope-containment. Expiry stored but not enforced (clock — backlog).

## Architectural choice

A durable, event-sourced authority layer that is component-complete and tested, not live-wired. The event log (owned by `feature-v0-core-persistence`) is the single source of truth for grant/revocation state. Authority writes `Grant`/`DescendantGrant`/`Revocation` events through the `Storage::append` port. An in-memory `AuthorityRegistry` is the hot lookup path, rebuilt from replay on startup (replay from LSN 0, matching the other projections). No bootstrap grant, no live consumer loop, no composition root — those are follow-on when the ingress exists.

The authority feature owns its event kinds end-to-end (writer pattern, like sessions' `ingest_session_report`). The descendant-grant reactor is a pure fold (`SpawnDescendantTail::observe`) — order-independent, exercised via replay/direct observe in tests. It produces an `Issuance` (not a side-effecting write); the test harness (and, later, a follow-on composition layer) feeds the issuance to `ingest_descendant_grant`.

The `GrantCheck` port (already declared in `core/src/acceptance/ports.rs`) is implemented by the `AuthorityRegistry`. It evaluates the verified `IssuerContext` against the durable grant set — never a self-asserted payload field. Deny-by-default. Revocation is durable and enforced; non-cascade is structural (no cascade mechanism).

This shape honors Ports & Adapters (authority depends on `Storage` and implements `GrantCheck`; acceptance depends on the `GrantCheck` trait + an `IssuerContext` port, not on authority; the ingress implements `IssuerContext` later), Single Source of Truth (the event log is the only source of grant state; the in-memory registry is a pure fold), Generated Contracts (`Grant`/`DescendantGrant`/`Revocation` are generated proto messages; `StoredEventKind` discriminators are schema-owned; `SessionRegistered.spawn_origin` is an additive field), and Fail Fast (invalid grants, unknown grant kinds, unverified identity, and log corruption are rejected at the boundary).

## Cross-feature dependencies (sequenced)

1. **`story-sessions-spawn-origin-field`** (prerequisite, sessions feature) — add `spawn_origin: TypedCorrelation` (optional) to the `SessionRegistered` proto message; carry it through ingestion. Unblocks the authority descendant-grant reactor. **Must land before `story-v0-core-authority-spawn-tail`.**
2. **`story-acceptance-issuer-context`** (prerequisite, acceptance feature) — change `submit` to take an `&dyn IssuerContext` instead of reading `Operation.sender` for the grant check; change `GrantCheck::check` signature to take `&dyn IssuerContext`. `Operation.sender` stays for audit. The `IssuerContext` trait is defined in the authority module (story 2); this story updates the call site. **Must land before `story-v0-core-authority-grant-check` can be integrated/tested end-to-end** (though the GrantCheck impl can be developed against a test double in parallel).

**Dependency-graph fix (rev2 finding F):** the `IssuerContext` trait is defined in authority story 2 (`story-v0-core-authority-grant-check`), which has no deps. The acceptance prerequisite imports it. The explicit edge is: `story-v0-core-authority-grant-check` (defines the trait) → `story-acceptance-issuer-context` (uses it). No "co-developed" ambiguity — the trait lands first (or in the same wave), then the acceptance call-site update. The registry story (story 1) does NOT depend on the trait (it takes actor/endpoint/domain as explicit params in `grant_authorizes`, or a minimal `IssuerRef` struct story 2 refines).

## Implementation Units

### Unit 1: Grant/revocation event model and the `AuthorityRegistry` projection

**File**: `core/src/authority/mod.rs`, `core/src/authority/state.rs`, `core/src/authority/events.rs`, `core/src/authority/registry.rs`

**Story**: `story-v0-core-authority-registry`

The durable event shape (already-defined proto messages — `Grant`, `DescendantGrant`, `Revocation`) and the in-memory projection that folds them. Mirrors `SessionRegistry`/`ElicitationSlotLayer`.

```rust
// core/src/authority/state.rs
use patchbay_contracts::patchbay::{
    ActorId, EndpointId, AuthorityDomainId, GrantId, TargetScope, TargetScopeKind,
    OperationKind, Generation, GrantRevocationPolicy,
};

/// The in-memory grant record, derived from the event log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantRecord {
    pub grant_id: GrantId,
    pub authority_domain_id: AuthorityDomainId,
    pub subject_actor_id: ActorId,
    pub subject_endpoint_id: Option<EndpointId>,
    pub subject_endpoint_class: String,
    pub target_scope: TargetScope,
    pub allowed_operation_kinds: Vec<OperationKind>,
    pub created_at: Option<::prost_types::Timestamp>,
    pub expires_at: Option<::prost_types::Timestamp>,  // stored, not enforced (clock — backlog)
    pub revocation_generation: Option<Generation>,
    pub revoked_at: Option<::prost_types::Timestamp>,
    pub revocation_policy: GrantRevocationPolicy,
    pub is_descendant: bool,
    pub provenance: GrantProvenanceKind,
}

impl GrantRecord {
    pub fn is_revoked(&self) -> bool { self.revocation_generation.is_some() }
    pub fn is_live(&self) -> bool { !self.is_revoked() }
    // NOTE: expiry NOT enforced in v0.1.0 (no clock). Documented gap (backlog).
}

pub const DESCENDANT_GRANT_ALLOWED_KINDS: &[OperationKind] = &[
    OperationKind::Instruct, OperationKind::Cancel, OperationKind::Interrupt,
    OperationKind::Query, OperationKind::ApprovalResponse, OperationKind::ElicitationResponse,
    OperationKind::Reconfigure, OperationKind::SessionManagement,
];

/// The verified-issuer view that grant_authorizes needs. A minimal struct
/// (story 2's IssuerContext trait produces this). Decouples the matching
/// predicate from the trait so story 1 has no forward dep.
pub struct IssuerRef<'a> {
    pub actor: &'a ActorId,
    pub endpoint: Option<&'a EndpointId>,
    pub authority_domain_id: &'a AuthorityDomainId,
}

/// Does `grant` authorize `(issuer, kind, target)`? Deny-by-default.
/// Full matching matrix (rev2 finding #3 + rev3 domain-equality, pinned):
/// 1. grant is live (not revoked)
/// 2. grant.authority_domain_id == issuer.authority_domain_id (domain equality — rev2 finding B)
/// 3. grant.subject_actor_id == issuer.actor (verified actor)
/// 4. endpoint narrowing: if grant has subject_endpoint_id, issuer.endpoint must match
/// 5. operation_kind in grant.allowed_operation_kinds
/// 6. target_scope_matches(grant.target_scope, requested_target_scope)
#[must_use]
pub fn grant_authorizes(
    grant: &GrantRecord,
    issuer: &IssuerRef<'_>,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
) -> bool {
    grant.is_live()
        && grant.authority_domain_id == *issuer.authority_domain_id
        && grant.subject_actor_id == *issuer.actor
        && grant_endpoint_matches(grant, issuer)
        && grant.allowed_operation_kinds.contains(&operation_kind)
        && target_scope_matches(&grant.target_scope, target_scope)
}

/// Scope-containment predicate (rev2 finding #3, pinned — not left to implementer).
/// The grant's target_scope defines what it authorizes; the requested must fall within it.
/// Match on TargetScopeKind:
/// - FleetSupervisor: authorizes any target (fleet = wildcard).
/// - AuthorityDomain: authorizes any target in the same authority domain.
/// - Adapter: authorizes targets on the same adapter_id.
/// - RuntimeSession: authorizes that exact session (adapter+deployment+runtime+generation).
/// - ProjectSessionGroup: authorizes sessions in that project/group (containment by project_or_group).
/// - Actor: authorizes targets with that actor.
/// - Resource: authorizes that exact resource_id.
/// - Unspecified: never matches (Fail Fast — reject at boundary, but defensive here).
#[must_use]
pub fn target_scope_matches(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    use TargetScopeKind as K;
    match K::try_from(grant_scope.kind) {
        Ok(K::FleetSupervisor) | Ok(K::AuthorityDomain) => true,  // wildcard
        Ok(K::Adapter) => same_adapter(grant_scope, requested),
        Ok(K::RuntimeSession) => same_session(grant_scope, requested),
        Ok(K::ProjectSessionGroup) => same_project_group(grant_scope, requested),
        Ok(K::Actor) => same_actor(grant_scope, requested),
        Ok(K::Resource) => same_resource(grant_scope, requested),
        _ => false,  // Unspecified or unknown
    }
}
```

```rust
// core/src/authority/registry.rs
#[derive(Debug, Clone, Default)]
pub struct AuthorityRegistry {
    grants: HashMap<GrantId, GrantRecord>,
}

impl AuthorityRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> { ... }
    pub fn get_grant(&self, grant_id: &GrantId) -> Option<&GrantRecord> { ... }
    pub fn live_grants(&self) -> impl Iterator<Item = &GrantRecord> { ... }
}
```

**Implementation Notes**:
- `grant_authorizes` takes `IssuerRef` (a minimal struct), NOT the `IssuerContext` trait — so story 1 has no forward dependency on story 2's trait. Story 2's `IssuerContext` impl produces an `IssuerRef` for the predicate. (Rev2 finding F fix.)
- `target_scope_matches` is fully specified here (rev2 finding #3) — the semantic 50/50 is pinned. Each `TargetScopeKind` has a defined containment rule.
- `observe` validates grant shape (Fail Fast); `observe_revocation` marks revoked (not delete); idempotent. Descendant grants must have exactly `DESCENDANT_GRANT_ALLOWED_KINDS`.

**Acceptance Criteria**:
- [ ] `observe` folds Grant, DescendantGrant, Revocation events correctly
- [ ] Revocation marks revoked (not deleted); `is_live()` returns false after
- [ ] `grant_authorizes` returns true only when: live + domain matches + verified actor matches + (endpoint narrows if present) + kind allowed + target in scope
- [ ] `target_scope_matches` implements the full scope-containment matrix (fleet=any, authority-domain=any, adapter=same adapter, runtime-session=exact, project-group=containment, actor=same actor, resource=exact)
- [ ] `DESCENDANT_GRANT_ALLOWED_KINDS` matches PROTOCOL.md exactly (8 kinds, spawn+attach excluded)
- [ ] `observe` rejects malformed grants; idempotent for re-delivered events

---

### Unit 2: `IssuerContext` port + `GrantCheck` impl (the acceptance seam)

**File**: `core/src/authority/issuer.rs`, `core/src/authority/check.rs`

**Story**: `story-v0-core-authority-grant-check`

Defines the verified-identity port (R2) and implements `GrantCheck` against the durable grant set. The trait lands here; the acceptance call-site update (`story-acceptance-issuer-context`) depends on it.

```rust
// core/src/authority/issuer.rs
use patchbay_contracts::patchbay::{ActorId, EndpointId, DeviceId, AuthorityDomainId, Generation};

/// Verified issuer identity, supplied by the authenticated ingress boundary.
/// NOT self-asserted: the operator actor and transport endpoint come from
/// verified connection/session evidence (SECURITY.md "compound issuer").
/// v0.1.0 tests supply a TestIssuerContext; the real verifier (operator-session
/// evidence, transport principal) lands with feature-v0-protocol-seam /
/// feature-v0-web-server (both at drafting).
pub trait IssuerContext: Send + Sync {
    fn verified_actor(&self) -> Option<&ActorId>;
    fn verified_endpoint(&self) -> Option<&EndpointId>;
    fn verified_device(&self) -> Option<&DeviceId>;
    fn endpoint_generation(&self) -> Option<Generation>;
    fn authority_domain_id(&self) -> &AuthorityDomainId;
}
```

```rust
// core/src/authority/check.rs
impl GrantCheck for AuthorityRegistry {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,  // R2: verified, not self-asserted
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        let Some(actor) = issuer.verified_actor() else {
            return Err(GrantDenied::NoGrant { /* unauthenticated */ });
        };
        // Domain equality (rev2 finding B): the issuer's verified domain must match
        // the requested authority_domain_id. No payload-domain-override hole.
        if issuer.authority_domain_id() != authority_domain_id {
            return Err(GrantDenied::NoGrant { /* cross-domain */ });
        }
        let issuer_ref = IssuerRef { actor, endpoint: issuer.verified_endpoint(), authority_domain_id };
        for grant in self.live_grants() {
            if grant_authorizes(grant, &issuer_ref, operation_kind, target_scope) {
                return Ok(Authorized { grant_id: Some(grant.grant_id.clone()) });
            }
        }
        Err(GrantDenied::NoGrant { actor: format!("{:?}", actor), kind: operation_kind, target: format!("{:?}", target_scope) })
    }
}
```

**Implementation Notes**:
- The `GrantCheck::check` signature changes from `actor: &ActorEndpointRef` to `issuer: &dyn IssuerContext` (port-shape change). The acceptance prerequisite (`story-acceptance-issuer-context`) updates the call site; it depends on this trait existing.
- Domain-equality pinned (rev2 finding B): `issuer.authority_domain_id() != authority_domain_id` → denied. No payload-domain-override hole.
- Deny-by-default: no verified actor → denied; no matching live grant → denied.
- v0.1.0 tests supply `TestIssuerContext`; real impl with the ingress.

**Acceptance Criteria**:
- [ ] `IssuerContext` trait defined (verified identity port)
- [ ] `check` returns `Authorized` for a verified issuer with a live matching grant
- [ ] `check` returns `GrantDenied` for an unauthenticated issuer (no verified actor)
- [ ] `check` returns `GrantDenied` for a cross-domain issuer (domain mismatch)
- [ ] `check` returns `GrantDenied` for a revoked grant
- [ ] `check` returns `GrantDenied` for a kind/target not covered (deny-by-default)
- [ ] `GrantCheck::check` takes `&dyn IssuerContext` (port-shape change)

---

### Unit 3: Grant + revocation ingestion (the writer)

**File**: `core/src/authority/projection.rs`, `core/src/authority/ingest.rs`

**Story**: `story-v0-core-authority-ingest`

The direct ingestion writer for grants, descendant grants, and revocations — the analog of sessions' `ingest_session_report`. Owns its event kinds end-to-end (writer pattern). No bootstrap grant (R1 dropped it).

```rust
// core/src/authority/projection.rs
/// Read + warm port (mirrors sessions' SessionProjection post-B5 fix).
/// &mut L so the writer can warm after each append (retry-safe).
pub trait GrantProjection: Send + Sync {
    fn current_grant(&self, grant_id: &GrantId) -> impl std::future::Future<Output = Option<GrantRecord>> + Send;
    fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError>;
}

// core/src/authority/ingest.rs
pub async fn ingest_grant<S, L>(storage: &S, projection: &mut L, grant: Grant) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantProjection { ... }

pub async fn ingest_descendant_grant<S, L>(storage: &S, projection: &mut L, grant: DescendantGrant) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantProjection { ... }
// Validates allowed-kinds match DESCENDANT_GRANT_ALLOWED_KINDS exactly (Fail Fast).

pub async fn ingest_revocation<S, L>(storage: &S, projection: &mut L, revocation: Revocation) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantProjection { ... }
// Two-lever non-cascade: revokes ONLY the named grant. No cascade code path.
```

**Implementation Notes**:
- `GrantProjection` takes `&mut L` with `observe(&mut self, ...)` (rev2 finding #7). Warm after each successful append so retry is idempotent.
- `ingest_descendant_grant` validates allowed-kinds (Fail Fast).
- `ingest_revocation` — non-cascade (structural). Revoking non-existent → error.
- No `ensure_bootstrap_operator_grant` (R1 dropped it). Tests inject grants directly.
- Writer pattern mirroring `ingest_session_report`.

**Acceptance Criteria**:
- [ ] `ingest_grant` writes a Grant event; projection reflects it
- [ ] `ingest_descendant_grant` rejects a descendant with the wrong allowed-kind set
- [ ] `ingest_revocation` marks ONLY the named grant revoked (non-cascade, two-lever)
- [ ] `ingest_revocation` does NOT revoke descendant grants under the revoked grant
- [ ] Revoking a non-existent grant returns an error (Fail Fast)
- [ ] Warm-after-write keeps the projection consistent (retry-safe)

---

### Unit 4: Descendant-grant-on-spawn log-tail reactor (order-independent, tested via replay)

**File**: `core/src/authority/spawn_tail.rs`

**Story**: `story-v0-core-authority-spawn-tail` (depends on `story-sessions-spawn-origin-field`)

A pure fold that produces a descendant-grant issuance on observing a completed spawn + correlated `SessionRegistered.spawn_origin`. **Order-independent** (rev2 finding D). Exercised via replay/direct observe in tests — no live consumer loop (rev2 finding E dropped).

```rust
// core/src/authority/spawn_tail.rs
use std::collections::{HashMap, HashSet};
use patchbay_contracts::patchbay::{CommandId, StoredEventKind, TargetScope, ActorId, GrantId, AuthorityDomainId};

/// Reactor: a pure fold producing descendant-grant issuances. Order-independent:
/// retains spawn-op, completion, and registration facts separately; try_issue
/// after any of them. Exercised via replay/direct observe (no live consumer).
#[derive(Debug, Clone, Default)]
pub struct SpawnDescendantTail {
    spawn_ops: HashMap<CommandId, SpawnOpInfo>,       // command_id -> spawn op (from OPERATION)
    completed: HashSet<CommandId>,                     // spawns that reached Completed
    registrations: HashMap<CommandId, RegistrationInfo>, // spawn_origin -> registration (from SessionRegistered)
    issued: HashSet<CommandId>,                        // in-memory idempotency for the fold
}

struct SpawnOpInfo { spawner_actor: ActorId, spawning_grant_id: Option<GrantId> }
struct RegistrationInfo { spawned_session_scope: TargetScope, authority_domain_id: AuthorityDomainId }

impl SpawnDescendantTail {
    pub fn new() -> Self { Self::default() }

    /// Fold one committed event. Order-independent: after any of {spawn op seen,
    /// completion seen, registration seen}, call try_issue(key).
    ///
    /// Domain isolation (rev3-review finding 1): all collections are keyed by
    /// (AuthorityDomainId, CommandId), NOT bare CommandId — events are
    /// authority-domain scoped and client-generated command IDs are not
    /// globally unique. A single tail instance serves one domain; events
    /// from another domain are rejected as CorruptLog.
    ///
    /// Duplicate handling (rev3-review finding 1): exact redelivery (same
    /// event at the same LSN) is a no-op; a conflicting duplicate (same key,
    /// different content) is CorruptLog (Fail Fast — mirrors SessionRegistry).
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
        // Track Spawn OPERATION events -> spawn_ops (key = (domain, command_id)).
        // Track COMMAND_TRANSITION to Completed for spawn commands -> completed.
        // Track SessionRegistered with spawn_origin -> registrations (key = (domain, spawn_origin)).
        // After any insertion, try_issue for that (domain, command_id):
        //   if spawn_ops.has(k) && completed.has(k) && registrations.has(k) && !issued.has(k):
        //     issued.insert(k); return Some(issuance with deterministic grant_id).
        // The issuance carries spawned_session_scope from the registration (NOT the spawn op's fleet target).
    }
}

pub struct DescendantGrantIssuance {
    pub spawn_operation_id: CommandId,
    pub spawning_grant_id: Option<GrantId>,
    pub spawned_session_scope: TargetScope,
    pub subject_actor_id: ActorId,
    pub authority_domain_id: AuthorityDomainId,
    pub allowed_operation_kinds: Vec<OperationKind>,  // DESCENDANT_GRANT_ALLOWED_KINDS
    /// Deterministic grant id derived from (authority_domain_id, spawn_operation_id)
    /// (rev3-review finding 1): computed inside a canonical helper, NOT delegated
    /// to the caller. Durable idempotency: re-observe -> same id -> no-op dup.
    pub descendant_grant_id: GrantId,
    /// audit_id: NOT populated in v0.1.0 (rev3-review finding 2). The protocol
    /// requires a spawn-completion audit link (DescendantGrant.audit_id field 14),
    /// but the audit producer is deferred (R4). The issuance carries None here;
    /// the descendant grant created from this issuance has audit_id = None until
    /// the audit producer lands. Documented gap (backlog). The descendant grant
    /// is component-tested, not protocol-complete.
    pub audit_id: Option<EventId>,
}

/// Canonical deterministic descendant grant id (rev3-review finding 1).
/// Namespaced to avoid collision with operator grants.
fn descendant_grant_id(domain: &AuthorityDomainId, spawn_op: &CommandId) -> GrantId {
    GrantId { value: format!("desc:{}:{}", domain.value, spawn_op.value) }
}
```

**Implementation Notes**:
- **Order-independent** (rev2 finding D): three separate maps (`spawn_ops`, `completed`, `registrations`); `try_issue` runs after any insertion. If `SessionRegistered` arrives before `Completed`, the registration is retained and the issuance fires when `Completed` arrives. Test all 6 permutations.
- `issued` is in-memory idempotency for the fold. Durable idempotency for tests = deterministic grant_id derived from `(authority_domain_id, spawn_command_id)`; the test harness calls `ingest_descendant_grant` with that ID, so a re-observe → same ID → no-op duplicate.
- `spawning_grant_id`: from the spawn op's authorization. **Provenance note (rev2 finding C, softened):** in v0.1.0 tests, the spawn op's `Authorized.grant_id` is available in-memory (the test drives both acceptance and authority). For replay durability, the provenance is reconstructed from the durable grant set (the test injects the spawning grant). Full durable acceptance-metadata (verified actor/endpoint/authorizing-grant on the command record) is follow-on when the live path exists — filed as backlog. `spawning_grant_id` may be `None` in v0.1.0 if not injectable; documented, not silently wrong.
- No live consumer loop, no composition root (rev2 finding E dropped). The reactor is a pure fold; tests feed events directly.

**Acceptance Criteria**:
- [ ] A Spawn OPERATION + Completed transition + SessionRegistered(spawn_origin=that command) produces exactly one `DescendantGrantIssuance`, regardless of arrival order (test all 6 permutations)
- [ ] A spawn reaching a non-Completed terminal produces NO issuance
- [ ] A SessionRegistered without `spawn_origin` does NOT trigger an issuance
- [ ] Replay (re-observing events) does not produce duplicate issuances (idempotent via `issued` + deterministic grant_id)
- [ ] The issuance's allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly
- [ ] The issuance's `spawned_session_scope` comes from the `SessionRegistered` event (not the spawn Operation's fleet target)

---

### Unit 5: Replay and module wiring

**File**: `core/src/authority/replay.rs`, `core/src/authority/mod.rs`, `core/src/lib.rs`

**Story**: `story-v0-core-authority-replay`

Rebuild the registry from the log + wire the module. No composition layer (rev2 finding E dropped — no live consumer).

```rust
// core/src/authority/replay.rs
pub async fn rebuild_from_log<S: Storage>(
    storage: &S, authority_domain_id: &AuthorityDomainId,
) -> Result<AuthorityRegistry, AuthorityError> {
    // Near-exact copy of session::rebuild_from_log: read_after(Lsn{0}), fold via observe,
    // validate LSN monotonicity + domain match.
}
```

**Implementation Notes**:
- `rebuild_from_log` mirrors `session::rebuild_from_log` / `elicitation::rebuild_slots_from_log`.
- No `AuthorityComposition` (rev2 finding E dropped). The registry is rebuilt via `rebuild_from_log`; the spawn-tail is a separate fold exercised in tests. A live composition layer is follow-on.
- Module wiring: `core/src/authority/` alongside `acceptance/`, `session/`, `storage/`; `lib.rs` exports it.

**Acceptance Criteria**:
- [ ] `rebuild_from_log` reconstructs the registry identically to a live registry
- [ ] `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`
- [ ] `core/src/authority/` module compiles and is exported from `core/src/lib.rs`

---

### Unit 6: Property tests for authority invariants (8 properties)

**File**: `core/tests/authority_proptest.rs`

**Story**: `story-v0-core-authority-proptests`

Property tests for the 8 stated-normative obligations. 7 are executable oracles; 1 (`ElicitationResponderAuthority`) is a documented untested gap (rev3 R6 — not a vacuous test).

```rust
proptest! {
    /// 1. NoCommandWithoutGrant: deny-by-default.
    #[test] fn no_command_without_grant(/* ... */) { ... }
    /// 2. CompoundIssuer: accepted commands use verified IssuerContext identity,
    ///    not self-asserted payload actor. (rev3-review finding 4: this is an
    ///    ACCEPTANCE-AUTHORITY integration property — GrantCheck no longer
    ///    receives Operation.sender, so the mutation must be acceptance
    ///    constructing issuer identity from Operation.sender. The proptest
    ///    story's depends_on includes story-acceptance-issuer-context.)
    #[test] fn compound_issuer(/* ... */) { ... }
    /// 3. GrantAuthorityIsCommandKinds: grant checks constrain by canonical OperationKinds.
    #[test] fn grant_authority_is_command_kinds(/* ... */) { ... }
    /// 4. RevocationPreventsFuture: revoked grant denies subsequent checks.
    #[test] fn revocation_prevents_future(/* ... */) { ... }
    /// 5. FleetAuthorityForSpawn: a fleet-scope spawn grant authorizes spawn
    ///    across any adapter; an adapter-scope grant authorizes spawn on that
    ///    adapter only; a runtime-session grant cannot authorize creating a
    ///    not-yet-existing session. (PROTOCOL line 173: adapter-level spawn
    ///    grants are expressible — fleet is the default, not the only option.
    ///    rev3-review finding 3: the prior oracle contradicted the protocol.)
    #[test] fn fleet_authority_for_spawn(/* ... */) { ... }
    /// 6. SpawnCreatesDescendantGrant: successful spawn produces a descendant grant.
    #[test] fn spawn_creates_descendant_grant(/* ... */) { ... }
    /// 7. SpawnRevocationDoesNotCascade: two levers. Mutation-survivable stand-in for the demoted formal property.
    #[test] fn spawn_revocation_does_not_cascade(/* ... */) { ... }
}
// 8. ElicitationResponderAuthority: NOT TESTED HERE. Authority does not enforce
//    response-Operation responder matching (Elicitation.expected_responder_actor);
//    that's an acceptance/elicitation concern. Documented untested gap (rev3 R6).
//    The obligation is real; owned by a future acceptance responder-validation feature.

// Mutation tests: cascade-revocation fails #7; payload-actor-trust fails #2.
```

**Implementation Notes**:
- 7 executable oracles + 1 documented gap (rev3 R6). All stated-normative (draft) — not formally checked.
- `spawn_revocation_does_not_cascade` (#7): both levers (revoke parent P → P denies + descendant D still authorizes; separately revoke D → D denies). Mutation-survivable.
- `compound_issuer` (#2): a self-asserted payload actor is NOT trusted — `IssuerContext` is authority.
- Mutation tests essential (non-vacuity).

**Acceptance Criteria**:
- [ ] 7 properties pass against the real implementation
- [ ] #7 fails against a cascade mutation (non-vacuous)
- [ ] #2 fails against a payload-actor-trust mutation (non-vacuous)
- [ ] `replay_matches_live` passes (supplementary)
- [ ] #8 (ElicitationResponderAuthority) documented as an untested gap, NOT a vacuous test

---

## Implementation Order

0. **`story-sessions-spawn-origin-field`** (prerequisite, sessions) — add `SessionRegistered.spawn_origin`. **Before story 4.**
0b. **`story-acceptance-issuer-context`** (prerequisite, acceptance) — `submit` takes `&dyn IssuerContext`; `GrantCheck::check` signature change. Depends on the `IssuerContext` trait (story 2). **Before story 2 integrates end-to-end.**
1. `story-v0-core-authority-registry` — grant/revocation event model + `AuthorityRegistry` + `grant_authorizes` + `target_scope_matches` (no deps; takes `IssuerRef` not the trait)
2. `story-v0-core-authority-grant-check` — `IssuerContext` trait + `impl GrantCheck` (depends on 1; defines the trait that 0b uses)
3. `story-v0-core-authority-ingest` — grant/revocation writer (depends on 1)
4. `story-v0-core-authority-spawn-tail` — order-independent reactor (depends on 1, 3, AND prerequisite 0)
5. `story-v0-core-authority-replay` — `rebuild_from_log` + wiring (depends on 1, 2, 3)
6. `story-v0-core-authority-proptests` — 7 property oracles + mutation tests + 1 documented gap (depends on 1-5)

Stories 1 is the foundation. 2 and 3 parallel after 1. 4 depends on 1, 3, and prerequisite 0. 5 depends on 1-3. 6 depends on all. Prerequisites 0 and 0b parallel with story 1; 0b depends on story 2's trait.

## Testing

### Unit Tests: `core/tests/authority_*.rs`
- `authority_registry.rs` — fold, revocation-marks-not-deletes, idempotent observe, malformed-grant rejection, `target_scope_matches` matrix (all 7 kinds)
- `authority_grant_check.rs` — verified-issuer grant match, unauthenticated denied, cross-domain denied, revoked denied, kind/target mismatch denied, payload-actor-NOT-trusted
- `authority_ingest.rs` — grant/descendant/revocation ingestion, descendant allowed-kind validation, non-cascade revocation
- `authority_spawn_tail.rs` — completed-spawn-produces-issuance (all 6 arrival orders), non-Completed no issuance, no-spawn_origin no issuance, idempotent replay
- `authority_replay.rs` — replay determinism, LSN monotonicity, cross-domain rejection
- `authority_proptest.rs` — 7 property oracles + mutation tests + #8 documented gap

### Integration Points
- **Acceptance ↔ Authority**: acceptance calls `GrantCheck::check` with `&dyn IssuerContext`. Cross-feature dependency #2.
- **Authority ↔ Sessions**: spawn-tail consumes `SessionRegistered` with `spawn_origin`. Cross-feature dependency #1.
- **Authority ↔ Storage**: writes via `Storage::append`; reads via `Storage::read_after`.
- **Authority ↔ Elicitation**: no direct coupling. Property #8 (responder authority) is a documented gap owned by a future acceptance feature.

## Risks

- **Weakest formal backing.** All 8 `authority.qnt` properties stated-normative; four demoted. Property tests are executable oracles (7) + 1 documented gap, NOT formally checked. v1 formal gate owns the real properties. Documented; not over-claimed.
- **Component-complete, not live.** v0.1.0 authority is tested via injected grants + replay, not a live operator-issuing path. The live path (ingress + fleet-target-resolution + live composition/wiring) is follow-on. This is honest about v0.1.0 scope (SPEC verification floor).
- **Two cross-feature prerequisites.** `spawn_origin` (sessions) + `IssuerContext` call-site (acceptance). Both small, additive, owned by their features. depends_on chain explicit.
- **Expiry enforcement deferred.** `expires_at` stored, not enforced (clock — backlog).
- **Distinct failed-authorization audit deferred (R4).** Grant-lifecycle provenance delivered; distinct denied-attempt audit is separate (backlog).
- **Fleet target resolution gap (R5).** Spawn end-to-end blocked until backlog item lands. Authority testable independently.
- **ElicitationResponderAuthority untested (R6).** Documented gap, not vacuous. Owned by a future acceptance responder-validation feature.
- **Provenance durability partial.** `spawning_grant_id` may be `None` in v0.1.0 tests if not injectable; full durable acceptance-metadata is follow-on (backlog).
- **Snapshot checkpointing deferred.** Replay from LSN 0, matching the other projections.

## Extension pressure classification

- **Committed v0.1.0 behavior**: deny-by-default grant evaluation against durable grants; the `GrantCheck` port with verified `IssuerContext` (not self-asserted) + domain equality; durable Grant/DescendantGrant/Revocation events with provenance; revocation marks-not-deletes (audit retention); the descendant-grant allowed-kind set (8 kinds, spawn+attach excluded); two-lever non-cascade revocation (structural); descendant-grant-on-spawn via order-independent log-tail correlating `spawn_origin`; the `AuthorityDomainId` key shape (federation seam).
- **Reserved seam**: multi-operator authority domains + operator provisioning/bootstrap (the live path — follow-on); delegation lineage (`parent_grant_id` — absent); per-spawn-variant authority; tighter endpoint-class narrowing; expiration enforcement (clock — backlog); distinct failed-authorization audit records (backlog); fleet target resolution for spawn (backlog); live composition/wiring layer (follow-on); durable acceptance-metadata for provenance (follow-on); ElicitationResponderAuthority enforcement (follow-on acceptance feature).
- **Explicitly rejected**: trusting self-asserted actor identity (compound-issuer — `IssuerContext` is authority, never `Operation.sender`); adapter capability declarations as grant authority; making acceptance write descendant grants (Ports & Adapters violation); cascading revocation as v0.1.0 behavior (two-lever is the rule); a vacuous ElicitationResponderAuthority test (documented gap, not fake test).

## Prior review history

- **Revision 1** (Q1-Q5): implicit operator authority + log-tail reactor + full protocol model. Design review #1 found 10 blockers. Bounced.
- **Revision 2** (R1a-R5a): vertical live slice — durable bootstrap operator grants + verified IssuerContext + descendant-grant reactor + composition layer + 2 prerequisites. Design review #2 found 4 blockers partially-resolved + 8 new defects (7 blocking). Bounced.
- **Revision 3** (this): component-complete, not live. Dropped bootstrap grant + live composition (R1, R3, E). Pinned domain-equality (B), order-independent reactor (D), full matching matrix (#3), ElicitationResponder narrowed to documented gap (R6/G). Addresses all rev-2 findings: A (bootstrap) dropped, B (domain-eq) pinned, C (provenance) softened + documented, D (ordering) fixed, E (composition) dropped, F (graph) fixed via IssuerRef decoupling, G (responder) narrowed, H (live claim) dropped.

## Design review #3 (revision 3, 2026-07-13)

**Verdict**: Approve with in-stride fixes — re-advanced to `stage: implementing`. NOT bounced: all 5 findings are mechanical (protocol/pattern-pinned), not semantic 50/50s, per the implementation-ambiguity rule (no reasonable implementer would pick a materially different option).

**Reviewer**: cross-model fresh-context (openai-codex/gpt-5.6-sol). Confirmed 14 of 18 prior findings RESOLVED; 4 carried as documented gaps/backlog (compound-issuer endpoint-class, provenance durability, audit_id, responder validation — all already deferred per R2/R4/R6).

### New findings (5) — all resolved in-stride

1. **Spawn-tail not domain-isolated / no deterministic grant_id (blocker → resolved).** Collections were keyed by bare `CommandId`; client command IDs aren't globally unique across domains. **Fix (pinned by `(authority_domain_id, LSN)` key shape, PROTOCOL):** key all maps by `(AuthorityDomainId, CommandId)`; conflicting duplicate = `CorruptLog` (mirrors `SessionRegistry`); deterministic `descendant_grant_id(domain, spawn_op)` computed in a canonical helper, included in the issuance.
2. **Descendant issuance can't satisfy audit_id (blocker → narrowed).** PROTOCOL line 186/495 requires a spawn-completion audit link (`DescendantGrant.audit_id` field 14); no producer exists (R4 defers audit). **Fix (pinned by R4):** `DescendantGrantIssuance.audit_id = None` in v0.1.0; descendant grant is **component-tested, not protocol-complete**. Filed `backlog-authority-durable-acceptance-metadata`.
3. **FleetAuthorityForSpawn oracle contradicted protocol (blocker → resolved).** Oracle said "spawn requires fleet grant"; PROTOCOL line 173 says adapter-level spawn grants are expressible (fleet is default, not only). **Fix (pinned by PROTOCOL):** oracle tests fleet=any-adapter, adapter=same-adapter, runtime-session=cannot-authorize-new-session.
4. **compound_issuer test missing acceptance dep (blocker → resolved).** GrantCheck no longer receives `Operation.sender`, so the mutation must be acceptance constructing issuer from payload sender. **Fix (pinned by the test's own logic):** `story-v0-core-authority-proptests` depends_on now includes `story-acceptance-issuer-context`.
5. **Untracked follow-ons (important → resolved).** Live composition, durable acceptance metadata, responder validation described as "follow-on" but no backlog items. **Fix (bookkeeping):** filed `backlog-authority-live-composition`, `backlog-authority-durable-acceptance-metadata`, `backlog-elicitation-responder-authority`.

### Why these are mechanical, not a bounce
Each finding has exactly one defensible answer, pinned by the protocol, the established sessions pattern, or an existing R-decision (R2/R4/R6). The "would a different reasonable implementer pick a materially different option?" test fails for all five — there is no semantic judgment to surface to the operator. Per the project's implementation-ambiguity rule, these resolve in-stride with rationale logged, not a drafting bounce.

## Implementation discovery (rev3-review, 2026-07-13)
The following were discovered during design review #3 and resolved in-stride (mechanical, protocol/pattern-pinned):
- Spawn-tail domain isolation: `(AuthorityDomainId, CommandId)` keying (PROTOCOL `(domain, LSN)` key shape).
- Conflicting-duplicate handling: `CorruptLog` (mirrors `SessionRegistry::observe`).
- Deterministic descendant grant_id: canonical helper, included in issuance (durable idempotency).
- `audit_id`/`spawning_grant_id` optionality: documented gaps, filed as backlog (R4 + provenance-durability follow-on).
- `FleetAuthorityForSpawn` oracle: corrected to match PROTOCOL line 173 (adapter-level spawn grants expressible).
- `compound_issuer` test dependency: added `story-acceptance-issuer-context` edge (the mutation is acceptance-side).

No semantic 50/50s surfaced. The design is implementer-ready.

## Implementation summary (implement-orchestrator, 2026-07-14)

All 6 child stories + 2 cross-feature prerequisites implemented across 5 waves and advanced to `stage: review`. The authority layer is component-complete and tested; it is NOT live-wired (per rev3 design — the live operator-issuing path is follow-on).

### Stories advanced to review
- `story-sessions-spawn-origin-field` (sessions prereq) — additive `SessionRegistered.spawn_origin` proto field 9; Rust+TS regen.
- `story-acceptance-issuer-context` (acceptance prereq) — atomic `GrantCheck::check`/`submit` signature change to `&dyn IssuerContext`; `CommandRecord.grant_id` seam added (unpopulated in v0.1.0, documented gap per R3 provenance-durability deferral).
- `story-v0-core-authority-registry` — `GrantRecord`, `grant_authorizes` (full matching matrix), `target_scope_matches` (7-kind containment), `DESCENDANT_GRANT_ALLOWED_KINDS`, `AuthorityRegistry` projection (marks-not-deletes, idempotent, Fail Fast).
- `story-v0-core-authority-grant-check` — `IssuerContext` trait (verified identity port) + `impl GrantCheck for AuthorityRegistry` (deny-by-default, domain-equality pinned).
- `story-v0-core-authority-ingest` — grant/descendant/revocation writer (warm-after-write, retry-safe); non-cascade revocation structural.
- `story-v0-core-authority-spawn-tail` — order-independent reactor (all 6 arrival permutations tested), domain-isolated `(domain, command_id)` keying, deterministic `descendant_grant_id`, conflicting-duplicate rejection.
- `story-v0-core-authority-replay` — `rebuild_from_log` mirroring session/elicitation replay; cross-domain rejection tested.
- `story-v0-core-authority-proptests` — 7 property oracles (100 cases each), 2 non-vacuous mutation tests (cascade + payload-actor-trust), `replay_matches_live`, #8 ElicitationResponderAuthority documented as untested gap (not vacuous).

### Cross-cutting deviations
- `CommandRecord.grant_id` field added but not durably populated in v0.1.0 — wiring would require storing it on the `Operation` proto (out of scope). Matches feature R3 provenance-durability deferral; `spawning_grant_id` may be `None` in v0.1.0 tests. Backlog: `backlog-authority-durable-acceptance-metadata`.
- Contract regen (Wave 1) brought TS gen into sync for `CommandTransition`/`COMMAND_TRANSITION=8` (pre-existing TS-gen staleness from the acceptance feature) as a side effect — benign, additions-only.

### Verification status
Build clean; full `patchbay-core` suite green (171 tests, 31 new authority tests across registry/grant-check/ingest/spawn-tail/replay/proptests); `cargo clippy --all-targets -- -D warnings` clean. Existing acceptance + sessions tests updated to the new `submit`/`GrantCheck` signature and remain green.

### Orchestrator notes
- Wave shape was mostly serial (inherent: the authority module is one coherent safety-critical unit with an atomic cross-module signature change and a shared `mod.rs`). One multi-item bundle (B1: S2+P0b+S3) collapsed the atomic signature change + writer into one green-build stride; one multi-item bundle (B2: S4+S5) kept `mod.rs` coherent across two log-consumer folds.
- One subagent misread the intra-run dep-readiness rule (refused to start because S1 was at `review` not `done`); corrected by re-dispatch with explicit guidance that intra-run `review`-stage deps are buildable by the wave-plan design.
- All work ran on `openai-codex/gpt-5.6-sol` (per routing rule: never `umans/*` subagents). Highest tier (xhigh) for the atomic signature-change bundle; high for the rest.

## Deep review (feature-level, 2026-07-14)

**Verdict**: Request changes — bounce to `stage: implementing`. 2 blockers (genuine, unanticipated by design) + 1 verification-coverage gap + 4 backlog items. The architecture and pinned invariants are sound; the blockers are localized code defects contradicting pinned design decisions.

**Depth**: Deep lane, two-phase (completeness → adversarial), cross-model fresh-context. Both phases ran `openai-codex/gpt-5.6-sol` (xhigh) — a different model class from the umans orchestrator, satisfying the cross-model advisory-review requirement. Both reviewers converged independently on the same top findings, raising confidence.

### Blockers (filed as fix stories)

1. **RuntimeSession scope match omits `deployment_scope`** (`core/src/authority/state.rs`, `same_session`) — `-> story-fix-authority-runtime-session-deployment-scope`. The exact-tuple match compares adapter+runtime+generation but NOT deployment_scope, contradicting the pinned design (Unit 1: "adapter+deployment+runtime+generation") and the committed v0.1.0 full-matching-matrix claim. The existing matrix test blesses the omission (both scopes use empty deployment_scope). A grant for `(pi, machine-a, session-1, gen-7)` would authorize `(pi, machine-b, session-1, gen-7)`. Both reviewers flagged this. This is a code bug contradicting a pinned decision, not a deferral.

2. **Conflicting same-generation revocations silently accepted** (`core/src/authority/registry.rs:133-145`) — `-> story-fix-authority-conflicting-revocation-detection`. `observe_revocation` treats a second revocation with the same `revocation_generation` as an exact redelivery (`Ok(())`) WITHOUT comparing policy/timestamp/actor/reason. This contradicts the rev3-review finding 1 guarantee ("conflicting duplicate = CorruptLog"). The spawn-tail's `insert_consistent` helper compares content correctly; the registry's revocation path does not — an internal inconsistency. The accepted-operation policy (`Continue`/`Cancel`/`RequireReauthorization`) is security-relevant.

### Important (filed as fix story / backlog)

3. **CompoundIssuer proptest doesn't drive `acceptance::submit`** (`core/tests/authority_proptest.rs`) — `-> story-fix-authority-compound-issuer-integration-test`. The #2 oracle calls `GrantCheck::check` directly, not `submit`. It proves `AuthorityRegistry` rejects a mismatched verified actor, but does NOT prove the `submit` call site passes a verified issuer (not the payload sender). rev3-review finding 4 intended this as an acceptance-authority integration property. The `submit` call site IS correct (verified in re-review), but no test *proves* it stays correct — a regression where `submit` derives the issuer from `Operation.sender` would not be caught. `[verification]`-tagged coverage gap.

4. **Descendant-grant issuance trusts `Operation.sender` for the subject actor** (`core/src/authority/spawn_tail.rs`) — `-> backlog-authority-payload-actor-in-descendant-issuance`. The spawn itself is authorized against the verified `IssuerContext`, but the descendant grant's *subject* is derived from the self-asserted payload. This is the same compound-issuer concern R2 resolved for `GrantCheck`, not yet extended to the spawn-tail (no durable acceptance metadata exists). Not blocking v0.1.0 (no live path; tests inject), but MUST resolve before the live spawn path. Couples with `backlog-authority-durable-acceptance-metadata`.

5. **Overlapping matching grants produce nondeterministic `grant_id`** (`core/src/authority/check.rs`) — `-> backlog-authority-grant-selection-determinism`. `live_grants()` iterates a `HashMap`; two matching grants return an arbitrary `grant_id` (affects provenance + revocation policy). Rev3 does not pin a selection rule. Latent in single-operator v0.1.0.

6. **Ingest checks for conflicts AFTER appending, not before** (`core/src/authority/ingest.rs`) — `-> backlog-authority-ingest-pre-append-conflict-check`. A conflicting grant is appended to the durable log FIRST, then `observe` rejects it — poisoning the log. Identical retries append a second event. Contradicts the "retry-safe" claim. The `authority_ingest.rs` warm-after-write test re-observes an event to the projection, not retries the writer — false confidence. Latent (single-writer, test-injected); blocking for the live path.

7. **Replay accepts gapped LSNs + silently ignores Unspecified-kind events** (`core/src/authority/replay.rs`, `registry.rs`) — `-> backlog-authority-replay-gap-detection`. `>` not `== prev+1`; Unspecified kind is a no-op, not `CorruptLog`. Defense-in-depth against a misbehaving storage adapter; latent (rusqlite is gap-free). Cross-cutting with sessions/acceptance replay.

### Findings evaluated and ACCEPTED as documented deferrals (not blockers)

These were flagged by reviewers but are EXPLICITLY deferred by the rev3 design + backed by backlog items — accepting the design's scoping, not re-litigating it:
- **Expiry not enforced** — R3/design line 553 + `backlog-grant-expiration-enforcement`. Stored, not enforced (no clock). Documented gap, not a defect.
- **`audit_id` = None in descendant issuance** — R4 + `backlog-authority-durable-acceptance-metadata`. Component-tested, not protocol-complete. Documented.
- **`spawning_grant_id` may be None** — R3 provenance-durability deferral + `backlog-authority-durable-acceptance-metadata`. Documented.
- **Endpoint-class narrowing not enforced** — reserved seam (design line 563: "tighter endpoint-class narrowing"). The grant stores `subject_endpoint_class` but doesn't match on it; endpoint narrowing by `subject_endpoint_id` IS enforced. Reserved, not committed.
- **Distinct failed-authorization audit deferred** — R4 + `backlog-authority-failed-authorization-audit`. Documented.
- **Fleet target resolution** — R5 + `backlog-fleet-target-resolution`. Out of scope.
- **Retry-after-revocation returns authorization_denied not existing command** — this is an acceptance-pipeline concern (grant check runs before dedup), latent (no live retries), and arguably correct per PROTOCOL (a revoked grant denies future authority; the existing command's state is a separate concern owned by the dedup path). Not an authority-feature blocker; noted for acceptance.
- **`desc:{domain}:{command}` grant-id namespace is a string convention** — nit-level; collision requires an operator grant literally named with the `desc:` prefix. Worth hardening but not blocking. Noted in the grant-selection-determinism backlog direction.

### Assessment

The pinned invariants are genuinely sound and well-tested: deny-by-default, domain-equality (enforced on both `grant_authorizes` and `check.rs`), non-cascade revocation (structural — no cascade code path, tested with a real two-lever test), order-independence (6 distinct permutations tested), the 8-kind SSOT (used by validation + issuance + oracle), the #8 honest gap (no vacuous test), and the two mutation tests are genuinely two-sided. The architecture honors Ports & Adapters (acceptance depends on the `IssuerContext` trait, not `AuthorityRegistry`; authority depends on `Storage`).

The blockers are localized: `same_session` missing one field, and `observe_revocation` missing a content comparison. Both contradict pinned design decisions and are code bugs, not design gaps. The verification-coverage gap (#3) is the class of issue this program exists to catch. The backlog items (#4-7) are latent risks honestly scoped to the live-path follow-on.

**Verification at review time**: 171 tests green, clippy clean. The blockers are not caught by the existing tests because the tests bless the bugs (the matrix test uses empty deployment_scope; no conflicting-revocation-content test exists). This is exactly why the deep lane runs fresh-context adversarial reviewers rather than trusting green tests.

**Next**: the 2 blocker fix stories + 1 verification-coverage story must land before this feature advances to `done`. The 4 backlog items track the live-path follow-on. Feature bounces to `stage: implementing`.
