---
provenance: agent-synthesis
updated: 2026-07-07
campaign: v0-stack-tooling
facet: reference-control-planes
---

# Reference control planes for human-operated agent sessions

## Verdict

No fetched source provides a strong, forkable precedent for Patchbay's full niche: a deployment-neutral human control plane for long-running agent sessions with durable command acceptance/delivery/execution state, reconnectable snapshots, typed response slots, and explicit authority across adapters. {inferred: cross-source comparison} The high-overlap agent-session precedents are product-specific: Claude Code Remote Control gives multi-surface control of a local Claude Code session, spawn/capacity modes, sync, reconnect, trusted devices, and outbound-only relay transport; Cursor Cloud Agents gives durable cloud-agent resources plus per-prompt runs, run-scoped SSE, terminal cancellation, and archive/delete; Codex app-server gives a rich local JSON-RPC control/event surface around threads/turns/items, generated schemas, and experimental remote-control controls; OpenCode gives a multi-client HTTP/OpenAPI/SSE server around sessions and messages.[claude-code-remote-control-current]{2}[claude-code-remote-control-current]{3}[claude-code-remote-control-current]{6}[claude-code-remote-control-current]{10}[claude-code-remote-control-current]{11}[cursor-cloud-agents-current]{3}[cursor-cloud-agents-current]{8}[cursor-cloud-agents-current]{10}[codex-appserver-current]{2}[codex-appserver-current]{6}[codex-appserver-current]{11}[opencode-server-docs-current]{4}[opencode-server-docs-current]{5}[opencode-server-docs-current]{8}[opencode-server-docs-current]{12}

The usable prior art is therefore partial-analogue prior art, not an architecture to fork. {inferred: cross-source comparison} Borrow from supervisors for state machines/restart/backoff/status, from remote terminal/IDE tools for attach/reconnect/session persistence, from Jupyter/Codex for correlation and event streams, and from Claude/Cursor/OpenCode for agent-session control surfaces; keep Patchbay's core self-grounded because none of those sources combines the full Patchbay semantic bundle.[systemd-systemctl]{2}[systemd-service-semantics]{8}[supervisor-process-states]{1}[tmux-session-persistence]{3}[gnu-screen-session-persistence]{4}[jupyter-kernel-messaging]{2}[codex-appserver-current]{7}[cursor-cloud-agents-current]{3}[claude-code-remote-control-current]{2}

## Existing agent-session control planes and near misses

### Claude Code Remote Control

Claude Code Remote Control is a high-overlap human control-surface analogue among fetched sources: it connects browser/mobile/Desktop surfaces to a local Claude Code session, keeps execution local, syncs the conversation across connected devices, lets terminal/browser/phone send messages interchangeably, and supports reconnect after laptop sleep/network drop.[claude-code-remote-control-current]{2}[claude-code-remote-control-current]{3} Server mode also exposes session creation controls with `--spawn same-dir|worktree|session`, `--capacity`, and pre-created sessions, while Trusted Devices can require enrolled device plus recent sign-in before viewing or steering sessions.[claude-code-remote-control-current]{6}[claude-code-remote-control-current]{11}

Near-miss boundary: Claude's source is a first-party product control surface for Claude Code, not an adapter-neutral coordination core. {inferred: source-scope comparison} Its remote surfaces are described as windows into a local Claude Code session, with traffic routed through Anthropic API and short-lived credentials, but the source does not describe a general registry of operation kinds, adapter capability manifests, LSN-ordered durable command logs, or cross-adapter authority semantics.[claude-code-remote-control-current]{4}[claude-code-remote-control-current]{10}[claude-code-remote-control-current]{11}

### Cursor Cloud Agents

Cursor Cloud Agents is a high-overlap durable agent/run API analogue: its API explicitly splits work into a durable agent plus per-prompt runs, creates an agent and initial run together, stores execution status on runs, enforces one active run per agent with `409 agent_busy`, streams run-scoped SSE, supports `Last-Event-ID` resume scoped to that run, terminalizes cancellation as `CANCELLED`, and supports reversible archive versus irreversible delete.[cursor-cloud-agents-current]{3}[cursor-cloud-agents-current]{4}[cursor-cloud-agents-current]{5}[cursor-cloud-agents-current]{6}[cursor-cloud-agents-current]{8}[cursor-cloud-agents-current]{9}[cursor-cloud-agents-current]{10}[cursor-cloud-agents-current]{11}

