# Patchbay Protocol

Patchbay protocol semantics are defined around durable operator intent, explicit authority, unambiguous target identity, and recoverable state.

This document defines concepts and required behavior, not a final wire encoding. It is the canonical source of truth for command state, session state, failure vocabulary, and transition semantics — the **product intent and vocabulary naming** authority (see `docs/VERIFICATION.md` Artifact authority order). Wire shape, field identity, and enum encoding are authority of the generated `.proto` contract once it exists; until then this document is the provisional wire reference. Future TypeScript/Rust enums, TLA+/Quint variables, conformance vectors, and UI presentation labels derive from these registries rather than redefining them.

## Actors and endpoints

An **actor** is any represented participant: operator, agent, adapter, daemon, service, or control surface.

A **device** is a physical or virtual host that can run one or more endpoints, such as a browser on a laptop, a CLI on a VM, or an adapter process near a runtime.

An **endpoint** is a concrete connection or addressable runtime instance for an actor on a device. An actor may have multiple endpoints across devices or deployments.

Actors, devices, and endpoints have stable identifiers assigned by Patchbay or verified through an adapter-specific trust root. Human-readable labels are metadata, not routing authority.

An **operator session** is an authenticated browser or CLI session for the human operator. Operator sessions are endpoint-bound server-side records, not bearer authority stored in UI state. V0 has one human operator, but commands still name and validate the issuing actor, device, endpoint, and operator session so future multi-operator authority domains can extend the model without changing command semantics.

## Sessions

A **session** is an adapter-reported runtime/control target. A session identity binds the fields needed to prevent wrong-target mutation:

- adapter id;
- deployment scope;
- runtime session id (adapter-reported, stable per session generation);
- session generation (adapter-reported, monotonic per session).

Project, cwd, and name are **metadata**, not identity: they describe the session for operator orientation and display, but they update independently of the identity tuple. A cwd change does not create a new session target, and human-readable labels cannot override verified target identity.

Session generation is adapter-reported because only the adapter can observe external runtime replacement. When the adapter reports a strictly-greater session generation for an existing session id, the core **tombstones** the prior generation: it marks the prior generation superseded at the next LSN, retains it for audit and late-event correlation, and treats the new generation as the live target. Late replies or events binding to a tombstoned generation are `stale_event` audit records; they do not mutate the live generation. This is consistent with the ratified first-durable-terminal-commit rule: late events do not rewrite committed state.

Generation rules:

- Supersession requires a strictly-greater generation. An equal report is a no-op (a capability redeclaration may proceed, but the generation is unchanged). A lower report is rejected as an audit record and the live generation is left unchanged.
- First registration (no live generation exists) is accepted; monotonicity has nothing to check against.
- The tombstone fact ("generation N existed, superseded at LSN X") is an audit record retained indefinitely. Per-generation detail (full command/event/reply state) is bounded and reclaimable by log compaction. After compaction, an operator querying an aged-out generation gets the tombstone plus any not-yet-compacted detail, with a note that older detail was compacted.
- Late replies or events must bind to the session generation they describe. A reply for an old generation cannot mutate a new generation.

## Operations, Observations, Elicitations, payloads, and correlation

Patchbay uses an actor-neutral vocabulary of three primitives — Operation, Observation, and Elicitation — with Payload as content carried inside any of them, not a standalone authority primitive. Every primitive carries `{sender, recipient}` actor fields. V0 Operations are operator-originated; the actor-neutral sender vocabulary is a reserved seam for non-operator senders (agent→agent, adapter→operator service Operations). V0 does not mediate non-operator-originated authority-bearing Operations.

### Operation

An **Operation** is an authorized control-plane request by an actor to an actor, core, adapter, fleet, session, service, or resource target. An Operation may be side-effecting, read-only, lifecycle-acting, response-submitting, or fleet-creating. Operations require verified sender identity, target identity/scope, authority evaluation, a registry-owned `OperationKind`, boundary validation, idempotency semantics where applicable, and durable lifecycle state after acceptance.

V0 Operations are operator-originated. The actor-neutral sender vocabulary is a reserved seam for non-operator senders (agent→agent, adapter→operator service Operations). V0 does not mediate non-operator-originated authority-bearing Operations. Initial implementation reuses `CommandState` and command ids by refinement equivalence (see `OperationState` ⇿ `CommandState` refinement below); command/message ids stay client-generated in the operator domain per the existing protocol. `Operation` is the actor-neutral vocabulary; `CommandState` is the checked lifecycle registry until the coordinated rename/model update occurs.

### Observation

An **Observation** is a source-authenticated fact, event, output, status emission, reply-like result, or lifecycle/status fact emitted by an actor, adapter, core, runtime, or service. Observations do not grant authority to act. They still require source identity, target/session/generation context where applicable, correlation context when they answer or relate to prior work, and LSN/cursor/snapshot reconciliation if durable.

Live streams are delivery optimizations. Durable core records and snapshots remain the authority for accepted Operations and reconciled state.

### Elicitation

An **Elicitation** is a durable pending response solicitation from one actor/system component to another. It opens a response slot rather than answering a prior request. It carries an adapter-assigned `ElicitationId`, opener, `expected_responder_actor` (the operator actor for committed v0 human-facing Elicitations), target/session/generation context, `response_contract`, timeout/cancellation/withdrawal policy, correlation to the work that caused it, and terminal lifecycle state. It does **not** carry an `expected_responder_endpoint` or bind to a specific operator-session endpoint in v0. The core assigns the durable LSN when it records the Elicitation, as for other durable events.

