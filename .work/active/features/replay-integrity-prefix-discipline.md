---
id: replay-integrity-prefix-discipline
kind: feature
stage: drafting
tags: [verification, protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Replay integrity: gap-free LSN + reject Unspecified (cross-projection)

## Brief
Close the replay-integrity gap split out of `authority-provenance-hardening`. Absorbs:

- `backlog-authority-replay-gap-detection` — **OPEN**: authority replay checks `event_lsn <= previous_lsn`, not `== previous_lsn + 1` (gap-free), and `StoredEventKind::Unspecified` is silently ignored (`registry.rs:59-67`); a gapped sequence could resurrect a revoked grant. Contradicts the gap-free LSN contract (`PROTOCOL.md:444-448`). *Src:* authority review Phase 2.

## Direction
This is **cross-cutting, not authority-only** — sessions and acceptance replay share the same `<=` check and the `Unspecified` no-op. Define a shared contiguous-prefix + gap-free replay discipline across authority/session/resource projections: require `event_lsn == previous_lsn + 1` (or document that storage guarantees gap-free delivery) and reject `Unspecified` as `CorruptLog` (Fail Fast). Add tests: gapped LSN sequence, Unspecified-kind event. Couples with `resource-reconciliation-followups` (its applied-prefix cursor is the resource-plane instance of this same invariant) and the sessions replay-equality work.

## Foundation references
- `docs/PROTOCOL.md` — gap-free LSN contract (`:444-448`); event-kind registry
- Code: `core/src/authority/replay.rs`, `core/src/authority/registry.rs`, sibling replay paths (sessions/acceptance)
