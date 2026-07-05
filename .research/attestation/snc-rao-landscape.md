---
source_handle: snc-rao-landscape
fetched: 2026-07-04
source_path: /home/agent/projects/SNC/.research/analysis/briefs/remote-agent-operation-landscape.md
provenance: source-direct
source_class: cross-corpus-attestation
note: Cross-corpus pointer to the operator's SNC synthesis brief (2026-06-03) — the prior landscape work this campaign should have built on rather than re-derived. The brief's central spawn-vs-pilot distinction is the load-bearing framing.
---

# Cross-corpus pointer: SNC remote-agent-operation landscape (prior synthesis)

The canonical brief is at `/home/agent/projects/SNC/.research/analysis/briefs/remote-agent-operation-landscape.md` (provenance: agent-synthesis, 2026-06-03, campaign `remote-agent-operation`).

## Load-bearing framing

1. **Spawn vs pilot is the central cut** in the remote-agent-operation space: "Pilot — steer a session that is already running on a machine that has the codebase. Solved, mature, low-friction. Spawn — cold-start a fresh session against the codebase from a remote device. This is the actual gap." `[snc-rao-landscape]` — the patchbay survey's "provision vs drive" is revised to adopt this framing plus the finer attach/operate split.
2. **Four spawn mechanisms are documented**: Claude Code Remote Control server mode (`--spawn worktree --capacity N`), Dispatch (Claude Desktop), SSH+tmux, OpenCode `serve`+`attach`. Each requires an already-running host process. `[snc-rao-landscape]`
3. **The deploy guide exists** at `/home/agent/projects/SNC/docs/ops/remote-agent-piloting.md` — a complete systemd-unit setup (`ExecStart=...claude remote-control --spawn worktree --capacity 8`) the operator built and deployed, standing up Claude Code Remote Control as an always-on service to enable spawn from a remote device. This is documented, deployed prior art for the provision action.
