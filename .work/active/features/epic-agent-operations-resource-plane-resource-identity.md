---
id: epic-agent-operations-resource-plane-resource-identity
kind: feature
stage: done
tags: [foundation, protocol, adapter]
parent: epic-agent-operations-resource-plane
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-08-04
---

# Resource identity, resolution & authority

## Brief

Promote the existing generic `TargetScopeKind = resource` from an untyped
`string resource_id` into a designed resource identity with target-resolution
semantics distinct from runtime-session identity. This is the foundation
feature for the resource plane: every other child feature depends on a
resource having a stable, typed, resolvable identity.

Today a resource-target Operation passes envelope and grant validation but
fails production target resolution because `TargetResolver` is hard-coded to
session fields (`core/src/session/resolver.rs` requires `runtime_session_id`).
This feature refactors the `TargetResolver` port to be target-kind-polymorphic
and adds a resource registry/resolver branch so resource targets resolve
without fabricating session identity. It also refines grant containment to
match on the full resource identity tuple (adapter_id, resource_id, kind)
rather than only `resource_id`, fencing cross-adapter resource-ID collision.

It does not define resource snapshot/revision state (that is `resource-state`),
the adapter capability manifest for resources (`capability-manifest`), or any
cockpit rendering (`cockpit-composition`).

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: foundation feature — others depend on its typed identity and resolver.

## Simplification opportunity

- Reuse the existing `TargetScope` envelope, `TargetScopeKind::Resource`, and the `TargetResolver` port (already generically named) rather than creating a parallel resolution subsystem. The polymorphism is the intended shape, not a retrofit.
- Eliminate the temptation to synthesize fake runtime-session identity for non-session targets.

## Foundation references

