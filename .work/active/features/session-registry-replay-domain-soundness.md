---
id: session-registry-replay-domain-soundness
kind: feature
stage: drafting
tags: [protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Session registry/replay/domain soundness

## Brief
Close the registry/replay/domain soundness gaps split out of `sessions-soundness-coverage` (currency verified 2026-08-09). Absorbs:

- `backlog-sessions-authority-domain-isolation` — **OPEN**: `SessionRegistry` has no owning domain (`registry.rs:50-55`); `TargetResolver::resolve` ignores `_authority_domain_id` (`resolver.rs:15-24`); `current_session` takes no domain arg (`ingest.rs:88-95`).
- `backlog-sessions-idempotency-and-concurrency` — **PARTIAL** (replay-equality half): production adapter ingress now serializes through the shared decision gate + rebuilds before/after report ingest (`adapter_service.rs:753-829`); but registry redelivery is content-blind (dup returns `Ok` by key, `registry.rs:329-334`) and state mutations no-op on any `event_lsn <= last_lsn`. Production serialization ≠ replay equality.
- `backlog-sessions-test-coverage-gaps` — **PARTIAL**: resolver enforces `RuntimeSession` kind + some malformed cases exist, but replay tests stay happy-path, acceptance uses `TestTargetResolver`, and the proptest fixes all reports to one adapter/session.

## Direction
Bind each `SessionRegistry` to an `AuthorityDomainId` at construction; validate on lookup/ingest; reject cross-domain (forward-compat for the `(authority_domain_id, LSN)` federation seam). For replay equality: compare event identity/payload on redelivery, not just key+LSN; either serialize the warm read-decide-append or make it append-then-replay. Coverage: the acceptance↔sessions integration test (highest value), table-driven malformed-event tests, and a multi-identity proptest (per-identity monotonicity, tombstone retention, no cross-session interference). **Production decision-gate serialization is a composition-root invariant tested independently — do not advertise it as core writer safety** (a future composition root can bypass the server gate; Fail Fast).

## Foundation references
- `docs/PROTOCOL.md` — authority-domain-scoped target resolution; `(authority_domain_id, LSN)` extension seam
- Code: `core/src/session/registry.rs`, `core/src/session/resolver.rs`, `core/src/session/ingest.rs`, `server/src/adapter_service.rs`
