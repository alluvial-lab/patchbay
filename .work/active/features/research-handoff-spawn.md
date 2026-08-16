---
id: research-handoff-spawn
kind: feature
stage: done
tags: [adapter, protocol, v1]
parent: epic-public-product-contract
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
research_refs: [v1-control-plane-and-spawn, outpost-pi-pitfall-harvest]
created: 2026-08-08
updated: 2026-08-12
---

# Spawn — logical target + generation lifecycle (v1 must)

## Redesign status and authority

This body supersedes the 2026-08-12 first design. The consolidated five-reviewer gate at `.work/active/reviews/spawn-stride-adversarial-review-2026-08-12.md` found that design **not safe to implement**. This redesign treats that review as the binding current gate and resolves its spawn-side BLOCKERs 1–5, 7, and 8 plus the spawn-side MATERIAL findings. Pi-only BLOCKERs 9–10 and the Pi-only MATERIALs remain for the immediately following redesign of `research-handoff-pi-adapter-capability`; this feature defines only the clean shared contracts that redesign must consume.

The earlier, incompletely retained “five-BLOCKER” review is not counted as evidence. Its BLOCKERs 1–2 cannot be reconstructed from the repository, so its claimed closure is withdrawn. The current five-reviewer review is the traceable re-run/superseding gate; the disposition matrix below links every current spawn-side finding to a design section and child checkpoint.

## Brief

Wire Patchbay's committed `spawn` OperationKind and restart-as-spawn-continuation so the operator can spawn and restart Pi agents from Patchbay instead of herdr. A successful fresh spawn creates one stable logical target and runtime generation `1`. An intentional restart is a new `spawn` Operation with a new command id/key, an exact prior-generation reference, and a core-prepared `N+1` claim.

The core owns durable Operation state, logical identity, compound continuation authority, exclusive claim/fence state, generation monotonicity, external-runtime uniqueness, tombstones, quarantined stale evidence, staged successor evidence, atomic authority-bearing promotion, and claim reconciliation. The adapter owns target-spec interpretation, external process/session creation, quiesce/terminate/respawn, native continuation, external-effect evidence, and honest reconciliation within an explicitly stated trust boundary.

Project/cwd remains adapter-owned in v1. Core `spawn` carries an opaque typed `target_spec`; project, cwd, Pi session path, and labels are never logical identity, routing authority, or Grant scope.

## Grounding and current-code constraints

- The design consumes `.research/analysis/campaigns/v1-control-plane-and-spawn/parent.md` and its `spawn-lifecycle` / `pi-adapter-probe` facets. The research supports separating persisted logical context from live runtime, explicit continuation status, persisted-entry cursor repair, and process replacement as the Pi runtime-package upgrade boundary; the exact Patchbay contracts below are `{extends}` design, not claims about Pi.
- `.research/analysis/campaigns/outpost-pi-pitfall-harvest/parent.md` independently corroborates old-incarnation callbacks killing a successor and non-exclusive claims creating multiple consumers. Its keyring comparison is an analogous authority warning only, not direct descendant-grant evidence.
- `docs/ARCHITECTURE.md` and `docs/PROTOCOL.md` require accepted-before-delivery, one authority-domain LSN order, derived projections, source-authenticated reports, and no remembered-stream authority.
- `docs/SECURITY.md` requires deny-by-default grants, exact runtime-generation targeting, revocation, durable provenance, and authenticated adapter ingress. A source-authenticated report is still a trust assumption about the current adapter's correctness; authentication does not prove a dishonest or buggy adapter told the truth about external effects.
- Existing `core/src/authority/spawn_tail.rs` and `server/src/spawn_completion.rs` already own grant-before-completed behavior, but their staged audit → grant → completion prefixes predate logical-target promotion. They must be migrated, not bypassed.
- Existing `core/src/session/registry.rs` keys the live slot by adapter/deployment/runtime id and advances on an adapter report. Managed spawn must instead stage a claimed successor and publish it only through one promotion decision.
- Existing `core/src/acceptance/observation.rs` can persist a raw stale Observation before a separate audit. The redesign forbids that replay shape: quarantined evidence is an outer durable event kind that no normal projection dispatches as an Observation.

**Dispatch rationale:** direct-read redesign. The current gate already contains five independent fresh-context reviews. This delegated lane cannot spawn further agents without violating the recursion guard, so no required fan-out was silently degraded; the adversarial pre-mortem is run inline below as the user directed.