- `docs/ARCHITECTURE.md` — adapter plane; resource plane
- `docs/PROTOCOL.md` — target scopes, grants, `TargetScopeKind`
- `contracts/proto/patchbay/common.proto:80-99` — `TargetScope`, `TargetScopeKind`
- `contracts/proto/patchbay/authority.proto` — grant target scopes
- `core/src/acceptance/ports.rs:91-96` — `TargetResolver` result hard-coded to session fields
- `core/src/session/resolver.rs` — production resolver requires session identity
- `core/src/authority/state.rs:283-285` — resource containment ignores adapter_id/subtype

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`
- No direct UI in this feature; it is the identity/resolution foundation the cockpit feature renders.

## Design decisions

- **Resource identity is a three-part tuple, not a larger scalar `ResourceId`.** `ResourceId` is an adapter-local typed scalar; the routable identity is `ResourceIdentity = (adapter_id, resource_kind, resource_id)`. Keeping the adapter and kind structural prevents the same local id from colliding across adapters or across two resource collections exposed by one adapter.
- **`ResourceKind` is an open typed identifier, not a core enum.** Identity requires a non-empty kind now, while the sibling capability-manifest feature owns which kinds an authenticated adapter declares. This keeps token-commune/provider vocabulary out of the core registry without leaving the identity stringly and ambiguous.
- **Resource identity has no runtime generation.** Resource replacement, tombstone, and revision semantics belong to `resource-state`; this feature must not copy runtime-session generation into a non-session identity or preselect the sibling state model.
- **The wire changes in place without losing durable audit decoding.** Protobuf tag 8 is renamed to `legacy_audit_resource_id` and retained only for the existing control-surface principal/endpoint/device audit targets. New operational resource Operations and Grants must carry field 9 `resource`; the acceptance, resolver, and authority boundaries reject the legacy scalar for operational use. The tag-preserving rename lets existing durable audit records decode and keeps the cited audit producers working while preventing them from entering the resource registry or grant matcher.
- **Resolution returns a target-kind enum.** `TargetBinding` becomes `RuntimeSession | Resource | AuthorityDomain`. Session resolution keeps its existing existence/generation behavior, resource resolution binds the exact typed identity, and the diagnostics-only resolver returns an honest authority-domain binding instead of fabricating a runtime session and generation.
- **One composite resolver dispatches ordinary targets.** The production resolver matches `TargetScopeKind` once and delegates to the existing `SessionRegistry` or the new `ResourceRegistry`. A parallel resource acceptance pipeline is rejected. Unsupported ordinary target kinds fail closed; core diagnostics retains its explicit special resolver.
- **Identity registration is intentionally state-agnostic.** `ResourceRegistry` owns only exact identity membership and lookup. It exposes a registration seam for the authenticated resource-report projection that `resource-state` will add; this feature adds no resource report, snapshot, revision, completeness tier, or stored-event variant. Until that sibling supplies a durable identity, an unregistered resource correctly returns `target_not_found`.
- **Resource grants are exact; broader authority stays explicit.** A `RESOURCE` grant matches only the same typed tuple and only a requested `RESOURCE` target. Adapter, fleet, and authority-domain grants remain the existing explicit ways to authorize a wider scope; there is no implicit same-kind or same-id wildcard.
- **Autopilot rationale.** The choices above are the least-irreversible shapes consistent with the foundation: an open adapter-owned kind, no invented generation, one existing resolver port, fail-closed registration, and a tag-preserving audit compatibility field. No strategic question is required.

## Codebase mapping

Direct reading covered the generated common/authority/adapter contracts, acceptance port and pipeline, session registry/resolver and replay tests, authority grant validation/matching and property tests, server projection composition and adapter delivery routing, diagnostics' authority-domain resolver, durable audit indexing, CLI audit target parsing, and the three control-surface audit producers. This is a bounded cross-cutting contract refactor with known call sites, so no exploratory fan-out was needed. The design-time advisory path was warranted by the authority and identity risk, but this delegated worker exposes no independent subagent/peer dispatch tool; the pre-mortem and the later explicit `thorough` feature review are the available scrutiny paths.

## Architectural choice

### Options considered

1. **Nested typed resource identity + polymorphic binding + composite registry (chosen).** Add typed `ResourceId`, `ResourceKind`, and `ResourceIdentity` messages; carry the tuple as one `TargetScope.resource`; return a `TargetBinding` enum; and dispatch through session/resource registries. This optimizes for unambiguous routing, adapter neutrality, and one acceptance path. It costs a generated-contract change and coordinated call-site updates.
2. **Keep flattened `adapter_id + string resource_id + string resource_kind`.** This is mechanically smaller and keeps current delivery filtering untouched, but permits partial/contradictory resource tuples, leaves `resource_id` untyped, and makes each consumer rediscover which flattened fields form identity. It also keeps the audit overload indistinguishable from an operational resource.
3. **Create a separate resource Operation/resolver subsystem.** This avoids changing `TargetBinding`, but duplicates acceptance ordering, grant checks, idempotency scoping, diagnostics, and delivery rules. It violates the epic's simplification direction and would let session/resource safety semantics drift.

The chosen option makes the tuple one generated boundary value and keeps acceptance-owned resolution polymorphic. The trickiest unit is the **contract-to-production resolver cutover**: every session path must keep resolving, resource delivery must select the nested adapter id, and the diagnostics resolver must stop synthesizing session identity without accidentally allowing ordinary Submit to target the core. That unit lands immediately after the contract checkpoint and is closed by integrated regression evidence.

## Implementation Units

### Unit 1: Typed resource identity and canonical boundary parser

**Files**: `contracts/proto/patchbay/common.proto`, `contracts/rust/src/gen/patchbay/patchbay.rs`, `contracts/ts/src/gen/patchbay/common_pb.ts`, `core/src/resource/mod.rs` (new), `core/src/resource/identity.rs` (new), `core/src/lib.rs`, `core/tests/resource_identity.rs` (new)

**Story**: `epic-agent-operations-resource-plane-resource-identity-typed-resource-identity`

```proto
message ResourceId {
  string value = 1;
}

// Adapter-owned open identifier. The capability manifest owns the admitted set.
message ResourceKind {
  string value = 1;
}

message ResourceIdentity {
  AdapterId adapter_id = 1;
  ResourceId resource_id = 2;
  ResourceKind resource_kind = 3;
}

