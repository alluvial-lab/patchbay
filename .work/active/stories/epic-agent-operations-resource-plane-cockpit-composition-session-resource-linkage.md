---
id: epic-agent-operations-resource-plane-cockpit-composition-session-resource-linkage
kind: story
stage: done
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

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; caller-selected highest tier for the responsive resource destination, grant explanations, stale-state presentation, and conformance binding.
- Review weight: `thorough`, explicitly supplied by the autopilot caller; feature review is deferred to the orchestrator.
- Files changed: `web-cockpit/src/ui/resource-view.ts`, `target-scope.ts`, `runtime-resource-link.ts`, `shell.ts`, `security-view.ts`, `session-detail.ts`, `icons.ts`, `shell.css`; `web-cockpit/tests/resource-view.test.ts`, `security-view.test.ts`, `shell.test.ts`; `contracts/scripts/check-presentation.mjs`; `.mockups/design-system/components.css`, `components.html`; and the generated traceability block in `docs/UX.md`.
- Tests added: resource grouping and canonical detail, freshness dominance, tombstone/replacement context, exact resource Operation/grant projection, mobile list/detail, pooled-only runtime linkage, peer navigation, and exact/broad explanatory scope matching.
- Simplification: Resources reuses the shell rail/bottom tabs, shared Operation delivery, generated target scopes, and one resource renderer across desktop/mobile; the private security formatter was removed.
- Discrepancies from design: none. The resource destination intentionally has no mutation controls, so lockdown remains readable without adding a resource-specific disabled-action path.
- Adjacent issues parked: none.
- Verification: `cd web-cockpit && npm test` passed 99/99; contracts generated drift, presentation conformance (5 registries including `ResourceFreshnessState`, axe passed), and model-promotion checks passed.
