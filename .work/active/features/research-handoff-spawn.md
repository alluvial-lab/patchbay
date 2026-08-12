---
id: research-handoff-spawn
kind: feature
stage: implementing
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

## Brief

Wire Patchbay's committed `spawn` OperationKind and restart-as-spawn-continuation so the operator can spawn and restart Pi agents from Patchbay instead of herdr. A successful fresh spawn creates one stable logical target and runtime generation `1`. Restart is a new accepted `spawn` Operation that claims the next generation on that same logical target, references the exact prior runtime generation, and leaves the prior generation tombstoned before the replacement can be presented as live.

The core owns durable Operation state, logical identity, authority, claim exclusivity, generation monotonicity, tombstones, and stale-event fencing. The adapter owns target-spec interpretation, external process/session creation, quiesce/terminate/respawn, Pi native-session continuation, adapter-local cursor reconciliation, and honest continuation proof.

Project/cwd remains adapter-owned in v1: core `spawn` carries an opaque typed `target_spec`; there is no universal `Project`/`Workspace` entity and cwd/project/name never become routing or grant identity.

## Grounding and current-code constraints

- Research: `.research/analysis/campaigns/v1-control-plane-and-spawn/parent.md`, especially `spawn-lifecycle` and `pi-adapter-probe`, separates persisted logical context from live runtime and bounds Pi `get_entries(since)` to persisted-entry reconciliation.
- Field corroboration: `.research/analysis/campaigns/outpost-pi-pitfall-harvest/parent.md` records old-action-kills-successor and non-exclusive-claim failures that directly corroborate the spawn review's stale-generation and exclusive-claim blockers; its descendant-authority comparison is explicitly analogous, not direct field evidence.
- Durable truth: `docs/PROTOCOL.md` and `docs/ARCHITECTURE.md` require accepted-before-delivery, one authority-domain LSN order, derived snapshots, source-authenticated adapter reports, and no remembered-stream authority.
- Authority: `core/src/authority/spawn_tail.rs` and `server/src/spawn_completion.rs` already defer spawn terminal success until the exact accepted grant, delivered/running lifecycle, successful result, correlated session fact, completion audit, and descendant grant are durable. The redesign extends that fold; it does not bypass or replace it.
- Acceptance: `core/src/acceptance/pipeline.rs` validates → authorizes → resolves → atomically deduplicates/appends. `core/src/target.rs` already implements the committed adapter-scoped spawn target, despite the staged child's stale fleet wording.
- Sessions: `core/src/session/registry.rs` requires positive generations and atomically tombstones a same-runtime-id generation bump, but its stable slot is still `(adapter, deployment, runtime_session_id)` and cannot represent a continuation whose external runtime id changes.
- Ingress gap: `core/src/acceptance/observation.rs` checks an Observation against its command's original target but does not consult the live/tombstoned session projection, so an old-generation result can still mutate command state after replacement.
- Pi gap: `pi-adapter/src/delivery.ts` rejects `spawn`; `pi-adapter/src/main.ts` only preprovisions sessions; `PiSession` already uses positive generation `1` and binding-local callbacks, but its `newSession()` chooses the next generation inside the adapter instead of consuming a core claim.

**Dispatch rationale:** direct-read only. The relevant paths and prior five-BLOCKER fresh-context design review were already concrete; spawning another explorer would duplicate evidence. The prior review supplies the independent advisory pass, and this design explicitly answers its lifecycle blockers.

## Work-nature test

**Non-zero design surface; full feature-design lane.** This work chooses identity, state-machine, authority, concurrency, error, reconnect, and wire-contract semantics. It is not transcription/config-as-prose. The three open forks materially affect public v1 protocol semantics and are resolved below. The previously decided adapter-owned project/cwd seam is retained rather than re-opened.

## Design decisions

### Fork resolutions

