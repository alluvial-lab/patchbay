---
id: backlog-authority-grant-selection-determinism
kind: feature
stage: backlog
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Backlog: Overlapping matching grants produce nondeterministic authorization provenance

## Source
Deep review of `feature-v0-core-authority` (Phase 1 + Phase 2, both reviewers).

## Finding
`impl GrantCheck for AuthorityRegistry` (`core/src/authority/check.rs`) iterates `live_grants()` (a `HashMap`) and returns the first `grant_authorizes` match as `Authorized { grant_id }`. When two live grants both match (e.g. a FleetSupervisor grant and an Adapter grant for the same actor), the returned `grant_id` is HashMap-iteration-order-dependent — not stable across process restarts or replay rebuilds. This affects future `spawning_grant_id` provenance and which revocation policy applies.

Revision 3 does not pin a grant-selection rule for overlapping matches. This is an unaddressed semantic choice, not a mechanical implementation detail.

## Direction
Make an explicit semantic decision: e.g. most-specific-scope-first (RuntimeSession > Adapter > FleetSupervisor), or first-by-stable-order (sort by grant_id), or reject ambiguity (two matching grants of different specificity is a configuration error). Then enforce deterministic ordering in `check` and add a test that overlapping grants return a stable `grant_id` before and after replay.

## Priority
Latent in v0.1.0 single-operator (the operator typically has one grant). Becomes real with multiple/narrower grants or delegation. Not blocking component-complete; resolve before live path or delegation work.
