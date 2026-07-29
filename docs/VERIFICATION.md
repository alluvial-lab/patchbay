# Patchbay Verification

Patchbay treats coordination semantics as specification-first. Formal models define the behavior the implementation must preserve.

## Artifact authority order

Authority is question-type-layered, not a single ranked list. Each artifact type owns one class of question and is authority only for that class:

| Question type | Authority | Not authority for |
|---|---|---|
| Invariants, dynamic/relational properties | Formal models (TLA+/Quint/Alloy), once promoted | wire shape, product intent naming |
| Wire shape, field identity, enum wire encoding, payload envelopes | `.proto` (Protobuf+Buf) | invariants, product intent, enum variant naming |
| Product intent, vocabulary naming, registry names | Prose (`docs/PROTOCOL.md`, `docs/SPEC.md`, `docs/SECURITY.md`, `docs/ARCHITECTURE.md`) | invariants, wire shape |
| Expected executable examples for a specific scenario | Conformance vectors, once promoted | invariants, wire shape, product intent |
| Anything | Implementation (never authority) | — |

Disagreements route to the artifact that owns the question type. "The higher artifact wins" is not a global rule. A contradiction between two promoted artifacts that each own their question type — for example a promoted conformance vector and the formal model whose property it exercises — is a **surfaced reconciliation event**, not a silent override by whichever artifact is ranked higher: either the model is wrong (update the model, then re-check every vector exercising that property) or the vector is wrong (demote, fix, re-promote). Implementation is never authority; if running code disagrees with a normative artifact, the code is the bug-fix target.

This layered order takes effect once generated schemas or IDL exist. `docs/PROTOCOL.md` remains the canonical source of truth for product intent and vocabulary naming both before and after `.proto` exists; it is the *provisional* wire reference only until `.proto` takes over wire-shape authority.

## v0.1.0 normative baseline (property-graded)

Each required model area below is obligated at v0.1.0. Properties within each area are tiered by risk, not all-or-nothing per area. This reconciles `docs/SPEC.md`'s verification-floor seed ("at least seed formal/property checks for command acceptance, idempotent retry, session identity, snapshots, and authority") with this document's required-areas list: the checked set is the seed done right, not a different program.

**checked-model** (also called **model-promoted**) — the formal model exists, passes with documented tool invocation, and carries promoted model metadata, but no promoted conformance vector has landed yet. This is the current status of the seed models listed under "Seed models (v0.1.0)" below. It is a real formal check, but it is not yet checked-normative product behavior.

Current checked-model properties:

- Operator intent delivery / lifecycle properties from `command_lifecycle.qnt`: `TerminalFinality`, `BoundaryDedup`, and `NoAcceptedToCompleted`.
- Session-generation monotonicity from `session_generation.qnt`: `GenerationMonotonic`.
- Browser session and CSRF boundary from `csrf_browser.qnt`: `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`, `RevokedSessionCannotCommand`, and `browser_local_state_not_authority`.

The session/principal revocation model and its four vectors are present as draft artifacts. They remain **stated-normative** until the independent attempted-evidence formulas, mutation gates, and vector promotion review are completed; passing the current checker is not presented as checked-normative evidence.

**checked-normative** — must clear the model-promotion rule **and** have ≥1 promoted conformance vector tracing to the property before v0.1.0 treats the behavior as checked product semantics. No properties are currently checked-normative because no conformance vector has been promoted yet.

**stated-normative** — documented v0.1.0 obligation with a draft model, no model yet, or a reserved property whose obligation is not backed by a promoted model. These are product obligations but must not be claimed checked until promoted through the model gate and, for checked-normative product semantics, the vector gate. A property with a promoted model but no promoted conformance vector is **checked-model**, not stated-normative. Current stated-normative areas include:

- OperationState transition adjacency and read/query lifecycle refinements: `NoAcceptedToCompleted` is checked-model, while the full transition graph and no-direct-to-completed fast-path reads rule remain stated-normative until a full adjacency/read-specific model or conformance vector coverage is promoted.
- Authority safety: no-command/no-Operation-without-grant rejection before acceptance and delivery; `CompoundIssuer`; `GrantAuthorityIsCommandKinds` / `GrantAuthorityIsOperationKinds`; revocation prevents future Operation acceptance under the revoked grant; fleet-spawn authorization (`FleetAuthorityForSpawn`); non-cascading spawn-grant revocation (`SpawnRevocationDoesNotCascade`); Elicitation responder authority (`ElicitationResponderAuthority`); and descendant-grant creation (`SpawnCreatesDescendantGrant`). These remain stated-normative until models represent the submitting evidence and claimed failure boundaries with mutation-survivable independent oracles.
- Session/principal revocation: `RevokeAllInvalidatesPriorSessionGeneration`, `PrincipalRevocationPreventsFuture`, `EndpointRevocationPreventsFuture`, and `DeviceRevocationPreventsFuture` remain stated-normative draft properties. The real-process/replay tests, vectors, and guard-removal mutations are implementation evidence and do not silently promote these model properties.
- Subscription audit, cursor-replay authorization, and grant authorization: `SubscriptionAudited`, `SubscriptionCursorReplayAuthorized`, and `SubscriptionGrantChecked` remain stated-normative until the model separates attempted audit/replay/actor/scope evidence from state written by the subscription actions.
- Command durability and terminal-race/retry refinements: `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, and `RetryAfterTerminalReturnsExisting` remain stated-normative until models represent their claimed failure boundaries.
- Session identity and stale-generation refinements: `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, and `LateGenerationInert` remain stated-normative until models represent per-session identity, target selection, and stale-event audit state.
- Elicitation lifecycle and timeout semantics: `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationCorrelationTyped`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`, and `ElicitationTimeoutNeitherSuccessNorDenial` remain stated-normative until the model uses mutation-survivable independent attempted evidence and represents the timeout grant boundary.
- Relational actor identity: `ActorIdsUnique` remains a stated-normative injectivity obligation; its retained Alloy fact-consequence check is only a structural regression test, not promoted assurance.
- Crash recovery: no accepted command disappears silently after an ungraceful restart; idempotent log replay; snapshot checkpointing as recovery-cost bound.
- Snapshot convergence: stale/cross-domain snapshot rejection, consistent-prefix materialization, late-event no-rewrite, compaction and cursor validity nuances, and "event streams not required for correctness when snapshots exist" as an operational property.
- Audit integrity: completeness of audit records and correlation coverage.
- Adapter failure visibility: failure-vocabulary distinguishability refinements.
- Reply and response-Operation correlation: `TypedCorrelation`, duplicate-reply idempotency/rejection, and reference-resolution edge cases remain stated-normative until correlation acceptance is checked against independent attempted evidence.

## v1 release assurance policy

Patchbay uses a **property-graded hybrid** for `v1.0.0`. Every public safety claim requires executable evidence against the implementation. Formal coverage additionally blocks release only where exhaustive state-machine/interleaving analysis is load-bearing:

1. command terminal races and first-durable-terminal-commit finality;
2. session-generation identity, monotonic replacement, and stale-generation inertness;
3. crash recovery, idempotent replay, consistent snapshots, and replay/snapshot convergence;
4. multi-surface Elicitation first-answer and stale-target races.

Multi-human delegation, lease exclusivity, federation, HA, and split-brain models are promotion gates for those future capabilities, not `v1.0.0` release gates.

Release-assurance vocabulary:

- **Specified** — required by canonical prose or generated contracts.
- **Model-checked** — established only for the bounded abstract model.
- **Implementation-checked** — exercised against running product code.
- **Release-verified** — carries every evidence form required by its property risk grade.

A formally gated property is release-verified only when the model represents the claimed failure boundary; the property name states exactly what the formula proves; adversarial mutation/non-vacuity checks demonstrate that the property is genuine; at least one executable vector runs against the implementation; the model and vector share a traceable property id; and CI runs the real checker and executable test rather than validating metadata alone.

This policy preserves the checked-model/stated-normative distinction as honest requirements bookkeeping but does not let a checked abstract model stand in for product evidence. Existing promoted properties whose formulas materially under-model their names must be rewritten, renamed/demoted, or removed before they can contribute to a release claim.

### `OperationState` ⇿ `CommandState` refinement (mixed property tiers by equivalence)

`OperationState` is not a new checked model. It reuses `CommandState` by documented refinement; this does not make the full `docs/PROTOCOL.md` transition graph checked. Only promoted properties from `specs/seed/command_lifecycle.qnt` apply as checked-model properties by equivalence. Demoted properties remain stated-normative obligations:

| Operation vocabulary | Existing artifact and tier |
|---|---|
| `Operation` accepted lifecycle record | `Command` record in `command_lifecycle.qnt`; `CommandDurability` is stated-normative |
| `OperationKind` | `CommandKind` / `CommandKindRequest` registry concept |
| `OperationState` state-name vocabulary | `CommandState` names: `accepted`, `delivered`, `running`, `completed`, `rejected`, `failed`, `expired`, `cancelled`, `superseded` |
| terminal finality | `TerminalFinality` — checked-model |
| first durable terminal commit | `PreAppendTerminalChoice` + `LsnDeterminesTerminalWinner` — stated-normative |
| idempotent retry | `BoundaryDedup` — checked-model; `RetryReusesIdAndKey` + `RetryAfterTerminalReturnsExisting` — stated-normative |
| no direct accepted-to-completed transition | `NoAcceptedToCompleted` — checked-model |

Classification: **checked-model by refinement only** for `TerminalFinality`, `BoundaryDedup`, and `NoAcceptedToCompleted`; the five demoted lifecycle properties are **stated-normative**. The specific no-`accepted → completed` adjacency is checked by `NoAcceptedToCompleted`; the full transition graph remains stated-normative, not fully checked. Read/query Operations use the same lifecycle in v0.1.0; they may skip `running`, but the no-direct-to-completed fast-path rule is also stated-normative. A future no-lifecycle read optimization is a reserved seam and would require its own registry/model decision. A future rename from `CommandState` to `OperationState` must update model names, property metadata, `.proto`, conformance vectors, and docs together.

### Elicitation model obligations (stated-normative)

`specs/seed/elicitation_lifecycle.qnt` currently has no promoted properties. `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationCorrelationTyped`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`, and `ElicitationTimeoutNeitherSuccessNorDenial` remain **stated-normative** obligations with no executable property formula. The removed formulas inspected terminal baselines or response evidence written by the same accepting action; coordinated mutations could rewrite that evidence and pass. Future promotion requires independent attempted-event evidence and mutation-survivable oracles, including explicit grant state for the timeout obligation.

