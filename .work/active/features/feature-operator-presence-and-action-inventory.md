---
id: feature-operator-presence-and-action-inventory
kind: feature
stage: drafting
tags: [foundation, protocol, adapter]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot, feature-research-harness-action-surfaces]
created: 2026-07-02
updated: 2026-07-05
gate_origin: null
release_binding: null
---

# Feature: Sharpen operator-presence positioning and derive the operator↔harness↔agent action inventory

## Brief

Patchbay's foundation docs specify, exhaustively, the *states* an accepted action passes through (`CommandState`, `SubmissionOutcome`, session axes, failure vocabulary). They do **not** yet specify the normative *action set* — what an operator, harness, agent, adapter, or service can actually do through Patchbay, which actions the core must durably carry, and which content remains payload interpreted by the harness.

This feature rolls Patchbay's positioning thesis forward and defines the action inventory that downstream protocol contracts, adapter parity checks, and UX acceptance criteria derive from. The design supersedes the stale five-primitive spine (`spawn/attach/operate/receive/payload`) with the reviewed, actor-neutral **Operation / Observation / Elicitation** frame. Spawn and attach remain real surveyed actions, but they are `OperationKind`s rather than top-level primitives. Payload remains content carried by an Operation, Observation, or Elicitation; it is not a protocol primitive that grants authority by itself.

## Design stance

This is a foundation design pass only. It writes the reviewable design into this item and intentionally does **not** edit foundation docs, spawn child stories, advance `stage`, or commit. The next adversarial review should attack this body before implementation edits `docs/VISION.md`, `docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/SPEC.md`, `docs/UX.md`, `docs/SECURITY.md`, and `docs/GLOSSARY.md`.

## 1. Positioning thesis (D1 roll-forward)

### Thesis

Patchbay's core is a **network-reachable fixed point**. Operator surfaces and agent/harness machines are both reconnecting clients of that fixed point. Neither side is load-bearing for the other:

- the operator's phone/laptop/desktop/CLI may disconnect or power off while accepted work remains visible and recoverable through core state;
- an agent/harness machine may disconnect or be replaced without invalidating other sessions or the operator's ability to control reachable targets;
- adapter processes run near runtimes when useful, but they do not become the durable source of truth;
- the v0 one-host setup is a deployment convenience, not the architecture.

This thesis is grounded in the remote-agent prior art: the operator's SNC landscape distinguishes **pilot** (steer an already-running session) from **spawn** (cold-start fresh work from a remote device), and documents the deployed always-on `claude remote-control --spawn worktree --capacity 8` systemd pattern as the practical bridge machine workaround `[snc-rao-landscape]` `[snc-rao-sp-cc-remote-control]{1}`. Patchbay's value is to make that control plane harness-agnostic, durable, authority-bearing, and not dependent on a specific workstation staying awake.

### Exact foundation-doc changes to implement later

Do **not** edit these docs in this pass; these are implementation instructions.

#### `docs/VISION.md`

Replace the current second paragraph:

> Patchbay gives an operator a reliable cockpit for discovering sessions, sending intent, receiving correlated replies, approving or interrupting work, and recovering state after disconnection. It starts with a Pi adapter because Pi is the first workflow target, but Pi is an adapter, not the architecture.

with:

> Patchbay gives an operator a reliable cockpit for discovering sessions, spawning or attaching to runtime sessions, submitting authorized Operations, receiving source-authenticated Observations, answering Elicitations, and recovering state after disconnection. The coordination core is a network-reachable fixed point: operator surfaces and agent/harness machines are reconnecting clients of it, and neither side is load-bearing for the other. Patchbay starts with a Pi adapter because Pi is the first workflow target, but Pi is an adapter, not the architecture.

In `## Why Patchbay exists`, after the first paragraph, add:

> The core must remain reachable independently of any one operator device or harness host. A colocated v0 deployment is a convenience for installation and testing; it is not the architectural model. The architectural model is a durable coordination core that reconnecting surfaces and adapters can independently join.

In `## What Patchbay is`, replace:

> a durable message, command, snapshot, and authority layer;

with:

> a durable Operation, Observation, Elicitation, snapshot, and authority layer;

In `## Success criteria`, replace:

> accepted commands are durable and visible through a canonical lifecycle until terminal outcome;

with:

> accepted Operations are durable and visible through the `CommandState`-equivalent lifecycle until terminal outcome;

and replace:

> replies correlate to the command or message they answer;

with:

> replies and response Operations correlate through typed references to the command/message/elicitation they answer;

#### `docs/ARCHITECTURE.md`

Replace the `### Operator intent plane` section text:

> The operator intent plane represents prompts, commands, approvals, cancels, resumes, compactions, session switches, and other human-directed actions.
>
> Every accepted operator intent has a durable command state from the canonical `CommandState` registry in `docs/PROTOCOL.md`. Control-surface-local submission states are separate and never become durable core states.

with:

> The operation plane represents authorized control-plane requests through an actor-neutral vocabulary, while v0 admits only operator-originated Operations. It includes spawn, attach, instruct, cancel, interrupt, query, approval response, elicitation response, reconfiguration, and session-management Operations; non-operator Operation senders remain reserved seams.
>
> Every accepted Operation initially reuses the canonical `CommandState` registry in `docs/PROTOCOL.md` by documented refinement equivalence. Control-surface-local submission states are separate and never become durable core states. A future rename to `OperationState` must update prose, generated contracts, formal models, conformance vectors, and implementations together.

Replace the subsection heading:

> ### Message and command plane

with:

> ### Operation, Observation, and Elicitation plane

and replace that subsection's first paragraph with:

> This plane defines Operation acceptance, delivery, reply/response correlation, idempotency, retries, expiration, failure semantics, source-authenticated Observations, and durable Elicitations. Its state machines and failure vocabulary are owned by `docs/PROTOCOL.md` until generated contracts take over as derived boundary artifacts.

In `### V0 process topology`, replace the existing reserved-seam bullet:

> **Split deployment**: the web server may run near the operator and the core elsewhere once the internal protocol seam is designed; v0 may colocate them on one host for simplicity.

with:

> **Split deployment**: the web server, CLI, core, and adapters may run on different machines. V0 may colocate them on one host for installation simplicity, but that colocation is a deployment convenience, not the architecture. The Rust coordination core remains the network-reachable fixed point and the single durable writer.

In `## Data flow`, replace the numbered flow with one that says: in v0 a control surface submits an operator-originated Operation; the core validates identity/authority/target/idempotency/kind/payload; accepted Operations are durably recorded before delivery; adapters and actors emit Observations; adapter/agent/harness openers create Elicitations over authenticated adapter channels; reconnecting surfaces reconcile by cursor and snapshot. Non-operator Operation submitters remain reserved seams.

## 2. Normative action inventory

This inventory is the normative action registry. `feature-protocol-idl-and-conformance` inherits this registry and derives its `.proto` enum/wire representation from it; the contract feature does **not** invent a separate command-kind list. If `.proto` needs a new action kind, the product-vocabulary registry in `docs/PROTOCOL.md` changes first, then `.proto`, models, vectors, and implementation follow.

Disposition meanings:

- **Committed v0** — part of the v0 protocol/action registry. Adapter support may still be capability-declared, but the core recognizes the action kind and semantics.
- **Reserved seam** — named now to avoid foreclosure, but not required for first Pi-backed executable behavior unless a later feature promotes it.
- **Rejected for v0** — explicitly not a v0 protocol action.

Scope honesty: **v0 Operations are operator-originated. The actor-neutral sender vocabulary is a reserved seam for non-operator senders (agent→agent, adapter→operator service Operations). v0 does not mediate non-operator-originated authority-bearing Operations.** Command/message ids therefore remain client-generated in the operator domain as already committed in `docs/PROTOCOL.md`; no new id-assignment rule is needed for non-operator Operation senders in v0. Elicitation openers are different: agents/adapters/harnesses may open Elicitations, and the operator answers through response Operations.

