---
source_handle: coder-workspaces
fetched: 2026-08-08
source_url: https://raw.githubusercontent.com/coder/coder/main/docs/user-guides/workspace-lifecycle.md
provenance: source-direct
---

## Summary
Coder models a workspace as a lifecycle-bearing compute environment created from a template. Resources may be persistent or ephemeral: stopping destroys ephemeral resources and leaves persistent resources idle; restarting recreates ephemeral resources. Workspace states include running, stopped, deleted, and error-derived failed/unhealthy states. Templates define the resources/environment, while workspace creation supplies a named instance.

## Key passages
- Workspaces are flexible, reproducible, isolated units of compute; their resources may be ephemeral or persistent. (Workspace ephemerality)
- States listed are Running, Stopped, and Deleted; errors can produce Failed or Unhealthy states. (Workspace States)
- A workspace is created from a template; templates generally define its resources and environment. (Workspace creation)
- A stopped workspace may resume running by manual start or user connection when automatic start is enabled. (Stopping workspaces)
- Deletion normally destroys resources and removes the workspace record, with an explicit orphan option for deleting the workspace without deleting resources. (Deleting workspaces)
