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

> The operation plane represents authorized control-plane requests by any actor to any actor, core, adapter, fleet, or runtime target. It includes spawn, attach, drive, cancel, interrupt, query, approval response, elicitation response, reconfiguration, and session-management Operations.
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

In `## Data flow`, replace the numbered flow with one that says: a control surface, adapter, agent, or service submits an Operation; the core validates identity/authority/target/idempotency/kind/payload; accepted Operations are durably recorded before delivery; adapters and actors emit Observations; Elicitations open durable pending response slots; reconnecting surfaces reconcile by cursor and snapshot.

## 2. Normative action inventory

D4 resolves to **normative**: this inventory is the normative action registry. `feature-protocol-idl-and-conformance` inherits this registry and derives its `.proto` enum/wire representation from it; the contract feature does **not** invent a separate command-kind list. If `.proto` needs a new action kind, the product-vocabulary registry in `docs/PROTOCOL.md` changes first, then `.proto`, models, vectors, and implementation follow.

Disposition meanings:

- **Committed v0** — part of the v0 protocol/action registry. Adapter support may still be capability-declared, but the core recognizes the action kind and semantics.
- **Reserved seam** — named now to avoid foreclosure, but not required for first Pi-backed executable behavior unless a later feature promotes it.
- **Rejected for v0** — explicitly not a v0 protocol action.

