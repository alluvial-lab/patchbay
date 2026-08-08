---
source_handle: pi-extensions
fetched: 2026-08-08
source_path: /home/agent/.local/lib/node_modules/@earendil-works/pi-coding-agent/docs/extensions.md
provenance: source-direct
---

## Summary
Pi extensions are TypeScript modules discovered globally, project-locally, or through explicit/package paths. `/reload`/`ctx.reload()` tears down the current extension runtime, reloads resources, and starts a new extension runtime in the same process/session. The documentation describes resources and extension binding, but does not state that a freshly rebuilt package `/dist` is guaranteed to replace already loaded compiled modules; process restart is therefore the reliable code-upgrade boundary for compiled extension dependencies.

## Key passages

1. Auto-discovered extensions in `~/.pi/agent/extensions/` or `.pi/extensions/` can be hot-reloaded with `/reload`; `-e` is for quick tests. (`Extension Locations`)
2. Startup and reload both rediscover skill, prompt, and theme paths; reload uses `reason: "reload"`. (`resources_discover`)
3. `session_start` fires for startup, reload, new, resume, and fork; the session replacement and reload lifecycle hooks are documented. (`session_start`)
4. `session_shutdown` receives `reason: "quit" | "reload" | "new" | "resume" | "fork"`; extensions should clean up session-scoped resources there. (`session_shutdown`)
5. `ctx.reload()` emits `session_shutdown`, reloads resources, then emits `session_start` with `reason: "reload"` and `resources_discover` with `reason: "reload"`. The invoking command continues in its old call frame; code after the await remains old code, and future commands/events/tool calls use the new extension version. The recommended pattern is `await ctx.reload(); return;`. (`ctx.reload()`)
6. Tools cannot call `ctx.reload()` directly; a command is the reload entrypoint and a tool may queue that command as a follow-up. (`ctx.reload()`)
7. After new/resume/fork, the old session runtime is torn down and a new extension instance is bound before the new `session_start`. Captured old session-bound objects are stale, cleanup may already have discarded extension variables/resources, and the new instance must reestablish any needed in-memory state. (`session_before_switch`; `session_before_fork`; `Session replacement lifecycle and footguns`)
8. `pi.appendEntry()` writes custom extension data into the persisted session. The documented restoration pattern scans those custom entries during `session_start` to reconstruct state; ordinary in-memory extension variables are not automatically persisted across replacement or process restart. (`pi.appendEntry`; `State Management`; `session_start`)

## Source-internal anchors
`docs/extensions.md`: “Extension Locations”; “resources_discover”; “session_start”; “session_shutdown”; “ctx.reload()” (around lines 1276–1327); “State Management”.
