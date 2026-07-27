---
id: epic-agent-operations-resource-plane
kind: epic
stage: drafting
tags: [foundation, protocol, adapter, ux]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-26
updated: 2026-07-26
---

# Agent-operations resource plane

## Brief

Evolve Patchbay from a control plane centered exclusively on runtime sessions into an operator-owned agent-operations control plane: sessions remain the product center, while operational resources that materially govern agent availability, capability, or safe control become first-class adapter targets. The first pressure case is model-capacity infrastructure, but the architecture must admit a resource only when its state changes what the operator can ask an agent to do or requires human action to keep agent work operating.

This epic promotes the existing generic resource target seam into a designed identity, snapshot/revision, Observation, query, authority, and presentation path without pretending resources are runtime sessions. It also establishes the composition rule for the cockpit: canonical protocol and presentation primitives provide delivery, reconciliation, stale-state, authority, and attention honesty; adapter-shaped projections provide domain views above that floor. It does not create an arbitrary monitoring platform or a dynamically loaded third-party UI plugin system.

## Strategic decisions

- **What does Patchbay become after v0.1.0?** A personal agent-operations control plane for sessions and the operational resources that govern their availability, capability, and safe control; it does not become a generic infrastructure dashboard.
- **Are resources represented as sessions?** No. Runtime sessions and operational resources are distinct target categories with honest identities and state; resource health must not be coerced into session connectivity or activity.
- **How does adapter-specific UX fit surface-neutrality?** The shared conformance floor remains mandatory, while adapter-shaped projections may compose richer domain views without inventing protocol states or presenting stale data as live.
- **What is the near-term human authority model?** One human operator per Patchbay deployment remains the committed short-term shape; shared multi-human Patchbay authority is not required by the resource plane.
- **What qualifies as an operational resource?** It must materially affect agent capability/availability or require operator attention to keep agent work operating. Arbitrary service telemetry is outside the product boundary.

## Arc position

This is the foundation epic for the post-v0.1.0 agent-operations arc. `epic-token-commune-observer` depends on it and supplies the first real resource adapter. `epic-token-commune-control-attention` then exercises durable mutation and attention workflows over the same resource model.

## Capability outline

- resource identity and target-resolution semantics distinct from runtime-session identity;
- resource snapshot/revision and reconnect behavior with explicit partial/no-snapshot degradation;
- resource-scoped query, Observation, authority, audit, and subscription behavior;
- presentation bindings that keep resource domain health separate from connectivity and command lifecycle;
- cockpit navigation/composition for sessions plus operational resources;
- adapter-shaped projection contracts that remain bounded by the surface-neutral conformance floor;
- conformance evidence showing a resource adapter cannot bypass Patchbay authority, durability, or stale-state rules.

## Scope boundaries

- No multi-human shared Patchbay deployment, delegation, quorum approval, federation, or agent-to-agent work routing.
- No model request/data-plane proxying through Patchbay.
- No universal monitoring ontology or arbitrary dashboard/plugin marketplace.
- No requirement that every adapter expose resources; Pi may remain session-centered.
- Exact wire registries and presentation extension mechanics are design work for this epic, not preselected by this scope item.

## Simplification opportunity

Use the existing `TargetScopeKind = resource`, Operation/Observation envelopes, snapshot discipline, authority checks, and presentation primitives rather than creating a parallel control subsystem. Eliminate the temptation to synthesize fake runtime sessions, generations, or activity states for non-session resources. Keep one adapter projection path instead of separate one-off diagnostic and dashboard state stores.

## Mockups

Epic design must mock the cross-feature cockpit composition for sessions plus resources after it decomposes the protocol and surface work. Existing palette and component artifacts under `.mockups/design-system/` are inherited.

## Extension pressure classification

- **Committed post-v0.1.0 direction:** first-class operational resources; personal one-operator control; adapter-shaped projections above the conformance floor.
- **Reserved seams:** dynamically loaded third-party surface plugins, multi-human shared authority, resource-to-resource coordination, and a broad external adapter ecosystem.
- **Explicitly rejected for this arc:** representing resource health as session connectivity/activity, turning Patchbay into generic monitoring, or making adapter-specific state part of the core protocol registry without a promotion ceremony.