message TargetScope {
  TargetScopeKind kind = 1;
  ActorId actor_id = 2;
  AdapterId adapter_id = 3;
  RuntimeSessionId runtime_session_id = 4;
  Generation session_generation = 5;
  string deployment_scope = 6;
  string project_or_group = 7;
  // Tag-preserved audit-only field for principal/endpoint/device records.
  // Invalid as operational resource identity in Operations and Grants.
  string legacy_audit_resource_id = 8;
  ResourceIdentity resource = 9;
}
```

```rust
// core/src/resource/identity.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceIdentity {
    adapter_id: AdapterId,
    resource_id: ResourceId,
    resource_kind: ResourceKind,
}

impl ResourceIdentity {
    pub fn new(
        adapter_id: AdapterId,
        resource_kind: ResourceKind,
        resource_id: ResourceId,
    ) -> Result<Self, ResourceIdentityError>;
    pub fn try_from_scope(scope: &TargetScope) -> Result<Self, ResourceIdentityError>;
    pub fn to_scope(&self) -> TargetScope;
    pub fn adapter_id(&self) -> &AdapterId;
    pub fn resource_kind(&self) -> &ResourceKind;
    pub fn resource_id(&self) -> &ResourceId;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResourceIdentityError {
    #[error("target scope is not a resource")]
    WrongTargetKind,
    #[error("resource identity is missing {field}")]
    Missing { field: &'static str },
    #[error("resource identity contains non-resource target fields")]
    MixedTargetFields,
    #[error("legacy audit resource id is not an operational resource identity")]
    LegacyAuditOnly,
}
```

**Implementation notes**:

- `new` and `try_from_scope` require non-empty adapter/kind/id values; private fields prevent a later registry producer from constructing an unchecked domain identity. `try_from_scope` additionally requires `TargetScopeKind::Resource` and rejects actor/session/project fields, a top-level adapter id, the legacy audit scalar, and dual encodings so protobuf bytes, authority matching, and idempotency keys cannot diverge for the same resource.
- The authority domain is not repeated in `ResourceIdentity`: resource registries and grants are already evaluated within the `authority_domain_id` passed to the acceptance port. Future federation composes domain + resource identity at the domain boundary.
- Keep tag 8's wire type and number. Update generated field names and repository-owned callers rather than adding a dual-read operational path. Existing stored audit bytes continue to decode; old raw resource Operations decode only into the legacy field and fail closed.
- Generate both Rust and TypeScript; generated files are artifacts and are never hand-edited.

**Acceptance criteria**:

- [ ] `(adapter-a, pool, shared)` differs from `(adapter-b, pool, shared)` and `(adapter-a, window, shared)` in equality, hashing, protobuf encoding, and target-key encoding.
- [ ] Empty, partial, mixed, legacy-only, and dual resource shapes are rejected before stateful work.
- [ ] Existing tag-8 audit records decode with their target id intact; no tag-8 value can become an operational `ResourceIdentity`.
- [ ] Generated Rust/TypeScript build and drift checks pass.

### Unit 2: Target-kind-polymorphic resolution and production composition

**Files**: `core/src/acceptance/ports.rs`, `core/src/acceptance/mod.rs`, `core/src/session/resolver.rs`, `core/src/session/registry.rs`, `core/src/resource/registry.rs` (new), `core/src/resource/resolver.rs` (new), `core/src/target.rs` (new), `core/src/diagnostics/mod.rs`, `server/src/state.rs`, `server/src/adapter_service.rs`, `core/tests/sessions_replay_resolver.rs`, `core/tests/resource_resolver.rs` (new), `server/src/adapter_service/tests.rs`

**Story**: `epic-agent-operations-resource-plane-resource-identity-polymorphic-target-resolution`

```rust
// core/src/acceptance/ports.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetBinding {
    RuntimeSession {
        adapter_id: AdapterId,
        deployment_scope: String,
        runtime_session_id: RuntimeSessionId,
        session_generation: Generation,
    },
    Resource(ResourceIdentity),
    AuthorityDomain(AuthorityDomainId),
}

pub trait TargetResolver: Send + Sync {
    fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> impl Future<Output = Result<TargetBinding, TargetNotFound>> + Send;
}
```

```rust
// core/src/resource/registry.rs
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceRegistry {
    identities: HashSet<ResourceIdentity>,
}

impl ResourceRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, identity: ResourceIdentity) -> bool;
    pub fn contains(&self, identity: &ResourceIdentity) -> bool;
    pub fn resources(&self) -> impl Iterator<Item = &ResourceIdentity>;
}

