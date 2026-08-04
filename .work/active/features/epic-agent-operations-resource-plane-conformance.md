---
id: epic-agent-operations-resource-plane-conformance
kind: feature
stage: implementing
tags: [foundation, verification]
parent: epic-agent-operations-resource-plane
depends_on: [epic-agent-operations-resource-plane-resource-identity, epic-agent-operations-resource-plane-resource-state, epic-agent-operations-resource-plane-capability-manifest, epic-agent-operations-resource-plane-cockpit-composition]
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-08-04
---

# Resource-plane conformance evidence

## Brief

Prove, via executable conformance vectors and property tests, that a resource
adapter cannot bypass Patchbay authority, durability, or stale-state rules.
This is the v1 adapter-boundary evidence surface for the operational-resource
shape — the resource-plane analogue of the session-shape conformance the
public-product-contract arc already requires.

Coverage includes: a resource Operation is grant-gated and authority-checked
like a session Operation; resource Observations are source-authenticated and
cannot fabricate authority; resource snapshot/reconnect honors the
completeness tier a resource declares (partial/none degrades honestly);
stale/offline resource state never renders as live; cross-adapter resource-ID
collision is fenced; and a resource adapter cannot inject core-only state.

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: closes the arc — depends on identity, state, manifest, and cockpit composition. Feeds the parent `epic-public-product-contract` adapter-portability proof.

## Simplification opportunity

- Extend the existing conformance-vector and property-test machinery rather than a parallel resource-only harness.

## Foundation references

