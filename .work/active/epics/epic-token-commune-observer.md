---
id: epic-token-commune-observer
kind: epic
stage: drafting
tags: [adapter, protocol, ux, integration]
parent: null
depends_on: [epic-agent-operations-resource-plane]
release_binding: null
gate_origin: null
created: 2026-07-26
updated: 2026-07-26
---

# token-commune observer adapter

## Brief

Build token-commune as Patchbay's first materially non-session reference adapter and the first consumer of the operational resource plane. The adapter is outboard: it authenticates to token-commune with the operator's member-scoped or admin-scoped gateway credential, reads metadata-only pool state, and reports resource snapshots and Observations to Patchbay. LLM request and response traffic remains entirely on token-commune's gateway data plane and never traverses Patchbay.

The first delivery is deliberately read-only and independently useful. The cockpit presents provider pools, contribution health, model availability, member draw, fingerprint state, and capacity/lifecycle events alongside Pi sessions through an adapter-shaped resource projection. token-commune's own CLI and `/ui` remain independent fallbacks. The adapter must state polling, snapshot completeness, event-gap, credential-scope, and staleness behavior honestly rather than claiming an event stream or authoritative reconstruction that the upstream contract cannot supply.

## Strategic decisions

- **Is token-commune merely a candidate?** No. It is the selected second reference adapter for the current v1 direction and the concrete design-pressure system for Patchbay's resource plane; conformance evidence must still prove that claim.
- **What ships first?** A read-only observer that is useful before any admin mutation API or Elicitation flow exists.
- **Where does integration code live?** Patchbay owns the reference adapter and a consumer-owned port over token-commune's external API. Required token-commune API work is coordinated in that repository rather than coupled through its internal implementation modules.
- **How are humans represented?** Each Patchbay deployment remains personal and uses its operator's token-commune credential. The shared gateway retains upstream member/admin policy; Patchbay grants add local defense in depth rather than replacing gateway authorization.
- **How rich is the UI?** The adapter gets a purpose-built pool/resource panel composed with Patchbay's shared primitives; it is not reduced to a generic session list.

## Arc position

Depends on `epic-agent-operations-resource-plane`. It is the implementation consumer that validates resource identity, snapshots, polling/Observation ingestion, adapter credential handling, and adapter-shaped cockpit projection. `epic-token-commune-control-attention` depends on this observer and adds mutations and human-action workflows.

## Capability outline

- token-commune adapter registration, attachment evidence, lifecycle, and scoped gateway credential handling;
- gateway/provider/contribution resource discovery and stable identity mapping;
- read-only queries for pool state, personal draw, model availability, fingerprint status, and recent events;
- explicit polling-to-Observation ingestion with deduplication, gap behavior, source timestamps, and stale-state handling;
- resource snapshots at the strongest tier the upstream API can actually satisfy;
- member and admin read views governed by both upstream credentials and Patchbay grants;
- responsive token-commune resource panel and CLI projections;
- adapter conformance vectors and end-to-end tests proving reconnect, snapshot, source authentication, redaction, and adapter-failure behavior;
- a documented external API contract boundary with token-commune, including any required cursor, identity, or read-scope additions.

## External collaboration boundary

Patchbay work items cannot own token-commune's repository state. Any gateway API additions—such as stable resource identifiers, event cursors, scoped read credentials, or snapshot completeness guarantees—must be scoped and delivered in token-commune's own substrate. This epic records those as external prerequisites and consumes only an explicit external contract, never `packages/shared` internals by filesystem coupling.

## Scope boundaries

- No admin mutations, contribution approval, decree changes, or fingerprint acceptance in this epic.
- No OAuth/device-flow secret transport through Patchbay.
- No multi-human shared Patchbay authority domain.
- No LLM traffic proxying, prompt/response capture, routing decisions, or allocation policy in Patchbay.
- No claim that polling is streaming or that recent-event reads repair unlimited history.

## Simplification opportunity

Reuse Patchbay's adapter lifecycle, query lifecycle, Observation ingestion, resource snapshot path, redaction rules, and presentation primitives. Keep token-commune policy in the gateway and avoid duplicating allocation, capacity, or role logic in Patchbay. Retain token-commune's `/ui` and CLI as boring independent fallbacks rather than replacing them.

## Mockups

Epic design must produce responsive mockups for the token-commune pool/resource surface and its composition with the existing session cockpit. Existing design-system tokens and components are inherited; domain-specific capacity, contribution-health, and fingerprint components may extend the showcase before screen work.

## Extension pressure classification

- **Committed post-v0.1.0 direction:** token-commune as the second reference adapter; outboard metadata-only integration; personal per-operator deployments; a rich resource panel.
- **Reserved seams:** upstream push/webhook delivery, third-party packaging of the adapter, cross-deployment shared presence, and generic dynamic adapter UI modules.
- **Explicitly rejected for this epic:** Patchbay in token-commune's LLM data path, copying gateway policy into Patchbay, shared filesystem imports from token-commune internals, or presenting quota health as runtime-session liveness.