Elicitation delivery rides the subscription layer: the Elicitation is fan-out delivered to every surface with an active, grant-checked subscription to the operator actor's Elicitation stream. Any authenticated endpoint for that operator actor may answer. First-answer-wins terminalizes the Elicitation (`answered` or the applicable terminal) for all subscribed surfaces; later response attempts from other surfaces are rejected as already-terminal/stale candidates and recorded with the same `stale_event` audit treatment used for late terminal candidates. The endpoint that actually answers is captured in the response Operation's audit record at response time; it is not pre-bound in the Elicitation record.

Elicitation is actor-neutral as a future-proof vocabulary: agent→operator questions, harness→client service requests, service→operator secret prompts, and future agent→agent/op→op solicitations use the same primitive when promoted. In v0, the opener is always an adapter/agent/harness, never the core; agents/adapters open Elicitations such as `AskUserQuestion`, tool-input requests, and approval gates. A response is submitted as an operator-originated `OperationKind = elicitation-response` or `approval-response` Operation correlated to the Elicitation. Two seams are explicit: the responder-binding seam is preserved by v0's operator-actor binding while endpoint/class/fallback-chain binding remains reserved; the responder-identity audit seam is built by recording the responding endpoint in the response Operation audit, with future multi-operator work adding responder-actor distinction when multiple operators can share a session.

Core prompts are **not** Elicitations. Lockdown, expired/revoked sessions, CSRF rejection, and similar cases are core-imposed states enforced by Operation rejection or pre-protocol operator-session establishment. The protocol assumes a valid operator session exists; login, re-authentication, and lockdown exit are control-surface/web-server concerns outside the normative Operation/Elicitation flow.

### Payload

A **Payload** is the adapter-specific content or schema-bound body carried inside an Operation, Observation, or Elicitation. Examples: prompt text, slash-command text, typed user input entries, tool-call arguments, function results, image/file references, question options, structured schemas, or adapter diagnostics. Payload does not itself grant authority, create lifecycle state, or define protocol kinds.

### Generic Message

Generic operator-originated no-grant `Message` is not a v0 action. Operator-originated content that drives work is payload of an authorized `instruct` Operation. Agent/harness/service-originated requests for a response are durable Elicitations. The `message id` space remains reserved for future informational surfaces and for current correlation-model compatibility.

### Reply and response correlation

A reply references a prior message or command by typed correlation. A reply is valid only when its correlation reference resolves to a known prior command or message id in the same authority/session context. Duplicate replies are either idempotent or visibly rejected. Response Operations to Elicitations (`approval-response`, `elicitation-response`) use a typed correlation reference to a known `ElicitationId` in the same authority/session/responder context; this response→Elicitation typed-correlation case is a new stated-normative obligation (see `docs/VERIFICATION.md`) not yet covered by the checked `reply_correlation.qnt` model.

## Id spaces

Patchbay uses five separate id spaces, each with a defined assigner, to prevent forgery and enable idempotent retry:

1. **Command id** — client/operator-domain generated today; identity for accepted lifecycle-bearing records. During the vocabulary transition, accepted Operations reuse this id space by refinement equivalence. A future `OperationId` rename is a coordinated artifact rename, not a sixth id space.
2. **Message id** — reserved in v0 even though generic operator-originated no-grant `Message` drops. It remains in the registry because current `TypedCorrelation` and future non-command informational surfaces may need it.
3. **Reply id** — adapter-or-core assigned for correlated reply/observation records that answer prior command/message/operation context.
4. **Event id** — core-assigned LSN, keyed as `(authority_domain_id, LSN)`.
5. **Elicitation id** — new id space, adapter-assigned when a pending response slot is opened. The core assigns an LSN when it durably records the Elicitation; it does not assign the `ElicitationId` in v0.

A command id and an idempotency key are **separate fields**: the command id is identity, and the idempotency key is the dedup handle. A retry reuses both; an intentional duplicate action uses a new command id and a new idempotency key.

Forgery-prevention justification:

- A response Operation must not be able to masquerade as the Elicitation it answers. Separate `CommandId`/`ElicitationId` spaces preserve direction: Elicitation opens a pending slot; response Operation answers it.
- A reply id cannot masquerade as command identity; the checked `TypedCorrelation` principle already enforces separate id spaces for command/message/reply and same-context typed references.
- `ElicitationId` is not a typed `ReplyId` subkind because an Elicitation is an initiation, while a Reply is a response. Modeling initiation as response inverts semantic direction and confuses lifecycle ownership.
- The existing `reply_correlation.qnt` does **not** cover response Operation → Elicitation. Extending typed correlation is a new verification obligation (see `docs/VERIFICATION.md`).

## Canonical state registries

These registries are committed v0 protocol behavior unless marked as an extension seam. Implementations may add display labels, colors, or adapter-specific metadata, but they must not add protocol states outside the registry without updating this document, contracts, models, and conformance vectors together.

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
- A duplicate submission with the same command id or idempotency key returns the existing command record and state; it does not create a new state transition.
- `rejected` means a known actor refused the command by semantics or policy. `failed` means an accepted attempt encountered an error. `expired`, `cancelled`, and `superseded` are distinct terminal outcomes and must not be collapsed into `failed`.