// core/src/target.rs
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetRegistry {
    sessions: SessionRegistry,
    resources: ResourceRegistry,
}

impl TargetRegistry {
    pub fn new(sessions: SessionRegistry, resources: ResourceRegistry) -> Self;
    pub fn sessions(&self) -> &SessionRegistry;
    pub fn sessions_mut(&mut self) -> &mut SessionRegistry;
    pub fn resources(&self) -> &ResourceRegistry;
    pub fn resources_mut(&mut self) -> &mut ResourceRegistry;
    pub fn observe_session_event(&mut self, event: &RecordedEvent) -> Result<(), SessionError>;
}

impl TargetResolver for TargetRegistry {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> Result<TargetBinding, TargetNotFound>;
}

pub fn target_adapter_id(scope: &TargetScope) -> Option<&AdapterId>;
```

**Implementation notes**:

- `TargetRegistry::resolve` parses `TargetScopeKind` once: runtime-session delegates to the existing session resolver, resource delegates to exact membership in `ResourceRegistry`, and every other ordinary kind returns `TargetNotFound`. It never falls through from malformed resource fields into the session branch.
- Preserve session behavior: a requested tombstone or wrong generation fails; absent generation may still select the live generation where existing callers rely on it; connectivity remains a delivery concern. The returned enum now also carries deployment scope so the binding is the complete session identity.
- `AuthorityDomainTargetResolver` remains separate and returns `TargetBinding::AuthorityDomain(authority_domain_id.clone())`. Ordinary Submit never receives that special resolver, preserving the core-local diagnostics boundary without fake session values.
- `ProjectionState::LockedTargetResolver` wraps `TargetRegistry`; rebuild/catch-up continue folding session events through `observe_session_event`. The resource registry starts empty and is populated only through its typed seam until `resource-state` adds durable resource-event folding.
- `target_adapter_id` is the single routing helper for adapter, runtime-session, and resource target shapes. Its resource branch returns an adapter only after the full resource parser succeeds. Adapter delivery subscriptions and authenticated Observation checks use it, so nested resource identity routes only to its owning adapter. Capability declarations remain advisory and are not consulted.

**Acceptance criteria**:

- [ ] Every existing session resolver/replay case returns the `RuntimeSession` binding with unchanged tombstone, generation, offline, and failed-state behavior.
- [ ] A registered typed resource resolves to `TargetBinding::Resource`; an unknown, legacy-only, mixed, or cross-adapter tuple returns `target_not_found`.
- [ ] Diagnostics query resolution returns `AuthorityDomain` and cannot make ordinary Submit resolve a core-local target.
- [ ] A resource Operation is delivered only on the authenticated adapter named inside its resource identity; a same-id second adapter cannot receive it.
- [ ] The registry remains identity-only: no snapshot tier, revision, health, generation, or payload state appears in this unit.

### Unit 3: Exact resource grant containment and fail-fast grant validation

**Files**: `core/src/authority/state.rs`, `core/src/authority/registry.rs`, `core/src/authority/ingest.rs`, `core/tests/authority_registry.rs`, `core/tests/authority_grant_check.rs`, `core/tests/authority_proptest.rs`

**Story**: `epic-agent-operations-resource-plane-resource-identity-resource-authority-containment`

```rust
// core/src/authority/state.rs
fn same_resource(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    matches!(
        TargetScopeKind::try_from(requested.kind),
        Ok(TargetScopeKind::Resource)
    ) && ResourceIdentity::try_from_scope(grant_scope)
        .ok()
        .zip(ResourceIdentity::try_from_scope(requested).ok())
        .is_some_and(|(grant, request)| grant == request)
}

