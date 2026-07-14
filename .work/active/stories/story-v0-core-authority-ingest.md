---
id: story-v0-core-authority-ingest
kind: story
stage: implementing
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Grant + revocation ingestion (writer) + bootstrap operator grant

## Scope
Implement Unit 3 of `feature-v0-core-authority` (revision 2): the direct ingestion writer for grants, descendant grants, and revocations + the bootstrap operator grant creation at init. Addresses review blockers #1 (bootstrap grant makes operator authority durable), #3 (descendant allowed-kind validation), #7 (`&mut L` warm-after-write).

## Units
- `core/src/authority/projection.rs` — `GrantProjection` trait (`GrantLookup` + `observe(&mut self, ...)`)
- `core/src/authority/ingest.rs` — `ingest_grant`, `ingest_descendant_grant`, `ingest_revocation`, `ensure_bootstrap_operator_grant`

## Implementation
See `feature-v0-core-authority.md` Unit 3 for exact signatures. Key points:
- `GrantProjection` takes `&mut L` with `observe(&mut self, ...)` — review blocker #7 (warm-after-write, mirrors sessions' post-B5 `SessionProjection`). Warm after each successful append so retry is idempotent.
- `ingest_descendant_grant` validates allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly (Fail Fast).
- `ingest_revocation` — two-lever non-cascade: revokes ONLY the named grant. No cascade code path (structural). Revoking non-existent → error.
- `ensure_bootstrap_operator_grant` (R1) — creates the durable operator grant at init: fleet-scope spawn grant + universal existing-session grant, subject = operator actor. Idempotent (checks existence first). This is what `GrantCheck` evaluates the operator against (no implicit bypass).
- Writer pattern mirroring `ingest_session_report`: validate → read current → write delta event → warm projection → return.
- Read `core/src/session/ingest.rs` FIRST — direct template (warm-after-write + retry-safety).

## Acceptance Criteria
- [ ] `ingest_grant` writes a Grant event; projection reflects it
- [ ] `ingest_descendant_grant` rejects a descendant with the wrong allowed-kind set
- [ ] `ingest_revocation` marks ONLY the named grant revoked (non-cascade, two-lever)
- [ ] `ingest_revocation` does NOT revoke descendant grants under the revoked grant
- [ ] Revoking a non-existent grant returns an error (Fail Fast)
- [ ] Warm-after-write keeps the projection consistent (retry-safe)
- [ ] `ensure_bootstrap_operator_grant` creates the operator grant; idempotent on re-call

## Notes
- Depends on story 1 (registry + `DESCENDANT_GRANT_ALLOWED_KINDS`).
- Add integration tests in `core/tests/authority_ingest.rs`. The non-cascade test is key.
- The bootstrap grant is the R1a fix — it makes operator authority durable + revocable (no implicit bypass).
