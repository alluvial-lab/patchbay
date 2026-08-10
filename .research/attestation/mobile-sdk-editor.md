---
source_handle: mobile-sdk-editor
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/pi-extension/node_modules/@earendil-works/pi-coding-agent/dist/modes/interactive/interactive-mode.js
provenance: source-direct
---

# Source summary

Installed Pi SDK interactive mode shows custom editor construction and callback copying, rather than delegation to a default editor instance.

## Key passages

{1} > `setEditorComponent` stores the factory and `getEditorComponent` returns the factory.

{2} > The custom editor is created with `tui`, theme, and keybindings; callbacks from the default editor are copied (`onSubmit`, `onChange`), and only current text is copied.

{3} > The normal submit path clears the current editor text before invoking `editor.onSubmit(text)`.

## Metadata

- Repository: `/home/agent/projects/outpost_pi`
- Installed SDK: `@earendil-works/pi-coding-agent` 0.80.6
- Source type: installed local SDK source