fn same_adapter(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    matches!(
        (grant_scope.adapter_id.as_ref(), target_adapter_id(requested)),
        (Some(grant_adapter), Some(requested_adapter)) if grant_adapter == requested_adapter
    )
}
```

**Implementation notes**:

- Resource grant validation requires the exact nested tuple and rejects `legacy_audit_resource_id`, partial tuples, and mixed target fields when a `Grant` or `DescendantGrant` is folded. The same parser used by acceptance defines identity completeness. `grant_matches_request` also denies any requested `RESOURCE` scope that fails that parser before considering an adapter/fleet/resource containment branch, so direct authority-port callers cannot use an incomplete nested adapter id.
- Resource containment is exact across all three fields and requires the requested scope itself to be `RESOURCE`; the current behavior where a resource grant can match a session-shaped request carrying an incidental resource scalar is removed.
- Adapter-scope grants intentionally contain resources whose nested adapter id matches. Fleet/authority-domain wildcards retain current authority-domain-bounded behavior. A resource-kind-wide or adapter+kind wildcard is not introduced; if later needed it is a new explicit grant-scope design.
- Property strategies generate independent adapter/kind/id dimensions. Mutation evidence must fail if any one equality check or the requested-kind check is removed.

**Acceptance criteria**:

- [ ] A live exact resource grant authorizes the allowed OperationKind on its exact tuple.
- [ ] Same local id under another adapter, same adapter/id under another kind, another id, missing kind, legacy scalar, and a non-resource requested kind all deny before acceptance.
- [ ] Adapter/fleet/authority-domain grant containment remains explicit and tested; endpoint, expiration, revocation, and OperationKind checks are unchanged.
- [ ] Existing session and control-surface revocation authority tests remain green.

### Unit 4: Integrated acceptance evidence, audit compatibility, and rolling foundation

**Files**: `core/src/acceptance/pipeline.rs`, `core/tests/acceptance_pipeline.rs`, `core/tests/acceptance_proptest.rs`, `core/tests/resource_acceptance.rs` (new), `core/src/authority/operator.rs`, `core/src/storage/audited.rs`, `server/src/service.rs`, `server/tests/grpc_smoke.rs`, `cli/src/commands/diagnostics.ts`, `cli/src/output.ts`, `cli/tests/diagnostics.test.ts`, `docs/PROTOCOL.md`, `docs/SECURITY.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`

**Story**: `epic-agent-operations-resource-plane-resource-identity-integration-conformance`

```rust
// acceptance boundary, before grant evaluation
match TargetScopeKind::try_from(target_scope.kind) {
    Ok(TargetScopeKind::Resource) => {
        ResourceIdentity::try_from_scope(target_scope)
            .map_err(|error| ValidationRejection::validation_failed(error.to_string()))?;
    }
    Ok(TargetScopeKind::Unspecified) | Err(_) => { /* existing rejection */ }
    Ok(_) if target_scope.resource.is_some() => {
        return Err(ValidationRejection::validation_failed(
            "non-resource target carries resource identity",
        ));
    }
    Ok(_) => {}
}
```

**Implementation notes**:

- Preserve acceptance order: structural resource validation → issuer/posture/response validation → grant check → target resolution → deduplicating append. A malformed resource tuple must not call the grant checker, resolver, or storage.
- Successful resource acceptance persists the original canonical typed target. `target_key_for` already encodes the entire deterministic `TargetScope`, so the full tuple scopes idempotency without a second hand-built resource key.
- Update test resolvers to the enum without weakening their call-order assertions. Add interface tests for authorized+registered acceptance, authorized+unknown rejection, malformed-before-grant, tuple-specific dedup, and cross-adapter/kind non-collision.
- The three control-surface principal/endpoint/device audit producers continue using tag 8 under its renamed generated field. Regression tests prove old/new audit records remain queryable and that those audit-only targets cannot resolve or satisfy a resource grant. CLI JSON shows both `legacyAuditResourceId` and nested `resource { adapterId, resourceKind, resourceId }`; `resource=ID` remains the existing audit-only filter. Canonical operational resource filtering uses `adapter=...;resource-kind=...;resource=...` with the same percent-encoding discipline as session identities.
- Roll foundation assertions forward in place: PROTOCOL names the tuple, resolver dispositions, and exact containment; SECURITY names the collision fence; VERIFICATION records implementation/property evidence as specified but leaves the parent epic's resource conformance vectors to the `conformance` feature; GLOSSARY distinguishes local `ResourceId` from full `ResourceIdentity`. Do not claim checked-normative evidence before that sibling lands.

**Acceptance criteria**:

- [ ] End-to-end acceptance proves a resource Operation can pass boundary, grant, and resolution without runtime-session fields and is durably scoped by the complete tuple.
- [ ] Cross-adapter and cross-kind collision cases deny and never append; mutation/property tests catch removal of either fence.
- [ ] Existing runtime-session acceptance and diagnostics-special-resolver tests remain green.
- [ ] Existing control-surface revocation audit target values survive generation, storage, query, and JSON output while remaining non-operational.
- [ ] Rust workspace tests/clippy, TypeScript contract and CLI tests, vector/model metadata checks, and generated drift checks pass.
- [ ] Foundation docs state the intended post-v0.1 resource semantics and honest assurance tier without importing snapshot, manifest, or cockpit scope.

## Implementation Order

1. `epic-agent-operations-resource-plane-resource-identity-typed-resource-identity`
2. After the contract lands, implement `epic-agent-operations-resource-plane-resource-identity-polymorphic-target-resolution` and `epic-agent-operations-resource-plane-resource-identity-resource-authority-containment` against the same parser (one feature owner should normally carry both checkpoints sequentially even though they are dependency-independent).
3. `epic-agent-operations-resource-plane-resource-identity-integration-conformance` after both resolver and authority checkpoints.
4. Advance child stories directly to `done` on green checkpoint evidence; review the integrated feature at effective weight `thorough` (explicit caller policy).

## Simplification

- Reuse `TargetScope`, `TargetScopeKind::Resource`, `TargetResolver`, deterministic target-key encoding, and the server's existing projection lock rather than adding resource-only acceptance, idempotency, or delivery paths.
- Replace the hard-coded binding struct with one enum and remove the diagnostics fake session id/generation.
- Centralize resource tuple parsing and target-adapter extraction so acceptance, resolver, grant validation, and adapter routing cannot carry divergent field lists.
- Preserve tag 8 only because durable audit records and current audit producers are verified consumers; reject it from all operational paths instead of supporting two resource identities.
- Do not create resource snapshots, revision fields, stored events, manifest registries, projection schemas, or UI labels in this feature.

## Testing

- **Interface tests** protect the acceptance ordering and `TargetResolver` seam: malformed resource identities reject before authority/state; exact authorized+registered identities append; unknown identities reject without append.
- **Authority property tests** protect the high-consequence collision fence across independent adapter/kind/id dimensions and include mutation checks for each equality component and requested target kind.
- **Regression tests** protect the full existing session resolver matrix, honest diagnostics authority binding, adapter delivery routing, and control-surface audit decoding/query behavior.
- **Contract checks** protect cross-language generation and deterministic target-key encoding. No test is added merely for getters or enum construction.
- **Deferred evidence**: promoted resource-plane conformance vectors and any formal authority-model promotion stay with `epic-agent-operations-resource-plane-conformance`; this feature records implementation evidence without overstating its tier.

## Risks

- **Registry population gap.** The identity-only registry cannot resolve a resource until `resource-state` durably registers it. This is deliberate fail-closed sequencing, not an ephemeral workaround; if the sibling is delayed, session behavior remains intact and resource Operations remain visibly `target_not_found`.
- **Legacy audit field misuse.** Keeping tag 8 for real durable audit data could invite a new operational caller. Central parsing rejects it in acceptance, grant ingestion, and resolution; tests treat any successful operational use as a security regression.
- **Nested adapter routing omissions.** Any delivery or authenticated-ingress call site that reads only top-level `adapter_id` would drop or misroute resource work. One helper plus server delivery/Observation regression tests makes the branch explicit.
- **Adapter kind instability.** A kind rename changes identity. The resource-state/manifest contracts must require stable declared kind identifiers and treat a deliberate rename as resource replacement; this feature does not guess replacement semantics.
- **Assurance staging.** Exact grant containment is a normative security rule but its promoted conformance evidence lands in the epic's closing feature. Implementation/property evidence is required now, and docs must label the assurance honestly until the closing feature promotes vectors.

## Extension pressure classification

- **Committed post-v0.1 direction:** operational resource identity is `(adapter_id, resource_kind, resource_id)`; ordinary target resolution is target-kind-polymorphic; a resource grant contains only the exact tuple; adapter/fleet/domain scopes remain the explicit wider grants.
- **Reserved seams:** resource replacement/revision/tombstones, capability-declared admitted kind sets, resource-kind-wide grants, cross-domain resource references, and promoted formal/conformance evidence. Their named sibling features own promotion.
- **Explicitly rejected for this arc:** fabricating runtime-session ids/generations for resources, treating a local resource id as globally unique, making adapter-specific resource kinds a core enum, or creating a parallel resource acceptance pipeline.

## Other agent review

- Invoked because: target resolution and grant containment are authority/security-critical cross-boundary contracts.
- Fixed/active blockers: the design itself closes the known cross-adapter/kind collision, fake diagnostics session, mixed-field ambiguity, and audit-overload risks.
- Parked: none from this pass.
- Rejected: none.
- Skipped/degraded: independent design-time advisory dispatch was unavailable in this worker tool surface; direct source verification, explicit alternatives, and the pre-mortem above were used. The caller-specified `thorough` implementation review remains mandatory and is not degraded.

## Implementation summary

All four child checkpoints completed directly to `done` in dependency order:

1. typed generated resource identity and canonical domain parser;
2. target-kind-polymorphic binding, identity-only resource registry, composite resolver, and canonical adapter routing;
3. exact resource grant containment and fail-fast durable grant validation;
4. integrated acceptance/audit/CLI compatibility evidence and rolling-foundation updates.

One coherent feature owner carried the cross-cutting contract refactor because the generated schema, parser, resolver, authority, acceptance, adapter routing, and audit compatibility paths share one identity invariant and overlapping write set. Splitting by story would have increased integration risk. The direct host ran `openai-codex/gpt-5.6-sol` at high reasoning; nested implementation dispatch was unavailable in this delegated tool surface. No sibling resource-plane feature was touched.

## Integrated verification

- `cargo test --workspace` and a post-commit `cargo test --workspace --quiet` — all Rust tests and doc tests passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cd cli && npm test` — 37 passed.
- `cd contracts/ts && npm run build && npm run check:drift` — passed.
- vector, model-metadata, and presentation conformance scripts — passed.
- Test-integrity check: no test was deleted, skipped, weakened, or rewritten to accept production output; new tests independently vary adapter/kind/id and assert ordering before stateful ports.

