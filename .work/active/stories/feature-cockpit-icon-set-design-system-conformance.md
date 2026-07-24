---
id: feature-cockpit-icon-set-design-system-conformance
kind: story
stage: done
tags: [ux, design-system, ui]
parent: feature-cockpit-icon-set
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Story: Icon primitive and presentation conformance

Create the skin-able icon primitive before cockpit consumers use it.

## Checkpoint

- Add `--icon-size-sm`, `--icon-size-md`, and `--icon-size-lg` to `.mockups/design-system/tokens.css`.
- Add `.icon` plus small/large size variants to `.mockups/design-system/components.css`: `currentColor`, no fill, 2px rounded Lucide stroke, fixed flex geometry, no pointer events.
- Add a standalone icon section to `.mockups/design-system/components.html` showing all cockpit icons, all size variants, and correctly labeled icon-only buttons.
- Extend `contracts/scripts/check-presentation.mjs` so `icon` is a locked primitive that requires an uncommented CSS selector and a DOM showcase element; extend its meta-test with a failing icon fixture.

## Acceptance evidence

- `node contracts/scripts/check-presentation.mjs` passes on the actual artifacts.
- `node contracts/scripts/test-presentation-check.mjs` proves omitted icon CSS/showcase coverage exits non-zero.
- The showcase is usable in light and dark themes and retains the single-file/no-build-step convention.

## Implementation notes
- Execution capability: direct-read only; the design named a small, self-contained CSS/mockup/check integration surface.
- Review weight: standard (default; feature review remains pending).
- Files changed: `.mockups/design-system/tokens.css`, `.mockups/design-system/components.css`, `.mockups/design-system/components.html`, `contracts/scripts/check-presentation.mjs`, `contracts/scripts/test-presentation-check.mjs`.
- Tests added/removed: expanded the presentation meta-test with failing missing-icon-CSS and missing-icon-showcase fixtures; these protect the locked primitive's two-sided conformance binding.
- Simplification: none beyond the shared tokenized primitive.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: `node contracts/scripts/check-presentation.mjs` and `node contracts/scripts/test-presentation-check.mjs` passed.