## Work-nature test

**Non-zero design surface; full feature-design lane.** This redesign changes a public authority contract, claim state machine, crash/retry semantics, replay envelope, promotion atomicity, runtime-identity uniqueness, cursor replacement contract, and implementation dependency graph. It is not a prose-only mapping. No UI mock is required because the surface reuses existing spawn/restart actions and canonical Operation/failure presentation.

## Design decisions

### 1. Continuation has compound authority, durably preserved

Fresh spawn still requires one live adapter-scoped Grant allowing `spawn`. Continuation requires **both**, evaluated under the same `CoreDecisionGate` and one sampled decision time:

1. the live adapter-scoped `spawn` Grant selected by the normal deterministic Grant rule; and
2. a live Grant for the same verified subject/endpoint and authority domain, scoped to the **exact prior runtime generation**, allowing `session-management` as the replacement authority.

The accepted envelope persists both selected Grant ids plus the exact prior reference. `authorizing_grant_id` remains the adapter-spawn grant; `ContinuationAuthorityProvenance` records the exact-generation replacement Grant. The descendant Grant provenance records both. A broad adapter spawn Grant cannot replace a protected target, and revoking the prior generation's session-management Grant prevents a later continuation from reviving it.

Because promotion creates future authority, the exact-prior replacement Grant must also remain live at promotion. Revocation/expiry before promotion suppresses descendant issuance and promotion even if generic already-accepted work might otherwise continue. Reauthorization cannot silently substitute another Grant id into the accepted provenance; reconciliation requires an explicit, audited operator action or target abandonment.

### 2. A successor is staged; one promotion decision makes it current

A managed fresh/successor `SessionReport` never directly mutates `SessionRegistry`. It becomes `SpawnSuccessorEvidenceStaged` only after exact claim/provenance classification. A successful Result is also evidence, not completion. The prior generation remains the current logical-target record (possibly offline/failed/stale and delivery-fenced) while the candidate is staged.

`SpawnPromotionCommitted` is one semantic durable decision consumed by authority, session, claim, and command projections. It contains or exactly references all required facts: accepted compound provenance, valid delivered/running lifecycle, successful Result, staged successor report, external-runtime uniqueness reservation, exact `N→N+1` (or fresh `∅→1`), completion audit linkage, and the new generation-scoped descendant Grant. The storage port atomically writes the promotion source and its audit, assigning/stamping both event ids inside one transaction. Replay sees neither or the complete promotion decision; no raw prefix can publish N+1 without authority.

The authority fold validates/installs descendant authority before the session fold exposes current/live state and before the command fold exposes `completed`. All folds consume the same event. Readers under the decision gate therefore observe either the pre-promotion state or the complete authority-bearing promotion.

### 3. Terminal command state does not by itself release a claim

`SpawnClaimDisposition` is independent from `CommandState`:

- `active` — accepted and exclusive; continuation also activates the delivery fence on N;
- `released_no_external_effect` — reusable only after a durable closed-vocabulary proof that no external effect occurred;
- `poisoned_pending_reconciliation` — an effect may exist, so the generation remains exclusively consumed;
- `promoted` — permanently consumed by the promotion;
- `target_abandoned` — permanently consumed and the logical target is durably retired.

`failed`, `cancelled`, or `expired` does **not** release a claim. Delivered cancellation/expiry, `execution_outcome_unknown`, launch-attempted loss, or any ambiguous external-effect evidence poisons it. A poisoned claim can later be promoted by reconciling the exact external runtime, or released only if later durable evidence proves no external effect. Otherwise the operator abandons the target. A retry with a new command/key cannot claim the same generation while the record is active or poisoned.

Closed no-effect proofs include: an atomic core-side terminal/fence decision before any durable delivery offer; an authenticated current-adapter refusal explicitly made before delivery responsibility; or current-adapter supervisor/journal evidence for the exact claim proving failure before external launch. The implementation must not infer “not delivered” merely from absence of a `delivered` acknowledgement.

### 4. The shared fence recognizes an exact claimed successor

`RuntimeGenerationDisposition` becomes:

- `Current`;
- `ClaimedSuccessor { claim_operation_id, expected_prior, claimed_generation }`;
- `Tombstoned { superseded_at_lsn }`;
- `Unknown`;
- `IdentityMismatch`.

