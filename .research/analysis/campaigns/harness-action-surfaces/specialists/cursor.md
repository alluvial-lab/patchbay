---
provenance: agent-synthesis
updated: 2026-07-03
campaign: harness-action-surfaces
facet: cursor
---

# Cursor harness action surface

## Scope and source base

This brief covers Cursor's documented IDE Agent surface, Cursor CLI output surface, MCP/extension configuration surface, Agents Window/subagents surface, and Cloud Agents API surface. The public docs expose two different control planes: a mostly GUI/CLI local-agent surface, and a programmatic Cloud Agents REST/SSE surface. [cursor-agent-overview]{1} [cursor-cli-output]{3} [cursor-cloud-agents-api]{2}

## Disconfirming analysis

A likely contrary expectation is that Cursor's VS Code-derived extension surface exposes a full operator-to-agent API. The fetched Extension API reference only documents `vscode.cursor` APIs for registering/unregistering MCP servers and plugin paths; it does not document message injection, approval decisions, cancellation, model selection, or session lifecycle operations. [cursor-extension-api]{1} [cursor-extension-api]{3} [cursor-extension-api]{7}

Another contrary expectation is that all Cursor agent runs require approval prompts. The Run Modes page says Cloud Agents run in dedicated machines and do not ask the user to approve actions, while local Run Modes govern approval/auto-review. [cursor-run-modes]{7}

## Operator to agent controls

### Local IDE / Agents Window

Cursor's local IDE surface is primarily an interactive operator-drive surface: the operator opens Agent in the sidepane, prompts it, and Agent can independently edit code and run terminal commands. [cursor-agent-overview]{1} Cursor documents queued messages while Agent is working, plus an immediate-message path for urgent redirection/interruption. [cursor-agent-overview]{5}

Local tool execution is controlled through Run Modes. The operator/configurator can set modes at Settings > Agents > Approvals & Execution; Run Modes determine autonomy for shell commands, MCP tools, and Fetch calls. [cursor-run-modes]{2} [cursor-run-modes]{3} The documented approval action is tool approval: Cursor interrupts for approval depending on mode, allowlist, sandbox viability, and classifier result. [cursor-run-modes]{1} [cursor-run-modes]{5}

Cursor also exposes checkpoint restore as a local lifecycle-ish control after code changes: Agent creates checkpoints when modifying code and users can restore an earlier state. [cursor-agent-overview]{4}

The Agents Window adds multi-agent management surfaces: it is documented as an agent-first workspace across local, cloud, remote SSH, and other environments, with parallel agents, local/cloud handoff, cloud subagents, and worktrees. [cursor-agents-window-subagents]{1} [cursor-agents-window-subagents]{3} [cursor-agents-window-subagents]{4} [cursor-agents-window-subagents]{6}

### MCP / extension controls

MCP is a tool-surface extension path, not a direct agent control protocol. Cursor can install/manage MCP servers via Customize or `mcp.json`, can use MCP tools in chat, and asks for MCP tool approval by default. [cursor-mcp-extension]{1} [cursor-mcp-extension]{7} [cursor-mcp-extension]{8} MCP follows the same Run Modes as terminal commands; allowlisted MCP tools run immediately in Auto-review while others route through the classifier. [cursor-mcp-extension]{9}

The Extension API is narrow: VS Code extensions can call `vscode.cursor.mcp.registerServer`, `vscode.cursor.mcp.unregisterServer`, `vscode.cursor.plugins.registerPath`, and `vscode.cursor.plugins.unregisterPath`. [cursor-extension-api]{1} [cursor-extension-api]{3} [cursor-extension-api]{4} [cursor-extension-api]{7} [cursor-extension-api]{8} This supports dynamic MCP/plugin provisioning, not direct replyable messages or run cancellation.

### Cloud Agents API

The Cloud Agents API is the clearest programmatic lifecycle surface. It lets clients create a durable Cloud Agent and enqueue the initial run with `POST /v1/agents`; send follow-up prompts by creating runs on an active agent; list/read agents and runs; stream a run; cancel an active run; archive/unarchive an agent; and permanently delete an agent. [cursor-cloud-agents-api]{4} [cursor-cloud-agents-api]{6} [cursor-cloud-agents-api]{7} [cursor-cloud-agents-api]{8} [cursor-cloud-agents-api]{11} [cursor-cloud-agents-api]{12} [cursor-cloud-agents-api]{13}

The Cloud API separates durable agents from per-prompt runs. Execution status lives on runs, while the agent resource holds durable metadata. [cursor-cloud-agents-api]{3} [cursor-cloud-agents-api]{5}

## Agent to operator events

For local CLI automation, `stream-json` emits NDJSON events: system initialization, user message, assistant message, tool-call start/completion, and final terminal result. [cursor-cli-output]{3} [cursor-cli-output]{5} [cursor-cli-output]{6} [cursor-cli-output]{7} [cursor-cli-output]{8} [cursor-cli-output]{9} Character-level assistant deltas are available when `--stream-partial-output` is combined with `--output-format stream-json`. [cursor-cli-output]{4}

For Cloud Agents, the run stream uses SSE scoped to one run. Documented event types include status updates, assistant text, tool_call status updates with stable tool-specific envelopes, interaction_update, error, done, heartbeat, and terminal status events. [cursor-cloud-agents-api]{8} [cursor-cloud-agents-api]{9} The stream can be resumed with `Last-Event-ID` within retention constraints. [cursor-cloud-agents-api]{10}

