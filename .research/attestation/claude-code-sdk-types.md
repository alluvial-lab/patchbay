---
source_handle: claude-code-sdk-types
fetched: 2026-07-03
source_url: https://raw.githubusercontent.com/anthropics/claude-agent-sdk-python/main/src/claude_agent_sdk/types.py
provenance: source-direct
---

# Attestation: Claude Agent SDK Python types

## Paraphrased summary

The Python SDK type definitions expose the structured message stream, options, hook input/output schemas, permission callbacks, session store types, and the SDK control protocol used between SDK and CLI. The source is the renamed `claude-agent-sdk-python` repository; the starting URL for `claude-code-sdk-python` redirected/renamed at GitHub API level, and the source-direct raw file fetched here is the canonical current path.

## Key passages

1. **Permission modes and updates.** The file defines permission modes `default`, `acceptEdits`, `plan`, `bypassPermissions`, `dontAsk`, and `auto`, plus permission updates with destinations such as user settings, project settings, local settings, and memory. Source anchor: comments and definitions around lines 23-139.

2. **Tool permission callback shape.** `ToolPermissionContext` carries permission suggestions, blocked path, decision reason, prompt title/display fields, and tool-use id; permission results are `PermissionResultAllow` with `updated_input` and optional `updated_permissions`, or `PermissionResultDeny` with message and optional interrupt flag; `CanUseTool` receives tool name, input, and context. Source anchor: lines 198-254.

3. **Hook event type subset in SDK.** `HookEvent` includes `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`, `Stop`, `SubagentStop`, `PreCompact`, `Notification`, `SubagentStart`, and `PermissionRequest`. Source anchor: lines 258-269.

4. **Hook input shapes.** Typed hook inputs include tool lifecycle inputs with `tool_name` and `tool_input`, `PostToolUse` with `tool_response`, failure inputs with `error`/`is_interrupt`, `UserPromptSubmit` with prompt, `Stop`/`SubagentStop`, `PreCompact` with trigger/custom instructions, `Notification`, `SubagentStart`, and `PermissionRequest` with tool input and permission suggestions. Source anchor: lines 273-397.

5. **Hook output shapes.** `PreToolUseHookSpecificOutput` supports `permissionDecision` values `allow`, `deny`, `ask`, and `defer`; `PostToolUseHookSpecificOutput` can replace tool output via `updatedToolOutput`/`updatedMCPToolOutput`; `UserPromptSubmit`, `SessionStart`, `Notification`, `SubagentStart`, and `PermissionRequest` each have event-specific outputs. Source anchor: lines 411-490.

6. **Content blocks and messages.** Content block dataclasses include `TextBlock`, `ThinkingBlock`, `ToolUseBlock`, `ToolResultBlock`, server-side tool use/result blocks, and messages include `UserMessage`, `AssistantMessage`, `SystemMessage`, `ResultMessage`, `StreamEvent`, and `RateLimitEvent`. Source anchor: lines 929-1038 and 1218-1318.

7. **Assistant and result metadata.** `AssistantMessage` carries content, model, optional parent tool-use id, error, usage, message id, stop reason, session id, and uuid. `ResultMessage` carries subtype, duration/cost/usage, `is_error`, turn count, session id, stop reason, result text, permission denials, deferred tool use, errors, API error status, and uuid. Source anchor: lines 1025-1038 and 1200-1217.

8. **Task lifecycle events.** The SDK models task started, progress, notification, and updated system messages; task statuses include pending/running/paused/completed/failed/killed, with terminal task statuses also including stopped, and the comments note that a stopped task may arrive as a `TaskUpdatedMessage` with status `killed` and no matching notification. Source anchor: lines 1049-1164.

9. **Deferred tool use.** `DeferredToolUse` records a tool call deferred by a `PreToolUse` hook returning `permissionDecision: "defer"`; the run stops and the result message carries the deferred call so the caller can inspect and decide whether to resume. Source anchor: lines 1186-1197.

10. **Options surface.** `ClaudeAgentOptions` includes tool availability and permission controls (`tools`, `allowed_tools`, `disallowed_tools`, `permission_mode`, `permission_prompt_tool_name`, `can_use_tool`), session controls (`continue_conversation`, `resume`, `session_id`, `fork_session`, `session_store`), model/thinking controls (`model`, `fallback_model`, `thinking`, `effort`), hooks, plugins, agents, skills, cwd, settings, environment, and partial/hook-event streaming toggles. Source anchor: lines 1634-1996.

11. **Control protocol request variants.** The SDK control protocol request union includes `interrupt`, `can_use_tool`, `initialize`, `set_permission_mode`, `hook_callback`, `mcp_message`, `rewind_files`, `mcp_reconnect`, `mcp_toggle`, and `stop_task`, wrapped in `SDKControlRequest` with a request id and responded to by success/error control responses. Source anchor: lines 1997-2098.

12. **Partial and hook events in stream.** `StreamEvent` represents partial assistant stream updates when streaming is enabled, carrying raw Anthropic API stream events; `HookEventMessage` is emitted when `include_hook_events` is true and carries hook-started/hook-response lifecycle data into the SDK message stream. Source anchor: lines 1218-1224 and 1279-1314.