Elicitation ids remain adapter-assigned in v0.1.0; the core assigns only the durable LSN at record time. The core does not open Elicitations in v0.1.0, so no core-opened-Elicitation property is reserved.

### `TypedCorrelation` response-Operation obligation (stated-normative)

`TypedCorrelation` remains the shared stated-normative obligation for Reply → Command/Message and response Operation → Elicitation typed correlation, same-context binding, expected-responder policy, and separation of CommandId, MessageId, ReplyId, EventId, and ElicitationId spaces. `specs/seed/reply_correlation.qnt` currently has no promoted properties and retains the model vocabulary but no executable `TypedCorrelation` formula. The removed formula inspected correlation evidence recorded by the same accepting action, so accepting arbitrary input while recording canonical evidence could pass. Future promotion requires independent attempted correlation evidence.

### `authority.qnt` promotion status

`specs/seed/authority.qnt` currently has no promoted properties. Its authority, spawn, revocation, descendant-grant, and responder obligations are draft/stated-normative because their removed formulas did not independently establish the claimed behavior.

Stated-normative properties with no executable formula:

- `NoCommandWithoutGrant`
- `CompoundIssuer`
- `GrantAuthorityIsCommandKinds`
- `RevocationPreventsFuture`
- `SpawnCreatesDescendantGrant`
- `FleetAuthorityForSpawn`
- `SpawnRevocationDoesNotCascade`
- `ElicitationResponderAuthority`

The first three general properties retain their documented refinements to `NoOperationWithoutGrant`, the operator-session issuer shape, and `GrantAuthorityIsOperationKinds`; those refinements do not promote the draft obligations. `SpawnCreatesDescendantGrant` requires a future model using the canonical descendant-grant OperationKind set and action-created grant state. `FleetAuthorityForSpawn` and `ElicitationResponderAuthority` require submitted actor/endpoint evidence that the accepting action cannot rewrite. `SpawnRevocationDoesNotCascade` requires a pre/post-state oracle that detects deletion of an existing descendant during fleet-grant revocation.

Reserved future authority properties (not v0.1.0 obligations): actor-neutral/non-operator Operation sender verification, agent/service grant subjects for authority-bearing Operations, tighter Elicitation responder binding by endpoint/endpoint class/fallback chain, and cross-actor delegation through `parent_grant_id`. The actor-neutral vocabulary remains the seam, but v0.1.0 properties must not pretend non-operator authority-bearing Operations exist.

Classification: all eight property ids above remain **stated-normative until promoted** with genuine, mutation-survivable formulas.

### Subscription authority obligations (stated-normative)

Subscriptions remain grant-checked protocol obligations without `OperationState` lifecycle, but `specs/seed/subscription_authority.qnt` currently has no promoted properties. `SubscriptionAudited`, `SubscriptionCursorReplayAuthorized`, and `SubscriptionGrantChecked` are **stated-normative** with no executable property formula. Their removed invariants compared audit/replay/actor/scope state written by the same establishment or replay actions rather than independent attempted evidence, so coordinated lies could pass.

The subscription model remains split out of `authority.qnt` to keep future checks tractable under Apalache while preserving the same grant-tuple vocabulary. Future promotion requires trace-faithful attempted establishment and replay evidence with mutation-survivable independent oracles.

The delegation precondition and lease safety sections below are preconditions for future behavior and are **not** part of the v0.1.0 normative baseline.

## TLA+ and Quint position

TLA+ and Quint are compatible at the architecture level because both model state machines in the TLA tradition. Patchbay does not need to choose a permanent winner before design starts.

Patchbay uses this policy:

- **TLA+ is the semantic baseline** for durable, long-lived protocol models because it is established and has mature TLC tooling.
- **Quint is the ergonomic authoring candidate** for models where readability, type checking, and developer/agent editing speed matter.
- Models may begin in Quint and be checked through available backends, including TLC where appropriate.
- A model promoted to normative status must have a stable checked artifact and documented tool invocation, regardless of whether it is authored in TLA+ or Quint.
- The repository keeps model intent portable: no product decision depends on a tool-specific trick when the underlying property can be stated plainly.

This means Patchbay can start with Quint for approachability and keep TLA+ as the reference-compatible foundation.

## Alloy position

Alloy is complementary rather than competing with TLA+/Quint.

Patchbay uses Alloy for bounded relational invariants:

- actor identity uniqueness;
- endpoint/address ambiguity;
- authority graph constraints;
- revocation relationships;
- lease exclusivity;
- routing legality;
- anti-spoofing relationships.

TLA+/Quint models dynamic histories. Alloy models relational shapes and small counterexamples.

**Measurement discipline (load-bearing):** verify Alloy assertions with `java -jar org.alloytools.alloy.dist.jar exec --command <label> --type text --output - <file>.als`. A `skolem $<AssertName>_...` line in the output means a counterexample was found (the assertion FAILS); its absence means `UNSAT` (the assertion holds). Do NOT use `--type json` or output-file-count to judge UNSAT — both give false positives (reported a passing assertion on an actually-failing check in the seed-model arc). For Quint, `quint verify`/`quint run` exit non-zero (1) when a counterexample is found; exit 0 = no violation.

## Required model areas

### Operator intent delivery

Properties:

- An accepted command is durably recorded before delivery.
- An accepted command cannot vanish silently.
- Every accepted command remains observable in exactly one canonical `CommandState` from `docs/PROTOCOL.md` until and after it reaches a terminal state.
- **TerminalFinality**: once a command reaches a terminal `CommandState`, later events for that command do not mutate the command state.
- **LsnDeterminesTerminalWinner** (stated-normative, not currently checked): for competing valid terminal candidates, the terminal winner is the candidate with the lowest committed log sequence number in the authority domain.
- **PreAppendTerminalChoice** (stated-normative, not currently checked): if terminal candidates are truly concurrent before durable append, the model may choose the appended winner nondeterministically; after an `LSN` is assigned, that order determines all later snapshots, replay, conformance traces, and UI reconciliation.
- Timeout does not imply success or denial.

### Wrong-session prevention

Properties:

- Commands bind to target session identity and generation. Session identity is the tuple adapter id + deployment scope + runtime session id + session generation; project, cwd, and name are metadata, not identity.
- **LateGenerationInert**: events/replies binding to a tombstoned session generation are `stale_event` audit records; they do not mutate the live generation.
- **GenerationMonotonic**: the checked temporal property proves that the live session generation never decreases. Strict-supersession (lower reports are rejected and equal reports are no-ops) is enforced by the action guard, not established by this checked temporal property.
- Human-readable labels cannot override verified target identity.

### Reply and response-Operation correlation

Properties:

- A reply references a known prior message or command by typed correlation.
- A response Operation (`approval-response` or `elicitation-response`) references a known prior Elicitation by typed correlation.
- **TypedCorrelation** (stated-normative, not currently checked): replies correlate by typed reference to known command/message ids in the same authority/session context, and response Operations correlate by typed reference to known ElicitationIds in the same authority/session/responder context; neither shape can forge correlation across id spaces (CommandId, MessageId, ReplyId, EventId, and ElicitationId) or across authority/session contexts.
- Duplicate replies are either idempotent or visibly rejected.