- **Initial generation: `1`.** Generation zero remains the generated-contract `UNSPECIFIED`/invalid sentinel. Existing `SessionRegistry`, checkpoint validation, session-report ingress, and `PiSession` all require a positive generation, and the formal generation vocabulary already treats positive values as real incarnations. Choosing `0` would require a second validity rule at every boundary and make missing/zero evidence ambiguous. This is evidence-decided, not operator-level.
- **Restart shape: a new `spawn` Operation with a typed continuation payload.** Every intentional restart receives a new command id and idempotency key, preserving accepted-intent accountability and first-terminal semantics. The payload's `SpawnContinuation` names the exact logical target and prior runtime generation; the prepared accepted claim names exactly the next generation. No new `restart` OperationKind is added, and restart is not hidden inside generic `session-management`. “New Operation” and “typed continuation” are complementary: the former is lifecycle identity; the latter is the spawn intent variant.
- **Crash state: explicit crash → session connectivity `failed`; activity `unknown`.** `failed` already means an explicit adapter/session error preventing reliable control. `stale` is reserved for loss of sufficiently fresh authority (including unexplained stream loss), and `offline` for authoritative clean/unavailable endpoint evidence. “Unavailable” is not a session-connectivity registry member and therefore is not introduced as a new state. A crash never increments generation, tombstones the target, or auto-restarts it. Running work resolves to `failed(execution_outcome_unknown)` when outcome is unprovable.

### Additional binding decisions

- **Logical target is stable identity; runtime id may change.** V1 binds a logical target to one authority domain, adapter, and deployment scope. Cross-adapter/deployment migration is reserved. Runtime session id belongs to a generation reference and can change on `new_context` continuation.
- **Core prepares generations; adapter reports exact claims.** A fresh accepted spawn prepares generation `1`; a continuation prepares exact `N+1`. The adapter cannot infer or allocate another number. An authenticated report becomes authoritative only when it echoes the accepted claim and exact spawn Operation provenance.
- **Accepted Operation is the exclusive generation claim.** No second mutable claim store or checkpoint becomes authority. A claim projection folds accepted spawn envelopes and terminal/generation events from the durable log. The shared `CoreDecisionGate` covers catch-up, claimability check, and accepted append.
- **Authority precedes public completion.** The existing completion driver remains the sole terminal-success owner. It issues a new generation-scoped descendant grant for a successful continuation before committing `completed`; the prior generation's descendant grant remains independently revocable/auditable and does not silently authorize the replacement.
- **No silent liveness inference.** Endpoint detach does not retire or increment a generation. Adapter stream loss degrades to stale. Explicit process crash reports failed. Reconnect must reconcile core LSN state and adapter-native Pi entry state before live presentation.
- **Adapter-owned deployment authority stays subordinate.** An optional opaque `deployment_authority_ref` may let the Pi adapter resolve a local expiring/revocable launch credential. It never becomes a core Workspace, a bearer payload, or a substitute for Patchbay Grants.

## Architectural options

### Option A — Treat `runtime_session_id` as the logical target

Keep the current `SessionRegistry` key and bump generation in place. This minimizes code, but it fails the grounded `new_context` case where continuation legitimately creates a different Pi/external session id. It also leaves the operator's stable target coupled to an adapter-native runtime identifier. **Rejected.**

### Option B — Make logical target the primary session-registry slot, with runtime-generation reverse index

Add a generated `LogicalTargetId`; store one current `RuntimeGenerationRef` plus retained tombstones under it; index exact runtime generations for routing/correlation. Fold the same durable session events into both views inside one registry. Accepted spawn envelopes carry the prepared claim. This requires a deliberate contract/projection migration but keeps one durable truth and supports changing runtime ids. **Chosen.**

### Option C — Let each adapter own logical identity and report whichever generation is current

This is smallest at the core, but it makes the adapter authoritative for merge/replacement and cannot prevent two continuation attempts from both creating runtimes. It reproduces the non-exclusive-generation-claim blocker and weakens wrong-session protection. **Rejected.**

## Core contract

