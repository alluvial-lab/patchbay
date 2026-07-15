---
id: backlog-authority-ingest-pre-append-conflict-check
kind: feature
stage: backlog
tags: [security, protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Backlog: Authority ingest must check for conflicts BEFORE appending, not after

## Source
Deep review of `feature-v0-core-authority` (Phase 1 + Phase 2, both reviewers).

## Finding
`ingest_grant` / `ingest_descendant_grant` / `ingest_revocation` (`core/src/authority/ingest.rs`) validate the grant, then `storage.append(...)`, then `projection.observe(...)` to warm. The conflict check happens in `observe` (the projection fold), which runs AFTER the durable append. So:
- Re-ingesting the same grant_id with DIFFERENT content: the conflicting event is appended to the durable log FIRST, then `observe` rejects it as `CorruptLog` — but the log is already poisoned. Every later `rebuild_from_log` fails.
- Retrying an identical successful ingestion: appends a SECOND lifecycle event instead of returning the original event id (no durable idempotency at the writer — only the fold's in-memory `issued` set is idempotent, and that's the spawn-tail, not ingest).

This contradicts the "warm-after-write, retry-safe" claim in the feature design (Unit 3). The sessions feature's `ingest_session_report` has the same shape but is protected by `append_dedup` for commands; authority grants have no such dedup key.

## Direction
- Pre-append: read the current projected grant via `GrantProjection::current_grant`; if it exists with identical content → return the existing event id (no-op); if it exists with different content → reject before append (`CorruptLog`); only append if absent.
- Durable idempotency for descendant grants: the deterministic `descendant_grant_id` is already computed; use it as the dedup key (or check existence before append).
- This needs a storage-level atomic check-and-append (like `append_dedup`) OR a serialized authority writer. The current `Storage::append` is not atomic with the projection read.

## Priority
The conflict-before-append gap is a real durability hazard (poisons the log). However, v0.1.0 authority is single-writer, test-driven (grants are injected once), with no live path — the window is not exercised. Blocking for the live path; latent now. The identical-retry gap is also latent (no retries in tests). Resolve together with `backlog-authority-live-composition` (which is where a real writer coordination layer lands).

## Note
The existing `authority_ingest.rs` test "warm-after-write" re-observes an already-committed event to the projection; it does NOT retry the writer (`ingest_grant` twice). The test gives false confidence about retry-safety.
