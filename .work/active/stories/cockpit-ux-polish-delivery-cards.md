---
id: cockpit-ux-polish-delivery-cards
kind: story
stage: done
parent: cockpit-ux-polish
depends_on: [cockpit-ux-polish-settings, cockpit-ux-polish-session-rows]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Cockpit instruction-card delivery stability

Fold command delivery into the operator instruction card and retain a stable structural action-slot reserve across state transitions without creating a second floating box.

## Checkpoint

- Refine `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/operation-delivery.ts`, and the relevant `web-cockpit/src/ui/shell.css` layout rules.
- Introduce an explicit instruction-card composition with body, target identity, delivery state, terminal-race explanation, failure mapping, and a fixed-width interrupt/cancel slot.
- Reuse `renderOperationDelivery`, canonical `OperationState` bindings, and existing failure/retry-safety components; do not create a delivery-specific state registry.
- Keep the reserved action slot present for non-running states so accepted → delivered → running changes retain the same action-slot element and CSS reserve. The slot may be empty or show the appropriate action for the current state; browser geometry beyond this structural contract is not claimed.

## Acceptance evidence

- [x] Running, completed, failed, and cancellation-race examples render delivery inside the instruction card and keep message ordering stable.
- [x] Cancel/interrupt controls remain keyboard reachable, disabled during lockdown, and labeled with the canonical reason when unavailable.
- [x] Delivery state remains visually distinct from session liveness and transcript content.
- [x] Narrow-width structure tests verify bounded overflow rules plus one action-slot element with the same CSS reserve in every state; they do not claim measured browser geometry.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-luna` high, direct implementation after settings and session-row checkpoints.
- Review weight: thorough (caller override), feature review remains pending after integrated implementation.
- Files changed: `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/operation-delivery.ts`, `web-cockpit/src/ui/shell.css`, `web-cockpit/tests/shell.test.ts`.
- Tests added/removed: canonical-state action-slot coverage, instruction-card integration assertions, and responsive CSS checks; web-cockpit shell tests pass after type build.
- Simplification: delivery remains one shared `renderOperationDelivery` primitive; instruction cards compose it without a second delivery registry or floating status box.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Review pass 2: the presentation fold records the first correlated cancel/interrupt Operation as a typed pending relation while the original target is non-terminal. Only a later production `COMMAND_TRANSITION` for that target supplies its terminal state and yields `<terminal> after cancellation/interrupt requested`; terminal-first ordering remains `<terminal> before ... arrived`.
- Review pass 2 tests feed both durable arrival orders through production `fold` inputs, cover both cancel and interrupt relation variants, and assert that the pending relation never synthesizes target terminal authority.
- Review pass 2 evidence is deliberately structural: canonical-state tests prove one action-slot element with the same CSS reserve in every rendered state; no browser-geometry/no-layout-shift claim is made.