| Action | Primitive | `OperationKind` | `{sender, recipient}` actor classes | What it does | V0 disposition | Surveyed evidence |
|---|---|---|---|---|---|---|
| Spawn runtime/session/fleet target | Operation | `spawn` | operator/control surface → core/fleet supervisor/adapter | Creates a runtime session, process, thread, cloud agent, sidecar session, or harness instance that did not previously exist. Spawn is one `OperationKind`; the payload carries `target_spec.shape` from a reserved open shape registry for vocabulary/audit/display such as "worktree", "session", "process", or "cloud environment". Target authority is fleet-level by default: any adapter/supervisor the operator can reach, with adapter-level grants still available through the target-scope registry when narrower authority is desired. | **Committed v0** as an operator-originated registry kind and authority model obligation. Concrete Pi support may require an adapter-side supervisor, but spawn is not merely a reserved seam. Shape support is adapter-declared and accepted/rejected by the adapter at delivery time. | Pi supervisor is currently out-of-band `[pi-extension]`; Claude Remote Control `--spawn worktree|same-dir|session --capacity N` `[snc-rao-sp-cc-remote-control]{1}`; Dispatch `[snc-rao-sp-cc-desktop]{2}`; Codex `thread/start/resume/fork` and `process/spawn` `[codex-appserver-readme]{5}` `[codex-appserver-types]{10}`; Cursor `POST /v1/agents` `[cursor-cloud-agents-api]{4}`; OpenCode `serve` `[snc-rao-ae-opencode-cli]{3}`; Antigravity local `Agent` sidecar and managed `environment="remote"` `[antigravity-sdk-repo]{2}` `[antigravity-managed-agent]{3}`. |
| Attach / reconnect / reconcile | Operation | `attach` | operator/control surface → core/runtime session/adapter | Connects or reconnects a control surface to an existing session or server and reconciles by cursor/snapshot/session generation. No work payload is implied. Transport subscriptions are grant-checked separately at establish time, not modeled as Operations. | **Committed v0.** Without attach/reconcile, remote operation and cross-device continuity fail. Adapter attachment itself remains the existing adapter registration/audit path rather than a non-operator Operation. | Pi `pair_request` + `session_sync` `[pi-extension]`; Claude remote/mobile connection and sync `[claude-code-remote-control]{5}`; Codex app-server client subscriptions `[codex-appserver-readme]{6}`; Cursor Cloud SSE stream `[cursor-cloud-agents-api]{8}`; OpenCode client connect to `serve` `[opencode-session-handler]` `[snc-rao-ae-opencode-cli]{3}`; Antigravity `LocalConnection` handshake `[antigravity-sdk-repo]{2}`. |
| Instruct a turn / prompt / user input | Operation | `instruct` | operator/control surface → agent/runtime session | Sends content that begins a turn, generation, run, prompt loop, or user-input item. Prompt text and slash-commands are payload, not separate protocol primitives. | **Committed v0** only for operator-originated instruct. Agent-trigger/service-originated instruct is a reserved actor-neutral seam, not v0 mediated behavior. | Pi `user_message` `[pi-extension]`; Claude CLI/SDK/Remote Control query/message `[claude-code-cli]{1}` `[claude-code-sdk-client]{5}` `[claude-code-remote-control]{5}`; Codex `turn/start` `[codex-appserver-readme]{5}`; Cursor prompt/run creation `[cursor-agent-overview]{1}` `[cursor-cloud-agents-api]{6}`; OpenCode `prompt`/`promptAsync`/`shell`/`loop` `[opencode-session-handler]`; Aider chat/`--message` `[aider-base-coder]{3}` `[aider-args]{1}`; Antigravity `Agent.chat()` / `Conversation.send()` `[antigravity-sdk-repo]{2}`. |
| Steer in-flight work | Operation | `instruct` | operator/control surface → running turn/session | Adds input or redirection to an in-flight turn without necessarily cancelling it. It is an `instruct` refinement, not a new primitive. | **Committed v0** only as the operator-originated `instruct` kind when an adapter declares in-flight steering; unsupported adapters reject at delivery. Automated trigger steering is reserved. | Codex `turn/steer` `[codex-appserver-readme]{5}` `[codex-appserver-types]{4}`; Cursor queued/immediate messages while Agent is working `[cursor-agent-overview]{5}`; Antigravity triggers can send automated content into an agent `[antigravity-sdk-repo]{2}`. |
| Agent-to-agent routed message | Operation | `agent-send` | agent/service/adapter → agent/service/adapter | Reserved slot for agent→agent mesh, op→op routing, and other non-operator Operation directions that need routing/audit/correlation. | **Reserved seam.** `agent-send` is named in the `OperationKind` registry but is not validatable in v0; submissions are rejected with `validation_failed`. Promotion is a registry update, not a schema change. | Reserved design seam informed by the remote-pi mesh's `agent_send`/`agent_request` prior art (not one of the 7 surveyed harnesses) and by Antigravity trigger / Codex service-request pressure. |
| Standalone adapter utility execution | Operation | `adapter-utility-exec` | operator/control surface → adapter utility surface | Reserved slot for standalone server-side command/process utility execution that does not create a thread/turn or persistent runtime session. | **Reserved seam.** Named in registry, not validatable in v0; submissions reject with `validation_failed`. Codex `command/exec` and `process/spawn` are the surveyed pressure; full lifecycle/idempotency modeling is deferred. | Codex `command/exec` and `process/spawn` `[codex-appserver-readme]` `[codex-appserver-types]`. |
| Cancel / interrupt active work | Operation | `cancel` or `interrupt` | operator/control surface → active Operation/turn/session | Requests an in-flight instruct/session action to stop. It races with completion under first durable terminal commit semantics; a late cancel does not rewrite a completed Operation. | **Committed v0.** Adapter capability determines delivery; core lifecycle semantics are fixed. Policy terminalization may also happen as core policy, but that is not a non-operator Operation. | Pi `cancel` `[pi-extension]`; Claude `interrupt()` / `stop_task` / stop commands `[claude-code-sdk-client]{6}` `[claude-code-sdk-types]{11}` `[claude-code-agent-view]{6}`; Codex `turn/interrupt` `[codex-appserver-readme]{5}`; Cursor cancel run `[cursor-cloud-agents-api]{11}`; OpenCode remove/cancel `[opencode-session-handler]`; Aider Ctrl-C `[aider-base-coder]{7}`; Antigravity `cancel()` / `halt_request` `[antigravity-sdk-repo]{2}`. |
| Submit approval decision | Operation | `approval-response` | authenticated operator endpoint for expected operator actor → opener agent/harness/adapter | Responds to a pending permission/tool approval Elicitation with allow/deny/allow-once/always/modified-input/policy-amendment where supported. Any authenticated operator endpoint may respond in v0; the response Operation audit captures which endpoint actually answered. | **Committed v0.** Tool approval is Pi-relevant and common; model promotion is required before product semantics are claimed checked. Non-operator/service responders and tighter endpoint binding are reserved. | Pi `approve_tool` `[pi-extension]`; Claude permission callbacks and hooks `[claude-code-user-input]{7}` `[claude-code-user-input]{8}` `[claude-code-hooks]{8}`; Codex command/file/permission approvals `[codex-appserver-readme]{8}` `[codex-appserver-types]{7}`; Cursor Run Mode/MCP approvals `[cursor-run-modes]{1}` `[cursor-mcp-extension]{8}`; OpenCode `permission.v2.replied` `[opencode-schema-events]`; Aider confirmation/`--yes-always` `[aider-args]{1}`; Antigravity `tool_confirmation` `[antigravity-sdk-repo]{2}`. |
| Submit elicitation answer/result | Operation | `elicitation-response` | authenticated operator endpoint for expected operator actor → opener agent/harness/adapter/service | Responds to a non-approval Elicitation. In committed v0, valid responses cover `question` contract kinds. Any authenticated operator endpoint may respond; first-answer-wins terminalizes the Elicitation for all surfaces and records the responding endpoint in the response Operation audit. Reserved contract kinds (`freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`) are named but not validatable until promoted. | **Committed v0** for the OperationKind and lifecycle; contract-kind support is committed only for `question` here (with `approval` handled by `approval-response`). `freeform` is a reserved contract seam. Unsupported/unknown contract kinds reject at submission with `validation_failed` unless a later registry update promotes them. | Claude `AskUserQuestion` answers `[claude-code-user-input]{1}` `[claude-code-user-input]{10}` `[claude-code-user-input]{11}`; Codex `item/tool/requestUserInput`, MCP elicitation, auth-token refresh/current-time/attestation/dynamic tool requests `[codex-appserver-protocol]{7}` `[codex-appserver-types]{8}`; OpenCode `question.replied` / `question.rejected` `[opencode-schema-events]`; Antigravity `question_response`, custom function results, managed `requires_action` `[antigravity-sdk-repo]{2}` `[antigravity-managed-agent]{3}`. |
| Query / read / status / snapshot refresh | Operation | `query` | operator/control surface → core/adapter/session/resource | Reads session status, snapshots, model lists, thread/session lists, capability state, history, tokens/settings, or other state. Read-only does **not** mean grant-free. | **Committed v0.** Reads use the full `OperationState` lifecycle by refinement to `CommandState`; they may skip `running`, but they do not use a direct-to-completed fast path. A no-lifecycle reads optimization is a reserved seam if polling volume warrants it later. | Pi `session_sync`, `list_models`, `ping` `[pi-extension]`; Claude session/model/MCP status reads `[claude-code-sessions]{10}` `[claude-code-sdk-client]{10}`; Codex `thread/list`, `thread/read`, `model/list`, capabilities/read `[codex-appserver-readme]{5}`; Cursor list/read agents/runs `[cursor-cloud-agents-api]{5}` `[cursor-cloud-agents-api]{7}`; OpenCode `status`, `summary.diff` `[opencode-session-handler]`; Aider `/tokens`, `/ls`, `/map`, `/settings`, `/help` `[aider-commands]{3}`; Antigravity history/is_idle/usage/polling `[antigravity-sdk-repo]{2}` `[antigravity-managed-agent]{3}`. |
| Reconfigure runtime/session/model/policy/tools | Operation | `reconfigure` | operator/control surface → core/adapter/session | Changes model, thinking/reasoning effort, permission mode, MCP/tool configuration, agent mode, environment/workspace settings, or other declared configuration. | **Committed v0** for registry; concrete knobs are adapter capabilities/payload schema. Policy/service-originated reconfiguration is reserved. | Pi `model_set`, `thinking_set` `[pi-extension]`; Claude `set_model`, `set_permission_mode`, MCP toggle/reconnect, CLI/SDK model/permission flags `[claude-code-sdk-client]{7}` `[claude-code-sdk-client]{8}` `[claude-code-sdk-client]{10}`; Codex thread/turn settings update `[codex-appserver-types]{1}` `[codex-appserver-types]{2}`; Cursor Run Modes and MCP/plugin registration `[cursor-run-modes]{3}` `[cursor-extension-api]{1}`; OpenCode model/agent switched events and handlers `[opencode-schema-events]`; Aider `/model`, `/editor-model`, `/weak-model` `[aider-commands]{3}`; Antigravity capability/model/workspace/MCP/subagent config `[antigravity-sdk-repo]{2}`. |
| Session management / lifecycle mutation | Operation | `session-management` | operator/control surface → core/adapter/session/thread/agent resource | Resumes, forks, archives, deletes, compacts, clears, resets, removes messages, reverts, shares, checkpoints/restores, disconnects, retires, or stops sessions/resources after they exist. | **Committed v0** as registry family; each adapter declares supported sub-actions. Spawn remains separate because its target may not exist yet. Service-originated session management is reserved. | Pi `session_new`, `session_compact` `[pi-extension]`; Claude resume/fork/compact/clear/background stop/rm/respawn `[claude-code-sessions]{4}` `[claude-code-slash-commands]{3}` `[claude-code-agent-view]{9}`; Codex thread resume/fork/archive/delete/unarchive/compact `[codex-appserver-readme]{5}`; Cursor checkpoint restore, archive/unarchive/delete Cloud Agent `[cursor-agent-overview]{4}` `[cursor-cloud-agents-api]{12}` `[cursor-cloud-agents-api]{13}`; OpenCode remove/removeMessage/revert/unrevert/compact/share `[opencode-session-handler]`; Aider `/load`, `/save`, `/undo`, `/commit`, `/clear`, `/reset` `[aider-commands]{3}`; Antigravity resume/disconnect/clear history `[antigravity-sdk-repo]{2}`. |
| Receive output / lifecycle facts / results | Observation | n/a | agent/adapter/core/service → operator/control surface/core/subscriber | Emits source-authenticated output, chunks, assistant messages, tool calls/results, turn/session lifecycle, errors, status facts, compaction events, or delivery acknowledgements. Observations do not grant authority to act, but they require source identity, correlation context, and cursor/LSN treatment where durable. | **Committed v0.** Observation streams are not authoritative alone; snapshots/core records reconcile. | Pi `message_update`, `tool_call`, `tool_execution_*`, `agent_end` `[pi-extension]`; Claude assistant/tool/result/hook/task streams `[claude-code-sdk-types]{6}` `[claude-code-sdk-types]{7}` `[claude-code-sdk-types]{12}`; Codex notifications and item deltas `[codex-appserver-readme]{6}` `[codex-appserver-readme]{7}`; Cursor CLI NDJSON and Cloud SSE `[cursor-cli-output]{3}` `[cursor-cloud-agents-api]{8}`; OpenCode SSE events `[opencode-schema-events]`; Aider local output/streaming `[aider-io]{1}` `[aider-base-coder]{4}`; Antigravity chunks/steps/hooks/tool results `[antigravity-sdk-repo]{2}`. |
| Open approval elicitation | Elicitation | n/a | agent/harness/adapter → expected operator actor via subscription fan-out | Opens a durable pending response slot for permission/tool approval. Carries opener, `expected_responder_actor` (operator in v0), response contract, target context, timeout/cancellation/withdrawal policy, and correlation; it does not bind to a specific endpoint. Delivery rides the operator's Elicitation subscription stream and fans out to subscribed surfaces. | **Committed v0.** Approval gates are common and Pi-relevant. Core does not open Elicitations in v0; non-operator/service responders and tighter endpoint binding are reserved; model promotion is required before product semantics are claimed checked. | Pi `tool_call` approval gate `[pi-extension]`; Claude tool permission requests `[claude-code-user-input]{1}` `[claude-code-user-input]{4}`; Codex command/file/permission approval requests `[codex-appserver-protocol]{6}` `[codex-appserver-types]{7}`; Cursor local tool/MCP approvals `[cursor-run-modes]{1}` `[cursor-mcp-extension]{8}`; OpenCode `permission.v2.asked` `[opencode-schema-events]`; Antigravity tool confirmation requests `[antigravity-sdk-repo]{2}`. |
| Open question/input/service elicitation | Elicitation | n/a | agent/harness/adapter → expected responder actor via subscription fan-out | Opens a durable pending response slot for a question or user-input request. V0 binds committed human-facing Elicitations to the operator actor, not a specific endpoint; subscribed operator surfaces receive the prompt through subscription fan-out. Reserved future promotions may use the same shape for service/function/secret/attachment/schema responses, non-operator responders, or tighter responder binding. | **Committed v0** for agent/adapter-opened operator questions/input using committed contract kinds. Service requests and non-operator responders are reserved seams. Core prompts are not Elicitations. | Claude `AskUserQuestion` `[claude-code-user-input]{1}` `[claude-code-user-input]{10}`; Codex user-input/MCP elicitation/auth-token/current-time/attestation/dynamic-tool requests `[codex-appserver-protocol]{7}` `[codex-appserver-types]{8}`; OpenCode `question.asked` `[opencode-schema-events]`; Antigravity `ASK_QUESTION`, function requests, triggers/service-style interactions `[antigravity-sdk-repo]{2}` `[antigravity-managed-agent]{3}`. |
| Carry interpreted content | Payload | n/a | any primitive's sender → recipient | Holds text, slash-command strings, images, mentions, skills, shell passthrough, schemas, function arguments/results, file/attachment references, or adapter-specific bodies. Patchbay validates envelopes/contracts; the harness interprets payload semantics. | **Committed v0** as carried content, not a standalone authority primitive. Operator-originated no-grant `Message` is rejected for v0; content that drives work is payload of `instruct`. | Pi slash-command text in `user_message` `[pi-extension]`; Claude slash-command prompt strings `[claude-code-slash-commands]{1}`; Codex typed `UserInput` entries `[codex-appserver-types]{5}`; Cursor prompts `[cursor-agent-overview]{1}`; OpenCode prompt text `[opencode-session-handler]`; Aider `/` and `!` command payloads `[aider-commands]{4}`; Antigravity `user_input` / `complex_user_input` `[antigravity-sdk-repo]{2}`. |
| Generic operator-originated no-grant replyable message | Rejected v0 direction | n/a | operator/control surface → agent/session | A replyable informational message that is not an authorized instruct/query/response Operation. | **Rejected for v0.** No surveyed harness exposes this as a distinct operator action. The id space remains reserved for compatibility and future evidence. | Campaign synthesis separates Q-A from Q-B: no generic operator-originated no-grant Message; agent-originated Elicitations are real. Concrete Q-B evidence: Claude `[claude-code-user-input]{1}` `[claude-code-user-input]{10}`; Codex `[codex-appserver-protocol]{7}` `[codex-appserver-types]{8}`; OpenCode `[opencode-schema-events]`; Antigravity `[antigravity-sdk-repo]{2}`. |

