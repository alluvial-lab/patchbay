---
source_handle: codex-appserver-types
fetched: 2026-07-03
source_url: https://github.com/openai/codex/tree/main/codex-rs/app-server-protocol/src/protocol/v2
provenance: source-direct
---

# Per-source attestation: codex-appserver-types

## Structural metadata

- Source kind: Rust app-server v2 protocol type definitions.
- Local fetched copies read:
  - `/tmp/codex-src/codex-rs/app-server-protocol/src/protocol/v2/thread.rs`
  - `/tmp/codex-src/codex-rs/app-server-protocol/src/protocol/v2/turn.rs`
  - `/tmp/codex-src/codex-rs/app-server-protocol/src/protocol/v2/item.rs`
  - `/tmp/codex-src/codex-rs/app-server-protocol/src/protocol/v2/command_exec.rs`
  - `/tmp/codex-src/codex-rs/app-server-protocol/src/protocol/v2/process.rs`
- Scope observed: params and response fields for thread and turn methods; item union; approval decisions; user-input request shape; command/process execution params.

## Paraphrased source summary

The v2 type definitions give parameter and payload shapes for app-server methods and events. Thread start/settings params carry model, model provider, cwd, approval policy, approvals reviewer, sandbox/permissions, service tier, reasoning effort, personality, and environment-related settings. Turn start params carry input plus per-turn overrides for cwd, runtime roots, approval policy, approvals reviewer, sandbox/permissions, model, service tier, effort, summary, personality, and output schema. Turn steering requires an expected active turn id. Turn interruption requires thread and turn ids. `ThreadItem` includes user and agent messages, reasoning, command execution, file change, MCP/dynamic/collab tool calls, web search, image view, sleep, review, and context-compaction items. Command and file approvals have accept/decline/cancel variants, with command approvals adding session and policy-amendment acceptances. Process-spawn params are standalone, connection-scoped, and not sandbox-configured.

## Key passages with source-internal anchors

1. `ThreadStartParams` carries model, model_provider, service tier, cwd, approval policy, approvals reviewer, sandbox, permissions, config, instructions, personality, ephemeral, environments, dynamic tools, and raw events flags. Anchors: `thread.rs` lines 51-146.

> pub struct ThreadStartParams {
>     pub model: Option<String>,
>     pub model_provider: Option<String>,
>     ...
>     pub cwd: Option<String>,
>     ...
>     pub approval_policy: Option<AskForApproval>,
>     ...
>     pub approvals_reviewer: Option<ApprovalsReviewer>,
>     pub sandbox: Option<SandboxMode>,
>     ...
>     pub permissions: Option<String>,
>     pub config: Option<HashMap<String, JsonValue>>,
>     ...
>     pub base_instructions: Option<String>,
>     pub developer_instructions: Option<String>,
>     pub personality: Option<Personality>,
>     ...
>     pub ephemeral: Option<bool>,
>     ...
>     pub environments: Option<Vec<TurnEnvironmentParams>>,
>     ...
>     pub dynamic_tools: Option<Vec<DynamicToolSpec>>,
>     ...
>     pub experimental_raw_events: bool,
> }

2. `ThreadSettingsUpdateParams` supports partial updates for loaded threads, including cwd, approval policy, approvals reviewer, sandbox/permissions, model, service tier, effort, summary, collaboration mode, and personality. Anchors: `thread.rs` lines 208-278.

> pub struct ThreadSettingsUpdateParams {
>     pub thread_id: String,
>     pub cwd: Option<PathBuf>,
>     pub approval_policy: Option<AskForApproval>,
>     pub approvals_reviewer: Option<ApprovalsReviewer>,
>     pub sandbox_policy: Option<SandboxPolicy>,
>     pub permissions: Option<String>,
>     pub model: Option<String>,
>     pub service_tier: Option<Option<String>>,
>     pub effort: Option<ReasoningEffort>,
>     pub summary: Option<ReasoningSummary>,
>     pub collaboration_mode: Option<CollaborationMode>,
>     pub personality: Option<Personality>,
> }

3. `TurnStartParams` takes thread id, client user message id, input, and per-turn overrides including cwd, workspace roots, approval policy, approvals reviewer, sandbox/permissions, model, service tier, effort, summary, personality, output schema, and collaboration mode. Anchors: `turn.rs` lines 65-156.

> pub struct TurnStartParams {
>     pub thread_id: String,
>     pub client_user_message_id: Option<String>,
>     pub input: Vec<UserInput>,
>     ...
>     pub cwd: Option<PathBuf>,
>     pub runtime_workspace_roots: Option<Vec<AbsolutePathBuf>>,
>     pub approval_policy: Option<AskForApproval>,
>     pub approvals_reviewer: Option<ApprovalsReviewer>,
>     pub sandbox_policy: Option<SandboxPolicy>,
>     pub permissions: Option<String>,
>     pub model: Option<String>,
>     pub service_tier: Option<Option<String>>,
>     pub effort: Option<ReasoningEffort>,
>     pub summary: Option<ReasoningSummary>,
>     pub personality: Option<Personality>,
>     pub output_schema: Option<JsonValue>,
>     pub collaboration_mode: Option<CollaborationMode>,
> }

