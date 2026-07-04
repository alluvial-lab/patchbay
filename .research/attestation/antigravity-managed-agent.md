---
source_handle: antigravity-managed-agent
fetched: 2026-07-03
source_url: https://ai.google.dev/gemini-api/docs/antigravity-agent
provenance: source-direct
---

# Per-source attestation: Gemini API Antigravity Agent docs

## Structural metadata

- Format fetched: HTML from Google AI for Developers; text extracted locally to `/tmp/antigravity-research/gemini-antigravity-agent.txt`.
- Document title: `Antigravity Agent | Gemini API | Google AI for Developers`.
- Scope: Managed Antigravity agent through the Gemini API Interactions API, not the local Python SDK or desktop/TUI surfaces.

## Paraphrased source summary

The document describes an Antigravity managed agent available through the Gemini API Interactions API. It presents single-call, multi-turn, background, cancellation, function-calling, MCP, tool, environment, and sandbox behavior. It says each call can provision or reuse a Linux sandbox and start an autonomous tool-use loop. It shows tool-call requests for custom functions as `requires_action` interactions whose steps include `function_call`; clients then send `function_result` in a subsequent interaction with `previous_interaction_id` and the same environment. It documents background polling, cancellation, and environment reuse.

## Key passages with source-internal anchors

### Anchor: title / introduction

> "The Antigravity agent is a general-purpose managed agent on the Gemini API. A single API call gives you an agent that reasons, executes code, manages files, and browses the web inside your own secure Linux sandbox, hosted by Google."

> "It is powered by Gemini 3.5 Flash and uses the same harness as the Antigravity IDE. Available through the Interactions API and Google AI Studio."

### Anchor: `Capabilities`

> "Each call can provision a Linux sandbox and starts a tool-use loop. The agent plans, acts, observes results, and repeats until the task is done."

> "Code execution: Run Bash, Python, and Node.js commands. Install packages, run tests, build apps."

> "File management: Read, write, edit, search, and list files in the sandbox. Files persist across interactions."

> "Context compaction: Automatic context compaction (triggered at ~135k tokens) to support long-running, multi-turn sessions without losing context or hitting token limits."

### Anchor: `Supported tools`

> "By default, the agent has access to `code_execution`, `google_search`, and `url_context`. Filesystem tools are enabled automatically when you specify the environment parameter."

> "Custom Functions ... Define custom functions that the agent can request to execute."

> "Remote MCP Server ... Register external Model Context Protocol (MCP) servers as tools."

### Anchor: `Function calling`

> "The following example demonstrates a 2-turn interaction. The agent first requests a custom get_weather function call, and the client executes it and returns the result in the second turn."

> `if interaction.status == "requires_action":`

> `pending_calls = [step for step in interaction.steps if step.type == "function_call" and step.id not in executed_calls]`

> `previous_interaction_id=interaction.id`

> `environment=interaction.environment_id`

> `{"type": "function_result", "name": fc_step.name, "call_id": fc_step.id, "result": function_result}`

### Anchor: `Background execution`

> "Use `background=True` to run the interaction asynchronously. The API returns immediately with an interaction ID that you poll until the status is completed or failed."

> `interaction = client.interactions.get(id=interaction.id)`

> "Background execution requires `store=True`, which is the default."

> "You can cancel a running background interaction using the cancel method."

> `client.interactions.cancel(id="INTERACTION_ID")`

> `POST ... /v1beta/interactions/INTERACTION_ID:cancel`

### Anchor: `Multi-turn with background execution`

> "When a background interaction involves stateful tools (like code execution in a sandbox), use the `environment_id` from the completed interaction to continue in the same environment. This ensures the agent picks up where it left off with all files and state intact."

> `previous_interaction_id=interaction.id`

> `environment=interaction.environment_id`

### Anchor: `Environments`

> "Each call creates or reuses a Linux sandbox."

> `"remote"` — "Provision a fresh sandbox with default settings."

> `"env_abc123"` — "Reuse an existing environment by ID, preserving all files and state."

> `{...}` — "Full EnvironmentConfig with custom sources and network rules."

### Anchor: `Limitations`

> "Filesystem tool: There is no filesystem tool at the moment. It is part of the environment."

> "Store requirement: Agent execution using `background=True` requires `store=True`."

> "Stateful only function calling: Function calling is only supported in stateful mode. You must use `previous_interaction_id` to continue the turn; reconstructing history manually (stateless mode) is not supported."
