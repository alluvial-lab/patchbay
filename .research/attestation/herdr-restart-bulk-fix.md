---
source_handle: herdr-restart-bulk-fix
fetched: 2026-08-09
source_path: e091a56d2c62919c675ed31f0540087faeb6f216:scripts/herdr-restart-agents.sh
provenance: source-direct
---

## Summary

Second corrective bulk-restart commit addressing invalid agent names, relaunch races, and unsafe interruption of agents that were not idle.

## Key passages

1. The commit message says a bulk run exposed that Herdr agent names must be lowercase: workspace IDs `wA`, `wB`, `wC`, and `wE` failed with `invalid_agent_name`, while `w4` through `w9` passed.
2. The same bulk run exposed a race with Herdr's pane-idle detection immediately after SIGTERM, so some relaunches did not stick.
3. The corrective code lowercases the workspace ID, retries one failed start after a three-second delay, and pauses two seconds between agents.
4. A new state guard permits restart only when `agent_status` is `idle` or `done`; other states are skipped and deferred, with the commit message naming a mid-turn agent as the motivating case.
5. The script still selects targets and reports them using generated workspace IDs, while wrapper classification remains a cwd-suffix test.

## Structural metadata

- Artifact type: corrective Git commit
- Commit: `e091a56d2c62919c675ed31f0540087faeb6f216`
- Commit date: 2026-08-04
- Path: `scripts/herdr-restart-agents.sh`
- Predecessors: `71fac20`, `aa11655`
