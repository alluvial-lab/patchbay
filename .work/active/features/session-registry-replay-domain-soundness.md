---
id: session-registry-replay-domain-soundness
kind: feature
stage: implementing
tags: [protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
---

# Session registry/replay/domain soundness

## Brief
Close the registry/replay/domain soundness gaps split out of `sessions-soundness-coverage` (currency verified 2026-08-09). Absorbs:

- `backlog-sessions-authority-domain-isolation` — **OPEN**: `SessionRegistry` has no owning domain (`registry.rs:50-55`); `TargetResolver::resolve` ignores `_authority_domain_id` (`resolver.rs:15-24`); `current_session` takes no domain arg (`ingest.rs:88-95`).
- `backlog-sessions-idempotency-and-concurrency` — **PARTIAL** (replay-equality half): production adapter ingress now serializes through the shared decision gate + rebuilds before/after report ingest (`adapter_service.rs:753-829`); but registry redelivery is content-blind (dup returns `Ok` by key, `registry.rs:329-334`) and state mutations no-op on any `event_lsn <= last_lsn`. Production serialization ≠ replay equality.
- `backlog-sessions-test-coverage-gaps` — **PARTIAL**: resolver enforces `RuntimeSession` kind + some malformed cases exist, but replay tests stay happy-path, acceptance uses `TestTargetResolver`, and the proptest fixes all reports to one adapter/session.

## Direction
Bind each `SessionRegistry` to an `AuthorityDomainId` at construction; validate on lookup/ingest; reject cross-domain (forward-compat for the `(authority_domain_id, LSN)` federation seam). For replay equality: compare event identity/payload on redelivery, not just key+LSN; either serialize the warm read-decide-append or make it append-then-replay. Coverage: the acceptance↔sessions integration test (highest value), table-driven malformed-event tests, and a multi-identity proptest (per-identity monotonicity, tombstone retention, no cross-session interference). **Production decision-gate serialization is a composition-root invariant tested independently — do not advertise it as core writer safety** (a future composition root can bypass the server gate; Fail Fast).

## Foundation references
- `docs/PROTOCOL.md` — authority-domain-scoped target resolution; `(authority_domain_id, LSN)` extension seam
- Code: `core/src/session/registry.rs`, `core/src/session/resolver.rs`, `core/src/session/ingest.rs`, `server/src/adapter_service.rs`

## Scope boundaries

This feature enforces the existing session/domain/replay contract; it does not create new protocol semantics.

- Bind only the runtime-session projection. `ResourceRegistry` domain ownership is a separate resource-plane concern; this feature must not quietly widen into a composite-registry redesign.
- Apply exact redelivery equality to event kinds that mutate `SessionRegistry`: `SessionState` and `SecurityLockdown`. Known sibling projection events remain ignored; complete-log framing and gap-free-prefix validation belong to `replay-integrity-prefix-discipline`.
- Make the core report writer append then fold through the supplied mutable session projection before returning. This closes caller-managed warm-state gaps for one projection instance; it does not make multiple independently stale projections globally concurrency-safe.
- Retain the server's shared `CoreDecisionGate` plus rebuild-before/after report ingestion. That composition-root invariant is the production cross-request serialization guarantee and remains independently tested.
- Adapter source ordering is out of scope. A later-arriving adapter report can still be older in source time even when its core LSN is newer; `adapter-report-source-ordering` owns that wire-level concern.
- No `.proto`, storage schema, model, conformance-vector, foundation-doc, or UI change is required. The foundation already states one authority-domain log per projection, `(authority_domain_id, LSN)` event identity, and idempotent replay; implementation is the lagging artifact.

## Design decisions

- **Require an authority domain at construction**: Change `SessionRegistry::new` to accept an owned `AuthorityDomainId` and return `Result`, rejecting an empty value. Remove `Default`; a domainless session projection is an invalid state, not a useful convenience.
- **Compare the durable envelope, not decoded semantic subsets**: Retain each successfully applied projection-owned `StoredEventPayload` by its domain-local LSN. Exact raw envelope equality is the redelivery condition; a semantically similar re-encoding or changed event kind is not the same durable record.
- **Make event identity the first duplicate key**: The registry's bound domain plus LSN identifies one owned event. An exact identity/payload replay returns before mutation. Same identity with different content, a new owned event below the applied high-water mark, or the same logical registration at a different LSN is corrupt history.
- **Record replay evidence only after successful application**: Decode and validate the complete mutation before changing its record, then add the payload to the applied-event ledger after the fold succeeds. Malformed/conflicting input leaves both projection state and equality evidence unchanged.
- **Remove content-blind local shortcuts**: Once exact event redelivery is centralized, delete registration's key-only success, generation-bump's partial tombstone equality, `event_lsn <= last_authoritative_lsn` no-ops, and pre-supersession LSN-only inertness. A later event aimed at a tombstone is corruption; only an already-recorded exact event is redelivery.
- **Carry the domain through the consumer-owned ports**: Add `authority_domain_id` to `SessionLookup::current_session` and return a typed `Result`. `ingest_session_report`, adapter-stale derivation, and `TargetResolver` validate the requested/report domain against the registry before stateful work or append.
- **Use append-then-fold for every report outcome**: Generalize the existing multi-delta `append_and_warm` helper and use it for registration, generation bump, each single delta, and every multi-delta prefix. Success means the supplied hot projection includes the committed event; a post-commit fold error is fail-closed and requires rebuild before reuse.
- **Keep one target-resolution implementation**: Remove the unused inherent `SessionRegistry::resolve(TargetScope) -> Option<_>` and retain the acceptance-owned `TargetResolver` adapter as the single runtime-session resolution boundary.
- **Treat the acceptance seam as the primary integration evidence**: Replace the coverage gap, not every test double: add focused real-`SessionRegistry` cases to `acceptance_pipeline`, while unrelated acceptance tests keep their narrow resolver double.
- **Keep assurance wording honest**: Tests provide implementation evidence for already stated `IdempotentLogReplay`, `SessionIdentityTuple`, and domain isolation obligations. They do not promote a formal property, vector, or checked-normative tier.
- **Execution posture**: Direct-read only. The source/test surface was enumerable by `SessionRegistry`, `SessionLookup`, `TargetResolver`, and constructor searches; nested exploration was prohibited by the delegated endpoint contract.
- **Review policy**: Effective `review_weight` is `thorough`, source: explicit operator selection. Pass it unchanged to feature and final completion review.

## Codebase mapping

Direct reading covered every session source module; registry, ingest, replay, resolver, property, malformed-event, and acceptance tests; the composite `TargetRegistry`; server projection construction; adapter-service report ingress; and the existing concurrent conflicting-report regression. Constructor migration is bounded to the 37 current `SessionRegistry::new()` call sites reported by `rg`; no domainless `Default` call site exists.

`replay-integrity-prefix-discipline` overlaps `core/src/session/{registry,replay}.rs` but owns a different invariant: exact successor validation for newly read complete prefixes. This feature owns equality when an already applied session/security event is presented again, including direct warm-path folds that do not pass through a cold-replay reader. The work remains dependency-independent as the sibling design specifies; implementation should preserve/use the shared prefix helper if it has landed rather than recreate it.

## UI fallback

No UI surface. The visible session identity and state vocabulary are unchanged, so no mockup is required.

## Extension pressure classification

- **Committed v0.1.0 behavior enforced**: each session projection is bound to its configured authority domain; runtime-session resolution/report ingestion cannot cross that boundary; event identity remains `(authority_domain_id, LSN)`; exact replay means identical durable payload; and successful report writes immediately update the supplied projection.
- **Reserved seams preserved**: future multiple authority domains/federation use separate bound registries and the existing domain-qualified event key. Snapshot/compaction work may later replace the in-memory equality ledger with a checkpoint-backed exact mechanism, but it must preserve equality rather than weaken it to LSN-only or hash-only acceptance.
- **Explicitly rejected for this feature**: a multi-domain `SessionRegistry`, bare-LSN identity across domains, trusting labels as routing scope, a new global core mutex/storage CAS hidden in the domain writer, treating any old LSN as redelivery, and advertising the core writer as safe across independently stale projections.
- **Parked-idea pressure test**: multi-human/federated authority remains additive because the authority-domain demarcator is explicit at construction and lookup. Desktop, agent-mesh, and customizable-skin ideas are unaffected.

## Architectural choice

### Option A — bound registry + exact owned-event ledger + append-then-fold (chosen)

Bind the projection at construction, retain the exact raw envelope for every successfully applied session/security event keyed by LSN, short-circuit only exact redelivery, and make every writer append feed that same fold before returning. This directly closes all three observed gaps without adding persistence, transport, or wire concepts. It costs memory proportional to projection-owned history, which is acceptable while v0.1.0 already replays the full log and has no checkpoint compaction.

### Option B — infer duplicate equality from current records and tombstones

Retain source fields on `SessionRecord`/`SessionTombstone` and compare a duplicate mutation with projected state. This looks smaller but cannot reconstruct the payload of old connectivity, activity, relabel, model, or lockdown events after later changes. It would keep scattered per-mutation duplicate rules and reproduce the content-blind gap.

### Option C — rebuild from storage after every append and rely on the server gate

A full rebuild makes the latest projection authoritative and the existing server already does this around adapter ingress. Making it the core writer contract would add a storage read to every delta, conflate a composition-root concurrency policy with a domain fold, and still would not define what direct `observe` redelivery means.

**Choice**: Option A. It follows the durable-log projection and fail-fast boundary patterns, makes exact equality local to the projection that needs it, and preserves the server gate as a separately named invariant.

## Trickiest unit first

The riskiest unit is the registry's replay discriminator. It must accept a complete exact prefix a second time even after later generations/tombstones, reject changed bytes at an already seen identity, reject an unseen old owned event rather than silently skipping it, and avoid recording a malformed event as applied. Security-lockdown events also mutate the session projection and therefore must use the same rule. This unit lands before port/writer migration so every later warm fold has one trustworthy definition of idempotence.

## Implementation Units

### Unit 1: Domain-bound registry and exact replay equality

**Files**: `core/src/session/registry.rs`, `core/src/session/mod.rs`

**Story**: `session-registry-replay-domain-soundness-bound-registry-contract`

```rust
// core/src/session/registry.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRegistry {
    authority_domain_id: AuthorityDomainId,
    applied_events: BTreeMap<u64, StoredEventPayload>,
    sessions: HashMap<SessionLiveKey, SessionRecord>,
    tombstones: HashMap<SessionTombstoneKey, SessionTombstone>,
    lockdown_active: bool,
}

impl SessionRegistry {
    pub fn new(
        authority_domain_id: AuthorityDomainId,
    ) -> Result<Self, SessionError>;

    pub fn authority_domain_id(&self) -> &AuthorityDomainId;

    pub(crate) fn require_authority_domain(
        &self,
        actual: &AuthorityDomainId,
    ) -> Result<(), SessionError>;

    fn classify_redelivery(
        &self,
        event_lsn: u64,
        payload: &StoredEventPayload,
    ) -> Result<ReplayDisposition, SessionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayDisposition {
    New,
    Exact,
}

// core/src/session/mod.rs
#[error("session registry authority_domain_id must not be empty")]
EmptyAuthorityDomain,

#[error("session authority domain mismatch: expected {expected:?}, got {actual:?}")]
AuthorityDomainMismatch {
    expected: AuthorityDomainId,
    actual: AuthorityDomainId,
},
```

**Implementation notes**:

- Parse the generated `StoredEventKind` first. For `SessionState` and `SecurityLockdown`, validate the outer event identity, require equality with the bound domain, then classify `(LSN, StoredEventPayload)` before decoding the mutation. Continue to ignore known concrete sibling kinds; the full-log replay boundary validates their framing.
- `applied_events` contains only event kinds that affect this projection. Store the full generated envelope (`kind` and payload bytes), not a decoded subset or hash. A B-tree gives an owned-event high-water mark without a second cursor.
- If an LSN is present with equal content, return `Ok(())` before any mutation. If present with different content, return `CorruptLog`. If absent but below the greatest applied owned-event LSN, reject as an unseen out-of-order owned event.
- Insert into `applied_events` only after the mutation handler returns successfully. Keep every handler validate-before-mutate so table-driven rejected cases leave a byte-for-byte equal registry.
- Change duplicate registration at a later LSN to `CorruptLog`; an exact original registration has already returned through the ledger. Apply the same simplification to generation bumps and tombstoned-generation events.
- Delete the four state/metadata `event_lsn <= last_authoritative_lsn` branches. `last_authoritative_lsn` remains the public record revision, not the replay-equality oracle.
- Reject an empty constructor domain as `SessionError::EmptyAuthorityDomain`; add no dummy/default domain.

**Acceptance criteria**:

- [ ] A registry cannot be constructed without a non-empty `AuthorityDomainId`, and exposes the exact bound id for composition/testing.
- [ ] Exact redelivery of every owned event kind is inert even after later events; conflicting same-identity payload or unseen older owned event is corrupt and leaves state unchanged.
- [ ] A duplicate registration or generation fact at a different LSN is not mistaken for redelivery.
- [ ] Cross-domain session/security events cannot mutate the registry; known sibling events remain projection no-ops.
- [ ] Record revisions, tombstone retention, state adjacency, and lockdown clamping remain unchanged for valid new events.

### Unit 2: Domain-aware lookup/resolution and append-then-fold ingress

**Files**: `core/src/session/ingest.rs`, `core/src/session/resolver.rs`, `core/src/session/replay.rs`, `core/src/target.rs`, `server/src/state.rs`

**Story**: `session-registry-replay-domain-soundness-bound-registry-contract`

```rust
// core/src/session/ingest.rs
pub trait SessionLookup: Send + Sync {
    fn current_session(
        &self,
        authority_domain_id: &AuthorityDomainId,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
    ) -> impl Future<Output = Result<Option<SessionRecord>, SessionError>> + Send;
}

async fn append_and_apply<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    event: SessionStateEvent,
) -> Result<EventId, SessionError>
where
    S: Storage,
    L: SessionProjection;

// core/src/session/replay.rs
let mut registry = SessionRegistry::new(authority_domain_id.clone())?;
```

**Implementation notes**:

- `SessionRegistry`'s `SessionLookup` implementation calls `require_authority_domain` before reading. `ingest_session_report` retains empty-field validation, then passes `report.authority_domain_id` into every initial and refreshed lookup; a mismatch fails before append.
- Route registration, generation bump, single connectivity/activity/model/relabel delta, and multi-delta prefixes through `append_and_apply`. The helper appends, validates the returned domain-qualified event id, then observes the exact committed envelope. Rename/remove the caller-managed warm-path comments.
- Preserve partial-failure semantics: each committed multi-delta prefix is visible; an append failure does not fold the candidate; a fold error after commit propagates and marks the projection unfit until rebuilt.
- Validate the explicit authority-domain argument to `adapter_stale_events`/`mark_adapter_sessions_stale` against the registry before deriving or appending changes.
- In `TargetResolver for SessionRegistry`, reject a mismatched resolver domain before target parsing/lookup and map it to the existing `TargetNotFound` port result. Remove the inherent Option-returning resolver and update its tests/callers to the trait boundary.
- `rebuild_from_log` constructs the registry with the requested domain and keeps/use the shared complete-prefix validator when the sibling replay-integrity feature lands. Equality remains in `observe`, not the cold reader.
- Remove `Default` from `TargetRegistry`; migrate `ProjectionState` and all direct test construction to pass the configured/test domain. Do not add a domainless compatibility constructor.
- Keep `server/src/adapter_service.rs`'s decision-gate and rebuild-before/after flow. Core warming is not a reason to delete the production serialization/reconciliation defense.

**Acceptance criteria**:

- [ ] Report ingest, current-session lookup, adapter-stale derivation, and runtime-session target resolution cannot use a registry bound to another domain.
- [ ] Every successful `IngestResult` leaves the supplied registry at the returned event's state without caller-managed observation.
- [ ] Every appended event is folded only after durability; partial append failures expose exactly their committed prefix.
- [ ] Cold rebuild and hot append-then-fold produce equal bound registries, including their replay-equality evidence.
- [ ] The server aggregate and adapter-service construction compile with explicit domains, and the shared decision gate behavior is unchanged.

### Unit 3: Boundary, malformed-replay, and multi-identity evidence

**Files**: `core/tests/acceptance_pipeline.rs`, `core/tests/sessions_registry.rs`, `core/tests/sessions_ingest.rs`, `core/tests/sessions_replay_resolver.rs`, `core/tests/sessions_proptest.rs`, `core/tests/conformance_vectors.rs`, `core/tests/resource_acceptance.rs`, `core/tests/resource_resolver.rs`, `server/src/adapter_service/tests.rs`

**Story**: `session-registry-replay-domain-soundness-integration-evidence`

```rust
// Collision-heavy property-test identity, deliberately independent of the
// registry's private key type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OracleSessionKey {
    adapter: String,
    deployment_scope: String,
    runtime_session: String,
}

struct OracleSessionState {
    live_generation: u64,
    tombstoned_generations: BTreeSet<u64>,
}
```

**Implementation notes**:

- Add focused `acceptance_pipeline` cases using a real populated `SessionRegistry` as `TargetResolver`: one same-domain Operation reaches acceptance, while a registry-domain mismatch returns `target_not_found` with no command append despite an otherwise-authorizing grant double.
- Turn malformed session-event coverage into a table whose cases include missing/empty/wrong outer domain, missing LSN, inner/outer domain mismatch, missing mutation and required identity/state fields, and unknown generated state values. Snapshot the registry before each rejection and assert exact non-mutation.
- Add replay-equality regressions for an exact full prefix twice, same LSN with mutated kind/bytes, unseen older owned LSN, same registration at a new LSN, and exact pre-supersession event after a generation bump.
- Update ingest tests so registration and each single-delta result are asserted in the hot registry immediately. Add cross-domain report/adapter-stale cases that inspect both possible domain logs and prove zero append.
- Extend/add a 100-case property sequence across a small collision matrix: same runtime under different adapters, same adapter/runtime under different scopes, and same adapter/scope under different runtimes. The independent oracle updates only the addressed key, verifies other keys before/after each report, checks generation non-decrease and all prior tombstones, then compares hot and rebuilt registries.
- Add an explicit faulty-key mutation witness (each identity dimension omitted in turn) so the no-cross-session-interference test cannot pass by sharing the production key/equality helper.
- Migrate constructor-only call sites in conformance/resource tests without manufacturing tests for the constructor syntax.
- Keep `concurrent_conflicting_model_reports_leave_a_replayable_log` as the separate server composition-root race test. It must remain green, but its comments/evidence must not call `ingest_session_report` globally concurrent-safe.

**Acceptance criteria**:

- [ ] The real acceptance pipeline admits only a live session in the requested registry domain and appends no command for cross-domain resolution.
- [ ] Malformed/conflicting owned events fail without any projection mutation; exact redelivery succeeds without mutation.
- [ ] Single-delta and multi-delta writer paths are immediately warm and replay-identical.
- [ ] Multi-identity generated sequences preserve per-key monotonicity, tombstones, and isolation across all canonical identity dimensions.
- [ ] The independent identity-key mutant is killed; evidence is described as implementation-checked only.
- [ ] Existing production gate concurrency evidence, conformance vectors, resource composition, and workspace tests stay green.

## Implementation Order

1. `session-registry-replay-domain-soundness-bound-registry-contract` — land Unit 1's bound registry and equality discriminator, then Unit 2's domain-aware ports, append-then-fold writer, resolver consolidation, and constructor migration.
2. `session-registry-replay-domain-soundness-integration-evidence` — add Unit 3's real acceptance seam, malformed/conflicting replay table, warm-path regressions, multi-identity oracle/mutant, and run the server gate regression.
3. Run focused plus workspace verification, advance child checkpoints according to their evidence policy, and review the integrated feature at `thorough` weight until a pass yields no receiver-confirmed material current-cycle blockers.

One feature-owning implementation worker should carry both checkpoints. The evidence deliberately overlaps the registry/writer files, so splitting write ownership would add conflict and make mutation evidence easier to detach from the behavior it protects.

## Simplification

- Delete the duplicate inherent session resolver; retain the acceptance-owned port implementation.
- Delete `SessionRegistry::default`, registration's key-only no-op, per-mutation LSN-only no-ops, tombstone LSN-only replay handling, and caller-managed single-delta warming.
- Reuse one `append_and_apply` helper and one owned-event equality ledger across every session/security mutation kind.
- Reuse generated `AuthorityDomainId`, `StoredEventPayload`, `StoredEventKind`, and existing `SessionError` corruption families. Add no schema, digest dependency, storage table, global lock, or replay framework.
- Do not duplicate every generic acceptance test with a real registry. Two integration cases protect the seam; narrow doubles remain useful elsewhere.
- Retain `last_authoritative_lsn` as the public session revision. Do not overload it as a duplicate detector again.

## Testing

- **Registry interface table** (`core/tests/sessions_registry.rs`): protects constructor/domain binding, exact envelope equality, corruption classification, validate-before-mutate, and removal of LSN-only skips.
- **Writer tests** (`core/tests/sessions_ingest.rs`): protect append-before-fold, immediate warming, domain mismatch before append, and committed-prefix behavior after partial failure.
- **Acceptance integration** (`core/tests/acceptance_pipeline.rs`): protects the actual acceptance→session resolver seam and kills the ignored-domain implementation.
- **Replay/resolver tests** (`core/tests/sessions_replay_resolver.rs`): protect same-domain rebuild/binding and cross-domain fail-closed behavior while preserving offline/failed target resolution.
- **Property/mutation evidence** (`core/tests/sessions_proptest.rs`): protects per-identity monotonicity, tombstone retention, no cross-session interference, and hot/rebuilt equality without reusing production key logic.
- **Production composition test** (`server/src/adapter_service/tests.rs`): protects shared decision-gate serialization separately from core-writer semantics.
- **No formal/vector promotion**: run model/vector drift checks, but do not edit promotion metadata or claim a stronger assurance tier.

Implementation verification commands:

```bash
cargo fmt --all -- --check
cargo test -p patchbay-core --test sessions_registry
cargo test -p patchbay-core --test sessions_ingest
cargo test -p patchbay-core --test sessions_replay_resolver
cargo test -p patchbay-core --test sessions_proptest
cargo test -p patchbay-core --test acceptance_pipeline
cargo test -p patchbay-core-server concurrent_conflicting_model_reports_leave_a_replayable_log
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node contracts/scripts/check-models.mjs
node contracts/scripts/check-vectors.mjs
```

## Risks

- **Applied-event memory grows with owned session/security history**: exact old-prefix redelivery cannot be checked from current records alone. v0.1.0 already has full-log recovery with no checkpoint compaction, so retain exact bytes now and make checkpoint-backed pruning a future storage/snapshot design; do not silently substitute probabilistic hash equality.
- **Semantic-equality temptation**: two decoded messages can mean the same thing but be different durable records. Compare the exact stored envelope so redelivery means the same committed event, not a normalization policy.
- **Post-commit fold failure**: append success followed by fold error leaves durable history ahead of the hot registry. Propagate the error and require rebuild-before-reuse; never retry from the stale projection as if nothing committed.
- **Domain mismatch collapse at acceptance**: the generic resolver port exposes `TargetNotFound`, so callers see canonical pre-acceptance `target_not_found`; internal lookup/ingest keeps the typed domain mismatch for diagnostics. Tests lock both layers.
- **Sibling replay implementation overlap**: `replay-integrity-prefix-discipline` may edit the same files. Preserve its shared new-prefix validator and layer exact redelivery inside `SessionRegistry::observe`; do not overwrite or duplicate the sibling rule.
- **False concurrency claim**: append-then-fold serializes only use of one `&mut SessionProjection`. Independent stale projections can still race unless the composition root coordinates them. Keep the server decision gate and its race test explicit.
- **Property oracle coupling**: deriving expected identity from production helpers would make cross-session interference self-defining. Use a test-owned tuple and explicit dimension-omission mutants.

Fallback if exact-payload retention proves materially too costly under measured use: scope a checkpoint/compaction mechanism that persists exact applied-event equality evidence through a typed session checkpoint and prunes only a covered prefix. Do not fall back within this feature to LSN-only acceptance, a collision-bearing digest as proof of equality, or a storage read inside every direct fold.

## Other agent review

- **Invoked because**: authority-domain isolation and replay equality are protocol/security-adjacent contracts spanning registry, writer, acceptance, and production composition seams.
- **Skipped/degraded**: the delegated endpoint contract explicitly prohibited nested subagents and peeragent, so no independent design-time pass ran. This is non-blocking under Part IV; the design instead uses direct exhaustive call-site mapping and explicit mutation witnesses.
- **Fixed/active blockers**: none. The review-vetted 2026-08-09 body settles the product direction; remaining choices were resolved toward the smallest exact, fail-closed design.
- **Parked**: checkpoint-backed equality-ledger pruning only if measured history cost justifies it; no backlog item was written because this delegated task forbids backlog changes and no present blocker exists.
- **Rejected**: record-derived duplicate fingerprints, storage-only rebuilding, global core locking, domainless compatibility construction, and claims that production gate serialization proves core writer safety.
- **Effective completion policy**: `review_weight: thorough`, source: explicit operator selection; pass unchanged to feature and final completion review.
