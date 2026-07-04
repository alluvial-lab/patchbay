---
provenance: agent-synthesis
updated: 2026-07-03
campaign: harness-action-surfaces
facet: claude-code
---

# Claude Code action surface

## Source scope

This brief uses fetched Claude Code documentation and the Python Agent SDK source/types. The `claude-code-sdk-python` starting point appears to have moved to the `claude-agent-sdk-python` repository: GitHub's repository contents API for the former returned paths under `anthropics/claude-agent-sdk-python`, and the fetched source-direct raw file is `src/claude_agent_sdk/types.py` at that current repository path. This metadata observation is included here as fetch provenance rather than as a numbered source claim.

## Operator → agent control actions

### Drive / prompt

Claude Code exposes several drive surfaces:

- Interactive CLI start (`claude`), interactive initial prompt (`claude "query"`), print-mode one-shot (`claude -p "query"`), and piped print-mode input.[claude-code-cli]{1}
- SDK `query()` and `ClaudeSDKClient.query()` send user prompts/messages; in streaming mode, a string prompt is wrapped as a JSON user message and async iterables can send multiple user messages.[claude-code-sdk-client]{5}
- Agent SDK slash commands are sent as prompt strings, and dispatchability is limited to commands that work without an interactive terminal; the init system message lists available commands.[claude-code-slash-commands]{1}[claude-code-slash-commands]{2}
- Remote Control lets terminal, browser, and mobile surfaces send messages into the same local session.[claude-code-remote-control]{5}
- Agent view dispatch starts a new background session for each prompt entered in its dispatch input rather than appending a follow-up to an existing row.[claude-code-agent-view]{2}

### Interrupt / cancel / stop

Claude Code has multiple interruption/stop planes:

- SDK `ClaudeSDKClient.interrupt()` sends an interrupt signal in streaming mode.[claude-code-sdk-client]{6}
- Single-message SDK mode explicitly does not support real-time interruption, so interruption is a streaming-mode capability, not a universal `query()` property.[claude-code-sdk-client]{1}
- SDK control protocol includes an `interrupt` request and a `stop_task` request.[claude-code-sdk-types]{11}
- `ClaudeSDKClient.stop_task(task_id)` stops a running task and should be followed by a task notification with status `stopped` in the message stream.[claude-code-sdk-client]{11}
- Agent view and shell surfaces can stop background sessions with `Ctrl+X`, `claude stop`, or `claude kill`; shell also exposes `respawn`, `rm`, and daemon stop commands.[claude-code-agent-view]{6}[claude-code-agent-view]{9}
- In attached background sessions, `Ctrl+C` cancels a running response or shell command, while detach controls leave the background session running.[claude-code-agent-view]{3}

### Approve / deny tool and answer questions

Claude Code exposes both human permission approvals and structured clarifying-question replies:

- SDK applications pass `canUseTool`/`can_use_tool`; it fires when Claude needs permission for a tool or calls `AskUserQuestion`, pauses execution, and resumes only when the callback returns.[claude-code-user-input]{1}[claude-code-user-input]{4}
- Tool approval responses can allow original/modified input, deny with a message Claude sees, or approve-and-remember by applying suggested permission updates.[claude-code-user-input]{7}[claude-code-user-input]{8}
- `AskUserQuestion` is a no-grant informational reply surface: Claude generates one to four structured questions with options; the operator/application returns answers or optional freeform response, and the response supplies information rather than granting authority to execute a tool.[claude-code-user-input]{1}[claude-code-user-input]{10}[claude-code-user-input]{11}
- The same `canUseTool` callback also carries tool approval requests, so applications must distinguish informational questions from authority-bearing approvals by `tool_name == "AskUserQuestion"` versus other tools.[claude-code-user-input]{4}
- Hooks can participate in tool authority: `PreToolUse` can allow, deny, ask, or defer; `PermissionRequest` can allow/deny/update input/permissions; silent `PreToolUse` output does not itself approve a call.[claude-code-hooks]{8}[claude-code-hooks]{9}
- The SDK control protocol has a `can_use_tool` control request variant with tool name, input, permission suggestions, blocked path, reason/title/display fields, tool-use id, and optional agent id.[claude-code-sdk-types]{11}

### Sync / refresh / reconnect

Claude Code exposes several sync and refresh actions:

