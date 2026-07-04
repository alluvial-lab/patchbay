---
source_handle: claude-code-sdk-client
fetched: 2026-07-03
source_url: https://raw.githubusercontent.com/anthropics/claude-agent-sdk-python/main/src/claude_agent_sdk/client.py
provenance: source-direct
---

# Attestation: Claude Agent SDK Python client

## Paraphrased summary

The Python `ClaudeSDKClient` is a bidirectional, stateful interface for interactive Claude Code conversations. It exposes methods to connect, send prompt/message streams, receive parsed messages, interrupt, set permission mode, set model, rewind files, reconnect/toggle MCP servers, stop tasks, query MCP status/context usage/server info, receive a single response through result, and disconnect.

## Key passages

1. **Client purpose.** The class docstring says `ClaudeSDKClient` supports bidirectional interactive conversations, streaming, interrupts, dynamic message sending, stateful context, and session management; it is intended for chat UIs, interactive debugging, real-time applications, and cases needing interrupt capabilities. Source anchor: lines 26-55.

2. **Connection and session resume.** `connect()` can be called with a prompt or message stream; it validates session store options, materializes resume/continue from a session store into a temporary config directory, and then connects to the CLI transport. Source anchor: lines 99-147.

3. **Permission callback prerequisites.** During connection, if `can_use_tool` is configured, the client requires streaming mode and disallows combining `can_use_tool` with `permission_prompt_tool_name`; it automatically sets `permission_prompt_tool_name` to `stdio` for the control protocol. Source anchor: lines 153-175.

4. **Receiving messages.** `receive_messages()` parses raw data from the query object into SDK `Message` objects and yields them. Source anchor: lines 271-281.

5. **Sending messages.** `query()` sends a new request in streaming mode; string prompts are wrapped as a JSON user message with `type: "user"`, user role/content, no parent tool use id, and session id, while async iterable prompts are written message-by-message with a session id inserted if absent. Source anchor: lines 283-311.

6. **Interrupt.** `interrupt()` sends an interrupt signal and only works in streaming mode. Source anchor: lines 313-318.

7. **Dynamic permission mode.** `set_permission_mode()` changes permission mode during a conversation, only in streaming mode, with valid values including `default`, `acceptEdits`, `plan`, `bypassPermissions`, `dontAsk`, and `auto`. Source anchor: lines 319-344.

8. **Dynamic model.** `set_model()` changes the AI model during a conversation, only in streaming mode. Source anchor: lines 346-368.

9. **File rewind.** `rewind_files()` rewinds tracked files to the state at a specific user message and requires `enable_file_checkpointing=True` plus replaying user messages to obtain the `UserMessage.uuid`. Source anchor: lines 370-400.

10. **MCP live control and queries.** `reconnect_mcp_server()` retries failed/disconnected MCP servers; `toggle_mcp_server()` enables/disables a server and removes/re-adds its tools; `get_mcp_status()` returns live server status including name, status, server info, error, config, scope, and tools. Source anchor: lines 402-504.

11. **Task stop.** `stop_task()` stops a running task by id, and after resolving, a `task_notification` system message with status `stopped` is expected in the stream. Source anchor: lines 450-471.

12. **Read-only context and server info.** `get_context_usage()` returns the same data as `/context`, including token categories, total/max tokens, percentage, model, MCP tool breakdown, memory files, and agents; `get_server_info()` returns initialization info such as available commands, current/available output styles, and server capabilities. Source anchor: lines 506-565.

13. **Response iterator boundary.** `receive_response()` yields messages until and including a `ResultMessage`, then terminates; if no result arrives, it continues indefinitely. Source anchor: lines 567-607.
