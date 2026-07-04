---
provenance: agent-synthesis
updated: 2026-07-03
campaign: harness-action-surfaces
facet: google-antigravity
---

# Google Antigravity action/event surface brief

## Scope and source basis

This brief covers fetched public Antigravity materials and the open-source Python SDK repository. The strongest source for lifecycle hooks and programmatic control is the SDK source and README. The Antigravity desktop/CLI product docs were not retrievable as content beyond the home-page surface overview during this pass; CLI-specific `agy` command semantics therefore remain an acquisition gap.

## Disconfirming analysis

- The home page names desktop, CLI, SDK, and IDE as product surfaces over a shared harness, but it is an overview, not a command/API reference; it cannot prove exact `agy` CLI verbs or flags. [antigravity-home]{1}
- The Python SDK repository is a programmatic local-harness client; it does not implement the desktop app or TUI. Findings from SDK source should not be promoted to all Antigravity surfaces unless corroborated by surface docs. [antigravity-sdk-repo]{2}
- The Gemini API Antigravity Agent docs describe a managed remote Interactions API surface, not the Python SDK local harness; it corroborates managed-agent lifecycle concepts but not SDK hook internals. [antigravity-managed-agent]{3}

## Operator → agent control actions

### Durable or lifecycle-bearing actions

- **Start/provision a local SDK agent session.** The SDK `Agent` is an async context manager that performs binary discovery, tool wiring, hook registration, policy setup, starts a local harness process, handshakes over stdio/WebSocket, initializes a conversation, and disconnects on context exit. [antigravity-sdk-repo]{2}
- **Send/drive a turn.** `Agent.chat()` delegates to `Conversation.chat()`, while `Conversation.send()` sends a prompt; `LocalConnection.send()` serializes prompts as `InputEvent(user_input=...)` or multimodal `complex_user_input`. [antigravity-sdk-repo]{2}
- **Cancel/interrupt an active turn.** `ChatResponse.cancel()`, `Conversation.cancel()`, and `LocalConnection.cancel()` are exposed; the local connection sends `InputEvent(halt_request=True)` and later raises an Antigravity cancellation error to consumers. [antigravity-sdk-repo]{2}
- **Resume/reconnect a persisted conversation.** `LocalAgentConfig` accepts `conversation_id` and `save_dir`; the local harness config carries `cascade_id`, receives initial history during initialization, and exposes `conversation_id`/history through `Conversation`. [antigravity-sdk-repo]{2}
- **Clear local SDK transcript memory.** `Conversation.clear_history()` clears recorded SDK-side history and usage counters without stopping the live conversation; this is local state management, not a harness compaction command. [antigravity-sdk-repo]{2}
- **Disconnect/retire the SDK sidecar session.** `Conversation.disconnect()` and `Agent.__aexit__()` tear down connection resources; the local strategy starts a compiled harness process and disconnects it on exit. [antigravity-sdk-repo]{2}
- **Managed Interactions API provision/reuse/cancel.** The managed API can create an interaction with `environment="remote"` to provision a sandbox, reuse an `environment_id`, poll stored background runs, and cancel by interaction ID. [antigravity-managed-agent]{3}

### Ephemeral/payload actions

- **Tool approval/denial.** `PreToolCallDecideHook`/policy evaluation can approve or deny tool calls; local built-in and MCP confirmations are sent back as `InputEvent(tool_confirmation=...)` with an accepted boolean. [antigravity-sdk-repo]{2}
- **Answer agent questions.** The `ASK_QUESTION` builtin and `OnInteractionHook` carry multiple-choice question specs and send `InputEvent(question_response=...)` answers. The interactive utility implements this via stdin. [antigravity-sdk-repo]{2}
- **Return custom tool/function results.** SDK host-side tools are executed through `ToolRunner` and returned via tool-response input events; the managed API exposes custom function calls as `requires_action` and expects a later `function_result` tied to `previous_interaction_id`. [antigravity-sdk-repo]{2} [antigravity-managed-agent]{3}
- **Trigger-originated messages.** SDK triggers are long-lived async tasks whose context can `send(content)` into the agent as `automated_trigger` messages. [antigravity-sdk-repo]{2}

### Configuration actions

- **Tool/capability configuration.** `CapabilitiesConfig` controls subagent enablement, enabled/disabled builtin tools, compaction threshold, and finish schema; disabled tools are hidden from model context, while policies deny visible tools at runtime. [antigravity-sdk-repo]{2}
- **Model configuration.** `LocalAgentConfig` accepts model/model-list and endpoint fields (`api_key`, Vertex/project/location); the local strategy serializes model targets into harness config at session start. I found configuration at startup, not a live `model_set` command on an existing session. [antigravity-sdk-repo]{2}
- **Workspace/app-data/skills/MCP/subagent configuration.** `LocalAgentConfig` accepts workspaces, app data dir, skills paths, MCP servers, and static `SubagentConfig` records; these are translated into harness config before the session starts. [antigravity-sdk-repo]{2}