## 3. Actor-neutral primitive definitions (A1)

These definitions should become protocol prose.

### Operation

An **Operation** is an authorized control-plane request by an actor to an actor, core, adapter, fleet, session, service, or resource target. An Operation may be side-effecting, read-only, lifecycle-acting, response-submitting, or fleet-creating. Operations require verified sender identity, target identity/scope, authority evaluation, a registry-owned `OperationKind`, boundary validation, idempotency semantics where applicable, and durable lifecycle state after acceptance.

v0 Operations are operator-originated. The actor-neutral sender vocabulary is a reserved seam for non-operator senders (agent→agent, adapter→operator service Operations). v0 does not mediate non-operator-originated authority-bearing Operations. Initial implementation reuses `CommandState` and command ids by refinement equivalence; command/message ids stay client-generated in the operator domain per the existing protocol. `Operation` is the actor-neutral vocabulary; `CommandState` is the checked lifecycle registry until the coordinated rename/model update occurs.

### Observation

An **Observation** is a source-authenticated fact, event, output, status emission, reply-like result, or lifecycle/status fact emitted by an actor, adapter, core, runtime, or service. Observations do not grant authority to act. They still require source identity, target/session/generation context where applicable, correlation context when they answer or relate to prior work, and LSN/cursor/snapshot reconciliation if durable.

