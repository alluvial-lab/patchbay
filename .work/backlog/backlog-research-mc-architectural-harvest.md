---
id: backlog-research-mc-architectural-harvest
kind: story
stage: backlog
tags: [adapter, architecture]
parent: null
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-08
updated: 2026-08-08
---

# Mission Control architectural harvest — adapter-manifest design directions

Harvest Mission Control's adapter-neutral structural ideas as **design direction** (MIT, but inspiration not code reuse) for Patchbay's adapter-manifest + control-plane design. MC lacks the durable-operation contract (it's task/run governance + approvals), but its shipped structure is worth borrowing.

**Directions to harvest:**
- **Declared capability depth, separate from runtime detection** — keep "is installed/reachable" separate from "what this adapter can honestly guarantee"; require a complete manifest; default uncertain fields false. Extend with durability dimensions: dedup strength, continuation proof, cursor support, generation-fence support.
- **Durable run/task provenance beside adapter-native identifiers** — useful separation of task relation, runtime/session identity, lineage, status, outcome, cost, evidence — but make the accepted Operation the source of truth, not a diagnostic record.
- **Atomic claim before delivery** — MC's compare-and-swap task claim prevents two scheduler workers concurrently dispatching one task; reuse + strengthen with caller idempotency + target generation.
- **Represent reconciliation capability honestly** — MC marks accepted-without-run-id work as manual-reconciliation-required; Patchbay adapters should declare reconciliation strength + return `unknown`/manual-required when the substrate can't prove an outcome.
- **Fail-closed workspace boundaries** — agent/workspace-bound expiring + revocable keys + strict-workspace denial as a deployment authority layer (but don't mistake role derivation for fine-grained operation authority).

## Research grounding

**Source**: `.research/analysis/campaigns/v1-control-plane-and-spawn/parent.md` (slug: `v1-control-plane-and-spawn`) — facet `peer-protocol-deep-dive` + attestation `mission-control-src`.

The directions are source-grounded in MC's actual code (cloned, MIT). Each is framed as a Patchbay extension (`{extends}`) that strengthens the borrowed idea with the operation contract MC lacks.
