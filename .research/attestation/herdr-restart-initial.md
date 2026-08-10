---
source_handle: herdr-restart-initial
fetched: 2026-08-09
source_path: 71fac20c33751cd586584b087ce73e45d855b0af:scripts/herdr-restart-agents.sh
provenance: source-direct
---

## Summary

Initial bulk restart script for reloading a rebuilt extension across Herdr-managed Pi processes.

## Key passages

1. The commit message states that Herdr 0.7.5 had no restart/reload command and that updating all agents required “graceful `/quit` + `herdr agent start --continue` per agent.”
2. The script defaults to skipping workspace `w3` because restarting the workspace containing the current conversation terminates that conversation; selection otherwise uses workspace IDs through `--only` and `--skip`.
3. It discovers target panes from `herdr pane list`, filters them by cwd containing `/home/agent/projects/`, and determines wrapper use by testing whether the cwd ends in `outpost_pi`.
4. Its first lifecycle algorithm sends `/quit\n` with `herdr pane send-text`, polls `herdr agent list`, attempts a best-effort literal `C-c` if the agent remains, and then relaunches in the same pane.
5. Relaunch uses the workspace ID as the agent name and always passes `--continue` for directly managed agents.

## Structural metadata

- Artifact type: Git commit introducing an operations shell script
- Commit: `71fac20c33751cd586584b087ce73e45d855b0af`
- Commit date: 2026-08-04
- Path: `scripts/herdr-restart-agents.sh`
- Subsequent corrective commits: `aa11655`, `e091a56`
