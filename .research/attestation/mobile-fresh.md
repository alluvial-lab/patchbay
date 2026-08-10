---
source_handle: mobile-fresh
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/pi-extension/src/index.ts; scripts/pi-restart-loop.sh; .work/active/stories/story-new-session-restart-fresh-extension-exit.md (commit ea6b5fd)
provenance: source-direct
---

# Source summary

The source records the implemented restart-fresh handshake and its safety gate for interactive `/new`.

## Key passages

{1} > when no command-capable context exists, acknowledge + reset + exit with `EXIT_FRESH_SESSION` only if the daemon supervisor or interactive restart wrapper owns the process.

{2} > Unmanaged interactive processes return a structured `action_error`; they never exit.

{3} > The shared handshake is `EXIT_FRESH_SESSION=42` ... the interactive wrapper now key[s] on the same code.

{4} > Until that migration, `/new` on those agents fails safely with `fresh_session_restart_unavailable` and does not kill the pane.

Script excerpt:

{5} > Mobile /new exits 42; that authorizes exactly one relaunch without `--continue`.

{6} > `export OUTPOST_PI_UNDER_RESTART_WRAPPER=1`

{7} > A fresh-session exit relaunches Pi once without `--continue`.

## Metadata

- Repository: `/home/agent/projects/outpost_pi`
- Commit: `ea6b5fd`
- Source type: local repository implementation and work record