### `OperationState` ⇿ `CommandState` refinement equivalence

`OperationState` is not a new checked model. It reuses `CommandState` by documented refinement: accepted Operations use the existing `CommandState` state names (`accepted`, `delivered`, `running`, `completed`, `rejected`, `failed`, `expired`, `cancelled`, `superseded`) and inherit only the properties actually checked in `command_lifecycle.qnt`: `CommandDurability`, `TerminalFinality`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `BoundaryDedup`, `RetryReusesIdAndKey`, and `RetryAfterTerminalReturnsExisting`. A future rename from `CommandState` to `OperationState` must update model names, property metadata, `.proto`, conformance vectors, and docs together; until then `CommandState` remains the checked lifecycle registry name and `Operation` is the actor-neutral protocol vocabulary that maps to it.

The full transition graph above is **stated-normative**, not checked by `command_lifecycle.qnt`: the current checked model permits any non-terminal state to commit any terminal candidate, so adjacency rules such as no `accepted → completed` require a strengthened lifecycle model or an OperationState-specific model before they can be claimed checked. Read/query Operations use the same stated-normative lifecycle in v0; they may skip `running`, but the no-direct-to-completed fast-path rule is also stated-normative, not checked. A no-lifecycle reads optimization is a reserved seam, promotable if polling volume warrants it later.

### `OperationKind` registry

One registry owns kinds, lifecycle policy, authority matching, adapter capability mapping, display labels, and generated contract variants. Adding or promoting a kind requires updating this document, `.proto`, model/vectors as applicable, and implementation together.