- Sessions persist to disk and can be continued, resumed by ID, or forked; `continue` finds the latest session for the cwd, `resume` selects an explicit session ID, and `fork` copies conversation history into a new session.[claude-code-sessions]{1}[claude-code-sessions]{4}
- Python `ClaudeSDKClient` handles session IDs internally for multi-turn use; TypeScript uses `continue: true` for subsequent `query()` calls.[claude-code-sessions]{5}
- SDK session enumeration/mutation functions include listing sessions, reading messages, getting session info, renaming, and tagging.[claude-code-sessions]{10}
- `ClaudeSDKClient.reconnect_mcp_server`, `toggle_mcp_server`, and `get_mcp_status` provide live MCP reconnect/enable/disable/status surfaces.[claude-code-sdk-client]{10}
- Remote Control reconnects to remote sessions when possible, keeps browser/mobile/terminal conversation views in sync, and has status indicators/URLs/QR codes.[claude-code-remote-control]{5}[claude-code-remote-control]{6}
- Hooks can request `reloadSkills` at `SessionStart` so skills installed or updated by startup hooks are available in the same session.[claude-code-hooks]{8}

### Model / thinking / permission reconfiguration

Claude Code exposes model and reasoning controls at launch and in selected live contexts:

- CLI flags include `--model`, `--fallback-model`, `--effort`, `--permission-mode`, and tool allow/deny flags.[claude-code-cli]{5}
- SDK options include `model`, `fallback_model`, `thinking`, `effort`, `permission_mode`, `allowed_tools`, `disallowed_tools`, `tools`, and `permission_prompt_tool_name`.[claude-code-sdk-types]{10}
- `ClaudeSDKClient.set_model()` and `set_permission_mode()` change model and permission mode during a streaming-mode conversation.[claude-code-sdk-client]{7}[claude-code-sdk-client]{8}
- Agent view can set a dispatch default model using `/model` in the dispatch input or flags when opening agent view, and individual background sessions can use distinct models.[claude-code-agent-view]{7}
- Hook common input exposes the current `permission_mode` and active effort level; model is only an optional `SessionStart` field, so hooks are not a general live model-state event feed.[claude-code-hooks]{5}

### Session new / compact / clear / resume / fork

- Starting a new session is implicit in `claude`, `claude "query"`, a fresh `query()`, a background dispatch, or a Remote Control server-mode on-demand session.[claude-code-cli]{1}[claude-code-agent-view]{2}[claude-code-remote-control]{3}
- Continue/resume/fork are explicit SDK/CLI session controls.[claude-code-sessions]{3}[claude-code-cli]{6}
- `/compact` and `/clear` are SDK-dispatchable slash commands; `/compact` emits a `compact_boundary` system message when compaction occurs, while `/clear` resets conversation context while leaving the previous conversation on disk for resume.[claude-code-slash-commands]{3}[claude-code-slash-commands]{4}
- Hooks expose `PreCompact` and `PostCompact`; `PreCompact` can block compaction and match manual versus automatic trigger.[claude-code-hooks]{3}[claude-code-hooks]{6}[claude-code-hooks]{8}
- `SessionStart` matchers include `startup`, `resume`, `clear`, and `compact`, so hook consumers can observe the lifecycle reason for a session start.[claude-code-hooks]{3}

### Provision / spawn / retire / stop

Claude Code has both operator-facing spawn controls and a privileged supervisor/server layer:

- Agent view dispatch, `/bg`, and `claude --bg` create background Claude Code sessions; each background session is a full conversation and each prompt in agent view starts a new session.[claude-code-agent-view]{1}[claude-code-agent-view]{8}
- Background sessions are hosted by a separate per-user supervisor process, which starts automatically, pre-warms workers, assigns workers to sessions, and applies session directory/settings/credentials.[claude-code-agent-view]{10}
- Background session stop/remove/respawn are operator actions through agent view keys or shell commands.[claude-code-agent-view]{6}[claude-code-agent-view]{9}
- Remote Control server mode exposes session creation capacity and spawn modes (`same-dir`, `worktree`, `session`) as CLI startup flags, while normal interactive Remote Control provides one remote session per interactive process unless server mode is used.[claude-code-remote-control]{3}[claude-code-remote-control]{7}
- The supervisor is privileged relative to the foreground TUI because it owns detached worker lifecycles, but it is not an application-level protocol primitive exposed through the Agent SDK control protocol; the SDK control request union contains interrupt/permission/init/hooks/MCP/rewind/stop-task controls, not create-session or retire-session variants.[claude-code-agent-view]{10}[claude-code-sdk-types]{11}