| Action | Primitive | `OperationKind` | `{sender, recipient}` actor classes | What it does | V0 disposition | Surveyed evidence |
|---|---|---|---|---|---|---|
| Spawn runtime/session/fleet target | Operation | `spawn` | operator/control surface/service/agent → core/adapter/fleet supervisor | Creates a runtime session, process, thread, cloud agent, sidecar session, or harness instance that did not previously exist. Target may be fleet-level because the concrete session identity does not exist yet. | **Committed v0** as a registry kind and authority model obligation. Concrete Pi support may require an adapter-side supervisor, but spawn is not merely a reserved seam. | Pi supervisor is currently out-of-band `[pi-extension]`; Claude Remote Control `--spawn worktree|same-dir|session --capacity N` `[snc-rao-sp-cc-remote-control]{1}`; Dispatch `[snc-rao-sp-cc-desktop]{2}`; Codex `thread/start/resume/fork` and `process/spawn` `[codex-appserver-readme]{5}` `[codex-appserver-types]{10}`; Cursor `POST /v1/agents` `[cursor-cloud-agents-api]{4}`; OpenCode `serve` `[snc-rao-ae-opencode-cli]{3}`; Antigravity local `Agent` sidecar and managed `environment="remote"` `[antigravity-sdk-repo]{2}` `[antigravity-managed-agent]{3}`. |
| Attach / reconnect / reconcile | Operation | `attach` | operator/control surface/adapter → core/runtime session/adapter | Connects a surface or adapter endpoint to an existing session or server, authenticates, subscribes, and reconciles by cursor/snapshot/session generation. No work payload is implied. | **Committed v0.** Without attach/reconcile, remote operation and cross-device continuity fail. | Pi `pair_request` + `session_sync` `[pi-extension]`; Claude remote/mobile connection and sync `[claude-code-remote-control]{5}`; Codex app-server client subscriptions `[codex-appserver-readme]{6}`; Cursor Cloud SSE stream `[cursor-cloud-agents-api]{8}`; OpenCode client connect to `serve` `[opencode-session-handler]` `[snc-rao-ae-opencode-cli]{3}`; Antigravity `LocalConnection` handshake `[antigravity-sdk-repo]{2}`. |
| Drive a turn / prompt / user input | Operation | `drive` | operator/control surface/agent trigger/service → agent/runtime session | Sends content that begins a turn, generation, run, prompt loop, or user-input item. Prompt text and slash-commands are payload, not separate protocol primitives. | **Committed v0.** Core carries the Operation and lifecycle; adapter interprets payload. | Pi `user_message` `[pi-extension]`; Claude CLI/SDK/Remote Control query/message `[claude-code-cli]{1}` `[claude-code-sdk-client]{5}` `[claude-code-remote-control]{5}`; Codex `turn/start` `[codex-appserver-readme]{5}`; Cursor prompt/run creation `[cursor-agent-overview]{1}` `[cursor-cloud-agents-api]{6}`; OpenCode `prompt`/`promptAsync`/`shell`/`loop` `[opencode-session-handler]`; Aider chat/`--message` `[aider-base-coder]{3}` `[aider-args]{1}`; Antigravity `Agent.chat()` / `Conversation.send()` `[antigravity-sdk-repo]{2}`. |
| Steer in-flight work | Operation | `drive` | operator/control surface/agent trigger → running turn/session | Adds input or redirection to an in-flight turn without necessarily cancelling it. It is a drive refinement, not a new primitive. | **Committed v0** only as the `drive` kind when an adapter declares in-flight steering; unsupported adapters reject at delivery. | Codex `turn/steer` `[codex-appserver-readme]{5}` `[codex-appserver-types]{4}`; Cursor queued/immediate messages while Agent is working `[cursor-agent-overview]{5}`; Antigravity triggers can send automated content into an agent `[antigravity-sdk-repo]{2}`. |
| Cancel / interrupt active work | Operation | `cancel` or `interrupt` | operator/control surface/policy → active Operation/turn/session | Requests an in-flight drive/session action to stop. It races with completion under first durable terminal commit semantics; a late cancel does not rewrite a completed Operation. | **Committed v0.** Adapter capability determines delivery; core lifecycle semantics are fixed. | Pi `cancel` `[pi-extension]`; Claude `interrupt()` / `stop_task` / stop commands `[claude-code-sdk-client]{6}` `[claude-code-sdk-types]{11}` `[claude-code-agent-view]{6}`; Codex `turn/interrupt` `[codex-appserver-readme]{5}`; Cursor cancel run `[cursor-cloud-agents-api]{11}`; OpenCode remove/cancel `[opencode-session-handler]`; Aider Ctrl-C `[aider-base-coder]{7}`; Antigravity `cancel()` / `halt_request` `[antigravity-sdk-repo]{2}`. |
| Submit approval decision | Operation | `approval-response` | expected responder actor/control surface/service → opener agent/harness/core | Responds to a pending permission/tool approval Elicitation with allow/deny/allow-once/always/modified-input/policy-amendment where supported. | **Committed v0.** Tool approval is Pi-relevant and common; model promotion is required before product semantics are claimed checked. | Pi `approve_tool` `[pi-extension]`; Claude permission callbacks and hooks `[claude-code-user-input]{7}` `[claude-code-user-input]{8}` `[claude-code-hooks]{8}`; Codex command/file/permission approvals `[codex-appserver-readme]{8}` `[codex-appserver-types]{7}`; Cursor Run Mode/MCP approvals `[cursor-run-modes]{1}` `[cursor-mcp-extension]{8}`; OpenCode `permission.v2.replied` `[opencode-schema-events]`; Aider confirmation/`--yes-always` `[aider-args]{1}`; Antigravity `tool_confirmation` `[antigravity-sdk-repo]{2}`. |
| Submit elicitation answer/result | Operation | `elicitation-response` | expected responder actor/control surface/service/adapter → opener actor/harness/service | Responds to a non-approval Elicitation: answer a question, provide a secret, return a function/tool result, upload/provide an attachment, satisfy a service request, or decline/reject. | **Committed v0** for the registry and lifecycle. Specific `response_contract.contract_kind`s can be adapter-capability gated; invalid response Operations are rejected unless policy terminalizes the Elicitation. | Claude `AskUserQuestion` answers `[claude-code-user-input]{1}` `[claude-code-user-input]{10}` `[claude-code-user-input]{11}`; Codex `item/tool/requestUserInput`, MCP elicitation, auth-token refresh/current-time/attestation/dynamic tool requests `[codex-appserver-protocol]{7}` `[codex-appserver-types]{8}`; OpenCode `question.replied` / `question.rejected` `[opencode-schema-events]`; Antigravity `question_response`, custom function results, managed `requires_action` `[antigravity-sdk-repo]{2}` `[antigravity-managed-agent]{3}`. |
| Query / read / status / snapshot refresh | Operation | `query` | operator/control surface/adapter/service → core/adapter/session/resource | Reads session status, snapshots, model lists, thread/session lists, capability state, history, tokens/settings, or other state. Read-only does **not** mean grant-free. | **Committed v0.** Reads may use a per-kind transition subset (`accepted→completed`) without `running`. | Pi `session_sync`, `list_models`, `ping` `[pi-extension]`; Claude session/model/MCP status reads `[claude-code-sessions]{10}` `[claude-code-sdk-client]{10}`; Codex `thread/list`, `thread/read`, `model/list`, capabilities/read `[codex-appserver-readme]{5}`; Cursor list/read agents/runs `[cursor-cloud-agents-api]{5}` `[cursor-cloud-agents-api]{7}`; OpenCode `status`, `summary.diff` `[opencode-session-handler]`; Aider `/tokens`, `/ls`, `/map`, `/settings`, `/help` `[aider-commands]{3}`; Antigravity history/is_idle/usage/polling `[antigravity-sdk-repo]{2}` `[antigravity-managed-agent]{3}`. |
| Reconfigure runtime/session/model/policy/tools | Operation | `reconfigure` | operator/control surface/policy/service → core/adapter/session | Changes model, thinking/reasoning effort, permission mode, MCP/tool configuration, agent mode, environment/workspace settings, or other declared configuration. | **Committed v0** for registry; concrete knobs are adapter capabilities/payload schema. | Pi `model_set`, `thinking_set` `[pi-extension]`; Claude `set_model`, `set_permission_mode`, MCP toggle/reconnect, CLI/SDK model/permission flags `[claude-code-sdk-client]{7}` `[claude-code-sdk-client]{8}` `[claude-code-sdk-client]{10}`; Codex thread/turn settings update `[codex-appserver-types]{1}` `[codex-appserver-types]{2}`; Cursor Run Modes and MCP/plugin registration `[cursor-run-modes]{3}` `[cursor-extension-api]{1}`; OpenCode model/agent switched events and handlers `[opencode-schema-events]`; Aider `/model`, `/editor-model`, `/weak-model` `[aider-commands]{3}`; Antigravity capability/model/workspace/MCP/subagent config `[antigravity-sdk-repo]{2}`. |
| Session management / lifecycle mutation | Operation | `session-management` | operator/control surface/service → core/adapter/session/thread/agent resource | Resumes, forks, archives, deletes, compacts, clears, resets, removes messages, reverts, shares, checkpoints/restores, disconnects, retires, or stops sessions/resources after they exist. | **Committed v0** as registry family; each adapter declares supported sub-actions. Spawn remains separate because its target may not exist yet. | Pi `session_new`, `session_compact` `[pi-extension]`; Claude resume/fork/compact/clear/background stop/rm/respawn `[claude-code-sessions]{4}` `[claude-code-slash-commands]{3}` `[claude-code-agent-view]{9}`; Codex thread resume/fork/archive/delete/unarchive/compact `[codex-appserver-readme]{5}`; Cursor checkpoint restore, archive/unarchive/delete Cloud Agent `[cursor-agent-overview]{4}` `[cursor-cloud-agents-api]{12}` `[cursor-cloud-agents-api]{13}`; OpenCode remove/removeMessage/revert/unrevert/compact/share `[opencode-session-handler]`; Aider `/load`, `/save`, `/undo`, `/commit`, `/clear`, `/reset` `[aider-commands]{3}`; Antigravity resume/disconnect/clear history `[antigravity-sdk-repo]{2}`. |
| Receive output / lifecycle facts / results | Observation | n/a | agent/adapter/core/service → operator/control surface/core/subscriber | Emits source-authenticated output, chunks, assistant messages, tool calls/results, turn/session lifecycle, errors, status facts, compaction events, or delivery acknowledgements. Observations do not grant authority to act, but they require source identity, correlation context, and cursor/LSN treatment where durable. | **Committed v0.** Observation streams are not authoritative alone; snapshots/core records reconcile. | Pi `message_update`, `tool_call`, `tool_execution_*`, `agent_end` `[pi-extension]`; Claude assistant/tool/result/hook/task streams `[claude-code-sdk-types]{6}` `[claude-code-sdk-types]{7}` `[claude-code-sdk-types]{12}`; Codex notifications and item deltas `[codex-appserver-readme]{6}` `[codex-appserver-readme]{7}`; Cursor CLI NDJSON and Cloud SSE `[cursor-cli-output]{3}` `[cursor-cloud-agents-api]{8}`; OpenCode SSE events `[opencode-schema-events]`; Aider local output/streaming `[aider-io]{1}` `[aider-base-coder]{4}`; Antigravity chunks/steps/hooks/tool results `[antigravity-sdk-repo]{2}`. |
| Open approval elicitation | Elicitation | n/a | agent/harness/adapter/core → expected responder actor/control surface/service | Opens a durable pending response slot for permission/tool approval. Carries opener, recipient/expected responder, response contract, target context, timeout/cancellation/withdrawal policy, and correlation. | **Committed v0.** Approval gates are common and Pi-relevant. Requires new Elicitation model before claiming checked semantics. | Pi `tool_call` approval gate `[pi-extension]`; Claude tool permission requests `[claude-code-user-input]{1}` `[claude-code-user-input]{4}`; Codex command/file/permission approval requests `[codex-appserver-protocol]{6}` `[codex-appserver-types]{7}`; Cursor local tool/MCP approvals `[cursor-run-modes]{1}` `[cursor-mcp-extension]{8}`; OpenCode `permission.v2.asked` `[opencode-schema-events]`; Antigravity tool confirmation requests `[antigravity-sdk-repo]{2}`. |
| Open question/input/service elicitation | Elicitation | n/a | agent/harness/server/core/service → expected responder actor/control surface/adapter/service | Opens a durable pending response slot for a question, secret, user-input request, service request, function result, current-time/auth/attestation request, or structured response. Actor-neutral: recipient need not be the human operator. | **Committed v0** as protocol shape; individual contract kinds may be capability gated. This avoids losing common agent-originated question surfaces while preserving future agent→agent/service requests. | Claude `AskUserQuestion` `[claude-code-user-input]{1}` `[claude-code-user-input]{10}`; Codex user-input/MCP elicitation/auth-token/current-time/attestation/dynamic-tool requests `[codex-appserver-protocol]{7}` `[codex-appserver-types]{8}`; OpenCode `question.asked` `[opencode-schema-events]`; Antigravity `ASK_QUESTION`, function requests, triggers/service-style interactions `[antigravity-sdk-repo]{2}` `[antigravity-managed-agent]{3}`. |
| Carry interpreted content | Payload | n/a | any primitive's sender → recipient | Holds text, slash-command strings, images, mentions, skills, shell passthrough, schemas, function arguments/results, file/attachment references, or adapter-specific bodies. Patchbay validates envelopes/contracts; the harness interprets payload semantics. | **Committed v0** as carried content, not a standalone authority primitive. Operator-originated no-grant `Message` is rejected for v0; content that drives work is payload of `drive`. | Pi slash-command text in `user_message` `[pi-extension]`; Claude slash-command prompt strings `[claude-code-slash-commands]{1}`; Codex typed `UserInput` entries `[codex-appserver-types]{5}`; Cursor prompts `[cursor-agent-overview]{1}`; OpenCode prompt text `[opencode-session-handler]`; Aider `/` and `!` command payloads `[aider-commands]{4}`; Antigravity `user_input` / `complex_user_input` `[antigravity-sdk-repo]{2}`. |
| Generic operator-originated no-grant replyable message | Rejected v0 direction | n/a | operator/control surface → agent/session | A replyable informational message that is not an authorized drive/query/response Operation. | **Rejected for v0.** No surveyed harness exposes this as a distinct operator action. The id space remains reserved for compatibility and future evidence. | Campaign synthesis separates Q-A from Q-B: no generic operator-originated no-grant Message; agent-originated Elicitations are real. Concrete Q-B evidence: Claude `[claude-code-user-input]{1}` `[claude-code-user-input]{10}`; Codex `[codex-appserver-protocol]{7}` `[codex-appserver-types]{8}`; OpenCode `[opencode-schema-events]`; Antigravity `[antigravity-sdk-repo]{2}`. |

