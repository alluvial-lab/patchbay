---
id: sessions-soundness-coverage
kind: feature
stage: drafting
tags: [protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Sessions soundness and coverage

## Brief
Consolidate parked session-registry soundness and evidence gaps into one currency-checked feature. Absorbed findings:

- `backlog-sessions-authority-domain-isolation`: bind session registries and target resolution to their authority domain and reject cross-domain access.
- `backlog-sessions-idempotency-and-concurrency`: compare redelivered event identity/payloads and serialize or otherwise make warm read-decide-append paths safe under concurrency and ordering.
- `backlog-sessions-test-coverage-gaps`: add replay-corruption, acceptance integration, malformed-event, resolver-boundary, and multi-identity property coverage for the session seam.
- `backlog-session-report-source-ordering`: add adapter-side report revisions or an equivalent rule so delayed reports cannot roll mutable session fields backward.

Feature-design verifies currency — some findings may be addressed by v0.1/resource-plane authority work; spawn child stories for the still-open ones.

## Simplification opportunity
Prefer one domain-scoped session/replay validation path and shared test fixtures over separate defensive checks for each report type; retain only coverage that exercises stable session and acceptance contracts.
