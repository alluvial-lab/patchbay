---
source_handle: mobile-newctx
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/backlog/backlog-mobile-new-button-newsession-no-command-ctx.md (commit 304b8b8)
provenance: source-direct
---

# Source summary

The source records the confirmed SDK lifecycle for `newSession` and the resulting mobile New-button failure.

## Key passages

{1} > The installed `@earendil-works/pi-coding-agent` 0.80.6 does **not** provide an `ExtensionCommandContext` to `session_start` or ordinary turn/event handlers.

{2} > The SDK creates an `ExtensionCommandContext` only when `AgentSession._tryExecuteExtensionCommand()` invokes a registered extension slash command.

{3} > There is no public `ExtensionAPI` method to obtain the runner's command context, to invoke an extension command programmatically without an LLM turn, or to replace the host's active `AgentSessionRuntime`.

{4} > The defect is therefore not an uncaught process crash or silent drop: the command is explicitly rejected and the required fresh session is not created.

## Metadata

- Repository: `/home/agent/projects/outpost_pi`
- Commit: `304b8b8`
- Source type: local repository work record
