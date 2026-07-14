---
id: backlog-authority-live-composition
kind: feature
stage: backlog
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Backlog: Live authority composition / consumer loop

## Source
Authority design review #3 (finding E re-scoped; finding 5 untracked follow-on). `feature-v0-core-authority` revision 3 drops the live path — the reactor is a pure fold exercised via replay/direct observe.

## Finding
Revision 3 deliberately ships authority as component-complete, not live: there is no live consumer loop, no composition root that feeds committed events to `SpawnDescendantTail` and writes the resulting descendant grants. This is acceptable for v0.1.0 (the ingress doesn't exist yet; SPEC verification floor), but the live path must land before authority is exercised end-to-end.

## Direction
Design a composition root + consumption protocol: startup rebuild → bootstrap (operator provisioning) → cursor catch-up → continuous committed-event delivery. Wire `SpawnDescendantTail`'s `Issuance` output to `ingest_descendant_grant` durably (deterministic grant_id already specified for idempotency). Couple with `feature-v0-protocol-seam`/`feature-v0-web-server` (the ingress that supplies the live event stream + verified issuer).

## Priority
Follow-on after the ingress features land. Not blocking for v0.1.0 authority component-complete delivery.
