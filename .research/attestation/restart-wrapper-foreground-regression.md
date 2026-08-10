---
source_handle: restart-wrapper-foreground-regression
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi@8bc44f442da3b09596a02a1de603aab4007296c6:scripts/pi-restart-loop.sh
provenance: source-direct
---

# Attestation: foreground TUI fix weakened restart-marker correlation

## Structural metadata

- Source type: corrective commit and script diff
- Commit: `8bc44f442da3b09596a02a1de603aab4007296c6`
- Path: `scripts/pi-restart-loop.sh`
- Scope used here: conflict between terminal ownership and exact PID fencing

## Paraphrased summary

The prior wrapper captured the Pi child PID by backgrounding the process, but Pi is a TUI and exited immediately when detached from the terminal. The fix ran Pi in the foreground. Because the shell no longer had the child PID, the wrapper replaced exact `.restart-marker-<child_pid>` lookup with a glob that accepted the first marker found in the shared state directory.

## Key passages

### {1} Root cause of the foreground fix

Anchor: commit message.

> "The prior version backgrounded pi (pi &; wait) to capture the PID for the PID-scoped marker. But pi is a TUI app — backgrounding disconnects the terminal, so pi detects 'not a TTY' and exits immediately with code 0."

### {2} Exact-PID behavior removed

Anchor: removed script lines 22-35.

The old code captured `$!`, waited for that child, and required `.restart-marker-$child_pid` before relaunch.

### {3} Replacement behavior accepts any marker

Anchor: added script lines in the exit-zero branch.

The new code runs Pi in the foreground, loops over `$REMOTE_DIR/.restart-marker-*`, selects the first non-symlink entry, removes it, and relaunches. The commit message says the wrapper "just checks presence of any recent marker."

### {4} Security and lifecycle intent in the surrounding history

The source's before/after comments show that the removed exact-PID lookup had been intended to prevent multi-Pi wrappers from consuming one another's restart intent; that intent is absent from the replacement behavior.