`ClaimedSuccessor` is returned only for a current authenticated adapter report whose exact Operation id, claim, logical target, expected prior (`None` for fresh), adapter/deployment, and claimed generation match the durable active claim. That disposition routes the report to staging; it does not make the successor current and does not authorize ordinary candidate output. All other ingress uses the same classifier. There is no SessionReport-specific bypass around the fence.

### 5. Stale evidence is durably quarantined, not stored as an Observation

Tombstoned/unknown/mismatched runtime evidence that merits retention is persisted as a self-contained outer `QuarantinedRuntimeEvidence` event containing the original candidate in a generated admitted-family `oneof`, classified target, current/tombstone/claim context, reason, and source attachment evidence. Unknown/untyped candidate families reject rather than entering an opaque payload escape hatch. Its audit is committed atomically with that envelope. Normal Observation, transcript, Elicitation, command, session, completion, and authority projections dispatch only on the outer stored kind and never unwrap it as authoritative evidence.

A claimed-successor report is similarly persisted as the dedicated staged-evidence event. Thus replay never applies a raw Observation and only later discovers a stale audit.

### 6. External runtime references have one logical owner

The logical-target projection owns a reverse index from exact `(authority_domain_id, adapter_id, deployment_scope, runtime_session_id, generation)` to `logical_target_id`. Staging reserves the exact key; duplicate ownership fails before promotion and emits the `duplicate-native-reference` outcome/vector. Tombstones retain the reservation for audit and late-event correlation. Adapter-specific stronger native-continuity uniqueness (for example, one Pi persisted session selected by two differently numbered Patchbay generations) remains an adapter conformance obligation and may not weaken this core floor.

### 7. A pending continuation fences generation-N delivery

Acceptance of a continuation claim atomically creates a durable pending-replacement fence for the exact prior generation. The accepted-continuation decision carries the complete precomputed effects for already accepted N work, so claim/fence visibility cannot race separate supersession writes. New N-bound Operations reject before acceptance with canonical `superseded` plus reason `replacement_pending`; they are not held in an invisible queue. Previously accepted but not yet offered work is durably superseded by that decision. Delivered/running work is explicitly marked for quiesce/outcome reconciliation and cannot be redelivered after the fence. Exact retries still reconcile to their existing records.

The fence remains while the claim is active or poisoned. It clears only on durable no-effect release (after current-N liveness is re-established), promotion, or target abandonment.

### 8. Cursor loss requires authoritative projection replacement

The spawn-side cursor leaf defines an adapter-neutral `ExternalCursorScope` keyed by verified external continuity identity, **not Patchbay generation**. For Pi this will be the verified Pi session identity; the following Pi redesign chooses its exact local representation.

A known cursor may apply a suffix. An unknown cursor begins a staged replacement epoch: fetch/validate the complete external set/tree, build a replacement projection, compare exact membership/identity, and atomically install `{projection, leaf, cursor, epoch}`. Upserting a full fetch into an old projection is forbidden because omitted stale entries would survive. No cursor/leaf becomes current before the replacement projection commits.

### 9. Authenticated exact-claim evidence has a stated trust boundary

Patchbay proves that a report came through the current authenticated adapter attachment and exactly correlates to a durable claim. This prevents cross-operation, cross-generation, and stale-attachment confusion. It does **not** cryptographically prove the adapter observed the external runtime honestly. The reference adapter must therefore use a durable journal/supervisor, exact external identity, and mutation-sensitive conformance evidence. A malicious or sufficiently buggy authenticated adapter remains outside the core proof boundary and is handled by revocation/diagnostics, not by overstating “authenticated exact claim” as external truth.

## Architectural options

### Option A — Promote on SessionReport, repair authority afterward

Closest to the current registry. It leaves the reviewed crash window: N+1 can be current while its creating Operation failed or its descendant Grant is absent. **Rejected.**

### Option B — Separate session/grant/command events in an atomic batch

A transaction prevents crash truncation, but independent event kinds still expose intermediate LSN prefixes to projections and require every reader to understand batch boundaries. **Rejected** in favor of one semantic replay unit.

### Option C — Stage all successor evidence and commit one cross-projection promotion event

Claims, reports, Results, and crash evidence remain durable without being live authority. One `SpawnPromotionCommitted` event installs descendant authority, exact generation transition, claim consumption, and completion together. This minimizes authoritative states and makes replay atomicity structural. **Chosen.**

