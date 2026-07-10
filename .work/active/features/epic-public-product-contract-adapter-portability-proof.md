---
id: epic-public-product-contract-adapter-portability-proof
kind: feature
stage: drafting
tags: [adapter, protocol]
parent: epic-public-product-contract
depends_on: [epic-public-product-contract-public-compatibility]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Adapter portability proof

## Brief

Prove that Patchbay's public adapter boundary is not merely Pi-shaped. A third party must be able to understand and implement the designated adapter contract, and Pi plus one materially different open-source adapter—or, if no suitable candidate survives evaluation, a materially distinct conformance reference adapter—must exercise the boundary before `v1.0.0`.

This feature consumes the canonical adapter capability manifest, registration lifecycle, generated contracts, Pi mapping, and public compatibility designation already established elsewhere. It selects the proof target only after refreshing its license, version, API stability, spawn/attach semantics, and redistribution facts; existing OpenCode research makes it a strong candidate but not a pre-decided commitment. The deliverable includes executable adapter conformance evidence and must preserve the ability for providers and adopters to build proprietary adapters under the legally reviewed interoperability boundary. It does not make second-adapter-specific concepts part of the core ontology.

## Epic context

- Parent epic: `epic-public-product-contract`
- Position in epic: consumer/proof of `epic-public-product-contract-public-compatibility`; executable release assurance later consumes its conformance evidence.
- Pi remains the first migration adapter; this feature proves portability rather than replacing the Pi path.

## Foundation references

- `docs/SPEC.md` — v1 adapter proof; adapter posture
- `docs/ARCHITECTURE.md` — adapter plane and lifecycle
- `docs/PROTOCOL.md` — adapter capability manifest and failure behavior
- `docs/ADAPTER-PI.md` — first-adapter mapping and parity floor
- `.research/analysis/campaigns/harness-action-surfaces/parent.md` — existing candidate survey
- `.research/analysis/campaigns/harness-action-surfaces/specialists/opencode.md` — candidate evidence requiring refresh
