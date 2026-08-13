# Patchbay Protocol

Patchbay protocol semantics are defined around durable operator intent, explicit authority, unambiguous target identity, and recoverable state.

This document defines concepts and required behavior, not a final wire encoding. It is the canonical source of truth for command state, session state, failure vocabulary, and transition semantics — the **product intent and vocabulary naming** authority (see `docs/VERIFICATION.md` Artifact authority order). Wire shape, field identity, and enum encoding are authority of the generated `.proto` contract once it exists; until then this document is the provisional wire reference. Future TypeScript/Rust enums, TLA+/Quint variables, conformance vectors, and UI presentation labels derive from these registries rather than redefining them.

## Actors and endpoints

An **actor** is any represented participant: operator, agent, adapter, daemon, service, or control surface.

A **device** is a physical or virtual host that can run one or more endpoints, such as a browser on a laptop, a CLI on a VM, or an adapter process near a runtime.

An **endpoint** is a concrete connection or addressable runtime instance for an actor on a device. An actor may have multiple endpoints across devices or deployments.

Actors, devices, and endpoints have stable identifiers assigned by Patchbay or verified through an adapter-specific trust root. Human-readable labels are metadata, not routing authority.

An **operator session** is an authenticated browser or CLI session for the human operator. Operator sessions are endpoint-bound server-side records, not bearer authority stored in UI state. v0.1.0 has one human operator, but commands still name and validate the issuing actor, device, endpoint, and operator session so future multi-operator authority domains can extend the model without changing command semantics.

## Sessions

A **session** is an adapter-reported runtime/control target. A session identity binds the fields needed to prevent wrong-target mutation:

- adapter id;
- deployment scope;
- runtime session id (adapter-reported, stable per session generation);
- session generation (adapter-reported, monotonic per session).

These are **runtime-session generations**. They are distinct from an **operator-session generation**, which the core assigns monotonically per operator actor when a browser or CLI authentication session is issued. A durable all-session revocation records an invalidated-through operator-session generation; restart replay preserves that floor, opaque pre-restart session ids are not restored, and the next successful login receives a higher generation. Operator-session generations fence control-surface authentication; runtime-session generations fence adapter-reported replacement and late runtime events. Neither generation is a substitute for the other.

Project, cwd, and name are **metadata**, not identity: they describe the session for operator orientation and display, but they update independently of the identity tuple. A cwd change does not create a new session target, and human-readable labels cannot override verified target identity.

Session generation is adapter-reported because only the adapter can observe external runtime replacement. When the adapter reports a strictly-greater session generation for an existing session id, the core **tombstones** the prior generation: it marks the prior generation superseded at the next LSN, retains it for audit and late-event correlation, and treats the new generation as the live target. Late replies or events binding to a tombstoned generation are `stale_event` audit records; they do not mutate the live generation. This is consistent with the ratified first-durable-terminal-commit rule: late events do not rewrite committed state.

Generation rules:

- Supersession requires a strictly-greater generation. An equal generation does not supersede the session; its report may update mutable state only under the source-order rules below. A lower report is rejected as an audit record and the live generation is left unchanged.
- First registration (no live generation exists) is accepted; monotonicity has nothing to check against.
- The tombstone fact ("generation N existed, superseded at LSN X") is an audit record retained indefinitely. Per-generation detail (full command/event/reply state) is bounded and reclaimable by log compaction. After compaction, an operator querying an aged-out generation gets the tombstone plus any not-yet-compacted detail, with a note that older detail was compacted.
- Late replies or events must bind to the session generation they describe. A reply for an old generation cannot mutate a new generation.

### Session-report source order

Every fresh typed `SessionReport` carries `SessionReportSourceCursor { adapter_generation, revision }`. The cursor orders the complete report—connectivity, activity, project/cwd/name, model, and later report-carried fields—rather than one mutable field at a time. `revision` is positive and strictly increasing within one authenticated adapter generation and runtime-session generation. A strictly newer authenticated adapter generation or runtime-session generation starts a fresh local revision scope.

The current authenticated attachment binds both adapter identity and adapter generation. Missing cursor/generation, revision zero, old attachment evidence, a cursor generation other than the current attachment, and unknown or `UNSPECIFIED` state values reject before session lookup or durable append. For the same runtime-session generation, a cursor is current only when its adapter generation is newer than the projected cursor's producer epoch, or its adapter generation is equal and its revision is strictly greater. Equal/lower revisions and older adapter generations append no session-state mutation, leave every report-carried field and the watermark unchanged, and record `STALE_EVENT_IGNORED` / `stale_event` audit evidence. A lower runtime-session generation remains stale regardless of its producer cursor.

Registration and runtime-generation replacement durably install the report's cursor. Every accepted equal-generation report writes one atomic full-report event, including a newer report whose visible values are unchanged, so restart replay restores the source watermark. Legacy durable registration/generation and field-delta events remain readable with no source cursor until the first revised report establishes one. Core-authored disconnect/lockdown degradation and those legacy deltas preserve the last adapter source cursor; they do not impersonate a producer report.

Adapter source revision is not a core revision. Core `(authority_domain_id, LSN)` remains the total durable arrival/commit order, snapshot revision authority, and event identity; it cannot reveal whether a later-arriving adapter report was produced earlier. Wall-clock timestamps and adapter-local promise-tail serialization likewise are not source authority.

## Operational-resource identity and resolution

An operational resource has the routable identity tuple **`(adapter_id, resource_kind, resource_id)`**. `ResourceId` is adapter-local; `ResourceKind` is an open adapter-owned identifier whose admitted set belongs to the adapter capability manifest. The core does not enumerate adapter resource kinds. A resource carries no runtime-session id, deployment scope, or generation; replacement, revision, and tombstone semantics belong to the resource-state contract.

Operational resource Operations and Grants encode this tuple in `TargetScope.resource`. Protobuf tag 8 remains `legacy_audit_resource_id` solely for durable principal/endpoint/device audit targets; it is not an operational identity and is rejected at Operation, Grant, and resolution boundaries. Mixed flattened/nested target shapes fail validation rather than being normalized.

Ordinary target resolution dispatches by `TargetScopeKind`: runtime sessions retain generation/tombstone resolution, while resources resolve only when their exact typed identity is registered in the authority-domain resource projection. Unknown or malformed resources return `target_not_found`; the core never fabricates runtime-session identity for them. Adapter delivery uses the adapter inside the validated resource tuple. The core-local diagnostics resolver remains a separate authority-domain binding and is not available to ordinary Submit.

A resource-scope Grant contains only the exact adapter/kind/local-id tuple and only a requested `RESOURCE` target. An adapter Grant explicitly contains canonical resources owned by that adapter; fleet and authority-domain Grants retain their existing wider authority-domain-bounded containment. No implicit resource-kind or local-id wildcard exists.

These current resource semantics are implementation-checked by Rust boundary, resolver, delivery-routing, replay-validation, and property tests. Promoted resource-plane conformance vectors and formal assurance remain owned by the resource-plane conformance feature; this implementation evidence does not promote the claim to checked-normative.

## Operations, Observations, Elicitations, payloads, and correlation

Patchbay uses an actor-neutral vocabulary of three primitives — Operation, Observation, and Elicitation — with Payload as content carried inside any of them, not a standalone authority primitive. Every primitive carries `{sender, recipient}` actor fields. v0.1.0 Operations are operator-originated; the actor-neutral sender vocabulary is a reserved seam for non-operator senders (agent→agent, adapter→operator service Operations). v0.1.0 does not mediate non-operator-originated authority-bearing Operations.

### Operation

An **Operation** is an authorized control-plane request by an actor to an actor, core, adapter, fleet, session, service, or resource target. An Operation may be side-effecting, read-only, lifecycle-acting, response-submitting, or fleet-creating. Operations require verified sender identity, target identity/scope, authority evaluation, a registry-owned `OperationKind`, boundary validation, idempotency semantics where applicable, and durable lifecycle state after acceptance.

v0.1.0 Operations are operator-originated. The actor-neutral sender vocabulary is a reserved seam for non-operator senders (agent→agent, adapter→operator service Operations). v0.1.0 does not mediate non-operator-originated authority-bearing Operations. Initial implementation reuses `CommandState` and command ids by refinement equivalence (see `OperationState` ⇿ `CommandState` refinement below); command/message ids stay client-generated in the operator domain per the existing protocol. `Operation` is the actor-neutral vocabulary; `CommandState` is the checked lifecycle registry until the coordinated rename/model update occurs.

### Observation

An **Observation** is a source-authenticated fact, event, output, status emission, reply-like result, or lifecycle/status fact emitted by an actor, adapter, core, runtime, or service. Observations do not grant authority to act. They still require source identity, target/session/generation context where applicable, correlation context when they answer or relate to prior work, and LSN/cursor/snapshot reconciliation if durable.

Live streams are delivery optimizations. Durable core records and snapshots remain the authority for accepted Operations and reconciled state.

### Elicitation

An **Elicitation** is a durable pending response solicitation from one actor/system component to another. It opens a response slot rather than answering a prior request. It carries an adapter-assigned `ElicitationId`, opener, `expected_responder_actor` (the operator actor for committed v0.1.0 human-facing Elicitations), target/session/generation context, `response_contract`, timeout/cancellation/withdrawal policy, correlation to the work that caused it, and terminal lifecycle state. It does **not** carry an `expected_responder_endpoint` or bind to a specific operator-session endpoint in v0.1.0. The core assigns the durable LSN when it records the Elicitation, as for other durable events.

Elicitation delivery rides the subscription layer: the Elicitation is fan-out delivered to every surface with an active, grant-checked subscription to the operator actor's Elicitation stream. Any authenticated endpoint for that operator actor may answer. First-answer-wins terminalizes the Elicitation (`answered` or the applicable terminal) for all subscribed surfaces; later response attempts from other surfaces are rejected as already-terminal/stale candidates and recorded with the same `stale_event` audit treatment used for late terminal candidates. The endpoint that actually answers is captured in the response Operation's audit record at response time; it is not pre-bound in the Elicitation record.

Elicitation is actor-neutral as a future-proof vocabulary: agent→operator questions, harness→client service requests, service→operator secret prompts, and future agent→agent/op→op solicitations use the same primitive when promoted. In v0.1.0, the opener is always an adapter/agent/harness, never the core; agents/adapters open Elicitations such as `AskUserQuestion`, tool-input requests, and approval gates. A response is submitted as an operator-originated `OperationKind = elicitation-response` or `approval-response` Operation correlated to the Elicitation. Two seams are explicit: the responder-binding seam is preserved by v0.1.0's operator-actor binding while endpoint/class/fallback-chain binding remains reserved; the responder-identity audit seam is built by recording the responding endpoint in the response Operation audit, with future multi-operator work adding responder-actor distinction when multiple operators can share a session.

Core prompts are **not** Elicitations. Lockdown, expired/revoked sessions, CSRF rejection, and similar cases are core-imposed states enforced by Operation rejection or pre-protocol operator-session establishment. The protocol assumes a valid operator session exists; login, re-authentication, and lockdown exit are control-surface/web-server concerns outside the normative Operation/Elicitation flow.

### Payload

A **Payload** is the adapter-specific content or schema-bound body carried inside an Operation, Observation, or Elicitation. Examples: prompt text, slash-command text, typed user input entries, tool-call arguments, function results, image/file references, question options, structured schemas, or adapter diagnostics. Payload does not itself grant authority, create lifecycle state, or define protocol kinds.

### Generic Message

Generic operator-originated no-grant `Message` is not a v0.1.0 action. Operator-originated content that drives work is payload of an authorized `instruct` Operation. Agent/harness/service-originated requests for a response are durable Elicitations. The `message id` space remains reserved for future informational surfaces and for current correlation-model compatibility.

### Reply and response correlation

A reply references a prior message or command by typed correlation. A reply is valid only when its correlation reference resolves to a known prior command or message id in the same authority/session context. Duplicate replies are either idempotent or visibly rejected. Response Operations to Elicitations (`approval-response`, `elicitation-response`) use a typed correlation reference to a known `ElicitationId` in the same authority/session/responder context. `TypedCorrelation` reserves both Reply → Command/Message and response Operation → Elicitation typed references across the disjoint id spaces as a **stated-normative** obligation; `reply_correlation.qnt` has no promoted formula until independent attempted correlation evidence is modeled.

## Id spaces

Patchbay uses five separate id spaces, each with a defined assigner, to prevent forgery and enable idempotent retry:

1. **Command id** — client/operator-domain generated today; identity for accepted lifecycle-bearing records. During the vocabulary transition, accepted Operations reuse this id space by refinement equivalence. A future `OperationId` rename is a coordinated artifact rename, not a sixth id space.
2. **Message id** — reserved in v0.1.0 even though generic operator-originated no-grant `Message` drops. It remains in the registry because current `TypedCorrelation` and future non-command informational surfaces may need it.
3. **Reply id** — adapter-or-core assigned for correlated reply/observation records that answer prior command/message/operation context.
4. **Event id** — core-assigned LSN, keyed as `(authority_domain_id, LSN)`.
5. **Elicitation id** — new id space, adapter-assigned when a pending response slot is opened. The core assigns an LSN when it durably records the Elicitation; it does not assign the `ElicitationId` in v0.1.0.

A command id and an idempotency key are **separate fields**: the command id is identity, and the idempotency key is the dedup handle. A retry reuses both; an intentional duplicate action uses a new command id and a new idempotency key.