Live streams are delivery optimizations. Durable core records and snapshots remain the authority for accepted Operations and reconciled state.

### Elicitation

An **Elicitation** is a durable pending response solicitation from one actor/system component to another. It opens a response slot rather than answering a prior request. It carries an adapter-assigned `ElicitationId`, opener, `expected_responder_actor` (the operator actor for committed v0 human-facing Elicitations), target/session/generation context, `response_contract`, timeout/cancellation/withdrawal policy, correlation to the work that caused it, and terminal lifecycle state. It does **not** carry an `expected_responder_endpoint` or bind to a specific operator-session endpoint in v0. The core assigns the durable LSN when it records the Elicitation, as for other durable events.

Elicitation delivery rides the subscription layer: the Elicitation is fan-out delivered to every surface with an active, grant-checked subscription to the operator actor's Elicitation stream. Any authenticated endpoint for that operator actor may answer. First-answer-wins terminalizes the Elicitation (`answered` or the applicable terminal) for all subscribed surfaces; later response attempts from other surfaces are rejected as already-terminal/stale candidates and recorded with the same `stale_event` audit treatment used for late terminal candidates. The endpoint that actually answers is captured in the response Operation's audit record at response time; it is not pre-bound in the Elicitation record.

Elicitation is actor-neutral as a future-proof vocabulary: agent→operator questions, harness→client service requests, service→operator secret prompts, and future agent→agent/op→op solicitations use the same primitive when promoted. In v0, the opener is always an adapter/agent/harness, never the core; agents/adapters open Elicitations such as `AskUserQuestion`, tool-input requests, and approval gates. A response is submitted as an operator-originated `OperationKind = elicitation-response` or `approval-response` Operation correlated to the Elicitation. Two seams are explicit: the responder-binding seam is preserved by v0's operator-actor binding while endpoint/class/fallback-chain binding remains reserved; the responder-identity audit seam is built by recording issuer endpoint in the response Operation audit, with future multi-operator work adding responder-actor distinction when multiple operators can share a session.

Core prompts are **not** Elicitations. Lockdown, expired/revoked sessions, CSRF rejection, and similar cases are core-imposed states enforced by Operation rejection or pre-protocol operator-session establishment. The protocol assumes a valid operator session exists; login, re-authentication, and lockdown exit are control-surface/web-server concerns outside the normative Operation/Elicitation flow.

### Payload

A **Payload** is the adapter-specific content or schema-bound body carried inside an Operation, Observation, or Elicitation. Examples: prompt text, slash-command text, typed user input entries, tool-call arguments, function results, image/file references, question options, structured schemas, or adapter diagnostics. Payload does not itself grant authority, create lifecycle state, or define protocol kinds.

## 4. `OperationKind` registry (A5)

One registry owns kinds, lifecycle policy, authority matching, adapter capability mapping, display labels, and generated contract variants. Adding or promoting a kind requires updating `docs/PROTOCOL.md`, `.proto`, model/vectors as applicable, and implementation together.

Initial registry:

| `OperationKind` | Meaning | Allowed `CommandState` / transition notes | V0 disposition |
|---|---|---|---|
| `spawn` | Create a new runtime/session/thread/agent/process/cloud resource; target is fleet-level by default before a session exists. This is one OperationKind; spawn variants are described by payload `target_spec.shape`, not by per-variant OperationKinds. | Full `CommandState` lifecycle by refinement: initial `accepted`; then `delivered`, optional `running`, or terminal. `running` is allowed for long provisioning. | Committed v0. Requires fleet-authority modeling. The `target_spec.shape` registry is reserved/open in v0 and adapter-enforced at delivery. |
| `attach` | Connect/reconnect a control surface endpoint to an existing session/server and reconcile. | Full lifecycle by refinement; may skip `running`, but not durable lifecycle. | Committed v0. |
| `instruct` | Send prompt/user input/steering content into a session/turn. | Full lifecycle allowed: `accepted → delivered → running → terminal`; in-flight steering may skip `running` if adapter reports immediate acceptance. | Committed v0 for operator-originated instruct. |
| `cancel` | Request cancellation of a target Operation/turn/session action. | Full lifecycle by refinement; the target Operation's terminal race is governed by first durable terminal commit, and cancellation completion does not rewrite an already-terminal target. | Committed v0. |
| `interrupt` | Request immediate stop/interrupt of active execution. | Same as `cancel`; reserved distinction for adapters that expose softer cancel vs harder interrupt. | Committed v0. |
| `query` | Read status, snapshot, capabilities, lists, history, metadata, or diagnostics. | Full lifecycle by refinement. Reads may skip `running`, but no v0 read uses a no-delivery direct-to-completed shortcut. A no-lifecycle read variant is reserved if polling volume warrants it later. | Committed v0. |
| `approval-response` | Respond to a permission/tool approval Elicitation. | Full lifecycle by refinement. Completion updates the Elicitation terminal (`answered` or `declined`) only if response validation succeeds and first-terminal rules allow. | Committed v0. |
| `elicitation-response` | Respond to non-approval Elicitations. | Full lifecycle by refinement. Invalid response Operation is rejected unless explicit Elicitation policy terminalizes the slot. | Committed v0 for `question` contracts; reserved for `freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, and `service_request` contracts. |
| `reconfigure` | Change model, reasoning/thinking level, permission mode, tools/MCP, agent mode, workspace, or adapter config. | Full lifecycle by refinement; `running` only for adapters with long reconfiguration. | Committed v0. |
| `session-management` | Resume, fork, compact, clear, archive/delete, revert, share/unshare, remove messages, checkpoint restore, disconnect/retire existing sessions/resources. | Full lifecycle by refinement because compaction/archive/delete can be long-running; quick local actions may skip `running`. | Committed v0. |
| `agent-send` | Reserved design seam for agent→agent mesh, op→op routing, adapter→operator service Operations, and other non-operator Operation directions. Informed by remote-pi mesh `agent_send`/`agent_request` prior art (not one of the 7 surveyed harnesses) and by Antigravity trigger / Codex service-request pressure. | Not validatable in v0. If submitted in v0, rejected before acceptance. | Reserved seam; v0 submissions reject with `validation_failed`. |
| `adapter-utility-exec` | Standalone adapter utility execution such as a sandboxed server-side command or connection-scoped process that does not create a thread/turn or persistent runtime session. | Not validatable in v0. If submitted in v0, rejected before acceptance; full lifecycle/idempotency modeling deferred. | Reserved seam; v0 submissions reject with `validation_failed`. |

Boundary rule: unknown `OperationKind` is `SubmissionOutcome = rejected` with `validation_failed` before grant evaluation. Reserved-but-not-validatable kinds such as `agent-send` and `adapter-utility-exec` also reject with `validation_failed` in v0. Unsupported-by-adapter known committed kind is a delivery-layer `unsupported_command` rejection after acceptance, matching the existing capability posture.

Spawn payload and authority commitments:

- **One `spawn` OperationKind.** Worktree, same-dir, session, process, thread, local sidecar, and cloud-environment spawns are not separate OperationKinds in v0. Per-variant OperationKinds are reserved only if a future registry update promotes them.
- **`target_spec.shape` = reserved open shape registry.** The spawn Operation payload includes `target_spec.shape`. V0 names shapes for vocabulary, audit, and display (for example, "spawned a worktree") but does not validate shape variants at the protocol layer. The adapter capability manifest declares which shapes the adapter supports; the adapter accepts or rejects the accepted Operation at delivery time with `unsupported_command`, consistent with the capability-not-authority discipline that the core does not gate on cached capability.
- **Target scope = fleet-level.** A v0 spawn grant authorizes all spawn variants across any adapter/supervisor the operator can reach. Adapter-level spawn grants remain expressible through the existing target-scope flexibility when narrower authority is desired; no schema change is needed.
- **Per-variant authority is reserved.** V0 does not implement "may spawn worktrees but not cloud environments" authority. If needed later, per-variant authority can be expressed through grant `target scope` or by promoting spawn variants to distinct OperationKinds; both are reserved seams, not v0 behavior.
- **Descendant authority = spawned-session manifest.** Spawn completion includes an auto-issued grant record for the spawned session: spawner/operator subject as subject, spawned session as target. This is an explicit, operator-visible, auditable grant record generated as part of spawn, not an implicit grant-matching rule. It preserves and builds the seam for future cross-operator delegation over spawned sessions.
- **Delegation remains out of v0.** `feature-design-grant-shape` intentionally removed `parent_grant_id` / delegated-by from the v0 grant shape. The auto-issued descendant grant is same actor (operator), new target (spawned session), not cross-actor delegation. A future delegated grant can reintroduce `parent_grant_id` and reference the auto-issued descendant grant directly; that is same infrastructure, not v0 cross-actor delegation.
- **Revocation uses two independent levers.** Revoking the spawn grant prevents future spawns. Already-spawned sessions keep operating under their auto-issued descendant grant until that grant is separately revoked. No cascade-revoke is v0 behavior; future cascade is a query over grant provenance and needs no schema change.
- **Idempotency is capability-manifest declared.** Spawn uses the adapter capability manifest's `idempotency strength` field (`none` / `at-Patchbay-boundary` / `end-to-end`). Most adapters likely declare `at-Patchbay-boundary`: Patchbay deduplicates the Operation record, while adapter retry may still create a duplicate external process. Adapters that track spawn idempotency internally may declare `end-to-end`. Duplicate external process reporting maps through the failure vocabulary; Patchbay does not solve adapter-side process duplication beyond its boundary dedup.

## 5. `ElicitationState` lifecycle (A3)

`ElicitationState` is a new registry, not a projection of `CommandState`.

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

Reserved future shapes:

- multi-responder quorum Elicitations;
- multi-answer accumulation;
- tighter responder binding to a specific endpoint, endpoint class, or fallback chain;
- delegated responder policy;
- escalation from one expected responder actor to another;
- cryptographic secret-entry envelopes;
- large file/attachment upload protocol;
- drawing/region-selection UI hints.

## 6. `response_contract` registry (A2)

Use `response_contract`, not `response-shape`. The registry describes what kind of response is semantically required; optional UI hints describe how a surface may render it.

Required fields:

- `contract_kind` — registry variant below;
- `schema_ref` or inline schema where structured validation is required;
- `ui_hints` — optional list such as `select-one`, `select-many`, `free-text`, `secret-input`, `upload`, `draw`, `confirm`, `diff-review`;
- `timeout_policy`;
- `invalid_response_policy`;
- `responder_policy` — v0 `expected_responder_actor` (operator actor for committed human-facing Elicitations); endpoint class, service role, fallback chain, and tighter binding are reserved. The responding endpoint is recorded in the response Operation audit, not pre-bound in the Elicitation;
- `sensitivity` — whether raw response may be logged, redacted, encrypted, or never persisted in plaintext.

Initial `contract_kind` registry:

The `elicitation-response` OperationKind is committed v0. The `response_contract.contract_kind` values have a committed/reserved split: committed v0 contract kinds are `approval` and `question`; reserved contract kinds are named in the registry but not validatable in v0. `freeform` is reserved because the solid surveyed grounding is currently Claude's optional `AskUserQuestion` freeform answer, while other surveyed response surfaces are structured question/answer, approval, secret, function-result, or service-request shapes rather than standalone unstructured Elicitation responses. A response submitted for an unknown or reserved/unsupported `contract_kind` is rejected at submission with `validation_failed` unless a later registry update promotes that contract kind.

| `contract_kind` | Semantics | Evidence | V0 disposition |
|---|---|---|---|
| `approval` | Allow/deny/allow-once/always/policy-amend/modified-input permission response. | Claude modified input / remember permissions `[claude-code-user-input]{7}` `[claude-code-user-input]{8}`; Codex command/file approvals `[codex-appserver-types]{7}`; OpenCode `permission.v2.replied` `[opencode-schema-events]`; Pi `approve_tool` `[pi-extension]`. | **Committed v0.** |
| `question` | Answer one or more questions, possibly with options and freeform text. | Claude `AskUserQuestion` `[claude-code-user-input]{10}`; Codex `requestUserInput` `[codex-appserver-types]{8}`; OpenCode `question.asked` `[opencode-schema-events]`; Antigravity `ASK_QUESTION` `[antigravity-sdk-repo]{2}`. | **Committed v0.** |
| `freeform` | Unstructured text response. | Claude optional freeform answer `[claude-code-user-input]{10}` is solid, but the earlier turn-input analogy was invalid; other surveyed response surfaces are structured question/answer, approval, secret, function-result, or service-request shapes. | **Reserved seam.** Named in registry, not validatable in v0 until broader Elicitation-response grounding is promoted. |
| `secret` | Provide sensitive secret/token/input with redaction/no-log policy. | Codex user-input question `is_secret` `[codex-appserver-types]{8}`; auth-token refresh request `[codex-appserver-protocol]{7}`. | **Reserved seam.** Named in registry, not validatable in v0. |
| `function_result` | Return custom tool/function result to a waiting service/harness. | Antigravity managed `requires_action` / function result `[antigravity-managed-agent]{3}`; Codex dynamic tool requests `[codex-appserver-protocol]{7}`. | **Reserved seam.** Named in registry, not validatable in v0. |
| `file_attachment` | Provide file/blob/image/attachment reference or upload. | Codex typed image/local image input `[codex-appserver-types]{5}`; Cursor/Antigravity image/file surfaces `[cursor-cloud-agents-api]{8}` `[antigravity-sdk-repo]{2}`. | **Reserved seam.** Named in registry, not validatable in v0. |
| `structured_schema` | Response must validate against declared JSON/protobuf/schema. | Antigravity finish schema and structured output surfaces `[antigravity-sdk-repo]{2}`; Codex output schema/typed requests `[codex-appserver-types]{3}` `[codex-appserver-types]{8}`. | **Reserved seam.** Named in registry, not validatable in v0. |
| `service_request` | Non-human service response such as current time, attestation generation, auth refresh, or adapter-provided evidence. | Codex `currentTime/read`, `attestation/generate`, auth-token refresh `[codex-appserver-protocol]{7}`. | **Reserved seam.** Named in registry, not validatable in v0. |

UI hints are optional sub-fields of `question` and `approval` contracts, not contract kinds. Examples include `select-one`, `select-many`, `free-text`, `upload`, and `draw`; the set is intentionally open and reserved for UI/adapters. UI hints are non-authoritative: changing a prompt from select-one to free-text does not change the protocol contract kind.

## 7. Id spaces (Decision 3)

Patchbay uses five separate id spaces:

1. **Command id** — client/operator-domain generated today; identity for accepted lifecycle-bearing records. During the vocabulary transition, accepted Operations reuse this id space by refinement equivalence. A future `OperationId` rename is a coordinated artifact rename, not a sixth id space.
2. **Message id** — reserved in v0 even though generic operator-originated no-grant `Message` drops. It remains in the registry because current `TypedCorrelation` and future non-command informational surfaces may need it.
3. **Reply id** — adapter-or-core assigned for correlated reply/observation records that answer prior command/message/operation context.
4. **Event id** — core-assigned LSN, keyed as `(authority_domain_id, LSN)`.
5. **Elicitation id** — new id space, adapter-assigned when a pending response slot is opened. The core assigns an LSN when it durably records the Elicitation; it does not assign the `ElicitationId` in v0.

Forgery-prevention justification:

- A response Operation must not be able to masquerade as the Elicitation it answers. Separate `CommandId`/`ElicitationId` spaces preserve direction: Elicitation opens a pending slot; response Operation answers it.
- A reply id cannot masquerade as command identity; the checked `TypedCorrelation` principle already enforces separate id spaces for command/message/reply and same-context typed references.
- `ElicitationId` is not a typed `ReplyId` subkind because an Elicitation is an initiation, while a Reply is a response. Modeling initiation as response inverts semantic direction and confuses lifecycle ownership.
- The existing `reply_correlation.qnt` does **not** cover response Operation → Elicitation. Extending typed correlation is a new verification obligation.

## 8. Presence / Subscription protocol section (A4)

Presence/Subscription is a named protocol section/registry, **not** a fifth primitive. Operations and Observations carry presence facts; the registry defines how they are interpreted and reconciled.

Subscription is the deliberate exception to lifecycle-bearing Operations. A subscription request is grant-checked at establish time at the transport layer, audited as a security-relevant decision, and reconciled by cursor on reconnect, but it is not durably recorded as an Operation and does not enter `OperationState`. This creates two authority mechanisms: grant-checked-with-lifecycle for Operations/Elicitations, and grant-checked-without-lifecycle for long-lived Subscriptions whose semantics do not fit a finite terminal Operation state. Elicitation delivery uses this subscription substrate: the core does not direct-address a specific endpoint per Elicitation; it fan-outs Elicitation events to all active, authorized subscriptions for the expected responder actor's Elicitation stream. On reconnect, the control surface re-subscribes and submits its cursor; the core replays authorized events with `LSN > cursor` and/or returns a fresh snapshot.

The section must distinguish these axes:

| Axis | Meaning | V0 registry/fields |
|---|---|---|
| Endpoint availability | Is a concrete endpoint connection/address reachable? | Reuse/align with `SessionConnectivityState`: `live`, `stale`, `offline`, `unknown`, `failed`; fields: endpoint id, device id, adapter generation, last authoritative LSN. |
| Actor presence | Is an actor currently represented by at least one usable endpoint, and with what attention posture? | `available`, `away`, `unavailable`, `unknown`; derived from endpoint observations and session connectivity state, never authority by itself. |
| Observation subscription | Which actor/endpoint/control surface is subscribed to which event/snapshot stream? | `subscribed`, `resuming`, `unsubscribed`, `failed`; fields: subscription id, authorized filter, cursor, last delivered LSN, audit id for establish/deny. |
| Attention-required state | Does a target require human/service attention? | `none`, `attention_requested`, `response_required`, `blocking`, `escalated`; source is Elicitation or adapter Observation. |
| Expected responder | Which actor should answer an Elicitation, and which endpoint actually did? | Field on Elicitation: `expected_responder_actor` (operator actor in v0). No `expected_responder_endpoint` is present in v0. Optional endpoint class/control-surface role, fallback/escalation policy, and responder generation are reserved seams. Response Operation audit records the actual responding endpoint. |
| Stale-presence reconciliation | What happens after disconnect/reconnect or missed presence events? | Presence Observations carry LSN/revision; reconnecting clients submit cursor; stale presence cannot be rendered as live; Elicitations may terminalize `stale` if opener/target generation is superseded. |

Implementation notes for docs:

- Attach Operations establish or refresh endpoint availability and trigger snapshot/cursor reconciliation; Subscriptions are separate grant-checked transport establishments without Operation lifecycle.
- Elicitation streams are subscription streams: all authorized subscribed surfaces for the expected operator actor receive the Elicitation, and the first valid answer clears it everywhere.
- Observation streams are optimizations; snapshots repair missed events.
- Presence is a derived fact, not a query target. One-shot "is session X present?" reads route through snapshot/status `query` Operations under the uniform read lifecycle; there is no distinct `query-presence` OperationKind.
- Single-operator v0 has no separate presence-leak threat inside the operator's authority domain. Filter-scoped subscriptions for multi-operator presence-leak prevention are a reserved seam; v0 must not bake in a hard-to-retract rule that all presence is globally public.
- Push notifications are an attention-routing surface, not authority. Claude Remote Control action-required notifications `[claude-code-remote-control]{10}`, Cursor SSE `[cursor-cloud-agents-api]{8}`, OpenCode SSE `[opencode-schema-events]`, and Pi pull-style `session_sync`/mesh presence `[pi-extension]` all fit this split.

## 9. Verification obligations and checked-vs-draft classification

Patchbay must be honest: prose may introduce O/O/E vocabulary now, but invariants are not checked until models/vectors are promoted. The design must not imply new checked properties exist.

### Existing checked properties unaffected

These remain checked-normative as currently documented and should not regress:

- `CommandDurability`, `BoundaryDedup`, `TerminalFinality`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` in `command_lifecycle.qnt`.
- `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, `GenerationMonotonic`, `LateGenerationInert` in `session_generation.qnt`.
- current `TypedCorrelation` for Reply → Command/Message in `reply_correlation.qnt`.
- `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`, `RevokedSessionCannotCommand` in `csrf_browser.qnt`.
- `ActorIdsUnique` in `patchbay-relational.als`.

D1/D4 do **not** require checked-property changes in v0. Command/message ids stay client-generated in the operator domain, because v0 has no non-operator Operation senders. `CompoundIssuer` stays operator-session-shaped: the web server/CLI endpoint is verified as a transport principal and the operator actor is independently verified against operator-session evidence. Elicitation openers inherit the adapter's authenticated channel, consistent with the existing adapter registration/session channel posture.

### `OperationState` ⇿ `CommandState` refinement mapping

`OperationState` is not a new checked model in this design. It reuses `CommandState` by documented equivalence:

| Operation vocabulary | Existing checked artifact |
|---|---|
| `Operation` accepted lifecycle record | `Command` record in `command_lifecycle.qnt` |
| `OperationKind` | `CommandKind` / `CommandKindRequest` registry concept |
| `OperationState` | `CommandState` exactly: `accepted`, `delivered`, `running`, `completed`, `rejected`, `failed`, `expired`, `cancelled`, `superseded` |
| terminal finality | existing `TerminalFinality` |
| first durable terminal commit | existing `PreAppendTerminalChoice` + `LsnDeterminesTerminalWinner` |
| idempotent retry | existing `BoundaryDedup`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` |