## Contract leaves (must land before operations)

### Leaf 1 — Logical-target and external-runtime identity

**Story:** `research-handoff-spawn-logical-target-identity-contract`

**Files:** `contracts/proto/patchbay/{common,sessions}.proto`, new `core/src/session/logical_target.rs`, session projection/replay/checkpoint tests.

```proto
message LogicalTargetId { string value = 1; }
message ExternalRuntimeRef {
  AdapterId adapter_id = 1;
  string deployment_scope = 2;
  RuntimeSessionId runtime_session_id = 3;
  Generation generation = 4;
}
message RuntimeGenerationRef {
  LogicalTargetId logical_target_id = 1;
  ExternalRuntimeRef external_runtime = 2;
}
```

Defines positive-generation validation, stable logical identity, current/tombstone/reserved-candidate identity slots, and the external-runtime reverse index without importing downstream claim/evidence types or accepting any Operation.

### Leaf 2 — Continuation payload and compound authority provenance

**Story:** `research-handoff-spawn-continuation-payload-authority-contract`

**Files:** `contracts/proto/patchbay/{operations,authority}.proto`, acceptance validation tests.

```proto
message SpawnRequest {
  oneof intent { FreshSpawn fresh = 1; SpawnContinuation continuation = 2; }
  SpawnTargetSpec target_spec = 3;
}
message SpawnContinuation { RuntimeGenerationRef prior = 1; }
message ContinuationAuthorityProvenance {
  RuntimeGenerationRef exact_prior = 1;
  GrantId replacement_grant_id = 2;
  OperationKind replacement_authority_kind = 3; // session-management
}
```

Defines one generated payload and durable two-Grant provenance carriage; the downstream claim leaf adds the claim/effect fields to the accepted envelope, and this leaf performs no target resolution.

### Leaf 3 — Claim registry and pending-replacement fence

**Story:** `research-handoff-spawn-claim-registry-contract`

**Files:** generated claim/event contracts, new `core/src/session/spawn_claim.rs`, replay/checkpoint/property tests.

```rust
pub struct SpawnClaimRecord {
    pub claim: SpawnGenerationClaim,
    pub accepted_lsn: u64,
    pub compound_authority: Option<ContinuationAuthorityProvenance>,
    pub disposition: SpawnClaimDisposition,
    pub pending_replacement: Option<RuntimeGenerationRef>,
}
```

Defines the exclusive key, disposition transitions, no-effect release rule, poison/reconciliation, delivery-fence query, and the accepted-continuation decision's explicit prior-work effects without delivering a spawn.

### Leaf 4 — Cursor authoritative-replacement contract

**Story:** `research-handoff-spawn-cursor-authoritative-replacement-contract`

**Files:** `contracts/proto/patchbay/adapter.proto`, new `operator-domain/src/reconciliation/external_cursor.ts` consuming generated types, contract tests.

```ts
export interface AuthoritativeCursorReplacement<Scope, Entry, Cursor, Leaf> {
  reconcileKnown(scope: Scope, cursor: Cursor): Promise<readonly Entry[]>;
  stageReplacement(scope: Scope): Promise<{ entries: readonly Entry[]; leaf: Leaf }>;
  commitReplacement(scope: Scope, replacement: ProjectionReplacement<Entry, Cursor, Leaf>): Promise<void>;
}
```

Defines scope-by-verified-external-identity and atomic exact-set replacement; Pi supplies the implementation in its redesign.

### Leaf 5 — Spawn execution/crash evidence

**Story:** `research-handoff-spawn-crash-external-effect-evidence-contract`

**Files:** `contracts/proto/patchbay/adapter_control.proto`, generated event contracts, core validation/fold tests.

Defines `SpawnExecutionPhase`, `ExternalEffectDisposition`, exact claim correlation, optional bounded external identity, and closed `NoExternalEffectProof` variants. This is the only input allowed to release or poison a claim after delivery begins.

### Leaf 6 — Runtime evidence and promotion envelopes

**Story:** `research-handoff-spawn-runtime-evidence-promotion-contract`

**Files:** `contracts/proto/patchbay/{common,observations,sessions,authority}.proto`, stored-event registry, storage port contract tests.

