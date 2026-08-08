---
source_handle: herdr-concepts
fetched: 2026-08-08
source_url: https://raw.githubusercontent.com/herdrdev/herdr/master/docs/next/website/src/content/docs/concepts.mdx
provenance: source-direct
---

## Summary
Herdr is a terminal workspace manager around real terminal processes. It distinguishes workspace, tab, pane, recognized agent, and persistent server session. The server owns panes/process state; clients attach and detach. Named sessions create separate runtime namespaces including panes, tabs, workspaces, sockets, and persisted runtime state.

## Key passages
- A workspace is a top-level project container, commonly one per repo, task, or investigation; it owns tabs and panes. (Workspace)
- A pane is a real terminal whose output and input are managed and whose process is preserved across client detach. (Pane)
- An agent is a process Herdr recognizes inside a pane; detection uses foreground process, screen manifests, and optional integrations. (Agent)
- A session is a persistent Herdr server namespace; named sessions are separate runtime namespaces. (Session)
- In the default background-server/client mode, the server owns panes and process state and the client is attached to that server; detaching the client leaves server and agents running. (Server and client)