Forgery-prevention justification:

- A response Operation must not be able to masquerade as the Elicitation it answers. Separate `CommandId`/`ElicitationId` spaces preserve direction: Elicitation opens a pending slot; response Operation answers it.
- A reply id cannot masquerade as command identity; the stated-normative `TypedCorrelation` obligation requires separate id spaces for command/message/reply and same-context typed references.
- `ElicitationId` is not a typed `ReplyId` subkind because an Elicitation is an initiation, while a Reply is a response. Modeling initiation as response inverts semantic direction and confuses lifecycle ownership.
- `TypedCorrelation` reserves response Operation → Elicitation as well as Reply → Command/Message, including same authority/session/responder context and separation of the five id spaces. This obligation is **stated-normative**; the current `reply_correlation.qnt` model has no promoted formula (see `docs/VERIFICATION.md`).

## Canonical state registries

These registries are committed v0.1.0 protocol behavior unless marked as an extension seam. Implementations may add display labels, colors, or adapter-specific metadata, but they must not add protocol states outside the registry without updating this document, contracts, models, and conformance vectors together.

### Command lifecycle state

`CommandState` is durable core state for an accepted command. Control-surface-local states such as `draft` and `submitting` are intentionally excluded from this registry.

| State | Terminal? | Meaning |
|---|---:|---|
| `accepted` | no | Patchbay validated the command, checked authority, deduplicated the idempotency key, and durably recorded the command before delivery. Delivery may not have been attempted yet. |
| `delivered` | no | The target adapter accepted delivery responsibility for the command. This does not imply execution started or completed. |
| `running` | no | The target adapter or runtime reports active execution for the command. |
| `completed` | yes | The command reached a successful semantic completion reported by the authoritative target context. |
| `rejected` | yes | Patchbay or the target adapter refused an already-recorded command before execution as a semantic/policy decision, such as unsupported command, invalid target, or delivery refusal. Pre-acceptance submission refusal is a `SubmissionOutcome`, not `CommandState = rejected`. |
| `failed` | yes | Delivery or execution reached a non-policy error after the command was accepted, such as adapter crash, transport failure after acceptance, runtime error, or unknown execution failure. |
| `expired` | yes | The command exceeded its validity window before reaching a later non-expired terminal state. |
| `cancelled` | yes | Operator or policy cancellation became the authoritative terminal outcome. |
| `superseded` | yes | A newer accepted command or policy explicitly replaced this command, and the old command must no longer be executed or presented as pending work. |

Allowed transitions:

```text
accepted  -> delivered | rejected | failed | expired | cancelled | superseded
delivered -> running | completed | rejected | failed | expired | cancelled | superseded
running   -> completed | failed | expired | cancelled | superseded

completed -> <terminal>
rejected  -> <terminal>
failed    -> <terminal>
expired   -> <terminal>
cancelled -> <terminal>
superseded -> <terminal>
```

Boundary rules:

- `accepted` is the only initial durable `CommandState` for a newly accepted command.
- Terminal states are final for that command id. Late adapter events are recorded as events for audit/reconciliation but do not mutate the command state.
- A duplicate submission with the same command id and idempotency key to the same target, with an identical payload, returns the existing command record and state; it does not create a new state transition. (See § Idempotency and retry for the dedup-scope, payload-equivalence, and key-retention rules. A payload mismatch is rejected at submission with `validation_failed`; a key reused against a different target is treated as a new command.)
- `rejected` means a known actor refused the command by semantics or policy. `failed` means an accepted attempt encountered an error. `expired`, `cancelled`, and `superseded` are distinct terminal outcomes and must not be collapsed into `failed`.

### `OperationState` ⇿ `CommandState` refinement equivalence

`OperationState` is not a new checked model. It reuses `CommandState` by documented refinement: accepted Operations use the existing `CommandState` state names (`accepted`, `delivered`, `running`, `completed`, `rejected`, `failed`, `expired`, `cancelled`, `superseded`). The promoted `command_lifecycle.qnt` properties inherited as checked-model by equivalence are `TerminalFinality`, `BoundaryDedup`, and `NoAcceptedToCompleted`. `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, and `RetryAfterTerminalReturnsExisting` are stated-normative with no executable property formula because the removed formulas did not model the failure boundaries named by those obligations. A future rename from `CommandState` to `OperationState` must update model names, property metadata, `.proto`, conformance vectors, and docs together; until then `CommandState` remains the checked lifecycle registry name and `Operation` is the actor-neutral protocol vocabulary that maps to it.

`command_lifecycle.qnt` uses `allowedTransition` to constrain its actions to the exact transition table above, and `NoAcceptedToCompleted` independently checks the specific no-`accepted → completed` adjacency as **checked-model**. That property does not by itself promote every edge in the table: the full transition graph remains **stated-normative** until independent properties or conformance vectors cover its remaining adjacency rules. Read/query Operations use the same stated-normative lifecycle in v0.1.0; they may skip `running`, but the read/query no-direct-to-completed fast-path rule remains stated-normative. A no-lifecycle reads optimization is a reserved seam, promotable if polling volume warrants it later.

### `OperationKind` registry

One registry owns kinds, lifecycle policy, authority matching, adapter capability mapping, display labels, and generated contract variants. Adding or promoting a kind requires updating this document, `.proto`, model/vectors as applicable, and implementation together.

| `OperationKind` | Meaning | Lifecycle notes | v0.1.0 disposition |
|---|---|---|---|
| `spawn` | Create a new runtime/session/thread/agent/process/cloud resource. v0.1.0 addresses one explicit attached adapter before the session exists; spawn variants are payload `target_spec.shape`, not per-variant OperationKinds. | Full `CommandState` lifecycle by refinement: initial `accepted`; then `delivered`, optional `running`, or terminal. `running` is allowed for long provisioning. | Committed v0.1.0 for adapter-scoped targets. Fleet-supervisor/authority-domain target selection is reserved and does not broadcast in v0.1.0. The `target_spec.shape` registry is reserved/open and adapter-enforced at delivery. |
| `attach` | Connect/reconnect a control surface endpoint to an existing session/server and reconcile. | Full lifecycle by refinement; may skip `running`, but not durable lifecycle. | Committed v0.1.0. |
| `instruct` | Send prompt/user input/steering content into a session/turn. | Full lifecycle allowed: `accepted → delivered → running → terminal`; in-flight steering may skip `running` if adapter reports immediate acceptance. | Committed v0.1.0 for operator-originated instruct. |
| `cancel` | Request cancellation of a target Operation/turn/session action. | Full lifecycle by refinement; the target Operation's terminal race is governed by first durable terminal commit, and cancellation completion does not rewrite an already-terminal target. | Committed v0.1.0. |
| `interrupt` | Request immediate stop/interrupt of active execution. | Same as `cancel`; reserved distinction for adapters that expose softer cancel vs harder interrupt. | Committed v0.1.0. |
| `query` | Read status, snapshot, capabilities, lists, history, metadata, or diagnostics. | Full lifecycle by refinement. Reads may skip `running`, but no v0.1.0 read uses a no-delivery direct-to-completed shortcut. A no-lifecycle read variant is reserved if polling volume warrants it later. | Committed v0.1.0. |
| `approval-response` | Respond to a permission/tool approval Elicitation. | Full lifecycle by refinement. On `completed`, the core decodes the typed `ApprovalResponsePayload.decision`: `APPROVED` selects `answered` and `DENIED` selects `declined`, only after response validation succeeds and first-terminal rules allow. | Committed v0.1.0. |
| `elicitation-response` | Respond to non-approval Elicitations. | Full lifecycle by refinement. Invalid response Operation is rejected unless explicit Elicitation policy terminalizes the slot. | Committed v0.1.0 for `question` contracts; reserved for `freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, and `service_request` contracts. |
| `reconfigure` | Change model, reasoning/thinking level, permission mode, tools/MCP, agent mode, workspace, or adapter config. | Full lifecycle by refinement; `running` only for adapters with long reconfiguration. | Committed v0.1.0. |
| `session-management` | Resume, fork, compact, clear, archive/delete, revert, share/unshare, remove messages, checkpoint restore, disconnect/retire existing sessions/resources. | Full lifecycle by refinement because compaction/archive/delete can be long-running; quick local actions may skip `running`. | Committed v0.1.0. |
| `agent-send` | Reserved design seam for agent→agent mesh, op→op routing, adapter→operator service Operations, and other non-operator Operation directions. Informed by remote-pi mesh `agent_send`/`agent_request` prior art (not one of the 7 surveyed harnesses) and by Antigravity trigger / Codex service-request pressure. | Not validatable in v0.1.0. If submitted in v0.1.0, rejected before acceptance. | Reserved seam; v0.1.0 submissions reject with `validation_failed`. |
| `adapter-utility-exec` | Reserved seam for standalone adapter utility execution that does not create a thread/turn or persistent runtime session. Codex `command/exec` and `process/spawn` are the surveyed pressure `[codex-appserver-protocol]{5}` `[codex-appserver-types]{9}` `[codex-appserver-types]{10}`. | Not validatable in v0.1.0. If submitted in v0.1.0, rejected before acceptance; full lifecycle/idempotency modeling deferred. | Reserved seam; named in registry, not validatable in v0.1.0; submissions reject with `validation_failed`. Full lifecycle/idempotency modeling deferred. |

Boundary rules:

- Unknown `OperationKind` is `SubmissionOutcome = rejected` with `validation_failed` before grant evaluation.
- Reserved-but-not-validatable kinds such as `agent-send` and `adapter-utility-exec` also reject with `validation_failed` in v0.1.0. Promotion is a registry update, not a schema change.
- Unsupported-by-adapter known committed kind is a delivery-layer `unsupported_command` rejection after acceptance, matching the existing capability posture (the core does not gate delivery on cached adapter capability).

#### Spawn payload and authority commitments

- **One `spawn` OperationKind with disjoint generated intents.** `SpawnRequest.intent` is exactly one of `fresh` or `continuation`; worktree, same-dir, session, process, thread, local sidecar, and cloud-environment spawns are not separate OperationKinds in v0.1.0. A continuation carries one exact `RuntimeGenerationRef` for the prior logical target and external runtime. Missing/wildcard identity, generation zero, and a prior generation that cannot advance reject as `validation_failed` before stateful work. Per-variant OperationKinds are reserved only if a future registry update promotes them.
- **`target_spec.shape` = reserved open shape registry.** The generated spawn payload includes bounded `target_spec.shape`, optional typed `adapter_payload`, and an optional opaque `deployment_authority_ref`. Project, cwd, adapter-native paths, and that authority reference remain adapter-owned metadata/configuration; none becomes logical identity, routing authority, or a substitute for a Patchbay Grant. v0.1.0 names shapes for vocabulary, audit, and display (for example, "spawned a worktree") but does not validate shape support at the protocol layer. The adapter capability manifest declares which shapes the adapter supports; the adapter accepts or rejects the accepted Operation at delivery time with `unsupported_command`, consistent with the capability-not-authority discipline.
- **Continuation authority is compound.** Fresh spawn requires the selected adapter-scoped `spawn` Grant. Continuation also requires a selected live `session-management` Grant scoped to the exact prior generation for the same verified subject, endpoint, and authority domain. Durable `ContinuationAuthorityProvenance` repeats the exact prior, replacement Grant id, and canonical authority kind; it composes with the accepted operation's existing `authorizing_grant_id`. Descendant provenance preserves the same two-Grant chain. A broad adapter-spawn Grant alone is not continuation authority.
- **Target scope = one explicit adapter in v0.1.0.** The operation-aware target resolver admits only a canonical adapter scope whose adapter has registered through attachment. Runtime-session and operational-resource scopes are incompatible with creation and reject before durable acceptance. Fleet-supervisor and authority-domain scope values remain wire/model seams, but their default-adapter selection policy is reserved; v0.1.0 neither resolves nor broadcasts them.
- **Per-variant authority is reserved.** v0.1.0 does not implement "may spawn worktrees but not cloud environments" authority. If needed later, per-variant authority can be expressed through grant `target scope` or by promoting spawn variants to distinct OperationKinds; both are reserved seams, not v0.1.0 behavior.
- **Descendant authority = spawned-session manifest.** Spawn completion includes an auto-issued grant record for the spawned session. This is an explicit, operator-visible, auditable grant record generated as part of spawn, not an implicit grant-matching rule. The descendant grant record is a normal grant instance with:
  - `grant id` — standard grant id (core-assigned).
  - `authority domain id` — same domain as the spawning grant.
  - `subject actor id` — the spawner (operator actor in v0.1.0).
  - `optional subject endpoint id or endpoint class` — the spawning endpoint, if applicable.
  - `target scope` — the spawned session/generation (an existing-session scope, now that the session exists).
  - `allowed OperationKinds` — the full set of committed kinds applicable to an existing session, enumerated explicitly (not a wildcard `all`): `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management`. `spawn` is excluded because recursive spawning requires a separate adapter-scoped spawn grant; `attach` is excluded because the spawned session is already attached to its spawner's control plane.
  - `creation time and provenance` — fresh spawn records `{ spawn_operation_id, spawning_grant_id }`; continuation additionally records `{ exact_prior, replacement_grant_id, replacement_authority_kind = session-management }`, preserving both selected Grant ids and the exact replaced generation.
  - `optional expiration` — none by default (the descendant grant lives until revoked or the session is retired).
  - `revocation generation or revoked time` — standard; revocable independently of the spawn grant (two-lever rule, no cascade).
  - `revocation policy for already accepted commands` — standard.
  - `audit id` — links to the spawn-completion audit event.