The feature is review-ready. Effective review weight is `thorough` from the explicit autopilot caller override; convergence requires a clean fresh-context pass after any receiver-confirmed material fix.

## Review findings — pass 1 (2026-08-04)

**Reviewer path**: same-harness fresh-context `openai-codex/gpt-5.6-sol` at xhigh; read-only Pi endpoint, not cross-model.

**Receiver-confirmed blockers**:

1. Resource status/result Observations authenticated only the adapter and did not compare their target tuple with the correlated accepted Operation. A same-adapter cross-kind/id Observation could terminalize the wrong resource command. Fix requires exact canonical target binding before append/transition plus authenticated-ingress regressions.
2. Tag-8 compatibility evidence decoded the old scope bytes and queried new producer records, but did not prove an old-wire durable audit record remained indexed/filterable or that nested resource filters hit stored audit data. This is explicit acceptance evidence and must land now.

**Rejected for this cycle**:

- Adding authority-domain state to `ResourceRegistry`/`TargetRegistry`. Production constructs one projection per validated authority-domain log, v0.1 has one configured domain, the designed registry signature is identity-only, and the same latent trait-argument issue already exists for sessions in `.work/backlog/backlog-sessions-authority-domain-isolation.md`. This feature neither creates nor should partially solve that cross-cutting seam.

