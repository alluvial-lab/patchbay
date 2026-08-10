---
source_handle: restart-hot-reload-code-ec2908c
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi@ec2908c84809237a49cf1a1f7d8dc409947fd8a7:pi-extension/src/index.ts;pi-extension/src/extension.test.ts;scripts/pi-restart-loop.sh
provenance: source-direct
---

# Attestation: reviewed hot-reload implementation at ec2908c

## Structural metadata

- Source type: implementation and regression tests at a fixed commit
- Commit: `ec2908c84809237a49cf1a1f7d8dc409947fd8a7`
- Paths: `pi-extension/src/index.ts`, `pi-extension/src/extension.test.ts`, `scripts/pi-restart-loop.sh`
- Scope used here: concrete fencing mechanisms and their test coverage

## Paraphrased summary

At this commit, the extension owns process-scoped hot-reload state and the wrapper consumes an exact child-PID marker. The handler rejects insecure state files, validates a nonce and expiry, takes an exclusive claim, checks disposal at several points, writes the restart marker, and signals SIGTERM. Tests cover duplicate settled notifications, non-idle deferral, quiescing rejection, daemon/nonce/toggle gates, disposed-session fencing, symlink rejection, directory permissions, cleanup, and stale identity sweep.

## Key passages

### {1} State-file admission and runtime identity

Anchor: `pi-extension/src/index.ts:2586-2707`.

The implementation requires a non-symlink directory with mode `0700` and owner uid, and regular state files with mode `0600` and owner uid. It writes `.runtime-self-<PID>` with `{pid, nonce, ts}` using `flag: "wx"`.

### {2} Process-scoped arming

Anchor: `pi-extension/src/index.ts:2713-2735`.

Arming is disabled for daemon mode, requires the secure toggle, writes `.hot-reload-armed-<PID>` with the module nonce and timestamp, and uses exclusive creation.

### {3} Synchronous settlement handler and local-delivery limitation

Anchor: `pi-extension/src/index.ts:2789-2874`.

The code comment explicitly says `agent_settled` is not an end-to-end WebSocket acknowledgment. The handler validates toggle, armed file, nonce, and expiry; sets `_hotReloading`; rechecks `_disposed` and `ctx.isIdle()`; takes `.claimed-<PID>` with `flag: "wx"`; writes `.restart-marker-<PID>` before `process.kill(process.pid, "SIGTERM")`.

### {4} Ingress during quiescence

Anchor: `pi-extension/src/index.ts` → `_deliverUserMessage` hot-reload branch.

The implementation sends a recoverable delivery error and states that `session_sync` recovers output while input must be resent after reconnect. It does not enqueue the input into the exiting process.

### {5} Exact wrapper-side marker correlation

Anchor: `scripts/pi-restart-loop.sh:21-38` at this commit.

The wrapper backgrounds Pi to capture `child_pid`, waits for that PID, constructs `.restart-marker-$child_pid`, and restarts only when that exact marker accompanies exit zero.

### {6} Duplicate settlement, order, and marker test

Anchor: `pi-extension/src/extension.test.ts:6725-6757` at this commit.

The test invokes `agent_settled` twice, expects one SIGTERM, and asserts that the PID-scoped marker exists before the mocked kill. It also asserts the exclusive claim remains and the armed request was consumed.

### {7} Lifecycle and filesystem tests

Anchor: `pi-extension/src/extension.test.ts:6760-7004` at this commit.

Tests cover non-idle deferral, quiescing errors rather than pending replay, daemon and nonce gates, disposed-session interruption, symlink rejection, insecure-directory rejection, global off cleanup, and dead-PID identity sweep.