### Read-only queries

- **History and state introspection.** `Conversation` exposes `history`, `last_response`, `turn_count`, `compaction_indices`, `is_idle`, `conversation_id`, and usage metadata. [antigravity-sdk-repo]{2}
- **Streaming observation without mutation.** `ChatResponse` exposes text deltas, thoughts, tool calls, full chunk resolution, final text, structured output extraction, and usage metadata. [antigravity-sdk-repo]{2}
- **Managed background status polling.** The managed API example polls `client.interactions.get(id=...)` until a background run is completed or failed. [antigravity-managed-agent]{3}

## Agent → operator event surface

- **Text and thought chunks.** `Conversation.receive_chunks()` yields `Text` deltas from model/user-targeted content deltas and `Thought` deltas from thinking deltas; `ChatResponse` exposes raw text iteration plus `.thoughts`. [antigravity-sdk-repo]{2}
- **Tool-call events.** `receive_chunks()` yields deduplicated `ToolCall` objects; `ChatResponse.tool_calls` exposes them as a dedicated stream. [antigravity-sdk-repo]{2}
- **Step lifecycle events.** `receive_steps()` yields `Step` records carrying type, source, target, status, content, deltas, tool calls, errors, completion marker, structured output, and usage metadata. Step types include text response, tool call, system message, compaction, finish, thinking, and unknown; statuses include active, done, waiting-for-user, error, canceled, and unknown. [antigravity-sdk-repo]{2}
- **Hook lifecycle callbacks.** Public hook classes cover session start/end, pre/post turn, pre/post tool call, tool error, interaction, and compaction; internal hooks observe pre/post step. [antigravity-sdk-repo]{2}
- **Tool results and errors.** `PostToolCallHook` receives `ToolResult`; local built-in tool results include command output, directory listing, search counts, file content/diff fallback, image name, web summary, and URL-content summary. `OnToolErrorHook` receives exceptions and can alter the error representation the model sees. [antigravity-sdk-repo]{2}
- **Human-interaction requests.** Local waiting states can carry `questions_request` and `tool_confirmation_request`; SDK code handles them asynchronously to avoid blocking parallel subagents. [antigravity-sdk-repo]{2}
- **Managed API function-call requests.** The managed API surfaces custom function requests through `interaction.status == "requires_action"` and `steps` containing unmatched `function_call` records. [antigravity-managed-agent]{3}

## Durable vs ephemeral vs read-only classification

| Surface element | Classification | Basis |
|---|---|---|
| Start `Agent` / `Conversation.create` | Durable/lifecycle-bearing | Starts harness sidecar and session. [antigravity-sdk-repo]{2} |
| `conversation_id`, `save_dir`, initial history | Durable/lifecycle-bearing | Reuses persisted conversation identity/history. [antigravity-sdk-repo]{2} |
| Managed `environment="remote"` / environment ID reuse | Durable/lifecycle-bearing | Creates or reuses Linux sandbox with files/state. [antigravity-managed-agent]{3} |
| `send` / `chat` prompt | Ephemeral/payload | Sends one turn payload into current session. [antigravity-sdk-repo]{2} |
| `cancel` / `halt_request` | Lifecycle-bearing control | Alters active execution lifecycle. [antigravity-sdk-repo]{2} |
| Tool confirmation | Ephemeral grant/deny | Boolean approval response for a requested operation. [antigravity-sdk-repo]{2} |
| Question response | Ephemeral payload | Answers an agent interaction request. [antigravity-sdk-repo]{2} |
| Trigger `send` | Ephemeral payload with background origin | Pushes automated content into the live agent. [antigravity-sdk-repo]{2} |
| Capability/model/workspace/MCP/subagent config | Lifecycle/configuration | Bound into harness config at session initialization. [antigravity-sdk-repo]{2} |
| `history`, `turn_count`, `is_idle`, usage | Read-only query | SDK-side introspection properties. [antigravity-sdk-repo]{2} |
| `ChatResponse` chunk streams | Read-only observation | Consume event stream without granting or driving. [antigravity-sdk-repo]{2} |

## Privileged sidecar/supervisor handling

The local SDK requires a compiled local harness binary. `LocalConnectionStrategy` locates it via `ANTIGRAVITY_HARNESS_PATH`, wheel resources, or `PATH`, launches it with `subprocess.Popen`, exchanges config over stdin/stdout, and then communicates over a localhost WebSocket with an API key from the harness output config. That harness is the privileged sidecar for filesystem, shell, subagent, and environment actions; Python code configures and supervises it rather than implementing those powers in pure Python. [antigravity-sdk-repo]{2}

The SDK also enforces safety before the sidecar executes sensitive operations: `LocalAgentConfig` defaults to current-workspace scoping and command confirmation policy, write tools/MCP servers require an explicit policy or decide hook, and built-in/MCP tool confirmations are routed through `PreToolCallDecideHook`. [antigravity-sdk-repo]{2}

