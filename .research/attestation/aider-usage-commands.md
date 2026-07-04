---
source_handle: aider-usage-commands
fetched: 2026-07-03
source_url: file:///tmp/aider/aider/website/docs/usage/commands.md
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: documented in-chat command inventory and operator interactions

## Core findings
- Usage page enumerates slash commands including `/add`, `/clear`, `/commit`, `/diff`, `/drop`, `/exit`, `/git`, `/help`, `/load`, `/model`, `/models`, `/read-only`, `/run`, `/save`, `/settings`, `/undo`, `/voice`, `/weak-model`, `/web`, plus mode commands (`/ask`, `/code`, `/architect`, `/context`, `/chat-mode`, `/ok`, `/edit`).
- `/run` is documented as shell execution and alias `!`.
- Keybindings section confirms interrupt and terminal-style editing keys; docs include “Interrupting with CONTROL-C is always safe” and mention prompt history behavior.
- In-keyboard interaction docs show `Ctrl-C` safety language consistent with in-loop interruption behavior.

## Evidence snippets
1) Command table rows in usage docs list major command/action surface.
2) Notes around “Interrupting with CONTROL-C” indicates resumable cancellation model.
3) File references keybinding list for interactive operator controls.