## Agent → operator events

### Message stream

The SDK message stream includes user, assistant, system, result, partial stream, hook-event, rate-limit, task, and mirror-error surfaces:

- Content blocks include text, thinking, tool use, tool result, server-side tool use, and server-side tool result.[claude-code-sdk-types]{6}
- `AssistantMessage` reports content, model, usage/error/stop fields, message id, session id, and parent tool-use id; `ResultMessage` reports turn completion metadata, cost/usage, session id, result text, permission denials, deferred tool use, errors, and API error status.[claude-code-sdk-types]{7}
- `StreamEvent` carries raw partial assistant stream updates when partial messages are enabled, and `HookEventMessage` carries hook-started/hook-response lifecycle events when hook events are included.[claude-code-sdk-types]{12}
- `ClaudeSDKClient.receive_messages()` yields parsed message objects, and `receive_response()` terminates after yielding a `ResultMessage`.[claude-code-sdk-client]{4}[claude-code-sdk-client]{13}
- CLI stream-json can include hook lifecycle events and partial messages using `--include-hook-events` and `--include-partial-messages`.[claude-code-cli]{7}

### Tool-call requests and results

- Tool calls appear as `ToolUseBlock` content blocks and tool results as `ToolResultBlock`; custom MCP tools are named `mcp__{server_name}__{tool_name}` and can be allowed to run without a prompt.[claude-code-sdk-types]{6}[claude-code-plugins-tools]{7}
- Custom-tools examples inspect `AssistantMessage` objects for tool calls and final `ResultMessage` for text results.[claude-code-plugins-tools]{9}
- Hook lifecycle events can be included in the stream, giving event-level visibility for `PreToolUse`, `PostToolUse`, `Stop`, and other hook events when enabled.[claude-code-sdk-types]{12}

### Turn / lifecycle / compaction / errors

- Hooks expose lifecycle events once per session, once per turn, and around every tool call; hook events include session start/end, prompt submission/expansion, permission request/denial, tool pre/post/failure/batch, subagent start/stop, task created/completed, compaction, notifications, message display, config change, and more.[claude-code-hooks]{2}
- Result messages carry stop reason, error status, and result subtype; task messages carry started/progress/notification/updated states and terminal statuses.[claude-code-sdk-types]{7}[claude-code-sdk-types]{8}
- Hook exit code and decision-control behavior determines whether an event blocks, feeds error to Claude, shows text to the user, or has no decision effect.[claude-code-hooks]{6}[claude-code-hooks]{8}
- Agent view surfaces session state summaries: needs input, idle, working, done, failed, stopped; a row can be peeked for the specific question, permission decision, recent output, or PR status.[claude-code-agent-view]{3}[claude-code-agent-view]{4}

## Durability classification

| Action/event | Classification | Rationale |
|---|---|---|
| User prompt / SDK user message | Ephemeral payload within durable session | Message is payload, but session transcript persists to disk.[claude-code-sessions]{1} |
| Continue / resume / fork / session id | Durable lifecycle-bearing | These select or create durable transcript histories.[claude-code-sessions]{4} |
| `/clear` | Durable lifecycle-bearing | It resets current context while prior conversation remains on disk for resume.[claude-code-sessions]{4} |
| `/compact` / auto compaction | Durable lifecycle-bearing | Compaction changes persisted conversation context and has hook events/compact boundary surfaces.[claude-code-hooks]{3}[claude-code-hooks]{8} |
| Tool approval allow/deny | Ephemeral authority decision, optionally durable when remembered | One-shot allow/deny gates one call; approve-and-remember can write a permission rule to local settings.[claude-code-user-input]{7}[claude-code-user-input]{8} |
| `AskUserQuestion` answer | Ephemeral informational payload | It returns answers/freeform response to Claude and does not grant tool authority.[claude-code-user-input]{10} |
| Model/permission mode set | Lifecycle-bearing session configuration | It changes subsequent behavior in the live session; background-session start config persists across supervisor restart.[claude-code-sdk-client]{7}[claude-code-sdk-client]{8}[claude-code-agent-view]{10} |
| MCP status/context/server info | Read-only query | SDK methods return status/usage/server info without asserting mutation.[claude-code-sdk-client]{10}[claude-code-sdk-client]{12} |
| MCP reconnect/toggle | Lifecycle-bearing external-service control | Reconnect/toggle changes server availability and tool set.[claude-code-sdk-client]{10} |
| Interrupt | Ephemeral control | It cancels/interrupts active generation in streaming mode, not durable state by itself.[claude-code-sdk-client]{6} |
| Stop task | Ephemeral-to-lifecycle for task | It changes a task to stopped/killed terminal status.[claude-code-sdk-client]{11}[claude-code-sdk-types]{8} |
| Background dispatch / `--bg` | Durable lifecycle-bearing | Creates a background session with on-disk state and supervisor ownership.[claude-code-agent-view]{8}[claude-code-agent-view]{12} |
| Background stop/rm/respawn | Durable lifecycle-bearing | These change session process/list/worktree lifecycle.[claude-code-agent-view]{9} |
| Hook event messages / partial stream events | Ephemeral events | They are stream observations; hook-added context may become transcript content when injected.[claude-code-sdk-types]{12}[claude-code-hooks]{7} |

