---
id: deployment-authority-workspace-scoped-revocable-keys
kind: story
stage: drafting
tags: [security, architecture]
parent: null
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Deployment-authority: workspace-scoped expiring/revocable keys

## Origin (research-grounded)
Decomposed from `mc-architectural-harvest` direction **5**.
- **Source campaign:** `.research/analysis/campaigns/v1-control-plane-and-spawn/`.
- **Harvest item:** `.work/active/stories/mc-architectural-harvest.md` (direction 5).

## Direction
Borrow MC's fail-closed workspace boundaries as a **deployment-authority layer**: agent/workspace-bound expiring + revocable keys + strict-workspace denial. Keep it distinct from fine-grained operation authority — MC's lesson is "don't mistake role derivation for fine-grained operation authority"; Patchbay's operation authority stays grant-based (v0.2.0), and this adds a deployment-scoped workspace-key layer on top.

## Scope / dependencies
Builds on the v0.2.0 revocation + lockdown arc (collapsed at v0.2.0). **Home stride is not yet fixed** — workspace-key scoping is relevant to the spawn stride (agent sessions/workspaces) and to `epic-public-product-contract-public-compatibility` (public-facing deployment). Leave `parent: null` + `depends_on: []` until the owning stride is chosen; re-parent when spawn or ppc design clarifies where workspace-key authority lives.

## Note
Design-bearing (authority layer) → `feature-design` when picked up; do not implement as a standalone patch outside its stride's authority model.
