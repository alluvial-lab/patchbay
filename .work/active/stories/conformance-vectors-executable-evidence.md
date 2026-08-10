---
id: conformance-vectors-executable-evidence
kind: story
stage: drafting
tags: [verification, v1]
parent: epic-public-product-contract-executable-release-assurance
depends_on: [research-handoff-spawn, research-handoff-pi-adapter-capability]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-08
updated: 2026-08-08
---

# Conformance vectors — non-orphaning acceptance + incarnation fence + spawn lifecycle

Executable evidence child of `epic-public-product-contract-executable-release-assurance`.

Promote the campaign's consolidated conformance vectors into executable evidence. These are the moat-validating + spawn-lifecycle vectors — the peer-derived failure shapes no fetched peer closes, plus the spawn lifecycle contract vectors. Feed `epic-public-product-contract-executable-release-assurance` and `epic-public-product-contract-adapter-portability-proof`.

**Vector groups (full table in the facet briefs):**
- Non-orphaning acceptance (no peer closes): `effect-before-response-loss`, `ack-before-dispatch-crash`, `accepted-without-external-run-id`, `offline-intent-process-crash`.
- Incarnation fence (no peer has): `cursor-gap-repair`, `version-is-not-generation`, `stale-worker-same-logical-id`.
- Spawn lifecycle: `spawn-continuation`, `detach-does-not-retire`, `crash-before-ack`, `restart-native-resume`, `restart-shape-only`, `reconnect-after-stream-loss`, `duplicate-continuation`, `stale-generation-event`, `equal/lower-generation-report`, `duplicate-native-reference`, `project-cwd-boundary`.
- Boundary honesty: `bounded-dedup-expiry`, `superseded-offline-operations`, `manifest-overclaim`, `authority-cross-resource`, `dedup-store-unavailable`.

## Research grounding

**Source**: `.research/analysis/campaigns/v1-control-plane-and-spawn/parent.md` (slug: `v1-control-plane-and-spawn`) — facets `peer-protocol-deep-dive` (peer-derived failure shapes) + `spawn-lifecycle` (lifecycle vectors).

Each vector is grounded in a concrete peer failure mode (amux's expiring dedup; CodeAgent's ack-before-dispatch; Mission Control's accepted-without-run-id; Happy's memory-only outbox) — they prove the Patchbay composition claim by showing the peer failure each Patchabay invariant prevents.