- **Delegation remains out of v0.1.0.** The auto-issued descendant grant is same actor (operator), new target (spawned session), not cross-actor delegation. No delegation lineage field is present in the v0.1.0 descendant grant. The reserved future direction is to inherit descendant allowed kinds from the spawning grant for delegation-aware authority; that future work must be designed with multi-operator / federated-authority semantics before use.
- **Revocation uses two independent levers.** Revoking the spawn grant prevents future spawns. Already-spawned sessions keep operating under their auto-issued descendant grant until that grant is separately revoked. No cascade-revoke is v0.1.0 behavior; future cascade is a query over grant provenance and needs no schema change.
- **Idempotency is capability-manifest declared.** Spawn uses the adapter capability manifest's `idempotency strength` field (`none` / `at-Patchbay-boundary` / `end-to-end`). Most adapters likely declare `at-Patchbay-boundary`: Patchbay deduplicates the Operation record, while adapter retry may still create a duplicate external process. Adapters that track spawn idempotency internally may declare `end-to-end`. Duplicate external process reporting maps through the failure vocabulary; Patchbay does not solve adapter-side process duplication beyond its boundary dedup.

### Submission outcome and local submission state

A submission is the request to create or retrieve a command record. Not every submission creates a durable command. Pre-acceptance refusal is represented as `SubmissionOutcome = rejected`; it is not `CommandState = rejected` unless an explicit audit policy creates a separate non-command audit record.

`SubmissionOutcome` is the boundary result returned by Patchbay for a submission attempt:

| Outcome | Meaning |
|---|---|
| `accepted` | Patchbay created or found a durable command record. The returned command id has `CommandState = accepted` or the existing deduplicated command state. |
| `rejected` | Patchbay refused the submission before creating a command record, such as validation failure, authorization denial, an unknown-to-Patchbay OperationKind, or invalid target known before acceptance. |
| `failed` | Patchbay could not complete the submission attempt due to service or transport failure. The client must not infer acceptance. |
| `unknown` | The client cannot determine whether Patchbay accepted the submission and must reconcile by idempotency key or snapshot. |

`LocalSubmissionState` exists only inside a control surface before or while it reconciles with Patchbay. It is not persisted as durable command state.

### Security lockdown posture

Security lockdown is a domain-keyed durable posture, not an `OperationKind` or command state. While active, all `ControlService.Submit` and `QueryDiagnostics` Operations are refused before acceptance with `SubmissionOutcome = rejected`, `FailureCode = authorization_denied`, and reason `security_lockdown_active`; no command record is appended. This includes exact retries and every committed OperationKind. Already accepted Operations may continue through their existing lifecycle, and adapter observations/reports are still accepted for reconciliation but cannot restore a runtime session to `live` while the posture is active.

The named non-Operation read exceptions are `Subscribe`, `LoadSnapshot`, and `LoadSecuritySnapshot`; fresh `VerifyOperatorPassword` login, logout/current-session revocation, and required audit ingress also remain available. A fresh login may inspect read-only snapshots/subscriptions but cannot submit Operations or perform security mutations until exit. `EnterSecurityLockdown` requires the authenticated authority-domain/session-management grant. `ExitSecurityLockdown` is present only on the loopback bootstrap `AdminService`; v0.1.0's CLI surface is `patchbay-cli lockdown-exit`, and no routine web bridge exists. Entry/exit source events and `LOCKDOWN_ENTERED`/`LOCKDOWN_EXITED` audit records commit atomically. Persisted reasons are bounded `[a-z0-9_]{1,64}` codes only; snapshots and audit projections do not expose bearer/session secrets.

| State | Meaning |
|---|---|
| `draft` | Local-only operator input that has not been submitted to Patchbay. It may be edited or discarded without protocol history. |
| `submitting` | The control surface sent a submission and is waiting for a `SubmissionOutcome`. |
| `submit_failed` | The control surface received or inferred `SubmissionOutcome = failed`. The operator may retry with the same idempotency key. |
| `unknown` | The control surface received or inferred `SubmissionOutcome = unknown` and must reconcile by querying Patchbay before claiming success or failure. |

Allowed local transitions:

```text
draft        -> submitting
submitting   -> draft | submit_failed | unknown | <reconciled command id> | <rejected submission>
submit_failed -> submitting | draft
unknown      -> submitting | submit_failed | <reconciled command id> | <rejected submission>
```

`<reconciled command id>` and `<rejected submission>` are exits from local submission state, not additional enum members. Once Patchbay returns or snapshots a command id, the UI derives command display from `CommandState`. A UI may still show local transport decoration, but durable truth comes from the core command record.

### Session state axes

Session presentation is the composition of two protocol axes. This avoids treating “live”, “idle”, “working”, “stale”, and “unknown” as one overloaded enum.

#### `SessionConnectivityState`

| State | Meaning |
|---|---|
| `live` | Patchbay has a sufficiently fresh authoritative signal that the adapter/session endpoint is reachable. |
| `stale` | Cached state exists, but Patchbay lacks a sufficiently fresh authoritative signal. Stale data must not be rendered as live. |
| `offline` | Patchbay has authoritative evidence that the adapter/session endpoint is unavailable. |
| `unknown` | Patchbay lacks enough information to classify the session as live, stale, or offline. |
| `failed` | Patchbay has an explicit adapter/session error that prevents reliable control or observation. |

#### `SessionActivityState`

| State | Meaning |
|---|---|
| `idle` | The session is not reporting active work. |
| `working` | The session reports active work, command execution, or adapter-known runtime activity. |
| `unknown` | Patchbay lacks a current authoritative activity report. |

Allowed connectivity observations:

```text
unknown -> live | stale | offline | failed
live    -> stale | offline | failed
stale   -> live | offline | unknown | failed
offline -> live | stale | unknown | failed
failed  -> live | stale | offline | unknown
```

Allowed activity observations:

```text
unknown -> idle | working
idle    -> working | unknown
working -> idle | unknown
```

Session transitions are driven by authoritative adapter events, timeout/staleness policy, and snapshots. Snapshots may move an axis to any state allowed above when they carry fresher authority than cached UI state.

A session also carries an adapter-reported **current model**: an opaque `provider/modelId` string (for example `kimi-coding/k3`). It is mutable non-identity metadata: its empty wire value means unavailable/unknown, and it never participates in routing, authority, or the session identity tuple. Registration and generation replacement carry the current model; accepted equal-generation adapter reports update it inside the atomic full-report event governed by the source cursor above. Legacy/core-authored `SessionModelChanged { identity, from, to }` deltas remain replayable and validate the full identity tuple and prior `from` value without advancing adapter source order. Snapshots materialize both the current model and last source cursor. Rich structured model descriptors and separate model-history projections are reserved seams.

Derived UI labels such as “Live idle”, “Working”, “Stale working”, or “Offline” are presentation labels over these axes, not protocol states. A stale or unknown connectivity value dominates presentation: stale working is not live working.

### `ElicitationState` lifecycle

`ElicitationState` is a new registry, not a projection of `CommandState`. Its lifecycle properties — `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationCorrelationTyped`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`, and `ElicitationTimeoutNeitherSuccessNorDenial` — are **stated-normative** with no promoted formula until independent attempted-event evidence and the timeout grant boundary are modeled (see `docs/VERIFICATION.md`). Elicitation ids are adapter-assigned in v0.1.0 and the core does not open Elicitations in v0.1.0.

| State | Terminal? | Meaning |
|---|---:|---|
| `opened` | no | Core durably recorded the Elicitation, but it may not yet be visible through subscription fan-out to the expected responder actor's subscribed surfaces. |
| `pending` | no | The Elicitation is visible on one or more subscribed surfaces for the expected responder actor and can accept a valid response Operation from any authenticated endpoint for that actor. |
| `answered` | yes | A valid response Operation satisfied the contract and first durable terminal commit selected it as the answer, terminalizing the slot for all surfaces. |
| `declined` | yes | The expected responder answered with a valid declining decision: the response slot was satisfied with negative valence. Covers approval denial when the response contract treats denial as terminal. |
| `expired` | yes | The response window closed before another terminal state won. |
| `cancelled` | yes | Core/operator/policy cancelled the pending slot from the responder/control-plane side. |
| `withdrawn` | yes | The opener withdrew the solicitation before it was answered, e.g. the tool call was no longer needed. |
| `superseded` | yes | A newer Elicitation or policy explicitly replaced this one. |
| `stale` | yes | The target/session/generation/opener context became stale or orphaned; responses must no longer mutate live state. |

Operator `ElicitationState = declined` is an answer to the response slot; it is distinct from machine-level `CommandState = rejected`, where Patchbay or an adapter refuses a command. A rejected response Operation never terminalizes an Elicitation: the response itself failed and the slot remains available for another valid response.

Allowed transitions:

```text
opened  -> pending | answered | declined | expired | cancelled | withdrawn | superseded | stale
pending -> answered | declined | expired | cancelled | withdrawn | superseded | stale

