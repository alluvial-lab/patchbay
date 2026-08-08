---
source_handle: herdr-state
fetched: 2026-08-08
source_url: https://raw.githubusercontent.com/herdrdev/herdr/master/docs/next/website/src/content/docs/session-state.mdx
provenance: source-direct
---

## Summary
Herdr explicitly separates live process persistence, layout restoration, screen-history replay, native agent-session restoration, and live handoff. Detach preserves processes. Full server restart restores shape but not arbitrary processes; native agent references may resume agent conversations. Unsupported, missing, invalid, duplicated, or stale references fall back to normal shells in the saved pane directory. Handoff is a distinct best-effort process-preservation path, while transient requests and streams may be interrupted.

## Key passages
- The state table says detach/reattach keeps processes running and resumes agent conversation because the process never stopped; server restart restores layout, may restore recent screen from history, and resumes conversation only through native agent session restore. (State table)
- After server stop/start, Herdr restores workspaces, tabs, panes, cwd, layout, and focus; snapshot restore does not preserve running shells, servers, tests, or arbitrary processes. (Snapshot restore)
- Native restore uses official integration-reported session references; unsupported, missing, invalid, duplicated, or stale references restore as normal shells in the saved directory. (Native agent session restore)
- Live handoff attempts to transfer live panes so processes survive replacement, but in-flight requests, waits, subscription streams, client sockets, and pane-to-pane messages may be interrupted; clients should reconnect and retry. (Live handoff)
