---
id: authority-grant-selection-determinism
kind: feature
stage: drafting
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Authority grant-selection determinism (stable rule + regression)

## Brief
Close the grant-selection determinism gap split out of `authority-provenance-hardening`. Absorbs:

- `backlog-authority-grant-selection-determinism` — **PARTIAL**: overlapping matching grants need a stable selection rule so the returned `grant_id` (and downstream `spawning_grant_id` provenance + revocation policy) is replay-stable. Candidates are now sorted by `grant_id` before selection (`core/src/authority/check.rs:47-58`), giving a stable rule; **but no overlapping-grants before/after-replay regression exists**. *Src:* authority review Phase 1+2.

## Direction
Ratify the selection rule explicitly (most-specific-scope-first / sort-by-`grant_id` / reject-ambiguity — the current sort-by-`grant_id` is the implemented candidate), document it, and add the missing regression: overlapping matching grants return a stable `grant_id` before and after replay rebuild. Latent in v0.1.0 single-operator; becomes real with multiple/narrower grants or delegation.

## Foundation references
- `docs/PROTOCOL.md` — grant lifecycle; provenance
- Code: `core/src/authority/check.rs` (`GrantCheck` selection)
