---
provenance: agent-synthesis
updated: 2026-07-04
campaign: harness-action-surfaces
supersedes: null
---

# Harness action surfaces — cross-harness synthesis

This brief synthesizes the operator/agent/harness action surface across seven harnesses, grounded in attested sources (Pi via remote_pi's `pi-extension`; the other six via per-harness specialist briefs under `specialists/`). It is consumed by `feature-operator-presence-and-action-inventory`'s design pass to derive the action inventory and classification.

## Methodology correction (reconciliation with prior research)

This engagement was initially scoped to patchbay's `.research/` substrate and missed the operator's broader prior research corpus at `/home/agent/projects/SNC/.research/` (855 attestations, 36+ synthesis briefs). On reconciliation, the operator's prior `remote-agent-operation-landscape.md` (2026-06-03) already establishes the **spawn-vs-pilot distinction** as the load-bearing cut in this space and documents Claude Code Remote Control `--spawn worktree --capacity N`, Dispatch, SSH+tmux, and OpenCode `serve`+`attach` as spawn mechanisms — with a deployed systemd-unit guide (`/home/agent/projects/SNC/docs/ops/remote-agent-piloting.md`). The initial patchbay synthesis wrongly concluded "no harness exposes remote-machine process spawn as an operator action"; that finding is **revised** below to correctly cite this prior art. Three cross-corpus pointer-attestations (`snc-rao-sp-cc-remote-control`, `snc-rao-ae-opencode-cli`, `snc-rao-landscape`) are added so the patchbay citation chain resolves to the SNC sources.

The operator further refined the vocabulary: "pilot" collapses two structurally distinct primitives (attach, then operate). OpenCode's `serve` (spawn) → client-connect+auth (attach) → `prompt`/`cancel` (operate) surface makes the distinction concrete. The revised spine below adopts **spawn / attach / operate / receive** + **payload**.

## Harnesses surveyed

| Harness | Surface shape | Specialist brief |
|---|---|---|
| Pi | extension-hook (pi.on events + ClientMessage control) | (grounded via remote_pi pi-extension source, in commissioning item) |
| Claude Code | CLI + Python/TS Agent SDK + Remote Control + Agent view | `specialists/claude-code.md` |
| Codex | `codex app-server` JSON-RPC + Python/TS SDK | `specialists/codex.md` |
| Cursor | IDE GUI + Cloud Agents REST/SSE + narrow VS Code extension API | `specialists/cursor.md` |
| OpenCode | typed HTTP API (effect/HttpApi) + SSE event bus + `serve`/attach | `specialists/opencode.md` |
| Aider | interactive CLI + slash commands + scripting | `specialists/aider.md` |
| Antigravity | Python SDK lifecycle hooks + managed Interactions API | `specialists/antigravity.md` |

## Cross-harness common action set (the harness-agnostic core)

Every surveyed harness exposes most of the following action classes — no single harness exposes *all* (e.g., Aider has no control-surface spawn and no separate scripting attach), but each primitive is attested across the set. The commonality is strong enough to support a normative action registry (informs the consuming feature's D4 decision). The revised spine is **spawn / attach / operate / receive / payload** — five primitives, where `operate` is the cluster of drive/request/query (lifecycle-bearing and lifecycle-acting actions within an attached session). This replaces the earlier six-class "drive/request/query/result/payload/provision" spine: `provision` is renamed `spawn` (the operator's established vocabulary, grounded in `[snc-rao-landscape]`); `result` is renamed `receive`; and a new `attach` primitive is split out from the old `drive` (which silently assumed attachment had already happened).

The tables below are a derived summary grounded in the per-harness attestations under `.research/attestation/` (cited per row in the Drive table; the other tables reuse the same per-harness attestation handles established there). Specialist briefs under `specialists/` are the analytical lens that organized the survey; the attestations are the citation substrate.

### Spawn (bring a new harness instance/session into existence)

Creates a session/process that did not exist. Structurally distinct from attach (which joins an existing session) and operate (which acts within one). The prior SNC research `[snc-rao-landscape]` established this as the "spawn vs pilot" cut; this survey refines it to spawn/attach/operate.

| Harness | Spawn mechanism |
|---|---|
| Pi | `pi-supervisord` (out-of-band sysadmin, systemd/launchd) `[pi-extension]` |
| Claude Code | `claude remote-control --spawn worktree|same-dir|session --capacity N` (server mode, operator-action session provisioning) `[snc-rao-sp-cc-remote-control]{1}`; Dispatch (mobile → Desktop → spawn) `[snc-rao-sp-cc-desktop]{2}` |
| Codex | `thread/start`/`thread/resume`/`thread/fork` (in-process session creation) `[codex-appserver-readme]{5}`; `process/spawn` (experimental unsandboxed host process) `[codex-appserver-types]{10}` |
| Cursor | `POST /v1/agents` (Cloud Agents, dedicated Cursor-managed machines) `[cursor-cloud-agents-api]{4}` `[cursor-run-modes]{7}` |
| OpenCode | `opencode serve` (headless HTTP API server; self-hosted, no vendor relay) `[snc-rao-ae-opencode-cli]{3}` |
| Aider | none in the control surface (operator starts the process) `[aider-args]` |
| Antigravity | SDK `Agent` context-manager starts a local harness process (programmatic local sidecar) `[antigravity-sdk-repo]{2}`; managed Interactions API `environment="remote"` provisions a Google-hosted sandbox `[antigravity-managed-agent]{3}` |

**Prior art exists.** This is the corrected finding: Claude Code Remote Control `--spawn`, OpenCode `serve`, Codex thread creation, Cursor Cloud Agents, and Antigravity managed environments are all operator-action spawn mechanisms. The operator has deployed the Claude Code path (`/home/agent/projects/SNC/docs/ops/remote-agent-piloting.md`). Patchbay's spawn is **not novel as a primitive** — it is novel in being **harness-agnostic + durable/authority-bearing** (grant-checked, LSN-tracked, snapshot-recoverable spawn across harnesses, not just within one).

### Attach (connect a control surface to a session that exists; reconcile state)

Joins an existing session. No content flows yet; the control surface establishes a connection, authenticates, and reconciles its state against the session's authoritative snapshot/generation. This primitive was previously collapsed into "drive"; it is split out because it has distinct protocol semantics (which session, which generation, snapshot reconciliation on join, connectivity state transition `unknown→live`).

| Harness | Attach mechanism |
|---|---|
| Pi | `pair_request` + `session_sync` (pair to a known peer, sync session state) `[pi-extension]` |
| Claude Code | connect from claude.ai/mobile to a running session; session sync across surfaces; trusted-device verification `[claude-code-remote-control]{1}` |
| Codex | client connects to app-server; subscribes to thread/turn/item notification stream `[codex-appserver-readme]{6}` |
| Cursor | client connects to Cloud Agents run stream (SSE) `[cursor-cloud-agents-api]` |
| OpenCode | client `createOpencodeClient` + auth headers, connect to running `serve` `[opencode-session-handler]` `[snc-rao-ae-opencode-cli]{3}` |
| Aider | (interactive: stdin attach is implicit; scripting mode has no separate attach) `[aider-io]` |
| Antigravity | SDK `LocalConnection` handshake over stdio/WebSocket `[antigravity-sdk-repo]{2}` |

### Operate — Drive (lifecycle-bearing operator intent within an attached session)

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

### Receive (agent→operator output — not an operator action)

All seven surface agent output that correlates back to a prior operate/drive-action. (Renamed from "Result" to "Receive" in the revised spine — the operator *receives* output; it is not an action the operator takes.)

| Harness | Receive types |
|---|---|
| Pi | `agent_chunk`, `agent_message`, `agent_done`, `tool_request`, `tool_result`, `compaction` `[pi-extension]` |
| Claude Code | message stream (assistant messages, tool_use, tool_result), task notifications `[claude-code-sdk-types]` |
| Codex | `item/started`→deltas→`item/completed`; `userMessage`/`agentMessage`/`reasoning`/`commandExecution`/`fileChange` items; `turn/started`/`turn/completed` `[codex-appserver-readme]{6}` |
| Cursor | run stream (SSE), agent output `[cursor-cloud-agents-api]` |
| OpenCode | `session.next.*` events, `permission.v2.*` events, `question.*` events `[opencode-schema-events]` |
| Aider | local output channels + analytics events (`message_send_*`, `command_*`) `[aider-io]` |
| Antigravity | `ChatResponse` stream, `agentMessage` items, tool-result events `[antigravity-sdk-repo]{2}` |

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

## Spawn postures diverge across harnesses (corrected)

This directly informs the consuming feature's D3 (spawn as a first-class action class). **Corrected finding:** spawn *is* exposed as an operator action by several harnesses — the initial synthesis wrongly concluded it was absent. Four postures surfaced:

**Posture 1 — out-of-band sysadmin:** Pi (`pi-supervisord`, systemd/launchd-managed, explicitly excluded from the setup wizard `[pi-extension]`), Aider (no instance spawn/retire in args/commands `[aider-args]` `[aider-commands]`).

**Posture 2 — programmatic local sidecar/session startup:** Antigravity's Python SDK starts a local harness process via the `Agent` async context manager (binary discovery, handshake over stdio/WebSocket) `[antigravity-sdk-repo]{2}`. OpenCode `serve` runs a headless HTTP API server the operator can start and to which clients attach `[snc-rao-ae-opencode-cli]{3}` — self-hosted, no vendor relay. (Both are "operator-started local process with programmatic attach" — distinct from posture 1's pure-sysadmin.)