### Idempotent retry

`BoundaryDedup` is the checked-model idempotency property: it checks deduplication at the Patchbay acceptance boundary, not end-to-end execution. It applies per-target by protocol refinement: a key dedups against commands to the same target. `RetryReusesIdAndKey` and `RetryAfterTerminalReturnsExisting` remain stated-normative obligations with no executable property formula; their removed formulas did not observe retry inputs or returned-record identity and therefore did not check those named behaviors. The protocol still requires key retention at least until terminal, while any post-terminal retention policy (whether a later same-key submission is treated as a duplicate of the terminal record or as a new command) is implementation-defined. No current property claims that an adapter executes a given Operation exactly once on retry; adapter-side execution idempotency is governed by the adapter's declared `idempotency_strength` capability and is not a formal property until a future adapter contract model is scoped (see the spawn-idempotency note below).

The `execution_outcome_unknown` failure term is a presentation/audit signal, not a checked property: it surfaces ambiguity to control surfaces so retry safety can be evaluated, but the protocol does not formally model adapter-side execution determinism.

Properties:

- Retrying the same idempotency key cannot double-apply a command at the Patchbay boundary (per-target: the key dedups against commands to the same target).
- A retry reuses both the command id and the idempotency key; an intentional duplicate action uses a new command id and a new idempotency key.
- Duplicate submission returns existing command state.
- Retrying after a command is terminal returns the existing terminal command record rather than creating a later terminal candidate.
- Explicit duplicate action requires a new command id/key.

### Snapshot convergence

Properties:

- A reconnecting control surface can recover authoritative state from snapshots.
- Stale cached live/working state is corrected by a newer authoritative snapshot.
- Event streams are not required for correctness when snapshots exist.
- A snapshot with a log sequence number strictly less than the core's current revision for that view is rejected as an authority source and replaced by the current view.
- A snapshot from a different authority domain or core generation is rejected outright.
- A late event whose log sequence number is older than the view it would mutate is recorded as an audit/reconciliation event and does not rewrite the current view.
- Terminal outcomes are deterministic after durable append: replay, snapshots, conformance traces, and UI reconciliation expose the terminal state chosen by committed log order.
- Snapshot materialization reads a consistent log prefix: it reflects every event with `LSN <= snapshot_LSN` and no event with `LSN > snapshot_LSN`.