## 3. Actor-neutral primitive definitions (A1)

These definitions should become protocol prose.

### Operation

An **Operation** is an authorized control-plane request by any actor to any actor, core, adapter, fleet, session, service, or resource target. An Operation may be side-effecting, read-only, lifecycle-acting, response-submitting, or fleet-creating. Operations require verified sender identity, target identity/scope, authority evaluation, a registry-owned `OperationKind`, boundary validation, idempotency semantics where applicable, and durable lifecycle state after acceptance.

Initial implementation reuses `CommandState` and command ids by refinement equivalence. `Operation` is the actor-neutral vocabulary; `CommandState` is the checked lifecycle registry until the coordinated rename/model update occurs.

### Observation

An **Observation** is a source-authenticated fact, event, output, status emission, reply-like result, or lifecycle/status fact emitted by an actor, adapter, core, runtime, or service. Observations do not grant authority to act. They still require source identity, target/session/generation context where applicable, correlation context when they answer or relate to prior work, and LSN/cursor/snapshot reconciliation if durable.

Live streams are delivery optimizations. Durable core records and snapshots remain the authority for accepted Operations and reconciled state.

### Elicitation

An **Elicitation** is a durable pending response solicitation from one actor/system component to another. It opens a response slot rather than answering a prior request. It carries an `ElicitationId`, opener, expected responder/recipient, target/session/generation context, `response_contract`, timeout/cancellation/withdrawal policy, correlation to the work that caused it, and terminal lifecycle state.

