---
id: cockpit-ux-polish-visual-contract
kind: story
stage: done
parent: cockpit-ux-polish
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Cockpit polish visual contract

Define the selected mockup direction as the implementation checkpoint for the composite cockpit surface. Keep the existing shell, tokens, and registry-derived presentation primitives as the single vocabulary.

## Checkpoint

- Use `.mockups/screens/cockpit-ux-polish/option-1.html` as the desktop reference and its responsive behavior as the baseline.
- Preserve the existing two-pane desktop / drill-in mobile topology; do not introduce a second cockpit shell or a new transcript model.
- Name the stable DOM seams shared by settings, session rows, and instruction cards before changing CSS: target identity, session status, instruction body, delivery slot, interrupt slot, and composer.

## Acceptance evidence

- [x] The implementation plan maps each visual region to existing or explicitly named classes and does not invent protocol states.
- [x] The presentation conformance check remains green; canonical state labels remain derived from the existing registries.
- [x] The selected mockup is linked from the parent feature and remains committed alongside implementation work.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-luna` high, direct source mapping only; this checkpoint is documentation/conformance-only and nested delegation is prohibited.
- Review weight: thorough (caller override), feature review remains pending after integrated implementation.
- Files changed: `.work/active/stories/cockpit-ux-polish-visual-contract.md`.
- Tests added/removed: none; `node contracts/scripts/check-presentation.mjs` passed with axe-core.
- Simplification: retained the existing shell, design-system tokens, and registry-derived primitives; no new protocol states or visual vocabulary were introduced.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Evidence: selected `.mockups/screens/cockpit-ux-polish/option-1.html` maps to existing `.cockpit`, `.sidebar`, `.session-row`, `.session-detail`, `.msg`, `.delivery-line`, `.composer`, and new settings/instruction-card seams to be implemented in the following checkpoints.
