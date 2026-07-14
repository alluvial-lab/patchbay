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

# Story: Grant + revocation ingestion (the writer)

## Scope
Implement Unit 3 of `feature-v0-core-authority`: the direct ingestion writer for grants, descendant grants, and revocations. The analog of sessions' `ingest_session_report` / acceptance's `ingest_observation`. Owns its event kinds end-to-end (writer pattern, Q2).

## Units
- `core/src/authority/ingest.rs` — `GrantLookup` trait, `ingest_grant`, `ingest_descendant_grant`, `ingest_revocation`

## Implementation
See `feature-v0-core-authority.md` Unit 3 for exact signatures. Writer pattern mirroring `ingest_session_report` / `ingest_observation`: validate → read current → write delta event → warm registry → return.

Key points:
- `ingest_descendant_grant` validates the allowed-kind set matches `DESCENDANT_GRANT_ALLOWED_KINDS` exactly (reject if spawn/attach included or a required kind missing — Fail Fast, Q5).
- `ingest_revocation` is the two-lever non-cascade enforcement point: revokes ONLY the named grant. The registry fold marks that one grant revoked; no other. There is NO cascade code path (non-cascade is structural).
- Warm-after-write mirrors sessions' post-B5 pattern (warm after each successful append so retry is idempotent).
- Encoding: `grant.encode_to_vec()` under `StoredEventPayload { kind: StoredEventKind::Grant as i32, payload }`.
- Read `core/src/session/ingest.rs` FIRST — the direct template for the writer pattern + warm-after-write.

## Acceptance Criteria
- [ ] `ingest_grant` writes a Grant event; registry reflects it
- [ ] `ingest_descendant_grant` rejects a descendant with the wrong allowed-kind set
- [ ] `ingest_revocation` writes a Revocation event; marks ONLY the named grant revoked
- [ ] `ingest_revocation` does NOT revoke descendant grants under the revoked grant (non-cascade, two-lever)
- [ ] Revoking a non-existent grant returns an error (Fail Fast)
- [ ] Warm-after-write keeps the registry consistent (retry-safe)

## Notes
- Depends on story 1 (registry + `DESCENDANT_GRANT_ALLOWED_KINDS`).
- Add integration tests in `core/tests/authority_ingest.rs`. The non-cascade test is the key one.
- Do NOT implement the spawn-tail or replay here.