answered   -> <terminal>
declined   -> <terminal>
expired    -> <terminal>
cancelled  -> <terminal>
withdrawn  -> <terminal>
superseded -> <terminal>
stale      -> <terminal>
```

Rules:

- `opened` is the only initial durable `ElicitationState`.
- First durable terminal commit wins. Later answer/decline/expire/cancel/withdraw/supersede/stale candidates become audit/reconciliation observations and do not rewrite state.
- First valid answer wins for single-answer contracts. When one subscribed surface answers, the Elicitation terminalizes for all surfaces; subsequent response attempts from other surfaces are rejected as already answered/terminal and audited as stale late terminal candidates. Multi-answer contracts are reserved; they must define completion policy in `response_contract` before use.

Terminal `ElicitationState` transitions are delivered on the same authorized Elicitation subscription stream as `opened`/`pending` (consistent with the Presence/Subscription model in [`§ Presence and Subscription`](#presence-and-subscription)). A surface that misses the terminal transition (e.g., it was offline when another surface answered) reconciles through cursor replay and/or snapshot repair on reconnect — the terminal state is part of the durable Elicitation record. A late second answer from a lagging surface arrives after the Elicitation is already terminal; it is rejected as a stale late terminal candidate (audited, does not rewrite state), and that rejection plus the terminal transition on the stream is what forces the lagging surface to resync to the `answered` (or other terminal) state.

- A response Operation must reference the `ElicitationId` with a typed correlation, must satisfy the active `response_contract`, and must be issued by an authenticated endpoint for the `expected_responder_actor` in v0.1.0. The responding endpoint is captured in the response Operation audit for debugging.
- Invalid response behavior in v0.1.0 is **reject the response Operation** (`SubmissionOutcome = rejected`) and leave the Elicitation `pending`. The terminal-on-invalid policy values (`terminal_declined`, `terminal_superseded`, and `terminal_cancelled`) are reserved seams: v0.1.0 treats them as `reject_and_keep_pending`; a future promotion must define and test the terminal transition.
- No-answer is not an Operation. It is either continued `pending` or a terminal policy event such as `expired`, `cancelled`, `withdrawn`, or `stale`.
- `answered` does not imply the underlying tool/action succeeded; it only means the response slot was satisfied. Subsequent work emits Operations/Observations as usual.

Reserved future Elicitation shapes: multi-responder quorum Elicitations; multi-answer accumulation; tighter responder binding to a specific endpoint, endpoint class, or fallback chain; delegated responder policy; escalation from one expected responder actor to another; surface-reject (an operator surface signals that it cannot handle an Elicitation), distinct from operator approve/decline and from machine command rejection — v0.1.0 leaves an unrenderable Elicitation pending until timeout or withdrawal; cryptographic secret-entry envelopes; large file/attachment upload protocol; drawing/region-selection UI hints.

### `response_contract` registry

A `response_contract` describes what kind of response is semantically required; optional UI hints describe how a surface may render it. The `elicitation-response` OperationKind is committed v0.1.0. The `response_contract.contract_kind` values have a committed/reserved split: committed v0.1.0 contract kinds are `approval` and `question`; reserved contract kinds are named in the registry but not validatable in v0.1.0. `freeform` is reserved because the solid surveyed grounding is currently Claude's optional `AskUserQuestion` freeform answer, while other surveyed response surfaces are structured question/answer, approval, secret, function-result, or service-request shapes rather than standalone unstructured Elicitation responses. A response submitted for an unknown or reserved/unsupported `contract_kind` is rejected at submission with `validation_failed` unless a later registry update promotes that contract kind.

Required fields:

- `contract_kind` — registry variant below;
- `schema_ref` or inline schema where structured validation is required. For committed v0.1.0 `approval` and `question` kinds, the typed contract body is authoritative and v0.1.0 validation ignores `schema_ref`; `schema_ref` is load-bearing only for the reserved `structured_schema` contract kind;
- `ui_hints` — optional list such as `select-one`, `select-many`, `free-text`, `secret-input`, `upload`, `draw`, `confirm`, `diff-review`;
- `timeout_policy`;
- `invalid_response_policy`;
- `responder_policy` — v0.1.0 `expected_responder_actor` (operator actor for committed human-facing Elicitations); endpoint class, service role, fallback chain, and tighter binding are reserved. The responding endpoint is recorded in the response Operation audit, not pre-bound in the Elicitation;
- `sensitivity` — whether raw response may be logged, redacted, encrypted, or never persisted in plaintext.

| `contract_kind` | Semantics | v0.1.0 disposition |
|---|---|---|
| `approval` | Permission response. v0.1.0 commits the binary `ApprovalDecision` values `APPROVED` and `DENIED`; the richer decisions (`ALLOW_ONCE`, `ALWAYS`, `POLICY_AMEND`, `MODIFIED_INPUT`) are named in the `ApprovalDecision` enum but reserved — a response carrying one rejects with `validation_failed` until promotion. | Committed v0.1.0 (binary decisions); richer decisions reserved. |
| `question` | Answer a single question with a typed `QuestionContract` (select-one option or free-text); multi-answer accumulation is a reserved seam. | Committed v0.1.0. |
| `freeform` | Unstructured text response. | Reserved seam. Named in registry, not validatable in v0.1.0; demoted until more than Claude's optional freeform answer is grounded as a genuine Elicitation response surface. |
| `secret` | Provide sensitive secret/token/input with redaction/no-log policy. | Reserved seam. Named in registry, not validatable in v0.1.0. |
| `function_result` | Return custom tool/function result to a waiting service/harness. | Reserved seam. Named in registry, not validatable in v0.1.0. |
| `file_attachment` | Provide file/blob/image/attachment reference or upload. | Reserved seam. Named in registry, not validatable in v0.1.0. |
| `structured_schema` | Response must validate against declared JSON/protobuf/schema. | Reserved seam. Named in registry, not validatable in v0.1.0. |
| `service_request` | Non-human service response such as current time, attestation generation, auth refresh, or adapter-provided evidence. | Reserved seam. Named in registry, not validatable in v0.1.0. |

UI hints are optional sub-fields of `question` and `approval` contracts, not contract kinds. Examples include `select-one`, `select-many`, `free-text`, `upload`, and `draw`; the set is intentionally open and reserved for UI/adapters. UI hints are non-authoritative: changing a prompt from select-one to free-text does not change the protocol contract kind.

### Failure and outcome vocabulary

Failures are layer-aware. Use the narrowest term that matches the authoritative event.

| Term | Layer | Meaning | Typical command effect |
|---|---|---|---|
| `validation_failed` | submission | Patchbay rejected the payload envelope, OperationKind, target identity/scope envelope, or required field before acceptance. This does not include spawn `target_spec.shape` variant support, which the adapter enforces at delivery with `unsupported_command`. | `SubmissionOutcome = rejected`; no `CommandState` |
| `authorization_denied` | submission | The actor/endpoint lacks a valid grant for the command. | `SubmissionOutcome = rejected`; no `CommandState` |
| `target_not_found` | submission/delivery | The addressed actor/session/resource does not exist in the relevant authority/session context. | submission `rejected` before acceptance, or command `rejected`/`failed` after acceptance by policy |
| `unsupported_command` | delivery | The adapter does not support the declared OperationKind at delivery time. (An unknown-to-Patchbay OperationKind is `validation_failed` at submission, before a grant is evaluated; the core does not gate delivery on cached adapter capability.) | command `rejected` after acceptance |
| `target_offline` | delivery | The target is known unavailable. | `failed` or `expired`, depending on command policy |
| `adapter_unavailable` | delivery | The adapter required for delivery is unavailable. | `failed` or remains `accepted` until retry/expiration policy resolves |
| `transport_timeout` | submission/delivery | A transport layer did not answer within its timeout. Timeout never implies success or denial. | local `unknown`/`submit_failed`, or durable `failed`/continued `accepted` by policy |
| `delivery_rejected` | delivery | The adapter received the command but refused delivery responsibility. | `rejected` |
| `execution_failed` | execution | The target began or accepted execution and reported failure. | `failed` |
| `execution_outcome_unknown` | execution | The target may have begun or completed execution, but Patchbay cannot determine the outcome (e.g. adapter crash after execute-before-ack, transport loss after delivery). The command transitions to `failed`; the ambiguity is surfaced to control surfaces so retry safety can be evaluated against the adapter's `idempotency_strength`. | `failed` (with ambiguity signal) |
| `expired` | policy/time | The command validity window closed. | `expired` |
| `cancelled` | policy/operator | Cancellation became the authoritative result. | `cancelled` |
| `superseded` | policy/operator | A newer command or policy replaced this command. | `superseded` |
| `stale_event` | reconciliation | A late event refers to an old command/session generation or terminal command. | audit record only; no state mutation |

### Spawn execution and external-effect evidence

An accepted spawn generation claim changes independently from `CommandState`. The only durable evidence family permitted to release or poison that claim is `SpawnExecutionEvidence`, stored under its dedicated event discriminator. It carries the complete exact `SpawnGenerationClaim`, current durable adapter-attachment provenance, a typed phase, an external-effect disposition, canonical failure code, and an optional exact candidate runtime. Adapter ingress replaces producer and attachment fields from the authenticated current attachment; replay revalidates that attachment against the latest durable registration. This proves source/correlation, not that a malicious or buggy authenticated adapter reported external reality honestly.

The phase/disposition registry is closed:

| `SpawnExecutionPhase` | Allowed `ExternalEffectDisposition` | Claim consequence on failure evidence |
|---|---|---|
| `accepted_not_offered` | `proved_none` | release only through the atomic core pre-delivery terminal proof |
| `offered` | `proved_none`, `may_exist` | explicit refusal-before-responsibility may release; delivered cancellation/expiry or ambiguity poisons |
| `quiescing_prior` | `proved_none`, `may_exist` | exact supervisor/journal pre-launch proof may release after prior-N liveness is renewed; ambiguity poisons |
| `prior_terminated` | `proved_none`, `may_exist` | exact supervisor/journal pre-launch proof may release after prior-N liveness is renewed; ambiguity poisons |
| `launch_attempted` | `may_exist`, `identified` | poison and retain the replacement fence |
| `external_identity_known` | `identified` | poison on failure and retain exact candidate identity |
| `handshake_reconciling` | `identified` | poison on failure; candidate remains staged, never inferred live |
| `success_evidence_reported` | `identified` | claim remains active until the separate atomic promotion contract commits |

The no-effect proof oneof has exactly three variants: (1) an atomic core-side terminal/fence decision while still `accepted_not_offered`, carrying the exact referenced `CommandTransition` whose command id, accepted pre-state, safe terminal outcome, failure code, and pre-offer ordering replay validates; (2) current authenticated-adapter refusal at `offered`, explicitly before delivery responsibility; or (3) current-adapter supervisor/journal evidence for the exact claim at an allowed pre-launch phase. A continuation additionally needs typed exact prior-N `live` session evidence before its fence clears. A poisoned claim requires a referenced no-effect proof later than the poisoning decision, and any later delivery/running, launch-attempted, identified-runtime, or ambiguity evidence invalidates an older proof through the release LSN. Silence, absence of a delivered acknowledgement, a generic or uncorrelated terminal command event, wrong-kind bytes, another claim, pre-acceptance evidence, stale attachment evidence, phase mismatch, delivered cancellation/expiry, and `execution_outcome_unknown` are never no-effect proof. `SpawnPromotionCommitted` remains a separate guarded event family; spawn execution evidence cannot promote a claim.

Adapter diagnostic reporting is a committed post-v0.1.0 capability extension: an adapter declares bounded `[a-z0-9_]{1,64}` diagnostic codes, and sends a typed `AdapterDiagnosticPayload` through `AdapterControlService.ReportDiagnostics`. The core records the authenticated source as one `Observation` plus one correlated `ADAPTER_DIAGNOSTIC_REPORTED` audit record in the same authority-domain append. Warning/error reports carry a canonical failure vocabulary value; informational reports may use `UNSPECIFIED`. These reports are evidence only: diagnostic cadence, timestamps, and silence never establish adapter/session liveness; heartbeat and last-report-age policy remain reserved. Adapter-specific codes are not core failure enum members.

## Acceptance semantics

Patchbay distinguishes acceptance from delivery and completion.

A command accepted by Patchbay is durably recorded before delivery. After acceptance, it remains visible as a `CommandState` until and after it reaches a terminal state. An accepted command cannot disappear silently.

Acceptance creates a command record only after boundary validation, authority checking, idempotency reconciliation, and target identity binding. Invalid submissions that fail before acceptance return `SubmissionOutcome = rejected` without creating durable command state. Audit policy may record rejected attempts, but those audit records are not command records and do not use `CommandState`.

### Operation validity windows

Every v0.1.0 Operation submission carries both `submitted_at` and a `validity_window = [starts_at, expires_at)`. The interval is half-open: the start instant is valid and the expiry instant is not. `starts_at` must be strictly earlier than `expires_at`, and `submitted_at` must lie inside the same half-open interval.

The core samples its injected acceptance clock once per submission and validates the window before grant evaluation, target resolution, idempotency lookup, or durable append:

- `core_now < starts_at` is not-yet-valid intent and rejects with `SubmissionOutcome = rejected` / `validation_failed`;
- `core_now >= expires_at` is expired intent and rejects with `SubmissionOutcome = rejected` / `expired`;
- `submitted_at > core_now`, a missing timestamp/window bound, an invalid Protobuf timestamp, a reversed/empty interval, or `submitted_at` outside the interval rejects with `validation_failed`;
- no rejected validity candidate creates or delivers a command record. A repeated request is revalidated at its new arrival time, so a retry after the original window expires is rejected before deduplication and cannot reactivate the intent.

v0.1.0 applies **zero clock-skew tolerance** at this boundary. This is deliberate for the committed colocated topology: the authenticated web server overwrites browser-provided timing with its own ingress time, while the colocated CLI stamps its Operation immediately before submission. Direct protocol clients must populate the same fields from their local submission time. A future split-deployment transport must explicitly promote a skew policy rather than silently widening acceptance.

The default control-surface window is five minutes: `submitted_at = starts_at = surface_now` and `expires_at = surface_now + 5 minutes`. A flow may choose a shorter window when its semantics require it; longer defaults require a protocol-policy update because they widen replay exposure.

## Idempotency and retry

Commands are deduplicated at the Patchbay acceptance boundary. Retrying the same command id and idempotency key returns the existing command record and does not create a new accepted record at the boundary. This is a boundary guarantee, not an end-to-end execution guarantee: an adapter that does not track idempotency internally may execute the same logical Operation more than once on retry, and that adapter-side behavior is governed by the adapter's declared `idempotency_strength` capability, not by Patchbay's boundary dedup.

**Idempotency-key dedup scope.** A key dedups only against existing commands to the same target. A retry is always a retry of the same command to the same target; a key reused across different targets does not dedup and is treated as a new command. (The checked `command_lifecycle.qnt` models the dedup handle abstractly as `appliedKeys`; per-target scoping is the protocol-level refinement of what that set represents.)

**Payload equivalence.** A retry must carry the same payload as the original. A submission arriving with an idempotency key already applied to a command to the same target, but with a non-identical payload, is rejected at submission with `validation_failed` before acceptance. An intentional duplicate action uses a new command id and a new idempotency key.

**Key retention.** An idempotency key stays dedup-eligible at least until its command reaches a terminal `CommandState`; this protocol obligation is what makes a retry before or at terminal return the existing record. `RetryAfterTerminalReturnsExisting` is stated-normative with no executable property formula; the removed terminal-stasis formula did not model returned-record identity or candidate creation. Whether a later submission reusing a key whose command is already terminal is treated as a duplicate of that terminal record or as a new command is an implementation-defined post-terminal policy, not a protocol constant.

Adapters that cannot guarantee idempotent external execution must report that limitation as a capability constraint (`idempotency_strength`). Patchbay still deduplicates at the coordination boundary and exposes the adapter limitation to control surfaces.

## Cancellation, expiration, supersession, and race semantics

Cancellation, expiration, supersession, adapter completion, adapter failure, and adapter rejection are terminal candidates competing for the same command's first durable terminal transition.

- Running is non-terminal. A running command remains observable until one terminal state wins.
- First durable terminal commit wins. The core assigns a total order to accepted state-transition events in the durable event log; the earliest committed valid terminal transition becomes authoritative.
- If two terminal candidates are truly concurrent before persistence, models may treat the winner as nondeterministic, but implementations must persist one total order and expose the chosen terminal state consistently in snapshots and conformance traces.
- Later conflicting terminal candidates are audit/reconciliation events, not `CommandState` rewrites and not a distinct durable conflict state.
- Cancellation is a command or policy request that races with execution. If `completed`, `failed`, `expired`, `cancelled`, `superseded`, or another terminal state is committed first, later cancellation cannot mutate that command. Depending on the API shape, the too-late cancellation attempt may be refused before acceptance, recorded as an ineffective cancellation failure, or preserved as an audit event; it never rewrites the original command's terminal state.
- Expiration is evaluated against the command validity window. If expiration wins before a later terminal outcome is committed, the command becomes `expired`; if a terminal outcome wins first, expiration does not rewrite history.
- Supersession requires an explicit replacement relationship to a newer accepted command or policy decision. Supersession is not a synonym for cancellation or failure, and a late supersession candidate does not rewrite an already-terminal command.
- Global priority ordering such as `cancelled > completed` is not part of v0.1.0's generic command lifecycle. Future safety-critical OperationKinds may define explicit abort or fencing policy, but that policy must be designed as OperationKind behavior rather than a hidden override of terminal-state finality.

## Snapshots and streams

Event streams are useful but not authoritative by themselves.

A snapshot is an authoritative state view for an actor, session, command, lease, or resource. Control surfaces reconcile against snapshots after reconnect, resume, tab restore, app restart, or suspected drift.

Snapshots expose the canonical state axes above. Stale cached state must not render as live state.

### Revisions and cursors

The coordination core owns a single totally-ordered durable event log per authority domain. Every accepted state-transition event is assigned a monotonic, gap-free **log sequence number** (`LSN`) at durable-commit time. The `LSN` is the canonical ordering for first-terminal-commit-wins and for snapshot reconciliation.

Event, cursor, and revision identity is the **`(authority_domain_id, LSN)` tuple**, not a bare LSN. v0.1.0 has one authority domain, so in practice every key carries the same domain id — but the *shape* of the durable key includes the domain demarcator. This is forward-compatibility hygiene: when federation arrives, cross-domain coordination becomes a layer on top of the per-domain keys, not a data migration that retroactively attaches domains to historical events. Hybrid logical clocks (HLC) / logical-clock abstraction was considered for cross-domain federation and deferred as premature; the per-domain key shape is the federation seam, not a blocker to it.

A **revision** is the `LSN` at which a specific view (command, session, actor, grant, audit record) was last durably updated. A **cursor** is an `LSN` a control surface or adapter holds to express "I have authoritative knowledge up to here."

v0.1.0 revision/cursor rules:

- Every snapshot carries the `LSN` it was materialized at and the per-view revisions it reflects.
- A control surface reconciles by submitting its cursor; the core returns events with `LSN > cursor` and/or a snapshot materialized at a later `LSN`.
- A snapshot with an `LSN` strictly less than the core's current state for that view is **older** and is rejected as an authority source; the core returns the current view instead.
- `core_generation` is a core-assigned, nonzero, opaque storage-continuity epoch persisted per authority domain when the storage lineage is first opened. It remains stable across ordinary process restarts; equality is a compatibility fence, not an ordering, authorization, or bearer-secret mechanism.
- A durable session checkpoint uses a private typed, versioned envelope. It is compatible only when the envelope identifies the supported session-checkpoint kind/version, the storage row's `EventId` has the current authority domain and a positive LSN, the embedded authority domain and nonzero core generation exactly equal the current durable values, and the embedded snapshot LSN exactly equals the storage-row LSN. Legacy undiscriminated bytes, another projection type, an unsupported version, missing/zero/mismatched anchors, or an undecodable payload make the checkpoint unusable and the core repairs from current log materialization or full replay. Compatibility does not establish freshness: an otherwise compatible older snapshot remains rejected by the separate per-view LSN rule above.
- Late events carry the `LSN` at which they were committed. The operational-resource projection classifies structurally valid re-feed against its whole validated applied prefix: an event at or below that prefix is inert and does not rewrite any resource/view state, while an event beyond the prefix must be the exact next LSN and satisfy the new-event fold. Other projections retain their own documented late-event audit/reconciliation rules.
- The core may serve a compressed snapshot at any `LSN`; cursors remain valid across compaction because revisions are monotonic.

### Operational-resource state and reconciliation

Operational resources use a separate revisioned `ResourceRegistry` projection,
keyed by the exact `ResourceIdentity`; they do not inherit runtime-session
generation or connectivity/activity. Each record carries schema-bound resource
and domain-projection envelopes, source adapter generation, observed time, the
LSN that last revised it, and Patchbay reconciliation freshness:

- `current` — the cached resource payload is confirmed by accepted current
  adapter evidence;
- `stale` — cached payload exists but is not confirmed by current evidence;
- `unknown` — Patchbay has no payload it can honestly classify as current.

Freshness is confidence in the cache, not adapter-owned domain health. Provider
exhaustion, credential hold, contribution health, and model availability remain
inside the manifest-bound payload and never become session state.

Completeness and revision are tracked independently for each
`(adapter_id, ResourceKind)` view. The report tier must be equal to or weaker
than that kind's manifest declaration. Typed `ResourceReport` ingress has two
modes:

- A reconnect **snapshot** with `authoritative` completeness is a complete
  external collection: every listed surviving identity must be an upsert with
  both valid manifest-bound envelopes, listed explicit tombstones/replacements
  may retire identities, and omitted active identities are terminally
  tombstoned. An `unknown` listed identity is invalid because authoritative
  completeness cannot install a surviving resource with no classifiable
  payload. An adapter whose external view may omit live members must not claim
  this tier.
- A `partial` snapshot updates listed identities and marks omitted identities
  stale only when both cached envelopes exist. A `none` snapshot carries no
  reconstructed mutations and applies the same cached-payload degradation.
  An `unknown` no-payload identity remains `unknown`; omission cannot invent a
  cache that does not exist.
- A live **delta** changes only explicitly named identities regardless of tier;
  omission has no meaning.

Every accepted report is normalized before append into one durable
`RESOURCE_STATE` event and advances each reported view revision even when its
payload bytes are unchanged. Core-assigned `(authority_domain_id, LSN)` order is
the only Patchbay revision authority. The event carries explicit prior record
revisions so replay rejects contradictory history. No resource projection
mutation occurs before durable append; restart and live catch-up fold the same
normalized event.

`ResourceRegistry` additionally carries one
`(authority_domain_id, applied_through_lsn)` cursor for the highest contiguous
prefix of the shared authority-domain log it has validated. Known sibling event
kinds advance this cursor without changing resources or views. Unknown or
`UNSPECIFIED` durable kinds, missing/zero LSNs, wrong/empty domains, gaps, and
malformed `RESOURCE_STATE` payloads fail before cursor advancement. Once this
structural validation succeeds, a record whose LSN is at or below the cursor is
a whole-event audit no-op before adapter-generation or prior-revision checks;
per-record/view revisions are not a second obsolete-event classifier. A new
record must be the next LSN. A lower source-adapter generation at that next LSN
is corrupt and leaves both cursor and projection unchanged.

Catch-up may re-feed an already validated prefix into an existing projection
and is idempotent. Full recovery is stricter: storage rows must start at LSN 1
and remain exactly gap-free, so duplicate, decreasing, or gapped rows are
corruption rather than benign redelivery. Report ingress catches the resource
projection up through the durable tail before deriving generation and
`from_revision_lsn` mutations; the composition-root decision gate serializes
competing resource decisions, but correctness does not assume every sibling
durable-log writer shares that gate. After the report append returns its LSN,
ingress reads the exact stored suffix from the prior applied cursor through that
LSN, folds every interleaved known sibling event in order, requires the suffix
to end with the exact committed report, and installs the complete projection
atomically. A valid authoritative committed report returns success after that
install. A missing, reordered, corrupt, or substituted suffix fails closed and
rebuilds from the authoritative log rather than continuing with a false cursor.

An authenticated adapter id and current adapter generation fence report
source. A newer adapter attachment generation fences prior cached records before its
replacement token becomes usable: records with both cached envelopes move from
`current` to `stale`, no-payload `unknown` records remain `unknown`, and prior
resource views move to `none` completeness until the new generation reports.
Abnormal stream loss applies the same record degradation to resources owned by
that adapter. An old attachment token or stream epoch is inert.

A tombstone is terminal for one exact resource identity. A permanent
replacement uses a distinct same-adapter `ResourceIdentity`; the old record
retains `replaced_by`, and the matching replacement upsert commits in the same
durable event. Tombstoning preserves cache honesty: a record with both cached
envelopes becomes `stale`, while a no-payload record remains `unknown`. Late
evidence cannot resurrect the retired tuple. Cross-adapter replacement and
reusing a tombstoned identity are invalid.

`LoadSnapshot` requires and echoes `SnapshotViewKind`. `session` returns only a
validated `SessionSnapshot`; `resource` returns only a `ResourceSnapshot`
materialized from the current resource projection. Resource reads never decode
the current typed/versioned session checkpoint slot. A legacy undiscriminated,
corrupt, cross-type, unsupported-version, or older session checkpoint is repaired by the current materialized session view;
a historical bound that the current implementation cannot reconstruct likewise
returns the newer current authority rather than an empty or older view.

These current resource semantics are implementation-checked by generated-contract,
Rust projection/replay/property, authenticated server-ingress, reconnect, and
real-process snapshot tests. Promoted resource conformance vectors and formal
assurance remain owned by the resource-plane conformance feature.

### Atomicity between events and snapshots

v0.1.0 requires the following atomicity guarantees at the persistence boundary:

- A command is durably recorded (`accepted`) before delivery is attempted. Delivery never relies on in-memory state.
- A terminal transition is committed to the log before it is reflected in snapshots or returned to control surfaces.
- A snapshot materialization reads a consistent log prefix: it reflects every event with `LSN <= snapshot_LSN` and no event with `LSN > snapshot_LSN`.
- Snapshot writes do not reorder the log. A snapshot is a derived artifact keyed by `LSN`; it never becomes a second source of ordering.

If the persistence backend cannot provide these atomicity guarantees, the core must treat the write as failed (`SubmissionOutcome = failed` for submissions, or `failed`/continued `accepted` per policy for delivery) rather than expose an inconsistent view.

## Presence and Subscription

Presence/Subscription is a named protocol section/registry, **not** a fifth primitive. Operations and Observations carry presence facts; the registry defines how they are interpreted and reconciled.

Subscription is the deliberate exception to lifecycle-bearing Operations. A subscription request is grant-checked at establish time at the transport layer, audited as a security-relevant decision, and reconciled by cursor on reconnect, but it is not durably recorded as an Operation and does not enter `OperationState`. This creates two authority mechanisms: grant-checked-with-lifecycle for Operations/Elicitations, and grant-checked-without-lifecycle for long-lived Subscriptions whose semantics do not fit a finite terminal Operation state. Elicitation delivery uses this subscription substrate: the core does not direct-address a specific endpoint per Elicitation; it fan-outs Elicitation events to all active, authorized subscriptions for the expected responder actor's Elicitation stream. On reconnect, the control surface re-subscribes and submits its cursor; the core replays authorized events with `LSN > cursor` and/or returns a fresh snapshot.

The section distinguishes these axes:

| Axis | Meaning | v0.1.0 registry/fields |
|---|---|---|
| Endpoint availability | Is a concrete endpoint connection/address reachable? | Reuse/align with `SessionConnectivityState`: `live`, `stale`, `offline`, `unknown`, `failed`; fields: endpoint id, device id, adapter generation, last authoritative LSN. |
| Actor presence | Is an actor currently represented by at least one usable endpoint, and with what attention posture? | `available`, `away`, `unavailable`, `unknown`; derived from endpoint observations and session connectivity state, never authority by itself. |
| Observation subscription | Which actor/endpoint/control surface is subscribed to which event/snapshot stream? | `subscribed`, `resuming`, `unsubscribed`, `failed`; fields: subscription id, authorized filter, cursor, last delivered LSN, audit id for establish/deny. |
| Attention-required state | Does a target require human/service attention? | `none`, `attention_requested`, `response_required`, `blocking`, `escalated`; source is Elicitation or adapter Observation. |
| Expected responder | Which actor should answer an Elicitation, and which endpoint actually did? | Field on Elicitation: `expected_responder_actor` (operator actor in v0.1.0). No `expected_responder_endpoint` is present in v0.1.0. Optional endpoint class/control-surface role, fallback/escalation policy, and responder generation are reserved seams. Response Operation audit records the actual responding endpoint. |
| Stale-presence reconciliation | What happens after disconnect/reconnect or missed presence events? | Presence Observations carry LSN/revision; reconnecting clients submit cursor; stale presence cannot be rendered as live; Elicitations may terminalize `stale` if opener/target generation is superseded. |

Implementation notes:

- Attach Operations establish or refresh endpoint availability and trigger snapshot/cursor reconciliation; Subscriptions are separate grant-checked transport establishments without Operation lifecycle.
- Elicitation streams are subscription streams: all authorized subscribed surfaces for the expected operator actor receive the Elicitation, and the first valid answer clears it everywhere.
- Observation streams are optimizations; snapshots repair missed events.
- Presence is a derived fact, not a query target. One-shot "is session X present?" reads route through snapshot/status `query` Operations under the uniform read lifecycle; there is no distinct `query-presence` OperationKind.
- Single-operator v0.1.0 has no separate presence-leak threat inside the operator's authority domain. Filter-scoped subscriptions for multi-operator presence-leak prevention are a reserved seam; v0.1.0 must not bake in a hard-to-retract rule that all presence is globally public.
- Push notifications are an attention-routing surface, not authority.

## Persistence and recovery

The coordination core owns durable command state, the event log, snapshots, and audit records through a storage port. v0.1.0 persistence assumptions:

- **Single-writer**: one authoritative core process writes to the log. There is no multi-writer, HA, or split-brain recovery in v0.1.0.
- **Local-first**: the default backend is embedded and local to the core process. Domain semantics must not depend on a specific storage engine.
- **Port-isolated**: the core reads/writes through a storage port; adapters and control surfaces never touch persistence directly.
- **Crash recovery**: on restart, the core validates the complete durable log prefix to the last committed `LSN`. The session projection may seed from the latest compatible checkpoint and fold only its post-anchor tail; any checkpoint/tail semantic disagreement discards the derived checkpoint and retries strict session replay from LSN 0. Authority, commands/inboxes, Elicitations, resources, diagnostics, security/operator, adapter, and operator-session projections still rebuild their owned state from the full log, although diagnostics seeds its embedded session view from the accepted session checkpoint. Accepted-but-not-yet-terminal commands are restored as `accepted` (or a later committed state) and continue through their lifecycle. No accepted command disappears silently after a crash.
- **Idempotent reprocessing**: replaying the log produces identical state. Re-delivery to adapters after recovery is governed by adapter capability and command policy, not by log replay.
- **Storage continuity epoch**: each authority-domain storage lineage has one durably persisted nonzero `core_generation`, stable across ordinary process restarts and carried by session/resource snapshots. v0.1.0 has no rollover API. A future destructive restore, divergent fork, authoritative-store replacement, HA promotion, or zero-downtime process-fencing design must explicitly define epoch rollover and, where needed, a separate process-incarnation fence before serving snapshots or cursors. An ordinary backup/restore that continues the same history may retain the epoch.
- **Snapshot materialization**: the store is snapshot-capable and `LoadSnapshot` materializes an on-demand snapshot at the current `LSN`. A best-effort production writer checks once per second and targets a complete session checkpoint after each 256 newly observed authority-domain events. A successful write atomically retains one latest non-regressing row per authority domain; failure leaves the prior row and authoritative log unchanged, emits bounded structured stderr evidence, and retries with capped 1-to-30-second backoff. The private format-2 payload restores the complete authority-domain-bound `SessionRegistry` (live records, source cursors, retained generation tombstones, lockdown clamp, and revisions) only after typed/versioned and exact domain/core-generation/LSN plus semantic validation; incompatibility or post-anchor tail disagreement falls back to full session replay. A seeded registry rejects direct covered-prefix re-feed because compacted prior event bytes are not retained, while tail events retain exact-envelope redelivery checks. Under healthy scheduling the session fold normally replays only the events after the latest checkpoint, but this is not a hard 256-event bound under write failure, starvation, overload, or pre-poll appends, and it is not a whole-core recovery bound because every named sibling projection still full-replays.

No derived checkpoint, replica, or projection may become an independent ordering or authority source: every recovery seed is validated against its durable log domain/epoch/LSN anchor, and the log remains authoritative after checkpoint failure. v0.1.0's single embedded backend is a version-scoped physical topology choice, not a prohibition on future derived checkpoints, replicas, projections, or storage backends.

v0.1.0 does not require WAL replication, remote replication, point-in-time cloning, or storage-engine hot swap. Those are reserved seams.

## Authority grants

A grant authorizes a subject (an actor, optionally narrowed to an endpoint or endpoint class) to perform a set of OperationKinds against a target scope. Grants are explicit, revocable, and evaluated inside one authority domain.

A v0.1.0 grant records:

- grant id;
- authority domain id;
- subject actor id;
- optional subject endpoint id or endpoint class;
- target scope, such as actor, adapter, runtime session, project/session group, fleet/supervisor scope, or other modeled resource;
- allowed OperationKinds;
- creation time and provenance;
- optional expiration;
- revocation generation or revoked time;
- revocation policy for already accepted commands.

Grant matching and grant selection are distinct. For one decision, the core first filters by exact authority domain and verified subject actor, optional endpoint narrowing, canonical `OperationKind` membership, and target-scope containment, then classifies the matches using one sampled decision time. Classification is mutually exclusive and revocation is tested first: a revoked grant whose expiration has also passed belongs to the revoked class, not the expired class. The core then considers the resulting liveness classes in the order live, expired, then revoked. Within each class, candidates are ordered by ascending lexicographic comparison of the exact UTF-8 bytes in `GrantId.value`; case folding, locale collation, numeric interpretation, and Unicode normalization are not applied. The first grant in the highest-priority available class supplies the decision's grant provenance. A live selection is retained as `AcceptedOperation.authorizing_grant_id` and therefore as `spawning_grant_id` when an accepted spawn produces a descendant grant; when no live grant exists, the selected expired or revoked id supplies denial and audit correlation. An identifier in a lower-priority class never outranks a live grant. Grant ids are opaque identity, not privilege or scope-specificity ranks; overlapping grants are valid and do not cause ambiguity denial. Projection, storage, and container iteration order is never authority.

Delegation is a reserved future direction, not a v0.1.0 field. A `parent grant id / delegated-by` field is intentionally absent from v0.1.0; it must be designed together with delegation semantics and multi-operator / federated-authority work, both of which are outside v0.1.0 scope. Device is part of the identity model (for audit and revocation grouping) but is not a grant-matching field; grant matching uses the issuer actor and optional endpoint. Adapter capability sets are not grant authority (see Adapter capabilities).

### Spawn authority

Spawn is adapter-scoped in v0.1.0: the Operation names one canonical attached-adapter scope before a target session exists, and a live matching spawn grant authorizes that selected scope. Existing runtime-session and operational-resource targets reject as incompatible before durable acceptance. Fleet-supervisor/authority-domain default selection and per-spawn-variant authority (e.g. "may spawn worktrees but not cloud environments") are reserved; v0.1.0 does not broadcast spawn to discover a target.

Successful spawn completion records an explicit, auditable **descendant grant** for the spawned session. This is an explicit grant record generated as part of spawn, not an implicit grant-matching rule. The descendant grant is the normal grant instance defined in `#### Spawn payload and authority commitments`: same authority domain as the spawning grant, spawner/operator subject, spawned session/generation target scope, explicitly enumerated existing-session `allowed OperationKinds`, spawn-operation/spawning-grant provenance, standard expiration and revocation metadata, and a spawn-completion audit id. It preserves the seam for future delegation without adding a delegation lineage field to v0.1.0; the reserved future direction is to inherit descendant allowed kinds from the spawning grant for delegation-aware authority.

