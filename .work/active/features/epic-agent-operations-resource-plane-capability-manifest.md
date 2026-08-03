---
id: epic-agent-operations-resource-plane-capability-manifest
kind: feature
stage: drafting
tags: [foundation, protocol, adapter]
parent: epic-agent-operations-resource-plane
depends_on: [epic-agent-operations-resource-plane-resource-identity]
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-07-30
---

# Resource capability manifest & projection contract

## Brief

Extend the adapter capability manifest so an adapter can declare the resource
kinds it targets, the resource payload/projection schemas it emits, and
per-resource snapshot tiers — and define the executable projection-contract
boundary that `UX.md:52-62` specifies but does not mechanize: canonical
protocol/presentation primitives (delivery, reconciliation, stale-state,
authority, attention honesty) remain mandatory, while adapter-shaped domain
projections compose richer views above that floor.

Today the manifest (`contracts/proto/patchbay/adapter.proto`) has no target
categories, no resource kinds, no projection schema identifiers, and
`snapshot_support` is adapter-wide and session-termed — it cannot say which
resource collection is complete or which axes are partial. This feature adds
the resource-aware manifest fields and the target-category registry. The
registry must be **extensible** so the reserved OKF knowledge-bundle third
kind (see parent epic) is an additive future promotion, not a rearchitecture.

The manifest must support multiple resource kinds under the single admission
rule: pooled token-commune pools (adminable) and direct-provider usage windows
(read-only). It must not accidentally admit foreign data sources (the OKF
third kind) while staying honest about operational resources — that promotion
is reserved, with OKF v0.2 named as the candidate format.

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: contract foundation — depends on `resource-identity`; consumed by `cockpit-composition` (which renders the declared projections) and `conformance`.

## Simplification opportunity

- Extend the existing `AdapterCapability` rather than creating a separate resource-capability surface; one manifest, target-kind-discriminated fields.

## Foundation references

- `docs/ARCHITECTURE.md` — adapter registration/lifecycle, capability manifests
- `docs/UX.md:40-62` — the presentation conformance floor + adapter-shaped projections above it (the projection contract to mechanize)
- `docs/PROTOCOL.md:553-593` — capability declarations, snapshot tiers
- `contracts/proto/patchbay/adapter.proto:9-34` — current manifest + `snapshot_support` tiers
- `contracts/proto/patchbay/adapter_control.proto` — typed report ingress paths

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`
- No direct UI; the contract the cockpit feature composes against.

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will fill in interfaces, signatures, and implementation units. -->
