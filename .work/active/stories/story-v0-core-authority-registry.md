---
id: story-v0-core-authority-registry
kind: story
stage: implementing
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Grant/revocation event model and AuthorityRegistry projection

## Scope
Implement Unit 1 of `feature-v0-core-authority`: the in-memory grant record, the grant-matching predicate, the canonical descendant allowed-kind set, and the `AuthorityRegistry` projection that folds Grant/DescendantGrant/Revocation events. Mirrors `SessionRegistry`/`ElicitationSlotLayer`.

## Units
- `core/src/authority/mod.rs` — module root, `AuthorityError` enum, re-exports
- `core/src/authority/state.rs` — `GrantRecord`, `grant_authorizes`, `DESCENDANT_GRANT_ALLOWED_KINDS`
- `core/src/authority/events.rs` — encoding helpers (Grant/DescendantGrant/Revocation → StoredEventPayload)
- `core/src/authority/registry.rs` — `AuthorityRegistry` projection (observe fold)
- `core/src/lib.rs` — add `pub mod authority;`

## Implementation
See `feature-v0-core-authority.md` Unit 1 for exact signatures. Key points:
- `GrantRecord` projects BOTH `Grant` and `DescendantGrant` proto messages (`is_descendant` flag distinguishes; provenance differs).
- `DESCENDANT_GRANT_ALLOWED_KINDS` is the SSOT — copied verbatim from PROTOCOL.md "Spawn payload and authority commitments" (8 kinds: instruct, cancel, interrupt, query, approval-response, elicitation-response, reconfigure, session-management; spawn+attach excluded).
- `grant_authorizes` is deny-by-default: live + subject matches + kind allowed + target in scope.
- `observe` validates grant shape (Fail Fast) and is idempotent for re-delivered events. `observe_revocation` marks revoked (not delete) — audit retention.
- Read `core/src/session/registry.rs` and `core/src/acceptance/elicitation.rs` FIRST — they're the direct templates.

## Acceptance Criteria
- [ ] `observe` folds Grant, DescendantGrant, Revocation events correctly
- [ ] Revocation marks the grant revoked (not deleted); `is_live()` returns false after
- [ ] `grant_authorizes` returns true only when live + subject matches + kind allowed + target in scope
- [ ] `DESCENDANT_GRANT_ALLOWED_KINDS` matches PROTOCOL.md exactly (8 kinds, spawn+attach excluded)
- [ ] `observe` rejects malformed grants as `CorruptRecord`
- [ ] `observe` is idempotent for re-delivered events

## Notes
- No deps. Foundation the other 5 stories build on.
- `AuthorityError` enum in `mod.rs` (all submodules use it): `CorruptRecord`, `CorruptLog`, `InvalidGrant`, `GrantNotFound`, `Storage(#[from])`.
- Do NOT implement GrantCheck, ingest, spawn-tail, or replay here — only the registry + state + error type + encoding helpers.
