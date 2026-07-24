---
id: feature-cockpit-icon-set-cockpit-chrome
kind: story
stage: done
tags: [ux, ui]
parent: feature-cockpit-icon-set
depends_on: [feature-cockpit-icon-set-design-system-conformance]
release_binding: v0.1.0
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Story: Apply typed Lucide icons across cockpit chrome

Convert cockpit action chrome only after the shared primitive and its conformance binding exist.

## Checkpoint

- Add `web-cockpit/src/ui/icons.ts`: a typed local Lucide path catalog and DOM SVG factory. Preserve the current paperclip path exactly.
- In `web-cockpit/src/ui/session-detail.ts`, replace the hand-built paperclip, text Send, Unicode back arrow, and text contextual Cancel/Interrupt buttons with accessible icon-only controls: paperclip, arrow-up, arrow-left, x, and square.
- In `web-cockpit/src/ui/shell.ts`/`shell.css`, add disabled, honestly unavailable sidebar Spawn (`plus`) and Attach (`link`) header affordances; they must not emit or infer Operations.
- Make chevron icons available through the catalog for genuine disclosure consumers, but do not add a delivery disclosure or trace UI: v0.1.0 remains current CommandState + last transition only.
- Update `web-cockpit/tests/shell.test.ts` for icon DOM/accessibility semantics and unchanged submission behavior.

## Acceptance evidence

- `cd web-cockpit && npm test` passes.
- Converted icon-only buttons have distinct `aria-label`/`title` values and `.icon[aria-hidden="true"]` descendants.
- No converted call site owns SVG paths or raw SVG setup; no expanded delivery trace appears.

## Implementation notes
- Execution capability: direct-read only; the feature design named the catalog/factory, cockpit consumers, and test integration points.
- Review weight: standard (default; feature review remains pending).
- Files changed: `web-cockpit/src/ui/icons.ts`, `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/shell.css`, `web-cockpit/tests/shell.test.ts`.
- Tests added/removed: factory-shape and compile-time vocabulary assertions, plus DOM assertions for accessible icon controls, disabled sidebar affordances, unchanged contextual actions, and no delivery disclosure.
- Simplification: removed the hand-built paperclip SVG and converted chrome icon setup to the single catalog/factory.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: `cd web-cockpit && npm test` passed (43 tests); presentation check and its meta-test passed after the integrated change.
