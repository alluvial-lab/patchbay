---
id: authority-provenance-hardening
kind: feature
stage: drafting
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Authority provenance hardening

## Brief
Consolidate the authority follow-ups absorbed from the parked backlog so acceptance provenance, authorization decisions, replay, and live composition are explicit and durable. Absorbed findings:

- `backlog-authority-durable-acceptance-metadata`: server-attested acceptance metadata must survive replay to populate descendant-grant provenance and audit linkage.
- `backlog-authority-failed-authorization-audit`: denied authorization attempts need distinct durable security audit records rather than only submission rejections.
- `backlog-authority-grant-selection-determinism`: overlapping matching grants need a stable selection rule and replay-stable authorization provenance.
- `backlog-authority-ingest-pre-append-conflict-check`: authority ingest must reject conflicts before append and make identical retries durable-idempotent so the log cannot be poisoned.
- `backlog-authority-live-composition`: a live composition root must consume committed events and durably feed descendant-grant issuance.
- `backlog-authority-payload-actor-in-descendant-issuance`: descendant subjects must derive from verified acceptance identity, not a self-asserted Operation sender.
- `backlog-authority-replay-gap-detection`: authority replay must detect LSN gaps and unspecified event kinds instead of silently accepting or dropping corruption.
- `backlog-elicitation-responder-authority`: elicitation response Operations must verify the issuer against the correlated elicitation's expected responder.

Feature-design verifies currency — some findings may be addressed by v0.1/resource-plane authority work; spawn child stories for the still-open ones.

## Simplification opportunity
Consolidate overlapping authority, acceptance, and replay checks into shared boundary and writer primitives; avoid preserving separate authority-only mechanisms where the core acceptance/storage seams can enforce the same guarantees.