**Closure policy**: `thorough`; after fixes and integrated verification, return to `review` for another fresh-context pass. No lower-risk finding required a new backlog item.

## Review fix verification — pass 1

- `CommandSnapshot` now carries the originating target; status/result Observation ingestion requires exact target equality before any append or transition. Core and authenticated server tests prove same-adapter cross-kind and cross-id resource results reject without durable evidence or command mutation.
- Durable audit tests decode a pre-rename tag-8 scope byte fixture, append/index it, query it through the current target key, and repeat the stored-data filter for nested operational resource identity. CLI JSON tests cover both legacy and nested target presentations.
- Corrected snapshot verified with `cargo test --workspace --quiet`, clippy `-D warnings`, all 37 CLI tests, generated-contract build/drift, and vector/model/presentation checks.

The corrected feature returns to `review`. Thorough convergence requires pass 2 to yield no receiver-confirmed material current-cycle blocker.

## Review findings — pass 2 (2026-08-04)

**Reviewer path**: same-harness fresh-context `openai-codex/gpt-5.6-sol` at xhigh; read-only Pi endpoint.

**Verified pass-1 fixes**: exact Observation target binding and durable audit-filter compatibility evidence are correct and regression-covered.

**Receiver-confirmed blocker**:

- Abnormal adapter-disconnect reconciliation selected running commands through only top-level `TargetScope.adapter_id`. Canonical resource identity nests the adapter, so a running resource command could remain indefinitely `running` instead of terminalizing `failed(execution_outcome_unknown)`. Fix the selector through `target_adapter_id` and prove current-adapter resource failure plus other-adapter/malformed inertness.

