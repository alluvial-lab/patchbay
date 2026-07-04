---
source_handle: codex-appserver-protocol
fetched: 2026-07-03
source_url: https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/src/protocol/common.rs
provenance: source-direct
---

# Per-source attestation: codex-appserver-protocol

## Structural metadata

- Source kind: Rust protocol definitions for app-server JSON-RPC request, response, and notification unions.
- Local fetched copy read at: `/tmp/codex-src/codex-rs/app-server-protocol/src/protocol/common.rs`.
- Scope observed: client-initiated method registry, server-initiated request registry, server notification registry, experimental method gating markers.

## Paraphrased source summary

This source defines typed app-server protocol unions. `ClientRequest` variants map client/operator-originated methods to params/response types. `ServerRequest` variants are requests initiated by app-server and answered by the client. `ServerNotification` variants are server-originated notifications. The definitions include lifecycle methods (`thread/start`, `thread/resume`, `thread/fork`, `thread/archive`, `thread/delete`, `thread/compact/start`, `turn/start`, `turn/steer`, `turn/interrupt`), read/list methods, process and command utility methods, approval and elicitation requests, and notification names for thread, turn, item, model, command, process, warning, and account events.

## Key passages with source-internal anchors

1. The `ClientRequest` macro documents that it generates requests from client to server. Anchors: lines 188 and 206-209.

> /// Generates an `enum ClientRequest` where each variant is a request that the
> /// Request from the client to the server.
> #[serde(tag = "method", rename_all = "camelCase")]
> pub enum ClientRequest {

2. Client request definitions map thread lifecycle methods. Anchors: lines 476-483.

> ThreadStart => "thread/start" {
>     params: v2::ThreadStartParams,
>     inspect_params: true,
>     serialization: None,
>     response: v2::ThreadStartResponse,
> },
> ThreadResume => "thread/resume" {

3. Client request definitions include `thread/fork`, `thread/archive`, `thread/delete`, `thread/unsubscribe`, settings update, `thread/unarchive`, `thread/compact/start`, and list/read surfaces. Anchors: lines 484-635.

> ThreadFork => "thread/fork" { ... }
> ThreadArchive => "thread/archive" { ... }
> ThreadDelete => "thread/delete" { ... }
> ThreadUnsubscribe => "thread/unsubscribe" { ... }
> ThreadSettingsUpdate => "thread/settings/update" { ... }
> ThreadUnarchive => "thread/unarchive" { ... }
> ThreadCompactStart => "thread/compact/start" { ... }
> ThreadList => "thread/list" { ... }
> ThreadLoadedList => "thread/loaded/list" { ... }
> ThreadRead => "thread/read" { ... }

4. Client request definitions map turn control methods. Anchors: lines 799-815.

> TurnStart => "turn/start" {
>     params: v2::TurnStartParams,
>     inspect_params: true,
>     serialization: thread_id(params.thread_id),
>     response: v2::TurnStartResponse,
> },
> TurnSteer => "turn/steer" { ... }
> TurnInterrupt => "turn/interrupt" { ... }

5. Client request definitions map command and process control methods. Anchors: lines 1051-1101.

> OneOffCommandExec => "command/exec" { ... }
> CommandExecWrite => "command/exec/write" { ... }
> CommandExecTerminate => "command/exec/terminate" { ... }
> CommandExecResize => "command/exec/resize" { ... }
> #[experimental("process/spawn")]
> ProcessSpawn => "process/spawn" { ... }
> ProcessWriteStdin => "process/writeStdin" { ... }
> ProcessKill => "process/kill" { ... }
> ProcessResizePty => "process/resizePty" { ... }

6. The `ServerRequest` macro documents server-to-client requests. Anchors: lines 1193-1212.

> /// Generates an `enum ServerRequest` where each variant is a request that the
> /// server can send to the client along with the corresponding params and
> /// response types.
> /// Request initiated from the server and sent to the client.
> #[serde(tag = "method", rename_all = "camelCase")]
> pub enum ServerRequest {

7. Server requests include command/file approvals, user-input request, MCP elicitation, permissions approval, dynamic tool call, auth token refresh, attestation, and current time. Anchors: lines 1452-1497.

> CommandExecutionRequestApproval => "item/commandExecution/requestApproval" { ... }
> FileChangeRequestApproval => "item/fileChange/requestApproval" { ... }
> ToolRequestUserInput => "item/tool/requestUserInput" { ... }
> McpServerElicitationRequest => "mcpServer/elicitation/request" { ... }
> PermissionsRequestApproval => "item/permissions/requestApproval" { ... }
> DynamicToolCall => "item/tool/call" { ... }
> ChatgptAuthTokensRefresh => "account/chatgptAuthTokens/refresh" { ... }
> AttestationGenerate => "attestation/generate" { ... }
> CurrentTimeRead => "currentTime/read" { ... }

8. The `ServerNotification` macro documents server-to-client notifications. Anchors: lines 1361-1376.

> /// Generates `ServerNotification` enum and helpers, including a JSON Schema
> /// exporter for each notification.
> /// Notification sent from the server to the client.
> #[serde(tag = "method", content = "params", rename_all = "camelCase")]
> pub enum ServerNotification {

9. Server notifications include thread lifecycle, settings, turn lifecycle, item lifecycle, agent-message deltas, command/process deltas, server-request resolution, MCP progress, account updates, remote-control status, filesystem changes, reasoning deltas, model notifications, warnings, and realtime notifications. Anchors: lines 1607-1686.

> ThreadStarted => "thread/started" ...
> ThreadStatusChanged => "thread/status/changed" ...
> ThreadArchived => "thread/archived" ...
> ThreadDeleted => "thread/deleted" ...
> ThreadUnarchived => "thread/unarchived" ...
> ThreadClosed => "thread/closed" ...
> TurnStarted => "turn/started" ...
> TurnCompleted => "turn/completed" ...
> ItemStarted => "item/started" ...
> ItemCompleted => "item/completed" ...
> AgentMessageDelta => "item/agentMessage/delta" ...
> CommandExecOutputDelta => "command/exec/outputDelta" ...
> ProcessOutputDelta => "process/outputDelta" ...
> ProcessExited => "process/exited" ...
> ServerRequestResolved => "serverRequest/resolved" ...
> FsChanged => "fs/changed" ...
> ReasoningSummaryTextDelta => "item/reasoning/summaryTextDelta" ...
> ModelRerouted => "model/rerouted" ...
> Warning => "warning" ...
