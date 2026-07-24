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
| `PATCHBAY_TLS_CERT` / `PATCHBAY_TLS_KEY` | web-server | non-localhost | Direct TLS for browser sessions on non-loopback binds. |
| `PATCHBAY_TRUST_LOOPBACK_PROXY` | web-server | no (default `false`) | Allow an explicitly configured loopback reverse proxy only when it attests `X-Forwarded-Proto: https`. |
| `PATCHBAY_CREDENTIALS_PATH` | CLI | no | CLI credential store (0600). |
| `PATCHBAY_SETUP_SECRET` | CLI | setup only | One-time setup secret. Supply through the environment or the CLI's non-echoing TTY prompt; never as an argument. |
| `PATCHBAY_OPERATOR_PASSWORD` | CLI | setup/login | Operator password. Supply through the environment or the CLI's non-echoing TTY prompt; never as an argument. |
| `PATCHBAY_OPERATOR_ID` | web-server | yes | Configured operator identity for core password verification; the web server refuses startup without it. |
| `PATCHBAY_OPERATOR_PASSWORD_HASH` | web-server | no | Optional local password-verifier fallback. Normal v0.1.0 login verifies the bootstrapped operator record at the core. |

## Startup order

1. **Core** — `patchbay-core-server`. At first run it prints a **one-time
   setup secret** to stderr (expires after one use or the TTL). Grab it.
2. **Bootstrap (first run only)** — from the CLI on the same host:
   `patchbay-cli setup` (talks to the loopback `AdminService` only, prompts
   non-echoingly for the setup secret and password unless
   `PATCHBAY_SETUP_SECRET` / `PATCHBAY_OPERATOR_PASSWORD` are set, then creates
   the operator record + authority grant + first principal). A second bootstrap
   is rejected (first-run-only). Do not put either secret in CLI arguments.
3. **Pi adapter** — `pi-adapter`, pointed at the core (`PATCHBAY_CORE_ADDR`)
   with the attachment secret.
4. **Web server** — `patchbay-web-server`, pointed at the core. It serves the
   cockpit assets and templates the configured authority domain into the page.
5. **Surfaces** — open the cockpit in a browser (it now has a login form;
   authenticate with the operator credentials created at setup), or
   `patchbay-cli login` (prompts non-echoingly for the password unless
   `PATCHBAY_OPERATOR_PASSWORD` is set; throttled core password verification;
   writes the 0600 credential store).

## Browser transport

- For a non-loopback browser listener, configure `PATCHBAY_TLS_CERT` and
  `PATCHBAY_TLS_KEY`; plaintext browser sessions fail closed.
- The local HTTP exception is only for a direct loopback browser request with
  no `X-Forwarded-Proto` header.
- A same-host reverse proxy must listen to the web server over loopback, set
  `PATCHBAY_TRUST_LOOPBACK_PROXY=true`, and **overwrite**
  `X-Forwarded-Proto` with its browser-facing scheme. The server accepts a
  proxied browser session only when that single header value is exactly `https`;
  `http`, missing/ambiguous forwarded values, and untrusted proxy headers fail
  closed. Do not use this setting for a non-loopback proxy.

## Everyday commands

- `patchbay-cli session-health` — connectivity × activity axes for sessions.
- `patchbay-cli instruct <target> <prompt>` — send a prompt (stable target
  identity shown before submission).
- `patchbay-cli cancel|interrupt <command-id>`.
- `patchbay-cli logout`.
- `audit-query` / `inspect-command` / `adapter-status` are reserved
  post-v0.1.0 (stubbed; they need the core-diagnostics projection).

## Known v0.1.0 limitations (honest, not defects)

- **Transport liveness is not an application heartbeat.** Each current adapter
  attachment holds one long-lived authenticated delivery stream, and an
  abnormal transport close marks its sessions `stale`. A network black hole
  that leaves TCP/HTTP2 apparently open can still delay that signal until the
  transport detects failure — and in one corner it never arrives: if a
  replacement attachment bumps the stream epoch *before* the black-holed old
  transport dies, the old stream's eventual drop is fenced inert, so its
  `running` commands are never terminalized and (per the delivered-not-running
  redelivery rule) never re-offered. That is permanent running-rot for that
  attachment, by design: the old process may still be alive behind the
  partition, and heartbeat/last-report-age policy remains the reserved
  escalation if operations show transport liveness is insufficient.
- **Core-side stream faults conflate with adapter loss.** If the delivery
  subscription itself fails core-side (a storage read error or a corrupt log
  event), the same abnormal-disconnect path fires: the (healthy) adapter's
  sessions go `stale` and its `running` commands are terminalized as
  `failed(execution_outcome_unknown)`, with later completions demoted to
  audit-only. This is deliberate fail-fast behavior on a corruption path, not
  a liveness signal about the adapter.
- **`execution_outcome_unknown` requires retry judgment.** If the adapter
  stream is lost after a command reaches `running`, the core records
  `failed(execution_outcome_unknown)`: the action may already have executed.
  Retry safety is determined by the adapter's `idempotency_strength`, not by
  the `failed` state alone. Commands still at `accepted` or `delivered` remain
  eligible for bounded redelivery because execution is not known to have
  started.

## Verification

- Composed separate-process end-to-end smoke (run after changes across the
  core, CLI, or Pi adapter; boots its own core + adapter and drives CLI login →
  instruct → durable command completion → live/idle): `cd e2e && npm test`.
  This complements the in-process Pi adapter integration suite rather than
  replacing it.
- Per-package suites: `cargo test --workspace`, `cd web-server && npm test`,
  `cd web-cockpit && npm test`, `cd cli && npm test`,
  `cd pi-adapter && npm test`, `cd contracts/ts && npm run check:vectors &&
  npm run check:presentation`.
