---
id: authority-writer-correctness
kind: feature
stage: drafting
tags: [security, foundation]
parent: null
depends_on: [authority-descendant-grant-completion]
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Authority writer correctness (pre-append conflict check + durable idempotency)

## Brief
Close the authority-ingest durability hazard split out of `authority-provenance-hardening`. Absorbs:

- `backlog-authority-ingest-pre-append-conflict-check` — **OPEN** (highest durability hazard in the set): authority ingest appends *before* the conflict check (which runs in `observe`), so a conflicting re-ingest poisons the durable log and an identical retry appends a second event. `current_grant` only chooses audit kind (`ingest.rs:39-48`); append-before-observe (`ingest.rs:179-187`); descendant grants share the non-dedup path (`ingest.rs:75-78`). *Src:* authority review Phase 1+2.

## Direction
Pre-append check-and-append: read the current projected grant → identical content returns the existing event id (no-op) → different content rejects before append (`CorruptLog`) → only append if absent. Use the deterministic `descendant_grant_id` as the dedup key. This needs a storage-level atomic check-and-append (like `append_dedup`) or a serialized authority writer — `Storage::append` is not atomic with the projection read. The existing "warm-after-write" test does NOT retry the writer (false confidence); add a writer-retry regression. Resolve together with `authority-descendant-grant-completion` (where the live writer-coordination layer lands).

## Foundation references
- `docs/PROTOCOL.md` — durable log integrity; authority lifecycle
- Code: `core/src/authority/ingest.rs`, `core/src/storage/` (append/append_dedup)