Defines `RuntimeGenerationDisposition`, `QuarantinedRuntimeEvidence`, `SpawnSuccessorEvidenceStaged`, and `SpawnPromotionCommitted`, including one atomic audited promotion append. No operation may consume these shapes before this leaf lands.

## Operational implementation units

### Unit 1 — Operation-aware target resolution and compound Grant decision

**Story:** `fleet-spawn-target-resolution` (historical id retained; body rewritten)

**Files:** `core/src/acceptance/{ports,pipeline}.rs`, `core/src/target.rs`, `core/src/authority/check.rs`, `server/src/{state,service}.rs`.

Resolve one explicit attached adapter. Fresh requires the selected spawn Grant. Continuation additionally resolves the exact current prior generation and selects its live session-management Grant under the same decision gate/time sample. Reject before acceptance if either half fails.

### Unit 2 — Atomic accepted claim and N-delivery fence

**Story:** `spawn-delivery-atomic-claim-idempotency-generation`

**Files:** acceptance pipeline/storage transaction, claim fold, delivery index/server tests.

Atomically deduplicate and append the accepted envelope containing claim + compound provenance. Competing distinct continuation claims cannot both append. The same event activates the exact-N pending-replacement fence and supersedes never-offered prior work before a delivery reader can offer it.

### Unit 3 — Claimed-successor staging and external-runtime reservation

**Story:** `research-handoff-spawn-logical-target-registration` (rewritten; old direct-live registration superseded)

**Files:** `core/src/session/{ingest,logical_target,spawn_claim}.rs`, `server/src/adapter_service.rs`.

Classify a first fresh/N+1 report as `ClaimedSuccessor`, reserve its external-runtime reverse key, and append staged evidence. It never calls ordinary live registration.

### Unit 4 — Promotion fold, exact monotonicity, and tombstones

**Story:** `research-handoff-spawn-generation-monotonicity-tombstoning`

**Files:** session/claim/command/authority folds, checkpoint/replay, `specs/seed/session_generation.qnt`.

Validate and fold `∅→1` / exact `N→N+1`; atomically tombstone N, install N+1 current, consume the claim, and preserve reverse-index/tombstone history. Independent attempted evidence and mutations cover strict pre-state and exclusivity.

### Unit 5 — Shared ingress fence and quarantine

**Story:** `research-handoff-spawn-stale-event-fencing`

**Files:** every runtime-targeted adapter ingress, quarantine append, transcript/Elicitation/ack paths, enumerate-first tests.

Use the shared classifier everywhere. Only exact successor reports stage; tombstoned/unknown/mismatched evidence is rejected or quarantined as an outer event and cannot mutate normal projections.

### Unit 6 — Duplicate, ambiguous-outcome, and claim reconciliation

**Story:** `research-handoff-spawn-idempotency-duplicate-handling`

**Files:** command/claim indexes, server redelivery, adapter execution-evidence port, reconciliation vectors.

Preserve exact boundary retry while poisoning ambiguous claims. Reconcile one known external runtime to its original claim or abandon; never auto-launch a replacement for a poisoned generation.

### Unit 7 — Atomic promotion completion driver

**Story:** `research-handoff-spawn-completion-promotion-driver`

**Files:** `server/src/spawn_completion.rs`, `core/src/authority/spawn_tail.rs`, storage atomic-promotion port/backend, driver and crash-prefix tests.

Migrate the current grant-before-completed owner. New managed spawns emit one audited promotion decision only after all evidence and exact prior replacement authority are valid. One-way replay migration handles legacy evidence-only, audit-only, audit+grant, and completed prefixes without treating them as the new managed shape or duplicating grants/terminals.

### Unit 8 — Adapter-local deployment authority

**Story:** `deployment-authority-workspace-scoped-revocable-keys`

Retains the adapter-local credential-reference boundary, but consumes the new continuation/claim evidence. It cannot substitute for either core Grant and must be rechecked per continuation.

### Unit 9 — Restart-as-continuation orchestration

**Story:** `research-handoff-spawn-restart-continuation-orchestration`

Consumes the completed shared contracts. It owns phase transitions and operator actions, not Pi subprocess/file details. The Pi feature supplies the concrete supervisor after its redesign.

### Unit 10 — Reconnect and cursor convergence

**Story:** `research-handoff-spawn-reconnect-cursor-reconcile`