`contracts/proto/patchbay/common.proto` adds distinct wrappers and qualifies runtime targets with logical identity:

```proto
message LogicalTargetId { string value = 1; }

message RuntimeGenerationRef {
  LogicalTargetId logical_target_id = 1;
  RuntimeSessionId runtime_session_id = 2;
  Generation generation = 3;
}
```

`contracts/proto/patchbay/operations.proto` owns the core-readable outer spawn envelope while leaving the target body adapter-owned:

```proto
message SpawnRequest {
  oneof intent {
    FreshSpawn fresh = 1;
    SpawnContinuation continuation = 2;
  }
  SpawnTargetSpec target_spec = 3;
}
message FreshSpawn {}
message SpawnContinuation { RuntimeGenerationRef prior = 1; }
message SpawnTargetSpec {
  string shape = 1;
  PayloadEnvelope adapter_payload = 2;
  string deployment_authority_ref = 3;
}
message SpawnGenerationClaim {
  LogicalTargetId logical_target_id = 1;
  RuntimeGenerationRef expected = 2; // absent for fresh
  Generation claimed_generation = 3; // 1 or expected + 1
  CommandId spawn_operation_id = 4;
}
```

The payload envelope itself must be generated Protobuf with the spawn schema. Core validates presence, exact typed continuation references, positive/overflow rules, and bounded opaque fields; it does **not** decide whether `target_spec.shape` is supported. The adapter reports unsupported shapes after acceptance as `unsupported_command`.

`AcceptedOperation`, `SubmissionResult`, and adapter `Delivery` carry the prepared `SpawnGenerationClaim`. An exact retry returns the persisted claim rather than a newly prepared value.

`contracts/proto/patchbay/sessions.proto` adds logical target/provenance to reports, events, and snapshots and replaces the same-runtime-only generation bump with an exact from/to advance:

```proto
enum ContinuationStatus {
  CONTINUATION_STATUS_UNSPECIFIED = 0;
  CONTINUATION_STATUS_RESUMED = 1;
  CONTINUATION_STATUS_NEW_CONTEXT = 2;
  CONTINUATION_STATUS_UNKNOWN = 3;
}

message LogicalTargetGenerationAdvanced {
  LogicalTargetId logical_target_id = 1;
  RuntimeGenerationRef from = 2;
  RuntimeGenerationRef to = 3;
  CommandId spawn_operation_id = 4;
  ContinuationStatus continuation_status = 5;
  SessionState initial_state = 6;
  SessionReportSourceCursor source_cursor = 7;
  // report-carried metadata follows; metadata is not identity
}
```

Fresh registration and continuation advance carry exact accepted-spawn provenance. Legacy/preprovisioned discovery remains an explicit authenticated registration path, not an implicit continuation.

## Trickiest unit first: exclusive claim → report → authority completion

The load-bearing path crosses three independently arriving durable facts: accepted spawn/claim, successful result, and authenticated session registration/advance. The implementation must tolerate either result/report order without allowing any one fact to terminalize success or grant authority.

1. Under `CoreDecisionGate`, submission catches up the claim/target/command projections.
2. Target resolution validates adapter scope and prepares a fresh or exact continuation claim.
3. Grant check authorizes adapter-scoped `spawn`; acceptance durably appends the Operation + claim before delivery.
4. Delivery carries that exact claim. The Pi adapter journals receipt before external create/continue.
5. Session report ingress authenticates adapter attachment/source cursor and requires exact claim/provenance. For continuation it appends one atomic generation-advance/tombstone decision.
6. Successful Result remains deferred evidence. It cannot itself complete or redeliver.
7. `SpawnCompletionDriver` folds the complete prefix, verifies accepted grant/sender/target/claim, delivered/running, result, and registration/advance, then writes completion audit → new descendant grant → completed transition.
8. Crash-prefix repair uses the same fold before listeners bind.

A failure at any step leaves an explicit accepted/failed/unknown record. There is no in-memory latch and no auto-allocation fallback.