The managed API sidecar/supervisor is remote: each call can provision or reuse a hosted Linux sandbox, and filesystem capability is part of that environment rather than a separate filesystem tool type. [antigravity-managed-agent]{3}

## Message vs command / no-grant informational replyable content

Within the SDK, there are ordinary agent text responses (`PostTurnHook`, `Step.content`, `ChatResponse` text chunks) that inform the operator without requesting a grant. Those responses are not modeled as a separate replyable operator-control primitive; the operator replies by sending another prompt/turn through `send`/`chat`. [antigravity-sdk-repo]{2}

The explicit agent-initiated replyable paths I found are command-like interaction/tool surfaces: `ASK_QUESTION` maps to `OnInteractionHook` and question-response events, tool confirmations map to boolean grants, and custom function calls require function results. I did not find an SDK type named `Message` distinct from `Command` that carries no-grant replyable content. [antigravity-sdk-repo]{2} [antigravity-managed-agent]{3}

## Spawn/retire of agent instances

The SDK exposes two different spawn concepts:

1. **Session/sidecar spawn:** entering an `Agent` or `LocalConnectionStrategy` starts a local harness process and initializes one conversation; exiting disconnects it. [antigravity-sdk-repo]{2}
2. **Subagent spawn inside a running harness:** `CapabilitiesConfig.enable_subagents`, the `START_SUBAGENT` builtin tool, and `SubagentConfig`/`CustomAgent` configuration enable model-driven subagent invocation. Subagent invocations appear as `START_SUBAGENT` tool calls and are subject to pre/post tool hooks; nested subagents are warned as unsupported. [antigravity-sdk-repo]{2}

I did not find a general SDK operator action to enumerate arbitrary live agent instances or retire a specific subagent instance independently after spawn. Retire/stop is exposed at the session/connection level (`disconnect`) and at active-turn level (`cancel`). [antigravity-sdk-repo]{2}

For desktop/CLI, the home page says Antigravity 2.0 can start agents in projects and orchestrate parallel local subagents, and says the CLI has parallel subagent management, but exact spawn/retire commands for those surfaces were not fetched. [antigravity-home]{1}

## Contradictions

| Sources | Relationship type | Positions |
|---|---|---|
| SDK repo vs managed API docs | Surface scope divergence | The SDK exposes local hooks, local harness sidecar process management, Python host tools, triggers, and conversation APIs; the managed API exposes remote interactions, hosted environments, background polling/cancel, and function-result continuation. These are compatible as different surfaces, not one shared API. [antigravity-sdk-repo]{2} [antigravity-managed-agent]{3} |
| Home page vs fetched CLI content | Evidence gap | The home page names CLI features and links a CLI overview, but the CLI route fetched as an SPA shell rather than substantive CLI docs. This pass cannot ground `agy` command verbs. [antigravity-home]{1} |
| SDK hook README vs local connection implementation | Implementation evolution | The hook README says `PostToolCallHook` fires when the harness reports built-in tool completion; local connection comments say post-tool hook for `STATE_DONE` is now dispatched by the harness via `CallHookRequest`, while local code still tracks errors. This is not a semantic contradiction for consumers, but it matters for implementation-level provenance. [antigravity-sdk-repo]{2} |

## Revisit if

- Canonical `agy` CLI documentation or binary help output is available; this is required to answer CLI-specific operator actions without inferring from SDK behavior.
- Antigravity desktop/2.0 API or automation docs are available; current desktop claims are only product-overview level.
- A newer SDK release changes hook names, adds live model reconfiguration, or exposes explicit session compaction/resume commands beyond config-time `conversation_id` and harness-triggered compaction.
- Managed Interactions API streaming docs are fetched; this brief only used the Antigravity Agent page’s references/examples, not a dedicated SSE event schema.

## Acquisition candidates

- **Antigravity CLI Quick Overview / `agy` help surface.** The home page names the Antigravity CLI and links `Explore the CLI Quick Overview` at `/docs/cli/overview`; fetch a substantive version of that doc or capture `agy --help`/subcommand help from an installed CLI. [antigravity-home]{1}
- **Antigravity SDK Overview docs.** The home page links `Review the SDK Overview` at `/docs/sdk/overview`; the open-source SDK repo was sufficient for internals, but the canonical published overview may clarify intended stable API. [antigravity-home]{1}
- **Antigravity permissions docs.** The home page links security-by-design and granular tool approval gates via `/docs/permissions`; fetching it would ground desktop/CLI approval behavior beyond SDK hooks. [antigravity-home]{1}
- **Gemini API streaming/background interaction event schema.** The managed-agent page points readers to streaming/background interaction docs; fetch those before asserting exact SSE event names. [antigravity-managed-agent]{3}