Near-miss boundary: Cursor's model is a vendor cloud-agent resource API, not a neutral human cockpit for heterogeneous local/headless harness sessions. {inferred: source-scope comparison} It offers strong resource/run split patterns for Patchbay's session/operation separation, but the fetched API is public beta and frames execution inside Cursor Cloud Agents rather than an adapter-neutral core.[cursor-cloud-agents-current]{1}[cursor-cloud-agents-current]{2}[cursor-cloud-agents-current]{12}

### Codex app-server

Codex app-server is a strong protocol-surface analogue: it exposes bidirectional JSON-RPC-like messaging, thread/turn/item primitives, version-matched generated schemas, bounded ingress queues with retryable overload, thread lifecycle/read APIs, turn start/steer/interrupt controls, notification streams, standalone command/process utilities, and experimental remote-control enable/disable/status/pairing/revocation methods.[codex-appserver-current]{2}[codex-appserver-current]{4}[codex-appserver-current]{5}[codex-appserver-current]{6}[codex-appserver-current]{7}[codex-appserver-current]{8}[codex-appserver-current]{9}[codex-appserver-current]{10}[codex-appserver-current]{11}

Near-miss boundary: Codex app-server is an app-server protocol for Codex rich clients, not a deployment-neutral human control plane. {inferred: source-scope comparison} It is highly relevant to Patchbay's generated-contract and action/event modeling, but its primitive vocabulary is Codex-specific (`thread`, `turn`, `item`) and its documented local control-plane socket is for app-server clients rather than for supervising heterogeneous harness adapters.[codex-appserver-current]{1}[codex-appserver-current]{3}[codex-appserver-current]{6}

### OpenCode server

OpenCode is a useful open-source shape for a multi-client agent harness server: the docs say running `opencode` starts a TUI and server, with the TUI as a client; the architecture supports multiple clients and programmatic interaction; `opencode serve` starts a standalone server; the server publishes OpenAPI 3.1; and events stream over SSE.[opencode-server-docs-current]{4}[opencode-server-docs-current]{5}[opencode-server-docs-current]{6}[opencode-server-docs-current]{8}[opencode-server-docs-current]{9}[opencode-server-docs-current]{12}

Near-miss boundary: OpenCode provides an adapter/harness server, not a separate durable authority layer. {inferred: source-scope comparison} It has sessions/messages APIs and basic auth, but the fetched server docs do not attest Patchbay-like accepted/delivered/running/terminal operation state, idempotency keys, per-operation authority grants, or snapshot tiers.[opencode-server-docs-current]{3}[opencode-server-docs-current]{10}[opencode-server-docs-current]{11}

## Process-supervisor patterns worth borrowing

Systemd and Supervisor are not agent-session control planes, but they are valuable state-machine exemplars. Systemd separates general ACTIVE state from type-specific SUB state, exposes human `status` versus parseable `show`, and treats status as current/most-recent runtime state while prior invocations live in journal history.[systemd-systemctl]{2}[systemd-systemctl]{3}[systemd-systemctl]{4} Supervisor similarly exposes per-process states (`STOPPED`, `STARTING`, `RUNNING`, `BACKOFF`, `STOPPING`, `EXITED`, `FATAL`, `UNKNOWN`) and a programmatic API returning daemon state and per-process snapshots.[supervisor-process-states]{1}[supervisor-xmlrpc-api]{1}[supervisor-xmlrpc-api]{4}[supervisor-xmlrpc-api]{5}

A concrete Patchbay pattern is to keep separate axes rather than overload one enum: process supervisors distinguish daemon/manager state, process active state, detailed substate, and logs/snapshots; Patchbay's SessionConnectivityState, SessionActivityState, OperationState, and durable observations should remain similarly separate. {inferred: analogy} Systemd's `status`/`show` distinction also supports Patchbay's design choice that UI display state should be derived from parseable core state, not be authority.[systemd-systemctl]{2}[systemd-systemctl]{3}[systemd-systemctl]{5}[supervisor-xmlrpc-api]{4}

