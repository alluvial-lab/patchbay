---
id: research-handoff-spawn-reconnect-cursor-reconcile
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-restart-continuation-orchestration, research-handoff-spawn-cursor-authoritative-replacement-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Reconnect and authoritative cursor convergence

## Redesign disposition

Rewritten. Unknown-cursor recovery cannot upsert a full fetch into an old projection or key external continuity solely by Patchbay generation.

## Checkpoint

Prove convergence across three distinct authorities: core `(authority_domain_id, LSN)` lifecycle replay; adapter external persisted-state cursor scoped by verified external continuity identity; and surface snapshot/cursor reconciliation. A remembered stream/process handle/wall clock proves none of them.

A known external cursor applies a suffix. Unknown cursor stages a complete exact-set/tree projection, validates it, and atomically replaces projection + leaf + cursor + epoch. Stale omitted entries disappear. The replacement remains stale/unknown until commit and current process evidence; cursor installation cannot precede projection replacement.

## Design

**Files**
- `core/src/session/{registry,replay}.rs`, `server/src/{checkpoint,snapshot}.rs` — replay promotion, claims, quarantine, tombstones, and descendant authority.
- Adapter-neutral cursor replacement consumers; Pi storage/reconciler remains downstream.
- `web-cockpit/src/domain/{reconcile,model}.ts` — replace cached logical/current state only from newer core authority.
- Cross-layer vectors/runners.

Core replay and external cursor replay remain distinct; neither cursor translates into the other. Endpoint detach/reconnect does not change runtime generation. Adapter reconnect reauthenticates attachment generation before reporting current evidence.

## Acceptance evidence

- [ ] Missing N→N+1 stream events reconcile to one logical target with N tombstoned and N+1 current/authorized only after promotion.
- [ ] A poisoned/staged candidate remains non-live after restart/reconnect.
- [ ] Unknown external cursor exact replacement removes stale omitted entries and atomically installs cursor/leaf/epoch.
- [ ] External cursor scope survives Patchbay generation when verified native continuity is the same; cross-native-session reuse rejects.
- [ ] Core replay reconstructs claims/fences/quarantine/promotion/authority deterministically.
- [ ] Cached N, remembered live streams, or stale upsert entries cannot overwrite repaired authority.
- [ ] Detach-does-not-retire, reconnect-after-stream-loss, cursor-gap-repair, and upsert-only mutations pass/fail as expected.

## Ordering constraint

Final spawn-side checkpoint after restart orchestration and the early cursor contract. The Pi redesign implements the reference external-cursor port.