**Closure policy**: `thorough`; fix, verify the integrated snapshot, and run pass 3.

## Review fix verification — pass 2

- Running-command disconnect reconciliation now selects adapter/session/resource targets through the canonical `target_adapter_id` helper. A canonical resource for the lost adapter terminalizes `failed(execution_outcome_unknown)`; another adapter and a malformed nested tuple remain inert.
- Added a focused core selector regression and extended the real adapter-stream-loss integration to deliver, acknowledge, run, disconnect, and rebuild a resource command alongside the existing session cases.
- Corrected snapshot verified with workspace tests, clippy `-D warnings`, 37 CLI tests, contract build/drift, and vector/model/presentation checks.

The feature returns to `review` for thorough convergence pass 3.

## Review (2026-08-04)

**Verdict**: Approve

**Blockers**: none remaining. Pass 1 fixed exact Observation-to-command resource target binding and durable legacy/nested audit-filter evidence. Pass 2 fixed resource running-command reconciliation on adapter disconnect. Pass 3 verified all three fixes and found no material current-cycle blocker.

**Important**: none. The authority-domain registry proposal was rejected for this feature because production projection composition is already domain-log-scoped, v0.1 is single-domain, the identity-only registry shape is designed, and the cross-cutting trait issue is already tracked at `.work/backlog/backlog-sessions-authority-domain-isolation.md`.

**Nits**: none.

**Rejected**: adding authority-domain state to `ResourceRegistry`/`TargetRegistry` in this feature, for the bounded rationale above.

**Notes**: Substrate feature review at explicit `thorough` weight. Three same-harness fresh-context passes used `openai-codex/gpt-5.6-sol` at xhigh through read-only ephemeral Pi endpoints; these were fresh-context but not cross-model because the host is also OpenAI lineage. Convergence sequence was review → receiver adjudication → fix → full verification → fresh review, repeated until pass 3 returned `ready` with no findings. Applicable correctness, security, generated-contract, compatibility, data/index, lifecycle, test-integrity, CLI, and foundation lenses were covered. Product UI/accessibility review was skipped because this feature has no UI surface; presentation conformance still passed.
