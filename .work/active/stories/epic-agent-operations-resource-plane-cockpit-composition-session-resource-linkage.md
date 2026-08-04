---
id: epic-agent-operations-resource-plane-cockpit-composition-session-resource-linkage
kind: story
stage: implementing
tags: [ux, protocol]
parent: epic-agent-operations-resource-plane-cockpit-composition
depends_on: [epic-agent-operations-resource-plane-cockpit-composition-shared-resource-rendering]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Resources destination and session resource linkage

## Checkpoint

Deliver the selected Resources peer destination, responsive list/detail
composition, canonical resource wrapper, pooled/direct local renderers,
resource grant-scope labels, registry-derived freshness presentation, and the
session runtime-context resource-linkage slot.

The usage linkage accepts only an exact session-supplied resource identity that
resolves to a decoded pooled-provider projection. It navigates to that resource
and renders as a mobile pill. It does not infer provider from the opaque model
string, link direct-provider windows, implement provider/model pickers, open
their bottom sheet, or issue `reconfigure`; those remain the explicit sibling
session-runtime seam recorded in the parent.

## Primary files

- `web-cockpit/src/ui/resource-view.ts` (new)
- `web-cockpit/src/ui/target-scope.ts` (new)
- `web-cockpit/src/ui/runtime-resource-link.ts` (new)
- `web-cockpit/src/ui/shell.ts`
- `web-cockpit/src/ui/security-view.ts`
- `web-cockpit/src/ui/session-detail.ts`
- `web-cockpit/src/ui/icons.ts`
- `web-cockpit/src/ui/shell.css`
- `contracts/scripts/check-presentation.mjs`
- `.mockups/design-system/components.css`
- `.mockups/design-system/components.html`
- `docs/UX.md` generated presentation traceability block
- `web-cockpit/tests/resource-view.test.ts` (new)
- `web-cockpit/tests/security-view.test.ts`
- `web-cockpit/tests/shell.test.ts`

## Acceptance evidence

- Desktop rail and mobile bottom tabs expose Resources as a peer destination;
  desktop two-pane and mobile list-to-detail use the same renderer.
- Pooled, direct, unavailable/invalid, stale/unknown, and tombstone/replacement
  cases preserve identity-before-intent and freshness dominance.
- Canonical wrapper shows exact identity, source/revisions, collection tier,
  freshness, visible grant context, and resource Operation delivery before any
  adapter-domain cards.
- Security and resource detail format exact resource scopes; explanatory grant
  containment never gates controls or substitutes for core authority.
- `ResourceFreshnessState` has exact proto/CSS/showcase parity and accessible
  current/stale/unknown presentation; shell CSS does not rebind those states.
- A valid pooled-resource linkage opens the exact detail on desktop/mobile;
  stale linkage remains honestly labeled, and direct/missing/tombstoned/invalid
  targets never become live links.
- Full web cockpit, presentation conformance, generated-contract drift, and
  model/vector metadata checks pass without weakening existing tests.

## Ordering

Final implementation checkpoint. On green verification, child stories advance
directly to `done`; the integrated feature proceeds to the caller-mandated
`thorough` feature review.
