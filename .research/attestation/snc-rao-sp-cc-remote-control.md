---
source_handle: snc-rao-sp-cc-remote-control
fetched: 2026-07-04
source_path: /home/agent/projects/SNC/.research/attestation/rao-sp-cc-remote-control.md
provenance: source-direct
source_class: cross-corpus-attestation
note: This attestation lives in the operator's broader SNC research corpus (fetched 2026-06-03 from https://code.claude.com/docs/en/remote-control). Cited here because patchbay's harness-action-surfaces engagement was scoped to patchbay's .research/ and missed the operator's prior grounded research on the same topic (see the campaign's methodology-correction note). The underlying source is the Claude Code Remote Control docs.
---

# Cross-corpus pointer: Claude Code Remote Control (SNC attestation)

The canonical attestation is at `/home/agent/projects/SNC/.research/attestation/rao-sp-cc-remote-control.md` (and a parallel `rao-ae-claude-code-remote-control.md`), fetched 2026-06-03 from `https://code.claude.com/docs/en/remote-control`.

## Load-bearing claims (for citation by handle)

1. **Remote Control server mode (`claude remote-control`) with `--spawn <mode>` exposes operator-action session provisioning.** `--spawn worktree` creates a fresh git-worktree-isolated session per remote connection; `--spawn same-dir` (default) shares cwd; `--spawn session` is single-session; `--capacity N` caps concurrent sessions (default 32). The server daemon must be pre-running on the host. `[snc-rao-sp-cc-remote-control]{1}` — this is the spawn primitive, prior art for patchbay's provisioning.
2. **Dispatch (Claude Desktop mobile → spawn a Code session) is a second Claude-ecosystem spawn mechanism.** Mobile-app-triggered, runs via the Desktop app on a macOS/Windows host. `[snc-rao-sp-cc-desktop]{2}`
3. **The spawn-vs-pilot distinction is the operator's own load-bearing framing** of this space, established in the SNC `remote-agent-operation-landscape.md` synthesis brief (2026-06-03): "Pilot — steer a session that is already running. Spawn — cold-start a fresh session against the codebase from a remote device." The patchbay survey's "provision vs drive" vocabulary is revised to adopt this framing plus the finer attach/operate split (see campaign parent.md).
4. **Transport relays through the Anthropic API** (outbound HTTPS, no inbound ports) — execution stays local but transport is not self-hosted end-to-end. `[snc-rao-se-cc-remote-control]{1}` (separate attestation in SNC).
