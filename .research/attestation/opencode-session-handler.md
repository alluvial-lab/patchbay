---
source_handle: opencode-session-handler
fetched: 2026-07-04
source_path: /tmp/opencode/packages/opencode/src/server/routes/instance/httpapi/handlers/session.ts
provenance: source-direct
---

# OpenCode session HTTP API handler

The session handler (`packages/opencode/src/server/routes/instance/httpapi/handlers/session.ts`) registers the inbound operator→agent control verbs over a typed HTTP API (effect/HttpApi). Each verb is an `Effect.fn` endpoint.

## Inbound control verbs (operator→agent)

From the handler's endpoint registrations and service wiring:

- **`prompt`** — `SessionHttpApi.prompt`, payload `PromptPayload`, calls `promptSvc.prompt(...)`. Synchronous (waits for the prompt result).
- **`promptAsync`** — `SessionHttpApi.promptAsync`, same payload, calls `promptSvc.prompt(...)` fire-and-forget, returns immediately. Logs `"prompt_async failed"` on error.
- **`shell`** — `SessionHttpApi.shell`, calls `promptSvc.shell(...)`, errors mapped via `SessionError.mapBusy` (returns a busy error if the session is busy).
- **`status`** — `SessionHttpApi.status`, returns `statusSvc.list()` (session status list).
- **`summary.diff`** — returns `summary.diff({ sessionID, messageID })` (a diff of the session summary at a message).
- **`share`** / **`unshare`** — `SessionShare.Service` share/unshare the session; errors mapped to typed 500 (storage/network failures).
- **`remove`** — removes a session (`session.remove`), then `promptSvc.cancel(sessionID)` (cancel any in-flight prompt).
- **`revert`** / **`unrevert`** — `SessionRevert.Service` revert/unrevert (git-pair revert semantics), busy-mapped.
- **`compact`** — `SessionCompaction.Service` create compaction (`compactSvc.create(...)`), preceded by `revertSvc.cleanup`.
- **`loop`** — `promptSvc.loop({ sessionID })` (re-runs the prompt loop).
- **`removeMessage`** — `session.removeMessage(ctx.params)` (removes a specific message).

The handler imports: `SessionShare`, `SessionPrompt`, `SessionRevert`, `SessionStatus`, `SessionSummary`, `SessionCompaction`, `Session` (the core session module). Busy errors map via `SessionError.mapBusy`.

## Structural notes

- The control surface is a typed HTTP API (effect/HttpRouter), not a custom RPC. Routes are grouped: `rootApiRoutes` (`/global/*` and control routes), `eventApiRoutes` (SSE), `ptyConnectApiRoutes` (WebSocket upgrade), `instanceApiRoutes` (instance routes), `uiRoute` (catch-all).
- Auth is router middleware (`authorizationRouterMiddleware`); public static assets bypass it.
- Two prompt shapes: synchronous (`prompt`) and async (`promptAsync`). `shell` is a third prompt variant.
- Revert/unrevert is first-class — OpenCode treats git-revert as a session-level operator action, not just a tool.
