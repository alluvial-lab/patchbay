---
source_handle: mobile-sdk-contract
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/pi-extension/node_modules/@earendil-works/pi-coding-agent/dist/core/extensions/types.d.ts; dist/core/extensions/runner.js; dist/core/agent-session.js
provenance: source-direct
---

# Source summary

Installed Pi SDK 0.80.6 definitions and runtime show that session-control belongs to the command context, ordinary contexts are constructed without those methods, and `sendUserMessage` disables command handling.

## Key passages

{1} > Extended context for command handlers. Includes session control methods only safe in user-initiated commands.

{2} > `ExtensionCommandContext` declares `newSession`, `fork`, `switchSession`, and `reload`.

{3} > Create an `ExtensionContext` for use in event handlers and tool execution.

{4} > `sendUserMessage` uses `prompt()` with `expandPromptTemplates: false` to skip command handling and template expansion.

{5} > `prompt()` handles extension commands first only when `expandPromptTemplates` is true and the text starts with `/`.

## Metadata

- Repository: `/home/agent/projects/outpost_pi`
- Installed SDK: `@earendil-works/pi-coding-agent` 0.80.6
- Source type: installed local SDK source