Normative model variables should include at least `LSN`, `Cursor`, `SnapshotRevision`, `AuthorityDomain`, `CoreGeneration` (the core's own incarnation, core-assigned on restart), `SessionGeneration`, `AdapterGeneration`, and the view variables (`CommandId`, `MessageId`, `ReplyId`, `CorrelationRef`, `SessionId`, `ActorId`) the snapshot reconciles.

### Crash recovery

Properties:

- After an ungraceful core restart, replay of the durable log reconstructs in-memory state up to the last committed `LSN`.
- Accepted commands are restored as `accepted` (or a later committed state) and continue through their lifecycle; no accepted command disappears silently.
- Log replay is idempotent: replaying the same committed prefix produces identical state.
- Snapshot checkpointing bounds recovery replay cost without becoming an alternate ordering authority.

Normative model variables should include at least `CommittedPrefixLSN` (the last durably committed log prefix), `CheckpointSnapshotLSN` (the latest snapshot loaded before tail replay), `RecoveredCommandState` (the command-id to `CommandState` map reconstructed from the log), `RecoveredInbox` (delivery/inbox queue state reconstructed from the log), `RecoveredSessionView` (session connectivity/activity axes reconstructed from the log), and `RecoveryPhase` (initial load vs tail-replay vs live). `Crash` and `Restart` are the transition triggers.

v0.1.0 models do not need to prove remote replication, HA failover, or split-brain resolution. Those are out of formal scope.

### Authority safety

Properties:

- Commands without grants are rejected before durable acceptance and before delivery.
- Grant matching checks issuer actor, optional endpoint, target scope, OperationKind, expiration, and revocation generation. Device is not a grant-matching field; it is an identity and revocation-grouping variable.
- Revocation prevents future command acceptance under the revoked grant.
- Already accepted commands follow the grant's revocation policy: continue, cancel where supported, or require reauthorization.
- Lockdown rejects new commands and marks affected runtime sessions stale until fresh authentication or operator action clears the condition.
- **CompoundIssuer**: when a command arrives through a control surface, the core verifies the transport endpoint (e.g. the web server, or a CLI endpoint) as a principal and independently verifies the operator actor against operator-session evidence. The core must not trust a self-asserted operator identity.
- **GrantAuthorityIsCommandKinds** (generalized by vocabulary rename to `GrantAuthorityIsOperationKinds`): grant authority is expressed only in canonical Patchbay OperationKinds. Adapter capability declarations are advisory UX state and are not an authority or delivery gate; the adapter accepts or rejects at delivery time.

Normative model variables should include at least `Actor`, `Device`, `Endpoint`, `OperatorSession`, `Grant`, `GrantScope`, `CommandKind`, `Target`, `TargetGeneration` (the session generation bound to a command target), `RevocationGeneration`, `CommandIssuer`, `AuthorityDomain`, `SessionGeneration`, `AdapterGeneration`, and `CorrelationRef`. `Device` is included as an identity, audit, and revocation-grouping variable even though it is not a grant-matching field.

### Delegation precondition

Delegation is not part of v0.1.0. The following property is a precondition that must be satisfied before any delegation-backed behavior ships; it is not a required v0.1.0 authority-safety obligation:

- Delegation cannot create authority beyond its parent grant.

### Lease safety

Lease safety remains a required model area before any lease-backed product behavior ships. It is not part of the v0.1.0 executable walking skeleton unless later foundation work explicitly promotes a specific lease-backed workflow. The exclusivity properties are a modeled precondition gated on a future fencing model, not a v0.1.0 guarantee (see `docs/PROTOCOL.md` § Leases for the canonical statement).

Properties:

- Two actors cannot simultaneously hold the same exclusive live lease within one authority domain.
- Expired leases do not authorize new exclusive action.
- Lease renewal respects holder identity and scope.

### Adapter failure visibility

Properties:

- Adapter disconnect, crash, rejection, unsupported command, target offline, timeout, expiration, cancellation, and supersession remain distinguishable using the failure/outcome vocabulary in `docs/PROTOCOL.md`.
- Adapter failure cannot appear as command completion.
- A `partial` or `no snapshot` adapter cannot cause the core to fabricate a live snapshot from cached or optimistic state; affected session axes move to `stale` or `unknown`.

### Browser session and CSRF boundary

Properties:

- A state-changing browser request without an authenticated operator session is rejected before command acceptance.
- A state-changing browser request without a valid session-bound CSRF proof is rejected before command acceptance.
- Revoked or expired operator sessions cannot issue new commands.
- **browser_local_state_not_authority**: browser-local UI claims cannot grant authority or override server-side session/CSRF checks. `csrf_browser.qnt` promotes this as a checked-model invariant by independently checking the raw server-side operator-session status and session-bound CSRF proof rather than trusting browser-local UI claims; the model has no grant state.

Formal models do not prove browser cookie mechanics or cryptographic token strength; they model the server-side effects of valid, missing, expired, and revoked session/CSRF evidence.

### Audit integrity

Properties:

- Security-relevant decisions produce audit records: authentication success/failure, session revocation, failed authorization, command acceptance/rejection, grant changes, lockdown, adapter failure, and stale-event rejection.
- Audit records correlate to actor, device, endpoint/session when known, target, command, outcome, and reason without requiring secret material in the model.
- Rejected attempts and failed checks can produce audit records without creating command records.
- Revocation and terminal command outcomes remain visible in audit history; they are not deleted by later state changes.

## Out of formal scope

Patchbay formal models do not prove:

- LLM output quality;
- correctness of cryptographic primitives;
- operating-system scheduling or mobile background behavior;
- UI rendering correctness;
- third-party harness internals;
- real-world network latency bounds;
- adapter-specific behavior beyond declared adapter contracts.

Those areas require tests, monitoring, adapter documentation, and operational discipline.

## Conformance testing

Formal models produce implementation obligations. The implementation uses:

- protocol golden vectors shared across languages and derived from the canonical state/failure registries in `docs/PROTOCOL.md`;
- terminal-commit race vectors covering completion before cancellation, cancellation before completion, expiration before late completion, retry after terminal, late terminal candidate as audit/reconciliation only, and replay of the same committed prefix;
- property tests for Rust core behavior;
- property tests for TypeScript operator-domain behavior;
- adapter conformance tests for declared capabilities;
- replay tests for event logs and snapshots;
- reconnect tests for stale control surfaces;
- session/principal/endpoint/device revocation vectors and real-process/replay tests covering old generations, same-id fences, unaffected identities, accepted-work continuation, and filtered subscription establishment.

### Conformance-vector reservations (stated-normative until promoted)

Reserve the following conformance-vector families. Each is draft until its referenced model property is promoted and the vector is reviewed.

- `operation-query-uniform-lifecycle`: query/read uses the normal Operation lifecycle (for example accepted, then delivered, then completed), not a direct-to-completed fast path.
- `operation-read-no-lifecycle-reserved`: no-lifecycle reads are rejected/unavailable in v0.1.0 unless promoted by registry update.
- `agent-send-reserved-validation`: `agent-send` submission rejects with `validation_failed` in v0.1.0.
- `spawn-fleet-authority`: spawn accepted with fleet grant; rejected with only per-session grant when target session does not exist.
- `spawn-descendant-grant`: successful spawn completion emits an explicit auditable descendant grant for the spawned session.
- `spawn-revocation-two-levers`: revoking the spawn grant blocks future spawns but leaves descendant grants live until separately revoked.
- `spawn-shape-adapter-unsupported`: `target_spec.shape` is carried for vocabulary/audit/display; unsupported shapes are rejected by the adapter at delivery with `unsupported_command`, not by protocol-layer shape validation.
- `elicitation-answer-first-wins`: two valid answers from different subscribed surfaces race; lower LSN wins and clears the Elicitation everywhere.
- `elicitation-responding-endpoint-audited`: response Operation audit records which authenticated endpoint answered for the expected operator actor.
- `elicitation-invalid-response`: invalid answer rejected and Elicitation remains pending by default.
- `elicitation-stale-generation`: answer after target generation tombstone records stale/audit and does not mutate live state.
- `operation-response-correlation-forgery`: response Operation using ReplyId/EventId/CommandId as ElicitationId rejected.
- `subscription-grant-checked`: subscription establish succeeds/fails by grant and records audit without OperationState.
- `subscription-audited`: subscription allow/deny decisions create security audit records without creating Operation records.
- `subscription-cursor-replay-authorized`: reconnect replay by cursor returns only events within the authorized subscription filter.
- `session-model-change-preserves-identity`: a `SessionModelChanged` mutation changes only opaque current-model metadata while retaining adapter id, deployment scope, runtime session id, and generation; it exercises `LabelsCannotOverrideIdentity` as a draft wire-shape example.

Spawn capability-manifest idempotency-strength handling (`none` / `at-Patchbay-boundary` / `end-to-end`) is classified as adapter-contract/conformance-only for now, outside the current formal model scope. The core's boundary dedup remains covered by `command_lifecycle.qnt` (see § Idempotent retry above for the boundary-vs-end-to-end scope statement); adapter-side duplicate external process prevention is not claimed as a formal property until a future adapter contract model is scoped.

A protocol semantic change updates `docs/PROTOCOL.md`, the model, generated contract, conformance vectors, and implementation together.

### Conformance-vector promotion and traceability

Conformance vectors are draft/derived until explicitly promoted, mirroring the model-promotion rule. A promoted vector is a peer authority for expected executable examples (see Artifact authority order). Vectors are never authority for invariants (that is the formal models) or for wire shape (that is `.proto`).

Vector promotion requires:

- a named model property the vector exercises (property id);
- the `.proto` fields/enums the vector constrains (or `none` for pure state-transition vectors);
- an expected outcome matching the referenced model property's invariant;
- a reviewed status (a vector is promoted by review, not automatically).

Each conformance vector lives as a JSON file under `contracts/vectors/` and carries a structured envelope naming these fields (see `contracts/vectors/README.md` for the complete schema):

```json
{
  "property_id": "<property-id>",
  "promotion_status": "draft | promoted",
  "proto_fields_constrained": ["field/path"],
  "expected_outcome": {}
}
```

Run `node contracts/scripts/check-vectors.mjs` from the repository root (or `npm run check:vectors` from `contracts/ts/`) to validate the vectors and regenerate the traceability table below.

A CI script reads all vectors and:

- fails if a checked-normative property lacks a promoted vector;
- fails if a vector references a missing or misspelled property id;
- fails if a promoted vector's expected outcome contradicts its referenced model property's invariant (a surfaced contradiction, per the authority order);
- generates the traceability table in this document as a checked-in artifact, so the human-readable mapping from property → `.proto` fields → vectors never drifts.

A promoted vector that later contradicts its model is a reconciliation event: either the model is wrong (update the model, re-check every vector exercising it) or the vector is wrong (demote, fix, re-promote). It is never a silent override.



<!-- BEGIN GENERATED MODEL-PROMOTION TRACEABILITY -->
<!-- Generated by `node contracts/scripts/check-models.mjs`; do not edit this block by hand. -->

### Generated model-promotion traceability table

Source models: `specs/seed/*.qnt` and `specs/seed/*.als`. Product tier is derived from model `status` plus promoted conformance-vector coverage; model files do not store a `tier` field.

Summary: 53 modeled properties (8 promoted, 45 draft), 3 reserved-unmodeled stated-normative properties, 0 properties with promoted vector coverage.

| Property id | Model status | Derived tier | Model | Backend | Promoted vectors | Invocation | Semantics |
|---|---|---|---|---|---|---|---|
| `ActorIdsUnique` | draft | stated-normative | specs/seed/patchbay-relational.als | alloy-cli | — | <TBD — demoted; assertion checks a constraint already imposed by the ActorIdsUnique fact; actor uniqueness belongs in generated/database constraints plus executable negative tests> | actor-id injectivity remains a product obligation; this retained fact-consequence check is only a structural regression test against accidental weakening of the ActorIdsUnique fact and is not independent assurance or proof of non-vacuity |
| `AuthorityGraphAcyclic` | draft | stated-normative | specs/seed/patchbay-relational.als | alloy-cli | — | <TBD — not yet checked; promote when delegation is modeled> | RESERVED: acyclicity of the grant issuer-subject graph is only meaningful once a delegation/parent-grant edge exists. v0 has no delegation (docs/PROTOCOL.md:305), so the graph has no cycle-bearing edge to check — asserting acyclicity now is either vacuous (empty graph) or false (unconstrained self-grants). Promote when delegation is added. |
| `BootstrapOnlyExit` | draft | stated-normative | specs/seed/security_lockdown.qnt | apalache | — | quint verify security_lockdown.qnt --invariant BootstrapOnlyExit --max-steps 12 | only the configured loopback admin bootstrap channel can clear lockdown |
| `BoundaryDedup` | promoted | checked-model | specs/seed/command_lifecycle.qnt | apalache | — | quint verify command_lifecycle.qnt --invariant boundary_dedup --max-steps 12 | retrying the same idempotency key cannot double-apply a command at the Patchbay boundary |
| `browser_local_state_not_authority` | promoted | checked-model | specs/seed/csrf_browser.qnt | apalache | — | quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12 | browser-local UI claims cannot grant authority or override server-side session/CSRF checks |
| `CommandDurability` | draft | stated-normative | specs/seed/command_lifecycle.qnt | apalache | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | an accepted command is durably recorded before delivery and cannot vanish silently |
| `CompoundIssuer` | draft | stated-normative | specs/seed/authority.qnt | apalache | — | <TBD — not yet checked; promote in a follow-on item> | accepted commands use verified session-derived actor identity, not self-asserted payload actor |
| `CrashNoAcceptedLost` | draft | stated-normative | specs/seed/snapshot_recovery.qnt | tlc | — | <TBD — not yet checked; promote in a follow-on item> | after ungraceful restart and replay, accepted pre-crash command entries remain reconstructable in-memory |
| `CsrfRejectsMissingProof` | promoted | checked-model | specs/seed/csrf_browser.qnt | apalache | — | quint verify csrf_browser.qnt --invariant csrf_rejects_missing_proof --max-steps 12 | SECURITY.md §CSRF requires a CSRF token tied to the authenticated operator session before command acceptance |
| `CsrfRejectsUnauthenticated` | promoted | checked-model | specs/seed/csrf_browser.qnt | apalache | — | quint verify csrf_browser.qnt --invariant csrf_rejects_unauthenticated --max-steps 12 | SECURITY.md §CSRF requires an authenticated operator session cookie before accepting a state-changing web command |
| `DeviceRevocationPreventsFuture` | draft | stated-normative | specs/seed/session_principal_revocation.qnt | apalache | — | quint verify session_principal_revocation.qnt --invariant device_revocation_prevents_future --max-steps 8 | a device fence rejects future Operations from principals bound to that device |
| `ElicitationCorrelationTyped` | draft | stated-normative | specs/seed/elicitation_lifecycle.qnt | apalache | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | response Operations reference a known ElicitationId in the same authority/session/generation/responder context and cannot forge across id spaces |
| `ElicitationFirstAnswerWins` | draft | stated-normative | specs/seed/elicitation_lifecycle.qnt | apalache-temporal | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | for single-answer contracts, the first durably committed valid answer wins and later answers are no-ops |
| `ElicitationInvalidResponseRejected` | draft | stated-normative | specs/seed/elicitation_lifecycle.qnt | apalache | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | invalid or duplicate response Operations are rejected/idempotent and never mutate the terminal answer |
| `ElicitationPendingFinality` | draft | stated-normative | specs/seed/elicitation_lifecycle.qnt | apalache-temporal | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | once an Elicitation reaches a terminal state, later answer/cancel/expire/withdraw/stale candidates do not mutate it |
| `ElicitationResponderAuthority` | draft | stated-normative | specs/seed/authority.qnt | apalache | — | <TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property> | response Operations are accepted only when the modeled submitting endpoint maps to the expected responder actor and the claimed actor matches that responder |
| `ElicitationStaleTargetInert` | draft | stated-normative | specs/seed/elicitation_lifecycle.qnt | apalache-temporal | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | responses to stale target/session generations do not cause the Elicitation to become answered or record answer data |
| `ElicitationTimeoutNeitherSuccessNorDenial` | draft | stated-normative | specs/seed/elicitation_lifecycle.qnt | apalache | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | timeout terminalizes as expired; timeout never implies answer, denial, or grant |
| `ElicitationWithdrawalFinality` | draft | stated-normative | specs/seed/elicitation_lifecycle.qnt | apalache-temporal | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | opener withdrawal terminalizes the Elicitation without allowing later response mutation |
| `EndpointRevocationPreventsFuture` | draft | stated-normative | specs/seed/session_principal_revocation.qnt | apalache | — | quint verify session_principal_revocation.qnt --invariant endpoint_revocation_prevents_future --max-steps 8 | an endpoint fence rejects future Operations from principals bound to that endpoint |
| `FleetAuthorityForSpawn` | draft | stated-normative | specs/seed/authority.qnt | apalache | — | <TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property> | spawn acceptance requires a live fleet-scope spawn Grant whose subject matches the submitting actor; per-session grants alone cannot authorize spawning a not-yet-existing session |
| `GenerationMonotonic` | promoted | checked-model | specs/seed/session_generation.qnt | apalache-temporal | — | echo y \| quint verify session_generation.qnt --temporal generation_monotonic --max-steps 10 | the live session generation never decreases (checked). Strict-supersession (equal/lower reports are no-ops) is additionally enforced by the action guard (`if gen > generation`) but is NOT a checked temporal property — it exceeded Apalache's experimental temporal support; see idea-tlc-temporal-workaround. |
| `GrantAuthorityIsCommandKinds` | draft | stated-normative | specs/seed/authority.qnt | apalache | — | <TBD — not yet checked; promote in a follow-on item> | grant checks constrain authority by canonical command kinds, not adapter capability declarations |
| `GrantAuthorityIsOperationKinds` | reserved-unmodeled | stated-normative | — | — | — | — | — |
| `IdempotentLogReplay` | draft | stated-normative | specs/seed/snapshot_recovery.qnt | tlc | — | <TBD — not yet checked; promote in a follow-on item> | replaying the same committed prefix does not produce additional state divergence |
| `LabelsCannotOverrideIdentity` | draft | stated-normative | specs/seed/session_generation.qnt | apalache | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | project/cwd/name labels update independently and cannot override the verified session identity tuple |
| `LateEventNoRewrite` | draft | stated-normative | specs/seed/snapshot_recovery.qnt | tlc | — | <TBD — not yet checked; promote in a follow-on item> | older late events are recorded as audit/reconciliation and must not rewrite the in-memory command view |
| `LateGenerationInert` | draft | stated-normative | specs/seed/session_generation.qnt | apalache-temporal | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | late replies/events for tombstoned generations are stale_event audit records and do not mutate the live generation |
| `LockdownEntryStalesSessions` | draft | stated-normative | specs/seed/security_lockdown.qnt | apalache | — | quint verify security_lockdown.qnt --invariant LockdownEntryStalesSessions --max-steps 12 | entry clamps every current runtime session to stale |
| `LockdownInvalidatesExistingOperatorSessions` | draft | stated-normative | specs/seed/security_lockdown.qnt | apalache | — | quint verify security_lockdown.qnt --invariant LockdownInvalidatesExistingOperatorSessions --max-steps 12 | entry advances the durable operator-session generation floor |
| `LockdownRejectsNewOperations` | draft | stated-normative | specs/seed/security_lockdown.qnt | apalache | — | quint verify security_lockdown.qnt --invariant LockdownRejectsNewOperations --max-steps 12 | an attempted Operation cannot become accepted while the durable posture is active |
| `LockdownReplayPersists` | draft | stated-normative | specs/seed/security_lockdown.qnt | apalache | — | quint verify security_lockdown.qnt --invariant LockdownReplayPersists --max-steps 12 | replay of the committed entry event preserves active posture across restart |
| `LsnDeterminesTerminalWinner` | draft | stated-normative | specs/seed/command_lifecycle.qnt | apalache-temporal | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | for competing valid terminal candidates, the terminal winner is the one with the lowest committed LSN in the authority domain; once terminal, exactly one LSN records it |
| `NoAcceptedToCompleted` | promoted | checked-model | specs/seed/command_lifecycle.qnt | apalache-temporal | — | echo y \| quint verify command_lifecycle.qnt --temporal no_accepted_to_completed --max-steps 10 | a command cannot transition directly from 'accepted' to 'completed'; it must pass through 'delivered' or 'running' |
| `NoCommandWithoutGrant` | draft | stated-normative | specs/seed/authority.qnt | apalache | — | <TBD — not yet checked; promote in a follow-on item> | commands that reach accepted state do so only with a live matching grant |
| `NoOperationWithoutGrant` | reserved-unmodeled | stated-normative | — | — | — | — | — |
| `PreAppendTerminalChoice` | draft | stated-normative | specs/seed/command_lifecycle.qnt | apalache-temporal | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | before an LSN is assigned, the terminal winner may be chosen nondeterministically; after assignment, the LSN order is stable and determines all later snapshots/replay |
| `PrincipalRevocationPreventsFuture` | draft | stated-normative | specs/seed/session_principal_revocation.qnt | apalache | — | quint verify session_principal_revocation.qnt --invariant principal_revocation_prevents_future --max-steps 8 | a principal fence rejects future Operations from that exact credential principal |
| `RetryAfterTerminalReturnsExisting` | draft | stated-normative | specs/seed/command_lifecycle.qnt | apalache-temporal | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | retrying after a command is terminal returns the existing terminal record rather than creating a later terminal candidate |
| `RetryReusesIdAndKey` | draft | stated-normative | specs/seed/command_lifecycle.qnt | apalache-temporal | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | a retry reuses both the command id and the idempotency key; the command-id-to-key binding is stable after acceptance (an intentional duplicate action uses a new command id and a new idempotency key, which is outside this model's `retry` action) |
| `RevocationPreventsFuture` | draft | stated-normative | specs/seed/authority.qnt | apalache-temporal | — | <TBD — not yet checked; promote in a follow-on item> | a command cannot become accepted in the transition if it is being submitted at or below a revoked generation |
| `RevokeAllInvalidatesPriorSessionGeneration` | draft | stated-normative | specs/seed/session_principal_revocation.qnt | apalache | — | quint verify session_principal_revocation.qnt --invariant revoke_all_invalidates_prior_session_generation --max-steps 8 | a session generation at or below the durable revoke-all floor cannot be accepted |
| `RevokedSessionCannotCommand` | promoted | checked-model | specs/seed/csrf_browser.qnt | apalache | — | quint verify csrf_browser.qnt --invariant revoked_session_cannot_command --max-steps 12 | SECURITY.md and VERIFICATION.md require revoked or expired operator sessions to be rejected before issuing new commands |
| `SenderMatchesClaim` | draft | stated-normative | specs/seed/patchbay-relational.als | alloy-cli | — | <TBD — not yet checked; promote when the dynamic CompoundIssuer binding is modeled> | RESERVED: sender == claimedSender is a DYNAMIC consistency property, not a relational one. In a static snapshot, sender and claimedSender are independent fields — nothing forces them equal except a fact, which makes the assert a tautology. The actual binding (an authenticated identity matches the self-asserted sender) is a CompoundIssuer-style verification action that belongs in authority.qnt (per the Alloy brief's caveat). Promote when that dynamic model exists. |
| `SessionIdentityTuple` | draft | stated-normative | specs/seed/session_generation.qnt | apalache | — | <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property> | session target identity is adapter id + deployment scope + runtime session id + generation, excluding project/cwd/name metadata |
| `SnapshotConsistentPrefix` | draft | stated-normative | specs/seed/snapshot_recovery.qnt | tlc | — | <TBD — not yet checked; promote in a follow-on item> | snapshot materialization reads a consistent durable-log prefix up to SnapshotLSN and does not include events beyond it |
| `SnapshotCrossDomainRejected` | draft | stated-normative | specs/seed/snapshot_recovery.qnt | tlc | — | <TBD — not yet checked; promote in a follow-on item> | applied snapshots do not change authority when origin domain or core generation differs |
| `SnapshotStaleRejected` | draft | stated-normative | specs/seed/snapshot_recovery.qnt | tlc | — | <TBD — not yet checked; promote in a follow-on item> | stale snapshots (LSN < SnapshotRevision) do not replace the current authoritative core view |
| `SpawnCreatesDescendantGrant` | draft | stated-normative | specs/seed/authority.qnt | apalache | — | <TBD — demoted; model uses invented kind names (reboot/snapshot/stop_session) contradicting PROTOCOL.md:181; allowed-kind set is a hard-coded pure function, not action-created state; v1 formal gate owns the real property> | successful spawn inserts an explicit descendant Grant record for the spawned session with non-spawn OperationKinds |
| `SpawnRevocationDoesNotCascade` | draft | stated-normative | specs/seed/authority.qnt | apalache-temporal | — | <TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property> | revoking the fleet spawn grant blocks future spawns and, when a descendant grant exists, does not revoke it |
| `SubscriptionAudited` | draft | stated-normative | specs/seed/subscription_authority.qnt | apalache | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | subscription allow/deny decisions create audit records without creating OperationState records |
| `SubscriptionCursorReplayAuthorized` | draft | stated-normative | specs/seed/subscription_authority.qnt | apalache | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | cursor replay returns only events with LSN greater than the requested cursor and inside the authorized subscription stream/filter |
| `SubscriptionGrantChecked` | draft | stated-normative | specs/seed/subscription_authority.qnt | apalache | — | <TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property> | subscription establishment succeeds only with a live subscribe-kind Grant record whose subject matches the submitting actor and stream/filter scope |
| `TerminalFinality` | promoted | checked-model | specs/seed/command_lifecycle.qnt | apalache-temporal | — | echo y \| quint verify command_lifecycle.qnt --temporal terminal_finality --max-steps 10 | once a command reaches a terminal CommandState, later events do not mutate it |
| `TimeoutNeitherSuccessNorDenial` | reserved-unmodeled | stated-normative | — | — | — | — | — |
| `TypedCorrelation` | draft | stated-normative | specs/seed/reply_correlation.qnt | apalache | — | <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property> | replies use typed same-context references to known prior commands/messages, response Operations use typed same authority/session/responder-context references to known prior Elicitations, and neither can masquerade across CommandId/MessageId/ReplyId/EventId/ElicitationId spaces |

<!-- END GENERATED MODEL-PROMOTION TRACEABILITY -->

<!-- BEGIN GENERATED CONFORMANCE VECTOR TRACEABILITY -->
<!-- Generated by `node contracts/scripts/check-vectors.mjs`; do not edit this block by hand. -->

### Generated conformance-vector traceability table

Source vectors: `contracts/vectors/*.json`. CI check: `node contracts/scripts/check-vectors.mjs` (or `npm run check:vectors` from `contracts/ts/`).

Summary: 40 vector(s), 0 promoted vector(s), 0 checked-normative properties requiring promoted-vector coverage. Current checked-normative coverage gate is empty by design.

| Property id | Classification | Vectors | `.proto` fields/enums exercised by vectors |
|---|---|---|---|
| `ActorIdsUnique` | stated-normative | — | — |
| `AuthorityGraphAcyclic` | stated-normative | — | — |
| `BootstrapOnlyExit` | stated-normative | [lockdown-bootstrap-only-exit](../contracts/vectors/lockdown-bootstrap-only-exit.json) (draft) | patchbay.BootstrapChannelKind<br>patchbay.ExitSecurityLockdownRequest<br>patchbay.ExitSecurityLockdownResult |
| `BoundaryDedup` | checked-model | [replay-committed-prefix-idempotent](../contracts/vectors/replay-committed-prefix-idempotent.json) (draft) | patchbay.Operation.command_id<br>patchbay.Operation.idempotency_key<br>patchbay.SubmissionResult.accepted_lsn<br>patchbay.SubmissionResult.deduplicated |
| `browser_local_state_not_authority` | checked-model | — | — |
| `CommandDurability` | stated-normative | [command-acceptance](../contracts/vectors/command-acceptance.json) (draft) | patchbay.Operation.authority_domain_id<br>patchbay.Operation.command_id<br>patchbay.Operation.idempotency_key<br>patchbay.Operation.kind<br>patchbay.Operation.recipient<br>patchbay.Operation.sender<br>patchbay.Operation.target_scope<br>patchbay.SubmissionResult.accepted_lsn<br>patchbay.SubmissionResult.operation_state<br>patchbay.SubmissionResult.outcome |
| `CompoundIssuer` | stated-normative | — | — |
| `CrashNoAcceptedLost` | stated-normative | — | — |
| `CsrfRejectsMissingProof` | checked-model | — | — |
| `CsrfRejectsUnauthenticated` | checked-model | — | — |
| `DeviceRevocationPreventsFuture` | stated-normative | [device-revocation-prevents-future](../contracts/vectors/device-revocation-prevents-future.json) (draft) | patchbay.RevokeControlSurfaceEndpointRequest.device_id<br>patchbay.RevokeControlSurfaceResult.revoked_principal_count |
| `ElicitationCorrelationTyped` | stated-normative | — | — |
| `ElicitationFirstAnswerWins` | stated-normative | — | — |
| `ElicitationInvalidResponseRejected` | stated-normative | [approval-response-approved](../contracts/vectors/approval-response-approved.json) (draft)<br>[approval-response-denied](../contracts/vectors/approval-response-denied.json) (draft)<br>[elicitation-response-question-answer-and](../contracts/vectors/elicitation-response-question-answer-and.json) (draft)<br>[elicitation-response-question-free-text](../contracts/vectors/elicitation-response-question-free-text.json) (draft)<br>[elicitation-response-question-select-one](../contracts/vectors/elicitation-response-question-select-one.json) (draft) | patchbay.ApprovalDecision<br>patchbay.ApprovalResponsePayload.decision<br>patchbay.ElicitationResponsePayload.clarification<br>patchbay.ElicitationResponsePayload.free_text<br>patchbay.ElicitationResponsePayload.selected_option_id<br>patchbay.Operation.payload<br>patchbay.PayloadEnvelope.content_type<br>patchbay.QuestionContract.allow_free_text<br>patchbay.QuestionContract.options<br>patchbay.ResponseContract.question<br>patchbay.ResponseOption.option_id |
| `ElicitationPendingFinality` | stated-normative | — | — |
| `ElicitationResponderAuthority` | stated-normative | — | — |
| `ElicitationStaleTargetInert` | stated-normative | — | — |
| `ElicitationTimeoutNeitherSuccessNorDenial` | stated-normative | — | — |
| `ElicitationWithdrawalFinality` | stated-normative | — | — |
| `EndpointRevocationPreventsFuture` | stated-normative | [endpoint-revocation-prevents-future](../contracts/vectors/endpoint-revocation-prevents-future.json) (draft) | patchbay.RevokeControlSurfaceEndpointRequest.endpoint_id<br>patchbay.RevokeControlSurfaceResult.revoked_principal_count |
| `FleetAuthorityForSpawn` | stated-normative | — | — |
| `GenerationMonotonic` | checked-model | — | — |
| `GrantAuthorityIsCommandKinds` | stated-normative | — | — |
| `GrantAuthorityIsOperationKinds` | stated-normative | — | — |
| `IdempotentLogReplay` | stated-normative | — | — |
| `LabelsCannotOverrideIdentity` | stated-normative | [session-model-change-preserves-identity](../contracts/vectors/session-model-change-preserves-identity.json) (draft) | patchbay.Session.model<br>patchbay.SessionModelChanged.adapter_id<br>patchbay.SessionModelChanged.deployment_scope<br>patchbay.SessionModelChanged.from<br>patchbay.SessionModelChanged.runtime_session_id<br>patchbay.SessionModelChanged.session_generation<br>patchbay.SessionModelChanged.to |
| `LateEventNoRewrite` | stated-normative | — | — |
| `LateGenerationInert` | stated-normative | — | — |
| `LockdownEntryStalesSessions` | stated-normative | [lockdown-stales-sessions](../contracts/vectors/lockdown-stales-sessions.json) (draft) | patchbay.SecurityLockdownEntered.affected_runtime_session_count<br>patchbay.SessionConnectivityState.SESSION_CONNECTIVITY_STATE_STALE |
| `LockdownInvalidatesExistingOperatorSessions` | stated-normative | — | — |
| `LockdownRejectsNewOperations` | stated-normative | [lockdown-rejects-operation](../contracts/vectors/lockdown-rejects-operation.json) (draft) | patchbay.SecurityLockdownState.active<br>patchbay.SubmissionResult.failure_code<br>patchbay.SubmissionResult.outcome<br>patchbay.SubmissionResult.reason_code |
| `LockdownReplayPersists` | stated-normative | [lockdown-replay-persists](../contracts/vectors/lockdown-replay-persists.json) (draft) | patchbay.SecurityLockdownEntered.reason_code<br>patchbay.SecurityLockdownEvent<br>patchbay.SecurityLockdownState.active |
| `LsnDeterminesTerminalWinner` | stated-normative | [late-terminal-candidate-audit-only](../contracts/vectors/late-terminal-candidate-audit-only.json) (draft) | patchbay.Observation.correlations<br>patchbay.Observation.event_id<br>patchbay.Observation.failure_code<br>patchbay.Observation.lsn<br>patchbay.Operation.command_id<br>patchbay.SubmissionResult.operation_state |
| `NoAcceptedToCompleted` | checked-model | [operation-query-diagnostics-lifecycle](../contracts/vectors/operation-query-diagnostics-lifecycle.json) (draft) | DiagnosticsResult.as_of_lsn<br>Operation.kind<br>QueryDiagnosticsRequest.operation<br>QueryDiagnosticsResponse.submission |
| `NoCommandWithoutGrant` | stated-normative | [failure-missing-grant](../contracts/vectors/failure-missing-grant.json) (draft) | patchbay.Grant.allowed_operation_kinds<br>patchbay.Operation.kind<br>patchbay.Operation.sender<br>patchbay.Operation.target_scope<br>patchbay.SubmissionResult.failure_code<br>patchbay.SubmissionResult.outcome |
| `NoOperationWithoutGrant` | stated-normative | [grant-expiry-rejected](../contracts/vectors/grant-expiry-rejected.json) (draft) | patchbay.Grant.expires_at<br>patchbay.SubmissionResult.decision_grant_id<br>patchbay.SubmissionResult.failure_code<br>patchbay.SubmissionResult.outcome<br>patchbay.SubmissionResult.reason_code |
| `PreAppendTerminalChoice` | stated-normative | [terminal-cancellation-before-completion](../contracts/vectors/terminal-cancellation-before-completion.json) (draft)<br>[terminal-completion-before-cancellation](../contracts/vectors/terminal-completion-before-cancellation.json) (draft) | patchbay.Observation.correlations<br>patchbay.Observation.failure_code<br>patchbay.Observation.kind<br>patchbay.Observation.lsn<br>patchbay.Operation.command_id<br>patchbay.Operation.correlations<br>patchbay.Operation.kind<br>patchbay.SubmissionResult.operation_state |
| `PrincipalRevocationPreventsFuture` | stated-normative | [principal-revocation-prevents-future](../contracts/vectors/principal-revocation-prevents-future.json) (draft) | patchbay.RevokeControlSurfacePrincipalRequest.principal_id<br>patchbay.RevokeControlSurfaceResult.newly_revoked |
| `RetryAfterTerminalReturnsExisting` | stated-normative | [retry-after-terminal-returns-existing](../contracts/vectors/retry-after-terminal-returns-existing.json) (draft) | patchbay.Operation.command_id<br>patchbay.Operation.idempotency_key<br>patchbay.SubmissionResult.command_id<br>patchbay.SubmissionResult.deduplicated<br>patchbay.SubmissionResult.operation_state |
| `RetryReusesIdAndKey` | stated-normative | — | — |
| `RevocationPreventsFuture` | stated-normative | [grant-revocation-policy-effects](../contracts/vectors/grant-revocation-policy-effects.json) (draft)<br>[grant-revocation-prevents-future](../contracts/vectors/grant-revocation-prevents-future.json) (draft) | patchbay.AcceptedOperation.authorizing_grant_id<br>patchbay.FailureCode.FAILURE_CODE_AUTHORIZATION_DENIED<br>patchbay.GrantRevocationEffect.failure_code<br>patchbay.GrantRevocationEffect.from_state<br>patchbay.GrantRevocationEffect.to_state<br>patchbay.Revocation.accepted_operation_policy<br>patchbay.Revocation.command_effects<br>patchbay.Revocation.grant_id<br>patchbay.SubmissionResult.outcome |
| `RevokeAllInvalidatesPriorSessionGeneration` | stated-normative | [session-revocation-generation](../contracts/vectors/session-revocation-generation.json) (draft) | patchbay.RevokeAllOperatorSessionsRequest.reason_code<br>patchbay.RevokeAllOperatorSessionsResult.invalidated_through_generation<br>patchbay.VerifyOperatorPasswordResult.operator_session_generation |
| `RevokedSessionCannotCommand` | checked-model | — | — |
| `SenderMatchesClaim` | stated-normative | — | — |
| `SessionIdentityTuple` | stated-normative | — | — |
| `SnapshotConsistentPrefix` | stated-normative | — | — |
| `SnapshotCrossDomainRejected` | stated-normative | — | — |
| `SnapshotStaleRejected` | stated-normative | [snapshot-reconciliation](../contracts/vectors/snapshot-reconciliation.json) (draft) | patchbay.Observation.lsn<br>patchbay.ObservationSubscription.cursor<br>patchbay.SessionSnapshot.authority_domain_id<br>patchbay.SessionSnapshot.core_generation<br>patchbay.SessionSnapshot.snapshot_lsn |
| `SpawnCreatesDescendantGrant` | stated-normative | — | — |
| `SpawnRevocationDoesNotCascade` | stated-normative | — | — |
| `SubscriptionAudited` | stated-normative | — | — |
| `SubscriptionCursorReplayAuthorized` | stated-normative | [subscription-resume-rechecked](../contracts/vectors/subscription-resume-rechecked.json) (draft) | patchbay.AuditEventKind.AUDIT_EVENT_KIND_SUBSCRIPTION_DENIED<br>patchbay.AuditRecord.grant_id<br>patchbay.SubscribeRequest.authority_domain_id<br>patchbay.SubscribeRequest.cursor |
| `SubscriptionGrantChecked` | stated-normative | [subscription-grant-checked](../contracts/vectors/subscription-grant-checked.json) (draft) | patchbay.AuditEventKind.AUDIT_EVENT_KIND_SUBSCRIPTION_ESTABLISHED<br>patchbay.AuditRecord.grant_id<br>patchbay.OperationKind.OPERATION_KIND_QUERY<br>patchbay.SubscribeRequest.authority_domain_id<br>patchbay.TargetScopeKind.TARGET_SCOPE_KIND_AUTHORITY_DOMAIN |
| `TerminalFinality` | checked-model | [terminal-expiration-before-completion](../contracts/vectors/terminal-expiration-before-completion.json) (draft) | patchbay.Observation.correlations<br>patchbay.Observation.lsn<br>patchbay.Operation.validity_window<br>patchbay.SubmissionResult.failure_code<br>patchbay.SubmissionResult.operation_state |
| `TimeoutNeitherSuccessNorDenial` | stated-normative | — | — |
| `TypedCorrelation` | stated-normative | [reply-correlation](../contracts/vectors/reply-correlation.json) (draft) | patchbay.Observation.correlations<br>patchbay.Observation.reply_id<br>patchbay.ReplyId.value<br>patchbay.TypedCorrelation.command_id<br>patchbay.TypedCorrelation.message_id |
| `boundary-validation` | descriptive boundary validation (draft-only) | [approval-response-invalid-reserved-decision](../contracts/vectors/approval-response-invalid-reserved-decision.json) (draft)<br>[approval-response-invalid-unspecified-decision](../contracts/vectors/approval-response-invalid-unspecified-decision.json) (draft)<br>[approval-response-invalid-wrong-content-type](../contracts/vectors/approval-response-invalid-wrong-content-type.json) (draft)<br>[audit-redaction-boundary](../contracts/vectors/audit-redaction-boundary.json) (draft)<br>[elicitation-response-invalid-both-primary-answers](../contracts/vectors/elicitation-response-invalid-both-primary-answers.json) (draft)<br>[elicitation-response-invalid-free-text-disallowed](../contracts/vectors/elicitation-response-invalid-free-text-disallowed.json) (draft)<br>[elicitation-response-invalid-mismatched-option](../contracts/vectors/elicitation-response-invalid-mismatched-option.json) (draft)<br>[elicitation-response-invalid-terminal-elicitation](../contracts/vectors/elicitation-response-invalid-terminal-elicitation.json) (draft)<br>[failure-missing-target](../contracts/vectors/failure-missing-target.json) (draft)<br>[failure-unknown-operation-kind](../contracts/vectors/failure-unknown-operation-kind.json) (draft) | AdapterCapabilitySummary.attachment_method_kind<br>AdapterDiagnosticPayload.adapter_generation<br>AdapterDiagnosticPayload.code<br>AdapterDiagnosticPayload.count<br>AdapterDiagnosticPayload.operation_kind<br>AdapterDiagnosticReport.payload<br>AuditRecord.operator_session_hash<br>AuditRecord.source_network<br>CommandSummary<br>patchbay.ApprovalDecision<br>patchbay.ApprovalResponsePayload.decision<br>patchbay.ElicitationResponsePayload.clarification<br>patchbay.ElicitationResponsePayload.free_text<br>patchbay.ElicitationResponsePayload.selected_option_id<br>patchbay.Operation.kind<br>patchbay.Operation.payload<br>patchbay.Operation.target_scope<br>patchbay.PayloadEnvelope.content_type<br>patchbay.QuestionContract.allow_free_text<br>patchbay.QuestionContract.options<br>patchbay.ResponseContract.question<br>patchbay.ResponseOption.option_id<br>patchbay.SubmissionResult.failure_code<br>patchbay.SubmissionResult.outcome<br>patchbay.TargetScope.kind<br>patchbay.TargetScope.runtime_session_id<br>patchbay.TargetScope.session_generation |

<!-- END GENERATED CONFORMANCE VECTOR TRACEABILITY -->

## Model promotion rule

A model becomes normative only when it includes:

- the property being checked;
- finite bounds or constants used for checking;
- command/tool invocation;
- expected pass/fail status;
- a short explanation connecting the model to product semantics.

**Genuine-checking discipline (load-bearing for safety-claiming models):** a promoted property must not be self-defining — the invariant must not reuse the same predicate the action's guard uses, or it can never catch a broken predicate. The test: mutate the predicate (break it to `true`, invert it, or weaken a guard); if the invariant still passes, it is self-defining and must be restructured to an **independent oracle** that checks raw state facts the action does not consult. Two refinements discovered in the seed-model arc:
- **Trace-fidelity:** even an independent oracle is insufficient if it checks *action-recorded* state rather than *environment pre-state*. A server-side-acceptance invariant (e.g. CSRF, CompoundIssuer) must check the raw submitted evidence as pre-state the accepting action reads but cannot rewrite — not the action's recorded trace of what was submitted.
- **Relational Alloy:** a `check` that asserts a constraint also enforced by a `fact` is a tautology; removing the fact to make it "genuine" without adding a real constraint turns vacuous-true into actually-false. A relational check must be true because of *other* constraints in the model, or be demoted to draft if none exist (the property may be inherently dynamic and belong in a TLA+/Quint model instead).

Draft models may explore ideas without becoming product commitments.

## Seed models (v0.1.0)

The v0.1.0 seed formal models live under `specs/seed/`. Each model carries its promotion metadata as inline `@promotion` comment blocks (one per checked/draft property) — the machine-readable source `contracts/scripts/check-models.mjs` reads to generate the model-promotion traceability table. The product tier is derived from model `status` plus promoted conformance-vector coverage; model files do not store a `tier` field. The property-id vocabulary established by the seed is the Single Source of Truth that `.proto` contracts, conformance vectors, and implementation all derive from.

### Checked-model (model promoted; awaiting conformance vectors)

These checked-model properties are **unaffected** by the O/O/E vocabulary roll-forward and apply to `OperationState` by equivalence for the listed properties only. They are not checked-normative until at least one conformance vector tracing to each property is promoted. No checked-normative property exists yet in this repository because no conformance vector has been promoted.

| Model | Language | Properties checked | Backend |
|---|---|---|---|
| `specs/seed/command_lifecycle.qnt` | Quint | `BoundaryDedup` (invariant); `TerminalFinality`, `NoAcceptedToCompleted` (temporal) — apply to `OperationState` by refinement equivalence | Apalache + Apalache-temporal |
| `specs/seed/session_generation.qnt` | Quint | `GenerationMonotonic` (temporal) | Apalache-temporal |
| `specs/seed/csrf_browser.qnt` | Quint | `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority` (invariants) | Apalache |

Each checked Quint model also commits a generated `*.emitted.tla` inspection artifact (via `quint compile --target tlaplus`); these are generated, never hand-edited, and are NOT an independent re-check lane (they `EXTENDS ... Apalache, Variants` and need the Apalache jar on the classpath — same toolchain reached via Quint).

The `OperationState` ⇿ `CommandState` refinement mapping (see `OperationState` ⇿ `CommandState` refinement above) means only the promoted `command_lifecycle.qnt` properties apply to OperationState as checked-model by equivalence; demoted properties remain stated-normative. No new model is introduced and no full transition-graph check is implied.

### Stated-normative (draft models; property-ids reserved)

| Model | Language | Reserved property-ids |
|---|---|---|
| `specs/seed/command_lifecycle.qnt` | Quint | `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` |
| `specs/seed/session_generation.qnt` | Quint | `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, `LateGenerationInert` |
| `specs/seed/reply_correlation.qnt` | Quint | `TypedCorrelation` |
| `specs/seed/elicitation_lifecycle.qnt` | Quint | `ElicitationCorrelationTyped`, `ElicitationFirstAnswerWins`, `ElicitationInvalidResponseRejected`, `ElicitationPendingFinality`, `ElicitationStaleTargetInert`, `ElicitationTimeoutNeitherSuccessNorDenial`, `ElicitationWithdrawalFinality` |
| `specs/seed/snapshot_recovery.qnt` | Quint | `SnapshotStaleRejected`, `SnapshotCrossDomainRejected`, `SnapshotConsistentPrefix`, `LateEventNoRewrite`, `CrashNoAcceptedLost`, `IdempotentLogReplay` |
| `specs/seed/authority.qnt` | Quint | `NoCommandWithoutGrant` (generalizes by refinement to `NoOperationWithoutGrant`), `CompoundIssuer`, `GrantAuthorityIsCommandKinds` (generalizes by vocabulary rename to `GrantAuthorityIsOperationKinds`), `RevocationPreventsFuture`, `SpawnCreatesDescendantGrant`, `FleetAuthorityForSpawn`, `SpawnRevocationDoesNotCascade`, `ElicitationResponderAuthority` |
| `specs/seed/subscription_authority.qnt` | Quint | `SubscriptionAudited`, `SubscriptionCursorReplayAuthorized`, `SubscriptionGrantChecked` |
| `specs/seed/patchbay-relational.als` | Alloy | `ActorIdsUnique` (injectivity obligation; retained check is structural regression only), `AuthorityGraphAcyclic` (reserved — needs delegation, out of v0.1.0), `SenderMatchesClaim` (reserved — dynamic CompoundIssuer binding, belongs in authority.qnt) |

`TimeoutNeitherSuccessNorDenial` is a reserved property-id for a future transport/failure-vocabulary model (not in `command_lifecycle.qnt` — it concerns the submission/transport layer, not command-lifecycle state). `ElicitationTimeoutNeitherSuccessNorDenial` is the Elicitation-specific stated-normative obligation with no executable property formula; its removed formula checked answer/decline fields but did not model grant state.

The Elicitation lifecycle, subscription, spawn-authority, descendant-grant, response-correlation, and command-lifecycle durability/retry properties are all stated-normative with no executable formula: their seed formulas either did not model the claimed failure boundary or inspected state recorded by the accepting action rather than independent attempted evidence (not mutation-survivable oracles). They are not checked-normative product semantics; the v1 formal gate owns their genuine formulas. The eight retained promoted properties (`TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`, `GenerationMonotonic`, `CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority`) are checked-model only and are not checked-normative product semantics until corresponding conformance vectors are promoted.

### Toolchain note (implementation discovery)

Quint temporal properties using `next()` inside `always()` are checked via the **Apalache default backend** (`echo y | quint verify --temporal <p> --max-steps 10`), not `--backend tlc`. The Quint→TLA+ compilation emits `[](...)` forms that TLC rejects with `[] followed by action not of form [A]_v`. Apalache checks these correctly but warns its temporal support is experimental; all checked temporal properties here are `always(...)` safety (not `eventually` liveness), the more conservative end of Apalache's temporal support. Tool versions: Quint 0.32.0, Apalache 0.56.1, Alloy 6.2.0, tla2tools 1.7.4. See `feature-formal-model-seed` Implementation discovery for detail.
