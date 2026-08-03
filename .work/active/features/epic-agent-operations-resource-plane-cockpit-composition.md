---
id: epic-agent-operations-resource-plane-cockpit-composition
kind: feature
stage: drafting
tags: [foundation, ux, protocol]
parent: epic-agent-operations-resource-plane
depends_on: [epic-agent-operations-resource-plane-resource-identity, epic-agent-operations-resource-plane-resource-state, epic-agent-operations-resource-plane-capability-manifest]
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-07-30
---

# Cockpit resource composition

## Brief

Render operational resources in the cockpit alongside runtime sessions, per
the Phase 4.6 mockup decision: **Resources as a peer destination** surfacing
two resource kinds (pooled token-commune pools + direct-provider usage
windows) under the single admission rule, plus a **session runtime-context
strip** whose usage cell links to the relevant resource (a pool when
provider = token-commune, a direct-provider window otherwise).

This feature delivers the resource-side rendering and linkage: the Resources
destination (list + detail, pooled/direct sections, mobile affordances),
`ResourceView`/collection/projection-decoder/navigation/detail-renderer, and
the runtime-context strip's **resource-linkage** (the usage cell). Grant-scope
labels extend to resources. The composition obeys the conformance floor:
resource domain health stays distinct from session connectivity/lifecycle;
stale/unknown/offline resource state never renders as live.

It does **not** own the provider concept itself (model-vs-provider split,
provider-switch reconfigure) — that is sibling session-runtime scope (see
parent epic Mockups). This feature renders the resource linkage the provider
concept consumes; the interactable provider/model pickers depend on sibling
work and are mocked here as the design direction, not implemented by this
feature in isolation.

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: the UI-bearing consumer — depends on identity, state, and manifest; the conformance feature closes over it.

## Simplification opportunity

- Reuse the shared presentation-component layer (`StateBadge`, `CommandTimeline`, attention primitives) and the lockdown mockup's rail/destination pattern rather than a new navigation system.
- Keep resource health projection adapter-owned; do not coerce it into session connectivity/activity axes.

## Foundation references

- `docs/UX.md:13-62,85-135` — surface-neutral floor, shared presentation layer, required surfaces
- `docs/ARCHITECTURE.md` — human control surface plane
- `web-cockpit/src/domain/model.ts` — presentation model (session-shaped today; this adds `ResourceView`)
- `web-cockpit/src/ui/shell.ts`, `session-list.ts`, `session-detail.ts` — current session-centric surfaces

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`, `components.css`
- Screens: `.mockups/screens/epic-agent-operations-resource-plane/index.html`
  - `option-1.html` — Resources destination (pooled + direct-provider sections) — selected direction
  - `session-context.html` — runtime-context strip (interactable; resource-linkage in scope, provider concept sibling)
  - `session-context-mobile.html` — mobile pill-buttons + bottom sheet
- Selected: option-1 navigation + runtime-context strip resource-linkage (2026-07-30)

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will fill in interfaces, signatures, and implementation units. -->
