---
source_handle: restart-herdr-restart-arc
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi git commits 71fac20c33751cd586584b087ce73e45d855b0af, aa116552262d719c10b3af9cd3fc67f507538798, e091a56d2c62919c675ed31f0540087faeb6f216 -- scripts/herdr-restart-agents.sh
provenance: source-direct
---

# Attestation: Herdr fleet restart bug-swatting arc

## Structural metadata

- Source type: three-commit operational history
- Commits: `71fac20`, `aa11655`, `e091a56`
- Path: `scripts/herdr-restart-agents.sh`
- Scope used here: external restart control, PID discovery, readiness checks, and settle/retry behavior

## Paraphrased summary

The first script attempted to restart every managed Pi by injecting `/quit`, waiting for Herdr's agent list to clear, and then starting with `--continue`. Validation showed that TUI text injection and literal interrupt text were not reliable. The next revision discovered the foreground process PID, sent SIGTERM, polled until the PID disappeared, and escalated to SIGKILL. A bulk run then exposed name validation and pane-idle races, producing lowercasing, an idle/done guard, delay, retry, and inter-agent settling.

## Key passages

### {1} Process restart is required for fleet code upgrade

Anchor: `71fac20` commit message and script header.

The commit says Herdr 0.7.5 had no restart/reload command and Pi `/reload` could not load rebuilt ESM extension code, so fleet adoption of a new `dist/` required quit plus relaunch with `--continue`.

### {2} TUI text injection failed as a lifecycle command

Anchor: `aa11655` commit message.

> "Validation on w6 showed herdr pane send-text doesn't drive the pi TUI input (/quit never landed; literal 'C-c' can't send a real interrupt), so pi never exited and herdr agent start refused with agent_pane_busy."

### {3} External PID signal path

Anchor: `aa11655` diff.

The revised script reads `foreground_processes[0]["pid"]` from `herdr pane process-info`, sends SIGTERM, polls with `os.kill(pid, 0)`, escalates to SIGKILL after a deadline, waits for the pane to settle, and then relaunches.

### {4} Snapshot status guard

Anchor: `e091a56` diff.

Before signaling, the script checks the pane's `agent_status` from the earlier pane-list snapshot and skips any value other than `idle` or `done`.

### {5} Relaunch readiness race

Anchor: `e091a56` commit message and diff.

> "rapid starts raced herdr's pane-idle detection right after SIGTERM, so some relaunches didn't stick"

The fix retries once after three seconds and waits two seconds before operating on the next pane.

### {6} Naming failure surfaced only in the bulk run

Anchor: `e091a56` commit message.

Workspace ids containing uppercase letters failed `invalid_agent_name`; the script changed the launch name to lowercase.
