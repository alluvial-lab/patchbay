---
provenance: agent-synthesis
updated: 2026-07-03
---

# Codex harness action surface brief

## Scope and source base

[high] This brief covers OpenAI Codex as exposed through `codex app-server`, the Python SDK, and the TypeScript SDK. `codex app-server` is the interface used for rich interfaces such as the Codex VS Code extension, and its protocol is bidirectional JSON-RPC-like messaging over supported transports.[codex-appserver-readme]{1}[codex-appserver-readme]{2} [high] The Python SDK and TypeScript SDK expose overlapping but not identical public surfaces: Python maps closely to app-server threads/turns; TypeScript wraps and spawns the `codex` CLI and exchanges JSONL events over stdio.[codex-python-sdk-api]{1}[codex-typescript-sdk]{1}

## Operator → agent/control actions

### Durable/lifecycle-bearing actions

[high] Codex's app-server primitives are Thread, Turn, and Item: thread APIs create/list/archive conversations, turn APIs drive conversations, and notifications stream progress.[codex-appserver-readme]{3} [high] Durable thread lifecycle actions include `thread/start`, `thread/resume`, `thread/fork`, `thread/list`, `thread/read`, `thread/archive`, `thread/delete`, `thread/unarchive`, `thread/unsubscribe`, and `thread/compact/start`.[codex-appserver-readme]{5}[codex-appserver-protocol]{2}[codex-appserver-protocol]{3} [high] Thread configuration can be set at start and partially updated later with model, cwd, approval reviewer/policy, sandbox/permissions, service tier, reasoning effort, summary, collaboration mode, and personality fields.[codex-appserver-types]{1}[codex-appserver-types]{2}

[high] Turn lifecycle/control actions include `turn/start`, `turn/steer`, and `turn/interrupt`: `turn/start` adds user input and begins generation, `turn/steer` adds user input to an in-flight regular turn, and `turn/interrupt` cancels an in-flight turn by thread/turn id.[codex-appserver-readme]{5}[codex-appserver-protocol]{4} [high] Turn start can also override cwd, workspace roots, approval policy/reviewer, sandbox/permissions, model, service tier, effort, summary, personality, output schema, and collaboration mode for the turn.[codex-appserver-types]{3} [high] Steering is explicitly tied to an expected active turn id, while interruption is tied to thread id plus turn id.[codex-appserver-types]{4}

[high] Python SDK equivalents include `thread_start`, `thread_list`, `thread_resume`, `thread_fork`, `thread_archive`, `thread_unarchive`, `models`, thread `run`, thread `turn`, thread `read`, `set_name`, `compact`, and turn-handle `steer`, `interrupt`, `stream`, and `run`.[codex-python-sdk-api]{2}[codex-python-sdk-api]{4}[codex-python-sdk-api]{6} [high] The Python SDK also exposes sync and async variants of these controls, and its turn result carries id, status, error, timestamps, final_response, items, and usage.[codex-python-sdk-api]{2}[codex-python-sdk-api]{3}[codex-python-sdk-api]{5}

### Ephemeral/payload actions

[high] `turn/start` and `turn/steer` carry `UserInput`, whose typed payloads include text, image, local image, skill, and mention entries.[codex-appserver-types]{5} [high] `thread/inject_items` appends raw Responses API items to loaded thread history without starting a user turn.[codex-appserver-readme]{5} [high] `review/start` starts an automated reviewer and streams review-mode items plus a final assistant review message.[codex-appserver-readme]{5}

[high] Standalone execution utilities include `command/exec` plus write/resize/terminate controls for a sandboxed server-side command session, and `process/spawn` plus write/resize/kill controls for an experimental unsandboxed host process session.[codex-appserver-readme]{5}[codex-appserver-protocol]{5} [high] `command/exec` is explicitly defined as a standalone command in the server sandbox without creating a thread or turn.[codex-appserver-types]{9} [high] `process/spawn` is explicitly defined as a standalone unsandboxed process on the app-server host, with connection-scoped output and exit notifications.[codex-appserver-readme]{9}[codex-appserver-types]{10}

