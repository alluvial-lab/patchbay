---
source_handle: herdr-wrapper-tty-fix
fetched: 2026-08-09
source_path: 8bc44f442da3b09596a02a1de603aab4007296c6:scripts/pi-restart-loop.sh
provenance: source-direct
---

## Summary

Commit correcting a restart wrapper that backgrounded an interactive Pi TUI in order to capture its PID, causing Pi to detect the absence of a terminal and exit.

## Key passages

1. The commit message says the prior wrapper used `pi &; wait` to capture a PID for a PID-scoped restart marker, but “backgrounding disconnects the terminal,” so Pi detected “not a TTY” and exited with code 0.
2. Because the exit was graceful and no marker existed, the wrapper interpreted it as a normal stop and ended the loop.
3. The correction runs Pi in the foreground so it owns the terminal, then searches for a restart marker after exit instead of constructing the exact child-PID marker path.
4. The revised script preserves the rule that only exit code 0 plus a non-symlink restart marker authorizes relaunch; exit 0 without a marker and all nonzero exits stop the loop.
5. The diff changes an exact-child marker check to the first matching `.restart-marker-*` glob, while the comment says the PID remains useful to extension tracking rather than wrapper matching.

## Structural metadata

- Artifact type: corrective Git commit
- Commit: `8bc44f442da3b09596a02a1de603aab4007296c6`
- Commit date: 2026-07-31
- Path: `scripts/pi-restart-loop.sh`
- Corrected assumption: an interactive TUI can be backgrounded while retaining terminal behavior
