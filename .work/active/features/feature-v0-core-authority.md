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
- `contracts/proto/patchbay/common.proto` — `ActorId`, `EndpointId`, `AuthorityDomainId`, `GrantId`, `TargetScope`, `ActorEndpointRef`, `TypedCorrelation`, `StoredEventKind` (`GRANT=4`, `DESCENDANT_GRANT=5`, `REVOCATION=6`)
- `specs/seed/authority.qnt` — stated-normative authority obligations (8 properties, all draft)

## Design decisions (feature-design, revision 2, 2026-07-13)

Resolved interactively with the operator after a pre-implementation design review (cross-model openai-codex/gpt-5.6-sol) found 10 blockers in revision 1. This revision supersedes revision 1's Q1-Q5. The original decisions NOT changed by the review (durable event-sourcing, log-tail reactor shape, full-protocol allowed-kinds, deny-by-default) are retained; the operator-identity + spawn-machinery cluster is revised.

The review's central finding: revision 1's implicit operator authority (`is_operator(actor) == "operator"` against a payload field) was self-defeating — it bypassed the durable descendant-grant/revocation machinery the feature exists to exercise, AND it trusted a self-asserted payload identity (violating SECURITY.md's compound-issuer rule). This revision goes vertical: durable operator grants + a real descendant-grant reactor, so the machinery is exercised end-to-end on live paths.

- **R1 — Operator authority model: durable bootstrap/operator grants (was: implicit).** Chosen over implicit operator authority (revision-1 Q1) — the review proved implicit authority nullifies the descendant-grant + revocation machinery (the operator's descendant grants are never consulted; revoking them can't deny future operations; the two-lever non-cascade is inert on the production path). A bootstrap operator grant (incl. a fleet-scope spawn grant) is created at init and durably recorded; `GrantCheck` evaluates against it. This satisfies `FleetAuthorityForSpawn` (requires a live fleet spawn grant), the `spawning_grant_id` provenance requirement, and makes revocation real. Chosen over (b) verified-implicit (ships ceremonial always-match operator grants with no payoff) and (c) defer-to-ingress (ships too little). The cost is real but bounded: a bootstrap grant is a single durable record created at first-start.
- **R2 — Verified `IssuerContext` port (was: self-asserted payload sender).** Chosen over trusting the payload `Operation.sender` (revision-1) — the review proved this violates the compound-issuer rule (SECURITY.md: "sender identity comes from the verified connection/session context, not from self-asserted payload fields"). Define an `IssuerContext` port carrying verified operator actor + verified transport endpoint + operator-session evidence, supplied by the authenticated ingress. v0.1.0 tests supply a test double; the real impl lands with `feature-v0-protocol-seam`/`feature-v0-web-server` (both at `drafting`). This is the Ports & Adapters move acceptance already made (`GrantCheck`/`TargetResolver` are ports implemented later). **Acceptance integration:** the `submit` pipeline signature changes to take an `&IssuerContext` (or the pipeline resolves one from an injected port) instead of reading `Operation.sender` for the grant check. The `Operation.sender` field remains for audit/recording but is NOT authority. This is a small acceptance change, filed as a dependency story.
- **R3 — Descendant-grant reactor: vertical slice (was: defer).** Chosen over deferring the reactor (revision-1 Q3 option b) — the review proved deferral ships ceremonial durable grants with no live trigger. The vertical slice exercises the full model end-to-end: operator spawn authority (fleet grant) → descendant grant on spawn completion → revocation (both levers) → non-cascade. **The spawn-result contract gap** (the reactor can't identify the spawned session from a `Completed` transition alone) is closed by one additive proto field: `SessionRegistered.spawn_origin: TypedCorrelation` (optional, references the spawn `CommandId`). This is a sessions-feature change, **sequenced first** as a prerequisite story — sessions owns its proto shape; authority's reactor story depends on it. The reactor then tails for `Spawn → Completed` AND a `SessionRegistered` carrying `spawn_origin` correlating to that spawn command, and issues the descendant grant. Chosen over (a) core-assigned spawn result in the Operation payload (bakes a protocol decision into the wrong layer) and (c) defer (ships too little).
- **R4 — Audit: minimal (grant/revocation events ARE the grant-lifecycle audit; distinct failed-authorization audit deferred).** Chosen over adding a full audit unit (expands scope into a cross-cutting concern that touches acceptance's rejection path too) and over a separate audit-feature dependency (premature). The durable `Grant`/`DescendantGrant`/`Revocation` events with `GrantProvenance`/`DescendantGrantProvenance` satisfy the grant-lifecycle audit need. The distinct failed-authorization audit record (SECURITY.md "audit records are distinct from durable command/session state") is a real requirement but a separate concern — deferred, filed as a backlog item. The feature does NOT claim to deliver full audit; it delivers grant-lifecycle provenance.
- **R5 — Fleet target resolution: out of scope (filed as backlog; acceptance/sessions concern).** The review correctly flagged that the existing `SessionRegistry`-backed `TargetResolver` rejects fleet spawn targets (no session exists yet). This is an acceptance/sessions gap (OperationKind-aware target resolution), not authority's. Authority's design flags it as a cross-cutting dependency and files a backlog item; it does not absorb the scope. (Note: until fleet-target resolution lands, spawn Operations would fail target resolution after passing the grant check. This is a known integration gap; the authority feature's GrantCheck impl + grant model are still valuable and testable independently. The spawn end-to-end path requires the fleet-resolution backlog item to land.)

### Retained from revision 1 (unchanged by the review)

- **Durable event-sourced storage** (rev1 Q2): Grant/DescendantGrant/Revocation events under the existing `StoredEventKind` discriminators; `AuthorityRegistry` projection folds them, mirroring `SessionRegistry`/`ElicitationSlotLayer`. Replay from LSN 0 (snapshot discriminator gap deferred, matches the other projections).
- **Full protocol model** (rev1 Q5): fleet spawn grants, descendant grants with the explicitly-enumerated allowed-kind set (8 kinds, spawn+attach excluded), two-lever non-cascade revocation, provenance. Deny-by-default.
- **Full feature with child stories** (rev1 Q4): the vertical slice is implemented as child stories with declared depends_on, including the sequenced sessions prerequisite.

## Architectural choice

A durable, event-sourced authority layer that exercises the full grant model end-to-end on live paths. The event log (owned by `feature-v0-core-persistence`) is the single source of truth for grant/revocation state. Authority writes `Grant`/`DescendantGrant`/`Revocation` events through the `Storage::append` port. An in-memory `AuthorityRegistry` is the hot lookup path, rebuilt from replay on startup (replay from LSN 0, matching the other projections). A bootstrap operator grant (incl. fleet spawn grant) is created at init.

The authority feature owns its event kinds end-to-end (writer pattern, like sessions' `ingest_session_report`), EXCEPT for descendant-grant issuance which is a pure log-tail (like the elicitation-slot layer) — because descendant grants are a *reaction* to spawn completion events that acceptance owns. The tail correlates a `Spawn → Completed` transition with a `SessionRegistered` event carrying `spawn_origin` (the sequenced sessions prerequisite), then issues the descendant grant.

The `GrantCheck` port (already declared in `core/src/acceptance/ports.rs`) is implemented by the `AuthorityRegistry`. It evaluates against the durable grant set using a verified `IssuerContext` (R2) — never a self-asserted payload field. The operator is identified by the verified `IssuerContext`, not by trusting `Operation.sender`. v0.1.0 `GrantCheck` evaluates the operator against the bootstrap operator grant; non-operator subjects against descendant grants. Revocation is durable and enforced; non-cascade is structural (no cascade mechanism — revoking a grant marks only that grant; descendant grants have separate grant_ids).

This shape honors Ports & Adapters (authority depends on `Storage` and implements `GrantCheck`; acceptance depends on the `GrantCheck` trait + an `IssuerContext` port, not on authority; the ingress implements `IssuerContext` later), Single Source of Truth (the event log is the only source of grant state; the in-memory registry is a pure fold), Generated Contracts (`Grant`/`DescendantGrant`/`Revocation` are generated proto messages; `StoredEventKind` discriminators are schema-owned; `SessionRegistered.spawn_origin` is an additive field), and Fail Fast (invalid grants, unknown grant kinds, unverified identity, and log corruption are rejected at the boundary).

## Cross-feature dependencies (sequenced)

1. **`story-sessions-spawn-origin-field`** (prerequisite, sessions feature) — add `spawn_origin: TypedCorrelation` (optional) to the `SessionRegistered` proto message; populate it from adapter spawn reports (the adapter correlates a session registration to the spawn command that created it). Regenerate contracts. This unblocks the authority descendant-grant reactor. **Must land before `story-v0-core-authority-spawn-tail`.**
2. **`story-acceptance-issuer-context`** (prerequisite, acceptance feature) — change `submit` to take an `&IssuerContext` (resolved from an injected port or passed by the caller) instead of reading `Operation.sender` for the grant check. `Operation.sender` stays for audit. The `IssuerContext` port is defined in the authority module (R2); acceptance depends on the trait. **Must land before `story-v0-core-authority-grant-check` can be integrated/tested end-to-end** (though the GrantCheck impl can be developed against a test double in parallel).

## Implementation Units

### Unit 1: Grant/revocation event model and the `AuthorityRegistry` projection

**File**: `core/src/authority/mod.rs`, `core/src/authority/state.rs`, `core/src/authority/events.rs`, `core/src/authority/registry.rs`

**Story**: `story-v0-core-authority-registry`

The durable event shape (already-defined proto messages — `Grant`, `DescendantGrant`, `Revocation`) and the in-memory projection that folds them. Mirrors `SessionRegistry`/`ElicitationSlotLayer`.

```rust
// core/src/authority/state.rs
use patchbay_contracts::patchbay::{
    ActorId, EndpointId, AuthorityDomainId, GrantId, TargetScope, OperationKind, Generation,
    GrantRevocationPolicy,
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
    pub expires_at: Option<::prost_types::Timestamp>,
    pub revocation_generation: Option<Generation>,
    pub revoked_at: Option<::prost_types::Timestamp>,
    pub revocation_policy: GrantRevocationPolicy,
    pub is_descendant: bool,
    pub provenance: GrantProvenanceKind,  // CreatedBy{actor, op_id, audit_id} | Descendant{spawn_op_id, spawning_grant_id}
}

/// A grant is live if not revoked. (Expiration needs a clock — deferred with
/// the time-driven staleness work; the field is stored but not enforced in
/// v0.1.0. This is a documented gap, filed as backlog.)
impl GrantRecord {
    pub fn is_revoked(&self) -> bool { self.revocation_generation.is_some() }
    pub fn is_live(&self) -> bool { !self.is_revoked() }
}

/// The canonical descendant-grant allowed-kind set from docs/PROTOCOL.md
/// "Spawn payload and authority commitments". SSOT — 8 existing-session kinds
/// (spawn + attach excluded).
pub const DESCENDANT_GRANT_ALLOWED_KINDS: &[OperationKind] = &[
    OperationKind::Instruct, OperationKind::Cancel, OperationKind::Interrupt,
    OperationKind::Query, OperationKind::ApprovalResponse, OperationKind::ElicitationResponse,
    OperationKind::Reconfigure, OperationKind::SessionManagement,
];

/// Does `grant` authorize `(issuer, kind, target)`? Deny-by-default.
/// Full matching matrix (addresses review blocker #3):
/// 1. grant is live (not revoked)
/// 2. authority_domain_id matches the grant's domain
/// 3. verified actor (from IssuerContext) matches grant subject_actor_id
/// 4. endpoint narrowing: if grant has subject_endpoint_id, the issuer's
///    verified endpoint must match (operator grants are endpoint-unscoped)
/// 5. operation_kind is in grant.allowed_operation_kinds
/// 6. target_scope_matches(grant.target_scope, requested_target_scope)
///
/// `target_scope_matches` is the scope-containment predicate (review blocker #3):
/// fleet scope matches any target; adapter scope matches same adapter; runtime-session
/// scope matches same session; project-group matches containment. Specified in Unit 1.
#[must_use]
pub fn grant_authorizes(
    grant: &GrantRecord,
    issuer: &IssuerContext,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
    authority_domain_id: &AuthorityDomainId,
) -> bool {
    grant.is_live()
        && &grant.authority_domain_id == authority_domain_id
        && grant_authorized_actor(grant, issuer)
        && grant.allowed_operation_kinds.contains(&operation_kind)
        && target_scope_matches(&grant.target_scope, target_scope)
}

/// Scope-containment predicate. The grant's target_scope defines what it
/// authorizes; the requested target_scope must fall within it.
/// - FleetSupervisor scope grant authorizes any target.
/// - Adapter scope grant authorizes targets on the same adapter.
/// - RuntimeSession scope grant authorizes that exact session.
/// - ProjectSessionGroup scope grant authorizes sessions in that group.
/// - Exact-equality fallback for other kinds.
/// This is the semantic 50/50 the review flagged — pinned here, not left to
/// the implementer.
#[must_use]
pub fn target_scope_matches(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    // match on TargetScopeKind; fleet = wildcard; adapter = same adapter_id;
    // runtime-session = same identity tuple; project-group = containment.
}
```

```rust
// core/src/authority/registry.rs
use std::collections::HashMap;
use patchbay_contracts::patchbay::{AuthorityDomainId, GrantId, StoredEventKind};
use crate::storage::{RecordedEvent, Storage};

#[derive(Debug, Clone, Default)]
pub struct AuthorityRegistry {
    grants: HashMap<GrantId, GrantRecord>,
}

impl AuthorityRegistry {
    pub fn new() -> Self { Self::default() }

    /// Fold one committed event. Consumes Grant/DescendantGrant/Revocation;
    /// ignores others. Idempotent. Validates grant shape (Fail Fast).
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> { ... }

    pub fn get_grant(&self, grant_id: &GrantId) -> Option<&GrantRecord> { ... }

    /// Live grants for GrantCheck evaluation / tests.
    pub fn live_grants(&self) -> impl Iterator<Item = &GrantRecord> { ... }
}
```

**Implementation Notes**:
- `GrantRecord` projects BOTH `Grant` and `DescendantGrant` proto messages (`is_descendant` + `provenance` distinguishes). Stores `expires_at` + `revocation_policy` (review blocker #3) — though expiry enforcement is deferred (clock, backlog).
- `grant_authorizes` takes the verified `IssuerContext` (not a payload `ActorEndpointRef`) — review blocker #2. The actor comes from the verified context.
- `target_scope_matches` is specified here (review blocker #3) — the semantic 50/50 is pinned, not left to the implementer.
- `observe` validates grant shape (non-empty grant_id, subject, target_scope; valid OperationKinds; descendant grants must have exactly `DESCENDANT_GRANT_ALLOWED_KINDS`) — Fail Fast. `observe_revocation` marks revoked (not delete) — audit retention. Idempotent.

**Acceptance Criteria**:
- [ ] `observe` folds Grant, DescendantGrant, Revocation events correctly
- [ ] Revocation marks the grant revoked (not deleted); `is_live()` returns false after
- [ ] `grant_authorizes` returns true only when: live + domain matches + verified actor matches + (endpoint narrows if present) + kind allowed + target in scope
- [ ] `target_scope_matches` implements the scope-containment matrix (fleet=any, adapter=same adapter, runtime-session=exact, project-group=containment)
- [ ] `DESCENDANT_GRANT_ALLOWED_KINDS` matches PROTOCOL.md exactly (8 kinds, spawn+attach excluded)
- [ ] `observe` rejects malformed grants as `CorruptRecord`; idempotent for re-delivered events

---

### Unit 2: `IssuerContext` port + `GrantCheck` impl (the acceptance seam)

**File**: `core/src/authority/issuer.rs`, `core/src/authority/check.rs`

**Story**: `story-v0-core-authority-grant-check`

Defines the verified-identity port (R2) and implements `GrantCheck` against the durable grant set (R1). Depends on the acceptance `IssuerContext` integration (cross-feature dependency #2) for end-to-end testing.

```rust
// core/src/authority/issuer.rs
use patchbay_contracts::patchbay::{ActorId, EndpointId, DeviceId, AuthorityDomainId, Generation};

/// Verified issuer identity, supplied by the authenticated ingress boundary.
/// NOT self-asserted: the operator actor and transport endpoint come from
/// verified connection/session evidence (SECURITY.md "compound issuer").
/// The real impl lands with feature-v0-protocol-seam / feature-v0-web-server
/// (both at drafting); v0.1.0 tests supply a test double.
pub trait IssuerContext: Send + Sync {
    /// The verified operator actor. None if unauthenticated.
    fn verified_actor(&self) -> Option<&ActorId>;

    /// The verified transport endpoint (the web server, CLI endpoint, etc.).
    fn verified_endpoint(&self) -> Option<&EndpointId>;

    /// The verified device.
    fn verified_device(&self) -> Option<&DeviceId>;

    /// The endpoint generation (for staleness/revocation checks).
    fn endpoint_generation(&self) -> Option<Generation>;

    /// The authority domain this issuer was verified within.
    fn authority_domain_id(&self) -> &AuthorityDomainId;
}
```

```rust
// core/src/authority/check.rs
use patchbay_contracts::patchbay::{AuthorityDomainId, OperationKind, TargetScope, GrantId};
use crate::acceptance::ports::{GrantCheck, Authorized, GrantDenied};
use super::issuer::IssuerContext;
use super::registry::AuthorityRegistry;
use super::state::{grant_authorizes, GrantRecord};

/// impl GrantCheck for AuthorityRegistry. Evaluates the verified issuer
/// against the durable grant set. Deny-by-default.
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
        // Evaluate against durable grants (incl. bootstrap operator grant).
        // Deny-by-default: first live grant that authorizes wins.
        for grant in self.live_grants() {
            if grant_authorizes(grant, issuer, operation_kind, target_scope, authority_domain_id) {
                return Ok(Authorized { grant_id: Some(grant.grant_id.clone()) });
            }
        }
        Err(GrantDenied::NoGrant { actor: format!("{:?}", actor), kind: operation_kind, target: format!("{:?}", target_scope) })
    }
}
```

**Implementation Notes**:
- The `GrantCheck` port signature ALREADY EXISTS in `ports.rs` but takes `&ActorEndpointRef`. **This is a port signature change** — `GrantCheck::check` takes `&dyn IssuerContext` instead of `&ActorEndpointRef`. This is the acceptance integration (cross-feature dependency #2): the `submit` pipeline passes the verified `IssuerContext` instead of `validated.sender`. File the acceptance change as a prerequisite story.
- Deny-by-default: no verified actor → denied; no matching live grant → denied.
- The bootstrap operator grant (R1) is a durable `Grant` event created at init (Unit 3's `ingest_grant`); the registry folds it on replay. `GrantCheck` evaluates the operator against it like any other grant — no special-casing, no implicit bypass.
- v0.1.0 tests supply a `TestIssuerContext` double; the real impl lands with the ingress features.

**Acceptance Criteria**:
- [ ] `check` returns `Authorized { grant_id: Some(...) }` for a verified operator with the bootstrap grant
- [ ] `check` returns `Authorized` for a non-operator with a live matching descendant grant
- [ ] `check` returns `GrantDenied` for an unauthenticated issuer (no verified actor)
- [ ] `check` returns `GrantDenied` for a revoked grant (revocation prevents future)
- [ ] `check` returns `GrantDenied` for a kind/target not covered by any live grant (deny-by-default)
- [ ] `GrantCheck` port signature changed to take `&dyn IssuerContext`; acceptance `submit` passes the verified context (cross-feature dependency #2 landed)

---

### Unit 3: Grant + revocation ingestion (the writer) + bootstrap

**File**: `core/src/authority/ingest.rs`

**Story**: `story-v0-core-authority-ingest`

The direct ingestion writer for grants, descendant grants, and revocations — the analog of sessions' `ingest_session_report`. Owns its event kinds end-to-end (writer pattern). Also: bootstrap operator grant creation at init.

```rust
// core/src/authority/ingest.rs
use patchbay_contracts::patchbay::{AuthorityDomainId, Grant, DescendantGrant, Revocation, GrantId, EventId};
use crate::storage::Storage;
use super::projection::GrantProjection;  // GrantLookup + observe(&mut self, ...)

/// Read + warm port (mirrors sessions' SessionProjection post-B5 fix).
/// The writer takes &mut L so it can warm after each append (retry-safe).
pub trait GrantProjection: Send + Sync {
    fn current_grant(&self, grant_id: &GrantId) -> impl std::future::Future<Output = Option<GrantRecord>> + Send;
    fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError>;
}

pub async fn ingest_grant<S, L>(storage: &S, projection: &mut L, grant: Grant) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantProjection { ... }

/// Descendant grant (from spawn completion). Validates allowed-kinds match
/// DESCENDANT_GRANT_ALLOWED_KINDS exactly (Fail Fast).
pub async fn ingest_descendant_grant<S, L>(storage: &S, projection: &mut L, grant: DescendantGrant) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantProjection { ... }

/// Revocation. Two-lever non-cascade: revokes ONLY the named grant.
/// The projection fold marks that one grant revoked; no other.
pub async fn ingest_revocation<S, L>(storage: &S, projection: &mut L, revocation: Revocation) -> Result<EventId, AuthorityError>
where S: Storage, L: GrantProjection { ... }

/// Create the bootstrap operator grant at first-start (fleet spawn grant +
/// universal existing-session grant). Idempotent: no-op if it already exists.
pub async fn ensure_bootstrap_operator_grant<S, L>(
    storage: &S, projection: &mut L, authority_domain_id: &AuthorityDomainId, operator_actor_id: &ActorId,
) -> Result<GrantId, AuthorityError> where S: Storage, L: GrantProjection { ... }
```

**Implementation Notes**:
- `GrantProjection` takes `&mut L` with `observe(&mut self, ...)` — review blocker #7 (warm-after-write). Mirrors sessions' post-B5 `SessionProjection`. Warm after each successful append so retry is idempotent.
- `ingest_descendant_grant` validates the allowed-kind set matches `DESCENDANT_GRANT_ALLOWED_KINDS` exactly (Fail Fast, review blocker #3).
- `ingest_revocation` is the two-lever non-cascade enforcement: revokes ONLY the named grant. No cascade code path (structural). Revoking a non-existent grant → error (Fail Fast).
- `ensure_bootstrap_operator_grant` (R1) creates the durable operator grant at init: a fleet-scope spawn grant + universal existing-session grant, subject = operator actor. Idempotent (checks if it exists first). This is what makes `GrantCheck` evaluate the operator against a real grant instead of implicit bypass.

**Acceptance Criteria**:
- [ ] `ingest_grant` writes a Grant event; projection reflects it
- [ ] `ingest_descendant_grant` rejects a descendant with the wrong allowed-kind set
- [ ] `ingest_revocation` marks ONLY the named grant revoked (non-cascade, two-lever)
- [ ] `ingest_revocation` does NOT revoke descendant grants under the revoked grant
- [ ] Revoking a non-existent grant returns an error
- [ ] Warm-after-write keeps the projection consistent (retry-safe)
- [ ] `ensure_bootstrap_operator_grant` creates the operator grant; idempotent on re-call

---

### Unit 4: Descendant-grant-on-spawn log-tail reactor (vertical slice)

**File**: `core/src/authority/spawn_tail.rs`

**Story**: `story-v0-core-authority-spawn-tail` (depends on `story-sessions-spawn-origin-field`)

The pure log-tail that reacts to spawn completion by issuing the descendant grant. Vertical slice (R3a): correlates `Spawn → Completed` with a `SessionRegistered` carrying `spawn_origin`.

```rust
// core/src/authority/spawn_tail.rs
use std::collections::{HashMap, HashSet};
use patchbay_contracts::patchbay::{CommandId, OperationKind, OperationState, StoredEventKind, TypedCorrelation, TargetScope, ActorId, GrantId};
use crate::storage::{RecordedEvent, Storage};
use super::state::DESCENDANT_GRANT_ALLOWED_KINDS;
use super::AuthorityError;

/// Reactor: tails the log for completed spawns and produces descendant-grant
/// issuances. Mirrors ElicitationSlotLayer (read-only over the command log),
/// but produces a side-effect (the issuance) the composition layer writes via
/// ingest_descendant_grant.
///
/// Correlation (R3a): a descendant grant is issued when:
/// 1. a Spawn OPERATION event is seen (track command_id)
/// 2. a COMMAND_TRANSITION to Completed for that command_id is seen
/// 3. a SessionRegistered event carrying spawn_origin = that command_id is seen
/// All three correlate; the SessionRegistered provides the spawned session identity.
#[derive(Debug, Clone, Default)]
pub struct SpawnDescendantTail {
    spawn_commands: HashMap<CommandId, SpawnInfo>,  // command_id -> spawn op info
    completed: HashSet<CommandId>,                   // spawns that reached Completed
    issued: HashSet<CommandId>,                      // idempotent: already produced issuance
}

struct SpawnInfo {
    spawning_grant_id: Option<GrantId>,  // from the spawn's authorization (if retained)
    spawner_actor: ActorId,
}

impl SpawnDescendantTail {
    pub fn new() -> Self { Self::default() }

    /// Fold one committed event. On a completed spawn with a correlated
    /// SessionRegistered, produce a DescendantGrantIssuance.
    pub fn observe(
        &mut self,
        event: &RecordedEvent,
    ) -> Result<Option<DescendantGrantIssuance>, AuthorityError> {
        // Track Spawn OPERATION events (record command_id + spawner).
        // Track COMMAND_TRANSITION to Completed for spawn commands.
        // Track SessionRegistered with spawn_origin = a tracked spawn command_id:
        //   when that spawn is also Completed and not yet issued, produce the issuance.
        //   The SessionRegistered provides the spawned session identity (adapter/deployment/runtime/gen).
    }
}

pub struct DescendantGrantIssuance {
    pub spawn_operation_id: CommandId,
    pub spawning_grant_id: Option<GrantId>,
    pub spawned_session_scope: TargetScope,  // from the SessionRegistered event
    pub subject_actor_id: ActorId,           // the spawner
    pub allowed_operation_kinds: Vec<OperationKind>,  // DESCENDANT_GRANT_ALLOWED_KINDS
    pub authority_domain_id: AuthorityDomainId,
}
```

**Implementation Notes**:
- This is the vertical slice (R3a). It depends on `story-sessions-spawn-origin-field` (the `SessionRegistered.spawn_origin` field) — sequenced first.
- The reactor correlates THREE events: the Spawn `OPERATION`, its `COMMAND_TRANSITION` to `Completed`, and the `SessionRegistered` carrying `spawn_origin = that command_id`. The `SessionRegistered` provides the spawned session identity (adapter/deployment/runtime/generation) — review blocker #4.
- `issued` is in-memory idempotency for the live tail; durable idempotency comes from the composition layer using a deterministic grant_id derived from `(authority_domain_id, spawn_command_id)` (so replay doesn't duplicate). Review blocker #5: the reactor is read-only over the log; the composition layer owns the write + durable dedup. The feature owns the wiring story (Unit 5/composition) — it does not leave the reactor unwired.
- `spawning_grant_id`: retained from the spawn's authorization IF the pipeline retains it. The acceptance `submit` currently discards `Authorized.grant_id`. **Integration note:** to populate provenance fully, acceptance must retain the `Authorized.grant_id` on the command record (small acceptance change). If not retained, `spawning_grant_id` is `None` for operator-authorized spawns (the bootstrap operator grant has a grant_id, so this IS populatable if acceptance retains it). File the acceptance change as part of cross-feature dependency #2 or a follow-on.

**Acceptance Criteria**:
- [ ] A Spawn OPERATION + Completed transition + SessionRegistered(spawn_origin=that command) produces exactly one `DescendantGrantIssuance`
- [ ] A spawn reaching a non-Completed terminal produces NO issuance
- [ ] A SessionRegistered without `spawn_origin` does NOT trigger an issuance
- [ ] Replay (re-observing events) does not produce duplicate issuances (idempotent via `issued` + deterministic grant_id in the composition layer)
- [ ] The issuance's allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly
- [ ] The issuance's `spawned_session_scope` comes from the `SessionRegistered` event (not the spawn Operation's fleet target)

---

### Unit 5: Replay, composition/wiring, and module wiring

**File**: `core/src/authority/replay.rs`, `core/src/authority/composition.rs`, `core/src/authority/mod.rs`, `core/src/lib.rs`

**Story**: `story-v0-core-authority-replay`

Rebuild the registry from the log; wire the reactor's output to the writer (the composition layer the review demanded — blocker #5); export the module.

```rust
// core/src/authority/replay.rs
pub async fn rebuild_from_log<S: Storage>(
    storage: &S, authority_domain_id: &AuthorityDomainId,
) -> Result<AuthorityRegistry, AuthorityError> {
    // Near-exact copy of session::rebuild_from_log: read_after(Lsn{0}), fold via observe,
    // validate LSN monotonicity + domain match.
}
```

```rust
// core/src/authority/composition.rs
/// The composition layer: owns the live reactor loop + durable dedup.
/// On a completed-spawn issuance, derives a deterministic grant_id from
/// (authority_domain_id, spawn_command_id) and calls ingest_descendant_grant.
/// Deterministic grant_id = durable idempotency (replay doesn't duplicate).
pub struct AuthorityComposition {
    registry: AuthorityRegistry,
    spawn_tail: SpawnDescendantTail,
}

impl AuthorityComposition {
    /// Observe a committed event: fold into the registry AND the spawn tail.
    /// If the tail produces an issuance, write the descendant grant durably.
    /// Deterministic grant_id makes this idempotent across replay/crash.
    pub async fn observe<S: Storage>(
        &mut self, storage: &S, event: &RecordedEvent,
    ) -> Result<(), AuthorityError> { ... }
}
```

**Implementation Notes**:
- `rebuild_from_log` mirrors `session::rebuild_from_log` / `elicitation::rebuild_slots_from_log`.
- `AuthorityComposition` (review blocker #5) owns the wiring: it folds each event into BOTH the registry (for grant state) AND the spawn tail (for descendant-grant issuance). When the tail produces an issuance, the composition layer writes the descendant grant via `ingest_descendant_grant` with a **deterministic grant_id** derived from `(authority_domain_id, spawn_command_id)` — this makes issuance idempotent across replay/crash (a re-observed completed spawn produces the same grant_id, so the write is a no-op duplicate). This is the durable idempotency the review demanded.
- The composition layer is the live consumer loop; replay uses `rebuild_from_log` (registry-only) + re-runs the composition's tail over the log to catch any issuances missed before a crash (catch-up). Review blocker #5's recovery-safe protocol.
- Module wiring: `core/src/authority/` alongside `acceptance/`, `session/`, `storage/`; `lib.rs` exports it.

**Acceptance Criteria**:
- [ ] `rebuild_from_log` reconstructs the registry identically to a live registry
- [ ] `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`
- [ ] `AuthorityComposition::observe` folds events into the registry AND issues descendant grants on completed spawns
- [ ] A crashed-then-restarted composition does NOT issue duplicate descendant grants (deterministic grant_id)
- [ ] `core/src/authority/` module compiles and is exported from `core/src/lib.rs`

---

### Unit 6: Property tests for authority invariants (8 properties)

**File**: `core/tests/authority_proptest.rs`

**Story**: `story-v0-core-authority-proptests`

Property tests for the 8 stated-normative obligations (review blocker #10: corrected count). None are formally checked, but each is testable as an executable oracle.

```rust
proptest! {
    /// 1. NoCommandWithoutGrant: a command that reaches accepted does so only
    ///    with a live matching grant. Deny-by-default.
    #[test] fn no_command_without_grant(/* ... */) { ... }

    /// 2. CompoundIssuer: accepted commands use verified issuer identity (from
    ///    IssuerContext), not self-asserted payload actor.
    #[test] fn compound_issuer(/* ... */) { ... }

    /// 3. GrantAuthorityIsCommandKinds: grant checks constrain authority by
    ///    canonical OperationKinds, not adapter capability.
    #[test] fn grant_authority_is_command_kinds(/* ... */) { ... }

    /// 4. RevocationPreventsFuture: after a grant is revoked, subsequent
    ///    checks for that grant's subject/kind/target are denied.
    #[test] fn revocation_prevents_future(/* ... */) { ... }

    /// 5. FleetAuthorityForSpawn: spawn acceptance requires a live fleet-scope
    ///    spawn grant; per-session grants alone cannot authorize spawning.
    #[test] fn fleet_authority_for_spawn(/* ... */) { ... }

    /// 6. SpawnCreatesDescendantGrant: successful spawn produces a descendant
    ///    grant for the spawned session with non-spawn OperationKinds.
    #[test] fn spawn_creates_descendant_grant(/* ... */) { ... }

    /// 7. SpawnRevocationDoesNotCascade: revoking a spawn grant does NOT revoke
    ///    descendant grants issued under it. Two independent levers. (Executable
    ///    stand-in for the demoted formal property — mutation-survivable.)
    #[test] fn spawn_revocation_does_not_cascade(/* ... */) { ... }

    /// 8. ElicitationResponderAuthority: response Operations are accepted only
    ///    when the verified issuer maps to the expected responder actor.
    #[test] fn elicitation_responder_authority(/* ... */) { ... }
}
// Mutation tests: a buggy registry that cascades revocation MUST fail #7;
// a buggy GrantCheck that trusts payload actor MUST fail #2.
```

**Implementation Notes**:
- 8 properties (review blocker #10: was miscounted as 7). All stated-normative (draft) — executable oracles, not formally checked.
- `spawn_revocation_does_not_cascade` (#7) is the executable stand-in for the demoted formal property. Must be mutation-survivable: create parent spawn grant P, descendant D with provenance linking to P, revoke P, prove P denies + D still authorizes, separately revoke D, prove denial (review blocker #9 — both levers).
- `compound_issuer` (#2) tests that a self-asserted payload actor is NOT trusted — the `IssuerContext` is the authority, not `Operation.sender`.
- Mutation tests essential (non-vacuity): cascade-revocation mutation fails #7; payload-actor-trust mutation fails #2.

**Acceptance Criteria**:
- [ ] All 8 properties pass against the real implementation
- [ ] #7 fails against a cascade mutation (non-vacuous)
- [ ] #2 fails against a payload-actor-trust mutation (non-vacuous)
- [ ] `replay_matches_live` passes (replay determinism — supplementary, not one of the 8)

---

## Implementation Order

0. **`story-sessions-spawn-origin-field`** (prerequisite, sessions feature) — add `SessionRegistered.spawn_origin`. **Must land before story 4.**
0b. **`story-acceptance-issuer-context`** (prerequisite, acceptance feature) — `submit` takes `&IssuerContext`; retain `Authorized.grant_id` on the command record. **Must land before story 2 integrates end-to-end.**
1. `story-v0-core-authority-registry` — grant/revocation event model + `AuthorityRegistry` projection (no deps; SSOT for grant state + `grant_authorizes` + `target_scope_matches`)
2. `story-v0-core-authority-grant-check` — `IssuerContext` port + `impl GrantCheck` (depends on 1; end-to-end test depends on 0b)
3. `story-v0-core-authority-ingest` — grant/revocation writer + bootstrap (depends on 1)
4. `story-v0-core-authority-spawn-tail` — descendant-grant reactor (depends on 1, 3, AND prerequisite 0)
5. `story-v0-core-authority-replay` — `rebuild_from_log` + composition/wiring (depends on 1, 2, 3, 4)
6. `story-v0-core-authority-proptests` — 8 property oracles + mutation tests (depends on 1-5)

Stories 1 is the foundation. 2 and 3 can proceed in parallel after 1 lands (both depend on the registry, not each other). 4 depends on 1, 3, and the sessions prerequisite (0). 5 depends on 1-4. 6 depends on all. The two prerequisites (0, 0b) can proceed in parallel with story 1.

## Testing

### Unit Tests: `core/tests/authority_*.rs`
- `authority_registry.rs` — fold correctness, revocation marks-not-deletes, idempotent observe, malformed-grant rejection, `target_scope_matches` matrix
- `authority_grant_check.rs` — operator-with-bootstrap-grant, non-operator descendant grant, unauthenticated denied, revoked denied, kind/target mismatch denied, payload-actor-NOT-trusted
- `authority_ingest.rs` — grant/descendant/revocation ingestion, descendant allowed-kind validation, non-cascade revocation, bootstrap idempotency
- `authority_spawn_tail.rs` — completed-spawn-produces-issuance, non-Completed no issuance, no-spawn_origin no issuance, idempotent replay
- `authority_replay.rs` — replay determinism, LSN monotonicity, cross-domain rejection, composition catch-up after crash
- `authority_proptest.rs` — 8 property oracles + mutation tests

### Integration Points
- **Acceptance ↔ Authority**: acceptance calls `GrantCheck::check` with a verified `IssuerContext` (not `Operation.sender`). Cross-feature dependency #2.
- **Authority ↔ Sessions**: the spawn-tail consumes `SessionRegistered` events with `spawn_origin`. Cross-feature dependency #1.
- **Authority ↔ Storage**: writes Grant/DescendantGrant/Revocation via `Storage::append`; reads via `Storage::read_after` for replay.
- **Authority ↔ Elicitation**: no direct coupling. Both are independent log consumers.

## Risks

- **Weakest formal backing.** All 8 `authority.qnt` properties are stated-normative; four actively demoted. The property tests (Unit 6) are executable oracles, NOT formally checked. The v1 formal gate owns the real authority properties. Documented; not over-claimed.
- **Two cross-feature prerequisites.** The vertical slice depends on (0) sessions adding `SessionRegistered.spawn_origin` and (0b) acceptance taking `&IssuerContext` + retaining `Authorized.grant_id`. Both are small, additive changes owned by their features. The depends_on chain makes the sequencing explicit.
- **Bootstrap operator grant is a v0.1.0 construct.** Created at init, durable. The operator actor id is a deployment value. Future multi-operator work adds operator provisioning — a reversal, not a gap-fill. Made explicit.
- **Expiry enforcement deferred.** `expires_at` is stored but not enforced (needs a clock — deferred with time-driven staleness, same as sessions). Filed as backlog.
- **Fleet target resolution gap (R5).** Until OperationKind-aware target resolution lands (backlog), spawn Operations fail target resolution after passing the grant check. The authority feature is still valuable and testable independently; the spawn end-to-end path requires the backlog item. Filed.
- **Distinct failed-authorization audit deferred (R4).** Grant-lifecycle provenance is delivered; the distinct SECURITY.md audit record for denied attempts is a separate concern. Filed as backlog.
- **Snapshot checkpointing deferred.** Same as the other projections — replay from LSN 0.

## Extension pressure classification

- **Committed v0.1.0 behavior**: deny-by-default grant evaluation against durable grants; the `GrantCheck` port with verified `IssuerContext` (not self-asserted); durable Grant/DescendantGrant/Revocation events with provenance; bootstrap operator grant; revocation marks-not-deletes (audit retention); the descendant-grant allowed-kind set (8 existing-session kinds, spawn+attach excluded); two-lever non-cascade revocation (structural); descendant-grant-on-spawn via log-tail correlating `spawn_origin`; the `AuthorityDomainId` key shape (federation seam).
- **Reserved seam**: multi-operator authority domains + operator provisioning (replaces the bootstrap grant — a reversal); delegation lineage (`parent_grant_id` — explicitly absent); per-spawn-variant authority; tighter endpoint-class narrowing; expiration enforcement (needs a clock — deferred); distinct failed-authorization audit records (deferred, R4); cascade-revocation as a query (future, no schema change); fleet target resolution for spawn (R5 backlog — acceptance/sessions concern).
- **Explicitly rejected**: trusting self-asserted actor identity (compound-issuer rule — `IssuerContext` is authority, never `Operation.sender`); adapter capability declarations as grant authority; making acceptance write descendant grants (Ports & Adapters violation — authority owns the reaction); cascading revocation as v0.1.0 behavior (two-lever is the rule).

## Prior review history

- **Revision 1** (Q1-Q5, 2026-07-13): implicit operator authority + log-tail reactor + full protocol model. Pre-implementation design review (cross-model gpt-5.6-sol) found 10 blockers. Bounced to drafting.
- **Revision 2** (R1-R5, this): vertical slice — durable bootstrap operator grants (R1a), verified `IssuerContext` port (R2a), descendant-grant reactor with `spawn_origin` correlation (R3a), minimal audit (R4c), fleet resolution out-of-scope (R5a). Addresses all 10 blockers.