## Implementation units and child checkpoints

### Unit 1 — Operation-aware adapter target and typed claim preparation

**Story:** `fleet-spawn-target-resolution`

**Files:** `contracts/proto/patchbay/operations.proto`, `core/src/acceptance/ports.rs`, `core/src/target.rs`, `core/src/acceptance/pipeline.rs`, `server/src/state.rs`.

- Preserve committed explicit attached-adapter resolution; correct the historical fleet wording.
- Resolve using the complete Operation so fresh/continuation payload and target are one boundary decision.
- Reject incompatible target kinds before durable acceptance; never broadcast.

### Unit 2 — Stable logical-target registration

**Story:** `research-handoff-spawn-logical-target-registration`

**Files:** `contracts/proto/patchbay/{common,sessions}.proto`, new `core/src/session/logical_target.rs`, `core/src/session/{registry,ingest}.rs`, `core/src/target.rs`, operator model/display files.

- Re-key the live slot by logical target; retain exact runtime-generation reverse index.
- Fresh spawn registers generation `1` only after exact claim-correlated report.
- Carry logical/current identity through snapshots and target-before-intent surfaces.

### Unit 3 — Exact monotonic advance and tombstones

**Story:** `research-handoff-spawn-generation-monotonicity-tombstoning`

**Files:** `contracts/proto/patchbay/sessions.proto`, `core/src/session/{events,ingest,registry,logical_target}.rs`, `server/src/adapter_service.rs`, `specs/seed/session_generation.qnt`.

- Require exact managed `N → N+1`, allow runtime id change, install tombstone/current atomically.
- Resolve old accepted work explicitly and preserve replay/checkpoint semantics.

### Unit 4 — Stale-event fence inventory

**Story:** `research-handoff-spawn-stale-event-fencing`

**Files:** `core/src/acceptance/{ports,observation,elicitation}.rs`, `core/src/adapter/mod.rs`, `server/src/adapter_service.rs`, `pi-adapter/src/pi_session.ts`.

- One runtime-generation classifier protects every runtime-targeted adapter ingress.
- Tombstoned evidence is durable audit only and cannot mutate command/session/Elicitation/transcript/authority state.

### Unit 5 — Atomic continuation claim

**Story:** `spawn-delivery-atomic-claim-idempotency-generation`

**Files:** accepted/delivery generated contracts, `core/src/session/logical_target.rs`, `core/src/acceptance/pipeline.rs`, `server/src/{state,service,adapter_service}.rs`.

- Accepted Operation is the durable exclusive claim.
- Competing distinct continuations cannot both be accepted/delivered for the same expected generation.

### Unit 6 — Retry and external duplicate honesty

**Story:** `research-handoff-spawn-idempotency-duplicate-handling`

**Files:** `core/src/acceptance/{pipeline,index}.rs`, `core/src/storage/port.rs`, `server/src/adapter_service.rs`, new `pi-adapter/src/spawn_journal.ts`.

- Preserve exact boundary dedup and return existing claim/state.
- Journal adapter execution where possible; otherwise surface `execution_outcome_unknown` and declared retry strength.

### Unit 7 — Pi restart-as-continuation orchestration and operator actions

**Story:** `research-handoff-spawn-restart-continuation-orchestration`

**Files:** new `pi-adapter/src/spawn_supervisor.ts`, `pi-adapter/src/{delivery,pi_session,session_registry,core_client,main}.ts`, `web-cockpit/src/{main,ui/session-detail}.ts`, new `cli/src/commands/spawn.ts`, `cli/src/main.ts`, and rolling foundation docs.

- New spawn Operation + typed continuation; no restart kind.
- Explicit Pi session selection, quiesce/terminate/respawn, persisted-entry reconcile, and honest continuation status.
- Reuse existing session-list/detail actions and delivery presentation; no new screen.

### Unit 8 — Reconnect/cursor convergence

**Story:** `research-handoff-spawn-reconnect-cursor-reconcile`