- `docs/VERIFICATION.md` — conformance vectors, property-graded baseline
- `docs/PROTOCOL.md` — authority, snapshots, stale-state rules
- `contracts/vectors/` — existing vector corpus to extend

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`
- No UI; verification artifacts.

## Design decisions

- **The existing vector corpus becomes executable in place.** `contracts/scripts/check-vectors.mjs` currently validates JSON metadata and regenerates traceability but does not run any vector against implementation code. Add `implementation_checks` to the existing envelope and make that same script dispatch generic core/server/web runners. Do not create a resource-vector directory, resource-only manifest, or second traceability table.
- **Promotion requires both static and running evidence.** Every promoted vector must name a registered property, pass a property-specific expected-outcome checker, and execute at least one implementation check. Static JSON consistency prevents contradictory expected outcomes; the package runner proves the expected example against real code. Neither substitutes for the other.
- **Existing session-shaped vectors are extended where their property already applies by refinement.** `command-acceptance.json`, `failure-missing-grant.json`, and `snapshot-reconciliation.json` gain resource cases rather than receiving resource-named forks. Dedicated new vectors exist only for resource-specific source, completeness, presentation, collision, and core-state-injection invariants.
- **No formal model is added in this feature.** The deliverable is promoted executable examples plus implementation property evidence. New resource property ids remain stated-normative/implementation-checked, and existing draft-model properties retain their current tier. A promoted vector does not make a draft/unmodeled property checked-normative or release-verified.
- **Observation source authentication means verified channel binding, not payload authority.** The vectors require a current authenticated adapter attachment and an exact owned target; unauthenticated, stale-token, and cross-adapter attempts append nothing. A payload `sender` claim may remain evidence about an adapter-reported actor, but it cannot create a Grant/Operation, select another target, or become a `RESOURCE_STATE` source.
- **Only typed reports may request resource mutations; only the core emits durable resource state.** An adapter can submit `ResourceReport` evidence containing report mutations, but it cannot submit `ResourceStateEvent`, choose `StoredEventKind::RESOURCE_STATE`, assign a core LSN/revision/domain, or use an opaque generic Observation to mutate `ResourceRegistry`.
- **Completeness properties use an independent reference model.** The property oracle consumes raw generated report mode/tier/listed identities/cache presence/generation and implements the prose truth table directly. It must not call `normalize_report`, `stale_change`, `ResourceRegistry::observe`, or presentation helpers whose behavior it judges.
- **Stale/offline coverage crosses the real seam.** Adapter disconnect is adapter-lifecycle offline evidence; the core must degrade a cached resource to stale/unknown. The cockpit property then proves that resource freshness, reconciliation, and tombstone state dominate adapter-owned health. No resource connectivity enum is invented.
- **Autopilot rationale.** These choices are the least irreversible path consistent with the foundation: one corpus, one checker/traceability source, existing Rust proptest and TypeScript fast-check styles, no new wire state, and honest assurance tiers. No strategic question or contradictory state remains.

## Codebase mapping

Direct reading covered the complete foundation authority/reconnect/security
contract, all four done resource-plane sibling designs and child checkpoints,
the vector corpus/README/checker, Rust acceptance/authority/Observation/resource
normalization/fold/replay/resolver paths, authenticated server adapter ingress,
resource snapshot materialization, and cockpit resource fold/reconciliation/
rendering tests. The important discovery is that the current vector script is a
metadata/traceability checker only; executable behavior today lives in Rust
`proptest` tests and TypeScript `fast-check` tests without a vector bridge. This
feature is cross-package but its unknowns are now bounded, so direct reading was
used rather than exploratory fan-out. Design-time independent advisory review
would be justified by the safety-claiming evidence surface, but the delegated
worker has no subagent dispatch; the explicit pre-mortem, mutation design, and
caller-mandated `thorough` deep review are the available scrutiny paths.

## Architectural choice

### Options considered

1. **Extend the existing vector envelope/checker with package runner bindings (chosen).** Add optional `implementation_checks` to each vector, require it for promotion, have `check-vectors.mjs` invoke generic Rust core/server and web-cockpit runners, and keep property tests in their established suites. This optimizes for one corpus, one traceability source, cross-language evidence, and incremental migration of existing draft vectors. It costs a small runner bridge and package test dependencies.
2. **Add a Rust-only resource conformance test with hard-coded fixtures.** This is mechanically smaller, but duplicates the JSON scenarios, cannot prove browser presentation, and lets vector metadata drift from implementation tests. It is a parallel resource harness and is rejected.
3. **Treat property tests as the executable interpretation of vectors without linking them.** This preserves current package tests, but the JSON vectors remain green-tick metadata: a vector can change or be promoted without any implementation test consuming it. That fails the feature brief and verification policy.

The chosen approach preserves the existing corpus as the single expected-example
authority while letting each owning package execute only the checks it can
observe. The trickiest unit is the **shared execution bridge**: if runner code
ignores vector input, registration is incomplete, or the umbrella script only
checks command exit status without knowing which cases ran, the feature would
manufacture assurance. That unit lands first and requires exact check-id
reporting before any resource vector can be promoted.

## Coverage map

| Coverage area | Vector(s) | Property / oracle | Core path exercised | Claim-breaking mutation that must fail |
|---|---|---|---|---|
| 1. Resource Operation grant/authority parity | extend `contracts/vectors/command-acceptance.json`; extend `contracts/vectors/failure-missing-grant.json` | `CommandDurability`; `NoCommandWithoutGrant` refined to `NoOperationWithoutGrant`; Rust `resource_operation_authority_matches_session_shape` | `acceptance::submit_with_clock` → `AuthorityRegistry::check_at` / `target_scope_matches` → `TargetRegistry::resolve` → `Storage::append_dedup_with_payload` | bypass grant check; capability-as-authority; omit requested kind/adapter/kind/id comparison; deliver before append |
| 2. Source-authenticated Observation cannot fabricate authority | `contracts/vectors/resource-observation-source-authenticated.json` | new stated property `ResourceObservationSourceAuthenticated`; server `resource_observation_source_binding` | `AdapterControlService::ingest_observation` → `authenticate_request` / `require_same_adapter` → `acceptance::ingest_observation`; authority/resource replay projections | accept missing/stale token; trust cross-adapter target; translate payload sender/grant claim into authority |
| 3. Snapshot/reconnect completeness and durability | extend `contracts/vectors/snapshot-reconciliation.json`; new `contracts/vectors/resource-snapshot-completeness-honesty.json` | `SnapshotStaleRejected`; new `ResourceSnapshotCompletenessHonesty`; Rust arbitrary trace reference model | server manifest tier admission → `resource::ingest_resource_report` / `normalize_report` → storage append → `ResourceRegistry::observe` / `rebuild_from_log` → `LoadSnapshot(RESOURCE)` | swap authoritative/partial/none omission; delta-as-snapshot omission; fold before append; lower generation/revision; resurrect tombstone |
| 4. Stale/offline never renders live | `contracts/vectors/resource-stale-never-live.json` | new `ResourceStaleNeverLive`; fast-check `resource_freshness_dominates_presentation` | disconnect `adapter_stale_event` → durable resource fold/snapshot → `markUnreconciled` / `rendersResourceCurrent` / `renderResourceDestination` | ignore reconciled, tombstoned, or freshness; promote adapter `health=serving` to current styling |
| 5. Cross-adapter resource-ID collision fenced | `contracts/vectors/resource-identity-collision-fenced.json` | new `ResourceIdentityCollisionFenced`; Rust independent tuple strategy | `ResourceIdentity::try_from_scope`; `same_resource`; resolver membership; target key; delivery adapter selection | omit adapter (also separately kind/local id) from grant/resolver/target key comparison |
| 6. Adapter cannot inject core-only state | `contracts/vectors/resource-core-state-injection-rejected.json` | new `ResourceCoreStateInjectionRejected`; Rust/server opaque-payload property | typed `ObservationRequest` dispatch; generic Observation append; `StoredEventKind`; `ResourceRegistry::observe`; core report normalization/LSN assignment | decode opaque Observation payload as `ResourceStateEvent`; accept adapter-assigned durable kind/LSN/revision/domain |

## Implementation Units

### Unit 1: Shared executable-vector bridge and property registration

**Files**: `contracts/vectors/README.md`, `contracts/scripts/check-vectors.mjs`, `core/tests/conformance_vectors.rs` (new), `server/tests/conformance_vectors.rs` (new), `web-cockpit/tests/conformance-vectors.test.ts` (new), `core/Cargo.toml`, `server/Cargo.toml`, `docs/VERIFICATION.md`

**Story**: `epic-agent-operations-resource-plane-conformance-vector-execution-bridge`

```json
{
  "vector_id": "resource-stale-never-live",
  "property_id": "ResourceStaleNeverLive",
  "promotion_status": "promoted",
  "implementation_checks": [
    { "runner": "rust-server", "case": "resource_disconnect_degrades_snapshot" },
    { "runner": "web-cockpit", "case": "resource_stale_presentation_dominance" }
  ],
  "proto_fields_constrained": [],
  "description": "...",
  "input": {},
  "expected_outcome": {},
  "invariant_check": "..."
}
```

```js
// contracts/scripts/check-vectors.mjs
const IMPLEMENTATION_RUNNERS = Object.freeze({
  "rust-core": { command: "cargo", args: ["test", "-p", "patchbay-core", "--test", "conformance_vectors"] },
  "rust-server": { command: "cargo", args: ["test", "-p", "patchbay-core-server", "--test", "conformance_vectors"] },
  "web-cockpit": { command: "npm", args: ["--prefix", "web-cockpit", "test"] },
});

