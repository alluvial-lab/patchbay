---
id: feature-adapter-staleness-liveness-core-delivery-subscription
kind: story
stage: implementing
tags: [adapter, protocol]
parent: feature-adapter-staleness-liveness
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Story: long-lived core delivery stream and adapter-loss reconciliation

Replace finite `ReceiveDeliveries` tails with a long-lived, incrementally
reconciled delivery subscription in `server/src/adapter_service.rs`. The stream
must remain pending when idle, apply durable events after its local scan cursor
into one `CommandIndex`, and yield only operations targeted to its current
adapter whose post-batch state is `accepted` or `delivered`.

On its epoch-fenced abnormal disconnect, preserve B3a by leaving
`delivered`-but-not-`running` commands redeliverable and terminalize only this
adapter's `running` commands as `failed` with
`FailureCode::ExecutionOutcomeUnknown`. Mark the adapter's sessions stale with
the existing canonical session writer. Rebuild the service command projection
after the reconciliation append.

Update `core/src/adapter/mod.rs` with the focused helper over `CommandIndex`,
`core/src/acceptance/index.rs` only as needed to enumerate records,
`server/Cargo.toml` for Tokio time support, and `docs/PROTOCOL.md` to record the
committed delivery-liveness shape and reserved heartbeat/age seam.

## Acceptance evidence

- A stream opened with no work stays pending and receives work accepted later
  without reopening or a per-poll full-log rebuild.
- Dropping the current idle or active stream durably marks its sessions stale;
  an older stream's drop after re-attach is inert.
- A dropped stream changes each matching `running` command exactly once to
  `failed` with `execution_outcome_unknown`; a delivered command remains
  redeliverable.
- Existing terminal-finality and B3a redelivery tests remain green.
