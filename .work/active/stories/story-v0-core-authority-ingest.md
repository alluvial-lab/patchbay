---
id: story-v0-core-authority-ingest
kind: story
stage: done
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Grant + revocation ingestion (writer)

## Scope
Implement Unit 3 of `feature-v0-core-authority` (revision 3): the direct ingestion writer for grants, descendant grants, and revocations. Owns its event kinds end-to-end (writer pattern). **No bootstrap grant** (R1 dropped it — tests inject grants directly).

## Units
- `core/src/authority/projection.rs` — `GrantProjection` trait (`GrantLookup` + `observe(&mut self, ...)`)
- `core/src/authority/ingest.rs` — `ingest_grant`, `ingest_descendant_grant`, `ingest_revocation`

## Implementation
See `feature-v0-core-authority.md` Unit 3 for exact signatures. Key points:
- `GrantProjection` takes `&mut L` with `observe(&mut self, ...)` (rev2 finding #7 — warm-after-write, mirrors sessions' post-B5 `SessionProjection`). Warm after each successful append so retry is idempotent.
- `ingest_descendant_grant` validates allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly (Fail Fast).
- `ingest_revocation` — two-lever non-cascade: revokes ONLY the named grant. No cascade code path (structural). Revoking non-existent → error.
- **No `ensure_bootstrap_operator_grant`** (R1 dropped it). Tests inject grants directly via `ingest_grant`.
- Writer pattern mirroring `ingest_session_report`: validate → read current → write delta event → warm projection → return.
- Read `core/src/session/ingest.rs` FIRST — direct template (warm-after-write + retry-safety).

## Acceptance Criteria
- [ ] `ingest_grant` writes a Grant event; projection reflects it
- [ ] `ingest_descendant_grant` rejects a descendant with the wrong allowed-kind set
- [ ] `ingest_revocation` marks ONLY the named grant revoked (non-cascade, two-lever)
- [ ] `ingest_revocation` does NOT revoke descendant grants under the revoked grant
- [ ] Revoking a non-existent grant returns an error (Fail Fast)
- [ ] Warm-after-write keeps the projection consistent (retry-safe)

## Notes
- Depends on story 1 (registry + `DESCENDANT_GRANT_ALLOWED_KINDS`).
- Add integration tests in `core/tests/authority_ingest.rs`. The non-cascade test is key.

## Implementation notes
- Files changed: `core/src/authority/projection.rs`, `core/src/authority/ingest.rs`, `core/src/authority/mod.rs`, `core/tests/authority_ingest.rs`.
- Tests added: grant append+warm, descendant exact-kind rejection and success, named-grant-only revocation, nonexistent-grant rejection, and committed-event redelivery consistency (6 tests).
- Discrepancies from design: none. Creation payloads are preflight-folded through a scratch `AuthorityRegistry` before append so registry shape validation remains the single implementation path; ingestion also validates caller-supplied domain/id/scope before event helpers normalize the wire message.
- Verification: `cargo build -p patchbay-core` and the focused `authority_ingest` suite passed.
- Adjacent issues parked: none.
