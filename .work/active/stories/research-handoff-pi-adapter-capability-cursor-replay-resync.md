---
id: research-handoff-pi-adapter-capability-cursor-replay-resync
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-rpc-process-supervisor, research-handoff-spawn-reconnect-cursor-reconcile]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Persisted Pi entry cursor replay and full resync

## Checkpoint

Implement Pi-native recovery as persisted-entry reconciliation, never remembered-stream authority. A generation-scoped cursor is committed only after the corresponding entry has been projected and acknowledged by the core. `get_entries(since)` supplies the authoritative append-order suffix/current `leafId`; a failed unknown cursor enters an explicit full replay from the session start and keeps the session stale until convergence.

Live RPC events remain notifications. Parallel tool updates may interleave and completion order may differ from assistant source order; the adapter must not relabel arrival order as a universal source order. Final message/session entries repair transient deltas.

## Design

**Files**
- New `pi-adapter/src/cursor_store.ts` — 0600, atomic-replace store keyed by logical target + generation + verified Pi session identity.
- New `pi-adapter/src/entry_reconciler.ts` — suffix/full-resync state machine, deterministic projection, current-leaf handling, and commit-after-core-ack.
- `pi-adapter/src/rpc_client.ts` and `pi_session.ts` — typed unknown-cursor failure; live entry notifications wake reconciliation rather than serving as the durable cursor.
- `pi-adapter/src/spawn_supervisor.ts` and `main.ts` — require successful reconciliation before `live` and before continuation success evidence.
- `pi-adapter/src/transcript_projection.ts`, core transcript ingress, and vectors — deterministic duplicate-inert entry ids across crash windows.

The Pi entry cursor and core `(authority_domain_id, LSN)` remain separate. Neither is translated into the other and neither proves process liveness.

## Acceptance evidence

- [ ] Known cursor returns and applies only the strict append-order suffix and current leaf.
- [ ] Cursor persistence occurs after core acknowledgement for each entry; crash between acknowledgement and cursor commit replays harmlessly.
- [ ] Unknown cursor cannot become an empty suffix: the adapter records bounded full-resync evidence, fetches all entries, converges idempotently, then atomically installs the new cursor/leaf.
- [ ] Pre-compaction and abandoned-branch entries remain recoverable; `leafId` updates the active branch view without pretending to order live tool notifications.
- [ ] RPC reconnect, adapter restart, and continuation replacement rebind subscriptions and stay stale/unknown until current-process handshake plus entry reconciliation complete.
- [ ] Mutations that silently return all entries for unknown `since`, commit before core acknowledgement, or infer live from a remembered event stream fail.

## Ordering constraint

Consumes the spawn reconnect/cursor contract and the managed RPC supervisor.
