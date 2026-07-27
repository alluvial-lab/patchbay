---
id: epic-public-product-contract-adapter-portability-proof
kind: feature
stage: drafting
tags: [adapter, protocol]
parent: epic-public-product-contract
depends_on: [epic-public-product-contract-public-compatibility, epic-token-commune-control-attention]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-26
---

# Adapter portability proof

## Brief

Prove that Patchbay's public adapter boundary is not merely Pi-shaped. A third party must be able to understand and implement the designated adapter contract, and Pi plus token-commune must exercise materially different runtime-session and operational-resource shapes before `v1.0.0`.

This feature consumes the canonical adapter capability manifest, registration lifecycle, generated contracts, Pi mapping, public compatibility designation, and the completed token-commune observer/control arc. The deliverable includes executable conformance evidence across attachment, identity, snapshots, query/Observation flow, durable mutations, authority, retry semantics, attention, and adapter-shaped presentation. It must preserve the ability for providers and adopters to build proprietary adapters under the legally reviewed interoperability boundary, and it must demonstrate that neither Pi nor token-commune concepts entered the core ontology.

## Epic context

- Parent epic: `epic-public-product-contract`
- Position in epic: consumer/proof of `epic-public-product-contract-public-compatibility` and the token-commune integration arc; executable release assurance later consumes its conformance evidence.
- Pi remains the first session/migration adapter; token-commune is the selected operational-resource reference adapter. This feature proves the public boundary across both shapes rather than replacing either path.

## Foundation references

- `docs/SPEC.md` — v1 adapter proof; adapter posture
- `docs/ARCHITECTURE.md` — adapter plane and lifecycle
- `docs/PROTOCOL.md` — adapter capability manifest and failure behavior
- `docs/ADAPTER-PI.md` — first-adapter mapping and parity floor
- `.work/active/epics/epic-agent-operations-resource-plane.md` — resource target and projection foundation
- `.work/active/epics/epic-token-commune-observer.md` — read-only reference adapter
- `.work/active/epics/epic-token-commune-control-attention.md` — durable control and attention reference path

## Strategic decision update

- **Reference target selection:** token-commune replaces the earlier open candidate search. Its independent gateway, materially non-session resource semantics, real consumer community, metadata-only boundary, and cross-repository contract pressure make it the chosen v1 portability proof. Existing OpenCode research remains useful background for future harness adapters but no longer controls this feature.
