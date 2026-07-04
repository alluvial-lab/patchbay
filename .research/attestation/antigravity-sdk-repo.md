---
source_handle: antigravity-sdk-repo
fetched: 2026-07-03
source_url: https://github.com/google-antigravity/antigravity-sdk-python
provenance: source-direct
---

# Per-source attestation: google-antigravity/antigravity-sdk-python

## Structural metadata

- Format fetched: Git repository cloned to `/tmp/antigravity-sdk-python` with `git clone --depth 1`.
- Repository identity from clone URL: `google-antigravity/antigravity-sdk-python`.
- Primary files read: `README.md`, `google/antigravity/agent.py`, `google/antigravity/conversation/conversation.py`, `google/antigravity/types.py`, `google/antigravity/hooks/README.md`, `google/antigravity/hooks/hooks.py`, `google/antigravity/hooks/hook_runner.py`, `google/antigravity/connections/connection.py`, `google/antigravity/connections/local/local_connection.py`, `google/antigravity/connections/local/local_connection_config.py`, `google/antigravity/triggers/README.md`, `google/antigravity/triggers/triggers.py`, `google/antigravity/triggers/trigger_runner.py`, and `google/antigravity/utils/interactive.py`.
- Scope: Python SDK public API, local-harness connection implementation, hooks, policies, triggers, and examples.

## Paraphrased source summary

The repository implements a Python SDK with a high-level `Agent`, a stateful `Conversation`, connection abstractions, a local harness connection, lifecycle hooks, tool policies, triggers, custom tools, MCP configuration, subagent configuration, streaming response wrappers, cancellation, persisted conversation identifiers, and local process management for a compiled harness binary. The SDK is not the Antigravity desktop or TUI implementation; it is a programmatic surface over an Antigravity harness process.

## Key passages with source-internal anchors

### Anchor: `README.md` / `Concepts` / `Simple Agent`

> "The `Agent` class is the easiest way to get started. It manages the full lifecycle — binary discovery, tool wiring, hook registration, and policy defaults — behind a single async context manager."

### Anchor: `README.md` / `Streaming Responses`

> "To stream agent output in real-time ... iterate over the `ChatResponse` object using an `async for` loop. The stream wrapper natively yields conversational `str` text tokens as they arrive"

### Anchor: `README.md` / `Sugared Thoughts & Tool Call Streams (Advanced)`

> "stream internal model reasoning/thinking or intercept tool call dispatches in real-time using dedicated async stream properties"

> "async for call in response.tool_calls"

### Anchor: `README.md` / `Advanced Usage with Conversation`

> "`Conversation` is a stateful session that accumulates step history, provides a `chat()` convenience method, and exposes state introspection"

> "Low-level: streaming steps"

### Anchor: `README.md` / `Hooks and Policies`

> "Control agent behavior with a declarative policy system"

> "deny('*')"; "allow('view_file')"; "ask_user('run_command', handler=my_handler)"

### Anchor: `README.md` / `Triggers`

> "Run background tasks that react to external events and push messages into the agent"

### Anchor: `hooks/README.md` / `Hook Taxonomy`

> "Inspect Hooks (Read-Only, Non-Blocking)"; "Decide Hooks (Read-Only, Blocking)"; "Transform Hooks (Modifying, Blocking)"

### Anchor: `hooks/README.md` / `Connection Compatibility` / `LocalConnection`

> "Built-in tool hooks ... `PreToolCallDecideHook` runs and can approve or deny built-in tools. `PostToolCallHook` fires when the harness reports the tool as complete. `OnToolErrorHook` fires when the tool fails."

> "Subagent invocations appear as `START_SUBAGENT` tool calls. `PreToolCallDecideHook` fires before the subagent starts, and `PostToolCallHook` fires when the subagent trajectory goes idle"

### Anchor: `hooks/README.md` / `Observing Model Responses`

> "Use `PostTurnHook`, which receives the complete model response after each agent turn completes."

> "Inspect `conversation.history` for the full step-by-step trajectory, including intermediate model steps."

### Anchor: `hooks/hooks.py` / concrete hook interfaces

Hook classes present in the file include `OnSessionStartHook`, `OnSessionEndHook`, `PreTurnHook`, `PostTurnHook`, `PreToolCallDecideHook`, `PostToolCallHook`, `OnToolErrorHook`, `OnInteractionHook`, and `OnCompactionHook`. Internal telemetry hooks `_PreStepHook` and `_PostStepHook` are also defined.

### Anchor: `hook_runner.py` / dispatch methods

The runner exposes dispatch methods for session start/end, pre/post turn, pre/post tool call, tool error, interaction, compaction, and internal pre/post step. `dispatch_pre_turn` and `dispatch_pre_tool_call` short-circuit when a decision hook returns `allow=False`; `dispatch_interaction` returns a `HookResult(allow=False, message="No interaction hook handled the request")` when no interaction hook handles the request.

### Anchor: `types.py` / `BuiltinTools`

The builtin tool enum includes `list_directory`, `search_directory`, `find_file`, `view_file`, `create_file`, `edit_file`, `run_command`, `ask_question`, `start_subagent`, `generate_image`, `search_web`, `read_url_content`, and `finish`. Helper groups define read-only, nondestructive, all, file, and none sets.

