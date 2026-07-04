---
source_handle: codex-appserver-readme
fetched: 2026-07-03
source_url: https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md
provenance: source-direct
---

# Per-source attestation: codex-appserver-readme

## Structural metadata

- Source kind: repository documentation for `codex app-server`.
- Local fetched copy read at: `/tmp/codex-src/codex-rs/app-server/README.md`.
- Scope observed: protocol, lifecycle overview, API overview, turn events, approvals, process lifecycle execution, experimental API gating.

## Paraphrased source summary

This README describes `codex app-server` as the Codex interface used by rich interfaces, including the Codex VS Code extension. It documents a bidirectional JSON-RPC-like protocol over stdio, websocket, or unix socket transports; thread, turn, and item primitives; methods to start/resume/fork/list/read/archive/delete/compact threads; methods to start/steer/interrupt turns; events for thread/turn/item lifecycles and streamed deltas; server-initiated approval and elicitation requests; and command/process/filesystem utility surfaces.

## Key passages with source-internal anchors

1. `codex app-server` is named as the interface for rich interfaces, including the Codex VS Code extension. Anchor: line 3.

> `codex app-server` is the interface Codex uses to power rich interfaces such as the [Codex VS Code extension](https://marketplace.visualstudio.com/items?itemName=openai.chatgpt).

2. The protocol is bidirectional JSON-RPC-like messaging. Anchor: line 22.

> Similar to [MCP](https://modelcontextprotocol.io/), `codex app-server` supports bidirectional communication using JSON-RPC 2.0 messages (with the `"jsonrpc":"2.0"` header omitted on the wire).

3. Core primitives are Thread, Turn, and Item, and thread APIs create/list/archive conversations while turn APIs drive conversations and notifications stream progress. Anchors: lines 67-73.

> The API exposes three top level primitives representing an interaction between a user and Codex:
>
> - **Thread**: A conversation between a user and the Codex agent. Each thread contains multiple turns.
> - **Turn**: One turn of the conversation, typically starting with a user message and finishing with an agent message. Each turn contains multiple items.
> - **Item**: Represents user inputs and agent outputs as part of the turn, persisted and used as the context for future conversations. Example items include user message, agent reasoning, agent message, shell command, file edit, etc.
>
> Use the thread APIs to create, list, or archive conversations. Drive a conversation with turn APIs and stream progress via turn notifications.

4. Lifecycle overview identifies `thread/start`, `thread/resume`, `thread/fork`, `turn/start`, streaming notifications, and `turn/interrupt`. Anchors: lines 77-81.

> - Start (or resume) a thread: Call `thread/start` to open a fresh conversation. The response returns the thread object and you’ll also get a `thread/started` notification. If you’re continuing an existing conversation, call `thread/resume` with its ID instead. If you want to branch from an existing conversation, call `thread/fork` to create a new thread id with copied history. Like `thread/start`, `thread/fork` also accepts `ephemeral: true` for an in-memory temporary thread.
> - Begin a turn: To send user input, call `turn/start` with the target `threadId` and the user's input. Optional fields let you override model, cwd, sandbox policy or experimental `permissions` profile selection, approval policy, approvals reviewer, etc. This immediately returns the new turn object. The app-server emits `turn/started` when that turn actually begins running.
> - Stream events: After `turn/start`, keep reading JSON-RPC notifications on stdout. You’ll see `item/started`, `item/completed`, deltas like `item/agentMessage/delta`, tool progress, etc. These represent streaming model output plus any side effects (commands, tool calls, reasoning notes).
> - Finish the turn: When the model is done (or the turn is interrupted via making the `turn/interrupt` call), the server sends `turn/completed` with the final turn state and token usage.

5. API overview lists thread lifecycle/read/query operations, settings update, archive/delete/unsubscribe, compaction, turn start/steer/interrupt, review, command execution, process spawning, filesystem methods, and related notifications. Anchors: lines 140-192.

> - `thread/start` — create a new thread; emits `thread/started` ...
> - `thread/resume` — reopen an existing thread by id so subsequent `turn/start` calls append to it. Accepts the same permission override rules as `thread/start`.
> - `thread/fork` — fork an existing thread into a new thread id by copying the stored history ...
> - `thread/list` — page through stored threads ...
> - `thread/read` — read a stored thread by id without resuming it ...
> - `thread/settings/update` — experimental; queue a partial update to a loaded thread’s next-turn settings without starting a turn or adding transcript items ...
> - `thread/archive` — move a thread’s rollout file into the archived directory ...
> - `thread/delete` — hard-delete an active or archived thread and any spawned descendant threads ...
> - `thread/compact/start` — trigger conversation history compaction for a thread ...
> - `turn/start` — add user input to a thread and begin Codex generation; responds with the initial `turn` object and streams `turn/started`, `item/*`, and `turn/completed` notifications ...
> - `turn/steer` — add user input to an already in-flight regular turn without starting a new turn ...
> - `turn/interrupt` — request cancellation of an in-flight turn by `(thread_id, turn_id)` ...
> - `command/exec` — run a single command under the server sandbox without starting a thread/turn ...
> - `process/spawn` — experimental; spawn a standalone process without the Codex sandbox on the host where the app server is running ...
> - `fs/readFile` — read an absolute file path and return `{ dataBase64 }`.

6. Turn event stream lifecycle and item event lifecycle are documented. Anchors: lines 1353-1355 and line 1354.

> The app-server streams JSON-RPC notifications while a turn is running. Each turn emits `turn/started` when it begins running and ends with `turn/completed` (final `turn` status). Token usage events stream separately via `thread/tokenUsage/updated`. Clients subscribe to the events they care about, rendering each item incrementally as updates arrive. The per-item lifecycle is always: `item/started` → zero or more item-specific deltas → `item/completed`.

7. `userMessage` and `agentMessage` item shapes are defined; agent text can stream through `item/agentMessage/delta`. Anchors: lines 1370-1371 and 1397-1399.

> - `userMessage` — `{id, clientId, content}` where `clientId` is the optional `clientUserMessageId` supplied to `turn/start` or `turn/steer`, and `content` is a list of user inputs (`text`, `image`, or `localImage`).
> - `agentMessage` — `{id, text}` containing the accumulated agent reply.
>
> - `item/agentMessage/delta` — appends streamed text for the agent message; concatenate `delta` values for the same `itemId` in order to reconstruct the full reply.

8. Approval requests are server-initiated JSON-RPC requests and require client response; command approval decisions include accept, acceptForSession, amendment choices, decline, and cancel. Anchors: lines 1446-1451.

> Certain actions (shell commands or modifying files) may require explicit user approval depending on the user's config. When `turn/start` is used, the app-server drives an approval flow by sending a server-initiated JSON-RPC request to the client. The client must respond to tell Codex whether to proceed. UIs should present these requests inline with the active turn so users can review the proposed command or diff before choosing.
>
> - Requests include `threadId` and `turnId`—use them to scope UI state to the active conversation.
> - Respond with a single `{ "decision": ... }` payload. Command approvals support `accept`, `acceptForSession`, `acceptWithExecpolicyAmendment`, `applyNetworkPolicyAmendment`, `decline`, or `cancel`. The server resumes or declines the work and ends the item with `item/completed`.

9. `process/spawn` starts unsandboxed processes on the app-server host, is experimental, and emits output/exit notifications; connection closure terminates process sessions. Anchors: lines 1160-1161, 1227-1235.

> Use `process/spawn` to start a standalone argv-based process without the Codex sandbox on the host where the app server is running. The `process/*` API is experimental and requires `initialize.params.capabilities.experimentalApi: true`. The spawn response means the process has started and the `processHandle` is registered; completion is reported later through `process/exited`.
>
> - `process/spawn` is intentionally unsandboxed and does not define sandbox-selection fields such as `sandboxPolicy` or `permissionProfile`.
> - Duplicate active `processHandle` values are rejected on the same connection; the same handle can be reused after the prior process exits.
> - `process/outputDelta` and `process/exited` notifications are connection-scoped. If the originating connection closes, the server terminates the process.

10. Dynamic tools can be enabled on `thread/start` and use `item/tool/call` request/response flow. Anchor: line 1557.

> `dynamicTools` on `thread/start` and the corresponding `item/tool/call` request/response flow are experimental APIs. To enable them, set `initialize.params.capabilities.experimentalApi = true`.

11. The item list names `collabToolCall` and identifies its tools as `spawn_agent`, `send_input`, `resume_agent`, `wait`, and `close_agent`. Anchor: line 1377.

> - `collabToolCall` — `{id, tool, status, senderThreadId, receiverThreadId?, newThreadId?, prompt?, agentStatus?}` describing collab tool calls (`spawn_agent`, `send_input`, `resume_agent`, `wait`, `close_agent`); `status` is `inProgress`, `completed`, or `failed`.