Classification: **checked-normative by refinement only** for Operations whose lifecycle semantics are exactly the existing Command lifecycle. Read/query Operations use the same lifecycle in v0; they may skip `running`, but the design does not claim a direct-to-completed fast path that contradicts the committed transition registry. A future no-lifecycle read optimization is a reserved seam and would require its own registry/model decision. A future rename from `CommandState` to `OperationState` must update model names, property metadata, `.proto`, conformance vectors, and docs together.

### New Elicitation model obligations

`ElicitationState` is **new stated-normative** until promoted. Elicitation ids are adapter-assigned in v0; the core assigns only the durable LSN at record time. The core does not open Elicitations in v0, so no core-opened-Elicitation property is reserved. Reserve these property ids:

- `ElicitationPendingFinality` — once an Elicitation reaches a terminal state, later answer/cancel/expire/withdraw/stale candidates do not mutate it.
- `ElicitationFirstAnswerWins` — for single-answer contracts, the first durably committed valid answer/decline terminal wins.
- `ElicitationCorrelationTyped` — response Operations reference a known ElicitationId in the same authority/session/responder context and cannot forge across id spaces or generations.
- `ElicitationTimeoutNeitherSuccessNorDenial` — timeout terminalizes as `expired`; timeout never implies answer, denial, or grant.
- `ElicitationInvalidResponseRejected` — invalid response Operations are rejected and do not satisfy the Elicitation unless explicit terminal-on-invalid policy is modeled.
- `ElicitationStaleTargetInert` — responses to stale/superseded target/session generations do not mutate live state.
- `ElicitationWithdrawalFinality` — opener withdrawal terminalizes without allowing later response mutation.

