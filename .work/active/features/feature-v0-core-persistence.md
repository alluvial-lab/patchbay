---
id: feature-v0-core-persistence
kind: feature
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-v0-core
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: Core persistence, event log, and recovery

## Brief

Build the durable event log, storage port, snapshot checkpointing, and crash recovery — the foundation every other core feature writes through. The core owns a single totally-ordered durable event log per authority domain; every accepted state-transition event is assigned a monotonic, gap-free log sequence number (LSN) at durable-commit time. The LSN is the canonical ordering for first-terminal-commit-wins and for snapshot reconciliation.

The storage port is the Ports & Adapters boundary: domain semantics read and write through it without depending on the backend choice. The first backend may be embedded (file or embedded database). Snapshots are derived checkpoints used to bound recovery replay cost; they are never an alternate ordering authority. On restart, the core replays the log (or loads the latest snapshot then replays the tail) to reconstruct in-memory state up to the last committed LSN. No accepted command disappears silently after a crash.

This is the root of the core epic — acceptance, authority, and sessions all depend on the event log and storage port. It is the riskiest feature because backend choice affects crash recovery correctness and the qualitative responsiveness floor.

## Epic context

- Parent epic: `epic-v0-core`
- Position in epic: root. Acceptance, authority, and sessions depend on this feature's storage port and event log. Those three can proceed in parallel once the port interface and event log are designed.

## Formal-model backing

- `BoundaryDedup` (promoted, `command_lifecycle.qnt`) — retrying the same idempotency key cannot double-apply a command at the boundary. The `appliedKeys` set and `lsn` variable live in the event-log/persistence layer.
- Crash/replay/snapshot convergence — stated-normative (v1 formal gate owns the real property). The removed `snapshot_recovery.qnt` draft formulas did not model the claimed failure boundary; the obligation survives as stated-normative.

## Foundation references

- `docs/PROTOCOL.md` — Snapshots and streams; Revisions and cursors; Atomicity between events and snapshots; Persistence and recovery
- `docs/ARCHITECTURE.md` — v0.1.0 persistence topology (single-writer, local-first, port-isolated, log + snapshots, crash recovery)
- `docs/VERIFICATION.md` — property-graded assurance; `BoundaryDedup` promoted property
- `contracts/proto/patchbay/common.proto` — `Lsn`, `ViewRevision`
- `contracts/rust/` — generated Rust bindings (starting contract for types)
- `specs/seed/command_lifecycle.qnt` — `appliedKeys`, `lsn`, `terminalLsn` state
- `specs/seed/snapshot_recovery.qnt` — stated-normative obligations for crash/replay convergence
