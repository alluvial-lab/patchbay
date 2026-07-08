---
source_handle: patchbay-protocol-subscription-cursors
fetched: 2026-07-07
source_path: docs/PROTOCOL.md
provenance: source-direct
---

# Attestation: Patchbay Protocol — log, cursor, snapshot, and subscription semantics

## Summary

The protocol document defines a single ordered durable event log per authority domain, cursor-based replay, snapshot reconciliation, and subscription semantics for long-lived event delivery. Subscriptions are grant-checked at transport establishment but are not lifecycle-bearing Operations. Reconnecting control surfaces submit a cursor and receive events after that cursor and/or a fresh snapshot.

## Key passages

1. Under "Revisions and cursors", the document says the coordination core owns "a single totally-ordered durable event log per authority domain" and every accepted state-transition event receives a monotonic, gap-free `LSN` at durable-commit time.

2. The document says event, cursor, and revision identity is the `(authority_domain_id, LSN)` tuple, not a bare LSN.

3. The document states that a control surface reconciles by submitting its cursor and the core returns events with `LSN > cursor` and/or a snapshot materialized at a later `LSN`.

4. The atomicity section says a terminal transition is committed to the log before it is reflected in snapshots or returned to control surfaces, and a snapshot reflects a consistent log prefix.

5. Under "Presence and Subscription", the document says Subscription is the deliberate exception to lifecycle-bearing Operations: it is grant-checked at transport establishment and reconciled by cursor on reconnect, but is not durably recorded as an Operation and does not enter `OperationState`.

6. The same section says that on reconnect, the control surface re-subscribes and submits its cursor; the core replays authorized events with `LSN > cursor` and/or returns a fresh snapshot.

7. Implementation notes say Observation streams are optimizations and snapshots repair missed events.
