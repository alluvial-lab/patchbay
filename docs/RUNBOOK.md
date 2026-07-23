# Patchbay v0.1.0 Runbook

How to bring up the v0.1.0 walking skeleton: one operator controls Pi-backed
agent sessions through the web cockpit and the CLI. Four logical processes —
the Rust coordination core, the Pi adapter, the web server, and the control
surfaces (browser cockpit + CLI).

## Prerequisites

- Node ≥ 22, Rust stable toolchain.
- Build the core: `cargo build -p patchbay-core-server` (binary at
  `target/debug/patchbay-core-server`).
- Install/build the TS packages: `contracts/ts` (`npm ci && npm run build`),
  `web-server`, `web-cockpit` (`npm ci && npm run build`, incl.
  `build:browser`), `pi-adapter`, `cli`.

## Environment

| Variable | Process | Required | Purpose |
|---|---|---|---|
| `PATCHBAY_CORE_SECRET` | core, web-server, CLI | yes | Shared secret authenticating control surfaces as principals to the core (`x-patchbay-core-secret`). |
| `PATCHBAY_ADAPTER_ATTACHMENT_SECRET` | core, pi-adapter | yes | Adapter trust root for `AdapterControlService.Attach`. |
| `PATCHBAY_AUTHORITY_DOMAIN_ID` | all | no (default `default`) | The authority domain. All processes must agree; the core rejects mismatches. |
| `PATCHBAY_BIND_ADDR` | core | no | Network listener for `ControlService` + `AdapterControlService`. |
| `PATCHBAY_ADMIN_BIND_ADDR` | core | no | Loopback-only listener for `AdminService` (bootstrap). Never network-reachable; the core refuses non-loopback admin binds. |
| `PATCHBAY_DB_PATH` | core | no | SQLite durable event log path. |
| `PATCHBAY_SETUP_SECRET_TTL_SECS` | core | no (default 600) | One-time setup-secret lifetime. |
| `PATCHBAY_CORE_ADDR` | web-server, CLI, pi-adapter | yes | The core's network listener address. |
| `PATCHBAY_CORE_ADMIN_ADDR` | CLI | setup only | The core's loopback admin listener (for `setup`). |
| `PATCHBAY_WEB_BIND_ADDR` | web-server | no | HTTP listener for the cockpit. |
| `PATCHBAY_TLS_CERT` / `PATCHBAY_TLS_KEY` | web-server | non-localhost | TLS for non-loopback binds (loopback uses the secure-cookie exception). |
| `PATCHBAY_CREDENTIALS_PATH` | CLI | no | CLI credential store (0600). |
| `PATCHBAY_OPERATOR_ID` / `PATCHBAY_OPERATOR_PASSWORD_HASH` | web-server | no | First-run fallback only; the core's operator record is the source of truth after bootstrap. |

## Startup order

1. **Core** — `patchbay-core-server`. At first run it prints a **one-time
   setup secret** to stderr (expires after one use or the TTL). Grab it.
2. **Bootstrap (first run only)** — from the CLI on the same host:
   `patchbay-cli setup` (talks to the loopback `AdminService` only, presents
   the setup secret, creates the operator record + authority grant + first
   principal). A second bootstrap is rejected (first-run-only).
3. **Pi adapter** — `pi-adapter`, pointed at the core (`PATCHBAY_CORE_ADDR`)
   with the attachment secret.
4. **Web server** — `patchbay-web-server`, pointed at the core. It serves the
   cockpit assets and templates the configured authority domain into the page.
5. **Surfaces** — open the cockpit in a browser (it now has a login form;
   authenticate with the operator credentials created at setup), or
   `patchbay-cli login` (throttled core password verification; writes the 0600
   credential store).

## Everyday commands

- `patchbay-cli session-health` — connectivity × activity axes for sessions.
- `patchbay-cli instruct <target> <prompt>` — send a prompt (stable target
  identity shown before submission).
- `patchbay-cli cancel|interrupt <command-id>`.
- `patchbay-cli logout`.
- `audit-query` / `inspect-command` / `adapter-status` are reserved
  post-v0.1.0 (stubbed; they need the core-diagnostics projection).

## Known v0.1.0 limitations (honest, not defects)

- **Adapter-liveness staleness coverage is narrow.** The core marks an
  adapter's sessions `stale` when the adapter's delivery stream drops
  abnormally (crash / transport error). But the v0.1.0 delivery model is a
  polling fallback: the stream drains the durable tail and completes in
  milliseconds per ~100ms poll, and command execution happens *after* stream
  completion. So the staleness signal only fires for deaths during an active
  stream drain. **An adapter that dies between polls or mid-execution leaves
  its sessions presented as `live/working` until the adapter restarts** and
  re-reports. Commands are not lost — accepted/delivered commands re-deliver
  on adapter restart (epic pass-2 B3a), and a replacement adapter process
  cannot compound the confusion (epic pass-2 B2 fencing). A heartbeat /
  last-report-age staleness mechanism (or a long-poll delivery redesign) is
  parked as a fast-follower (`backlog-adapter-staleness-full-coverage`).
- **Commands rot at `running` if execution never completes.** The redelivery
  bound is `delivered`-but-not-`running` (epic pass-2 B3a, operator decision
  Q1a). A command that reaches `running` but never completes (adapter death
  mid-execution) is not re-offered. Same recovery as above: adapter restart.

## Verification

- Composed end-to-end smoke (boots core + adapter and drives
  CLI login → instruct → live/working → completed/idle):
  `cd e2e && npm test`.
- Per-package suites: `cargo test --workspace`, `cd web-server && npm test`,
  `cd web-cockpit && npm test`, `cd cli && npm test`,
  `cd pi-adapter && npm test`, `cd contracts/ts && npm run check:vectors &&
  npm run check:presentation`.