Consumes the atomic replacement cursor contract and promotion event. Core replay, adapter external-state replacement, and surfaces converge without treating a remembered stream or stale upsert set as authority.

## Failure-phase connectivity and claim mapping

| Durable phase/evidence | Prior N | Candidate N+1 | Claim/fence outcome |
|---|---|---|---|
| authority/validation rejection before acceptance | unchanged | absent | no claim/fence |
| claim accepted, before any offer | current state retained but N delivery-fenced | absent | active; core-proven never-offered terminal may release atomically |
| quiesce begun, prior still running | `stale` or last confirmed connectivity; activity `unknown`; no new delivery | absent | active; release only after no-effect proof and renewed N liveness |
| prior terminated cleanly before launch | `offline`, activity `unknown`, still current/non-tombstoned | absent | active; may release only with durable proof launch never occurred |
| launch attempted, identity not durably known | N offline/failed/stale by evidence | unpublishable/unknown | poisoned; fence retained |
| external identity known, handshake/reconcile incomplete | N offline/failed/stale | staged, never live | active or poisoned by failure evidence |
| successor crashes before promotion | N remains current but unavailable as evidenced | staged failed evidence only | poisoned unless exact no-effect proof is later established |
| handshake + authoritative replacement reconcile + successful Result/report | N remains current and fenced until commit | staged and ready | active; completion driver may promote |
| atomic promotion committed | tombstoned at promotion LSN | current; `live` only if staged current evidence supports it | promoted; old fence consumed; descendant Grant durable |
| unexplained stream loss at any delivered/launch phase | `stale`/activity `unknown` as applicable | never inferred live | `execution_outcome_unknown`; poisoned |
| operator target abandonment | logical target retired; no revival | any candidate becomes audit-only | target_abandoned; claim permanently consumed |

No row allocates a generation on crash, detach, reconnect, timeout, or clean exit. No protocol `restarting` state is added.

## Validated bottom-up order

```text
logical-target-identity-contract
  ├─ cursor-authoritative-replacement-contract
  └─ continuation-payload-authority-contract
       └─ claim-registry-contract
            └─ crash-external-effect-evidence-contract
                 └─ runtime-evidence-promotion-contract

(all contract leaves complete)
  └─ fleet-spawn-target-resolution
       ├─ deployment-authority-workspace-scoped-revocable-keys
       └─ spawn-delivery-atomic-claim-idempotency-generation
            └─ logical-target-registration (staging only)
                 ├─ idempotency-duplicate-handling
                 └─ generation-monotonicity-tombstoning
                      └─ stale-event-fencing
                           + idempotency-duplicate-handling
                             └─ completion-promotion-driver
                                  + deployment-authority...
                                    └─ restart-continuation-orchestration
                                         └─ reconnect-cursor-reconcile
```

Every operation consumes an earlier contract leaf. The logical-target projection and claim registry are defined before target resolution; continuation does not depend on downstream Pi mechanisms; the cursor leaf is independent and adapter-neutral; the completion driver has explicit file ownership. The Pi feature remains downstream of the spawn feature and will consume these contracts rather than define them backward.

## Simplification and cleanup

- Replace direct managed `SessionRegistered`/`SessionGenerationBumped` publication with staged successor + one promotion event. Keep only explicit legacy replay normalization where real stored data requires it; do not run dual live semantics.
- Keep one claim projection folded from durable events; no mutable side table or adapter-local generation allocator becomes core authority.
- Keep one runtime-generation classifier and one quarantine envelope rather than ad hoc SessionReport/Observation exceptions.
- Extend the existing completion owner; do not add a second reactor or allow generic Result ingestion to terminalize spawn.
- Remove any claim-release rule derived solely from terminal `CommandState`.
- Reject new N work during replacement rather than introduce a hidden hold queue or a new protocol connectivity state.
- Keep Pi/project/cwd/native-cursor details behind adapter-neutral generated envelopes and adapter-owned profiles.

## Testing and assurance

