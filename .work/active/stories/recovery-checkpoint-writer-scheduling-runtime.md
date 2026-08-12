---
id: recovery-checkpoint-writer-scheduling-runtime
kind: story
stage: done
tags: [perf, protocol, storage]
parent: recovery-checkpoint-writer
depends_on: [recovery-checkpoint-writer-session-recovery-state]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-10
updated: 2026-08-11
---

# Schedule and persist session checkpoints

## Checkpoint

Add the production event-gap writer, atomic latest-only storage behavior, retry loop, and failure observer without making checkpoint success part of Operation or adapter availability.

## Design element

- Add `server/src/checkpoint.rs` with a typed `SessionCheckpointPolicy`, deterministic `run_once`, long-running `run`, tick outcomes, failure stages, and an injectable `CheckpointObserver`.
- The private v0.1.0 default targets 256 newly observed authority-domain events, checks once per second, and retries failures with capped 1→30 second backoff. These are operational defaults, not protocol/SLA values.
- Under the shared decision gate: catch up state, validate the latest session checkpoint, decide due/skip, and materialize at the applied head. Release the gate before encoding and storage I/O; later events do not invalidate prefix N.
- Make SQLite snapshot writes atomically retain only the newest non-regressing session row per authority domain. Failure rolls back to the prior row and consumes no event/audit LSN.
- Start and monitor the worker beside both tonic servers. Structured stderr failure observations contain stage/anchor/class but no payload; failures never append audit events, reject accepted work, or stop serving.

## Acceptance evidence

- [x] Threshold comparison writes exactly when due and skips below the configured gap under a fixed clock.
- [x] Later appends during storage I/O do not alter the checkpoint's consistent prefix.
- [x] An injected first-write failure preserves the old checkpoint/log, is observed, and succeeds on retry.
- [x] Successful writes retain one row per domain, reject stale replacement, and create no LSN.
- [x] Crash/cancellation leaves either the prior or complete new checkpoint, never a partial authority source.

## Implementation evidence

- `server/src/checkpoint.rs` provides deterministic event-gap ticks, idle fast paths, forced repair, structured redacted failure classes, and capped retry; `main.rs` monitors the long-running task.
- Encoding and SQLite I/O occur after releasing the shared decision gate. SQLite replaces the latest row atomically and rejects regression.
- Scheduler tests cover due/skip, below-threshold incompatible repair, exact prior-row preservation on injected failure, observable retry, and successful advancement; storage tests cover latest-only/non-regressing rows.

## Ordering constraints

Depends on `recovery-checkpoint-writer-session-recovery-state`, whose complete format and materializer the writer consumes. Blocks `recovery-checkpoint-writer-bounded-recovery-evidence`.
