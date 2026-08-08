---
id: epic-token-commune-observer-cockpit-panel-cockpit-integration
kind: story
stage: done
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-verdict-synthesis]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-08
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

## Implementation notes

- Extended the existing `decodeResourceProjection` registry with the two exact token-commune descriptor pairs; snapshot and live folds continue to share that single ingress path.
- Added `tokenCommunePanelInput` over canonical `PresentationModel.resources` and per-kind collection metadata. Pool and member-draw records are independently deny-by-default filtered by visible, unrevoked, unexpired `query` grants under strict resource/adapter/fleet/domain containment before the shared compositor runs.
- Recognized token resource kinds render once through the local known panel path and are not duplicated into generic detail; no route, poller, cache, dynamic renderer, admin role, or core authority claim was added.
- Verification: web type build plus 21 focused decoder/resource/panel tests passed, including exact adapter/provider joins, pool-vs-draw grant separation, expiry/revocation, and non-duplication.
