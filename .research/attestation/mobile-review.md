---
source_handle: mobile-review
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/active/features/feature-mobile-slash-command-invocation.md (commit 7ceb72f)
provenance: source-direct
---

# Source summary

The source records the design review that rejected editor-seam driving as the durable foundation.

## Key passages

{1} > `setEditorComponent`'s factory receives `(tui, theme, keybindings)` but NOT the default editor instance.

{2} > Installing a custom editor constructs a NEW editor copying only selected state ... NOT history/full state; refs reset across `/reload`; conflicts with other extensions' custom editors.

{3} > `onSubmit` is a user-event callback, not an injection contract.

{4} > direct invocation bypasses the editor's pre-clear, so a mobile `/new`/`/reload` can **clobber text being composed in the TUI**.

{5} > No robust ack ... many commands give NO completion signal.

{6} > `AgentSession.prompt()` does NOT error on unknown commands — it falls through to **sending the slash-prefixed text to the model as a prompt**.

{7} > The durable foundation is an **upstream host-operation/submit-input API** ... Until that exists: retain typed actions ... and at most ship the editor-seam as a version-pinned, TUI-only experimental bridge for a CURATED set.

## Metadata

- Repository: `/home/agent/projects/outpost_pi`
- Commit: `7ceb72f`
- Source type: local repository design-review record