| `OperationKind` | Meaning | V0 disposition |
|---|---|---|
| `spawn` | Create a new runtime/session/thread/agent/process/cloud resource; target is fleet-level by default before a session exists. This is one OperationKind; spawn variants are described by payload `target_spec.shape`, not by per-variant OperationKinds. | Committed v0. Requires fleet-authority modeling. The `target_spec.shape` registry is reserved/open in v0 and adapter-enforced at delivery. |
| `attach` | Connect/reconnect a control surface endpoint to an existing session/server and reconcile. | Committed v0. |
| `instruct` | Send prompt/user input/steering content into a session/turn. | Committed v0 for operator-originated instruct. |
| `cancel` | Request cancellation of a target Operation/turn/session action. | Committed v0. |
| `interrupt` | Request immediate stop/interrupt of active execution. | Committed v0. |
| `query` | Read status, snapshot, capabilities, lists, history, metadata, or diagnostics. | Committed v0. |
| `approval-response` | Respond to a permission/tool approval Elicitation. | Committed v0. |
| `elicitation-response` | Respond to non-approval Elicitations. | Committed v0 for `question` contracts; reserved for `freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, and `service_request` contracts. |
| `reconfigure` | Change model, reasoning/thinking level, permission mode, tools/MCP, agent mode, workspace, or adapter config. | Committed v0. |
| `session-management` | Resume, fork, compact, clear, archive/delete, revert, share/unshare, remove messages, checkpoint restore, disconnect/retire existing sessions/resources. | Committed v0. |
| `agent-send` | Reserved design seam for agent→agent mesh, op→op routing, adapter→operator service Operations, and other non-operator Operation directions. Informed by remote-pi mesh `agent_send`/`agent_request` prior art (not one of the 7 surveyed harnesses) and by Antigravity trigger / Codex service-request pressure. | Reserved seam; v0 submissions reject with `validation_failed`. |
| `adapter-utility-exec` | Reserved seam for standalone adapter utility execution that does not create a thread/turn or persistent runtime session. Codex `command/exec` and `process/spawn` are the surveyed pressure `[codex-appserver-protocol]{5}` `[codex-appserver-types]{9}` `[codex-appserver-types]{10}`. | Reserved seam; named in registry, not validatable in v0; submissions reject with `validation_failed`. Full lifecycle/idempotency modeling deferred. |

Boundary rules:

- Unknown `OperationKind` is `SubmissionOutcome = rejected` with `validation_failed` before grant evaluation.
- Reserved-but-not-validatable kinds such as `agent-send` and `adapter-utility-exec` also reject with `validation_failed` in v0. Promotion is a registry update, not a schema change.
- Unsupported-by-adapter known committed kind is a delivery-layer `unsupported_command` rejection after acceptance, matching the existing capability posture (the core does not gate delivery on cached adapter capability).

#### Spawn payload and authority commitments

- **One `spawn` OperationKind.** Worktree, same-dir, session, process, thread, local sidecar, and cloud-environment spawns are not separate OperationKinds in v0. Per-variant OperationKinds are reserved only if a future registry update promotes them.
- **`target_spec.shape` = reserved open shape registry.** The spawn Operation payload includes `target_spec.shape`. V0 names shapes for vocabulary, audit, and display (for example, "spawned a worktree") but does not validate shape variants at the protocol layer. The adapter capability manifest declares which shapes the adapter supports; the adapter accepts or rejects the accepted Operation at delivery time with `unsupported_command`, consistent with the capability-not-authority discipline.
- **Target scope = fleet-level.** A v0 spawn grant authorizes all spawn variants across any adapter/supervisor the operator can reach. Adapter-level spawn grants remain expressible through the existing target-scope flexibility when narrower authority is desired; no schema change is needed.
- **Per-variant authority is reserved.** V0 does not implement "may spawn worktrees but not cloud environments" authority. If needed later, per-variant authority can be expressed through grant `target scope` or by promoting spawn variants to distinct OperationKinds; both are reserved seams, not v0 behavior.
- **Descendant authority = spawned-session manifest.** Spawn completion includes an auto-issued grant record for the spawned session: spawner/operator subject as subject, spawned session as target. This is an explicit, operator-visible, auditable grant record generated as part of spawn, not an implicit grant-matching rule. It preserves and builds the seam for future cross-operator delegation over spawned sessions.
- **Delegation remains out of v0.** `feature-design-grant-shape` intentionally removed `parent_grant_id` / delegated-by from the v0 grant shape. The auto-issued descendant grant is same actor (operator), new target (spawned session), not cross-actor delegation. A future delegated grant can reintroduce `parent_grant_id` and reference the auto-issued descendant grant directly; that is same infrastructure, not v0 cross-actor delegation.
- **Revocation uses two independent levers.** Revoking the spawn grant prevents future spawns. Already-spawned sessions keep operating under their auto-issued descendant grant until that grant is separately revoked. No cascade-revoke is v0 behavior; future cascade is a query over grant provenance and needs no schema change.
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

Derived UI labels such as “Live idle”, “Working”, “Stale working”, or “Offline” are presentation labels over these axes, not protocol states. A stale or unknown connectivity value dominates presentation: stale working is not live working.

### `ElicitationState` lifecycle

`ElicitationState` is a new registry, not a projection of `CommandState`. It is **stated-normative** until promoted (see `docs/VERIFICATION.md`); Elicitation ids are adapter-assigned in v0 and the core does not open Elicitations in v0.

| State | Terminal? | Meaning |
|---|---:|---|
| `opened` | no | Core durably recorded the Elicitation, but it may not yet be visible through subscription fan-out to the expected responder actor's subscribed surfaces. |
| `pending` | no | The Elicitation is visible on one or more subscribed surfaces for the expected responder actor and can accept a valid response Operation from any authenticated endpoint for that actor. |
| `answered` | yes | A valid response Operation satisfied the contract and first durable terminal commit selected it as the answer, terminalizing the slot for all surfaces. |
| `declined` | yes | The expected responder explicitly refused/rejected/denied the Elicitation without satisfying it. Covers question rejection and approval denial when the response contract treats denial as terminal. |
| `expired` | yes | The response window closed before another terminal state won. |
| `cancelled` | yes | Core/operator/policy cancelled the pending slot from the responder/control-plane side. |
| `withdrawn` | yes | The opener withdrew the solicitation before it was answered, e.g. the tool call was no longer needed. |
| `superseded` | yes | A newer Elicitation or policy explicitly replaced this one. |
| `stale` | yes | The target/session/generation/opener context became stale or orphaned; responses must no longer mutate live state. |

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
- A response Operation must reference the `ElicitationId` with a typed correlation, must satisfy the active `response_contract`, and must be issued by an authenticated endpoint for the `expected_responder_actor` in v0. The responding endpoint is captured in the response Operation audit for debugging.
- Invalid response behavior: default is **reject the response Operation** (`SubmissionOutcome = rejected` before acceptance, or `OperationState = rejected` after acceptance by policy) and leave the Elicitation `pending`. A contract may explicitly specify terminal-on-invalid policy, but that policy must name the terminal outcome (`declined`, `superseded`, or `cancelled`) and be tested.
- No-answer is not an Operation. It is either continued `pending` or a terminal policy event such as `expired`, `cancelled`, `withdrawn`, or `stale`.
- `answered` does not imply the underlying tool/action succeeded; it only means the response slot was satisfied. Subsequent work emits Operations/Observations as usual.

Reserved future Elicitation shapes: multi-responder quorum Elicitations; multi-answer accumulation; tighter responder binding to a specific endpoint, endpoint class, or fallback chain; delegated responder policy; escalation from one expected responder actor to another; cryptographic secret-entry envelopes; large file/attachment upload protocol; drawing/region-selection UI hints.

### `response_contract` registry

A `response_contract` describes what kind of response is semantically required; optional UI hints describe how a surface may render it. The `elicitation-response` OperationKind is committed v0. The `response_contract.contract_kind` values have a committed/reserved split: committed v0 contract kinds are `approval` and `question`; reserved contract kinds are named in the registry but not validatable in v0. `freeform` is reserved because the solid surveyed grounding is currently Claude's optional `AskUserQuestion` freeform answer, while other surveyed response surfaces are structured question/answer, approval, secret, function-result, or service-request shapes rather than standalone unstructured Elicitation responses. A response submitted for an unknown or reserved/unsupported `contract_kind` is rejected at submission with `validation_failed` unless a later registry update promotes that contract kind.

Required fields:

- `contract_kind` — registry variant below;
- `schema_ref` or inline schema where structured validation is required;
- `ui_hints` — optional list such as `select-one`, `select-many`, `free-text`, `secret-input`, `upload`, `draw`, `confirm`, `diff-review`;
- `timeout_policy`;
- `invalid_response_policy`;
- `responder_policy` — v0 `expected_responder_actor` (operator actor for committed human-facing Elicitations); endpoint class, service role, fallback chain, and tighter binding are reserved. The responding endpoint is recorded in the response Operation audit, not pre-bound in the Elicitation;
- `sensitivity` — whether raw response may be logged, redacted, encrypted, or never persisted in plaintext.

| `contract_kind` | Semantics | V0 disposition |
|---|---|---|
| `approval` | Allow/deny/allow-once/always/policy-amend/modified-input permission response. | Committed v0. |
| `question` | Answer one or more questions, possibly with options and freeform text. | Committed v0. |
| `freeform` | Unstructured text response. | Reserved seam. Named in registry, not validatable in v0; demoted until more than Claude's optional freeform answer is grounded as a genuine Elicitation response surface. |
| `secret` | Provide sensitive secret/token/input with redaction/no-log policy. | Reserved seam. Named in registry, not validatable in v0. |
| `function_result` | Return custom tool/function result to a waiting service/harness. | Reserved seam. Named in registry, not validatable in v0. |
| `file_attachment` | Provide file/blob/image/attachment reference or upload. | Reserved seam. Named in registry, not validatable in v0. |
| `structured_schema` | Response must validate against declared JSON/protobuf/schema. | Reserved seam. Named in registry, not validatable in v0. |
| `service_request` | Non-human service response such as current time, attestation generation, auth refresh, or adapter-provided evidence. | Reserved seam. Named in registry, not validatable in v0. |

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
| `expired` | policy/time | The command validity window closed. | `expired` |
| `cancelled` | policy/operator | Cancellation became the authoritative result. | `cancelled` |
| `superseded` | policy/operator | A newer command or policy replaced this command. | `superseded` |
| `stale_event` | reconciliation | A late event refers to an old command/session generation or terminal command. | audit record only; no state mutation |

Extension seam: future adapters may attach adapter-specific diagnostic codes, but those codes map onto this vocabulary at the Patchbay boundary.

## Acceptance semantics

Patchbay distinguishes acceptance from delivery and completion.

A command accepted by Patchbay is durably recorded before delivery. After acceptance, it remains visible as a `CommandState` until and after it reaches a terminal state. An accepted command cannot disappear silently.

Acceptance creates a command record only after boundary validation, authority checking, idempotency reconciliation, and target identity binding. Invalid submissions that fail before acceptance return `SubmissionOutcome = rejected` without creating durable command state. Audit policy may record rejected attempts, but those audit records are not command records and do not use `CommandState`.

## Idempotency and retry

Commands are idempotent by default at the Patchbay boundary. Retrying the same command id or idempotency key does not apply the command twice.

A duplicate command returns the existing command state unless the operator explicitly creates a new command with a new command id and idempotency key.

Adapters that cannot guarantee idempotent external execution must report that limitation as a capability constraint. Patchbay still deduplicates at the coordination boundary and exposes the adapter limitation to control surfaces.

## Cancellation, expiration, supersession, and race semantics

Cancellation, expiration, supersession, adapter completion, adapter failure, and adapter rejection are terminal candidates competing for the same command's first durable terminal transition.

- Running is non-terminal. A running command remains observable until one terminal state wins.
- First durable terminal commit wins. The core assigns a total order to accepted state-transition events in the durable event log; the earliest committed valid terminal transition becomes authoritative.
- If two terminal candidates are truly concurrent before persistence, models may treat the winner as nondeterministic, but implementations must persist one total order and expose the chosen terminal state consistently in snapshots and conformance traces.
- Later conflicting terminal candidates are audit/reconciliation events, not `CommandState` rewrites and not a distinct durable conflict state.
- Cancellation is a command or policy request that races with execution. If `completed`, `failed`, `expired`, `cancelled`, `superseded`, or another terminal state is committed first, later cancellation cannot mutate that command. Depending on the API shape, the too-late cancellation attempt may be refused before acceptance, recorded as an ineffective cancellation failure, or preserved as an audit event; it never rewrites the original command's terminal state.
- Expiration is evaluated against the command validity window. If expiration wins before a later terminal outcome is committed, the command becomes `expired`; if a terminal outcome wins first, expiration does not rewrite history.
- Supersession requires an explicit replacement relationship to a newer accepted command or policy decision. Supersession is not a synonym for cancellation or failure, and a late supersession candidate does not rewrite an already-terminal command.
- Global priority ordering such as `cancelled > completed` is not part of v0's generic command lifecycle. Future safety-critical OperationKinds may define explicit abort or fencing policy, but that policy must be designed as OperationKind behavior rather than a hidden override of terminal-state finality.

## Snapshots and streams

Event streams are useful but not authoritative by themselves.

A snapshot is an authoritative state view for an actor, session, command, lease, or resource. Control surfaces reconcile against snapshots after reconnect, resume, tab restore, app restart, or suspected drift.

Snapshots expose the canonical state axes above. Stale cached state must not render as live state.

### Revisions and cursors

The coordination core owns a single totally-ordered durable event log per authority domain. Every accepted state-transition event is assigned a monotonic, gap-free **log sequence number** (`LSN`) at durable-commit time. The `LSN` is the canonical ordering for first-terminal-commit-wins and for snapshot reconciliation.

Event, cursor, and revision identity is the **`(authority_domain_id, LSN)` tuple**, not a bare LSN. V0 has one authority domain, so in practice every key carries the same domain id — but the *shape* of the durable key includes the domain demarcator. This is forward-compatibility hygiene: when federation arrives, cross-domain coordination becomes a layer on top of the per-domain keys, not a data migration that retroactively attaches domains to historical events. Hybrid logical clocks (HLC) / logical-clock abstraction was considered for cross-domain federation and deferred as premature; the per-domain key shape is the federation seam, not a blocker to it.

A **revision** is the `LSN` at which a specific view (command, session, actor, grant, audit record) was last durably updated. A **cursor** is an `LSN` a control surface or adapter holds to express "I have authoritative knowledge up to here."

V0 revision/cursor rules:

- Every snapshot carries the `LSN` it was materialized at and the per-view revisions it reflects.
- A control surface reconciles by submitting its cursor; the core returns events with `LSN > cursor` and/or a snapshot materialized at a later `LSN`.
- A snapshot with an `LSN` strictly less than the core's current state for that view is **older** and is rejected as an authority source; the core returns the current view instead.
- A snapshot from a different authority domain or a different core generation is rejected outright.
- Late events carry the `LSN` at which they were committed; an event whose `LSN` is older than the view it would mutate is recorded as an audit/reconciliation event and does not rewrite the current view.
- The core may serve a compressed snapshot at any `LSN`; cursors remain valid across compaction because revisions are monotonic.

### Atomicity between events and snapshots

V0 requires the following atomicity guarantees at the persistence boundary:

- A command is durably recorded (`accepted`) before delivery is attempted. Delivery never relies on in-memory state.
- A terminal transition is committed to the log before it is reflected in snapshots or returned to control surfaces.
- A snapshot materialization reads a consistent log prefix: it reflects every event with `LSN <= snapshot_LSN` and no event with `LSN > snapshot_LSN`.
- Snapshot writes do not reorder the log. A snapshot is a derived artifact keyed by `LSN`; it never becomes a second source of ordering.

If the persistence backend cannot provide these atomicity guarantees, the core must treat the write as failed (`SubmissionOutcome = failed` for submissions, or `failed`/continued `accepted` per policy for delivery) rather than expose an inconsistent view.

## Presence and Subscription

Presence/Subscription is a named protocol section/registry, **not** a fifth primitive. Operations and Observations carry presence facts; the registry defines how they are interpreted and reconciled.

Subscription is the deliberate exception to lifecycle-bearing Operations. A subscription request is grant-checked at establish time at the transport layer, audited as a security-relevant decision, and reconciled by cursor on reconnect, but it is not durably recorded as an Operation and does not enter `OperationState`. This creates two authority mechanisms: grant-checked-with-lifecycle for Operations/Elicitations, and grant-checked-without-lifecycle for long-lived Subscriptions whose semantics do not fit a finite terminal Operation state. Elicitation delivery uses this subscription substrate: the core does not direct-address a specific endpoint per Elicitation; it fan-outs Elicitation events to all active, authorized subscriptions for the expected responder actor's Elicitation stream. On reconnect, the control surface re-subscribes and submits its cursor; the core replays authorized events with `LSN > cursor` and/or returns a fresh snapshot.

The section distinguishes these axes:

| Axis | Meaning | V0 registry/fields |
|---|---|---|
| Endpoint availability | Is a concrete endpoint connection/address reachable? | Reuse/align with `SessionConnectivityState`: `live`, `stale`, `offline`, `unknown`, `failed`; fields: endpoint id, device id, adapter generation, last authoritative LSN. |
| Actor presence | Is an actor currently represented by at least one usable endpoint, and with what attention posture? | `available`, `away`, `unavailable`, `unknown`; derived from endpoint observations and session connectivity state, never authority by itself. |
| Observation subscription | Which actor/endpoint/control surface is subscribed to which event/snapshot stream? | `subscribed`, `resuming`, `unsubscribed`, `failed`; fields: subscription id, authorized filter, cursor, last delivered LSN, audit id for establish/deny. |
| Attention-required state | Does a target require human/service attention? | `none`, `attention_requested`, `response_required`, `blocking`, `escalated`; source is Elicitation or adapter Observation. |
| Expected responder | Which actor should answer an Elicitation, and which endpoint actually did? | Field on Elicitation: `expected_responder_actor` (operator actor in v0). No `expected_responder_endpoint` is present in v0. Optional endpoint class/control-surface role, fallback/escalation policy, and responder generation are reserved seams. Response Operation audit records the actual responding endpoint. |
| Stale-presence reconciliation | What happens after disconnect/reconnect or missed presence events? | Presence Observations carry LSN/revision; reconnecting clients submit cursor; stale presence cannot be rendered as live; Elicitations may terminalize `stale` if opener/target generation is superseded. |

Implementation notes:

- Attach Operations establish or refresh endpoint availability and trigger snapshot/cursor reconciliation; Subscriptions are separate grant-checked transport establishments without Operation lifecycle.
- Elicitation streams are subscription streams: all authorized subscribed surfaces for the expected operator actor receive the Elicitation, and the first valid answer clears it everywhere.
- Observation streams are optimizations; snapshots repair missed events.
- Presence is a derived fact, not a query target. One-shot "is session X present?" reads route through snapshot/status `query` Operations under the uniform read lifecycle; there is no distinct `query-presence` OperationKind.
- Single-operator v0 has no separate presence-leak threat inside the operator's authority domain. Filter-scoped subscriptions for multi-operator presence-leak prevention are a reserved seam; v0 must not bake in a hard-to-retract rule that all presence is globally public.
- Push notifications are an attention-routing surface, not authority.

## Persistence and recovery

The coordination core owns durable command state, the event log, snapshots, and audit records through a storage port. V0 persistence assumptions:

- **Single-writer**: one authoritative core process writes to the log. There is no multi-writer, HA, or split-brain recovery in v0.
- **Local-first**: the default backend is embedded and local to the core process. Domain semantics must not depend on a specific storage engine.
- **Port-isolated**: the core reads/writes through a storage port; adapters and control surfaces never touch persistence directly.
- **Crash recovery**: on restart, the core replays the durable log to reconstruct in-memory state up to the last committed `LSN`. Accepted-but-not-yet-terminal commands are restored as `accepted` (or a later committed state) and continue through their lifecycle. No accepted command disappears silently after a crash.
- **Idempotent reprocessing**: replaying the log produces identical state. Re-delivery to adapters after recovery is governed by adapter capability and command policy, not by log replay.
- **Snapshot checkpointing**: snapshots are periodic materializations used to bound replay cost on recovery; they are derived artifacts, never an alternate source of truth. A recovery may load the latest snapshot then replay events with `LSN > snapshot_LSN`.

V0 does not require WAL replication, remote replication, point-in-time cloning, or storage-engine hot swap. Those are reserved seams.

## Authority grants

A grant authorizes a subject (an actor, optionally narrowed to an endpoint or endpoint class) to perform a set of OperationKinds against a target scope. Grants are explicit, revocable, and evaluated inside one authority domain.

A v0 grant records:

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

Delegation is a reserved future direction, not a v0 field. A `parent grant id / delegated-by` field is intentionally absent from v0; it must be designed together with delegation semantics and multi-operator / federated-authority work, both of which are outside v0 scope. Device is part of the identity model (for audit and revocation grouping) but is not a grant-matching field; grant matching uses the issuer actor and optional endpoint. Adapter capability sets are not grant authority (see Adapter capabilities).

### Spawn authority

Spawn is fleet-level by default in v0: a spawn grant authorizes spawning across any adapter/supervisor the operator can reach, before a target session exists. Adapter-level spawn grants remain expressible through the existing target-scope flexibility when narrower authority is desired; no schema change is needed. Per-spawn-variant authority (e.g. "may spawn worktrees but not cloud environments") is reserved.

Successful spawn completion records an explicit, auditable **descendant grant** for the spawned session: spawner/operator subject as subject, spawned session as target. This is an explicit grant record generated as part of spawn, not an implicit grant-matching rule. It preserves and builds the seam for future cross-operator delegation over spawned sessions: a future delegated grant can reintroduce `parent_grant_id` and reference the auto-issued descendant grant directly; that is same infrastructure, not v0 cross-actor delegation.

Revocation uses two independent levers: revoking the spawn grant prevents future spawns, but already-spawned sessions keep operating under their auto-issued descendant grant until that grant is separately revoked. No cascade-revoke is v0 behavior; future cascade is a query over grant provenance and needs no schema change.

Grant checks happen before command acceptance. A submission without a live matching grant is rejected before delivery with `SubmissionOutcome = rejected` and `authorization_denied` or a narrower applicable failure term.

Authorization is deny-by-default. Control surfaces may hide unavailable actions, but UI availability is never authority. Sender identity is derived from the verified connection/session context, not from self-asserted payload fields, display names, project labels, cwd metadata, or adapter-reported friendly names.

Revocation prevents future authority. Already accepted commands follow the policy attached to their grant and OperationKind: continue, cancel where supported, or require reauthorization. Revocation does not delete command history; late events after revocation are audit/reconciliation events unless they are valid transitions for commands already accepted under the relevant policy.

V0 revocation actions include current-session revocation, all-session revocation, endpoint/device revocation, adapter/session grant revocation, and security lockdown. A lockdown rejects new commands, marks affected runtime sessions stale, requires fresh login, and records the reason.

## Leases

A lease is a time-bounded exclusive claim over a resource or coordination role. A lease has:

- resource id;
- holder actor;
- scope;
- expiration;
- renewal rules;
- release rules.

Within one modeled Patchbay authority domain, two live leases cannot grant exclusive ownership of the same resource and scope at the same time.

V0 reserves leases as an extension seam. Lease-backed behavior must define its own lifecycle registry before shipping; it must not overload `CommandState` or session state.

## Adapter capabilities

Adapters declare supported Operations and guarantees in a capability manifest:

- supported `OperationKind`s (and, for `spawn`, supported `target_spec.shape` values);
- streaming support (boolean);
- snapshot support (authoritative / partial / none);
- cancellation support (boolean);
- session replacement support (boolean);
- idempotency strength (`none` / `at-Patchbay-boundary` / `end-to-end`);
- attachment method (adapter-specific descriptor);
- known failure modes (advisory list mapping to the failure vocabulary).

Each capability is shaped by where the core's behavior branches. Snapshot support is tiered because the core's reconciliation contract on reconnect depends on the tier. Idempotency strength is an enum because the core's retry behavior depends on it. Streaming, cancellation, and session replacement are boolean: the core does the same thing regardless of the value beyond display.

Control surfaces render unsupported actions as unavailable rather than attempting best-effort hidden behavior.

Adapter capability declarations are advisory for control-surface UX only: they let a control surface render an action unavailable before submission. They are not an authority gate and not a delivery gate. The core does not gate delivery on a cached adapter capability; it delivers the OperationKind to the adapter, and the adapter accepts or rejects based on its own support at delivery time. An adapter's `unsupported_command` is a delivery-layer, adapter-reported rejection. An unknown-to-Patchbay OperationKind is `validation_failed` at submission, before a grant is evaluated. Grant authority is expressed only in canonical Patchbay OperationKinds, which are stable and registry-owned; an adapter capability change never widens or narrows a grant.

### Adapter registration and lifecycle

An adapter is a **principal** with an explicit registration lifecycle. At attach time it submits (a) attachment evidence verified by an adapter-specific trust root (the Pi adapter uses configured local material; future adapters may use mTLS or OAuth — the mechanism is adapter-specific, not mandated by the core), and (b) its capability manifest. The core records the adapter id, capability manifest, attach LSN, and adapter generation (adapter-reported, monotonic per adapter, used to reject stale events from a prior adapter attachment).

Attach, detach, failure, and capability redeclaration are audit events. Capability redeclaration is allowed with audit; when an adapter loses a capability it previously had, the core records the change and degrades affected sessions per the rules below. Sessions discovered or reported by the adapter inherit the adapter's authenticated channel.

### Adapter snapshot capability tiers

Adapter snapshot support is not boolean. V0 recognizes three tiers:

- **Authoritative snapshot** — the adapter can return a complete, authoritative view of the session at a session generation the core can reconcile. The core treats this as a valid snapshot source and may use it to repair missed events.
- **Partial snapshot** — the adapter can return some state (e.g. command history or last-known status) but cannot fully reconstruct the session view. The core marks the unreconciled axes `unknown` or `stale` per `SessionConnectivityState`/`SessionActivityState` rather than synthesizing live state.
- **No snapshot** — the adapter cannot snapshot. The core holds the last-known cached view marked `stale` (or `unknown` if no cached view exists) and does not present it as live. Reconnect after missed events cannot be repaired by a snapshot; the control surface must reconcile against command/event records it can still query, and present unreconciled session state honestly.

Degraded behavior rules:

- The core never fabricates a snapshot from optimistic UI or cached state when an adapter reports no or partial snapshot capability.
- A `partial` or `no snapshot` adapter does not weaken durable command state: accepted commands and their `CommandState` remain authoritative from the core's log.
- If an adapter loses the ability to snapshot it previously had, the core records the capability change as an audit record and moves affected sessions to `stale` or `unknown` until a fresh authoritative signal arrives.
- If an adapter claims an `authoritative` snapshot but returns a snapshot that is incomplete, malformed, non-monotonic, targeted at the wrong session generation, or otherwise non-conformant with its declared capability, the core rejects it as an authority source, records an audit record, and degrades the affected session axes to `stale` or `unknown`. An adapter that repeatedly fails its declared snapshot capability may have that capability reclassified by the core; reclassification is itself an audited capability change. The core never promotes a rejected snapshot to authoritative.

## Extension pressure classification

- **Committed v0 behavior:** `SubmissionOutcome`, `CommandState` (the checked lifecycle registry, reused by `OperationState` refinement equivalence), `LocalSubmissionState`, `SessionConnectivityState`, `SessionActivityState`, the `OperationKind` registry (committed kinds: `spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management`), the `ElicitationState` lifecycle (stated-normative until promoted), the `response_contract` registry (committed contract kinds: `approval`, `question`), the five id spaces, the Presence/Subscription axes, failure vocabulary, idempotent retry at the Patchbay boundary, and stale/unknown presentation honesty.
- **Reserved extension seams:** adapter-specific diagnostics, future OperationKinds (including per-variant spawn kinds), reserved `response_contract.contract_kind` values (`freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`), reserved `agent-send` and `adapter-utility-exec` OperationKinds (rejected with `validation_failed` in v0), non-operator Operation senders (agent→agent, adapter→operator service Operations), no-lifecycle reads optimization, tighter Elicitation responder binding (endpoint/endpoint class/fallback chain), responder-actor distinction for multi-operator sessions, cross-actor delegation through `parent_grant_id`, per-spawn-variant authority, presence-leak prevention for multi-operator, multi-answer/quorum Elicitations, richer activity details, multi-operator authority domains, lease lifecycle, native/mobile-specific local cache states, and additional control surfaces.
- **Rejected direction:** Pi-specific state names, UI-only optimistic states, transport-specific errors, adapter-specific lifecycle variants becoming core protocol states without registry updates, a generic operator-originated no-grant `Message` as a v0 action, and a `query-presence` OperationKind (presence is a derived fact, not a query target).

## Security and trust boundary

Patchbay protocol assumes cryptographic primitives work as specified by their libraries and deployments. Formal models cover authority and identity relationships, not primitive cryptographic correctness.

Browser control uses server-side operator sessions with hardened cookies and CSRF protection for state-changing requests. Browser-local UI state is never authority for command submission, grant status, or session liveness.

Sender identity is derived from verified connection/authentication context, not from self-asserted display names or payload fields. External actor identities remain claims until verified by an adapter-specific trust root or deployment policy.

Security audit records are durable protocol-adjacent records for authentication, authorization, session management, command lifecycle, revocation, adapter attach/detach/failure, and stale-event rejection. Audit records are distinct from durable command/session state-transition events: they may record rejected attempts and failed checks that do not create command records. Audit records must not directly store raw session cookies, CSRF tokens, access tokens, passwords, bootstrap secrets, encryption keys, command prompt bodies by default, or sensitive attachments.