Supervisor's backoff/fatal semantics and systemd's restart policy are useful for adapter/runtime health but should not be copied wholesale into OperationState. Supervisor `BACKOFF` loops until `startretries` reaches `FATAL`, `FATAL` needs manual restart, and administrative stop moves through `STOPPING` to `STOPPED`; systemd `Restart=` policies cover clean/unclean exits, timeouts, watchdogs, exceptions, and start-rate limiting.[supervisor-process-states]{4}[supervisor-process-states]{5}[supervisor-process-states]{6}[supervisor-process-states]{7}[systemd-service-semantics]{8}[systemd-service-semantics]{9}[systemd-service-semantics]{10}

Systemd's service-type nuance is especially relevant to Patchbay's accepted-vs-delivered-vs-running split: `Type=simple` can report start success before the service binary can be invoked, `Type=exec` waits until exec succeeds, and `Type=notify` lets service code explicitly declare readiness.[systemd-service-semantics]{3}[systemd-service-semantics]{4}[systemd-service-semantics]{5} Patchbay should preserve this distinction at adapter boundaries: an adapter accepting delivery responsibility is not the same thing as target execution or semantic completion. {inferred: analogy}

## Remote-control and reconnect patterns worth borrowing

Terminal multiplexers provide the oldest direct pattern for durable attach/detach: tmux sessions are server-managed collections of pseudo-terminals, multiple clients can connect to the same session, and sessions survive SSH timeouts or detaches; GNU Screen likewise keeps programs running while detached and exposes list/reattach/detach/create status labels.[tmux-session-persistence]{1}[tmux-session-persistence]{2}[tmux-session-persistence]{3}[gnu-screen-session-persistence]{1}[gnu-screen-session-persistence]{3}[gnu-screen-session-persistence]{4} These tools validate Patchbay's attach/reconnect priority, but their command substrate is terminal bytes and pseudo-terminal state, not typed durable operator intent. {inferred: analogy}

VS Code Remote SSH/Tunnels validates a remote-machine/server-client split rather than a command-lifecycle model: Remote SSH installs VS Code Server on the remote OS and runs commands/extensions/terminals on the remote machine, while Remote Tunnels starts VS Code Server plus a secure tunnel, lets clients connect to active machines, and can run the tunnel as a service.[vscode-remote-ssh]{1}[vscode-remote-ssh]{2}[vscode-remote-ssh]{6}[vscode-remote-tunnels]{1}[vscode-remote-tunnels]{2}[vscode-remote-tunnels]{4}[vscode-remote-tunnels]{7} VS Code Tunnels also shows an access pattern Patchbay can pressure-test: both tunnel hosting and connecting require account authentication, outbound Azure connections avoid local firewall listeners, and SSH over the tunnel provides end-to-end encryption.[vscode-remote-tunnels]{6}

Jupyter's kernel protocol gives useful correlation and status mechanics: messages have `msg_id` and `session`, replies and side effects copy the causing request header into `parent_header`, execute flows publish `busy`, reply with `ok`/`error`/deprecated `aborted`, then publish `idle`, and execute replies carry execution counts when history is stored.[jupyter-kernel-messaging]{1}[jupyter-kernel-messaging]{2}[jupyter-kernel-messaging]{4}[jupyter-kernel-messaging]{5}[jupyter-kernel-messaging]{8} This supports Patchbay's typed correlation and observation-stream design, but Jupyter's source is a frontend-kernel execution protocol rather than human authority over autonomous agent sessions. {inferred: source-scope comparison}

## What Patchbay should borrow, not borrow, and keep novel

Borrow:

- **State axes and parseable snapshots:** systemd's ACTIVE/SUB plus `show`, Supervisor's process snapshot structs, and Cursor's durable agent/run split support Patchbay's choice to separate session axes, operation lifecycle, and display state.[systemd-systemctl]{2}[systemd-systemctl]{3}[supervisor-xmlrpc-api]{4}[cursor-cloud-agents-current]{3}[cursor-cloud-agents-current]{5}
- **Terminal finality and restart/backoff vocabulary:** Supervisor `FATAL` requiring manual action and systemd restart/rate-limit semantics are useful for adapter/runtime lifecycle policy, distinct from operator OperationState.[supervisor-process-states]{6}[systemd-service-semantics]{10}
- **Attach/reconnect and multi-surface UX:** tmux/screen detached persistence, VS Code remote clients, and Claude Remote Control's synced terminal/browser/phone surfaces all validate reconnect-first cockpit design.[tmux-session-persistence]{3}[gnu-screen-session-persistence]{4}[vscode-remote-tunnels]{4}[claude-code-remote-control-current]{3}
- **Correlation:** Jupyter parent headers and Codex thread/turn/item notifications are practical precedents for correlating requests, side effects, streams, and terminal replies.[jupyter-kernel-messaging]{2}[codex-appserver-current]{7}
- **Generated/open contracts:** Codex version-specific schema generation and OpenCode's OpenAPI endpoint support Patchbay's generated-contract discipline.[codex-appserver-current]{5}[opencode-server-docs-current]{8}

Do not borrow unmodified:

- **Process-manager restart states as command states.** Supervisor/systemd process states model runtime supervision; Patchbay Operations model accepted human intent and delivery/execution state, so process states should inform adapter health and failure mapping, not replace OperationState. {inferred: analogy}[supervisor-process-states]{1}[systemd-service-semantics]{8}
- **Terminal attach as semantic recovery.** tmux/screen preserve terminal sessions but not typed operation acceptance, idempotency, authority, or snapshots; Patchbay should not reduce agent control to terminal bytes. {inferred: analogy}[tmux-session-persistence]{1}[tmux-session-persistence]{3}[gnu-screen-session-persistence]{1}
- **Vendor-specific agent/run APIs as the core model.** Claude, Cursor, Codex, and OpenCode each expose valuable surfaces, but each is bound to one harness/product/server; Patchbay's neutral core should map adapters into Patchbay's registry rather than adopting any one vocabulary as canonical. {inferred: cross-source comparison}[claude-code-remote-control-current]{2}[cursor-cloud-agents-current]{2}[codex-appserver-current]{1}[opencode-server-docs-current]{4}

## Contradictions

No direct factual contradiction surfaced among fetched sources. The important divergences are structural:

| Handles | Relationship | Detail |
|---|---|---|
| `claude-code-remote-control-current` / `cursor-cloud-agents-current` / `codex-appserver-current` / `opencode-server-docs-current` | incommensurable | Claude is local-session remote control, Cursor is cloud-agent resource/run API, Codex is a rich local app-server protocol, and OpenCode is a TUI/server HTTP+SSE harness. They address overlapping operator-control problems with incompatible product boundaries rather than one shared model.[claude-code-remote-control-current]{2}[cursor-cloud-agents-current]{3}[codex-appserver-current]{6}[opencode-server-docs-current]{4} |
| `systemd-service-semantics` / `supervisor-process-states` / `cursor-cloud-agents-current` | qualifies | Supervisor/systemd lifecycle states describe supervised processes; Cursor run states describe cloud-agent executions. Both inform lifecycle modeling, but neither directly qualifies as Patchbay's durable accepted Operation lifecycle.[systemd-service-semantics]{8}[supervisor-process-states]{1}[cursor-cloud-agents-current]{10} |
| `tmux-session-persistence` / `jupyter-kernel-messaging` | tension | tmux persistence is session/PTY-level attach continuity, while Jupyter is message-level request/reply correlation. Patchbay needs both attach continuity and typed operation correlation, so neither alone is sufficient.[tmux-session-persistence]{3}[jupyter-kernel-messaging]{2}[jupyter-kernel-messaging]{4} |

## Disconfirming analysis

Before accepting the novelty finding, I checked likely disconfirmers in the fetched corpus:

- **Claude Code Remote Control** could have been a direct precedent because it offers cross-device human control, synced sessions, spawn/capacity, trusted devices, and outbound relay transport. It does not disconfirm Patchbay's novelty because the fetched source frames web/mobile as windows into one local Claude Code session/product, not as a neutral durable Operation/Observation/Elicitation/authority core across adapters.[claude-code-remote-control-current]{2}[claude-code-remote-control-current]{4}[claude-code-remote-control-current]{6}[claude-code-remote-control-current]{10}[claude-code-remote-control-current]{11}
- **Cursor Cloud Agents** could have been a direct precedent because it has durable agents, per-prompt runs, cancellation finality, stream resume, and archive/delete. It does not disconfirm Patchbay's novelty because it is a Cursor cloud-agent API, not a human cockpit for arbitrary headless local/remote harness sessions.[cursor-cloud-agents-current]{2}[cursor-cloud-agents-current]{3}[cursor-cloud-agents-current]{8}[cursor-cloud-agents-current]{10}[cursor-cloud-agents-current]{11}
- **Codex app-server** could have been a direct precedent because it uses bidirectional JSON-RPC, generated schemas, threads/turns/items, notifications, and remote-control controls. It does not disconfirm Patchbay's novelty because it is Codex's app-server interface and local control-plane socket, not a deployment-neutral authority-bearing core.[codex-appserver-current]{1}[codex-appserver-current]{2}[codex-appserver-current]{3}[codex-appserver-current]{5}[codex-appserver-current]{11}
- **OpenCode server** could have been a direct precedent because it is open source and already has a TUI/server split, OpenAPI, multiple clients, sessions/messages, and SSE. It does not disconfirm Patchbay's novelty because the fetched docs attest harness-server APIs rather than durable accepted/delivered/running/completed operation semantics and grant-checked authority.[opencode-server-docs-current]{4}[opencode-server-docs-current]{5}[opencode-server-docs-current]{8}[opencode-server-docs-current]{10}[opencode-server-docs-current]{12}
- **Process supervisors** could have been a direct precedent because they have lifecycle states, restart/backoff, status APIs, and start/stop controls. They do not disconfirm novelty because their controlled object is a process, not an agent session carrying prompts, tool approvals, elicitation responses, source-authenticated observations, and human authority semantics.[systemd-service-semantics]{1}[systemd-service-semantics]{8}[supervisor-process-states]{1}[supervisor-xmlrpc-api]{6}
- **Terminal multiplexers and remote IDEs** could have been direct precedents because they solve attach/reconnect. They do not disconfirm novelty because they recover remote terminal/IDE interaction, not durable semantic command acceptance and idempotent retry.[tmux-session-persistence]{3}[gnu-screen-session-persistence]{4}[vscode-remote-ssh]{2}[vscode-remote-tunnels]{3}

A broad web search attempt for quoted combinations around "human control plane", "agent sessions", and "coding agent control plane" was made during this pass, but DuckDuckGo's HTML endpoint returned an anti-bot challenge rather than usable results. That search result is not cited as source evidence; the novelty finding is therefore scoped to the fetched corpus above, not to the entire public web. {confidence: fetched-corpus-limited}

## Revisit if

- Claude, Codex, Cursor, OpenCode, or another harness publishes a durable, vendor-neutral agent-session control-plane specification rather than product-specific control docs.
- Codex's experimental remote-control methods stabilize into a documented external controller protocol with authority/revocation/durable operation semantics.[codex-appserver-current]{11}
- OpenCode's OpenAPI surface adds explicit accepted/delivered/running/terminal operation state, idempotency, or grant-checking beyond basic auth and session/message APIs.[opencode-server-docs-current]{3}[opencode-server-docs-current]{8}[opencode-server-docs-current]{10}
- Cursor Cloud Agents exits public beta with generalized external adapter/session support beyond Cursor-managed cloud agents.[cursor-cloud-agents-current]{1}[cursor-cloud-agents-current]{2}
- A new project appears that explicitly models human-operated, reconnectable, durable command control for heterogeneous autonomous agent sessions.

## Acquisition candidates

- **enriching** — Codex generated app-server JSON Schema/TypeScript output from the exact Codex binary version Patchbay evaluates. Source: `codex-appserver-current`; class: generated schema artifact; web-availability: not a static web page, generated by CLI; completes: exact method/field/state schema beyond README prose.[codex-appserver-current]{5}
- **enriching** — OpenCode `/doc` OpenAPI 3.1 document from a running pinned OpenCode version. Source: `opencode-server-docs-current`; class: generated server contract; web-availability: available from a running server, not stable as a single public static URL; completes: exact request/response/event surface for adapter design.[opencode-server-docs-current]{8}
