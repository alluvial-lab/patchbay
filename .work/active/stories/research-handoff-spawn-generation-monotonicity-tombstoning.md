---
id: research-handoff-spawn-generation-monotonicity-tombstoning
kind: story
stage: implementing
tags: [protocol, verification, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-registration]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Atomic promotion fold, exact generations, and tombstones

## Redesign disposition

Rewritten. The old adapter report-driven generation advance is superseded. Only `SpawnPromotionCommitted` can publish a managed generation.

## Checkpoint

Implement each projection's fail-closed fold of the single promotion event. For fresh spawn, validate `∅→1`. For continuation, validate exact current `N→N+1`, exact accepted/staged claim, external-runtime reservation, and exact prior. The same event tombstones N, installs N+1 current, consumes the claim/fence, installs descendant authority, and completes the Operation according to projection ownership.

There is no state in which N+1 is current without descendant authority. Staged candidates remain non-live. Runtime session id may change; logical target remains stable.

## Design

**Files**
- `core/src/session/{events,registry,logical_target,spawn_claim,replay}.rs` — session/claim promotion fold.
- `core/src/authority/{events,projection,replay}.rs` — descendant authority fold.
- `core/src/acceptance/{index,replay,transitions}.rs` — completed command fold.
- `server/src/{checkpoint,snapshot}.rs` — promotion-aware recovery state.
- `specs/seed/session_generation.qnt` plus conformance/property tests.

Each projection independently validates the promotion event against its pre-state. Unknown/malformed event shape, missing claim/stage/Grant, wrong transition, duplicate runtime ownership, or conflicting replay fails before any installed mutation.

## Acceptance evidence

- [ ] Managed current state changes only through promotion, never a raw/staged report.
- [ ] Fresh accepts only generation 1; continuation accepts only exact N+1 with changing runtime id allowed.
- [ ] N tombstone and N+1 current/authority/completion become observable only from one complete promotion event.
- [ ] Promotion permanently consumes the claim and replacement fence while retaining exact provenance/tombstone/reverse-index history.
- [ ] Equal/lower/unclaimed-greater evidence cannot promote.
- [ ] Replay/checkpoint produce the exact same logical targets, claims, Grant, command state, and tombstones.
- [ ] Formal/model and executable mutations catch missing strict advance, exact pre-state, uniqueness, or authority-bearing atomicity.

## Ordering constraint

Consumes staged claimed-successor evidence. The stale fence implements every ingress against this finalized projection before completion is enabled.
