---
source_handle: amux-outbox
fetched: 2026-08-08
source_url: https://github.com/mixpeek/amux/tree/7de5a76b595733a1eea768e355ef07a8ba46ac28
provenance: source-direct
---

## Summary

amux is a local-first control plane over long-running terminal agent workers. It has several durability mechanisms at different scopes: a browser mutation outbox, persistent send deduplication, a SQLite steering queue restored on server start, append-only observable session events, and SQLite task claiming. These mechanisms deliberately expire, collapse, or drop stale intent rather than retaining every accepted request indefinitely. The project is explicit that its HTTP surface has no built-in authentication and should be kept behind localhost or Tailscale.

## Structural metadata

- Repository: `mixpeek/amux`
- Commit: `7de5a76b595733a1eea768e355ef07a8ba46ac28`
- Commit subject: `fix(digest): the owner's SMS says what to DO, and stops cutting mid-word`
- License presented by repository: MIT + Commons Clause
- Fetched as a local Git clone and inspected at the pinned commit.

## Key passages

1. The browser replaces `fetch` for queueable mutation methods. When offline or when a network call throws, it appends the URL, method, headers, body, and timestamp to `offlineQueue`, persists the queue to localStorage and IndexedDB, and returns synthetic HTTP 202 `{ ok: true, queued: true, offline: true }`. Interactive/ephemeral endpoints, uploads, browser calls, and non-string bodies are excluded. (`amux-server.py`, “GLOBAL OFFLINE OUTBOX — fetch interceptor”; `_queueOp`; `saveQueue`)

2. Reconnect replay runs through one page-side replayer. Transient server/network failures are returned to the queue; permanent non-success 4xx results are surfaced but not retried forever. The e2e test verifies queue survival across an offline reload, server-side application after reconnect, retention of transient failures, and preservation of the original message id. (`amux-server.py`, `runSyncBanner`; `tests/e2e-offline-sync.mjs`)

3. Before replay, `reconcileQueue` drops operations older than seven days, removes contradictory start/stop or create/delete operations, and applies last-write-wins collapse to repeated writes for the same note, file, preferences, or memory resource. The queue is capped at 200 entries and drops the oldest entry when full. (`amux-server.py`, `reconcileQueue` and `_queueOp`)

4. Sends and steers carry `msg_id`. The SQLite `send_dedup` primary key is `(session, msg_id)`; the server records the id before delivery so a retry after a response-losing server restart is ignored. Entries older than 600 seconds are deleted, storage failure makes dedup best-effort rather than fail-closed, and failed sends explicitly release the id for retry. (`amux-server.py`, `send_dedup` schema, `_send_dedup_seen`, `_send_dedup_forget`, and `/send`/`/steer` handlers)

5. The peek/send contract labels logical sends “exactly-once” and requires a persistent dedup table because server restart is itself part of the response-loss window. It also waits through boot races and verifies submission against captured pane state instead of equating successful `tmux send-keys` with delivery. (`docs/peek-parity.md`, P5)

6. `_steer_enqueue` inserts messages into an in-memory queue and SQLite `steering_queue`, deduplicates or replaces an already-waiting equivalent/guarded nudge, and emits a queued event. `_load_steering_from_db` reconstructs the queue in timestamp order at startup. Delivery happens only after a continuously idle turn boundary, uses an in-flight marker to prevent two loops from double-delivering, and removes the row after a successful send. (`amux-server.py`, `_steer_enqueue`, `_load_steering_from_db`, `_steer_try_deliver`)

7. Steering intent is bounded: a sweep drops rows whose session no longer exists or which have remained undelivered beyond `AMUX_STEER_MAX_AGE_SECS` (default 14 days), while guarded nudges are revalidated at delivery and dropped if their asserted condition is stale. Drops are audited as `message.dropped`. (`amux-server.py`, `_STEER_MAX_AGE_SECS`, `_steer_sweep_undeliverable`, and delivery-time guard handling)

8. `session_events` is an append-only log of observable transitions and action receipts, not model reasoning. An optional unique `idem` key uses `INSERT OR IGNORE` so retries or restart replay cannot double-record externally visible events; an hourly retention sweep removes old rows. (`amux-server.py`, `session_events` schema and `_emit_event`)

9. The worker model has a stable UUID that survives stop/start, SQLite-backed board state, and atomic task claiming. Those identities and task claims are not paired with an operation-level runtime generation/incarnation in the fetched source. Searches for generation fencing found generation guards for caches and the tunnel subsystem, not for agent-worker command delivery. (`README.md`, “Agent infrastructure”; `amux-server.py`, board claiming and tunnel/cache generation guards)

10. The README states that amux has no built-in authentication, is local-first, and should use Tailscale or localhost rather than exposure to the internet. Server-verified origin fields and per-session routing exist for some actions, but they do not constitute a general authenticated resource-authority system. (`README.md`, FAQ/security text; `amux-server.py`, owner-alert provenance ledger)
