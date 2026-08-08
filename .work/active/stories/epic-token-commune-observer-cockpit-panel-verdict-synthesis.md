---
id: epic-token-commune-observer-cockpit-panel-verdict-synthesis
kind: story
stage: implementing
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-pool-compositor]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Patchbay-owned verdict synthesis

## Checkpoint

Implement the feature's exact, pure verdict precedence over canonical pool freshness, credential distribution, per-contribution 5h facts, and current model availability. Keep draw outside the verdict and expose fail-closed unknown/model-unavailable outcomes where the four selected normal labels would lie.

## Primary files

- `operator-domain/src/token-commune.ts`
- `operator-domain/tests/token-commune.test.ts`

## Acceptance evidence

- Stale/unreconciled/tombstoned wrappers dominate every positive native label.
- Auth broken requires no fresh contribution plus native auth-broken evidence.
- Model-unavailable follows current reported zero-available model evidence.
- Pool exhaustion requires a non-empty listing whose contributions are all exhausted or all measured at 100% in the 5h display window; an empty listing and one high maximum are insufficient.
- Runnable requires fresh credentials, an exact available model, and a native sub-100% 5h reading.
- Partial/contradictory evidence is unknown and draw never changes the verdict.

## Ordering

Depends on the signal compositor. Web and CLI integrations consume this completed synthesis contract.
