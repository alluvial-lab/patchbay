---
source_handle: herdr-setup-pane-fix
fetched: 2026-08-09
source_path: "d4d5561425a7edae0a6e72e4f9b97687c60e9b31:scripts/herdr-setup.sh; d4d5561425a7edae0a6e72e4f9b97687c60e9b31:scripts/herdr-start-agents.sh"
provenance: source-direct
---

## Summary

Commit correcting workspace-to-pane discovery in the multi-working-directory setup script and adding a separate script for starting agents in already-created workspaces.

## Key passages

1. The commit message says the lookup through `herdr pane list --json` failed because it filtered on the wrong field name; it replaced that lookup with the workspace-create response path `result.root_pane.pane_id`.
2. `herdr-setup.sh` contains a static twelve-entry `label|cwd|use_wrapper` registry with absolute paths, including both top-level and nested project directories.
3. Workspace creation collapses two conditions into one operator message: “workspace already exists or failed — skipping.” A missing parsed pane ID yields “could not find pane … start pi manually.”
4. The launch path branches on `use_wrapper`: outpost_pi receives a shell command through `pane send-text`; other projects use `agent start <label> --kind pi --pane <pane_id> -- --continue`.
5. The new `herdr-start-agents.sh` independently reparses `pane list --json`, reads `workspace_label`/`workspace_id`, `id`, and `cwd`, and falls back from failed `agent start` to injecting `pi --continue` through `pane send-text`.

## Structural metadata

- Artifact type: Git commit and two shell-script paths
- Commit: `d4d5561425a7edae0a6e72e4f9b97687c60e9b31`
- Commit date: 2026-08-03
- Parent behavior visible in diff: pane-list lookup by `workspace_label`
- Corrected behavior: pane ID read from workspace-create response