- **Generated contracts:** Rust/TypeScript generation/drift checks for logical ids, exact runtime refs, continuation provenance, claim/evidence dispositions, quarantine, staged evidence, cursor replacement, and promotion.
- **Compound authority:** exact prior Grant absent/revoked/expired/wrong subject/wrong endpoint/wrong generation; adapter spawn Grant alone must fail. Mutation removing either half fails.
- **Claim concurrency/poison:** two distinct N+1 attempts; delivered cancellation/expiry; effect-before-ack; no-effect proof; poison reconciliation; target abandonment. No new claim/delivery while active or poisoned.
- **Promotion/crash:** result-first/report-first; evidence-only, audit-only, audit+grant legacy prefixes; new atomic source+audit crash; no public N+1 or completed state without descendant authority.
- **Claimed successor:** fresh generation 1 and exact N+1 first reports stage successfully; wrong Operation/expected prior/runtime/generation fail without an ad hoc bypass.
- **Quarantine/replay:** stale Observation/result/transcript/ack/Elicitation/report persists only in the outer envelope; replay mutation that dispatches its nested candidate fails.
- **External identity:** `duplicate-native-reference` reserves one exact external runtime owner across hot fold, restart replay, and tombstones.
- **Pending replacement:** a barrier race between continuation acceptance and N-bound instruct proves either instruct accepted before the fence and explicitly resolved, or rejected after it; never delivered after the fence.
- **Cursor replacement:** unknown cursor full fetch omitting a stale projected entry removes it in the atomic replacement; upsert-only and cursor-before-projection mutations fail.
- **Formal/release assurance:** extend genuine attempted-evidence properties for compound authority, exclusive/poisoned claim, atomic promotion, generation monotonicity, and stale inertness. Green implementation tests are not called model-checked or release-verified without promotion.

## Adversarial pre-mortem

### Forced adversary 1 — authority laundering

**Attack:** obtain adapter-wide spawn authority, target a protected/revoked generation, then rely on the new descendant Grant to regain control.

**Defense:** exact-prior session-management Grant at acceptance and still-live check at promotion; both ids and target preserved in accepted/descendant provenance. A descendant Grant cannot cite only the broad spawn Grant for continuation.

### Forced adversary 2 — crash between “session exists” and “grant exists”

**Attack:** crash after SessionReport application but before descendant issuance, then replay N+1 as current without authority.

**Defense:** report stages only; one promotion event is the first source that any projection may interpret as current/authority/completed. Atomic source+audit append and one replay unit remove the prefix.

### Forced adversary 3 — ambiguous failure creates two runtimes

**Attack:** launch N+1, lose response, terminalize failed/cancelled/expired, release claim, and accept another N+1.

**Defense:** terminal state cannot release. Launch/delivery ambiguity poisons the exact generation until reconciliation or target abandonment.

### Forced adversary 4 — first successor report is rejected or bypasses the fence

**Attack:** exploit `Unknown` fail-closed behavior to make all fresh reports impossible, pressuring implementation into a hidden SessionReport exception.

**Defense:** `ClaimedSuccessor` is part of the shared classifier and validates exact durable Operation provenance; it routes only to staging. Enumerate-first tests prohibit another path.

### Forced adversary 5 — stale raw event replays as current

**Attack:** append raw Observation, later append stale audit, then let a transcript/completion projection consume the raw record before seeing the audit.

**Defense:** only the outer quarantine kind is durable. The nested candidate is diagnostic evidence and cannot be dispatched by normal replay.

### Additional material adversaries

- **Duplicate native runtime:** reverse index rejects a second logical owner before staging/promotion.
- **Work enters dying N:** the accepted claim activates a delivery fence in the same durable decision; new N work rejects, and pre-existing work is explicitly resolved.
- **Phase ambiguity:** the table above maps every orchestration phase to N/N+1 connectivity and claim outcome; no “restart failed” catch-all invents liveness.
- **Dishonest adapter:** exact authentication/correlation prevents confusion but not lies; this limitation is explicit and Pi must prove its journal/supervisor behavior through conformance.
- **Cursor truncation:** unknown cursor requires exact-set replacement, not upsert, before installing the new cursor.

### Riskiest assumption and fallback

The riskiest unit is the cross-projection promotion event and storage append that stamps an audit link while remaining one semantic replay decision. If the current storage abstraction cannot implement that atomically, implementation must stop at a spike and extend the storage port; it must **not** fall back to publishing N+1 first or to independent event prefixes. The safe fallback is to leave the claim active/poisoned, N current but stale/offline, and the candidate staged for operator reconciliation.

## Review traceability

