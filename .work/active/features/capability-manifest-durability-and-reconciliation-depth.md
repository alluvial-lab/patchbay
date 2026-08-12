---
id: capability-manifest-durability-and-reconciliation-depth
kind: feature
stage: drafting
tags: [adapter, architecture]
parent: null
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Capability manifest: declared durability + reconciliation depth

## Origin (research-grounded)
Harvested from Mission Control's adapter-neutral structure (MIT; inspiration, not code reuse). Decomposed from `mc-architectural-harvest` directions **1 + 4**.
- **Source campaign:** `.research/analysis/campaigns/v1-control-plane-and-spawn/` (facet `peer-protocol-deep-dive`, attestation `mission-control-src`).
- **Harvest item:** `.work/active/stories/mc-architectural-harvest.md` (directions 1 + 4).

## Direction
The v0.2.0 capability manifest (`docs/ARCHITECTURE.md`: declares target category + `ResourceKind` + snapshot tier + projection schema; advisory, never replaces grants/delivery outcomes) declares *what* an adapter targets, not *how durably* it can guarantee outcomes. Extend it with:

- **Declared durability dimensions** (separate from runtime "is installed/reachable" detection): dedup strength, continuation proof, cursor support, generation-fence support. Require a complete manifest; **default uncertain fields false**.
- **Reconciliation-strength declaration** + return `unknown` / manual-required when the substrate can't prove an outcome (Patchbay already has `unknown` as the transport terminal from the token-commune coordination; declare the adapter's reconciliation strength explicitly rather than discovering it at runtime).

## Scope / dependencies
Extends the shipped v0.2.0 capability manifest (`epic-agent-operations-resource-plane-capability-manifest` arc, collapsed at v0.2.0 release). No active `depends_on` (the v0.2.0 dependency is historical/shipped). Design-bearing → route through `feature-design`.

## Non-goal
Do not let capability declarations replace grants or adapter-authoritative delivery outcomes (the v0.2.0 invariant holds).