### Read-only/query and environment-management actions

[high] Read/query surfaces include `thread/list`, `thread/read`, `thread/loaded/list`, `model/list`, `modelProvider/capabilities/read`, filesystem read/metadata/directory methods, plugin/skills/hooks list/read methods, permission-profile list, experimental-feature list, and remote-control status read.[codex-appserver-readme]{5}[codex-appserver-protocol]{3}[codex-appserver-protocol]{5} [high] Python SDK exposes `models()` as a public model-list method.[codex-python-sdk-api]{2}

### Approvals and user input requested by the agent/server

[high] When configured actions require approval, app-server sends server-initiated JSON-RPC requests to the client, and the client responds with a decision; command approvals support accept, acceptForSession, policy-amendment acceptances, decline, and cancel.[codex-appserver-readme]{8} [high] The typed server-request registry includes command-execution approval, file-change approval, tool user-input request, MCP elicitation, permissions approval, dynamic tool call, ChatGPT auth-token refresh, attestation generation, and current-time read.[codex-appserver-protocol]{6}[codex-appserver-protocol]{7} [high] The type definitions confirm command and file-change approval decision enums and tool user-input question/answer payloads.[codex-appserver-types]{7}[codex-appserver-types]{8}

## Agent → operator events

[high] App-server emits a server-originated notification stream for thread lifecycle, turn lifecycle, and item lifecycle; after thread start/resume, clients keep reading notifications for thread, turn, and item events.[codex-appserver-readme]{6}[codex-appserver-protocol]{8} [high] Turn-level events include `turn/started`, `turn/completed`, `turn/diff/updated`, `turn/plan/updated`, model safety buffering, model reroute, model verification, moderation metadata, and error events.[codex-appserver-readme]{6}[codex-appserver-protocol]{9} [high] Item lifecycle is `item/started` → zero or more item-specific deltas → `item/completed`.[codex-appserver-readme]{6}

[high] Item payloads include `userMessage`, `agentMessage`, `reasoning`, `commandExecution`, `fileChange`, MCP/dynamic/collab tool calls, web search, image view, sleep, review-mode entries, and context compaction.[codex-appserver-types]{6} [high] Agent textual output appears as `agentMessage` items and streams incrementally through `item/agentMessage/delta`.[codex-appserver-readme]{7} [high] Command/process streams have output delta notifications and process/session exit notifications.[codex-appserver-protocol]{9}[codex-appserver-readme]{9}

[high] The TypeScript SDK event union is smaller than app-server's full event surface: it exposes `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.started`, `item.updated`, `item.completed`, and `error` events.[codex-typescript-sdk]{7} [high] TypeScript `runStreamed()` exposes an async generator for intermediate tool calls, streaming responses, and file-change notifications, while `run()` buffers to final response/items/usage.[codex-typescript-sdk]{3}[codex-typescript-sdk]{9}

## Durability classification