| Review finding | Resolution | Owning child checkpoint(s) |
|---|---|---|
| BLOCKER 1 compound continuation authority | two live Grants, exact prior, accepted + descendant provenance, promotion-time liveness | continuation contract; target resolution; completion driver |
| BLOCKER 2 premature promotion | staged report + one atomic authority/session/claim/command promotion | promotion contract; generation fold; completion driver |
| BLOCKER 3 ambiguous failure releases claim | independent claim disposition; closed no-effect proof; poison/reconcile/abandon | claim contract; crash evidence; duplicate handling |
| BLOCKER 4 no legitimate successor disposition | shared `ClaimedSuccessor` exact Operation/claim classifier | promotion contract; staging; stale fence |
| BLOCKER 5 raw stale Observation | outer quarantine envelope + atomic audit; no raw authoritative record | promotion contract; stale fence |
| BLOCKER 7 hidden cycles | six early contract leaves + bottom-up graph above | all contract leaves/dependency metadata |
| BLOCKER 8 no completion owner | explicit child owns both named files and crash-prefix migration | completion-promotion-driver |
| MATERIAL duplicate native reference | core reverse index + vector | identity contract; staging |
| MATERIAL N work during replacement | durable pending-replacement acceptance/delivery fence | claim contract; atomic claim |
| MATERIAL phase connectivity | explicit table and typed crash phase/effect evidence | crash evidence; restart orchestration |
| MATERIAL “authenticated exact claim” overclaim | explicit adapter-honesty trust assumption | continuation/crash contracts; conformance |
| MATERIAL prior-review traceability | prior gate disclaimed; current five-reviewer gate is traceable re-run | this section + cited review |

## UI fallback / Mockups

No net-new screen or journey. Fresh spawn remains an existing entry action and continuation/restart remains a session-detail action. The existing canonical Operation lifecycle, failure, retry-risk, stale/offline, Grant, and audit presentation handles the new semantics. `replacement_pending` is a bounded reason over canonical `superseded`, not a new presentation state.

## Extension pressure classification

- **Committed v1.0.0:** stable logical target; generation 1; exact typed continuation; compound adapter-spawn + exact-prior session-management authority; durable two-Grant provenance; exclusive poison-retaining claim; pending-replacement fence; exact external-runtime reverse index; `ClaimedSuccessor`; quarantined stale evidence; staged successor; atomic authority-bearing promotion; adapter-neutral authoritative cursor replacement; explicit crash/effect phases; authority-before-completion.
- **Reserved seams:** cross-adapter/deployment target migration, per-spawn-variant OperationKinds/authority, fleet selection, stronger hardware/remote attestation of adapter truth, automatic reconciliation policy, HA/multi-core claims, adapter-specific native-continuity uniqueness above the exact core tuple, and core `ProjectRef`.
- **Explicitly rejected for this v1 arc:** continuation authorized by adapter spawn Grant alone; inheritance/revival of prior authority; direct SessionReport promotion; release-on-terminal; reuse of poisoned generation; raw stale Observation plus later audit; hidden N-work queue; upsert-only unknown-cursor recovery; adapter-local generation allocation; project/cwd/native path as core identity.

The parked multi-human, mesh, desktop, and skin ideas remain pressure-test inputs only. Authority-domain qualification, explicit Grant provenance, generated contracts, and surface-neutral state presentation preserve those seams without implementing them.

## Child stories

The authoritative dependency metadata lives in each child file; the list is repeated here for reviewability after the re-slice.

**Contract leaves**
- `research-handoff-spawn-logical-target-identity-contract`
- `research-handoff-spawn-continuation-payload-authority-contract`
- `research-handoff-spawn-claim-registry-contract`
- `research-handoff-spawn-cursor-authoritative-replacement-contract`
- `research-handoff-spawn-crash-external-effect-evidence-contract`
- `research-handoff-spawn-runtime-evidence-promotion-contract`

**Operations**
- `fleet-spawn-target-resolution`
- `spawn-delivery-atomic-claim-idempotency-generation`
- `research-handoff-spawn-logical-target-registration`
- `research-handoff-spawn-generation-monotonicity-tombstoning`
- `research-handoff-spawn-stale-event-fencing`
- `research-handoff-spawn-idempotency-duplicate-handling`
- `research-handoff-spawn-completion-promotion-driver`
- `deployment-authority-workspace-scoped-revocable-keys`
- `research-handoff-spawn-restart-continuation-orchestration`
- `research-handoff-spawn-reconnect-cursor-reconcile`
