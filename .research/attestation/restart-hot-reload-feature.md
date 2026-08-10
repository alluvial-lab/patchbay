---
source_handle: restart-hot-reload-feature
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi@6aeb3329f75273e85dd9829e0992f8d945e1a1de:.work/active/features/feature-extension-hot-reload-via-process-restart.md
provenance: source-direct
---

# Attestation: extension hot-reload via process restart feature record

## Structural metadata

- Source type: completed operational feature record with preserved design and review history
- Commit: `6aeb3329f75273e85dd9829e0992f8d945e1a1de`
- Internal structure: brief; adversarial findings; revised design; implementation summary; review record
- Scope used here: failure modes, rejected seam choices, accepted lifecycle protocol, and acknowledged residual limitations

## Paraphrased summary

The feature record begins with a turn-end, timer, machine-global sentinel design and preserves the review findings that invalidated it. It then records a process-scoped arming and restart protocol based on a nonce, exclusive file creation, synchronous quiescing, an idle recheck, graceful SIGTERM, and a wrapper marker. Its review record distinguishes recoverable rejection from replay and describes follow-up findings that were not closed.

## Key passages

### {1} Mid-turn timer race

Anchor: `## Adversarial review findings` → `### Blockers` → `B1`.

A queued message was drained before the old turn-end restart helper armed a 500 ms timer. The record states that the queued message could start a new turn and then be killed by the timer; a newly arriving prompt had the same problem.

### {2} Machine-global request and non-exclusive check

Anchor: `B2`.

The original sentinel was shared by every interactive Pi and supervised daemon. The record says another process could consume the request and that two processes could both pass an existence check before unlinking.

### {3} Normal quit and restart intent were ambiguous

Anchor: `B3`.

The initial wrapper restarted on exit zero, but both SIGTERM and normal `/quit` produced exit zero. The feature records that this made an in-TUI stop impossible and that arming without a wrapper could kill the only process without relaunch.

### {4} Delay is not delivery acknowledgment

Anchor: `B4`.

The record states that WebSocket send queues bytes and close does not prove flush; a 500 ms sleep did not prove final output or `working=false` reached the app.

### {5} Unfenced old timer can kill a successor

Anchor: `### Major` → `M1`.

> "A `/reload`, `/new`, or resume during the 500ms delay sets `_disposed=true`, then the successor's `session_start` resets it to false. The old timer now sees false and kills the NEW session."

It also notes that multiple arms could leave multiple untracked timers.

### {6} Stale request and restart-window input loss

Anchor: `M2` and `M3`.

The record says an armed request could survive disable/re-enable or crash and fire in a successor. It separately says an app message could be accepted near disconnect, dropped after the destination disappeared, and never resent.

### {7} Revised identity and single-winner claim

Anchor: `## Revised design` → `### Architectural choice`.

The accepted request is PID-scoped and bound to a module nonce. A `.claimed-<PID>` file is created with `O_CREAT|O_EXCL`; the armed request is removed after the claim.

### {8} Settlement plus quiescing and recheck

Anchor: same section.

The feature says `agent_settled` is not enough by itself. The handler stays synchronous, sets `_hotReloading`, then checks `ctx.isIdle()`; ingress observes the gate before shutdown is requested.

### {9} Graceful shutdown and typed wrapper intent

Anchor: same section.

The accepted hot-reload path writes a durable marker before sending SIGTERM. Exit zero plus marker means restart-with-continue; exit zero without marker means stop. Daemon mode is excluded from this path.

### {10} Review correction to delivery behavior

Anchor: `## Review record`.

The first implementation used `delivery_pending`; review changed it to recoverable `delivery_error` because the exiting process could not honor replay. The same review changed the marker from global to PID-scoped and required the wrapper to validate the child PID.

### {11} Residual findings after feature closure

Anchor: `## Review record` → `Important findings parked`.

The record lists shell ancestor discovery, wrapper filesystem validation, subprocess tests, and command error reporting as parked rather than closed. It says closure occurred after receiver-confirmed fixes without another independent pass.