A successful spawn `Result` is durable completion evidence, not by itself the public terminal transition or permission to redeliver the spawn. One exact-correlation rule qualifies that evidence everywhere: one or more identical non-empty `CommandId` references collapse to the same logical correlation, while empty or conflicting ids reject at observation ingress and cannot enter only the redelivery-suppression side of the decision. Delivery reconstruction suppresses redelivery once qualifying success is durable, including across adapter reconnect/core restart, because repeating a non-idempotent external spawn could duplicate the runtime. The core records a bounded, redacted `CommandRunning/spawn_completion_deferred` audit checkpoint for `inspect-command`; the canonical command remains `delivered` or `running`, with no terminal success claim, while registration/authority completion is pending. The fail-closed completion owner accepts the evidence only after the exact authorizing parent grant and verified accepted sender/target are durable and the command has reached `delivered` or `running` through valid gap-free LSN order; accepted-state or preseed facts are inert. The correlated `SessionRegistered` or `SessionGenerationBumped` target must be contained by the accepted adapter scope. While holding the shared decision gate, the owner records the verified `CommandCompleted/spawn_completion` audit, the audit-linked descendant grant, and finally `CommandState = completed`. Revocation `command_effects` participate in the same lifecycle fold, so an earlier cancellation/reauthorization terminal suppresses issuance. Startup replays and repairs any committed prefix through the same fold before service projections or listeners open; a historical completed transition must itself follow a valid delivered/running lifecycle before it may repair a missing audit/grant, and never receives a duplicate terminal transition. Broader scope containment retained by the repair fold is a forward/legacy seam, not v0.1.0 fleet selection. The staged completion audit is durable provenance, not premature public lifecycle output; production stderr reports spawn completion only after the grant and terminal transition are durable. Thus readers using the core decision gate observe either the pre-completion prefix or the final authority-bearing completion, not an audit/grant-only public completion.

