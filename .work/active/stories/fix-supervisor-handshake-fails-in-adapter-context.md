---
id: fix-supervisor-handshake-fails-in-adapter-context
kind: story
stage: review
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
- [x] The failing handshake step is identified by name in the local adapter
      log (failure code, no raw text).
- [x] Regression covering the supervisor-context handshake timing/whatever
      the root cause turns out to be; full four groups + pi suites.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the
  real-process supervisor, external-effect ambiguity, and redacted diagnostics
  boundary). Direct-read implementation was used because the reproduction named
  the integration points and this delegated worker cannot fan out under the
  harness recursion guard.
- Review weight: `standard` (project default), with this `[verification]` story
  left at `stage: review` for the orchestrator's configured review/UAT lane.
- Root cause and ruled-out timing suspect: the live challenged handshake did
  **not** fail. Journal phase `HANDSHAKE_RECONCILING` (7) is appended only after
  `runtimePort.handshake`, `getEntries`, and materialization classification have
  all succeeded. The live target set `sessionRoot=/home/agent/uat/sessions` but
  omitted optional `sessionDirectory`, so Pi selected its default
  `~/.pi/agent/sessions/...` path. The new session was still memory-only; the
  next `stageClaimedSuccessor` call then rejected that declared path as outside
  the configured continuity root. The identical manual probe stopped after the
  handshake, before cursor staging, while the passing real-process E2E explicitly
  supplied `sessionDirectory=sessionRoot`. This explains both apparently
  contradictory observations without a readiness race or retry.
- Fix: managed launch now canonicalizes `sessionRoot`, uses it as the default
  `--session-dir`, and rejects an explicit session directory outside that root
  before recording a launch attempt. `sessionRoot` is therefore the safe launch
  default as well as the continuity/integrity boundary; callers no longer need
  to duplicate it merely to keep fresh sessions in scope.
- Diagnostic-code mechanism: `PiControlHandshakeError` carries a closed
  `handshake:<PiControlHandshakeFailure>` diagnostic code, and
  `SpawnSupervisorError` carries `spawn:<FailureCode>` while preserving a nested
  handshake code through ambiguity normalization. `diagnosticError` prefers
  that classification and admits only a bounded 128-byte code token; it never
  copies message, cause, stack, path, or arbitrary metadata. The core forwarder
  remains unchanged and structurally ignores adapter-local `error`, so only the
  existing canonical failure/report vocabulary crosses the core boundary.
- Files changed: `pi-adapter/src/{control_handshake,adapter_diagnostics,spawn_supervisor}.ts`;
  `pi-adapter/tests/{adapter_diagnostics,spawn_supervisor,e2e}.test.ts`;
  `pi-adapter/scripts/mutation-cycle.mjs`; this story. No Protobuf/generated
  contract or foundation-document change was required.
- Tests added/extended: a supervisor regression proves omitted
  `sessionDirectory` produces `--session-dir <sessionRoot>`; the real-core/real-Pi
  lifecycle E2E now omits the duplicate field and still proves fresh promotion,
  continuation, reload, reconnect, and child cleanup; a failure-injection test
  proves `handshake:COMMAND_MISSING` survives supervisor wrapping; the local
  NDJSON test proves the code is retained while message text is absent.
- Mutation evidence: `npm run test:mutations` killed **33/33**, including new
  witnesses that restore the cwd/default-root mismatch and strip the challenged
  handshake code during supervisor wrapping.
- Real-process evidence: `npm run test:e2e` passed **4/4**. The exact
  supervisor-path lifecycle passed without an explicit `sessionDirectory`, and
  all Pi children started by the tests were reaped. Full Pi suite passed
  **131/131**.
- Full verification (2026-08-17):
  1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
  2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS** (60 vectors, 19 promoted, 33 implementation checks, 38 vector mutation witnesses).
  3. `cd operator-domain && npm run build && npm test` — **PASS** (34/34).
  4. `cd pi-adapter && npm test && npm run test:mutations` — **PASS** (131/131; 33/33 mutations killed).
  Consumer suites: `cd web-cockpit && npm test` — **PASS** (153/153);
  `cd cli && npm test` — **PASS** (54/54 plus real-core resource projection);
  `cd token-commune-adapter && npm test` — **PASS** (63/63, including both
  real-core flows). Separate `cd pi-adapter && npm run test:e2e` — **PASS**
  (4/4).
- Simplification: one configured root now governs both child placement and
  continuity validation by default; no readiness timer, retry loop, alternate
  handshake, or duplicate diagnostic transport was added.
- Discrepancies from the initial diagnosis: the prime `get_commands` readiness
  suspect was disproven by the durable phase order and an exact bind → handshake
  → stage probe. The fault was the post-handshake session-root mismatch.
- Adjacent issues parked: none. The live stack was deliberately not restarted or
  modified; clean-world cockpit spawn remains the orchestrator/operator retest.