Required artifacts before claiming product semantics checked: promoted Quint/TLA+ model, finite bounds, tool invocation, expected pass/fail status, promoted conformance vector for each checked-normative property, and `.proto` fields traced when contracts exist.

### `TypedCorrelation` extension

Current `reply_correlation.qnt` checks Reply → Command/Message only. Extend it to include:

- `Operation(kind=approval-response|elicitation-response) → ElicitationId` typed correlation;
- same authority domain;
- same target/session/generation context or explicit stale rejection;
- expected responder actor policy in v0, with responding endpoint captured in the response Operation audit;
- no cross-id-space masquerade: CommandId, MessageId, ReplyId, EventId, and ElicitationId remain disjoint;
- duplicate response Operation behavior: idempotent return of existing response state or visible rejection, per policy.

Classification: **new stated-normative** until promoted.

### `authority.qnt` promotion requirements

`authority.qnt` is draft/stated-normative today. The O/O/E vocabulary and spawn behavior cannot ship grant-sensitive behavior as checked until authority is promoted.

Required properties to promote or add for v0:

- Existing reserved `NoCommandWithoutGrant` generalized by documented refinement to `NoOperationWithoutGrant` for grant-requiring committed OperationKinds.
- Existing `CompoundIssuer` retained in its operator-session shape: verified web-server/CLI transport principal plus independently verified operator actor; payload `sender` is not authority.
- Existing `GrantAuthorityIsCommandKinds` generalized by vocabulary rename to `GrantAuthorityIsOperationKinds`: grants are expressed over canonical OperationKinds, not adapter capability declarations.
- Existing `RevocationPreventsFuture` over Operation acceptance after grant/endpoint/session revocation.
- New `FleetAuthorityForSpawn`: spawn Operations targeting a not-yet-existing session require a live grant over fleet/supervisor/project/session-group scope, not a per-session target grant.
- New `SpawnCreatesDescendantGrant`: successful spawn completion records an explicit, auditable descendant grant whose subject is the spawner/operator and whose target is the spawned session.
- New `SpawnRevocationDoesNotCascade`: revoking a spawn grant prevents future spawns but does not revoke already-created descendant grants unless those grants are separately revoked.
- New `ElicitationResponderAuthority`: a response Operation is accepted only from an authenticated endpoint for the expected responder actor in v0; the responding endpoint is audited but not pre-bound in the Elicitation.

Reserved future authority properties (not v0 obligations): actor-neutral/non-operator Operation sender verification, agent/service grant subjects for authority-bearing Operations, tighter Elicitation responder binding by endpoint/endpoint class/fallback chain, and cross-actor delegation through `parent_grant_id`. The actor-neutral vocabulary remains the seam, but v0 checked properties must not pretend non-operator authority-bearing Operations exist.

Classification: **stated-normative until promoted**. This design must not say these are checked.

### Subscription authority obligations

Subscriptions are grant-checked without `OperationState` lifecycle. Reserve these property ids as stated-normative until a subscription/audit model exists:

- `SubscriptionGrantChecked` — a subscription establishment succeeds only when the actor/session has a live grant for the subscribed stream/filter scope.
- `SubscriptionAudited` — subscription allow/deny decisions create security audit records without creating Operation records.
- `SubscriptionCursorReplayAuthorized` — reconnect replay returns only events with `LSN > cursor` within the authorized subscription filter.

### Conformance vector obligations

Reserve vector families:

- `operation-query-uniform-lifecycle`: query/read uses the normal Operation lifecycle (for example accepted, then delivered, then completed), not a direct-to-completed fast path.
- `operation-read-no-lifecycle-reserved`: no-lifecycle reads are rejected/unavailable in v0 unless promoted by registry update.
- `agent-send-reserved-validation`: `agent-send` submission rejects with `validation_failed` in v0.
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
- `subscription-cursor-replay-authorized`: reconnect replay by cursor returns only events within the authorized subscription filter.

## 10. Normative inventory and inheritance direction

The inventory is **normative**.

Inheritance direction:

1. This feature defines the design of the normative action registry.
2. Implementation rolls it into `docs/PROTOCOL.md` as the product-vocabulary authority.
3. `feature-protocol-idl-and-conformance` derives `.proto` enum/wire encodings from that registry.
4. `.proto` is authority for wire shape and enum encoding only, not for inventing product-vocabulary variants.
5. Formal models own invariants; conformance vectors own expected executable examples.

The contract feature's Q4 (what are the command/action kinds?) dissolves: it consumes `OperationKind`, `ElicitationState`, `response_contract.contract_kind`, and Presence/Subscription registries from this foundation work.

## 11. Vocabulary application

### Glossary carve: Patchbay Command vs harness slash-command

For v0, do not rename every checked artifact. Instead:

- **Patchbay Command** remains the checked lifecycle registry name where formal models already use it.
- **Operation** is the actor-neutral protocol vocabulary that maps to that lifecycle by refinement.
- **Harness slash-command** is payload content interpreted by a harness (`/compact`, `/model`, `/agile-workflow:review`, Aider `/run`, shell `!`, etc.). It is not a Patchbay Command and does not bypass Operation authority.

Exact doc changes later:

- In `docs/GLOSSARY.md`, replace `## Command` body:
  > Operator intent that may cause action. Commands require target identity, authority, validation, and idempotency semantics.

  with:
  > A Patchbay lifecycle record for an accepted authorized request, currently used by the checked `CommandState` formal model. The actor-neutral protocol vocabulary is Operation; `CommandState` remains the checked lifecycle registry until a coordinated rename. A harness slash-command is different: it is payload text interpreted by a harness and has no Patchbay authority by itself.

- Add glossary entries for `Operation`, `Observation`, `Elicitation`, `Payload`, `OperationKind`, `ElicitationId`, and `Response contract`.
- Add a `Harness slash-command` entry:
  > Text such as `/compact`, `/model`, `/review`, or `!cmd` carried inside an Operation payload and interpreted by a harness. It is not a Patchbay protocol kind.

### Prompt-as-payload

Exact doc changes later:

- In `docs/UX.md` `### Send intent`, replace:
  > The operator can send a prompt, command, approval, cancel, or other adapter-supported action to a selected session.

  with:
  > The operator can submit Operations to a selected target: spawn or attach where supported, instruct a turn with prompt payload, cancel or interrupt active work, answer approvals or Elicitations, query status/snapshots, reconfigure adapter-declared settings, or perform session-management actions.

