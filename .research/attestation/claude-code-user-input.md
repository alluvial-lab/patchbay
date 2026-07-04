---
source_handle: claude-code-user-input
fetched: 2026-07-03
source_url: https://code.claude.com/docs/en/agent-sdk/user-input.md
provenance: source-direct
---

# Attestation: Claude Agent SDK user input and approvals

## Paraphrased summary

The user-input guide documents the SDK's operator-presence surfaces for permissions and clarifying questions. Claude can pause in-loop to request permission for a tool or ask structured clarifying questions through the `AskUserQuestion` tool. Applications surface those requests, collect a user decision or answer, and return allow/deny or answer payloads through the `canUseTool` callback. The guide distinguishes this paused in-loop flow from normal conversation turns, where Claude finishes and waits for a next message.

## Key passages

1. **Two input-request cases.** Claude requests user input when it needs tool permission or when it has clarifying questions via the `AskUserQuestion` tool; both trigger `canUseTool` and pause execution until the application returns a response, unlike normal turns where Claude finishes and waits for a next message. Source anchor: lines 9-11.

2. **Clarifying-question authority boundary.** For clarifying questions, Claude generates the questions and options; the application presents them and returns selections, and cannot add questions to this flow. Source anchor: line 13.

3. **Pending behavior and defer.** The callback may stay pending indefinitely; execution remains paused until the callback returns, and the SDK cancels the wait only if the query is cancelled. If user response may outlive the process, a `defer` hook decision lets the process exit and resume later from the persisted session. Source anchor: line 15.

4. **Detection.** Applications pass a `canUseTool` callback; it fires for tools needing approval and for `AskUserQuestion`, which is identified by `tool_name == "AskUserQuestion"`. If `tools` restricts capabilities, `AskUserQuestion` must be included for the agent to ask clarifying questions. Source anchor: lines 19-49 and 424-446.

5. **No callback for auto-approved tools.** The guide states the callback never fires for auto-approved tools; allow rules and modes like `acceptEdits` or `bypassPermissions` resolve before `canUseTool`, and logic that must observe or gate every tool should use `PreToolUse` hooks. Source anchor: line 49.

6. **Permission callback payload.** Tool approval callbacks receive tool name, tool input, and extra context including suggestions and cancellation signal; input contents vary by tool. Source anchor: lines 54-64.

7. **Allow/deny responses.** The response table shows allow (`PermissionResultAllow` / `{behavior: "allow", updatedInput}`) and deny (`PermissionResultDeny` / `{behavior: "deny", message}`); when allowing, pass original or modified input, and when denying, provide a message Claude sees and may adapt to. Source anchor: lines 202-211.

8. **Approval variants.** The guide describes approve as-is, approve with modified input, approve-and-remember by returning a suggested permission rule such as one written to `.claude/settings.local.json`, reject, and reject with guidance that Claude reads and uses to decide a different approach. Source anchor: lines 233-408.

9. **Streaming input for redirection.** For a complete change of direction rather than a nudge attached to a denied tool, the guide directs applications to send a new instruction through streaming input. Source anchor: line 408.

10. **Question shape and answer shape.** `AskUserQuestion` input contains a `questions` array; each question has `question`, short `header`, options, and multi-select metadata. The response passes original `questions` plus `answers` keyed by question text, with optional freeform `response` when the UI lets the user type a general reply. Source anchor: lines 482-636.

11. **Limitations.** `AskUserQuestion` is not currently available in subagents spawned through the Agent tool, and each call supports one to four questions with two to four options each. Source anchor: lines 828-831.

12. **Other input surfaces.** The guide names streaming input for chat interfaces/follow-up messages during long-running operations and custom tools for structured input, forms/wizards, external approval systems, and domain-specific interactions. Source anchor: lines 833-855.