| Action/event family | Classification | Basis |
|---|---|---|
| `thread/start`, `thread/resume`, `thread/fork`, `thread/archive`, `thread/delete`, `thread/unarchive`, `thread/compact/start` | Durable/lifecycle-bearing | These create, resume, branch, persistently archive/delete/unarchive, or compact thread history.[codex-appserver-readme]{5} |
| `thread/list`, `thread/read`, `thread/loaded/list`, model/capability/list methods | Read-only/query | These page/list/read stored or loaded state and model/capability metadata.[codex-appserver-readme]{5}[codex-appserver-protocol]{3} |
| `thread/settings/update`, turn-start config overrides | Lifecycle-adjacent configuration | Settings update queues next-turn settings without adding transcript items; turn-start config fields alter turn/subsequent settings.[codex-appserver-readme]{5}[codex-appserver-types]{2}[codex-appserver-types]{3} |
| `turn/start` | Durable/payload-bearing | It adds user input to a thread and begins generation; user input becomes a `userMessage` item.[codex-appserver-readme]{5}[codex-appserver-readme]{7} |
| `turn/steer` | Ephemeral in-flight payload, persisted as user input | It adds input to an active regular turn without a new turn and echoes optional client id on the corresponding `userMessage` item.[codex-appserver-readme]{5}[codex-appserver-readme]{7} |
| `turn/interrupt` | Lifecycle-bearing cancellation | It requests cancellation of an in-flight turn and the turn finishes with interrupted status.[codex-appserver-readme]{5} |
| Approval responses | Ephemeral gate decision with possible session/policy durability | Decisions resume/decline current work; command decisions may be accept-for-session or policy amendment.[codex-appserver-readme]{8}[codex-appserver-types]{7} |
| `command/exec` | Ephemeral utility execution | Runs outside thread/turn creation; result/output are tied to command session rather than conversation lifecycle.[codex-appserver-types]{9} |
| `process/spawn` | Ephemeral privileged host process lifecycle | Process handle is connection-scoped; output/exit are connection-scoped and closed connection terminates the process.[codex-appserver-readme]{9}[codex-appserver-types]{10} |
| Notifications | Mostly read-only event stream, some lifecycle terminal facts | Notifications report thread/turn/item state and deltas; item completion is authoritative for execution/result state.[codex-appserver-readme]{6}[codex-appserver-readme]{8} |

## Privileged sidecar/supervisor implications

[high] A Codex-like desktop control surface would typically talk to `codex app-server`, because app-server is the documented interface for rich clients and owns transports, request serialization, notifications, approvals, and host execution utilities.[codex-appserver-readme]{1}[codex-appserver-readme]{2}[codex-appserver-protocol]{1} [high] TypeScript SDK's embedding model also involves a supervising host process because the SDK spawns the `codex` CLI and exchanges JSONL over stdin/stdout; it additionally lets a host control the CLI environment, useful for Electron-like sandboxed hosts.[codex-typescript-sdk]{1}[codex-typescript-sdk]{6}

[high] Host process spawning is privileged relative to the agent conversation: `process/spawn` runs without the Codex sandbox on the app-server host, requires experimental API opt-in, has no sandbox-selection fields, and is connection-scoped.[codex-appserver-readme]{9} [high] `command/exec` is less privileged than `process/spawn` in the documented surface because it runs under the server sandbox and supports sandbox/permission selection.[codex-appserver-types]{9} [high] Filesystem utility methods operate on absolute paths and therefore also belong in a trusted local control-plane boundary rather than inside an adapter-neutral core.[codex-appserver-readme]{5}

## No-grant informational replyable content vs command shape

[medium] Codex has first-class message items: `userMessage` and `agentMessage`; agent replies stream through `item/agentMessage/delta` and complete as an `agentMessage` item.[codex-appserver-readme]{7}[codex-appserver-types]{6} [medium] The operator reply path observed in the fetched app-server and SDK sources is not a separate generic `message/reply` method; it is `turn/start` for a new turn or `turn/steer` for in-flight input, with typed `UserInput` payloads.[codex-appserver-readme]{5}[codex-appserver-types]{4}[codex-appserver-types]{5}

[medium] Codex does expose replyable server requests that are not approval grants, including `item/tool/requestUserInput`, `mcpServer/elicitation/request`, `currentTime/read`, `attestation/generate`, auth-token refresh, and dynamic tool calls.[codex-appserver-protocol]{7} [medium] These are server-initiated request/response envelopes, not plain informational `agentMessage` content; the fetched type for tool user input is a structured list of questions and answers, and MCP elicitation is documented separately from the agent-message stream.[codex-appserver-types]{8}[codex-appserver-readme]{8}

[medium] Therefore, on the fetched sources, the universal conversation shape is "operator drives with `turn/start`/`turn/steer`; agent replies with item/message events," while non-conversational replyable content is modeled as typed server requests rather than as a generic no-grant informational Message command.[codex-appserver-readme]{4}[codex-appserver-protocol]{7}

## Spawn/retire of agent instances