## Privileged sidecar / supervisor requirements

Claude Code background-session provisioning uses a privileged local supervisor sidecar rather than only in-band SDK calls: agent view/background sessions are hosted by a per-user supervisor that pre-warms workers and manages detached Claude Code processes.[claude-code-agent-view]{10} Operators can trigger background sessions via agent view, `/bg`, and `claude --bg`, but the actual process lifetime is managed by the supervisor.[claude-code-agent-view]{8}[claude-code-agent-view]{10}

Remote Control server mode is another out-of-band process surface: `claude remote-control` starts a server mode that registers/polls via Anthropic APIs and can spawn sessions according to startup flags such as `--spawn` and `--capacity`.[claude-code-remote-control]{2}[claude-code-remote-control]{3}[claude-code-remote-control]{8} In contrast, the Python SDK control protocol exposes live controls for interrupt, permission callback, initialization, permission mode, hooks, MCP, file rewind, and task stop, but not a generic create-agent-instance or retire-agent-instance request.[claude-code-sdk-types]{11}

## Message-vs-command finding

Claude Code is not purely “operator drives, agent replies.” It has at least one explicit agent→operator informational reply surface that is not an authority grant: `AskUserQuestion`. The user-input guide says Claude calls `AskUserQuestion` for clarifying questions, the application presents Claude-generated questions/options, and the response returns answers or freeform text; this differs from tool approval even though both are delivered through `canUseTool`.[claude-code-user-input]{1}[claude-code-user-input]{2}[claude-code-user-input]{10} The distinction is implementation-level: `tool_name == "AskUserQuestion"` is routed differently from permission requests for tools such as Bash/Write/Edit.[claude-code-user-input]{4}

Claude Code also has notification/read-only surfaces that are informational rather than commands: Remote Control mobile push notifications can be configured for “Claude decides” and “actions required,” and agent view row summaries/peek panels show recent output or needed input.[claude-code-remote-control]{10}[claude-code-agent-view]{3} These may prompt operator attention, but the documented authority-bearing path remains tool permission allow/deny or permission-mode/rule changes.[claude-code-user-input]{7}[claude-code-user-input]{8}

## Spawn/retire finding

Spawn/retire is exposed to operators for background and Remote Control sessions, but not as a portable Agent SDK in-band control request. Operator-facing spawn surfaces include agent view dispatch, `claude --bg`, `/bg`, and Remote Control server-mode session creation.[claude-code-agent-view]{8}[claude-code-remote-control]{3} Operator-facing retire/stop surfaces include `Ctrl+X`, `claude stop`/`kill`, `claude rm`, daemon stop, and remote-session process termination constraints.[claude-code-agent-view]{6}[claude-code-agent-view]{9}[claude-code-remote-control]{11} The implementation sidecar/supervisor owns detached process lifetime and per-job state.[claude-code-agent-view]{10}[claude-code-agent-view]{12}

## Comparison to Pi action-surface lens

