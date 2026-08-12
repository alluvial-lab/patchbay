---
id: resource-reconciliation-followups
kind: feature
stage: done
tags: [adapter, protocol]
parent: null
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-08-08
updated: 2026-08-10
---

# Resource reconciliation follow-ups

## Brief
Consolidate the two items parked from the resource-state review into the resource reconciliation follow-up. Absorbed findings:

- **`backlog-resource-generation-obsolete-event-no-op`** — preserve obsolete-event no-op behavior when generation monotonicity and replay/catch-up ordering interact. *Src:* parked from the `…resource-state` review. *Currency (2026-08-09 review):* **OPEN** — the adapter-generation rejection runs first (`core/src/resource/registry.rs:100-112`), before per-view/per-record obsolete-LSN filtering (`registry.rs:119-149`), so an event otherwise obsolete for its affected records can still become corruption after a newer generation is projected — exactly as the finding states. *Direction:* the no-op rule must be defined at the replay/catch-up prefix boundary, not by weakening generation monotonicity inside the fold: an event is wholly inert only when the projection is known to represent a contiguous prefix through its LSN; a lower-generation event is otherwise corruption. Track a validated global applied cursor outside the resource fold. *Disposition:* **keep** — the obsolete-event semantic rule must be specified in protocol terms *before* the evidence is generated (the review's BLOCKER: a broad no-op can mask a real mutation).
- **`backlog-resource-reconciliation-arbitrary-sequences`** — expand reconciliation evidence to arbitrary sequences (the brief's "two-report sampler" framing is **stale**). *Src:* parked from the `…resource-state` review. *Currency:* **PARTIAL** — an arbitrary 1–20-step, 100-case report trace already exists (`core/tests/resource_reconciliation.rs:159-179`), but it fixes adapter generation to 1 (`:319-323`) and doesn't combine generation transitions, explicit replacements, replay, and terminal mutation attempts in one generated trace; focused replacement/generation/terminal tests exist separately (`resource_ingest.rs:172-273`, `resource_state.rs:15-77`). *Direction:* add the missing cross-dimensional traces (generation transitions within a generated trace; same-event replacements; post-terminal mutation attempts; obsolete catch-up prefixes; hot/replay/replay-twice equality after every accepted prefix; negative traces leave prefix+projection unchanged). *Disposition:* **keep**, narrowed to the missing dimensions — do NOT reimplement a generic arbitrary-sequence test.

*Currency verified 2026-08-09. Per the review this feature is **coherent as one narrowed feature** — both surviving concerns belong to resource reconciliation — but: (1) specify the obsolete-event no-op/corruption rule in protocol terms before generating evidence (it's a semantic choice, not just a test); (2) acknowledge the existing arbitrary-sequence coverage and focus only on the missing cross-dimensional traces.*

## Simplification opportunity
Express obsolete-event handling and arbitrary sequence evidence through the existing resource conformance/reconciliation fold rather than adding a parallel resource-state mechanism.

## Obsolete-event no-op/corruption rule (specified 2026-08-09)

Specified in protocol terms — the review's BLOCKER (this semantic choice must be fixed *before* evidence is generated: a broad no-op can mask a real mutation). Verified against `core/src/resource/registry.rs` `apply_validated`. `docs/PROTOCOL.md:43` delegates resource obsolete/replacement/tombstone semantics to the resource-state contract, i.e. this feature.

**The problem (confirmed in code).** `apply_validated` runs the generation guard *first*: it computes `projected_generation` = max applied `source_adapter_generation` across the adapter's views (`registry.rs:101-107`) and rejects any event with `generation.value < projected_generation` as `CorruptLog` ("lowers adapter generation", `:108-112`) — *before* the per-record obsolete filter (`revision_lsn >= event_lsn → continue`, `:119-125` views / `:144-150` resources). So an event that is per-record-obsolete can still be rejected as corruption once a newer generation has been projected. This contradicts the feature's own observer contract ("a redelivered event at or below the record revision is inert") and bites under catch-up/reconnect re-feed ordering (ordered replay from LSN 0 does not produce it — the original finding correctly notes it is latent).

**Why per-record obsolete is NOT a sound inertness test (do not just reorder the checks).** A single `ResourceStateEvent` can touch several views and identities. An event may be obsolete for one projected record yet still carry a previously-unseen identity, a current mutation for another view, a terminal replacement, or a view revision not yet represented. Per-record `revision_lsn >= event_lsn` across the event's touched records is therefore necessary but not sufficient for inertness.

**Specified rule — inertness at the applied-prefix boundary, tracked outside the fold.**
1. Maintain a **validated applied-LSN cursor per authority-domain resource projection** — the highest *contiguous* LSN the projection has applied (the global applied prefix). The registry observes the shared log for every adapter; adapter generation remains source-adapter-scoped inside the new-event fold. This is global prefix state, NOT derivable from per-record `revision_lsn` (records are sparse; a record's revision can lag the prefix).
2. **Evaluate prefix-coverage BEFORE the generation guard.** An incoming event whose `event_lsn ≤ cursor` is **prefix-covered → inert audit no-op**, regardless of its source generation. This restores the "redelivered event at or below revision is inert" contract for the obsolete case without weakening generation monotonicity.
3. An event whose `event_lsn > cursor` is **new** → it must satisfy generation monotonicity (a lower source-generation event beyond the prefix is corruption: ordered application is monotonic per adapter) + per-record `from_revision` validation, then advance the cursor to the new contiguous frontier.
4. Generation monotonicity is thereby preserved as a **new-event** invariant; obsolete events are routed through the prefix cursor rather than the generation guard — so the two no longer collide.

**Cross-cutting seam.** This is the resource-plane instance of the cross-projection replay-integrity invariant (couples with the `authority-provenance-hardening` replay-gap-detection split and the sessions replay-equality work): a shared contiguous-prefix + gap-free + reject-`Unspecified` replay discipline across authority/session/resource projections. Scope the resource cursor here; promote to a shared replay-integrity seam if a second projection needs the same cursor.

**Evidence the feature must then generate** (narrows the arbitrary-sequence finding): obsolete catch-up-prefix events across adapter-generation transitions; a lower-generation event that is prefix-covered (inert, not corruption) vs one beyond the prefix (corruption); same-event replacements; post-terminal mutation attempts; hot/replay/replay-twice equality after every accepted prefix; negative traces whose rejected candidates leave the durable prefix + projection unchanged.

## Design decisions

- **Represent coverage once per authority-domain `ResourceRegistry`, not as a second per-adapter map.** The vetted rule is adapter-scoped in effect because generation is checked for `source_adapter_id`, but prefix coverage is a fact about the one shared authority-domain log that the registry observes, including interleaved sibling event kinds. A per-adapter cursor would still need the global cursor to prove those intervening LSNs and would duplicate state. The registry instance therefore stores one `(authority_domain_id, applied_through_lsn)` prefix outside its resource/view fold; this is the precise, smaller representation of the body's global-prefix direction.
- **Validate outer identity and owned payload before classifying redelivery; classify before generation and revision checks.** Missing/zero LSN, wrong/empty domain, unknown or `UNSPECIFIED` durable kind, and malformed `RESOURCE_STATE` payloads never become inert merely because they claim an old LSN. Once structural validation succeeds, `event_lsn <= applied_through_lsn` is a whole-event no-op. Only `event_lsn == applied_through_lsn + 1` may enter the new-event fold; a gap is corruption.
- **The durable record already is the audit/reconciliation evidence.** Prefix-covered re-feed does not append another audit event or advance a cursor. It returns success with the complete projection unchanged; adding a second durable record for delivery duplication would itself change the prefix and undermine idempotence.
- **Full recovery remains stricter than catch-up re-feed.** `rebuild_from_events` consumes storage rows from LSN 1 and rejects duplicate, decreasing, or gapped rows. `ResourceRegistry::observe` may accept an already-covered record only when a caller is re-feeding a previously validated prefix into an existing projection. This prevents a corrupt storage log from laundering duplicate rows as ordinary catch-up.
- **Synchronize before normalizing, then reconcile the exact committed suffix.** `ingest_resource_report` incrementally folds the storage tail through the registry's applied cursor before deriving `from_revision_lsn` or generation effects. `CoreDecisionGate` remains the serialization boundary for competing resource decisions, but sibling durable writers are not assumed to share it. After append, ingress reads and atomically folds the stored contiguous suffix through the returned report LSN, including interleaved known sibling events, and requires the suffix to end with the exact committed report. A valid authoritative commit returns success; a missing/corrupt/substituted suffix still fails closed and triggers authoritative rebuild. Generation monotonicity is never weakened to recover.
- **Reuse existing assurance vocabulary.** The arbitrary trace remains implementation/property evidence. One exact `resource-replay-prefix-idempotent` vector traces the selected obsolete-event rule to the existing stated-normative `IdempotentLogReplay` property. Promoting that executable example does not promote the draft model or create checked-normative status.
- **No UI work or mockups.** This changes replay/resource projection semantics and test evidence only; no human surface or visual structure changes.
- **Execution and review posture.** Direct-read mapping was sufficient because the touched module, replay path, server catch-up path, and tests are explicit. Nested subagents/peer review were prohibited for this delegated endpoint, so design-time advisory review is recorded as unavailable and non-blocking. Implementation remains one cohesive feature-owner bundle. Effective implementation/final review weight is `thorough` from the explicit operator selection and must be passed unchanged.

## Codebase mapping

Direct reading verified `core/src/resource/{registry,replay,ingest,state}.rs`, `core/src/target.rs`, server-wide projection catch-up in `server/src/state.rs`, adapter report/registration serialization in `server/src/adapter_service.rs` and `core/src/adapter/mod.rs`, the focused resource state/replay/ingest tests, the existing 100-case reconciliation property, and the Rust conformance-vector runner. The server already feeds every durable event to the composite target registry during catch-up and rebuilds the adapter-owned resource projection under `CoreDecisionGate` before report ingestion. The missing source of truth is local to `ResourceRegistry`: it currently ignores sibling events before identity validation and has only record/view revisions, so it cannot prove a whole applied prefix.

## Architectural choice

### Options considered

1. **Authority-domain applied prefix inside the `ResourceRegistry` boundary (chosen).** Add one domain-qualified cursor beside resources/views. `observe` validates the event, treats covered events as whole-event no-ops, advances for known sibling events, and folds a resource event only at the next LSN. This optimizes for one source of obsolete-event truth, atomic cursor+projection updates, and later extraction into the cross-projection replay-integrity seam. It requires all hot writers to catch up the registry before deriving a new event.
2. **Thread an external cursor argument through every resource fold call.** Keeping the cursor in server/replay orchestration would make coverage explicit, but `ResourceRegistry` is also used by adapter ingestion, target composition, core tests, and registration/degradation batches. Threading two independently mutable objects through those paths makes atomicity and equality harder and permits cursor/projection skew.
3. **Reorder the generation guard after per-record obsolete checks.** This is the smallest edit but is rejected: one event can touch multiple views/identities, so partial obsolescence can hide a new mutation, replacement, or terminal action. It preserves the exact broad no-op the review identified as unsafe.

The chosen design places the trickiest unit first: **classifying and atomically advancing the applied prefix without weakening generation or accepting a corrupt replay row**. Generated sequence evidence follows only after that rule is executable.

## Implementation Units

### Unit 1: Prefix-aware resource projection boundary

**Files**: `core/src/resource/registry.rs`, `core/src/resource/replay.rs`, `core/src/resource/ingest.rs`, `core/tests/resource_state.rs`, `core/tests/resource_replay.rs`, `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, `docs/GLOSSARY.md`

**Story**: `resource-reconciliation-followups-applied-prefix-semantics`

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AppliedPrefix {
    authority_domain_id: Option<AuthorityDomainId>,
    applied_through_lsn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrefixPosition {
    Covered,
    Next,
}

impl AppliedPrefix {
    fn classify(
        &self,
        authority_domain_id: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<PrefixPosition, ResourceError>;

    fn advance(
        &mut self,
        authority_domain_id: &AuthorityDomainId,
        event_lsn: u64,
    );
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResourceRegistry {
    resources: HashMap<ResourceIdentity, ResourceRecord>,
    views: HashMap<ResourceViewKey, ResourceViewRecord>,
    applied_prefix: AppliedPrefix,
}

impl ResourceRegistry {
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), ResourceError>;
    pub(crate) fn applied_lsn(&self) -> u64;
}

pub(crate) async fn catch_up_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    registry: &mut ResourceRegistry,
) -> Result<(), ResourceError>;
```

**Implementation notes**:

- `AppliedPrefix::classify` rejects an empty/mismatched domain and LSN 0. It returns `Covered` for `1..=applied_through_lsn`, `Next` only for the checked `applied_through_lsn + 1`, and `CorruptLog` for a gap. Use checked addition and fail on overflow.
- `observe` parses `StoredEventKind` and rejects unknown/`Unspecified` before prefix advancement. For `ResourceState`, decode and run the existing structural `validate_event` first. Then classify the LSN. A covered event returns `Ok(())` before generation and `from_revision_lsn` checks. A known sibling at `Next` advances only the prefix; a resource event at `Next` applies against a clone, advances the clone only after every mutation succeeds, and installs clone + prefix together.
- Delete the per-view and per-resource `revision_lsn >= event_lsn { continue; }` branches. With a validated contiguous prefix, covered events are already intercepted as a whole and any new event is necessarily later than every revision derived from that prefix. `validate_from_revision` remains the exact new-event history check.
- `catch_up_from_log` reads after `registry.applied_lsn()` and feeds every returned durable kind in strict contiguous order. `ingest_resource_report` calls it before `normalize_report`. After append, read the bounded stored suffix from that applied cursor through the returned report LSN, require its final record to equal the committed report, fold all interleaved known sibling events against a clone, and install only after the complete suffix succeeds. Do not add a second storage head or snapshot cursor. The caller still serializes competing resource decisions through `CoreDecisionGate`; unrelated shared-log writers need not use it.
- `rebuild_from_events` independently requires the requested authority domain and exact LSN sequence `1, 2, ...`; it then delegates payload/fold behavior to `observe`. It must not treat duplicate rows in storage as benign.
- Roll PROTOCOL's current per-view late-event wording forward to whole prefix coverage and state that a lower-generation event beyond the prefix is corruption. ARCHITECTURE/GLOSSARY name the resource projection cursor without presenting it as a new wire or persistence primitive.

**Acceptance criteria**:

- [ ] A structurally valid generation-1 resource event re-fed after generation 2 at a covered LSN is inert, including cursor, resources, views, tombstones, and revisions.
- [ ] The equivalent generation-1 event at the next LSN is `CorruptLog`; neither cursor nor projection changes.
- [ ] Sibling events establish coverage only when domain/kind/LSN framing is valid and contiguous; malformed or gapped events do not advance.
- [ ] Full replay rejects duplicate/decreasing/gapped storage rows while repeat catch-up against an existing complete prefix is idempotent.
- [ ] Report normalization observes the current durable prefix before deriving generation/revision mutations; an append/fold failure rebuilds rather than continuing with a false cursor.

### Unit 2: Cross-dimensional traces and executable prefix witness

**Files**: `core/tests/resource_reconciliation.rs`, `core/tests/resource_state.rs`, `core/tests/resource_replay.rs`, `core/tests/conformance_vectors.rs`, `contracts/vectors/resource-replay-prefix-idempotent.json` (new), `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`

**Story**: `resource-reconciliation-followups-cross-dimensional-evidence`

```rust
#[derive(Clone, Debug)]
enum ReconciliationAction {
    Report {
        generation_step: GenerationStep,
        mode: ResourceReportMode,
        tier: AdapterSnapshotSupport,
        mutation: ReportAction,
    },
    ReplaceActive,
    RefeedCovered,
    MutateRetired,
    LowerGenerationBeyondPrefix,
}

fn arbitrary_reconciliation_trace()
    -> impl Strategy<Value = Vec<ReconciliationAction>>;

#[derive(Clone, Debug, Default)]
struct ReconciliationOracle {
    generation: u64,
    active: HashSet<ResourceIdentity>,
    retired: HashSet<ResourceIdentity>,
    applied_through_lsn: u64,
}
```

**Implementation notes**:

- Evolve the existing `arbitrary_resource_report_trace_matches_independent_truth_table` strategy; do not create another generic mode/tier sampler. Keep 100 cases and 1–20 actions, but add actions that force generation advance, same-event `old tombstone + distinct new upsert`, an upsert/unknown/tombstone attempt against retired identity, covered event re-feed, and a lower-generation next-event candidate.
- The oracle is intentionally smaller than production. It uses raw action, active/retired identity, generation, and durable-event facts to predict accept/reject and visible membership. It must not call `normalize_report`, `stale_change`, prefix classification, production generation comparison, or production tombstone helpers.
- After every accepted append, read the exact durable event prefix and assert: hot registry = fresh replay A = fresh replay B; then feed the same prefix into a clone of replay A and assert unchanged. Compare records/views and the embedded cursor, so equality is horizon-sensitive by design.
- Before every predicted rejection, clone the registry and read the durable events. After the attempt, compare the full registry and exact event list, not only resource count. This protects prefix, revisions, freshness, tombstones, and append-before-fold together.
- Add `resource-replay-prefix-idempotent.json` under the existing `IdempotentLogReplay` property. Its deterministic sequence fixes the exact semantic example: generation-1 upsert, generation-2 atomic replacement, generation-1 covered re-feed (success/no change), and generation-1 next-event candidate (corruption/no change). Add a property-specific static expectation in `check-vectors.mjs` and a `rust-core` product runner case. Promotion remains implementation evidence for a stated-normative property because the formal model stays draft.
- Update VERIFICATION's operational-resource evidence and generated traceability honestly; do not claim model-checked, checked-normative, or release-verified status.

**Acceptance criteria**:

- [ ] Generated traces cover all five missing dimensions and shrink to readable failing sequences.
- [ ] Hot/replay/replay-twice equality is checked after every accepted prefix, not only once at trace end.
- [ ] Every rejected candidate proves durable prefix + full projection unchanged.
- [ ] The executable vector kills moving prefix coverage after the generation guard, partial installation from a failed replacement, terminal resurrection, and cursor advance on rejection. Its covered-LSN witness re-feeds the immutable exact committed record; it does not claim to kill per-record filtering via an alternative same-LSN payload.
- [ ] Existing completeness, ingest, server, resource conformance, and vector checks remain green without weakening an expected outcome.

## Implementation Order

1. `resource-reconciliation-followups-applied-prefix-semantics`
2. `resource-reconciliation-followups-cross-dimensional-evidence` depends on the fixed prefix semantics.
3. Advance child checkpoints directly on their required evidence, then review the integrated feature at effective weight `thorough`; repeat fresh-context review → adjudication → fix → verification until a pass yields no receiver-confirmed material current-cycle blockers.

The feature remains one implementation-owner bundle: both stories modify the same fold/test concepts, and the second exists to preserve semantic-before-evidence ordering rather than to manufacture a parallel worker target.

## Simplification

- Remove the two per-record/per-view obsolete-LSN skip paths; whole-event prefix coverage becomes the single no-op rule.
- Keep the cursor inside the existing resource projection boundary instead of adding a replay service, adapter cursor registry, wire field, snapshot table, or parallel resource-state mechanism.
- Reuse the authority-domain log, `ResourceRegistry::observe`, `CoreDecisionGate`, `ResourceError`, existing proptest, and the existing `IdempotentLogReplay` property vocabulary.
- Do not broaden the current omission truth-table test or duplicate it; add only the missing cross-dimensional actions.
- Preserve generation monotonicity and terminal tombstones. No compatibility shim is owed for internal pre-release projection behavior.

## Testing

- **Prefix interface regressions** protect the precise covered-vs-next generation rule, atomic prefix advance, sibling-event coverage, and strict full replay.
- **Generated reconciliation property** protects interactions that focused tests cannot compose economically: generation, replacement, terminality, prefix re-feed, and rejection atomicity across 1–20 actions.
- **Executable conformance vector** protects the review-selected obsolete-event example through a stable JSON input and production runner, independently of random generation.
- **Existing focused tests retained** protect completeness tiers, unknown/no-payload honesty, source generation, replacement shape, append-before-fold, and server ingress. Do not replace them with the generator.
- **No low-value additions**: no tests for cursor getters, enum serialization, or trivial clone/default behavior; no second random test with the same action space.

Planned implementation verification:

```bash
cargo test -p patchbay-core --test resource_state --test resource_replay --test resource_ingest --test resource_reconciliation
cargo test -p patchbay-core --test conformance_vectors -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node contracts/scripts/check-vectors.mjs
node contracts/scripts/check-models.mjs
node contracts/scripts/check-generated-drift.mjs
```

## Risks

- **A caller can lie about coverage by feeding only resource events.** The cursor advances on every known durable event, report ingestion catches up from storage before normalization and folds the exact stored suffix through its returned append LSN afterward, and full replay requires gap-free order. The decision gate serializes competing resource decisions without becoming a requirement for unrelated shared-log writers. Fallback: rebuild the resource projection from LSN 0; never infer coverage from record revisions.
- **A cursor update could survive a failed fold.** Clone/apply/advance/install is one in-memory transaction, and negative traces compare the entire registry. Fallback: discard and rebuild from the authoritative durable log.
- **The generator could become self-confirming.** Its oracle is prohibited from using production predicates/helpers and the stable vector supplies a literal second oracle. Mutation witnesses must demonstrate failure when each protected branch is broken.
- **Adding the cursor makes registry equality horizon-sensitive.** This is intentional. Tests comparing registries must compare the same durable prefix; an equal resource map at different horizons is not sufficient evidence for obsolete-event safety.
- **The existing shared replay paths do not yet all enforce the same discipline.** This feature scopes the resource instance only. `replay-integrity-prefix-discipline` remains the independent cross-projection consolidation item; this feature does not depend on it or preempt its shared abstraction.
- **An ungated sibling writer can interleave after catch-up.** This is valid in the shared authority-domain log: post-append bounded suffix folding consumes the sibling and the committed report together, so no false gap or retry ambiguity escapes. A competing *resource-decision* writer outside `CoreDecisionGate` can still invalidate normalized prior revisions; that malformed committed history fails closed, rebuilds, and surfaces corruption rather than weakening the generation/revision rules.

## Extension pressure classification

- **Committed post-v0.1 direction:** resource projection redelivery is a whole-event decision backed by a validated `(authority_domain_id, applied_through_lsn)` prefix; prefix-covered events are inert before adapter-generation checks, while new events preserve monotonic generation, exact prior revision, replacement, and tombstone rules.
- **Reserved seam:** promotion of this local cursor into a shared authority/session/resource replay-integrity component belongs to `replay-integrity-prefix-discipline` once a second projection consumes the abstraction. A typed persisted projection checkpoint may later seed the same cursor but is not introduced here.
- **Explicitly rejected for this feature:** per-record obsolete filtering, reordering generation beneath partial record checks, per-adapter duplicate prefix maps, a new wire cursor, or a parallel resource state/replay mechanism.
- **Pressure-test result:** the domain-qualified cursor preserves the federation demarcator, remains adapter-neutral and surface-neutral, and does not foreclose the parked multi-human, desktop, mesh, or skin ideas.

## Other agent review

- Review would be warranted because: the design is replay/protocol-bearing and a broad no-op could mask mutation.
- Fixed/active blockers: the review-vetted body already selected prefix-boundary inertness over per-record filtering; this design makes structural-validation order, gap behavior, cursor atomicity, and evidence authority explicit.
- Parked: none created here. Cross-projection consolidation already exists as `replay-integrity-prefix-discipline` and remains independent.
- Rejected: per-record filtering and per-adapter cursor duplication for the reasons above.
- Skipped/degraded: the delegated endpoint explicitly prohibited nested subagents and peeragent. Design-time independent advice was therefore unavailable and non-blocking per policy. The caller-selected `thorough` feature and final completion review remains mandatory.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-sol`; explicit caller selection for protocol replay integrity and negative-state atomicity.
- Review weight: `thorough` from the explicit caller selection; implementation stops at `stage: review` for a fresh reviewer.
- Integrated child commits: `4597fda` (`resource-reconciliation-followups-applied-prefix-semantics`) and `1aa4351` (`resource-reconciliation-followups-cross-dimensional-evidence`); feature integration commit: `9170538`.
- Files changed: `core/src/resource/{registry,replay,ingest}.rs`; `core/tests/{resource_state,resource_replay,resource_ingest,resource_reconciliation,conformance_vectors}.rs`; `contracts/vectors/resource-replay-prefix-idempotent.json`; `contracts/scripts/check-vectors.mjs`; `docs/{PROTOCOL,ARCHITECTURE,GLOSSARY,VERIFICATION}.md`.
- Tests added/removed: added prefix framing/atomicity and ingest catch-up regressions, replaced the generic bounded sampler with the 100-case cross-dimensional trace, and added the promoted vector's exact Rust runner/static expectation; removed no focused contract tests.
- Simplification: whole-event applied-prefix classification replaces both per-view/per-record obsolete branches; existing storage, fold, report, reconciliation, and `IdempotentLogReplay` surfaces carry all evidence without a new service/model/property id.
- Discrepancies from design: none. An initial repeat of the workspace test encountered environmental disk exhaustion from generated incremental artifacts; the target directory was cleared and the final full run passed with `CARGO_INCREMENTAL=0`.
- Adjacent issues parked: none.

## Integrated verification
- Both child checkpoints are `stage: done` with their own implementation evidence and commits.
- `CARGO_INCREMENTAL=0 cargo test --workspace` — passed on the final tree, including core/server integration, conformance runners, and doc tests.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `node contracts/scripts/check-vectors.mjs` — passed: 53 vectors, 16 promoted, 21 implementation checks, 37 existing declared mutation witnesses.
- `node contracts/scripts/check-models.mjs` — passed with generated traceability current; `IdempotentLogReplay` remains stated-normative because its model is draft.
- `node contracts/scripts/check-generated-drift.mjs` — passed; generated Rust/TypeScript contracts are unchanged.
- Acceptance walk: covered generation-1 re-feed after generation 2 is a complete no-op; the same generation at the next LSN is atomic corruption; sibling framing/gaps/full replay and report catch-up are enforced; accepted trace prefixes converge across hot/two-fresh/covered replays; rejected next-generation and terminal candidates preserve exact durable events and the full cursor-bearing projection; the promoted vector and verification prose retain implementation-checked rather than model-checked authority. The covered witness is the exact immutable committed record, not an alternative same-LSN payload, so it makes no per-record-filter mutation-kill claim.

## Review pass 1 receiver fix
- Accepted material finding: the successful replacement path did not discriminate failure atomicity. The promoted vector now constructs a structurally valid next-LSN pair where the active replacement source can be tombstoned first but the paired upsert targets an already-terminal identity. The production fold returns `TerminalTombstone`; exact registry equality covers cursor/resources/views, and the storage event list remains the same three-record durable prefix.
- Evidence correction: covered replay feeds the exact committed `RecordedEvent` read from storage. The prior per-record-filter mutation-kill wording was retracted because an alternative payload at the same `(authority_domain_id, LSN)` violates the immutable committed-record contract; no payload-equivalence check inside `ResourceRegistry` is claimed.
- Static/vector/docs alignment: `resource-replay-prefix-idempotent.json`, its Rust runner, the `IdempotentLogReplay` static expectation, and `docs/VERIFICATION.md` now name only the executed failed-replacement and exact-record witnesses.
- Focused verification: `CARGO_INCREMENTAL=0 cargo test -p patchbay-core --test conformance_vectors -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p patchbay-core --test resource_state --test resource_replay --test resource_ingest --test resource_reconciliation`; `CARGO_INCREMENTAL=0 cargo clippy -p patchbay-core --test conformance_vectors --test resource_state --test resource_replay --test resource_ingest --test resource_reconciliation -- -D warnings`; `CARGO_INCREMENTAL=0 node contracts/scripts/check-vectors.mjs` — all passed (53 vectors, 16 promoted, 21 implementation checks, 37 mutation witnesses).
- Lifecycle: feature intentionally remains at `stage: review` for the required thorough follow-up pass.

## Review pass 2 receiver fix
- Accepted material finding: post-append ingest observed only the locally returned `RESOURCE_STATE` event and therefore treated a valid interleaved sibling LSN as a gap. The assumption that every shared-log writer participates in `CoreDecisionGate` was invalid for sibling/audit writers; after an authoritative report commit this escaped as `Internal` and made an unnecessary retry ambiguous.
- Fix: pre-append catch-up and resource-decision serialization remain unchanged. After append, `catch_up_through_event` reads the bounded durable suffix from the registry's applied cursor through the returned report LSN, requires strict domain/LSN contiguity and the exact committed report as the final record, folds every known sibling/resource event against a clone, and installs only the complete result. A valid committed report now returns success; missing, reordered, corrupt, or substituted suffixes remain fail-closed and run the authoritative rebuild path.
- Deterministic regression: `report_ingest_folds_an_interleaved_ungated_audit_and_returns_committed_success` injects a real audit append inside the storage port after catch-up but before the resource append. It proves one report attempt returns the committed LSN without a false gap/Internal/retry ambiguity, consumes audit + report in order, and leaves hot projection equal to fresh replay. `report_ingest_fails_closed_when_the_committed_suffix_is_missing` withholds the bounded suffix once and proves ingest returns an error while rebuild reinstalls durable authority.
- Protocol alignment: `docs/PROTOCOL.md` now distinguishes serialization of competing resource decisions from unrelated sibling writers and specifies exact post-append suffix folding plus fail-closed suffix validation.
- Verification: `CARGO_INCREMENTAL=0 cargo test -p patchbay-core --test resource_state --test resource_replay --test resource_ingest --test resource_reconciliation`; `CARGO_INCREMENTAL=0 cargo test -p patchbay-core --test conformance_vectors -- --nocapture`; `node contracts/scripts/check-vectors.mjs` (53 vectors, 16 promoted, 21 implementation checks, 37 mutation witnesses); `CARGO_INCREMENTAL=0 cargo test --workspace`; `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings` — all passed.
- Lifecycle: feature remained at `stage: review` for thorough pass-2 fix verification; no terminal transition was made by the fixer.

## Review closure — pass 3

- Fresh-context adversarial pass 3 found no material current-cycle blockers.
- Confirmed bounded stored-suffix folding consumes ungated siblings, requires
  the exact committed report at the returned LSN, installs atomically, and
  preserves fail-closed rebuild behavior for invalid suffixes.
- Recurring finding: the post-append gap assumption recurred after pass 1
  because the first fix addressed evidence rather than writer interleaving; the
  pass-2 suffix-fold correction removed the root assumption, and pass 3 was
  clean.
- Effective weight: `thorough` (explicit operator). Verdict: approved.
