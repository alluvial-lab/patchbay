---
source_handle: opencode-schema-events
fetched: 2026-07-04
source_path: /tmp/opencode/packages/schema/src/session-event.ts, session-status-event.ts, permission.ts, question.ts
provenance: source-direct
---

# OpenCode schema: events, status, permission, question

The `packages/schema/src/` package defines the typed event/permission/question surfaces. These are the outbound (agent→operator) event types and the approval/question interaction schemas.

## Session events (outbound, agent→operator)

From `session-event.ts`. Event types use `Event.define({ type: "session.next.*", ... })`:

- `session.next.agent.switched` — **AgentSwitched** (messageID). Agent changed.
- `session.next.model.switched` — **ModelSwitched** (messageID). Model changed.
- `session.next.moved` — **Moved**. Session moved.
- `session.next.prompted` — **Prompted**. A prompt was issued.
- `session.next.prompt.admitted` — **PromptAdmitted**. Prompt admitted into the session.
- `session.next.context.updated` — **ContextUpdated** (messageID). Context changed.
- `session.next.synthetic` — **Synthetic** (messageID). A synthetic event.
- `session.next.shell.started` (namespace `Shell`) — shell prompt started.
- `UnknownError`, `FileAttachment`, `Source` (event source struct).

## Session status (lifecycle)

From `session-status-event.ts`. `SessionStatus.Info` is a union of literals:
- `idle` — `Schema.Literal("idle")`
- `busy` — `Schema.Literal("busy")`

Events: `session.status` (Status), `session.idle` (Idle). So the lifecycle is a two-state idle/busy (no separate "running"/"working"/"completed" states at the session-status level — completion is per-message, not per-session-status).

## Permission (tool approval)

From `permission.ts`. Permission IDs start with `per`. 

- **Source** — a union; one variant `{ type: "tool", ... }` (permission sourced from a tool call).
- **Request** — the permission request struct.
- **Reply** — `Schema.Literals(["once", "always", "reject"])` — the operator's decision: allow once, allow always (rule), or reject.
- **Effect** — `Schema.Literals(["allow", "deny", "ask"])` — the rule effect: allow, deny, or ask.
- **Rule** / **Ruleset** — persistent permission rules (the "always" replies become rules).
- Events: `permission.v2.asked` (Asked), `permission.v2.replied` (Replied).

So tool approval is: tool call → `permission.v2.asked` event → operator replies `once`/`always`/`reject` → `permission.v2.replied` event. "always" persists as a Rule.

## Question (agent→operator question prompt)

From `question.ts`. Question IDs start with `que`.

- **Option** — `{ label, ... }` a selectable option.
- **Info** — question info; `custom: Schema.optional(Schema.Boolean)` (allow custom-typed answer, default true).
- **Prompt** — the question prompt.
- **Tool** — `{ messageID, callID }` (a question can be associated with a tool call).
- **Request** — the question request (with options, info, tool ref).
- **Answer** — `Schema.Array(Schema.String)` — answers are arrays of selected labels (multi-select).
- **Reply** — `{ answers: Array<Answer> }`.
- **Replied** / **Rejected** — the operator either answers (Replied) or rejects the question (Rejected).
- Events: `question.asked` (Asked), `question.replied` (RepliedEvent), `question.rejected` (RejectedEvent).

So questions are a structured multi-choice prompt the agent sends to the operator; the operator replies with selected labels or rejects.

## Event manifest composition

`event-manifest.ts` composes: session V1 durable + live definitions, SessionEvent, Permission, Plugin, Question, FileSystem, Pty, plus Installation/Mcp/Lsp/Vcs/Workspace/Worktree events. The full outbound event set spans session, permission, question, pty, file-system, MCP, LSP, VCS (git), workspace, worktree.