Revocation uses two independent levers: revoking the spawn grant prevents future spawns, but already-spawned sessions keep operating under their auto-issued descendant grant until that grant is separately revoked. No cascade-revoke is v0.1.0 behavior; future cascade is a query over grant provenance and needs no schema change.

Grant checks happen before command acceptance. A submission without a live matching grant is rejected before delivery with `SubmissionOutcome = rejected` and `authorization_denied` or a narrower applicable failure term.

Authorization is deny-by-default. Control surfaces may hide unavailable actions, but UI availability is never authority. Sender identity is derived from the verified connection/session context, not from self-asserted payload fields, display names, project labels, cwd metadata, or adapter-reported friendly names.

Revocation prevents future authority. Already accepted commands follow the policy attached to their grant and OperationKind: continue, cancel where supported, or require reauthorization. Revocation does not delete command history; late events after revocation are audit/reconciliation events unless they are valid transitions for commands already accepted under the relevant policy.

v0.1.0 revocation actions include current-session revocation, all-session revocation, principal/endpoint/device revocation, adapter/session grant revocation, and security lockdown. Session/principal/endpoint/device revocation uses `continue` for already accepted work: it prevents future acceptance and subscription establishment without deleting command or audit history. A lockdown rejects new commands, marks affected runtime sessions stale, requires fresh login, and records the reason.

## Leases

A lease is a time-bounded exclusive claim over a resource or coordination role. A lease has:

- resource id;
- holder actor;
- scope;
- expiration;
- renewal rules;
- release rules.

Within one modeled Patchbay authority domain, the following exclusivity properties hold **once a fencing model (lease epochs or fencing tokens) exists and lease-backed behavior is promoted into v0.1.0 by a future feature**. They are a modeled precondition required before any such promotion, not a v0.1.0 guarantee, and are not checked in v0.1.0:

- Two actors cannot simultaneously hold the same exclusive live lease within one authority domain.
- Expired leases do not authorize new exclusive action.
- Lease renewal respects holder identity and scope.

v0.1.0 reserves leases as an extension seam. A future feature promoting leases into v0.1.0 must define the fencing mechanism, lessor authority, lease lifecycle registry, partition behavior, and adapter obligations before shipping lease-backed behavior. That feature must not overload `CommandState` or session state.

## Adapter capabilities

Adapters declare supported targets, Operations, and guarantees in one capability manifest:

- target categories from the registry below;
- supported `OperationKind`s (and, for `spawn`, supported `target_spec.shape` values);
- streaming support (boolean);
- runtime-session snapshot support (authoritative / partial / none);
- exact per-`ResourceKind` snapshot tier and payload/projection schema descriptors;
- cancellation support (boolean);
- session replacement support (boolean);
- idempotency strength (`none` / `at-Patchbay-boundary` / `end-to-end`);
- attachment method (adapter-specific descriptor);
- known failure modes (advisory list mapping to the failure vocabulary).

`AdapterTargetCategory` is the closed generated category registry. `ResourceKind` remains an open adapter-owned identifier beneath the operational-resource category.

| Target category | Disposition | Meaning |
|---|---|---|
| `runtime_session` | Committed | Runtime sessions with the canonical session identity and connectivity/activity contract. |
| `operational_resource` | Committed current direction | Operational resources with exact adapter-owned kinds and resource projection contracts. |
| `knowledge_bundle` | Reserved | Wire-present candidate for OKF v0.2 knowledge bundles. Registration rejects it until a separate report, authority, snapshot/reconciliation, presentation, and conformance contract is promoted. |

Fresh attach requires at least one explicit, unique committed category. Unknown, unspecified, duplicate, or reserved categories reject before a durable adapter-registration append; the rejected attempt may still produce its required audit record. `runtime_session` requires a specified `session_snapshot_support` tier; without that category the session tier remains unspecified. `operational_resource` requires one or more unique declarations, and resource declarations are forbidden without that category. Each declaration binds one non-empty `ResourceKind`, its own specified snapshot tier, and a `ResourceProjectionContract` targeting `operational_resource` with complete payload and domain-projection `SchemaDescriptor`s. At most 128 resource kinds may be declared; schema refs are bounded non-empty identifiers with known content types.

`Attach` is the only producer of durable adapter registration. The core's configured `PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS` map selects the expected attachment credential by the registration's claimed `adapter_id`; credentials are non-empty ASCII values and must be unique per adapter. A claim is not trusted merely because its id is present: the supplied attachment evidence must match the credential for that exact id before registration can be accepted. Generic authenticated Event ingress rejects a payload claiming the `patchbay.AdapterRegistration` schema before append. Adapter replay recognizes that schema only inside the complete canonical attachment envelope: the event, Observation, canonical adapter target, canonical adapter actor/endpoint sender, protobuf payload descriptor, and embedded adapter/domain/endpoint/generation/capability identity must agree. This envelope is durable routing identity, so ordinary restart replay keeps the adapter eligible for explicit adapter-scoped spawn resolution. It does not persist or reconstruct an attachment token or live delivery channel; spawn delivery continues to wait or fail under the existing adapter delivery behavior until a current attachment can receive it.

Durable pre-category registrations are the narrow compatibility exception: replay may normalize a category-less, resource-empty legacy manifest to session-only. Fresh attach never applies that normalization, and the legacy projection cannot admit a resource or knowledge bundle. Replay otherwise uses the same fail-closed registration-envelope and capability validator as attach.

A schema descriptor is an exact `(schema_ref, content_type)` format binding, not proof that opaque bytes semantically satisfy the named schema. Resource ingress must first select the exact authenticated `(adapter_id, resource_kind)` declaration and match both payload and projection descriptors. A local typed decoder remains responsible for rejecting malformed bytes before installing resource state or a domain projection. Adapter-supplied renderer code, HTML, CSS, and dynamic plugins are not loaded: surfaces nest locally decoded adapter domain data beneath canonical Patchbay identity, revision, staleness, authority, attention, and Operation presentation.

