---
source_handle: mobile-editor-spike
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/backlog/backlog-mobile-new-button-newsession-no-command-ctx.md (commit 7373273)
provenance: source-direct
---

# Source summary

The source records a probe of installed Pi SDK 0.80.6 and inventories possible prompt/command ingress paths.

## Key passages

{1} > There is, however, a **TUI-only indirect submission seam** through the public custom-editor API.

{2} > Calling the retained component's `onSubmit?.("/new")` therefore enters the exact handler that recognizes `/new`.

{3} > This is not a documented command API; it is a UI-component callback seam backed by current InteractiveMode implementation.

{4} > `pi.sendUserMessage` — no. Its implementation deliberately calls `prompt(..., { expandPromptTemplates: false, source: "extension" })`, skipping extension-command dispatch ... A slash string becomes an agent prompt.

{5} > `ctx.ui.setEditorComponent` + retained `onSubmit` — yes, TUI only, indirect.

{6} > The custom-editor bridge works only when `ctx.mode === "tui"` ... It replaces or wraps the user's editor, can conflict with other custom-editor extensions, exposes a `void` callback rather than an awaitable success/error result, and becomes stale across `/new`, `/resume`, `/fork`, and `/reload`.

## Metadata

- Repository: `/home/agent/projects/outpost_pi`
- Commit: `7373273`
- Source type: local repository work record
