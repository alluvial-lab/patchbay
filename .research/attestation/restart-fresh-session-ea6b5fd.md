---
source_handle: restart-fresh-session-ea6b5fd
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi@ea6b5fd7ee5e15e86de4db98f162f4eab7a70ef8:pi-extension/src/index.ts;pi-extension/src/daemon/rpc_child.ts;pi-extension/src/daemon/supervisor.ts;pi-extension/src/pi_restart_loop.test.ts;scripts/pi-restart-loop.sh;.work/active/stories/story-new-session-restart-fresh-restart-mechanism.md
provenance: source-direct
---

# Attestation: restart-fresh extension and wrapper contract

## Structural metadata

- Source type: implementation commit, test, and completed story record
- Commit: `ea6b5fd7ee5e15e86de4db98f162f4eab7a70ef8`
- Scope used here: distinction between restart-as-continuation and restart-fresh, manager capability gating, and current wrapper marker behavior

## Paraphrased summary

This commit generalized exit code 42 as a process-manager request for exactly one restart without `--continue`. Daemon supervisor and interactive wrapper both understand it. The extension only uses this direct-exit path when an owning supervisor or wrapper is declared by environment; unmanaged interactive agents return a structured error. The wrapper continues to use the separate exit-zero-plus-marker path for hot-reload continuation.

## Key passages

### {1} Fresh-session operation is distinct from continuation restart

Anchor: `pi-extension/src/daemon/rpc_child.ts` and `scripts/pi-restart-loop.sh`.

`EXIT_FRESH_SESSION = 42` means the next launch omits `--continue` once. Hot-reload remains exit zero plus a marker and relaunches with `--continue`; later restarts return to continuation behavior.

### {2} Process-manager capability gate

Anchor: `pi-extension/src/index.ts` → `case "session_new"`.

When no command-context `newSession` capability exists, the extension checks `OUTPOST_PI_DAEMON=1` or `OUTPOST_PI_UNDER_RESTART_WRAPPER=1`. Without either, it sends `fresh_session_restart_unavailable` and does not exit.

### {3} Direct exit after a fixed acknowledgment window

Anchor: same branch.

For a manager-owned process, the extension sends `action_ok`, resets its session-scoped projection, and schedules `process.exit(EXIT_FRESH_SESSION)` after 100 ms.

### {4} Wrapper marker behavior at this commit

Anchor: `scripts/pi-restart-loop.sh:57-72`.

On exit zero, the wrapper scans `$REMOTE_DIR/.restart-marker-*`, takes the first existing non-symlink marker, removes it, and restarts with `--continue`. It does not correlate the marker filename to the Pi process that just exited.

### {5} Wrapper test coverage

Anchor: `pi-extension/src/pi_restart_loop.test.ts`.

The test proves this sequence: initial `--continue` launch exits 42; the next launch omits `--continue`; that fake process creates its own marker and exits zero; the third launch resumes with `--continue`. The test uses one wrapper and one marker and does not exercise a foreign marker from another process.

### {6} Manager coverage remains deployment-dependent

Anchor: `.work/active/stories/story-new-session-restart-fresh-restart-mechanism.md` → `Implementation notes`.

The story states that 11 Herdr-managed agents were intentionally not migrated to the wrapper. Until an operational follow-up, mobile `/new` on those agents fails safely rather than restarting fresh.

### {7} Code comment and wrapper behavior diverge

Anchor: `pi-extension/src/index.ts` → `_restartMarkerPath`; `scripts/pi-restart-loop.sh:57-72`.

The extension comment says the wrapper validates the marker against its child PID before relaunch. The wrapper implementation at the same commit instead scans for any `.restart-marker-*` and selects the first existing non-symlink marker without comparing its suffix or contents to the exited process.
