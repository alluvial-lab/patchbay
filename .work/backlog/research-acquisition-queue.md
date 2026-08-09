---
id: research-acquisition-queue
kind: feature
stage: backlog
tags: [research]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Research acquisition queue

The standing append-only queue of research-side acquisition candidates — sources
worth acquiring (blocking gaps + enriching candidates) consolidated from research
engagements. Promotion into this queue is the **verification-independent offgas**
(research-side write, unconditional); triage is operator-controlled via
`scripts/refresh-scan.py` (drains the queue: re-probes cited sources, classifies
re-acquirable/stale/still-dead, prints a batch worklist; writes nothing — the
operator triages, accepted items drive the refresh branch). This is the
`agentic-research` acquisition-queue drain loop.

Append new candidates here (one entry per source; **merge** `completes`, never
re-add an already-queued source).

## Candidates

### CodeAgent Mobile backend implementation
- **Engagement:** `v1-control-plane-and-spawn` (facet `peer-protocol-deep-dive`)
- **Class:** `primary-doc`
- **Web availability:** not in the fetched public client repo (README excludes backend/mobile/web); GitHub account enumeration + plausible-backend-name probes did not locate a public backend.
- **Completes:** whether CodeAgent command acceptance is durably stored; queue retention + terminal disposition; ack-vs-execution semantics; result idempotency; authoritative baton snapshot + ownership-conflict rules; stale-client fencing; exact session/plugin authority checks. Until acquired, CodeAgent's backend-level closure of the operation/ownership contract stays acquisition-gated (one matrix cell).
- **Urgency:** blocking (the closest UX-shape peer's backend durability is the one unknown that could narrow the moat claim further).
