---
id: epic-token-commune-observer-cockpit-panel-pool-compositor
kind: story
stage: done
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-projection-decoder]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-08
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

## Implementation notes

- Implemented one deterministic summary per exact `(adapterId, provider)`; resource ids, private member labels, synthesized contribution keys, and input order do not participate in joins.
- The capacity signal selects the maximum real non-null `5h` reading with newest-observation tie-breaks. Null, absent, non-5h, stale, current, and ambiguous draw evidence remain distinct; the summary has no aggregate/remaining/mean/weighted capacity field.
- Verification: `cd operator-domain && npm test` — 7/7 tests passed, including wrong-adapter join, divergent draw, null/non-5h selection, anonymous output, and exact summary-shape witnesses.