For MCP in the GUI, Cursor shows MCP tool responses in chat with expandable argument/response views, and attaches returned images to chat for vision-capable models. [cursor-mcp-extension]{10} [cursor-mcp-extension]{11}

## Action classification

- Durable / lifecycle-bearing: Cloud Agent create, get/list, archive, unarchive, permanent delete; Cloud run create, get/list, cancel; Agents Window local/cloud handoff; worktree-backed agent isolation. [cursor-cloud-agents-api]{4} [cursor-cloud-agents-api]{6} [cursor-cloud-agents-api]{11} [cursor-cloud-agents-api]{12} [cursor-cloud-agents-api]{13} [cursor-agents-window-subagents]{4} [cursor-agents-window-subagents]{6}
- Ephemeral / payload-bearing: local prompt messages, queued/immediate messages, Cloud run prompts, CLI user/assistant/tool events, MCP tool calls and responses. [cursor-agent-overview]{5} [cursor-cloud-agents-api]{6} [cursor-cli-output]{6} [cursor-cli-output]{7} [cursor-cli-output]{8} [cursor-mcp-extension]{10}
- Read-only queries: Cloud list/get agents and list/get runs; CLI final JSON/text output; stream observation. [cursor-cloud-agents-api]{5} [cursor-cloud-agents-api]{7} [cursor-cli-output]{1} [cursor-cli-output]{10}
- Configuration/provisioning: Run Mode settings, MCP allowlists, extension API MCP/plugin registration, model and environment settings during Cloud Agent creation. [cursor-run-modes]{3} [cursor-mcp-extension]{6} [cursor-extension-api]{1} [cursor-cloud-agents-api]{14}

## Privileged sidecar / supervisor handling

For local agents, Cursor handles privileged execution through its own Run Modes, sandbox, classifier, allowlist, and user approval path rather than exposing a general supervisor API. Commands that need full system access cannot be sandboxed and go to the classifier; some commands bypass sandbox and ask for approval. [cursor-run-modes]{5} [cursor-run-modes]{6}

For Cloud Agents, privilege is moved out-of-process into Cursor-managed or configured cloud machines: the docs say Cloud Agents run in their own dedicated machine and never ask for action approval. [cursor-run-modes]{7} Cloud Agent creation can include repository, MCP server, model, and encrypted session-scoped environment variables. [cursor-cloud-agents-api]{14}

For enterprise MCP control, Cursor exposes admin allowlists and network controls that can approve local command-pattern servers, approve remote URL-pattern servers, and restrict auto-runnable tools. [cursor-mcp-extension]{6}

## Message vs command shape

Cursor clearly has ordinary operator prompt/assistant reply flows in local Agent, CLI, and Cloud Agents. [cursor-agent-overview]{1} [cursor-cli-output]{7} [cursor-cloud-agents-api]{6} The Cloud API's follow-up action is a run-creating prompt, not an out-of-band no-grant informational replyable message. [cursor-cloud-agents-api]{6}

The closest documented "message distinct from command" is local queued/immediate messaging: a user can send a message while Agent is working, and immediate messaging can interrupt or redirect current work. [cursor-agent-overview]{5} However, the fetched docs do not document a Pi-like inbound `Message` primitive that delivers no-grant, replyable content without driving a run or agent turn. The public programmable surface uses prompts/runs and event streams. [cursor-cloud-agents-api]{4} [cursor-cloud-agents-api]{6} [cursor-cloud-agents-api]{8}

## Spawn / retire exposure

Spawn is exposed in multiple ways. In the Cloud API, `POST /v1/agents` creates a durable agent and initial run; `POST /runs` creates follow-up runs. [cursor-cloud-agents-api]{4} [cursor-cloud-agents-api]{6} In the agent UI, subagents may be launched automatically, explicitly by `/name`, in parallel, or as cloud subagents on separate VMs/branches. [cursor-agents-window-subagents]{7} [cursor-agents-window-subagents]{8} [cursor-agents-window-subagents]{10} [cursor-agents-window-subagents]{11} [cursor-agents-window-subagents]{12}

Retire/stop is also exposed for Cloud Agents: cancel terminates a run and cannot be resumed; archive soft-deletes the agent by preventing new runs; delete permanently removes it. [cursor-cloud-agents-api]{11} [cursor-cloud-agents-api]{12} [cursor-cloud-agents-api]{13}

## Contradictions

No direct contradictions were found among the fetched sources. The only structural distinction is surface-specific: local Run Modes can ask for approval, while Cloud Agents are documented as not using Run Modes and not asking approval because they run in dedicated machines. [cursor-run-modes]{7}

## Revisit if

- Cursor publishes a broader Agent/Composer extension API beyond MCP/plugin registration.
- The Cloud Agents API exits public beta or changes its durable-agent/run split.
- Cursor publishes formal event schemas for IDE GUI Agent beyond CLI NDJSON and Cloud SSE.
- Cursor publishes documentation for a replyable, no-grant message primitive separate from prompts/runs.

## Acquisition candidates

- `https://cursor.com/docs-static/cloud-agents-openapi.yaml` — named by the fetched Cloud Agents API page as the full OpenAPI specification for detailed schemas and examples. [cursor-cloud-agents-api]{15}