### Anchor: `types.py` / `CapabilitiesConfig`

`CapabilitiesConfig` has `enable_subagents`, `enabled_tools`, `disabled_tools`, `compaction_threshold`, and `finish_tool_schema_json`. Its docstring distinguishes hiding tools via `enabled_tools`/`disabled_tools` from policy denial at runtime.

### Anchor: `types.py` / `Step`

`Step` carries `id`, `step_index`, `type`, `source`, `target`, `status`, `content`, `content_delta`, `thinking`, `thinking_delta`, `tool_calls`, `error`, `is_complete_response`, `structured_output`, and `usage_metadata`.

### Anchor: `types.py` / `ChatResponse`

`ChatResponse` exposes `.chunks`, async iteration over text deltas, `.thoughts`, `.tool_calls`, `.resolve()`, `.text()`, `.structured_output()`, `.usage_metadata`, and `.cancel()`.

### Anchor: `conversation/conversation.py` / core send/receive and lifecycle

`Conversation.send()` sends a prompt after draining or waiting for an in-progress turn. `receive_steps()` records and yields `Step` objects until idle. `receive_chunks()` yields `Thought`, `Text`, and deduplicated `ToolCall` events. `Conversation` exposes `history`, `last_response`, `turn_count`, `compaction_indices`, `clear_history`, `is_idle`, `conversation_id`, `total_usage`, `cancel()`, `wait_for_idle()`, `wait_for_wakeup()`, and `disconnect()`.

### Anchor: `connections/connection.py` / abstract connection

The abstract `Connection` interface defines `send`, `receive_steps`, `disconnect`, `cancel`, `wait_for_idle`, `wait_for_wakeup`, `_send_tool_results`, and `send_trigger_notification`.

### Anchor: `connections/local/local_connection.py` / input events and cancellation

`LocalConnection.send()` serializes string prompts as `InputEvent(user_input=...)` and multimodal prompts as `InputEvent(complex_user_input=...)`. `cancel()` sets `_client_cancelled`, creates `InputEvent(halt_request=True)`, and sends it over the WebSocket. `send_trigger_notification()` sends `InputEvent(automated_trigger=content)`.

### Anchor: `connections/local/local_connection.py` / questions and tool confirmations

The reader loop handles `questions_request` and `tool_confirmation_request` while a step is in `STATE_WAITING_FOR_USER`, debouncing duplicate requests and launching background handlers. `_handle_question_request()` maps multiple-choice questions to `AskQuestionInteractionSpec`, dispatches `OnInteractionHook`, and sends `InputEvent(question_response=...)`. `_handle_tool_confirmation_request()` maps built-in or MCP tool fields to a `ToolCall`, dispatches `PreToolCallDecideHook`, and sends `InputEvent(tool_confirmation=...)` with only an accepted boolean.

### Anchor: `connections/local/local_connection.py` / local sidecar process

`LocalConnectionStrategy.__aenter__()` starts the compiled local harness with `subprocess.Popen([self._binary_path], stdin=..., stdout=..., stderr=...)`, exchanges an `InputConfig`/`OutputConfig` handshake to obtain a WebSocket URL and API key, sends `InitializeConversationEvent(config=harness_config)`, receives initial history, and constructs `LocalConnection`. `__aexit__()` disconnects the connection. `_get_default_binary_path_external()` finds the local harness binary from `ANTIGRAVITY_HARNESS_PATH`, the installed wheel package resources, `localharness` on `PATH`, or raises if it cannot find it.

### Anchor: `connections/local/local_connection_config.py` / local config

`LocalAgentConfig` defaults to local harness backend settings, default capabilities, `policy.confirm_run_command`, and current working directory as workspace. It accepts `conversation_id`, `save_dir`, `app_data_dir`, `model`/`models`, `api_key`, `vertex`, `project`, `location`, skills paths, MCP servers, hooks, triggers, and subagents.

### Anchor: `connections/local/local_connection.py` / subagent configuration

`LocalConnectionStrategy._to_harness_side_tools_proto()` controls whether the start-subagent tool is enabled. `_build_custom_subagents_protos()` turns `SubagentConfig` records into `CustomAgent` protos, validates custom tool registration on the main agent, and warns that nested subagents are not supported.

### Anchor: `triggers/README.md` and `triggers.py`

Triggers are long-lived async functions running alongside an agent session. They react to external events and push messages back into the agent. `TriggerContext.send(content)` calls `connection.send_trigger_notification(content)`.

### Anchor: `trigger_runner.py`

`TriggerRunner.start()` creates an asyncio task per trigger; `stop()` cancels all tasks and waits for cleanup. Unhandled non-cancellation exceptions are logged and swallowed; no automatic restart is performed.

### Anchor: `utils/interactive.py`

`ToolConfirmationHook` prompts on stdin and returns `HookResult(allow=True)` or `HookResult(allow=False, message="User denied tool call.")`. `AskQuestionHook` prompts on stdin and returns `QuestionHookResult`. `run_interactive_loop()` registers an `AskQuestionHook`, upgrades policies, reads user input, sends it to the conversation, watches step types, and prints completed agent responses.