async function runImplementationChecks(vectors) {
  // Validate runner/case registration, dispatch each used runner once, and
  // require its machine-readable executed-check list to equal the requested ids.
}

function validateImplementationChecks(vector, filename) {
  // Optional for draft vectors; non-empty and fully registered for promoted vectors.
}
```

```rust
// generic shape duplicated only at test boundaries in core/server
#[derive(serde::Deserialize)]
struct ConformanceVector {
    vector_id: String,
    property_id: String,
    promotion_status: String,
    implementation_checks: Vec<ImplementationCheck>,
    input: serde_json::Value,
    expected_outcome: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct ImplementationCheck { runner: String, case: String }

fn vectors_for_runner(runner: &str) -> Result<Vec<ConformanceVector>, String>;
async fn execute_case(vector: &ConformanceVector, case: &str) -> Result<(), String>;
```

```ts
// web-cockpit/tests/conformance-vectors.test.ts
interface ImplementationCheck { runner: "web-cockpit"; case: string }
interface ConformanceVector {
  vector_id: string;
  property_id: string;
  implementation_checks: readonly ImplementationCheck[];
  input: unknown;
  expected_outcome: unknown;
}
function vectorsForRunner(runner: "web-cockpit"): readonly ConformanceVector[];
function executeVectorCase(vector: ConformanceVector, caseName: string): void | Promise<void>;
```

**Implementation notes**:

- Preserve all current envelope fields and generated traceability. `implementation_checks` is additive; draft vectors without it remain metadata-only, but `promotion_status: promoted` requires a non-empty list.
- The umbrella script runs each used package runner once, passing the requested `vector_id:case` set through an environment variable or temporary JSON file and requiring a machine-readable executed set back. A successful unrelated package test does not satisfy registration.
- Runner cases must parse every identity/tier/outcome field they rely on from `input`/`expected_outcome`; missing or unknown fields fail. Hard-coded fixtures may provide infrastructure defaults only (clock, temporary database), never the claimed identity or outcome.
- Add `serde`/`serde_json` only as Rust dev-dependencies. The conformance JSON is a test artifact, not a runtime core contract.
- Register `ResourceObservationSourceAuthenticated`, `ResourceSnapshotCompletenessHonesty`, `ResourceStaleNeverLive`, `ResourceIdentityCollisionFenced`, and `ResourceCoreStateInjectionRejected` in the existing stated-normative property registry and prose. Add property-specific entries to `INVARIANT_EXPECTATION_CHECKS`; no generic “expected true” checker is acceptable.

**Acceptance criteria**:

- [ ] Every promoted vector is rejected unless at least one known runner executes every registered case and reports its exact id.
- [ ] Unknown/duplicate runner cases and missing vector input fail before traceability is regenerated.
- [ ] Existing draft vectors and the single generated table remain valid; no second registry appears.
- [ ] A test that replaces a vector identity/outcome causes its implementation check or static invariant checker to fail.

### Unit 2: Authority, source authentication, collision, and core-state isolation

**Files**: `contracts/vectors/command-acceptance.json`, `contracts/vectors/failure-missing-grant.json`, `contracts/vectors/resource-observation-source-authenticated.json` (new), `contracts/vectors/resource-identity-collision-fenced.json` (new), `contracts/vectors/resource-core-state-injection-rejected.json` (new), `core/tests/conformance_vectors.rs`, `server/tests/conformance_vectors.rs`, `core/tests/authority_proptest.rs`, `core/tests/acceptance_proptest.rs`, `server/src/adapter_service/tests.rs`

**Story**: `epic-agent-operations-resource-plane-conformance-authority-source-isolation`

```rust
// core/tests/authority_proptest.rs
fn resource_operation_authority_matches_session_shape(
    exact: &ResourceIdentity,
    requested: &ResourceIdentity,
    operation_kind: OperationKind,
) -> Result<SubmissionOutcome, String>;

fn resource_collision_oracle(
    exact: &ResourceIdentity,
    requested: &ResourceIdentity,
) -> bool {
    exact.adapter_id() == requested.adapter_id()
        && exact.resource_kind() == requested.resource_kind()
        && exact.resource_id() == requested.resource_id()
}

proptest! {
    #[test]
    fn resource_operation_authority_is_exact(
        (adapter, other_adapter) in any_distinct_ids(),
        (kind, other_kind) in any_distinct_ids(),
        (local_id, other_id) in any_distinct_ids(),
        operation_kind in any_committed_operation_kind(),
    ) { /* exact accepts; each independently changed dimension denies */ }
}
```

```rust
// server/src/adapter_service/tests.rs
async fn execute_resource_observation_source_case(
    authenticated_adapter: AdapterId,
    target: ResourceIdentity,
    claimed_sender: ActorEndpointRef,
    payload: PayloadEnvelope,
) -> Result<ObservedSourceOutcome, Status>;

proptest! {
    #[test]
    fn resource_observation_source_binding(
        (authenticated, other) in any_distinct_adapter_ids(),
        sender_claim in any_actor_endpoint_ref(),
        opaque_payload in prop::collection::vec(any::<u8>(), 0..256),
    ) { /* current owner accepted as Observation evidence; other/stale source inert */ }
}
```

**Implementation notes**:

- The positive case uses an exact resource Grant and an exact registered resource through `submit_with_clock`; it asserts `SubmissionOutcome::Accepted`, `decision_grant_id`, one durable Operation at `accepted`, and no delivery before the append exists.
- Negative cases use the same registered universe so denial cannot be explained by `target_not_found`. Independently vary grant liveness, OperationKind, adapter, resource kind, and local id, then assert no Operation append/delivery.
- Source cases run through real adapter authentication. A target owned by another adapter and stale/missing token reject before append. A same-adapter forged sender/payload may be recorded only as Observation evidence; compare authority/resource/command projections before and after to prove it grants nothing and cannot affect a differently targeted command.
- Core-state injection encodes a plausible `ResourceStateEvent` into a generic Observation payload. Assert the stored discriminator is `OBSERVATION`, replay leaves `ResourceRegistry` byte-for-byte unchanged, and subsequent exact resolution still fails. The typed ResourceReport case asserts the server replaces source/domain with authenticated request context and storage assigns LSN/revisions.
- Mutation functions deliberately omit adapter, kind, or local id; trust `Operation.sender`; skip adapter authentication/target ownership; or route Observation payload bytes to the resource fold. Feed production and mutant outcomes to the same raw-input oracle.

**Acceptance criteria**:

- [ ] Resource acceptance and missing-grant vectors extend the session files and execute the same pipeline, not a copied resource pipeline.
- [ ] Cross-adapter/kind/id attempts are registered yet denied before append and never reach either adapter.
- [ ] Unauthenticated/stale/cross-owner Observations append nothing; forged payload identity creates no authority or core state.
- [ ] Opaque `ResourceStateEvent` bytes remain Observation payload and cannot populate/alter resource resolution.
- [ ] Every named source/authority/collision/injection mutant fails.

### Unit 3: Durable completeness and reconnect properties

**Files**: `contracts/vectors/snapshot-reconciliation.json`, `contracts/vectors/resource-snapshot-completeness-honesty.json` (new), `core/tests/conformance_vectors.rs`, `server/tests/conformance_vectors.rs`, `core/tests/resource_reconciliation.rs`, `core/tests/resource_replay.rs`, `server/tests/grpc_smoke.rs`

**Story**: `epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty`

```rust
#[derive(Debug, Clone)]
enum ResourceTraceStep {
    Snapshot {
        generation: u64,
        tier: AdapterSnapshotSupport,
        listed: std::collections::BTreeMap<ResourceIdentity, ReportMutation>,
    },
    Delta {
        generation: u64,
        tier: AdapterSnapshotSupport,
        explicit: std::collections::BTreeMap<ResourceIdentity, ReportMutation>,
    },
    Disconnect { generation: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedResourceRecord {
    cache: CachePresence,
    freshness: ResourceFreshnessState,
    tombstoned: bool,
    replaced_by: Option<ResourceIdentity>,
}

fn any_resource_trace() -> impl Strategy<Value = Vec<ResourceTraceStep>>;
fn apply_reference_step(
    model: &mut std::collections::BTreeMap<ResourceIdentity, ExpectedResourceRecord>,
    step: &ResourceTraceStep,
) -> Result<(), ExpectedRejection>;

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    #[test]
    fn arbitrary_resource_report_trace_converges(
        initial in any_resource_population(),
        trace in any_resource_trace(),
    ) { /* compare reference, hot registry, replay, replay twice, snapshot */ }
}
```

**Implementation notes**:

- Extend `snapshot-reconciliation.json` with a discriminated `ResourceSnapshot` case: cached view LSN below the current resource view is not authority, `LoadSnapshotRequest.view_kind = RESOURCE` is echoed, and the replacement payload/domain/LSN match.
- `resource-snapshot-completeness-honesty.json` contains independent authoritative, partial, none, and delta cases over the same baseline identities, including cached and no-payload unknown members. It observes append count/kind, record/view revisions, hot result, replayed result, and materialized snapshot.
- The reference model uses only raw trace inputs and the documented tier truth table. It does not decode production events or call production tier/freshness helpers. Invalid generated steps have an explicit expected-rejection oracle and must leave storage/projection unchanged.
- Compare stable semantic records rather than hash-map iteration order. Separately assert resource/view revisions equal committed LSNs and source generation never decreases.
- Mutation spot-check executors: authoritative omission→stale, partial/none omission→tombstone/current, delta omission→snapshot behavior, update-before-append, last-generation-wins rollback, and tombstone upsert resurrection. Each must disagree with the reference model on a deterministic witness.

**Acceptance criteria**:

- [ ] All three snapshot tiers and live delta behavior match the independent reference model across arbitrary bounded traces.
- [ ] Hot projection, two replays, ordinary exact resolution, and materialized snapshot converge on the same semantic state.
- [ ] Report tier overclaim, wrong domain/generation/LSN/prior revision, and resurrection reject without partial state.
- [ ] The existing session snapshot vector remains intact while its resource sibling case uses explicit view discrimination.
- [ ] Every tier/durability/replay mutant fails a deterministic witness plus generated property run.

### Unit 4: Stale/offline presentation dominance

**Files**: `contracts/vectors/resource-stale-never-live.json` (new), `server/tests/conformance_vectors.rs`, `web-cockpit/tests/conformance-vectors.test.ts`, `web-cockpit/tests/model.test.ts`, `web-cockpit/tests/resource-view.test.ts`, `web-cockpit/tests/reconcile.test.ts`

**Story**: `epic-agent-operations-resource-plane-conformance-stale-presentation-dominance`

```ts
const resourceViewArbitrary: fc.Arbitrary<ResourceView> = fc.record({
  freshness: fc.constantFrom(
    ResourceFreshnessState.CURRENT,
    ResourceFreshnessState.STALE,
    ResourceFreshnessState.UNKNOWN,
  ),
  reconciled: fc.boolean(),
  tombstoned: fc.boolean(),
  domainHealth: fc.constantFrom("serving", "degraded", "exhausted", "unknown"),
}).map(({ freshness, reconciled, tombstoned, domainHealth }) =>
  validResourceView({ freshness, reconciled, tombstoned, domainHealth }));

function resourceMayRenderCurrent(view: ResourceView): boolean {
  return view.reconciled
    && !view.tombstoned
    && view.freshness === ResourceFreshnessState.CURRENT;
}

test("resource freshness dominates presentation across generated states", async () => {
  await fc.assert(fc.asyncProperty(resourceViewArbitrary, async (view) => {
    assert.equal(rendersResourceCurrent(view), resourceMayRenderCurrent(view));
    const rendered = renderResourceDestination(/* model containing view */);
    assert.equal(Boolean(rendered.element.querySelector(".resource-freshness--current")), resourceMayRenderCurrent(view));
    if (!resourceMayRenderCurrent(view)) assertNoLiveResourceClaim(rendered.element);
  }), { numRuns: 100 });
});
```

**Implementation notes**:

- The vector registers `rust-server:resource_disconnect_degrades_snapshot` and `web-cockpit:resource_stale_presentation_dominance`. The first creates current cached resource state then applies the real abnormal-disconnect source event and materializes ResourceSnapshot. The second consumes the resulting proto-shaped expected record and renders it with an adapter projection that deliberately claims `health = serving`.
- The arbitrary constructs internally valid views: unknown has no cached payload/projection; current/stale have both; tombstones never remain effective current. This avoids passing merely because invalid fixtures throw.
- `assertNoLiveResourceClaim` checks canonical current class/badge, current wording, link state, and meter/domain-health qualification. Stale cached meters may remain visible only with last-reported qualification; unknown emits no current domain health/meter.
- Explicit mutants implement `freshness == CURRENT` only, ignore tombstone, ignore reconciliation, or derive current from adapter health. The independent conjunction must reject all four.

**Acceptance criteria**:

- [ ] Real adapter disconnect yields stale cached or no-payload unknown resource state in the materialized snapshot.
- [ ] Model predicate and DOM agree exactly with the independent current-eligibility conjunction for every generated valid view.
- [ ] Adapter-owned `serving`/`ok` cannot override stale, unknown, tombstoned, or unreconciled canonical state.
- [ ] Stream-break and unequal-horizon snapshot repair never install a half-reconciled live resource.
- [ ] Every presentation-dominance mutant fails.

### Unit 5: Promotion, mutation ledger, and assurance traceability

**Files**: `contracts/scripts/check-vectors.mjs`, the eight modified/new vector files named above, `docs/VERIFICATION.md`, and the preceding package test files

**Story**: `epic-agent-operations-resource-plane-conformance-promotion-traceability-closeout`

```md
<!-- docs/VERIFICATION.md, prose outside generated blocks -->
### Operational-resource conformance evidence (implementation-checked)

| Property id | Executable vectors | Property implementation | Mutation witness | Assurance tier |
|---|---|---|---|---|
| `ResourceObservationSourceAuthenticated` | ... | ... | ... | promoted vector + implementation-checked; not model-checked |
```

**Implementation notes**:

- Promote only after each vector executes and the deep verification review confirms its expected outcome and mutation witness. Modified session-shaped vectors may be promoted because their complete session+resource scenario now executes; their draft model status still prevents checked-normative classification.
- Record one concrete killed mutant per coverage area in the feature implementation notes. The runner's aggregate executed count must equal the union of all `implementation_checks`, not merely the number of JSON files.
- Regenerate only the existing conformance traceability block. Update prose property classifications in place; do not hand-edit generated table rows.
- If a vector exposes a real implementation mismatch, apply the minimal fix at the owning acceptance/ingress/fold/presentation boundary and record it. Do not alter the vector to mirror the bug.

**Acceptance criteria**:

- [ ] All eight designed modified/new vectors are promoted, executable, statically consistent, and traceable to exact proto fields.
- [ ] Every one of the six coverage claims has a recorded production path, independent property oracle, and killed mutant.
- [ ] Full verification is green without deletion, skip, weakened expectation, or hard-coded runner success.
- [ ] Documentation says exactly “promoted vector + implementation-checked” and does not claim model-checked, checked-normative, or release-verified evidence.

## Implementation Order

1. `epic-agent-operations-resource-plane-conformance-vector-execution-bridge`
2. In parallel after the bridge: `epic-agent-operations-resource-plane-conformance-authority-source-isolation` and `epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty`
3. `epic-agent-operations-resource-plane-conformance-stale-presentation-dominance` after durable reconnect evidence supplies the real degradation source
4. `epic-agent-operations-resource-plane-conformance-promotion-traceability-closeout` after all vector/property implementations and mutation witnesses are green
5. Advance child stories through the project-specific `[verification]` deep lane; review the integrated feature at effective weight `thorough` until a fresh-context pass yields no receiver-confirmed material blocker.

One feature owner should normally carry the five checkpoints as one cohesive
verification bundle. The vector envelope, property registry, runner dispatch,
and mutation/traceability evidence are shared; splitting package ownership would
make it easier for a JSON expected example and its implementation check to drift.

## Simplification

- Evolve `contracts/scripts/check-vectors.mjs`; do not add a resource vector tool, directory, registry, or report.
- Extend the three existing session-shaped vectors where the invariant is shared; add only genuinely resource-specific examples.
- Reuse `AuthorityRegistry`, `TargetRegistry`, `ResourceRegistry`, the authority-domain log, authenticated adapter service, `LoadSnapshot`, `PresentationModel`, Rust `proptest`, and TypeScript `fast-check`.
- Keep JSON execution metadata additive and test-only. No production conformance framework, new wire field, resource lifecycle, connectivity enum, checkpoint store, or adapter capability is introduced.
- Consolidate current narrow resource reconciliation examples into one arbitrary-trace property rather than accumulating one test per branch; retain focused regressions only when they provide a deterministic mutation witness.
- No existing test is identified for removal. Existing branch examples may be factored into shared fixtures only after the arbitrary property independently covers their contract.

## Testing

The tests are the deliverable, so their own integrity is part of acceptance:

- **Vector interface checks**: JSON envelope/property/proto-field validation, static invariant expectation validation, exact runner/case registration, and execution-set accounting. Risk protected: metadata-only or unexecuted promotion.
- **Core/server executable vectors**: real acceptance, authenticated ingress, durable append/fold/replay, resolver, and snapshot boundaries. Risk protected: a scenario executor that tests helper logic instead of the product seam.
- **Rust properties**: independently generated identity dimensions and arbitrary report traces against raw-input reference oracles. Risk protected: collision widening, dishonest completeness, replay divergence, and core-state injection.
- **TypeScript properties**: valid generated resource views and rendered DOM against an independent current-eligibility conjunction. Risk protected: stale/adapter-health presentation bypass.
- **Mutation spot-checks**: deterministic witnesses for every claim-breaking mutation plus generated runs. A property that passes production but also passes its mutant is a blocker and must be restructured.
- **Regression surface**: existing session authority/snapshot/reconciliation, command lifecycle, adapter attach, generated contract, and presentation checks remain green. The resource analogue must not weaken the session shape it extends.
- **No green-tick theater**: runner code must consume vector input; expected outcomes are not derived from production output; reference models do not call judged helpers; no assertion is solely “function returned success.”

Verification commands include `node contracts/scripts/check-vectors.mjs`, focused
`cargo test` for the generic conformance runner and changed property suites,
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`npm --prefix web-cockpit test`, generated drift, model metadata, and
presentation conformance.

## Risks

- **Metadata runner masquerading as execution (highest risk).** The current checker does not run product code. The bridge requires exact runner/check accounting and vector-field consumption. Fallback is to leave vectors draft and the feature incomplete, not to promote metadata.
- **Reference oracle duplicates production.** A copied normalization algorithm can share the same bug. The resource trace oracle is deliberately a small truth-table/set model over raw input and may not call production helpers. Deep review must compare each branch to PROTOCOL, not implementation.
- **Source authentication can be overstated.** The adapter channel proves the adapter attachment and target ownership; it does not automatically prove every actor named inside an Observation payload. Evidence is scoped to channel binding and non-authority. Any stronger sender claim requires a separate verified actor contract.
- **Promotion-tier confusion.** Promoted executable examples plus properties are meaningful implementation evidence, but without a promoted model they are not checked-normative or release-verified. Generated and prose tables must retain that distinction.
- **Cross-language runner cost/flakiness.** The umbrella checker invokes Rust and web suites. Dispatch each used runner once, use deterministic in-memory stores/fixed clocks, and report individual check ids; never hide a flaky runner behind retry.
- **Existing review-discovered seams recur.** Resource-state review already found unknown→stale fabrication, authoritative unknown, manifest-degradation atomicity, and insufficient arbitrary traces. These become explicit generated cases and mutation witnesses rather than relying on prior regressions alone.
- **No design-time independent advisory dispatch.** The delegated worker cannot run one. The project-specific `[verification]` deep story lane and explicit `thorough` feature review must attack runner field consumption, oracle independence, and mutation survival before promotion.

## Extension pressure classification

- **Committed post-v0.1 direction:** promoted executable resource vectors and implementation properties for exact grant/target authority, authenticated adapter-source binding, durability/replay, snapshot completeness honesty, stale presentation dominance, collision fencing, and core-owned state emission.
- **Reserved seams:** formal/model promotion of the new resource property ids; token-commune external-contract certification; third-party adapter certification profiles; knowledge-bundle conformance; periodic typed resource checkpoints; multi-authority/federated resource evidence.
- **Explicitly rejected for this arc:** a resource-only conformance harness, capability-derived authority, adapter-selected durable state/LSN/revision, opaque Observation-driven resource folding, adapter-health-driven live presentation, and claiming release verification from vectors/property tests alone.

## Other agent review

- Invoked because: this feature claims security, durability, and stale-state evidence across generated contracts, Rust core/server, and TypeScript presentation.
- Fixed/active blockers in design: the current metadata-only vector gap is made an explicit first checkpoint; every property has a raw-input oracle and named mutant; assurance-tier wording is bounded.
- Parked: none. Formal promotion is an explicit reserved seam, not silently included in this implementation-evidence feature.
- Rejected: Rust-only hard-coded resource vectors and unlinked property tests because neither makes the existing JSON corpus executable.
- Skipped/degraded: independent advisory dispatch is unavailable in this worker surface. Direct grounding was exhaustive across sibling designs/implementation/review findings and foundation/code paths; the caller's `thorough` review and the project verification deep lane remain mandatory.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, high reasoning, explicitly selected by the autopilot caller for executable safety evidence.
- Review weight: `thorough`, explicit caller override. The feature remains `implementing` while all five verification children wait at `review` for their required deep lane.
- Integrated delivery: the existing corpus now has additive exact package execution accounting; all eight designed resource/shared vectors are promoted; five stated resource property ids are registered; Rust core/server and web-cockpit runners consume vector fields and report eleven exact package check ids, with the three extended vectors executing and validating both their session and resource cases; independent Rust and fast-check oracles cover authority, source binding, completeness/durability, stale presentation, collision fencing, and core-state isolation.
- Mutation ledger:
  - Authority/durability: fabricated accepted `EventId` without append failed `command-acceptance`; an empty-grant authorization bypass failed `failure-missing-grant`.
  - Observation source: unconditional adapter ownership matching admitted a cross-owner request and failed `resource-observation-source-authenticated`; missing/stale token cases remained separately executed.
  - Identity collision: omitting adapter from production equality failed both generated authority property and `resource-identity-collision-fenced`; explicit kind/id-omitting mutant witnesses also fail the independent tuple oracle.
  - Core-state injection: dispatching opaque Observation payload bytes to `ResourceStateEvent` failed `resource-core-state-injection-rejected` on forged domain/state; the retained durable discriminator regression proves production ignores sibling event kinds.
  - Completeness/durability/replay: authoritative→stale, partial/none→tombstone, delta-as-snapshot, fold-before-failed-append, generation rollback, tombstone resurrection, and zero materialized snapshot LSN mutants each failed a vector/property witness. The partial-tombstone mutation initially exposed a missing tombstone assertion in the vector runner; the oracle was strengthened before promotion and the same mutant then failed.
  - Stale presentation: omission of disconnect resource degradation, freshness-only current eligibility, and adapter-health-driven effective freshness each failed server/vector/generated DOM evidence.
  - Runner integrity: missing promoted implementation registration, unregistered case, duplicate execution reports, and a changed vector identity all failed closed; failed checks did not regenerate traceability.
- Integrated verification: final evidence is recorded in the promotion child story; no existing test/vector was weakened, skipped, or deleted.
- Discrepancies from design: no formal model was added; the new properties remain implementation-evidence stated-normative. The arbitrary completeness trace constrains authoritative omission to an optional terminal step so subsequent generated actions cannot become invalid resurrection attempts; focused regressions cover rejection traces.
- Adjacent issues parked: none.
