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
| `PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS` | core | yes | JSON object mapping each adapter id to its independently provisioned `AdapterControlService.Attach` credential, for example `{"pi":"...","token-commune":"..."}`. Credentials must be non-empty ASCII and distinct per adapter. |
| `PATCHBAY_ADAPTER_ATTACHMENT_SECRET` | pi-adapter, token-commune-adapter | yes | This adapter process's credential; it must match only its own id in the core's credential map. |
| `PATCHBAY_ADAPTER_LOG` | pi-adapter | no (default `$XDG_STATE_HOME/patchbay/adapter.log` when set and absolute, else `~/.local/state/patchbay/adapter.log`) | Durable adapter diagnostics log path. |
| `PATCHBAY_AUTHORITY_DOMAIN_ID` | all | no (default `default`) | The authority domain. All processes must agree; the core rejects mismatches. |
| `PATCHBAY_BIND_ADDR` | core | no | Network listener for `ControlService` + `AdapterControlService`. |
| `PATCHBAY_ADMIN_BIND_ADDR` | core | no | Loopback-only listener for `AdminService` (bootstrap). Never network-reachable; the core refuses non-loopback admin binds. |
| `PATCHBAY_DB_PATH` | core | no | SQLite durable event log path. |
| `PATCHBAY_SETUP_SECRET_TTL_SECS` | core | no (default 600) | One-time setup-secret lifetime. |
| `PATCHBAY_CORE_ADDR` | web-server, CLI, pi-adapter | yes | The core's network listener address. |
| `PATCHBAY_CORE_ADMIN_ADDR` | CLI | setup and lockdown recovery | The core's loopback admin listener (for `setup` and `lockdown-exit`). |
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
   setup secret** to stdout (expires after one use or the TTL). Grab it.
2. **Bootstrap (first run only)** — from the CLI on the same host:
   `patchbay-cli setup` (talks to the loopback `AdminService` only, prompts
   non-echoingly for the setup secret and password unless
   `PATCHBAY_SETUP_SECRET` / `PATCHBAY_OPERATOR_PASSWORD` are set, then creates
   the operator record + authority grant + first principal). A second bootstrap
   is rejected (first-run-only). Do not put either secret in CLI arguments.
3. **Pi adapter** — `pi-adapter`, pointed at the core (`PATCHBAY_CORE_ADDR`)
   with the per-adapter attachment credential mapped to `pi` in the core.
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
- `patchbay-cli revoke-all-sessions [--reason-code CODE]` — revoke every core operator session for the actor, clear local credentials, and require trusted-host login.
- `patchbay-cli revoke-principal <principal-id> [--reason-code CODE]` — revoke one control-surface principal.
- `patchbay-cli revoke-endpoint <endpoint-id> [--reason-code CODE]` / `patchbay-cli revoke-device <device-id> [--reason-code CODE]` — revoke a credential scope. Self-targeted credentials are cleared only after a confirmed core response.
- `patchbay-cli audit-query [flags]` — query the redacted audit projection.
- `patchbay-cli inspect-command <command-id> [flags]` — inspect lifecycle and
  related redacted audit history.
- `patchbay-cli adapter-status [adapter-id ...] [flags]` — inspect adapter
  state, capabilities, and recent diagnostics.
- `patchbay-cli lockdown-enter --reason-code CODE --confirm LOCKDOWN` — enter
  lockdown through authenticated ControlService; credentials are cleared only
  after a confirmed active response.
- `patchbay-cli lockdown-exit [--reason-code CODE]` — exit only through the
  loopback AdminService bootstrap channel; it intentionally reads no credential
  file and accepts no setup-secret/password flags.

### Lockdown recovery

Run `patchbay-cli lockdown-enter --reason-code suspected_endpoint_compromise
--confirm LOCKDOWN` from an authenticated trusted surface. It rejects new
Operations, clamps runtime sessions stale, and invalidates existing operator
sessions. Fresh login is read-only until recovery. After a restart or
self-lockout, run `patchbay-cli lockdown-exit` on the core host with
`PATCHBAY_CORE_ADMIN_ADDR` pointing at the configured loopback listener and no
credential file; then run `patchbay-cli login` for a higher-generation session.
If exit fails, posture remains locked. Never use the consumed setup secret as a
recovery shortcut.

The diagnostics and emergency-control CLI is VM-local in v0.1.0 because the core listener is loopback-only. Run `patchbay-cli audit-query`, `inspect-command`, `adapter-status`, or revocation commands on the VM; workstation operators may use an SSH tunnel or run the CLI on the VM. After confirmed all-session revocation, run `patchbay-cli login` from that trusted host. If the principal, endpoint, or device itself was revoked, use a distinct unrevoked identity or new endpoint/device configuration. The one-time `setup` secret is consumed bootstrap material, not recovery. Supported remote CLI transport is reserved for the future split-transport milestone.

Diagnostic flags are `audit-query --kind --actor-id --endpoint-id --command-id
--target --failure-code --reason-code --since --until --before-event --limit
1..500 --json`; `inspect-command --audit-before-event --audit-limit 1..200
--json`; and `adapter-status --after-adapter-id --limit 1..500 --json`.
`--since` is inclusive; time/cursor upper bounds are exclusive, and adapter
cursors are opaque and exclusive. Audit pagination defaults to 100 (maximum
500), command-related audit pagination defaults to 50 (maximum 200), and
adapter-status pagination defaults to 100 (maximum 500). JSON uses the common
`{ submission, resultEventId, asOfLsn, result }` envelope with decimal-string
LSNs/generations, RFC 3339 timestamps or `null`, generated lower-case enum
names, and redacted safe projections. Typed empty pages and `found: false` are
successful and exit `0`. Exit codes are `0` success, `1` local/transport/
protocol failure, `2` pre-acceptance rejection, `3` execution failure, and `4`
unknown submission outcome.

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
- Per-package suites: `./scripts/test-rust` (cleans and scopes Rust test
  temporary files under `target/test-tmp/`), `cd web-server && npm test`,
  `cd web-cockpit && npm test`, `cd cli && npm test`,
  `cd pi-adapter && npm test`, `cd contracts/ts && npm run check:vectors &&
  npm run check:presentation`.
