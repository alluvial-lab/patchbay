---
id: cockpit-ux-polish-delivery-cards
kind: story
stage: implementing
parent: cockpit-ux-polish
depends_on: [cockpit-ux-polish-settings, cockpit-ux-polish-session-rows]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Cockpit instruction-card delivery stability

Fold command delivery into the operator instruction card and reserve a stable action slot so state transitions do not move the transcript or create a second floating box.

## Checkpoint

- Refine `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/operation-delivery.ts`, and the relevant `web-cockpit/src/ui/shell.css` layout rules.
- Introduce an explicit instruction-card composition with body, target identity, delivery state, terminal-race explanation, failure mapping, and a fixed-width interrupt/cancel slot.
- Reuse `renderOperationDelivery`, canonical `OperationState` bindings, and existing failure/retry-safety components; do not create a delivery-specific state registry.
- Keep the reserved action slot present for non-running states so accepted → delivered → running changes do not cause layout shift. The slot may be empty or show the appropriate action for the current state.

## Acceptance evidence

- [ ] Running, completed, failed, and cancellation-race examples render delivery inside the instruction card and keep message ordering stable.
- [ ] Cancel/interrupt controls remain keyboard reachable, disabled during lockdown, and labeled with the canonical reason when unavailable.
- [ ] Delivery state remains visually distinct from session liveness and transcript content.
- [ ] Narrow-width tests verify no horizontal overflow and no vertical jump when the action appears/disappears.