Each capability is shaped by where the core's behavior branches. Snapshot support is tiered because the core's reconciliation contract on reconnect depends on the tier, and resource tiers are never inferred from the session tier. Idempotency strength is an enum because retry presentation depends on it. Streaming, cancellation, and session replacement are boolean: the core does the same thing regardless of the value beyond display.

Control surfaces render unsupported actions as unavailable rather than attempting best-effort hidden behavior.

Adapter capability declarations are advisory for control-surface UX only: they let a control surface render an action unavailable before submission. They are not an authority gate and not a delivery gate. The core does not gate delivery on a cached adapter capability; it delivers the OperationKind to the adapter, and the adapter accepts or rejects based on its own support at delivery time. An adapter's `unsupported_command` is a delivery-layer, adapter-reported rejection. An unknown-to-Patchbay OperationKind is `validation_failed` at submission, before a grant is evaluated. Grant authority is expressed only in canonical Patchbay OperationKinds, which are stable and registry-owned; an adapter capability change never widens or narrows a grant.

### Adapter registration and lifecycle

An adapter is a **principal** with an explicit registration lifecycle. At attach time it submits (a) attachment evidence verified against the configured credential for its claimed adapter id (the Pi and token-commune adapters use configured local material; future adapters may use mTLS or OAuth — the mechanism is adapter-specific, not mandated by the core), and (b) its capability manifest. Only after that evidence verifies does the core accept and record the adapter id, endpoint, authority domain, capability manifest, attach LSN, and adapter generation (adapter-reported, monotonic per adapter, used to reject stale events from a prior adapter attachment). A successful attachment receives a current process-local token; later adapter RPCs must prove the same adapter id/evidence and current token, so replacing an attachment fences its prior token and delivery stream.

Attach, detach, failure, and capability redeclaration are audit events. Capability redeclaration is allowed with audit. Before a redeclared manifest and replacement token become usable, the core atomically records the registration together with any required resource degradation: removed resource kinds move their existing views to `none`; down-tiered kinds move to the incoming weaker tier; schema-incompatible kinds move to `none`; affected cached `current` records become `stale`, while no-payload `unknown` records remain `unknown`. A newer attachment generation applies the same record degradation to every prior resource view and moves those views to `none` until a report from that generation arrives. If this registration/degradation batch cannot commit, the replacement attachment is not published. Session capability loss follows the session rules below. Sessions and resources discovered or reported by the adapter inherit the adapter's authenticated channel.

A current adapter attachment maintains one long-lived authenticated delivery subscription. The subscription incrementally follows the durable log and remains pending while idle; finite tails that complete between polls are not the liveness mechanism. Abnormal stream loss is connection-liveness evidence: the core marks that adapter's sessions `stale` and terminalizes its `running` commands as `failed` with `execution_outcome_unknown`. Commands still at `accepted` or `delivered` remain eligible for the existing bounded redelivery behavior because execution is not known to have started. Attachment-token and stream-epoch fences make a replaced attachment's late disconnect inert. This transport signal does not detect every network black hole; heartbeat/last-report-age policy, including its freshness deadline and adapter capability implications, remains a reserved seam.

### Adapter session and resource snapshot capability tiers

Adapter snapshot support is not boolean. The registry recognizes three tiers, applied once to the runtime-session collection and independently to every declared resource kind:

- **Authoritative snapshot** — the adapter can return a complete, authoritative view of the session at a session generation the core can reconcile. The core treats this as a valid snapshot source and may use it to repair missed events.
- **Partial snapshot** — the adapter can return some state (e.g. command history or last-known status) but cannot fully reconstruct the session view. The core marks the unreconciled axes `unknown` or `stale` per `SessionConnectivityState`/`SessionActivityState` rather than synthesizing live state.
- **No snapshot** — the adapter cannot snapshot. The core holds the last-known cached view marked `stale` (or `unknown` if no cached view exists) and does not present it as live. Reconnect after missed events cannot be repaired by a snapshot; the control surface must reconcile against command/event records it can still query, and present unreconciled session state honestly.

For operational resources, a tier describes only the exact declared `ResourceKind`; it does not fall back to `session_snapshot_support` or another resource kind. An authoritative resource tier claims a complete collection view for that kind, partial claims an incomplete view whose unreconciled axes remain stale/unknown, and none claims no collection snapshot. Resource revision, replacement, and cached-state degradation are owned by the resource-state contract.

Degraded behavior rules:

- The core never fabricates a snapshot from optimistic UI or cached state when an adapter reports no or partial snapshot capability.
- A `partial` or `no snapshot` adapter does not weaken durable command state: accepted commands and their `CommandState` remain authoritative from the core's log.
- If an adapter loses the ability to snapshot it previously had, the core records the capability change as an audit record and moves affected sessions to `stale` or `unknown` until a fresh authoritative signal arrives.
- If an adapter claims an `authoritative` snapshot but returns a snapshot that is incomplete, malformed, non-monotonic, targeted at the wrong session generation, or otherwise non-conformant with its declared capability, the core rejects it as an authority source, records an audit record, and degrades the affected session axes to `stale` or `unknown`. An adapter that repeatedly fails its declared snapshot capability may have that capability reclassified by the core; reclassification is itself an audited capability change. The core never promotes a rejected snapshot to authoritative.

## Extension pressure classification

- **Committed post-v0.1 adapter direction:** the `AdapterTargetCategory` registry admits `runtime_session` and `operational_resource`; each operational resource kind has an exact snapshot tier and schema-bound payload/domain projection contract. Resource state uses typed snapshot/delta reports, current/stale/unknown reconciliation freshness, core-LSN record/view revisions, terminal exact-identity tombstones with atomic distinct replacement, durable `RESOURCE_STATE` replay, and discriminated session/resource snapshot loading. `knowledge_bundle` is wire-present and registration-rejected with OKF v0.2 named as the candidate format. Schema matching is a format binding, not semantic payload validation, and capabilities remain advisory rather than authority.
- **Committed v0.1.0 behavior:** `SubmissionOutcome`, `CommandState` (the checked lifecycle registry, reused by `OperationState` refinement equivalence), `LocalSubmissionState`, `SessionConnectivityState`, `SessionActivityState`, authenticated whole-report `SessionReportSourceCursor` ordering with one atomic durable report event and snapshot watermark, opaque adapter-reported mutable session `model` metadata (atomic full-report updates plus replayable legacy/core-authored `SessionModelChanged` deltas), the `OperationKind` registry (committed kinds: `spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management`), the `ElicitationState` lifecycle (stated-normative formal obligations for finality, first valid answer, typed correlation, invalid-response rejection, stale-target inertness, withdrawal, and timeout/grant behavior), the `response_contract` registry (committed contract kinds: `approval`, `question`), the five id spaces, the Presence/Subscription axes, failure vocabulary, idempotent retry at the Patchbay boundary, stale/unknown presentation honesty, one persisted nonzero storage-continuity epoch per authority domain with a private typed/versioned session-checkpoint envelope and exact domain/epoch/LSN snapshot compatibility, and one long-lived authenticated delivery subscription per current adapter attachment whose abnormal loss marks sessions `stale` and resolves `running` commands to `failed(execution_outcome_unknown)`.
- **Reserved extension seams:** multiple concurrent session-report producers, vector-clock/per-field merge semantics, explicit continuity-epoch rollover for destructive restore/divergent fork/store replacement, a distinct process-incarnation fence for HA/multi-core/zero-downtime work, typed composite/per-projection checkpoint namespaces, richer structured session-model descriptors and a distinct model-history projection, heartbeat/last-report-age adapter liveness policy (timer, freshness deadline, restart policy, and any adapter-declared liveness capability), fleet-supervisor/authority-domain spawn target selection (with broadcast explicitly excluded from the v0.1.0 path), future OperationKinds (including per-variant spawn kinds), reserved `response_contract.contract_kind` values (`freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`), reserved richer `ApprovalDecision` values (`ALLOW_ONCE`, `ALWAYS`, `POLICY_AMEND`, `MODIFIED_INPUT` — named in the enum, rejected with `validation_failed` in v0.1.0), surface-reject (an operator surface signals it cannot handle an Elicitation; distinct from operator approve/decline and from machine command rejection), reserved `agent-send` and `adapter-utility-exec` OperationKinds (rejected with `validation_failed` in v0.1.0), non-operator Operation senders (agent→agent, adapter→operator service Operations), no-lifecycle reads optimization, tighter Elicitation responder binding (endpoint/endpoint class/fallback chain), responder-actor distinction for multi-operator sessions, cross-actor delegation lineage, per-spawn-variant authority, presence-leak prevention for multi-operator, multi-answer/quorum Elicitations, richer activity details, multi-operator authority domains, lease lifecycle, native/mobile-specific local cache states, and additional control surfaces.
- **Rejected direction:** core arrival LSN, wall-clock timestamps, Pi promise tails, model-only revisioning, or independent per-field counters as session-report source authority; missing/zero cursor compatibility on fresh ingress; silent equal/lower-cursor mutation; finite clean-completing delivery tails as the v0.1.0 adapter-liveness mechanism; Pi-specific state names; UI-only optimistic states; transport-specific errors; adapter-specific lifecycle variants becoming core protocol states without registry updates; a generic operator-originated no-grant `Message` as a v0.1.0 action; and a `query-presence` OperationKind (presence is a derived fact, not a query target).

## Extension seams registry

This is the cross-cutting consolidation of reserved and rejected seams across the v0.1.0 foundation, indexed by extension area. Canonical entries remain in their per-registry homes (OperationKind enum, adapter capability manifest, state registries, failure vocabulary); this section is the cross-cutting index that tags each with its classification and where it was settled. It is the single view to answer "what did v0.1.0 leave open?" The standing discipline and pressure-test checklist live in `docs/SPEC.md` ("Non-foreclosure discipline") and `AGENTS.md` ("Extension pressure-test checklist").

Classification key: **C** = committed v0.1.0; **R** = reserved seam (v0.1.0 does not implement; named in registry, wire-present where forward-compat matters, submission rejects); **X** = explicitly rejected in v0.1.0 (promotion is a reversal, not a gap).