**Posture 3 — in-process session/thread creation:** Codex (`thread/start`/`thread/resume`/`thread/fork` create conversations within a running app-server `[codex-appserver-readme]{5}`), Claude Code (Remote Control server-mode `--spawn same-dir|worktree|session` spawns sessions within an attached harness `[snc-rao-sp-cc-remote-control]{1}`), OpenCode (`prompt`/`remove` create/remove sessions within the server `[opencode-session-handler]`).

**Posture 4 — cloud-managed agent creation:** Cursor Cloud Agents (`POST /v1/agents` creates a durable Cloud Agent on dedicated Cursor-managed machines `[cursor-cloud-agents-api]{4}` `[cursor-run-modes]{7}`), Antigravity managed Interactions API (`environment="remote"` provisions a Google-hosted sandbox `[antigravity-managed-agent]{3}`), Claude Code Dispatch (mobile-app-triggered spawn via Desktop `[snc-rao-sp-cc-desktop]{2}`).

**Synthesis (corrected):** Spawn-as-operator-action **has prior art** across postures 2–4 — Claude Code Remote Control `--spawn`, OpenCode `serve`, Codex thread creation, Cursor Cloud Agents, Antigravity managed, and Claude Dispatch all expose it. The operator has deployed the Claude Code path (`/home/agent/projects/SNC/docs/ops/remote-agent-piloting.md` — a systemd unit with `ExecStart=...claude remote-control --spawn worktree --capacity 8`). What is **novel for patchbay** is not the spawn primitive but **harness-agnostic + durable/authority-bearing spawn**: grant-checked, LSN-tracked, snapshot-recoverable spawn across harnesses (not just within one vendor's ecosystem), unifying the four postures under one authority model. The operator's actual pain ("spawn a fresh Pi on my VM from my phone") is *partially* served by Claude Code's path but *not* for Pi — and patchbay's value is making it harness-agnostic.

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

**`[medium]` Spawn-scope divergence across harnesses.** The seven harnesses expose four spawn postures (out-of-band sysadmin / programmatic local sidecar+serve / in-process session creation / cloud-managed). This is not a contradiction *within* a source but a divergence *across* sources — the harness-agnostic action model must accommodate all four. The consuming feature's design pass must decide whether patchbay's Spawn action class targets (a) only out-of-band-supervised spawn on an arbitrary operator machine (the operator's actual pain for Pi), (b) in-process session creation, (c) cloud-managed, or some combination — and how patchbay's authority/durability layer wraps each posture.

**`[low]` Aider's `!`/`/run` shell passthrough.** Aider treats `!`-prefixed input as a shell command (`command_run`), distinct from chat — a harness-specific payload variant, not a contradiction. Noted as a payload-class refinement.

## Disconfirming analysis

Before claiming the five-primitive common set (spawn/attach/operate/receive/payload) is universal, I searched across the specialist briefs for actions that *don't* fit:
- Codex `turn/steer` — adds input to an in-flight turn without canceling. This is a *refinement of operate/drive* (drive into a running turn), not a new class. Fits under Operate/Drive.
- Codex `review/start` (automated reviewer) — a harness-specific automated action, not operator-initiated. Out of scope for the operator action inventory.
- OpenCode `revert`/`unrevert` — git-pair revert as a session action. This is a *session-management request* (lifecycle-acting on the session's git state), fits under Operate/Request.
- Cursor checkpoint restore — local lifecycle-ish control; fits under Operate/Request (acts on session state).
- Antigravity triggers (`automated_trigger` messages) — long-lived async tasks that send content into the agent. These are *automated operate/drive* (non-operator-initiated), a distinct source but the same action shape.
- Aider `/undo`, `/commit`, `/save`, `/load` — session-state mutations; fit under Operate/Request (lifecycle-acting on session/git state).

No action surfaced that breaks the five-primitive spine. The classification survives the survey.

## Answers to the engagement's seed questions

1. **Operator→agent control actions across harnesses:** spawn (provision/attach), then operate (drive, interrupt, approve/answer, query, plus session-management requests like revert/compact/share/move/remove-message and reconfigure like model/agent switch). Cataloged per-harness above.
2. **Agent→operator events:** message chunks, tool-call requests, tool results, turn/agent lifecycle, compaction, errors — universal across all seven (the Receive primitive).
3. **Durable vs ephemeral vs read-only:** spawn/attach = lifecycle-of-the-connection; operate/drive = lifecycle-bearing (terminal-race semantics); operate/request (interrupt/approve/session-mgmt) = lifecycle-acting; operate/query = read-only; receive = agent output; payload = content.
4. **Privileged sidecar/supervisor (spawn):** four postures — out-of-band sysadmin (Pi `pi-supervisord`, Aider); programmatic local sidecar/serve (Antigravity SDK, OpenCode `serve`); in-process session creation (Codex threads, Claude Code Remote Control `--spawn`); cloud-managed (Cursor Cloud Agents, Antigravity managed, Claude Dispatch). **Spawn has prior art** — the operator deployed the Claude Code path. Patchbay's novelty is harness-agnostic + durable/authority-bearing spawn, not the primitive itself.
5. **No-grant informational replyable content:** no harness exposes a generic *operator-originated* no-grant `Message` command (Q-A); several expose *agent-originated* question/elicitation reply paths (Q-B — Claude `AskUserQuestion`, Codex tool user-input requests, OpenCode `question.asked`, Antigravity `ASK_QUESTION`), which the consuming feature must model separately (likely as a Request variant). The operator-originated `Message` type drops for v0; the agent-originated question surface is a separate modeling question.
6. **Spawn mechanisms:** four postures (out-of-band sysadmin / programmatic local sidecar+serve / in-process session creation / cloud-managed); **prior art exists** across postures 2–4. Patchbay's value is unifying these under one harness-agnostic, durable, authority-bearing spawn.

## Revisit if

- A harness surfaces genuine no-grant informational replyable content (operator-originated) in a future version — would re-open the `Message` question.
- A harness not surveyed exposes a spawn primitive that crosses the four-posture taxonomy — would extend the spawn findings.
- Deeper survey of OpenCode's `control-plane`/`installation` modules surfaces fleet-management semantics beyond `serve` — would refine the OpenCode spawn posture.

## Acquisition candidates

- **`blocking`** — Antigravity `agy` CLI canonical docs (the specialist could not fetch CLI verbs/flags beyond the SPA shell; `antigravity-google` docs `/docs/cli/overview`). Completes: Antigravity CLI control semantics.
- **`blocking`** — Antigravity SDK Overview published docs + permissions docs. Completes: Antigravity SDK lifecycle-hook detail.
- **`enriching`** — Cursor Cloud Agents OpenAPI spec (`https://cursor.com/docs-static/cloud-agents-openapi.yaml`, source-bound via the Cloud Agents API docs). Would deepen the Cursor provisioning surface.
- **`enriching`** — Codex `codex app-server generate-json-schema --experimental` output from the exact Codex binary version under evaluation. Would ground the exact app-server schema.

These persist research-side; promotion to `.work/` is operator-confirmed at the research-handoff gate.