Elicitation is actor-neutral: agent→operator questions, harness→client service requests, adapter→core requests, service→operator secret prompts, and future agent→agent/op→op solicitations use the same primitive. A response is submitted as an `OperationKind = elicitation-response` or `approval-response` Operation correlated to the Elicitation.

### Payload

A **Payload** is the adapter-specific content or schema-bound body carried inside an Operation, Observation, or Elicitation. Examples: prompt text, slash-command text, typed user input entries, tool-call arguments, function results, image/file references, question options, structured schemas, or adapter diagnostics. Payload does not itself grant authority, create lifecycle state, or define protocol kinds.

## 4. `OperationKind` registry (A5)

One registry owns kinds, transition subsets, authority matching, adapter capability mapping, display labels, and generated contract variants. Adding a kind requires updating `docs/PROTOCOL.md`, `.proto`, model/vectors as applicable, and implementation together.

Initial registry:

| `OperationKind` | Meaning | Allowed `CommandState` subset / transition notes | V0 disposition |
|---|---|---|---|
| `spawn` | Create a new runtime/session/thread/agent/process/cloud resource; target may be fleet/supervisor-level before a session exists. | `accepted → delivered? → running? → completed | rejected | failed | expired | cancelled | superseded`. `running` is allowed for long provisioning. | Committed v0. Requires fleet-authority modeling. |
| `attach` | Connect/reconnect a surface/adapter endpoint to an existing session/server and reconcile. | Usually `accepted → completed` or `accepted → delivered → completed`; `running` is not required. | Committed v0. |
| `drive` | Send prompt/user input/steering content into a session/turn. | Full lifecycle allowed: `accepted → delivered → running → terminal`; in-flight steering may skip `running` if adapter reports immediate acceptance. | Committed v0. |
| `cancel` | Request cancellation of a target Operation/turn/session action. | `accepted → delivered? → completed | rejected | failed | expired | superseded`. The target Operation's terminal race is governed by first durable terminal commit; the cancellation Operation completing does not rewrite an already-terminal target. | Committed v0. |
| `interrupt` | Request immediate stop/interrupt of active execution. | Same as `cancel`; reserved distinction for adapters that expose softer cancel vs harder interrupt. | Committed v0. |
| `query` | Read status, snapshot, capabilities, lists, history, metadata, or diagnostics. | May go `accepted → completed` without `delivered` or `running`; may also use `accepted → delivered → completed` when an adapter must answer. Read-only remains grant-checked. | Committed v0. |
| `approval-response` | Respond to a permission/tool approval Elicitation. | `accepted → completed | rejected | failed | expired | superseded`. Completion updates the Elicitation terminal (`answered` or `declined`) only if response validation succeeds and first-terminal rules allow. | Committed v0. |
| `elicitation-response` | Respond to non-approval Elicitations: question, secret, freeform, function result, service request, attachment, schema response. | Same as `approval-response`. Invalid response Operation is rejected unless explicit Elicitation policy terminalizes the slot. | Committed v0. |
| `reconfigure` | Change model, reasoning/thinking level, permission mode, tools/MCP, agent mode, workspace, or adapter config. | `accepted → delivered? → running? → completed | rejected | failed | expired | cancelled | superseded`; `running` only for adapters with long reconfiguration. | Committed v0. |
| `session-management` | Resume, fork, compact, clear, archive/delete, revert, share/unshare, remove messages, checkpoint restore, disconnect/retire existing sessions/resources. | Full lifecycle allowed because compaction/archive/delete can be long-running; quick local actions may skip `running`. | Committed v0. |