| Extension area | Decision | Class | Settled in |
|---|---|---|---|
| principals / authority domains | single operator + single authority domain | C | `feature-v0-walking-skeleton`; SECURITY §"v0.1.0 authority domain" |
| principals / authority domains | multi-operator / federated authority domains, shared authority administration, handoffs | R | SECURITY; GLOSSARY "authority domain"; `idea-multi-human-coordination` |
| adapters | Pi as the first workflow-migration adapter | C | SPEC "Adapter posture" |
| operational resources | typed identity `(adapter_id, resource_kind, resource_id)`, exact resource-grant containment, and target-kind-polymorphic ordinary resolution | C (post-v0.1 direction) | `epic-agent-operations-resource-plane-resource-identity`; PROTOCOL "Operational-resource identity and resolution" |
| operational resources | manifest-admitted exact resource kinds, per-kind snapshot tiers, and schema-bound adapter domain projections above the canonical presentation floor | C (post-v0.1 direction) | `epic-agent-operations-resource-plane-capability-manifest`; PROTOCOL "Adapter capabilities" |
| operational resources | per-adapter-kind completeness, revisioned resource records, typed report ingress, current/stale/unknown reconciliation, terminal replacement tombstones, durable replay, and discriminated resource snapshots | C (post-v0.1 direction) | `epic-agent-operations-resource-plane-resource-state`; PROTOCOL "Operational-resource state and reconciliation" |
| operational resources | resource-kind-wide grants and typed periodic resource checkpoint namespaces | R | resource-plane sibling/future checkpoint features; PROTOCOL "Operational-resource identity and resolution" |
| adapter target categories | `runtime_session` and `operational_resource` admitted; `knowledge_bundle` wire-present but registration-rejected with OKF v0.2 as candidate format | C (shape/committed values) / R (knowledge-bundle value) | `epic-agent-operations-resource-plane-capability-manifest`; PROTOCOL "Adapter capabilities" |
| adapter projections | locally decoded schema-bound domain projections nested beneath canonical authority/delivery/stale-state/attention presentation | C (post-v0.1 direction) | `epic-agent-operations-resource-plane-capability-manifest`; UX "Adapter-shaped resource projections" |
| adapter projections | dynamic loading of adapter-provided renderer/plugin code | X | `epic-agent-operations-resource-plane-capability-manifest` |
| adapters | other harnesses, shell jobs, CI jobs, project tools, notification systems, human approval surfaces | R | SPEC "Adapter posture" |
| spawn target resolution | one canonical attached-adapter target, selected explicitly by the submitting Operation | C | `authority-descendant-grant-completion`; PROTOCOL "Spawn payload and authority commitments" |
| spawn target resolution | fleet-supervisor/authority-domain default adapter selection | R | `authority-descendant-grant-completion`; PROTOCOL "Spawn authority" |
| spawn target resolution | broadcasting one non-idempotent spawn to discover a willing adapter | X | `authority-descendant-grant-completion`; ARCHITECTURE runtime/session plane |
| adapter capabilities | 3-tier snapshot model; capability manifest fields (`supported_operation_kinds`, snapshot tier, `streaming`, `cancellation`, `session_replacement`, `idempotency_strength`, `attachment_method`, `known_failure_modes`) | C | `feature-session-identity-adapter-contract`; PROTOCOL "Adapter capabilities" |
| adapter capabilities | adapter-declared typed diagnostic reports, bounded recent adapter-status evidence, and existing-view cockpit composition | C | `epic-observability-dogfooding-cockpit-diagnostics`; PROTOCOL Payload and failure vocabulary |
| session report ordering | authenticated `(adapter_generation, revision)` cursor scoped by runtime-session generation; whole-report atomic durable application and snapshot watermark | C | `adapter-report-source-ordering`; PROTOCOL "Session-report source order" |
| session report ordering | multiple concurrent producers, vector clocks, and per-field merge semantics | R | `adapter-report-source-ordering`; PROTOCOL "Session-report source order" |
| session report ordering | core arrival LSN, wall clock, Pi promise tails, model-only revisions, or independent per-field counters as source authority | X | `adapter-report-source-ordering`; PROTOCOL "Session-report source order" |
| session runtime metadata | opaque adapter-reported current `model` with registration/generation/snapshot carriage, atomic full-report updates, and replayable legacy/core-authored `SessionModelChanged { identity, from, to }` deltas | C | `feature-session-model-field`; `adapter-report-source-ordering`; PROTOCOL "Session state axes" |
| session runtime metadata | structured model descriptors, availability/capability metadata, and a separate model-history projection | R | `feature-session-model-field`; PROTOCOL "Session state axes" |
| adapter execution idempotency | end-to-end adapter-side execution idempotency (no double-execute on retry); declared via the `idempotency_strength` capability, not a formal property until a future adapter contract model is scoped | R | PROTOCOL `idempotency_strength`; VERIFICATION spawn-idempotency note |
| human control surfaces | responsive web cockpit + CLI | C | `feature-v0-walking-skeleton`; SPEC "Starting scope" |
| human control surfaces | native mobile / Expo app | R | SPEC; `idea-desktop-app-surface` (analog) |
| human control surfaces | native desktop app | R | `idea-desktop-app-surface`; SPEC "Starting scope" |
| human control surfaces | notification surface as a control surface | R | SPEC; GLOSSARY "control surface" |
| human control surfaces | operator-customizable skins/layouts above the conformance floor | R | `feature-ux-v0-acceptance`; `idea-operator-customizable-ux-skins` |
| human control surfaces | shared presentation-component layer (registry-derived static conformance check + skin-able CSS/showcase artifacts; implemented v0.1.0) | C | `feature-v0-presentation-component-layer`; `feature-ux-v0-acceptance`; ARCHITECTURE "presentation model" |
| transports / deployment topology | single authoritative core; adapters/surfaces may be separate processes | C | SPEC topology |
| adapter liveness | one long-lived authenticated delivery subscription per current attachment; abnormal loss marks sessions `stale` and resolves matching `running` commands as `failed(execution_outcome_unknown)` | C | `feature-adapter-staleness-liveness`; PROTOCOL "Adapter registration and lifecycle" |
| adapter liveness | heartbeat/last-report-age policy, freshness deadline, and adapter-declared liveness capability | R | `feature-adapter-staleness-liveness`; PROTOCOL "Adapter registration and lifecycle" |
| adapter liveness | finite clean-completing delivery tails as the liveness mechanism | X | `feature-adapter-staleness-liveness` |
| transports / deployment topology | HA, clustering, split-brain recovery, multiple authoritative cores | R | SPEC non-goals; ARCHITECTURE |
| storage / persistence backends | local durable event/snapshot store behind ports | C | `feature-persistence-snapshot-model` |
| storage / continuity identity | one core-assigned nonzero storage-continuity epoch per authority domain, persisted on first open, stable across ordinary restart, and required by exact domain/epoch/LSN snapshot compatibility | C | `snapshot-core-generation-semantics`; PROTOCOL "Revisions and cursors" |
| storage / checkpoint framing | private typed/versioned session-checkpoint envelope; legacy undiscriminated and cross-type bytes are disposable misses that replay from LSN 0 | C | `snapshot-core-generation-semantics`; PROTOCOL "Revisions and cursors" |
| storage / checkpoint scheduling | complete latest-only session checkpoint targeted every 256 observed events with one-second polling, observable retrying failure, compact covered-prefix rejection, and session-only tail recovery; composite/per-projection namespaces remain reserved | C (session writer) / R (other projections) | `recovery-checkpoint-writer`; PROTOCOL "Persistence and recovery" |
| storage / continuity identity | explicit epoch rollover for destructive restore/divergent fork/store replacement and a distinct process-incarnation fence for HA/multi-core/zero-downtime work | R | `snapshot-core-generation-semantics`; ARCHITECTURE persistence topology |
| storage / persistence backends | WAL shipping, remote replicas, point-in-time clone, storage-engine hot swap | R | ARCHITECTURE; PROTOCOL |
| protocol contract versions | Protobuf + Buf; generated-drift, conformance-vector, model-promotion, and presentation checks in CI | C | `feature-protocol-idl-and-conformance` |
| protocol contract versions | reserved enum values wire-present, rejected at submission (`agent-send`, `adapter-utility-exec`, `freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`) | C (shape) / R (value) | `feature-protocol-idl-and-conformance`; PROTOCOL registries |
| protocol contract versions | `(authority_domain_id, LSN)` tuple key shape (federation seam) | C | PROTOCOL event/cursor identity |
| protocol contract versions | JSON Schema / TypeBox / Zod for JSON-native local validation; TypeSpec for multi-output authoring | R | `feature-verification-contract-authority` |
| formal-model / checker backends | Quint primary; TLA+ semantic baseline; Alloy 6 relational; model-intent portable across backends (tool choice committed; switching reserved via portability) | C | `feature-research-formal-methods-tooling`; VERIFICATION |
| formal-model / checker backends | switching checker backend / authoring language | R | VERIFICATION |
| formal-model / checker backends | Elicitation, spawn-authority, subscription, and response-correlation model families retain stated-normative property ids and model vocabulary but have no promoted properties; future formulas require independent attempted evidence and mutation-survivable oracles | R | VERIFICATION model table |
| notification providers | notification provider as a future control surface / delivery channel | R | SPEC; GLOSSARY |
| third-party tool integrations | `agent-send` OperationKind (agent→agent mesh, op→op routing, adapter→operator service) | R | PROTOCOL OperationKind registry; `idea-agent-to-agent-mesh-seam` |
| third-party tool integrations | `adapter-utility-exec` OperationKind (standalone adapter utility exec) | R | PROTOCOL OperationKind registry |
| offline / queued operator intent | `queued_message_set` / `queued_message_clear` are transport/pairing, out of adapter Operation scope; migration switch-decision requires an explicit accept-or-replace. (Whether Patchbay later adds an offline-queued-intent OperationKind is an open design question, not a settled classification — it is not a wire-present reserved value in v0.1.0.) | X | `feature-pi-parity-checklist` §7-8 |
| encryption / key-management | passphrase primary authenticator for v0.1.0 | C | SECURITY |
| encryption / key-management | passkeys / MFA | R | SECURITY |
| encryption / key-management | adapter-proves-identity; mechanism deferred (not mTLS-mandated; requirement committed, mechanism is adapter's choice) | C | `feature-session-identity-adapter-contract` |
| federation / relay / multi-core | federated authority, relay, and multi-core coordination | X (v0.1.0); R (future) | SPEC non-goals; PROTOCOL delegation |
| federation / relay / multi-core | cross-domain coordination as a layer on per-domain keys (the v0.1.0 key shape includes the demarcator; forward-compatibility seam) | R | PROTOCOL `(authority_domain_id, LSN)` |
| multi-human coordination / approval | multi-human grants, audit, handoffs, approval workflows | X (v0.1.0); R (future) | SECURITY; `idea-multi-human-coordination` |
| multi-human coordination / approval | quorum / multi-answer Elicitations; tighter responder binding (endpoint/class/fallback) | R | PROTOCOL Elicitation; SECURITY |
| approval response contract | binary `ApprovalDecision` values `APPROVED`/`DENIED` (decision-driven terminal: `APPROVED`→`answered`, `DENIED`→`declined`) | C | `feature-v0-approval-response-contract`; PROTOCOL `approval` contract kind |
| approval response contract | richer `ApprovalDecision` values `ALLOW_ONCE`/`ALWAYS`/`POLICY_AMEND`/`MODIFIED_INPUT` (named in enum, not validatable in v0.1.0; reject with `validation_failed` until promotion) | R | `feature-v0-approval-response-contract`; PROTOCOL `approval` contract kind |
| Elicitation responder surface | surface-reject (an operator surface signals it cannot handle an Elicitation), distinct from operator approve/decline and from machine command rejection; v0.1.0 leaves an unrenderable Elicitation `pending` until timeout/withdrawal | R | `feature-v0-approval-response-contract`; PROTOCOL reserved Elicitation shapes |
| delegation | `parent_grant_id` / delegation lineage field | X (v0.1.0); R (future) | PROTOCOL; SECURITY; `feature-design-grant-shape` |
| delegation | per-spawn-variant authority ("may spawn worktrees but not cloud envs") | R | PROTOCOL spawn authority |
| leases | lease-backed exclusive coordination (deferred from v0.1.0; reserved as a future seam. Promotion requires a future feature to design the fencing model, lessor authority, lease lifecycle registry, partition behavior, and adapter obligations before shipping lease-backed behavior.) | X (v0.1.0); R (future) | `feature-lease-scope-decision`; PROTOCOL § Leases |
| observability / diagnostics | v0.1.0 observability = redacted process audit lines + CLI `session-health` + web current-`CommandState` presentation | C | SPEC v0.1.0 observability scope; UX CLI; SECURITY Audit events |
| observability / diagnostics | durable queryable audit log + core-diagnostics backing CLI `audit-query`/`inspect-command`/`adapter-status` (projection over the durable event log; no second writer) | C | `epic-observability-dogfooding`; SPEC post-v0.1.0 observability scope; UX CLI; SECURITY Audit events |
| observability / diagnostics | adapter diagnostics as payload: adapter-process durable diagnostics log + adapter-reported diagnostics recorded by the core and surfaced in the cockpit's existing views | C | `epic-observability-dogfooding`; `epic-observability-dogfooding-cockpit-diagnostics`; SPEC post-v0.1.0 observability scope; PROTOCOL Payload |
| observability / diagnostics | per-command delivery-trace timeline UI; metrics (counters/histograms/throughput); dedicated health/status dashboard; raw `event-inspect <lsn>`; SIEM export and long-retention compliance archives; quantitative performance budgets/SLAs | R | SPEC v0.1.0 observability scope; SPEC post-v0.1.0 observability scope; SPEC v0.1.0 performance posture; SECURITY reserved seams |
| observability / diagnostics | dedicated per-command trace storage (violates SSOT/single-writer); metrics pipeline as the primary v0.1.0 observability substrate (premature for single-operator v0.1.0) | X | SPEC v0.1.0 observability scope |
| observability / diagnostics | no-lifecycle bypass read of the audit log (CLI-local, not routed as a `query` Operation) | R | UX CLI; PROTOCOL Persistence and recovery (control surfaces never touch persistence directly) |

### How to read this registry

- A row tagged **C** is v0-committed behavior with its canonical home in the named registry; this table does not override that home.
- A row tagged **R** is a named seam v0.1.0 declines to implement. Where forward-compatibility matters it is wire-present and rejected at submission; otherwise it is named as reserved in docs. Promotion to committed is a registry/classification update.
- A row tagged **X** is a direction v0.1.0 explicitly rejects with rationale. Promotion is a reversal requiring a protocol-change ceremony, not a gap-fill.
- A row tagged **C (shape) / R (value)** means the v0.1.0 shape is committed (e.g. reserved enum values are wire-present) while the individual value is reserved (rejected at submission). The shape preserves the seam; the value is the future capability.
- A row tagged **X (v0.1.0); R (future)** is a direction rejected for v0.1.0 but explicitly preserved as a future seam (the distinction from pure X is that the design keeps the door open and names the seam).

This registry consolidates; it does not decide. If a future design surfaces a classification this table gets wrong, the fix is to update the canonical registry entry and this row together, not to edit this table in isolation.

## Security and trust boundary

Patchbay protocol assumes cryptographic primitives work as specified by their libraries and deployments. Formal models cover authority and identity relationships, not primitive cryptographic correctness.

Browser control uses server-side operator sessions with hardened cookies and CSRF protection for state-changing requests. Browser-local UI state is never authority for command submission, grant status, or session liveness.

Sender identity is derived from verified connection/authentication context, not from self-asserted display names or payload fields. External actor identities remain claims until verified by an adapter-specific trust root or deployment policy.

Security audit records are durable protocol-adjacent records for authentication, authorization, session management, command lifecycle, revocation, adapter attach/detach/failure, and stale-event rejection. Audit records are distinct from durable command/session state-transition events: they may record rejected attempts and failed checks that do not create command records. Audit records must not directly store raw session cookies, CSRF tokens, access tokens, passwords, bootstrap secrets, encryption keys, command prompt bodies by default, sensitive attachments, or adapter attachment material (the `attachment_method.descriptor` bytes an adapter presents at enrollment — mTLS material, configured local material, OAuth tokens, future trust-root proofs). The complete redaction list and the diagnostic-projection rules are canonical in [`docs/SECURITY.md`](SECURITY.md) Audit events; this summary mirrors them and defers to that section for additions.
