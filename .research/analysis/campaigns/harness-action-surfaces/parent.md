---
provenance: agent-synthesis
updated: 2026-07-04
campaign: harness-action-surfaces
supersedes: null
---

# Harness action surfaces — cross-harness synthesis

This brief synthesizes the operator/agent/harness action surface across seven harnesses, grounded in attested sources (Pi via remote_pi's `pi-extension`; the other six via per-harness specialist briefs under `specialists/`). It is consumed by `feature-operator-presence-and-action-inventory`'s design pass to derive the action inventory and classification.

## Harnesses surveyed

| Harness | Surface shape | Specialist brief |
|---|---|---|
| Pi | extension-hook (pi.on events + ClientMessage control) | (grounded via remote_pi pi-extension source, in commissioning item) |
| Claude Code | CLI + Python/TS Agent SDK + Remote Control + Agent view | `specialists/claude-code.md` |
| Codex | `codex app-server` JSON-RPC + Python/TS SDK | `specialists/codex.md` |
| Cursor | IDE GUI + Cloud Agents REST/SSE + narrow VS Code extension API | `specialists/cursor.md` |
| OpenCode | typed HTTP API (effect/HttpApi) + SSE event bus | `specialists/opencode.md` |
| Aider | interactive CLI + slash commands + scripting | `specialists/aider.md` |
| Antigravity | Python SDK lifecycle hooks + managed Interactions API | `specialists/antigravity.md` |

## Cross-harness common action set (the harness-agnostic core)

Every surveyed harness exposes the following action classes. The commonality is strong enough to support a normative action registry (informs the consuming feature's D4 decision). The six-class spine aligns with the consuming feature: **drive / request / query / result / payload / provision**; `interrupt`, `approve`/`answer`, session-management (revert/compact/share/move), and reconfigure (model/agent switch) all map under `Request` (lifecycle-acting on a drive-action or session state).

The tables below are a derived summary grounded in the per-harness attestations under `.research/attestation/` (cited per row in the Drive table; the Cancel/Approve/Query/Result/Payload tables reuse the same per-harness attestation handles established there). Specialist briefs under `specialists/` are the analytical lens that organized the survey; the attestations are the citation substrate.

### Drive (lifecycle-bearing operator intent)

All seven expose a drive action: send content to the agent that begins a turn/generation with a lifecycle.

| Harness | Drive verbs |
|---|---|
| Pi | `user_message` (ClientMessage) `[pi-extension]` |
| Claude Code | `query()` / `ClaudeSDKClient.query()` (SDK) `[claude-code-sdk-client]{5}`, `claude` / `claude "query"` / `claude -p` (CLI) `[claude-code-cli]{1}`, Remote Control message `[claude-code-remote-control]{5}` |
| Codex | `turn/start` (app-server) `[codex-appserver-readme]{5}`, `thread run` / `thread turn` (Python SDK) `[codex-python-sdk-api]{2}` |
| Cursor | Agent sidepane prompt `[cursor-agent-overview]{1}`, `POST /v1/agents` + run enqueue (Cloud Agents) `[cursor-cloud-agents-api]{4}` |
| OpenCode | `prompt`, `promptAsync`, `shell`, `loop` `[opencode-session-handler]` |
| Aider | chat message (non-`/` input), `--message`/`--message-file` (one-shot) `[aider-base-coder]` `[aider-args]` |
| Antigravity | `Agent.chat()` / `Conversation.send()` / `Conversation.chat()` `[antigravity-sdk-repo]{2}` |

**Common shape:** content + target session/thread + (optional) config overrides. Lifecycle-bearing (busy/idle or turn-started/completed). This is the universal "operator drives agent" action.

### Interrupt / cancel (lifecycle-acting request)

All seven expose cancellation of an in-flight turn. The structural shape is consistent: it acts *on* a drive-action's lifecycle, racing with completion toward terminal.

| Harness | Cancel verbs |
|---|---|
| Pi | `cancel` (ClientMessage) `[pi-extension]` |
| Claude Code | `interrupt()` (SDK streaming) `[claude-code-sdk-client]{6}`, `interrupt`/`stop_task` control request `[claude-code-sdk-types]{11}`, `Ctrl+C`/`Ctrl+X`/`claude stop`/`claude kill` `[claude-code-agent-view]{6}` |
| Codex | `turn/interrupt` (by thread+turn id) `[codex-appserver-readme]{5}` |
| Cursor | cancel an active run (Cloud Agents) `[cursor-cloud-agents-api]`, immediate-message path (local) `[cursor-agent-overview]{5}` |
| OpenCode | `remove` session → `promptSvc.cancel`; `removeMessage` `[opencode-session-handler]` |
| Aider | `Ctrl+C` (double-press to exit) `[aider-io]` |
| Antigravity | `ChatResponse.cancel()` / `Conversation.cancel()` / `LocalConnection.cancel()` (sends `halt_request`) `[antigravity-sdk-repo]{2}` |

**Common shape:** targets an in-flight drive-action's turn; is a terminal candidate. Codex's `turn/steer` (add input to in-flight turn, non-canceling) is a variant — "redirect without cancel" — worth noting as a refinement some harnesses expose.

### Approve / answer (lifecycle-acting request — gate decision)

All seven expose a tool-approval or question-answer surface where the agent gates on operator input.

| Harness | Approval/answer verbs |
|---|---|
| Pi | `approve_tool` (ClientMessage) `[pi-extension]` |
| Claude Code | permission hooks (allow/deny) `[claude-code-hooks]`, structured clarifying-question replies `[claude-code-user-input]` |
| Codex | command-execution approval, file-change approval, tool user-input request `[codex-appserver-protocol]{7}` `[codex-appserver-types]{8}` |
| Cursor | tool approval (Run Modes dependent) `[cursor-run-modes]`, MCP tool approval `[cursor-mcp-extension]` |
| OpenCode | `permission.v2.replied` (`once`/`always`/`reject`), `question.replied`/`question.rejected` `[opencode-schema-events]` |
| Aider | `--yes`/`--yes-always`/`confirm_ask()` edit-approval flags, in-chat confirmation (attested flags only) `[aider-args]` |
| Antigravity | `PreToolCallDecideHook` (approve/deny), `tool_confirmation` InputEvent, `question_response` InputEvent `[antigravity-sdk-repo]{2}` |

**Common shape:** agent requests permission/answer → operator replies with a decision. This is a gate-resolution action, structurally distinct from drive (no content payload, no separate drive lifecycle — it unblocks an existing drive-action).

### Query / observe (read-only)

All seven expose read/state actions with no lifecycle effect.

| Harness | Query verbs |
|---|---|
| Pi | `session_sync`, `list_models` |
| Claude Code | session list/read, model list, remote-control status read |
| Codex | `thread/list`, `thread/read`, `model/list`, `modelProvider/capabilities/read` |
| Cursor | list/read agents and runs (Cloud Agents) |
| OpenCode | `status`, `summary.diff` |
| Aider | `/tokens`, `/ls`, `/map`, `/settings`, `/help` |
| Antigravity | poll stored background runs, conversation_id/history read |

**Common shape:** read state, no mutation, no lifecycle.

### Result (agent→operator output — not an operator action)

All seven surface agent output that correlates back to a prior drive-action.

| Harness | Result types |
|---|---|
| Pi | `agent_chunk`, `agent_message`, `agent_done`, `tool_request`, `tool_result`, `compaction` |
| Claude Code | message stream (assistant messages, tool_use, tool_result), task notifications |
| Codex | `item/started`→deltas→`item/completed`; `userMessage`/`agentMessage`/`reasoning`/`commandExecution`/`fileChange` items; `turn/started`/`turn/completed` |
| Cursor | run stream (SSE), agent output |
| OpenCode | `session.next.*` events, `permission.v2.*` events, `question.*` events |
| Aider | local output channels + analytics events (`message_send_*`, `command_*`) |
| Antigravity | `ChatResponse` stream, `agentMessage` items, tool-result events |

**Common shape:** agent→operator, correlates to a prior drive-action. Not an operator action.

### Payload (content the agent/harness interprets)

All seven carry content inside drive-actions that the harness/agent parses. Slash-commands are one example (present in Pi, Claude Code, Aider); other harnesses carry typed input payloads (Codex `UserInput`, OpenCode prompt text, Antigravity `user_input`/`complex_user_input`, Cursor prompts). The common shape is content the harness interprets, not a universal slash-command form.

| Harness | Payload examples |
|---|---|
| Pi | slash-command text (carried by `user_message`) |
| Claude Code | slash-command text (dispatched by the harness if non-interactive-compatible) |
| Codex | `UserInput` text/image/skill/mention entries (carried by `turn/start`) |
| Cursor | prompt text |
| OpenCode | prompt text (carried by `prompt`/`promptAsync`/`shell`) |
| Aider | `/`-prefixed slash commands, `!` shell passthrough (distinct from chat messages) |
| Antigravity | `user_input` / multimodal `complex_user_input` content |

**Common shape:** content carried by a drive-action; the harness interprets it (slash-commands, shell passthrough). Patchbay carries it; it doesn't interpret it.

## Provisioning — the divergent action class

This is the most interesting cross-harness finding, and it directly informs the consuming feature's D3 (provision as a first-class action class).

### Provisioning postures diverge across harnesses

This is the most interesting cross-harness finding, and it directly informs the consuming feature's D3 (provision as a first-class action class). Four postures surfaced:

**Posture 1 — out-of-band sysadmin:** Pi (`pi-supervisord`, systemd/launchd-managed, explicitly excluded from the setup wizard `[pi-extension]`), Aider (no instance spawn/retire in args/commands `[aider-args]` `[aider-commands]`), OpenCode (the surveyed session handler creates/removes sessions *within* a running server; spawning the server process itself is out-of-band `[opencode-session-handler]`). `{confidence: shallow}` for OpenCode — the `control-plane`/`installation` modules were not deeply surveyed and may carry more; flagged as a revisit-if.

**Posture 2 — programmatic local sidecar/session startup:** Antigravity's Python SDK starts a local harness process via the `Agent` async context manager (binary discovery, handshake over stdio/WebSocket) `[antigravity-sdk-repo]{2}`. This is local programmatic sidecar spawn by an SDK client, not arbitrary-remote-machine fleet spawn — but it is also not "operator starts the process themselves" in the sysadmin sense. Distinct from posture 1.

**Posture 3 — in-process session/thread creation:** Codex (`thread/start`/`thread/resume`/`thread/fork` create conversations within a running app-server `[codex-appserver-readme]{5}` `[codex-appserver-types]`; `process/spawn` is an experimental unsandboxed host-process spawn, not an agent instance `[codex-appserver-types]{10}`), Claude Code (Remote Control server-mode `--spawn same-dir|worktree|session` spawns sessions within an attached harness `[claude-code-remote-control]{3}` `[claude-code-remote-control]{1}` `[claude-code-remote-control]{7}` `[claude-code-remote-control]{11}`), OpenCode (`prompt`/`remove` create/remove sessions within the server).

**Posture 4 — cloud-managed agent creation:** Cursor Cloud Agents (`POST /v1/agents` creates a durable Cloud Agent on dedicated Cursor-managed machines `[cursor-cloud-agents-api]{4}` `[cursor-run-modes]{7}`), Antigravity managed Interactions API (`environment="remote"` provisions a Google-hosted sandbox `[antigravity-managed-agent]{3}`).

**Synthesis:** *No surveyed harness exposes "spawn a new agent/harness process on an arbitrary operator-controlled machine" as an operator action.* The closest are cloud-managed provisioning (posture 4) and in-process session creation (posture 3). The operator's actual pain — "spawn a fresh Pi on my VM from my phone without direct access" — is unserved by all seven. Patchbay provisioning this would be genuinely novel, not catching up to prior art.

## The "Message" question (no-grant informational replyable content)

Resolves the consuming feature's provisional vocabulary decision. **Two distinct questions must be separated** (the adversarial-read caught these being conflated):

**Q-A — generic operator-originated no-grant informational `Message` command:** *No surveyed harness exposes a distinct operator action that sends replyable informational content without a grant, separate from drive.* In every harness, operator-originated content that initiates a reply cycle is a drive-action (prompt with authority context). The closest candidates are Aider's split of `/`-commands from chat (but chat messages are drive-actions `[aider-base-coder]`) and Codex's thread-item manipulation primitives (state-mutation within a thread, not a replyable-message concept `{confidence: specialist-claim-not-attested}`). On this question, `{inferred: convergence}`: the operator-originated shape is universally drive, and `Message` (in the PROTOCOL sense of an operator-originated no-grant replyable type) can drop for v0.

**Q-B — agent-originated no-grant replyable question/elicitation paths:** *Several harnesses DO expose agent-initiated replyable surfaces that carry no operator grant.* These are not operator-originated Messages — they are the agent asking the operator (gate-style Request replies), but they are replyable content that is not itself a drive:
- Claude Code `AskUserQuestion` — a no-grant informational reply surface where the agent asks and the operator answers `[claude-code-user-input]{1}` `[claude-code-user-input]{4}` `[claude-code-user-input]{10}` `[claude-code-user-input]{11}`.
- Codex server-originated non-approval requests (`item/tool/requestUserInput`, MCP elicitation) `[codex-appserver-protocol]{7}` `[codex-appserver-types]{8}`.
- OpenCode `question.asked` / `question.replied` / `question.rejected` `[opencode-schema-events]`.
- Antigravity `ASK_QUESTION` builtin + `question_response` InputEvent `[antigravity-sdk-repo]{2}`.

**Implication for the consuming feature:** the PROTOCOL `Message` type (operator-originated, no-grant, replyable) is not exercised by any surveyed harness's operator surface — so dropping it for v0 loses nothing on the operator side. BUT the agent-originated question/elicitation surface (Q-B) is real and common across harnesses, and the consuming feature's design pass must decide whether patchbay models *that* (likely as a `Request` variant — the agent issues a question-request, the operator replies) rather than as an operator-originated `Message`. The provisional `Message`-drop decision holds for the operator-originated type; the agent-originated question surface is a separate modeling question the design pass inherits. The formal-model amendment to `TypedCorrelation` should be re-scoped: it narrows *operator-originated* correlation to the command (drive) space, but must still accommodate agent-originated question/elicitation replies as a typed reference target (they are replies/results, not operator messages).

## Contradictions

**`[medium]` Provisioning-scope divergence across harnesses.** The seven harnesses expose four provisioning postures (out-of-band sysadmin / programmatic local sidecar / in-process session creation / cloud-managed). This is not a contradiction *within* a source but a divergence *across* sources — the harness-agnostic action model must accommodate all four. The consuming feature's design pass must decide whether patchbay's Provision action class targets (a) only out-of-band-supervised spawn on an arbitrary operator machine (the operator's actual pain), (b) in-process session creation, (c) cloud-managed, or some combination.

**`[low]` Aider's `!`/`/run` shell passthrough.** Aider treats `!`-prefixed input as a shell command (`command_run`), distinct from chat — a harness-specific payload variant, not a contradiction. Noted as a payload-class refinement.

## Disconfirming analysis

Before claiming the six-class common set (drive/request/query/result/payload/provision) is universal, I searched across the specialist briefs for actions that *don't* fit:
- Codex `turn/steer` — adds input to an in-flight turn without canceling. This is a *refinement of drive* (drive into a running turn), not a new class. Fits under Drive.
- Codex `review/start` (automated reviewer) — a harness-specific automated action, not operator-initiated. Out of scope for the operator action inventory.
- OpenCode `revert`/`unrevert` — git-pair revert as a session action. This is a *session-management request* (lifecycle-acting on the session's git state), fits under Request.
- Cursor checkpoint restore — local lifecycle-ish control; fits under Request (acts on session state).
- Antigravity triggers (`automated_trigger` messages) — long-lived async tasks that send content into the agent. These are *automated drive* (non-operator-initiated), a distinct source but the same action shape as Drive.
- Aider `/undo`, `/commit`, `/save`, `/load` — session-state mutations; fit under Request (lifecycle-acting on session/git state).

No action surfaced that breaks the six-class spine. The classification survives the survey.

## Answers to the engagement's seed questions

1. **Operator→agent control actions across harnesses:** drive, interrupt, approve/answer, query, plus session-management requests (revert/compact/share/move/remove-message) and reconfigure (model/agent switch). Cataloged per-harness above.
2. **Agent→operator events:** message chunks, tool-call requests, tool results, turn/agent lifecycle, compaction, errors — universal across all seven.
3. **Durable vs ephemeral vs read-only:** drive = lifecycle-bearing; interrupt/approve/session-mgmt = lifecycle-acting requests; query = read-only; result = agent output; payload = content.
4. **Privileged sidecar/supervisor:** four postures — out-of-band sysadmin (Pi `pi-supervisord`, Aider, OpenCode); programmatic local sidecar (Antigravity SDK `Agent` context-manager starts a local harness process); in-process session creation (Codex threads, Claude Code Remote Control spawn modes); cloud-managed (Cursor Cloud Agents, Antigravity managed Interactions API). None expose "spawn on an arbitrary operator machine" as an operator action.
5. **No-grant informational replyable content:** no harness exposes a generic *operator-originated* no-grant `Message` command (Q-A); several expose *agent-originated* question/elicitation reply paths (Q-B), which the consuming feature must model separately (likely as a Request variant). The operator-originated `Message` type drops for v0; the agent-originated question surface is a separate modeling question.
6. **Provisioning mechanisms:** four postures (out-of-band sysadmin / programmatic local sidecar / in-process session creation / cloud-managed); no harness exposes remote-machine process spawn as an operator action. Patchbay pioneering this would be novel.

## Revisit if

- A harness not surveyed (Continue, OpenCode's `control-plane` module deep-survey, a future Codex/Claude version) exposes remote-machine process spawn as an operator action — would change Provision from "novel" to "prior art exists."
- The Claude Code Remote Control "spawn modes" finding (single-source) is disconfirmed by deeper docs — would narrow the provisioning finding.
- A harness surfaces genuine no-grant informational replyable content in a future version — would re-open the `Message` question.

## Acquisition candidates

- **`blocking`** — Antigravity `agy` CLI canonical docs (the specialist could not fetch CLI verbs/flags beyond the SPA shell; `antigravity-google` docs `/docs/cli/overview`). Completes: Antigravity CLI control semantics.
- **`blocking`** — Antigravity SDK Overview published docs + permissions docs. Completes: Antigravity SDK lifecycle-hook detail.
- **`enriching`** — Cursor Cloud Agents OpenAPI spec (`https://cursor.com/docs-static/cloud-agents-openapi.yaml`, source-bound via the Cloud Agents API docs). Would deepen the Cursor provisioning surface.
- **`enriching`** — Codex `codex app-server generate-json-schema --experimental` output from the exact Codex binary version under evaluation. Would ground the exact app-server schema.

These persist research-side; promotion to `.work/` is operator-confirmed at the research-handoff gate.