4. `TurnSteerParams` requires target thread id, input, and expected active turn id. `TurnInterruptParams` requires thread id and turn id. Anchors: `turn.rs` lines 174-207.

> pub struct TurnSteerParams {
>     pub thread_id: String,
>     pub client_user_message_id: Option<String>,
>     pub input: Vec<UserInput>,
>     ...
>     pub expected_turn_id: String,
> }
>
> pub struct TurnInterruptParams {
>     pub thread_id: String,
>     pub turn_id: String,
> }

5. User input is a tagged union including text, image, local image, skill, and mention. Anchors: `turn.rs` lines 254-326.

> pub enum UserInput {
>     Text { text: String, text_elements: Vec<TextElement> },
>     Image { detail: Option<ImageDetail>, url: String },
>     LocalImage { path: AbsolutePathBuf },
>     Skill { name: String, path: AbsolutePathBuf },
>     Mention { name: String, path: AbsolutePathBuf },
> }

6. `ThreadItem` includes `UserMessage`, `AgentMessage`, `Reasoning`, `CommandExecution`, `FileChange`, MCP/dynamic/collab tool calls, web search, image view, sleep, review mode, and context compaction. Anchors: `item.rs` lines 162-352.

> pub enum ThreadItem {
>     UserMessage { id: String, client_id: Option<String>, content: Vec<UserInput> },
>     HookPrompt { ... },
>     AgentMessage { id: String, text: String, phase: Option<MessagePhase>, ... },
>     Plan { id: String, text: String },
>     Reasoning { ... },
>     CommandExecution { ... },
>     FileChange { ... },
>     McpToolCall { ... },
>     DynamicToolCall { ... },
>     CollabAgentToolCall { ... },
>     WebSearch { ... },
>     ImageView { ... },
>     Sleep { ... },
>     EnteredReviewMode { ... },
>     ExitedReviewMode { ... },
>     ContextCompaction { ... },
> }

7. Command and file-change approval decisions are explicit enums. Command approvals include accept, acceptForSession, acceptWithExecpolicyAmendment, applyNetworkPolicyAmendment, decline, and cancel. File-change approvals include accept, acceptForSession, decline, and cancel. Anchors: `item.rs` lines 49-93.

> pub enum CommandExecutionApprovalDecision {
>     Accept,
>     AcceptForSession,
>     AcceptWithExecpolicyAmendment { execpolicy_amendment: ExecPolicyAmendment },
>     ApplyNetworkPolicyAmendment { network_policy_amendment: NetworkPolicyAmendment },
>     Decline,
>     Cancel,
> }
>
> pub enum FileChangeApprovalDecision {
>     Accept,
>     AcceptForSession,
>     Decline,
>     Cancel,
> }

8. Tool user-input request params carry thread id, turn id, item id, questions, and optional timeout; responses map question ids to answers. Anchors: `item.rs` lines 1604-1651.

> pub struct ToolRequestUserInputQuestion {
>     pub id: String,
>     pub header: String,
>     pub prompt: String,
>     pub is_secret: bool,
>     pub options: Option<Vec<ToolRequestUserInputOption>>,
> }
> pub struct ToolRequestUserInputParams {
>     pub thread_id: String,
>     pub turn_id: String,
>     pub item_id: String,
>     pub questions: Vec<ToolRequestUserInputQuestion>,
>     pub timeout_ms: Option<u64>,
> }
> pub struct ToolRequestUserInputResponse {
>     pub answers: HashMap<String, ToolRequestUserInputAnswer>,
> }

9. `CommandExecParams` runs a standalone command in the server sandbox without creating a thread or turn; it can stream stdin/stdout/stderr, set cwd/env/size, and choose sandbox policy or permission profile. Anchors: `command_exec.rs` lines 16-103.

> /// Run a standalone command (argv vector) in the server sandbox without
> /// creating a thread or turn.
> pub struct CommandExecParams {
>     pub command: Vec<String>,
>     pub process_id: Option<String>,
>     pub tty: bool,
>     pub stream_stdin: bool,
>     pub stream_stdout_stderr: bool,
>     ...
>     pub cwd: Option<PathBuf>,
>     pub env: Option<HashMap<String, Option<String>>>,
>     pub sandbox_policy: Option<SandboxPolicy>,
>     pub permission_profile: Option<String>,
> }

10. `ProcessSpawnParams` spawns a standalone process without a Codex sandbox on the app-server host, with a connection-scoped `processHandle`, absolute cwd, stream controls, env overrides, and PTY size. Anchors: `process.rs` lines 16-85.

> /// Spawn a standalone process (argv vector) without a Codex sandbox on the host
> /// where the app server is running.
> ///
> /// `process/spawn` returns after the process has started and the connection-scoped
> /// `processHandle` has been registered. Process output and exit are reported via
> /// `process/outputDelta` and `process/exited` notifications.
> pub struct ProcessSpawnParams {
>     pub command: Vec<String>,
>     pub process_handle: String,
>     pub cwd: AbsolutePathBuf,
>     pub tty: bool,
>     pub stream_stdin: bool,
>     pub stream_stdout_stderr: bool,
>     ...
>     pub env: Option<HashMap<String, Option<String>>>,
>     pub size: Option<ProcessTerminalSize>,
> }