**Files:** new `pi-adapter/src/cursor_store.ts`, `pi-adapter/src/{spawn_supervisor,pi_session}.ts`, `core/src/session/{registry,replay}.rs`, `server/src/{snapshot,adapter_service}.rs`, `web-cockpit/src/domain/{reconcile,model}.ts`.

- Keep Pi entry cursor and core LSN cursor distinct.
- Unknown Pi cursor forces full resync; remembered stream never implies live.

### Unit 9 — Adapter-local deployment authority

**Story:** `deployment-authority-workspace-scoped-revocable-keys`

**Files:** generated spawn target spec, new `pi-adapter/src/deployment_authority.ts`, `pi-adapter/src/{spawn_supervisor,main}.ts`, `docs/SECURITY.md`.

- Opaque reference only; protected adapter-local credential resolution, expiry/revocation/scope checks, and canonical redaction.
- No core Workspace/Project authority or second Grant mechanism.

## Implementation order and dependency graph

```text
fleet-spawn-target-resolution
  └─ research-handoff-spawn-logical-target-registration
       └─ research-handoff-spawn-generation-monotonicity-tombstoning
            ├─ research-handoff-spawn-stale-event-fencing
            └─ spawn-delivery-atomic-claim-idempotency-generation
                 └─ research-handoff-spawn-idempotency-duplicate-handling

research-handoff-spawn-stale-event-fencing
  + research-handoff-spawn-idempotency-duplicate-handling
    └─ research-handoff-spawn-restart-continuation-orchestration
         ├─ research-handoff-spawn-reconnect-cursor-reconcile
         └─ deployment-authority-workspace-scoped-revocable-keys
```

One feature owner remains the baseline because contracts, core projections, server gate, Pi adapter, and surfaces share one invariant chain. Stories are durable design/verification checkpoints, not one-worker-per-file assignments.

## Simplification and cleanup

- Replace the current same-runtime-id `SessionGenerationBumped` assumption with one logical-target transition rather than maintaining parallel “legacy” and “v1” lifecycle APIs. Preserve substantial durable data through an explicit one-way migration/replay normalization where required; do not run dual live semantics.
- Remove Pi adapter-owned `current + 1` generation allocation from `PiSession.newSession()`; it must consume the core claim.
- Consolidate runtime-generation classification in one port instead of repeating generation comparisons across SessionReport, Observation, acknowledgement, transcript, and Elicitation ingress.
- Extend the existing `SpawnCompletionDriver`; do not add a second completion/reactor or let adapter Result terminalize directly.
- Reuse canonical `SessionConnectivityState`, `CommandState`, failure vocabulary, and existing web delivery components. Do not add `unavailable`, `restarting`, or `continued` protocol states.
- Correct stale fleet terminology in the staged target-resolution story/tests/docs while preserving the reserved fleet seam.

## Testing and assurance

Smallest useful surface, organized by risk rather than file count:

- **Interface/contracts:** generated Rust/TypeScript compile and drift checks for spawn envelope, claim, logical id, runtime reference, continuation status, snapshot, and Delivery carriage.
- **Acceptance/authority:** real Submit → adapter delivery → report/result in either order → audit → descendant grant → completed, plus crash repair at evidence-only, report-only, audit-only, grant-only, and complete prefixes. Protects authority-before-public-completion and the restart descendant-grant blocker.
- **Concurrency regression:** barrier race between two distinct continuation Operations at expected generation N; only one accepted claim/delivery. Mutation removes claim check and must fail.
- **Generation property/model:** independent attempted claim/report evidence for initial-one, exact N+1, monotonic current generation, atomic tombstone/current exclusivity, and lower/equal inertness. Promote under the project's deep verification lane with mutation-survivable oracles.
- **Ingress inventory:** enumerate every runtime-targeted adapter ingress and submit generation N after N+1. Protects the known stale Observation gap rather than testing only SessionReport.
- **Adapter E2E:** fresh Pi spawn, native resume, new-context fallback, duplicate delivery, effect-before-response-loss, journal unavailable, explicit crash, and old callback after replacement.
- **Reconnect E2E:** detach without retirement; stream loss during generation advance; core restart/checkpoint; Pi known-cursor suffix and unknown-cursor full resync; web/CLI converge to the same target/generation.
- **Surface:** target-before-intent, canonical continuation status, failed/stale distinction, idempotency-strength retry warning, and unsupported adapter action.
- **Vectors:** implement the research candidates `spawn-continuation`, `detach-does-not-retire`, `crash-before-ack`, `restart-native-resume`, `restart-shape-only`, `reconnect-after-stream-loss`, `duplicate-continuation`, `stale-generation-event`, `equal/lower-generation-report`, `duplicate-native-reference`, and `project-cwd-boundary` through the single vector registry.
- **Test removal:** retire same-runtime-id-only bump fixtures and any test that treats generation allocation inside Pi adapter state as correct. Keep legacy durable-event decode tests only where real stored data requires migration coverage.

## Pre-mortem and risks

### Riskiest assumption

That three separately arriving facts—accepted claim, session report, successful result—can be composed without a visibility window or a second authority. The existing completion driver proves the shape is feasible, but logical-target registration and generation effects add more cross-projection state.

**Mitigation:** keep one decision gate, one durable prefix, one claim projection, and one completion owner. Append generation/tombstone plus old-generation command/Elicitation effects as one audited decision. Rebuild/repair from the same fold before listeners bind.

### Production failure modes attacked

1. **Old generation mutates successor.** A delayed result/ack/transcript callback reaches a generic ingress that checks command target but not tombstone. Mitigation: shared enumerate-first `RuntimeGenerationFence`; dedicated stale-event story.
2. **Two restarts create two runtimes.** Different keys bypass boundary dedup and both compute N+1. Mitigation: accepted Operation as exclusive claim under reconciled `CoreDecisionGate`; adapter consumes persisted claimed generation; competing-claim mutation test.
3. **Authority is stranded or widened.** Gen N+1 becomes live before a descendant grant, or silently inherits gen N authority. Mitigation: completion audit → new generation-scoped descendant grant → completed; no grant inheritance, old/new independently revocable.
4. **Crash looks like restart.** Stream loss or process exit silently allocates N+1. Mitigation: explicit crash = failed, unexplained loss = stale, clean unavailable = offline; only a new accepted continuation may claim a generation.
5. **Duplicate external process after response loss.** Core boundary dedup cannot prove adapter execution. Mitigation: durable adapter journal/external identity reconciliation where possible; otherwise `execution_outcome_unknown` and no automatic relaunch.
6. **Cursor says live when only history is known.** Pi entry suffix or remembered stream is treated as process liveness. Mitigation: separate cursor authorities; full resync on unknown Pi cursor; live requires current authenticated evidence.
7. **Project/cwd becomes routing authority.** Convenience target metadata or deployment credential widens scope. Mitigation: adapter-owned target spec/reference, canonical adapter Grant first, no core Project/Workspace.
8. **Green tests miss async lifecycle leaks.** Late callback errors occur after assertions. Mitigation: E2E waits for observation/journal/process cleanup and fails on unhandled async errors, following the pitfall harvest.

### Fallback if the riskiest unit fails

Fail closed without replacing the current generation. Keep the accepted spawn visible as failed or `execution_outcome_unknown`, keep the logical target failed/stale, and require explicit operator reconciliation or an intentional fresh spawn with a new logical target. Do not fall back to automatic generation allocation, in-place reload, ambiguous `--continue`, or optimistic live state.

### Least-certain boundary

Pi effect-before-ack recovery: after an external process/session is created but before its identity is durably journaled/reported, neither the core log nor Pi session history necessarily proves whether replay is safe. The design therefore does **not** promise exactly-once spawn. `idempotency_strength` stays honest and the ambiguous outcome is operator-visible unless implementation evidence proves end-to-end dedup.

