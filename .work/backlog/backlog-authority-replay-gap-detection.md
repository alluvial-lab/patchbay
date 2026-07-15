---
id: backlog-authority-replay-gap-detection
kind: feature
stage: backlog
tags: [verification, protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Backlog: Authority replay must detect gapped / Unspecified-kind event sequences

## Source
Deep review of `feature-v0-core-authority` (Phase 2 adversarial).

## Finding
`rebuild_from_log` (`core/src/authority/replay.rs`) checks `event_lsn > previous_lsn` (strictly increasing) but does NOT require `event_lsn == previous_lsn + 1` (gap-free). A storage layer that omits an event (returns LSNs 1, 3 with 2 missing) is accepted, which could resurrect a grant by skipping its revocation. `PROTOCOL.md` defines LSNs as gap-free.

Additionally, `StoredEventKind::Unspecified` is silently ignored by `AuthorityRegistry::observe` (treated as a no-op like Operation/Elicitation/etc.), rather than rejected as `CorruptLog`. A corrupted event whose kind byte becomes 0 is silently dropped.

The replay tests cover live-equivalence and cross-domain rejection but NOT out-of-order/gapped LSNs or Unspecified-kind rejection.

## Direction
- Decide gap policy: require `event_lsn == previous_lsn + 1` (strict gap-free) OR keep `>` but document that storage guarantees gap-free delivery. Note the sessions/acceptance replay use the same `>` check — this is a cross-cutting replay-discipline decision, not authority-specific. Filing here for the authority surface.
- `observe` should reject `StoredEventKind::Unspecified` as `CorruptLog` (Fail Fast) rather than no-op, OR document that storage never emits Unspecified. The sessions/elicitation registries also no-op on Unspecified — cross-cutting.
- Add tests: gapped LSN sequence, Unspecified-kind event.

## Priority
Latent — depends on storage layer behavior (rusqlite returns gap-free, monotonic LSNs by construction). The gap check is defense-in-depth against a storage adapter that doesn't guarantee gap-free. Not blocking v0.1.0 (rusqlite is sound); resolve when a second storage backend is considered, or as hardening.