Boundary rule: unknown `OperationKind` is `SubmissionOutcome = rejected` with `validation_failed` before grant evaluation. Unsupported-by-adapter known kind is a delivery-layer `unsupported_command` rejection after acceptance, matching the existing capability posture.

## 5. `ElicitationState` lifecycle (A3)

`ElicitationState` is a new registry, not a projection of `CommandState`.

| State | Terminal? | Meaning |
|---|---:|---|
| `opened` | no | Core durably recorded the Elicitation, but it may not yet be visible/delivered to the expected responder/subscription. |
| `pending` | no | The Elicitation is visible to the expected responder(s) and can accept a valid response Operation. |
| `answered` | yes | A valid response Operation satisfied the contract and first durable terminal commit selected it as the answer. |
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
- First valid answer wins for single-answer contracts. Multi-answer contracts are reserved; they must define completion policy in `response_contract` before use.
- A response Operation must reference the `ElicitationId` with a typed correlation and must satisfy the active `response_contract`.
- Invalid response behavior: default is **reject the response Operation** (`SubmissionOutcome = rejected` before acceptance, or `OperationState = rejected` after acceptance by policy) and leave the Elicitation `pending`. A contract may explicitly specify terminal-on-invalid policy, but that policy must name the terminal outcome (`declined`, `superseded`, or `cancelled`) and be tested.
- No-answer is not an Operation. It is either continued `pending` or a terminal policy event such as `expired`, `cancelled`, `withdrawn`, or `stale`.
- `answered` does not imply the underlying tool/action succeeded; it only means the response slot was satisfied. Subsequent work emits Operations/Observations as usual.

Reserved future shapes:

- multi-responder quorum Elicitations;
- multi-answer accumulation;
- delegated responder policy;
- escalation from one expected responder to another;
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
- `responder_policy` — expected actor, endpoint class, or service role;
- `sensitivity` — whether raw response may be logged, redacted, encrypted, or never persisted in plaintext.

Initial `contract_kind` registry:

| `contract_kind` | Semantics | Evidence | V0 disposition |
|---|---|---|---|
| `approval` | Allow/deny/allow-once/always/policy-amend/modified-input permission response. | Claude modified input / remember permissions `[claude-code-user-input]{7}` `[claude-code-user-input]{8}`; Codex command/file approvals `[codex-appserver-types]{7}`; OpenCode `permission.v2.replied` `[opencode-schema-events]`; Pi `approve_tool` `[pi-extension]`. | Committed v0. |
| `question` | Answer one or more questions, possibly with options and freeform text. | Claude `AskUserQuestion` `[claude-code-user-input]{10}`; OpenCode `question.asked` `[opencode-schema-events]`; Antigravity `ASK_QUESTION` `[antigravity-sdk-repo]{2}`. | Committed v0. |
| `secret` | Provide sensitive secret/token/input with redaction/no-log policy. | Codex user-input question `is_secret` `[codex-appserver-types]{8}`; auth-token refresh request `[codex-appserver-protocol]{7}`. | Reserved seam unless a v0 adapter requires it; registry named now. |
| `freeform` | Unstructured text response. | Claude optional freeform answer `[claude-code-user-input]{10}`; drive-like user input surfaces across all harnesses. | Committed v0. |
| `function_result` | Return custom tool/function result to a waiting service/harness. | Antigravity managed `requires_action` / function result `[antigravity-managed-agent]{3}`; Codex dynamic tool requests `[codex-appserver-protocol]{7}`. | Reserved seam for non-Pi adapters. |
| `file_attachment` | Provide file/blob/image/attachment reference or upload. | Codex typed image/local image input `[codex-appserver-types]{5}`; Cursor/Antigravity image/file surfaces `[cursor-cloud-agents-api]{8}` `[antigravity-sdk-repo]{2}`. | Reserved seam. |
| `structured_schema` | Response must validate against declared JSON/protobuf/schema. | Antigravity finish schema and structured output surfaces `[antigravity-sdk-repo]{2}`; Codex output schema/typed requests `[codex-appserver-types]{3}` `[codex-appserver-types]{8}`. | Reserved seam until generated contract exists. |
| `service_request` | Non-human service response such as current time, attestation generation, auth refresh, or adapter-provided evidence. | Codex `currentTime/read`, `attestation/generate`, auth-token refresh `[codex-appserver-protocol]{7}`. | Reserved seam, but actor-neutral Elicitation must allow it. |

UI hints are non-authoritative. A `question` contract may render as select-one, select-many, free-text, voice input, or CLI prompt without changing protocol semantics. A `service_request` may render no UI at all.

## 7. Id spaces (Decision 3)

Patchbay uses five separate id spaces:

1. **Command id** — client/operator-domain generated today; identity for accepted lifecycle-bearing records. During the vocabulary transition, accepted Operations reuse this id space by refinement equivalence. A future `OperationId` rename is a coordinated artifact rename, not a sixth id space.
2. **Message id** — reserved in v0 even though generic operator-originated no-grant `Message` drops. It remains in the registry because current `TypedCorrelation` and future non-command informational surfaces may need it.
3. **Reply id** — adapter-or-core assigned for correlated reply/observation records that answer prior command/message/operation context.
4. **Event id** — core-assigned LSN, keyed as `(authority_domain_id, LSN)`.
5. **Elicitation id** — new id space, adapter-or-core assigned when a pending response slot is opened.

Forgery-prevention justification:

- A response Operation must not be able to masquerade as the Elicitation it answers. Separate `CommandId`/`ElicitationId` spaces preserve direction: Elicitation opens a pending slot; response Operation answers it.
- A reply id cannot masquerade as command identity; the checked `TypedCorrelation` principle already enforces separate id spaces for command/message/reply and same-context typed references.
- `ElicitationId` is not a typed `ReplyId` subkind because an Elicitation is an initiation, while a Reply is a response. Modeling initiation as response inverts semantic direction and confuses lifecycle ownership.
- The existing `reply_correlation.qnt` does **not** cover response Operation → Elicitation. Extending typed correlation is a new verification obligation.

## 8. Presence / Subscription protocol section (A4)

Presence/Subscription is a named protocol section/registry, **not** a fifth primitive. Operations and Observations carry presence facts; the registry defines how they are interpreted and reconciled.

The section must distinguish these axes:

| Axis | Meaning | V0 registry/fields |
|---|---|---|
| Endpoint availability | Is a concrete endpoint connection/address reachable? | Reuse/align with `SessionConnectivityState`: `live`, `stale`, `offline`, `unknown`, `failed`; fields: endpoint id, device id, adapter generation, last authoritative LSN. |
| Actor presence | Is an actor currently represented by at least one usable endpoint, and with what attention posture? | `available`, `away`, `unavailable`, `unknown`; derived from endpoint observations and policy, never authority by itself. |
| Observation subscription | Which actor/endpoint/control surface is subscribed to which event/snapshot stream? | `subscribed`, `resuming`, `unsubscribed`, `failed`; fields: subscription id, filter, cursor, last delivered LSN. |
| Attention-required state | Does a target require human/service attention? | `none`, `attention_requested`, `response_required`, `blocking`, `escalated`; source is Elicitation or adapter Observation. |
| Expected responder | Which actor/control surface/service role should answer an Elicitation? | Fields on Elicitation: `expected_responder_actor`, optional endpoint class/control-surface role, fallback/escalation policy, responder generation. |
| Stale-presence reconciliation | What happens after disconnect/reconnect or missed presence events? | Presence Observations carry LSN/revision; reconnecting clients submit cursor; stale presence cannot be rendered as live; Elicitations may terminalize `stale` if opener/target generation is superseded. |

Implementation notes for docs:

- Attach Operations establish or refresh endpoint availability and subscription state.
- Observation streams are optimizations; snapshots repair missed events.
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

Classification: **checked-normative by refinement only** for Operations whose lifecycle semantics are exactly the existing Command lifecycle. A future rename from `CommandState` to `OperationState` must update model names, property metadata, `.proto`, conformance vectors, and docs together.

### New Elicitation model obligations

`ElicitationState` is **new stated-normative** until promoted. Reserve these property ids:

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
- expected responder actor/endpoint policy;
- no cross-id-space masquerade: CommandId, MessageId, ReplyId, EventId, and ElicitationId remain disjoint;
- duplicate response Operation behavior: idempotent return of existing response state or visible rejection, per policy.

Classification: **new stated-normative** until promoted.

### `authority.qnt` promotion requirements

`authority.qnt` is draft/stated-normative today. The O/O/E frame broadens grant semantics and cannot ship grant-sensitive behavior as checked until authority is promoted.

Required properties to promote or add:

- Existing reserved `NoCommandWithoutGrant` generalized to `NoOperationWithoutGrant` for grant-requiring OperationKinds.
- Existing `CompoundIssuer` generalized to actor-neutral sender verification: verified connection/session/service evidence determines sender; payload `sender` is not authority.
- Existing `GrantAuthorityIsCommandKinds` generalized to `GrantAuthorityIsOperationKinds`: grants are expressed over canonical OperationKinds, not adapter capability declarations.
- Existing `RevocationPreventsFuture` over Operation acceptance after grant/endpoint/session revocation.
- New `FleetAuthorityForSpawn`: spawn Operations targeting a not-yet-existing session require a live grant over a fleet/supervisor/project/session-group scope, not a per-session target grant.
- New `ActorNeutralGrantSubjects`: grant subjects may be operator, control surface endpoint, adapter, service, or agent actor as explicitly modeled; v0 provisioning may restrict which subjects are issued, but the model must not assume human-only senders.
- New `ElicitationResponderAuthority`: a response Operation is accepted only from the expected responder actor/endpoint/service or an authorized fallback/escalation subject.

Classification: **stated-normative until promoted**. This design must not say these are checked.

### Conformance vector obligations

Reserve vector families:

- `operation-query-fast-path`: query goes `accepted→completed` without `running`.
- `spawn-fleet-authority`: spawn accepted with fleet grant; rejected with only per-session grant when target session does not exist.
- `elicitation-answer-first-wins`: two valid answers race; lower LSN wins.
- `elicitation-invalid-response`: invalid answer rejected and Elicitation remains pending by default.
- `elicitation-stale-generation`: answer after target generation tombstone records stale/audit and does not mutate live state.
- `operation-response-correlation-forgery`: response Operation using ReplyId/EventId/CommandId as ElicitationId rejected.

## 10. D4 resolution: normative inventory and inheritance direction

D4 resolves to **normative**.

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
  > The operator can submit Operations to a selected target: spawn or attach where supported, drive a turn with prompt payload, cancel or interrupt active work, answer approvals or Elicitations, query status/snapshots, reconfigure adapter-declared settings, or perform session-management actions.

- In `docs/SPEC.md` V0 includes bullet `Initial command kinds`, replace:
  > send message/prompt, cancel or interrupt where the adapter supports it, request status/snapshot refresh, and receive correlated replies/events.

  with:
  > initial `OperationKind` registry: `spawn`, `attach`, `drive`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, and `session-management`; prompt text, slash-commands, images, and structured user input are payloads carried by `drive` or response Operations. Observations carry output/events/status; they are not command kinds.

### Message-drop

Operator-originated no-grant `Message` drops for v0. Agent-originated replyable questions/requests are Elicitations.

Exact doc changes later:

- In `docs/PROTOCOL.md`, replace section heading `## Messages, commands, and replies` with `## Operations, Observations, Elicitations, payloads, and correlation`.
- Replace current `### Message` prose:
  > A message carries information. It may ask for a reply but does not itself grant authority to act.

  with:
  > Generic operator-originated no-grant `Message` is not a v0 action. Operator-originated content that drives work is payload of an authorized `drive` Operation. Agent/harness/service-originated requests for a response are durable Elicitations. The `message id` space remains reserved for future informational surfaces and for current correlation-model compatibility.

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
- Add fleet/supervisor target note for spawn.

### `docs/PROTOCOL.md`

- Replace `Messages, commands, and replies` with O/O/E definitions.
- Add/replace id-space registry with five spaces including ElicitationId; keep message id reserved.
- Add `OperationKind` registry table and per-kind transition subsets.
- Document `CommandState` refinement equivalence for Operation lifecycle.
- Add `ElicitationState` lifecycle table and response-validation rules.
- Add `response_contract` registry.
- Add Presence/Subscription section/registry.
- Update adapter capabilities from `command kinds` to `OperationKind`s while preserving capability-not-authority rule.
- Update failure vocabulary if needed for invalid elicitation response/stale elicitation references without inventing checked properties.

### `docs/VERIFICATION.md`

- Add an honest section: Operation vocabulary currently refines checked `CommandState`; no new Operation model claim.
- Add Elicitation model obligation and reserved property ids listed above.
- Add `TypedCorrelation` extension obligation for response Operation → Elicitation.
- Add authority promotion requirements: fleet authority for spawn and actor-neutral grant subjects.
- Update seed model table/classification without falsely promoting new properties.
- Add conformance-vector reservation rows.

### `docs/SPEC.md`

- Update `Initial command kinds` to the `OperationKind` registry.
- Update Core concepts list with Operation, Observation, Elicitation, Payload; retain Command as checked lifecycle legacy/refinement term if needed.
- Update adapter posture to report Operation capabilities, Observations, Elicitations, snapshots, and presence/subscription facts.
- Update v0 exclusions only if they conflict with spawn being committed v0; be precise that HA/multi-core remains excluded while fleet-level spawn authority is in scope.

### `docs/UX.md`

- Update `Send intent` to submit Operations and answer Elicitations.
- Add attention-required/expected-responder language.
- Update presentation states to include pending Elicitations and Observations without treating streams as authoritative.
- Keep mobile-first expectations; do not add native mobile scope.

### `docs/SECURITY.md`

- Replace command-only authorization language with Operation authorization where appropriate.
- Add fleet-authority note for spawn: spawn target may be a supervisor/fleet/project scope rather than an existing session.
- Add responder authorization for Elicitations.
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

1. **Spawn target taxonomy:** Spawn is locked into v0, but the exact target scopes need attack: `fleet`, `adapter supervisor`, `project/session group`, `host endpoint`, and `cloud provider resource` may need separate grant-scope variants. Is one `spawn` OperationKind with target-scope registry enough, or do cloud/in-process/process/thread spawns need distinct kinds?

2. **Committed `response_contract` subset:** Approval/question/freeform are clearly v0-relevant; secret/function_result/file_attachment/structured_schema/service_request are evidenced but may be non-Pi seams. Should the registry include all at v0 with capability gating, or should some remain named reserved variants outside generated contracts until a non-Pi adapter lands?

3. **Presence responder policy:** In one-human v0, should `expected_responder` bind to the operator actor, a specific operator session endpoint, a control-surface class, or a service role? Binding too narrowly breaks cross-device answering; binding too broadly may weaken audit and CSRF/session guarantees.

4. **Service requests under Elicitation naming:** Codex `currentTime/read`, auth refresh, and attestation requests fit actor-neutral Elicitation mechanically, but the word may mislead humans toward only UI questions. Should docs use a broader display label such as `PendingResponse` while keeping `Elicitation` as model vocabulary, or is `response_contract.contract_kind=service_request` enough?

## Risks / pre-mortem

- **Verification overclaim risk:** The largest failure mode is prose making O/O/E sound checked before Elicitation and authority models are promoted. The implementation must mark every new property as stated-normative until models/vectors pass.
- **Registry bloat risk:** A too-wide `response_contract` registry could overfit non-Pi adapters. Mitigation: registry owns names; adapter capabilities and generated schemas gate concrete payload use.
- **Spawn authority risk:** Spawn in v0 forces grant semantics for targets that do not exist yet. If fleet authority is underspecified, the security model will accidentally assume per-session grants and reject real spawn or over-authorize supervisors.
- **Presence blind spot risk:** Without a Presence/Subscription section, Elicitations can be correctly modeled but routed poorly — e.g., a pending approval exists but no control surface knows it is the expected responder.
