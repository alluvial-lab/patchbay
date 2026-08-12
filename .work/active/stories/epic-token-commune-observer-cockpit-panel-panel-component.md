---
id: epic-token-commune-observer-cockpit-panel-panel-component
kind: story
stage: done
tags: [ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-cockpit-integration]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-07
updated: 2026-08-08
---

# Option-7 token-commune panel component

## Checkpoint

Implement the signed-off calm per-pool list inside the Resources destination with option-7 chrome, responsive rows, summary counts, exact signal labels, stale/unknown dominance, and the complete derivation/honesty footer. Do not add per-contribution drill-down or controls.

## Primary files

- `web-cockpit/src/ui/token-commune-panel.ts`
- `web-cockpit/src/ui/resource-view.ts`
- `web-cockpit/src/ui/shell.css`
- `web-cockpit/tests/token-commune-panel.test.ts`
- `web-cockpit/tests/resource-view.test.ts`
- `web-cockpit/tests/shell.test.ts`

## Acceptance evidence

- Eyebrow, heading, summary, model/draw/health/5h/verdict column order, labels, and responsive collapse match `option-7.html`.
- Runnable, exhausted, auth-broken, telemetry-stale, model-unavailable, unknown, null-reading, and no-reading states are distinct; stale is never styled live.
- Model ids and native availability are exact live catalog values—including unavailable models—and no Patchbay alias is invented.
- Footer owns draw conversion, maximum-native-5h selection, display-window caveat, verdict synthesis, PARTIAL/polled ages, no aggregate, anonymous attribution, and omitted drill-down.
- DOM/text/attributes expose no raw JSON, member name, contribution subkey/id, pool remaining/mean/sum, or mutation/admin control; tests reject fabricated aliases without rejecting an exact source id.

## Ordering

Depends on cockpit integration. Final honesty evidence depends on this component and the CLI projection.

## Implementation notes

- Implemented the signed-off option-7 calm provider list with the exact model → draw → credentials → highest-5h → verdict column order, responsive single-column collapse, selected-row handling, and existing cockpit tokens/chrome.
- Stale telemetry uses the established stale surface and never receives runnable styling. Credential evidence and capacity telemetry remain separately labeled, exact unavailable model ids stay visible, and the upstream-rejected `gpt-5.6` alias is withheld at the surface as defense in depth.
- The footer visibly owns native draw formatting, maximum real anonymous 5h selection, Patchbay's display-window choice, full Patchbay verdict precedence, polling/PARTIAL and age distinctions, no native aggregate, anonymous contributors, and omitted MVP drill-down.
- Verification: full `web-cockpit` build/test passed 113/113; the panel's axe-core scan found 0 critical violations.
