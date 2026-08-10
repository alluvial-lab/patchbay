---
source_handle: herdr-ancestor-fix
fetched: 2026-08-09
source_path: d1773edd692a6e33757a853e3a93d25d11d3db7b:scripts/hot-reload.sh
provenance: source-direct
---

## Summary

Commit correcting process discovery for shell-triggered hot reload by replacing immediate-parent PID assumptions with an ancestor-chain search for a runtime identity file.

## Key passages

1. The commit message says the immediate parent may be “an intermediate bash/subshell (e.g. the agent's bash tool), not pi itself.”
2. The old algorithm read only the parent PID of the shell and expected `.runtime-self-<parent-pid>` to exist.
3. The replacement begins at the shell PID, walks parent PIDs until PID 1, and stops at the nearest ancestor with a validated `.runtime-self-<PID>` file.
4. The located identity file is owner/mode validated, parsed for a PID-matching runtime identity and non-empty nonce, and used to create `.hot-reload-armed-<PID>` with exclusive creation.

## Structural metadata

- Artifact type: corrective Git commit
- Commit: `d1773edd692a6e33757a853e3a93d25d11d3db7b`
- Commit date: 2026-07-31
- Path: `scripts/hot-reload.sh`
- Corrected assumption: immediate parent process is Pi
