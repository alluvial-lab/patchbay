---
source_handle: snc-rao-ae-opencode-cli
fetched: 2026-07-04
source_path: /home/agent/projects/SNC/.research/attestation/rao-ae-opencode-cli.md
provenance: source-direct
source_class: cross-corpus-attestation
note: Cross-corpus pointer to the operator's SNC research (fetched 2026-06-03). The patchbay OpenCode specialist missed that OpenCode exposes a `serve` + `attach` spawn/attach surface (flagged the control-plane modules as shallow and did not survey them); the SNC attestation grounds it.
---

# Cross-corpus pointer: OpenCode serve + attach (SNC attestation)

The canonical attestation is at `/home/agent/projects/SNC/.research/attestation/rao-ae-opencode-cli.md`, fetched 2026-06-03.

## Load-bearing claims

1. **OpenCode `serve` exposes a self-hosted spawn/attach surface** — `opencode serve` runs a headless HTTP API (default port 4096); clients attach and issue actions. Fully self-hosted; no vendor relay. `[snc-rao-ae-opencode-cli]{3}` — this contradicts the patchbay synthesis's claim that OpenCode provisioning is "out-of-band sysadmin"; OpenCode has operator-action spawn via `serve`.
2. **The attach primitive is distinct from operate.** OpenCode's surface separates `serve` (spawn the host), client-connect+auth (attach), and `prompt`/`cancel`/etc. (operate). This is the source of the spawn/attach/operate distinction the revised patchbay spine adopts.
