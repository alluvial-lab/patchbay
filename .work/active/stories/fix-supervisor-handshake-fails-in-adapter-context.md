---
id: fix-supervisor-handshake-fails-in-adapter-context
kind: story
stage: drafting
tags: [verification, adapter]
parent: null
depends_on: [fix-pi-managed-spawn-delivery-wiring, fix-cockpit-spawn-target-shape-mismatch]
release_binding: null
gate_origin: null
created: 2026-08-17
updated: 2026-08-17
---

# Fix: supervisor handshake fails inside the live adapter while the identical manual path succeeds

## Reproduction (live UAT, clean world, 2026-08-17 00:37)

Clean world, aligned config (executable=node, cliPath=adapter's pinned
node_modules pi 0.84.1, controlExtensionPath=dist build, cwd=workspace,
environment PI_OFFLINE=1). `spawn pi`:

- delivery accepted → running → supervisor validates, journals, LAUNCHES a real
  child (phase 5), binds identity (phase 6, runtime session id captured), then
  `HANDSHAKE_RECONCILING` (phase 7) fails within ~100ms → correct
  `execution_outcome_unknown` + `poisoned_pending_reconciliation`.

## What has been ruled out

1. The handshake machinery works on this box: `pi-adapter npm run test:e2e`
   4/4 including the real `pi --mode rpc` handshake.
2. A manual probe using the SAME modules (`RpcManagedPiRuntimePort` +
   `performPiControlHandshake` via `port.handshake`), same cwd, same extension
   build, same `PI_OFFLINE=1` env, same launch shape (node + pinned cli.js)
   succeeds: `HANDSHAKE OK cwd: /home/agent/uat/workspace sessionId: 01a00d24…`.
3. Version drift (system pi 0.84.2 vs pinned 0.84.1) — config now uses the
   pinned cli.
4. Shape/payload mismatches — fixed in the two dependency stories; the durable
   accepted envelope decodes and validates through the supervisor's
   `#validate` (verified by direct probe).
5. Env sanitization — SAFE_INHERITED_ENV + explicit PI_OFFLINE reaches the
   child in both paths (same `sanitizedEnvironment` code).

## Remaining suspects (for the fix worker)

- **Timing**: the supervisor handshakes ~115ms after identity bind; the manual
  path has more elapsed readiness margin. Suspect `get_commands` returns
  before the extension registers (`COMMAND_MISSING`) — check whether the
  port/e2e path has an implicit readiness wait the supervisor path lacks, or
  whether the first `getCommands` needs a bounded retry.
- **Diagnostics blind spot (fix alongside)**: `SpawnSupervisorError` messages
  are stripped by `diagnosticError` redaction everywhere including the
  adapter-LOCAL ndjson log, making live diagnosis need this whole dance. The
  local adapter log should retain a redacted-but-actionable failure code
  (e.g. `handshake_failure: COMMAND_MISSING`) — codes, not message text.
- Adapter-process context: the long-running adapter's own event-loop/env vs
  the one-shot probe (less likely, but unverified).

## Acceptance

- [ ] Live `spawn pi` on a clean world completes: staged → promoted → live
      child; `inspect-command` shows promoted + completed via promotion.
- [ ] The failing handshake step is identified by name in the local adapter
      log (failure code, no raw text).
- [ ] Regression covering the supervisor-context handshake timing/whatever
      the root cause turns out to be; full four groups + pi suites.
