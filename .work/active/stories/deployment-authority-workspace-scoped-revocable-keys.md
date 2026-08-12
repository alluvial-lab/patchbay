---
id: deployment-authority-workspace-scoped-revocable-keys
kind: story
stage: drafting
tags: [security, architecture]
parent: research-handoff-spawn
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
Builds on the v0.2.0 revocation + lockdown arc (collapsed at v0.2.0). **Tentatively parented to `research-handoff-spawn`** (2026-08-12): agent/workspace-key scoping is spawn-adjacent (spawn creates agent sessions/workspaces). Flip to `epic-public-product-contract` or a dedicated deployment-authority stride if that design claims workspace-key authority instead. Leave `depends_on: []` until the authority-scope decision firms up the ordering.

## Note
Design-bearing (authority layer) → `feature-design` when picked up; do not implement as a standalone patch outside its stride's authority model.
