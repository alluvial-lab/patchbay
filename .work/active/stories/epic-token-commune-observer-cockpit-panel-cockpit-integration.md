---
id: epic-token-commune-observer-cockpit-panel-cockpit-integration
kind: story
stage: implementing
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-verdict-synthesis]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Cockpit data-layer and grant integration

## Checkpoint

Route the two token-commune contracts through the existing local resource decoder used by snapshot and live folds, adapt canonical `ResourceView`/collection/grant state into shared pool summaries, and apply deny-by-default local query-grant visibility independently to pool and member draw.

## Primary files

- `web-cockpit/src/domain/resource-projection.ts`
- `web-cockpit/src/domain/model.ts`
- `web-cockpit/src/ui/resource-view.ts`
- `web-cockpit/src/ui/target-scope.ts`
- `web-cockpit/tests/resource-projection.test.ts`
- `web-cockpit/tests/resource-view.test.ts`

## Acceptance evidence

- Snapshot and live `RESOURCE_STATE` upserts use the same decoder path; no new fetch/cache exists.
- Adapter/provider collisions do not cross-join; ids/member names/array position cannot redirect draw.
- Stale/unknown/invalid/unreconciled token resources become honest summaries rather than vanishing or looking current.
- Pool and draw each require a live non-expired visible `query` grant under strict exact/adapter/fleet/domain scope; local filtering never claims core authority or infers admin role.
- Non-token generic Resources rendering remains available and recognized token rows are not duplicated into a drill-down.

## Ordering

Depends on verdict synthesis. The option-7 component depends on this complete input seam.