### Prior review blocker disposition

The persisted review summary identifies BLOCKERs 3–5 by number; the operator brief additionally names authority-before-delivery and no silent generation allocation as load-bearing review risks. This design does not invent labels for the two review findings whose full text is not retained in the item.

- **BLOCKER 3 (stale Observation ingress):** dedicated shared-fence checkpoint across all runtime ingress.
- **BLOCKER 4 (no exclusive generation claim):** durable accepted claim + decision-gate race test; boundary dedup alone is explicitly insufficient.
- **BLOCKER 5 (restart strands descendant authority):** existing completion fold extended so every continuation issues a new generation-scoped descendant grant before completion.
- **Authority-before-delivery/completion:** grant check and claim preparation precede accepted append/delivery; completion audit and descendant grant precede public terminal success.
- **No silent generation allocation:** only accepted claims prepare generation `1`/`N+1`; adapter reports must echo the claim; crash, timeout, reconnect, and stream loss never advance generation.

## UI fallback / Mockups

No net-new screen or navigation flow is introduced. Fresh spawn is an entry action in the existing session-list empty/action area; restart is a session-detail header action; both reuse canonical Operation delivery/failure components and the existing responsive shell. The parent epic intentionally has no mockup plan, and this is minor composition rather than a new surface, so feature-level mockups are skipped.

## Extension pressure classification

- **Committed v1.0.0:** one adapter-scoped `spawn` kind; stable logical target; positive initial generation `1`; typed fresh/continuation intent; exact N→N+1 managed continuation; generation tombstones/stale-event fence; explicit crash→failed mapping; core-LSN/Pi-entry cursor reconciliation; authority-before-completion; adapter-owned target spec; honest idempotency strength.
- **Reserved seams:** fleet-default selection, cross-adapter/deployment logical-target migration, core `ProjectRef`, per-spawn-variant OperationKinds/authority, automatic continuation fallback policy, stronger native-state proof, heartbeat freshness policy, end-to-end spawn idempotency for adapters that cannot prove it, HA/multi-core generation claims, and multi-human descendant authority.
- **Explicitly rejected for this v1 arc:** generation zero as a real incarnation; broadcast spawn; cwd/project/label as core identity or grant authority; a separate `restart` OperationKind; restart hidden in `session-management`; arbitrary process-state restoration claims; crash/reconnect allocating a generation; treating `unavailable` as a new session state; inheriting prior-generation descendant authority; or rendering remembered streams/cursors as live.

The parked multi-human, mesh, desktop, and skin ideas do not become requirements here. Logical target and authority-domain qualification preserve their future seams without implementing them.

## Child stories

- `fleet-spawn-target-resolution` — `depends_on: []`
- `research-handoff-spawn-logical-target-registration` — `depends_on: [fleet-spawn-target-resolution]`
- `research-handoff-spawn-generation-monotonicity-tombstoning` — `depends_on: [research-handoff-spawn-logical-target-registration]`
- `research-handoff-spawn-stale-event-fencing` — `depends_on: [research-handoff-spawn-generation-monotonicity-tombstoning]`
- `spawn-delivery-atomic-claim-idempotency-generation` — `depends_on: [fleet-spawn-target-resolution, research-handoff-spawn-generation-monotonicity-tombstoning]`
- `research-handoff-spawn-idempotency-duplicate-handling` — `depends_on: [spawn-delivery-atomic-claim-idempotency-generation]`
- `research-handoff-spawn-restart-continuation-orchestration` — `depends_on: [research-handoff-spawn-stale-event-fencing, research-handoff-spawn-idempotency-duplicate-handling]`
- `research-handoff-spawn-reconnect-cursor-reconcile` — `depends_on: [research-handoff-spawn-restart-continuation-orchestration]`
- `deployment-authority-workspace-scoped-revocable-keys` — `depends_on: [research-handoff-spawn-restart-continuation-orchestration]`
