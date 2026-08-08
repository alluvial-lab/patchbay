---
id: epic-token-commune-observer-cockpit-panel-pool-compositor
kind: story
stage: implementing
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-projection-decoder]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Per-pool signal compositor

## Checkpoint

Compose decoded provider-pool and member-draw resources into one deterministic summary per exact `(adapter_id, provider)`, preserving independent wrapper freshness and observed times. Produce draw allowance, anonymous credential distribution, exact model ids, and the maximum real 5h utilization without a pool aggregate.

## Primary files

- `operator-domain/src/token-commune.ts`
- `operator-domain/tests/token-commune.test.ts`

## Acceptance evidence

- Draw joins only by exact adapter/provider and is current, stale, unavailable, or ambiguous without selecting/aggregating multiple reports.
- Health counts are recomputed from anonymous rows and disagreeing supplied counts reject.
- Highest 5h uses only non-null native `usedFraction`; no listing, no 5h row, all-null rows, stale, and current remain distinct.
- Capacity output contains no remaining/mean/sum/weighted value or contributor/member identity.
- Exact available model ids pass through unchanged; no alias appears.

## Ordering

Depends on the manifest-bound decoder. Verdict synthesis depends on this stable signal model.
