---
source_handle: claude-code-sessions
fetched: 2026-07-03
source_url: https://code.claude.com/docs/en/agent-sdk/sessions.md
provenance: source-direct
---

# Attestation: Claude Agent SDK sessions

## Paraphrased summary

The sessions guide explains Claude Agent SDK session persistence and lifecycle operations: sessions accumulate prompts, tool calls, tool results, and responses, are written to disk automatically, and can be continued, resumed by ID, or forked. It documents automatic session management, capturing session IDs, resume/fork options, local transcript storage, cross-host caveats, and session enumeration/mutation APIs.

## Key passages

1. **Session definition and persistence.** A session is the conversation history accumulated while the agent works; it contains prompt, every tool call, every tool result, and every response, and the SDK writes it to disk automatically for later return. Source anchor: lines 9-11.

2. **Return semantics.** Returning to a session gives the agent prior context: files read, analysis performed, decisions made; use cases include follow-ups, recovering from interruption, and branching to alternatives. Source anchor: line 11.

3. **Choose approach.** The guide maps one-shot tasks to a single `query()`, multi-turn chat in one process to `ClaudeSDKClient` or TypeScript `continue`, process restart to `continue_conversation`/`continue`, specific past session to `resume`, alternative exploration to fork, and stateless TypeScript tasks to disabling persistence. Source anchor: lines 19-30.

4. **Continue/resume/fork.** Continue finds the most recent session in the current directory; resume takes a specific session ID; fork creates a new session starting with a copy of the original history while leaving the original unchanged. Source anchor: lines 32-41.

5. **Automatic session management.** Python `ClaudeSDKClient` handles session IDs internally so each `client.query()` continues the same session; TypeScript uses `continue: true` on subsequent `query()` calls to pick up the most recent session. Source anchor: lines 43-135.

6. **Capture session ID.** Resume and fork require a session ID; it can be read from the result message's `session_id`, and TypeScript also exposes it on the init `SystemMessage` while Python nests it in `SystemMessage.data`. Source anchor: lines 140-188.

7. **Resume by ID.** Passing a session ID to `resume` returns to that specific session with full context, useful after limit errors or process restarts. Source anchor: lines 192-236.

8. **Fork.** Forking with `resume` and `fork_session` creates a distinct session id that shares prior history but diverges; it branches conversation history, not filesystem state. Source anchor: lines 244-320.

9. **Storage/cwd caveat.** Sessions are stored under `~/.claude/projects/<encoded-cwd>/*.jsonl` or under `$CLAUDE_CONFIG_DIR/projects/<encoded-cwd>/*.jsonl`; mismatched cwd is a common cause of a resume call returning fresh context. Source anchor: line 239.

10. **Session list and mutation APIs.** The SDKs expose functions for enumerating sessions and messages (`listSessions`/`getSessionMessages` and Python equivalents) and for lookup/mutation (`get_session_info`, `rename_session`, `tag_session`, and TypeScript equivalents). Source anchor: lines 329-331.
