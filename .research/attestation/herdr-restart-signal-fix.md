---
source_handle: herdr-restart-signal-fix
fetched: 2026-08-09
source_path: aa116552262d719c10b3af9cd3fc67f507538798:scripts/herdr-restart-agents.sh
provenance: source-direct
---

## Summary

Commit replacing terminal-text shutdown with direct process discovery and signaling after validation showed that injected TUI commands did not terminate Pi.

## Key passages

1. The commit message records validation on workspace `w6`: “herdr pane send-text doesn't drive the pi TUI input (`/quit` never landed; literal `C-c` can't send a real interrupt),” leaving Pi running and causing `agent_pane_busy` on relaunch.
2. The replacement obtains `foreground_processes[0].pid` from `herdr pane process-info --pane <id>` and sends that PID `SIGTERM`.
3. The source describes SIGTERM as Pi's graceful `session_shutdown` path, including `working=false` publication and relay drain; after a deadline, it escalates to `SIGKILL`.
4. The commit message records successful validation on `w6`, where the old PID was replaced and the agent resumed idle on rebuilt code.
5. Although the executable algorithm changed, the surrounding script comment and dry-run description still said it would send `/quit`; the helper `pane_still_running_pi` also remained after the PID-poll replacement.

## Structural metadata

- Artifact type: corrective Git commit
- Commit: `aa116552262d719c10b3af9cd3fc67f507538798`
- Commit date: 2026-08-04
- Path: `scripts/herdr-restart-agents.sh`
- Validation target named in commit: workspace `w6`
