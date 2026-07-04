---
provenance: agent-synthesis
updated: 2026-07-04
---

# OpenCode — operator/agent/harness action surface

OpenCode (repo `sst/opencode`, cloned to `/tmp/opencode`) is an open-source CLI/TUI agent harness with a typed HTTP API server (effect/HttpApi). Unlike Pi's extension-hook model, OpenCode exposes its control surface as a typed HTTP API plus an SSE event stream. This brief catalogs the action surface grounded in the actual source.

## Outbound (agent→operator events)

From the schema event definitions `[opencode-schema-events]{1}`:

**Session lifecycle / status:**
- `session.status` / `session.idle` — SessionStatus is a two-state `idle` | `busy` union (no separate running/completed at session level).
- `session.next.prompted`, `session.next.prompt.admitted` — prompt lifecycle.
- `session.next.agent.switched`, `session.next.model.switched` — agent/model reconfiguration.
- `session.next.moved` — session moved.
- `session.next.context.updated` — context change.
- `session.next.shell.started` — shell prompt started.
- `session.next.synthetic` — synthetic event.

**Tool approval (permission):**
- `permission.v2.asked` — a tool requests permission; carries the request.
- `permission.v2.replied` — operator replied (`once` / `always` / `reject`).

**Question (structured agent→operator prompt):**
- `question.asked` — agent asks the operator a multi-choice question.
- `question.replied` / `question.rejected` — operator answers or rejects.

**Adjacent event domains** (in the manifest): Pty (terminal), FileSystem, FileSystemWatcher, MCP, LSP, VCS (git), Workspace, Worktree, Installation, Reference, Plugin, ProjectDirectories, ModelsDev, Integration, Catalog. These are outbound state-change events across the harness's full surface.

The outbound surface is a **typed event bus** (effect) with SSE delivery — richer and more structured than Pi's hook callbacks.

## Inbound (operator→agent control)

From the session HTTP handler `[opencode-session-handler]{1}`:

**Drive (lifecycle-bearing):**
- `prompt` — synchronous prompt (waits for result).
- `promptAsync` — fire-and-forget prompt.
- `shell` — prompt with shell (busy-mapped).
- `loop` — re-run the prompt loop.

**Request (lifecycle-acting):**
- `remove` (session) — removes a session, cancels in-flight prompt (`promptSvc.cancel`).
- `removeMessage` — removes a specific message.

**Reconfigure:**
- (Agent/model switching is inbound via events `session.next.agent.switched`/`model.switched`, but the inbound trigger is the session/agent API — see the agent/provider handlers.)

**Session management (read + mutate):**
- `status` — list session statuses (query).
- `summary.diff` — diff summary at a message (query).
- `revert` / `unrevert` — git-pair revert/unrevert (first-class session action in OpenCode; busy-mapped).
- `compact` — compact the session (`SessionCompaction`).
- `share` / `unshare` — share/unshare the session.

**Permission (tool approval) — separate handler:**
- The operator replies to `permission.v2.asked` with `once` / `always` / `reject` (from `[opencode-schema-events]{1}`). "always" persists as a Rule in a Ruleset.

**Question (answer) — separate handler:**
- The operator answers `question.asked` with `answers` (array of selected labels) or rejects.

## Provisioning

**No first-class spawn/retire operator action found in the surveyed source.** OpenCode runs as a server process (`packages/opencode/src/server/server.ts`); sessions are created/removed via the session API, but spawning a *new OpenCode process* on a target machine is not an operator action in the surveyed API — it is out-of-band (the operator starts the `opencode` server process themselves). This mirrors remote_pi's `pi-supervisord` posture: provisioning is sysadmin, not operator-action. `{confidence: shallow-survey}` — the `control-plane` and `installation` modules exist in the source tree and may carry more, but were not deeply surveyed in this pass.

## No-grant informational replyable content?

**No separate "Message" type distinct from prompt/drive.** The inbound surface is prompts (`prompt`/`promptAsync`/`shell`/`loop`) and approval/answer replies to agent-initiated questions. Outbound is events. There is no "send informational replyable content without a grant" path — the operator drives (prompt) and the agent replies (events + question.asked). The `question` channel is the closest thing to a non-drive replyable message, but it is agent→operator (the agent asks, the operator answers), not operator→agent informational content. So OpenCode supports "Message dropped for v0" — the universal shape is "operator drives, agent replies."

## Structural shape classification

- **Drive:** `prompt`, `promptAsync`, `shell`, `loop` — lifecycle-bearing (idle↔busy), the terminal-candidate-competitors.
- **Request:** `remove`/`removeMessage` (acts on session/message lifecycle), permission-reply (acts on tool-call gate), question-reply (acts on question gate).
- **Query:** `status`, `summary.diff`.
- **Result:** all `session.next.*` events, `permission.v2.*` events, `question.*` events — agent→operator output.
- **Payload:** prompt text (carried by `prompt`/`promptAsync`/`shell`).
- **Provision:** absent as operator action (out-of-band server-process start).

## Revisit if

- The `control-plane` / `installation` modules are surveyed deeply and found to expose spawn/retire as operator actions (would promote Provision from absent to present).
- A newer OpenCode version adds a session-creation lifecycle distinct from `prompt` (would add a session-provision action within an existing process).
- The `MoveSession` (`@opencode-ai/core/control-plane/move-session`) module is found to be an operator action for moving sessions across machines (would be a cross-machine fleet action).

## Disconfirming analysis

Searched for evidence that OpenCode exposes no-grant informational replyable content: the inbound API is prompts + approval/question replies; the `question` channel is agent→operator. No inbound "send a message that isn't a prompt" verb found. Searched for provisioning: the session API creates/removes sessions within a running server, but spawning the server itself is out-of-band. The `control-plane` module name suggests fleet capability may exist, but the surveyed handler/route set did not surface it as an HTTP operator verb — left as a Revisit-if rather than asserted absent.

## Contradictions

None within this facet's sources. (Cross-harness contradictions surface in the parent synthesis.)