Against the provided Pi comparison lens, Claude Code has a broader documented local operator UI for background sessions: agent view bundles session spawning, peeking/replying, attaching, stopping, removing, respawning, and shell commands in one local supervisor-backed surface.[claude-code-agent-view]{1}[claude-code-agent-view]{9}[claude-code-agent-view]{10} Claude Code's Agent SDK exposes in-band live controls such as interrupt, set permission mode, set model, MCP reconnect/toggle, file rewind, and task stop, but not the same explicit `session_new`, `session_sync`, `thinking_set`, or `list_models` control vocabulary described in the Pi lens.[claude-code-sdk-client]{6}[claude-code-sdk-client]{7}[claude-code-sdk-client]{8}[claude-code-sdk-client]{10}[claude-code-sdk-client]{11} Claude Code does expose thinking/effort at option/CLI level, and effort appears in hook input, but I did not attest a live `set_effort` client method parallel to `set_model`.[claude-code-sdk-types]{10}[claude-code-hooks]{5}

## Disconfirming analysis

Before asserting the no-grant message finding, I checked the user-input guide for whether clarifying questions share the same approval semantics as tool permission. The source explicitly says tool permissions and clarifying questions are two situations, both trigger `canUseTool`, and `AskUserQuestion` should be detected and handled differently; its response format is answers/freeform reply, not allow/deny.[claude-code-user-input]{1}[claude-code-user-input]{4}[claude-code-user-input]{10} This disconfirms a universal “all operator replies are grants” reading.

Before asserting that generic spawn/retire is not in the Agent SDK control protocol, I checked the SDK `SDKControlRequest` union and `ClaudeSDKClient` public methods. The union includes interrupt/permission/init/permission-mode/hook/MCP/rewind/stop-task, and the client exposes live methods for interrupt, set permission mode/model, rewind, MCP reconnect/toggle/status, context usage, server info, and stop task; I found no source-attested create-session/retire-session control request in those fetched types/methods.[claude-code-sdk-types]{11}[claude-code-sdk-client]{6}[claude-code-sdk-client]{12} This does not rule out other SDK language bindings having helper APIs, but it gates the stronger claim to the fetched Python SDK and docs.

Before asserting supervisor/sidecar involvement, I checked agent view and Remote Control docs. Agent view explicitly documents a per-user supervisor with worker processes and state directories; Remote Control server mode explicitly documents local server-mode startup flags and outbound API polling.[claude-code-agent-view]{10}[claude-code-agent-view]{12}[claude-code-remote-control]{8} This supports classifying background provisioning as out-of-band local process management rather than a pure prompt-level command.

Before asserting SDK event breadth, I compared hooks docs with SDK hook types. The hooks reference lists more lifecycle events than the Python SDK `HookEvent` type subset; therefore the brief separates the full Claude Code hook surface from the SDK-typed hook subset rather than merging them.[claude-code-hooks]{2}[claude-code-sdk-types]{3}

## Contradictions

No direct contradiction was found among attested sources. One scoped tension is worth preserving:

| Handles | Relationship | Detail |
|---|---|---|
| `claude-code-hooks` / `claude-code-sdk-types` | qualifies | The hooks documentation enumerates the broad Claude Code hook event surface, while Python SDK `HookEvent` types model a subset. The SDK source does not contradict the docs; it qualifies what the Python SDK type surface exposes directly.[claude-code-hooks]{2}[claude-code-sdk-types]{3} |
| `claude-code-user-input` / `claude-code-sdk-client` | qualifies | User-input docs say `canUseTool` covers approvals and questions, but the Python client source says `can_use_tool` requires streaming mode and cannot be combined with `permission_prompt_tool_name`; the client source qualifies where that documented callback can be used in Python.[claude-code-user-input]{4}[claude-code-sdk-client]{3} |

## Revisit if

- Revisit if Anthropic publishes a stable Agent SDK “control protocol” spec separate from Python source; this brief uses the fetched Python SDK types as source-direct evidence for control request variants.[claude-code-sdk-types]{11}
- Revisit if TypeScript SDK docs/source expose live controls not present in Python `ClaudeSDKClient`, especially for effort/thinking or session lifecycle.
- Revisit if Remote Control or agent view adds public programmable APIs for spawning/retiring sessions rather than CLI/TUI/supervisor-only surfaces.
- Revisit if `AskUserQuestion` becomes available inside Agent-tool subagents; current user-input docs say it is not currently available there.[claude-code-user-input]{11}