- In `docs/SPEC.md` V0 includes bullet `Initial command kinds`, replace:
  > send message/prompt, cancel or interrupt where the adapter supports it, request status/snapshot refresh, and receive correlated replies/events.

  with:
  > initial `OperationKind` registry: committed `spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, and `session-management`, plus reserved `agent-send` and `adapter-utility-exec` (rejected with `validation_failed` in v0); prompt text, slash-commands, images, and structured user input are payloads carried by `instruct` or response Operations. Observations carry output/events/status; they are not command kinds.

### Message-drop

Operator-originated no-grant `Message` drops for v0. Agent-originated replyable questions/requests are Elicitations.

Exact doc changes later:

- In `docs/PROTOCOL.md`, replace section heading `## Messages, commands, and replies` with `## Operations, Observations, Elicitations, payloads, and correlation`.
- Replace current `### Message` prose:
  > A message carries information. It may ask for a reply but does not itself grant authority to act.

  with:
  > Generic operator-originated no-grant `Message` is not a v0 action. Operator-originated content that drives work is payload of an authorized `instruct` Operation. Agent/harness/service-originated requests for a response are durable Elicitations. The `message id` space remains reserved for future informational surfaces and for current correlation-model compatibility.

- Replace current `### Reply` prose with wording that reply-like Observations and response Operations use typed correlation references; response Operations to Elicitations are a new typed-correlation case.

## 12. Implementation scope for the follow-on implement step

The implement step edits foundation docs only; it should not implement Rust/TypeScript code yet.

### `docs/VISION.md`

- Update opening thesis to network-reachable fixed point.
- Replace message/command wording with Operation/Observation/Elicitation where it describes product intent.
- Update success criteria for accepted Operations, Elicitations, and typed correlation.
- Preserve Pi-as-first-adapter, not architecture.

### `docs/ARCHITECTURE.md`

- Rename/operator-intent plane to operation plane.
- Replace message/command plane with Operation/Observation/Elicitation plane.
- Update V0 process topology split-deployment bullet to label colocation as deployment convenience.
- Update data flow for Operations, Observations, Elicitations, and subscriptions.
- Add fleet/supervisor target note for spawn and the settled one-`spawn`/`target_spec.shape` taxonomy.

### `docs/PROTOCOL.md`

- Replace `Messages, commands, and replies` with O/O/E definitions.
- Add/replace id-space registry with five spaces including ElicitationId; keep message id reserved.
- Add `OperationKind` registry table with committed/reserved dispositions, including reserved `agent-send` and `adapter-utility-exec`; keep `spawn` as one kind and put spawn variants in payload `target_spec.shape`.
- Document `CommandState` refinement equivalence for Operation lifecycle, including that reads use the uniform lifecycle in v0 and no-lifecycle reads are only a reserved optimization seam.
- Document that `target_spec.shape` is a reserved open registry named for spawn vocabulary/audit/display, not protocol-layer validation in v0; adapters declare supported shapes and reject unsupported shapes at delivery with `unsupported_command`.
- Add `ElicitationState` lifecycle table and response-validation rules, including expected-responder actor binding, first-answer-wins clearing across all subscribed surfaces, and responding-endpoint audit.
- Add `response_contract` registry.
- Add Presence/Subscription section/registry, including Elicitation stream fan-out through grant-checked subscriptions rather than direct per-Elicitation endpoint addressing.
- Update adapter capabilities from `command kinds` to `OperationKind`s while preserving capability-not-authority rule.
- Update failure vocabulary if needed for invalid elicitation response/stale elicitation references without inventing checked properties.

### `docs/VERIFICATION.md`

- Add an honest section: Operation vocabulary currently refines checked `CommandState`; no new Operation model claim.
- Add Elicitation model obligation and reserved property ids listed above; state that ElicitationIds are adapter-assigned and the core does not open Elicitations in v0.
- Add `TypedCorrelation` extension obligation for response Operation → Elicitation.
- Add authority promotion requirements: fleet authority for spawn, auto-issued descendant grant records, two-lever spawn/descendant revocation, and retained operator-session-shaped `CompoundIssuer`.
- Add subscription authority obligations: `SubscriptionGrantChecked`, `SubscriptionAudited`, and `SubscriptionCursorReplayAuthorized`.
- Update seed model table/classification without falsely promoting new properties.
- Add conformance-vector reservation rows.

### `docs/SPEC.md`

- Update `Initial command kinds` to the `OperationKind` registry, with one `spawn` kind and `target_spec.shape` for spawn vocabulary.
- Update Core concepts list with Operation, Observation, Elicitation, Payload; retain Command as checked lifecycle legacy/refinement term if needed.
- Update adapter posture to report Operation capabilities, Observations, Elicitations, snapshots, and presence/subscription facts.
- Update v0 exclusions only if they conflict with spawn being committed v0; be precise that HA/multi-core remains excluded while fleet-level spawn authority is in scope.

### `docs/UX.md`

- Update `Send intent` to submit Operations and answer Elicitations.
- Add attention-required/expected-responder language: v0 Elicitations target the operator actor, fan out to all subscribed surfaces, and clear everywhere on first answer.
- Update presentation states to include pending Elicitations and Observations without treating streams as authoritative.
- Keep mobile-first expectations; do not add native mobile scope.

### `docs/SECURITY.md`

- Replace command-only authorization language with Operation authorization where appropriate.
- Add fleet-authority note for spawn: spawn target may be a supervisor/fleet/project scope rather than an existing session.
- Add responder authorization for Elicitations: expected responder is the operator actor in v0; any authenticated operator endpoint may answer; the responding endpoint is captured in audit.
- Preserve CSRF/state-changing request requirements; response Operations and spawn are state-changing.
- Add secret response-contract redaction/no-log obligations.

### `docs/GLOSSARY.md`

- Add Operation, Observation, Elicitation, Payload, OperationKind, ElicitationId, Response contract, Harness slash-command.
- Rewrite Command to distinguish Patchbay checked lifecycle term from harness slash-command payload.
- Mark generic operator-originated Message as not a v0 action while keeping the id-space term reserved.

### `.work` relationship updates

- Update `feature-protocol-idl-and-conformance` body to state it inherits the normative registry from this feature and its Q4 is dissolved.
- Do **not** advance this feature to implementing until adversarial review accepts the design.

## 13. Open questions for adversarial review

N1–N3 are resolved and must not be re-opened: v0 Elicitations bind to the operator actor and deliver by subscription fan-out; `spawn` is one OperationKind with payload `target_spec.shape`; `SubscriptionCursorReplayAuthorized` joins the verification checklist.

1. **Presence responder notification policy:** Given that every authorized subscribed surface receives the Elicitation stream, how should attention-required state choose which surface(s) get extra push/badge/foreground notification in one-human v0? This is notification/attention routing only; it does not change the settled subscription fan-out delivery, Elicitation responder binding, or first-answer-wins semantics.

2. **Service requests under Elicitation naming:** Codex `currentTime/read`, auth refresh, and attestation requests fit actor-neutral Elicitation mechanically but are reserved in v0; the word may mislead humans toward only UI questions. Should docs use a broader display label such as `PendingResponse` while keeping `Elicitation` as model vocabulary, or is `response_contract.contract_kind=service_request` enough when that seam is promoted?

## Risks / pre-mortem

- **Verification overclaim risk:** The largest failure mode is prose making O/O/E sound checked before Elicitation and authority models are promoted. The implementation must mark every new property as stated-normative until models/vectors pass.
- **Registry bloat risk:** A too-wide `response_contract` registry could overfit non-Pi adapters. Mitigation: registry owns names; adapter capabilities and generated schemas gate concrete payload use.
- **Spawn authority risk:** Spawn in v0 forces grant semantics for targets that do not exist yet. If fleet authority is underspecified, the security model will accidentally assume per-session grants and reject real spawn or over-authorize supervisors.
- **Presence blind spot risk:** Even with subscription fan-out, Elicitations can be surfaced poorly — e.g., subscribed surfaces receive the stream event but no foreground attention route makes the pending approval obvious to the operator.
