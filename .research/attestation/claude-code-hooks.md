---
source_handle: claude-code-hooks
fetched: 2026-07-03
source_url: https://code.claude.com/docs/en/hooks.md
provenance: source-direct
---

# Attestation: Claude Code hooks reference

## Paraphrased summary

The hooks reference defines Claude Code hooks as user-defined command, HTTP, MCP-tool, prompt, or agent handlers that run at lifecycle points and receive JSON event context. It enumerates hook events across session, turn, tool, subagent, task, worktree, compaction, notification, elicitation, and session-end surfaces. Hook output can add context, block some actions, alter tool inputs/results, route permission decisions, or do side effects depending on event. Some hook output is shown to Claude or the user; some events have no decision control. Hooks can be configured in settings and inspected through a read-only `/hooks` menu.

## Key passages

1. **Hook definition and transport.** The reference says hooks are user-defined shell commands, HTTP endpoints, or LLM prompts that execute automatically at lifecycle points; when an event fires and matches, Claude Code passes JSON context to the handler, with command hooks receiving stdin and HTTP hooks receiving POST bodies, and the handler can inspect input and optionally return a decision. Source anchor: “Hook lifecycle,” lines 13-18.

2. **Lifecycle-event inventory.** The summary table names lifecycle events including `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PostToolUseFailure`, `Notification`, `SubagentStop`, `Stop`, `StopFailure`, `FileChanged`, `PreCompact`, `Elicitation`, and `SessionEnd`; surrounding lifecycle diagram text additionally names `Setup`, `UserPromptExpansion`, `PermissionDenied`, `PostToolBatch`, `SubagentStart/Stop`, `TaskCreated`, `TaskCompleted`, `WorktreeCreate`, `WorktreeRemove`, `MessageDisplay`, `ConfigChange`, `PostCompact`, `InstructionsLoaded`, and `CwdChanged`. Source anchor: “Hook lifecycle” table and diagram alt text, lines 21-64.

3. **Matcher surface.** Matchers filter events by tool name, session-start source (`startup`, `resume`, `clear`, `compact`), session-end reason (`clear`, `resume`, `logout`, `prompt_input_exit`, `bypass_permissions_disabled`, `other`), notification type (`permission_prompt`, `idle_prompt`, `auth_success`, `elicitation_dialog`, `elicitation_complete`, `elicitation_response`, `agent_needs_input`, `agent_completed`), compaction trigger (`manual`, `auto`), subagent type, and stop-failure error type; several events ignore matchers and always fire. Source anchor: “Matcher patterns,” lines 209-227 and 251.

4. **Handler kinds.** Hook handlers can be command, HTTP, MCP tool, prompt, or agent handlers; command hooks use stdin/stdout/exit codes, HTTP hooks POST the event JSON and use JSON response bodies, and MCP tool hooks call configured MCP tools. Source anchor: “Hook handler fields,” lines 302-444 and “Prompt and agent hook fields,” lines 468-477.

5. **Common input fields.** Common hook input includes session identifiers, transcript path, current working directory, optional prompt id, permission mode, active effort level, and agent information; only `SessionStart` can receive a `model` field and it is not guaranteed. Source anchor: “Common input fields,” lines 597-618.

6. **Exit-code controls.** Exit code 2 is a blocking error for selected events: `PreToolUse` blocks a tool call, `PermissionRequest` denies permission, `UserPromptSubmit` blocks prompt processing, `Stop` prevents Claude from stopping, `SubagentStop` prevents the subagent from stopping, `PostToolBatch` stops the agentic loop before the next model call, and `PreCompact` blocks compaction; for other events it is non-blocking, user-only, or ignored. Source anchor: “Exit code output,” lines 643-696.

7. **Context injection.** Hook stdout/additionalContext can add context for Claude at session/subagent start, prompt submission/expansion, tool events, post-tool batches, stop/subagent stop, and similar points; injected text is saved in the transcript, and resuming replays saved text instead of re-running past hooks, while `SessionStart` hooks run again on resume with `source: "resume"`. Source anchor: “Add context for Claude,” lines 787-821.

8. **Decision-control matrix.** Decision control differs by event: `PreToolUse` supports permission decisions `allow`, `deny`, `ask`, `defer`; `PermissionRequest` can allow or deny and update input/permissions; `PostToolUse` can replace tool output; `UserPromptSubmit`, `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`, `Stop`, `SubagentStop`, `ConfigChange`, and `PreCompact` use top-level `decision: "block"`; `Elicitation` and `ElicitationResult` support accept/decline/cancel; `SessionStart`, `Setup`, and `SubagentStart` add context and `SessionStart` also accepts `initialUserMessage`, `watchPaths`, `sessionTitle`, and `reloadSkills`; events such as `WorktreeRemove`, `Notification`, `SessionEnd`, `PostCompact`, `InstructionsLoaded`, `StopFailure`, `CwdChanged`, and `FileChanged` have no decision control. Source anchor: “Decision control,” lines 823-846.

9. **PreToolUse input mutation and no implicit approval.** `PreToolUse` hook output can replace a tool’s arguments before it runs; a silent/empty hook has no decision and normal permission flow applies, so a hook can deny the call but staying silent does not approve it. Source anchor: “How a hook resolves,” lines 90-146 and “Decision control,” lines 843-846.

10. **Hooks menu.** Typing `/hooks` opens a read-only browser for configured hooks, showing each event, matcher, type, source file, command/prompt/URL; modification requires editing settings JSON or asking Claude to edit it. Source anchor: “The `/hooks` menu,” lines 566-579.
