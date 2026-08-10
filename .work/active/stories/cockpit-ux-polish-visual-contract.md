---
id: cockpit-ux-polish-visual-contract
kind: story
stage: implementing
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

- [ ] The implementation plan maps each visual region to existing or explicitly named classes and does not invent protocol states.
- [ ] The presentation conformance check remains green; canonical state labels remain derived from the existing registries.
- [ ] The selected mockup is linked from the parent feature and remains committed alongside implementation work.