[medium] The stable operator-facing lifecycle surface includes starting/resuming/forking/deleting/archiving threads, not a stable top-level `agent/spawn` or `agent/retire` method in the fetched app-server method registry.[codex-appserver-protocol]{2}[codex-appserver-protocol]{3} [medium] The docs do show multi-agent/collaboration concepts as item/tool-call surfaces: `collabToolCall` describes `spawn_agent`, `send_input`, `resume_agent`, `wait`, and `close_agent`, and its item type carries sender/receiver thread ids and spawned-agent metadata.[codex-appserver-readme]{11}[codex-appserver-types]{6} [medium] Thread list/read surfaces are aware of spawned descendants, and archive/delete attempt to include spawned descendant rollouts, so spawned-agent lifecycle has persisted thread edges even though operator spawn/retire is not exposed as a simple top-level stable method in the fetched registry.[codex-appserver-readme]{5}

[high] `process/spawn` should not be conflated with agent-instance spawn: it starts an unsandboxed host process by argv and reports output/exit by process handle.[codex-appserver-readme]{9}[codex-appserver-types]{10}

## Comparison lens notes

[medium] Relative to a control surface that separates inbound controls from outbound hooks, Codex app-server combines both over one bidirectional JSON-RPC-like channel: client requests carry operator/control actions; server requests carry approvals/elicitations/tool calls; server notifications carry lifecycle, item, output, model, warning, and error events.[codex-appserver-readme]{2}[codex-appserver-protocol]{1}[codex-appserver-protocol]{6}[codex-appserver-protocol]{8} [medium] Codex's app-server surface is richer than a minimal terminal SDK because it includes local filesystem and process-control helpers, remote-control status/pairing, plugin/skills surfaces, and typed approval requests.[codex-appserver-readme]{5}[codex-appserver-protocol]{7}

## Disconfirming analysis

- Searched the fetched app-server method registry for a generic message/reply action. The registry entries found for conversation input were `turn/start` and `turn/steer`; replyable server-originated cases were typed requests such as approvals, tool user input, MCP elicitation, dynamic tool call, auth refresh, attestation, and current time.[codex-appserver-protocol]{4}[codex-appserver-protocol]{7}
- Searched the fetched surfaces for top-level operator `agent/spawn` / `agent/retire`. The fetched registry shows thread lifecycle methods and experimental `process/spawn`; agent spawning appears as a `collabToolCall` item/tool concept rather than a stable operator method.[codex-appserver-protocol]{2}[codex-appserver-protocol]{5}[codex-appserver-readme]{11}
- Checked whether TypeScript SDK carries full app-server controls. The fetched README and source show a CLI-spawning JSONL wrapper with run/runStreamed/resume and a smaller event union, so it does not disconfirm app-server's richer surface; it indicates a narrower SDK layer.[codex-typescript-sdk]{1}[codex-typescript-sdk]{7}[codex-typescript-sdk]{8}

## Contradictions

No direct contradictions were found among fetched sources. The main divergence is surface breadth: app-server exposes a large JSON-RPC control/event protocol, Python SDK exposes a higher-level thread/turn subset with stream/steer/interrupt controls, and TypeScript SDK wraps the CLI with a smaller event vocabulary.[codex-appserver-protocol]{9}[codex-python-sdk-api]{6}[codex-typescript-sdk]{7}

## Revisit if

- OpenAI publishes stable app-server schemas for a newer Codex version; generated schema output may be more precise than repository prose for exact method availability.[codex-appserver-readme]{5}
- The experimental collaboration/multi-agent APIs stabilize; current evidence for `spawn_agent`/`close_agent` is item/tool-call oriented, while the stable operator registry does not expose a top-level agent spawn/retire method.[codex-appserver-readme]{11}[codex-appserver-protocol]{2}
- A desktop app API distinct from `codex app-server` becomes publicly documented; this brief treats app-server as the canonical rich-interface control surface because the fetched documentation names it that way.[codex-appserver-readme]{1}

## Acquisition candidates

- Enriching: generated `codex app-server generate-json-schema --experimental` output from the exact Codex binary version under evaluation. The app-server README states schema generation is version-specific and can include experimental methods/fields.[codex-appserver-readme]{5}
