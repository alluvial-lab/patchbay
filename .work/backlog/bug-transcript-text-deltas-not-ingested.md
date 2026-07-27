---
id: bug-transcript-text-deltas-not-ingested
tags: [bug, adapter, transcript]
created: 2026-07-27
---

# Assistant text never reaches the core — only tool events ingested

**Reported** 2026-07-27 during live dogfooding (operator sees tool calls in the
cockpit but no agent text).

**Evidence** (from the durable log, `~/.local/state/patchbay/patchbay.sqlite3`):
recent turns' ingested transcript events contain ONLY
`"kind":"tool_requested"` (21) and `"kind":"tool_finished"` (22) — zero
message/text delta events. The cockpit renders correctly from what it
receives; the gap is upstream: pi-adapter's transcript projection
(`pi_session.ts` / `transcript_projection.ts`) is not producing text events
from the Pi session's message hooks, or the hooks aren't firing for this
session shape (gpt-5.6-sol via `pi --mode rpc`).

**First suspects**: event-hook wiring in `PiSession` (does it subscribe to
`message_update`/`message_end`?), the projection's event-kind mapping, or the
harness emitting text on a channel the adapter doesn't read. The v0.1.0
session-note "agent messages rendered word fragments out of order" arc
(report-chaining fix) is adjacent context.

**Repro**: send any prompt to the preprovisioned session from the cockpit;
observe tool events but no text; confirm via
`sqlite3 $PATCHBAY_DB_PATH "SELECT payload FROM events ..." | grep -o '"kind":"..."'`.
