---
id: story-fix-sessions-multi-delta-atomicity
kind: story
stage: implementing
tags: [protocol, bug, verification, foundation]
parent: feature-v0-core-sessions
depends_on: [story-fix-sessions-ingest-correctness]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Story: Fix sessions multi-delta append atomicity (B5, regression from B3 fix)

## Scope

A new correctness blocker introduced by the B3 fix (`story-fix-sessions-ingest-correctness`, commit `06e6251`). The B3 fix correctly removed the early-return truncation and made the equal-generation branch append all changed deltas sequentially — but the sequential appends are non-atomic, and a mid-sequence failure leaves a half-applied log that makes retry produce an unreplayable duplicate.

## Blocker

### B5 — Partial multi-delta failure makes retry produce an unreplayable log

The equal-generation branch in `ingest_session_report` appends connectivity, activity, and metadata deltas sequentially (each `storage.append(...).await?`). The `?` propagates errors immediately. If the connectivity append succeeds but the activity append fails:
1. The connectivity event is durable (committed).
2. The function returns `Err` (the activity storage error).
3. The in-memory registry is NOT warmed (the warm path only runs after success).
4. The caller retries the SAME report against the unchanged registry.
5. Retry re-derives the connectivity delta (still changed) and appends it AGAIN.
6. Replay now sees two `Unknown → Live` connectivity deltas at different LSNs. The second fails at `registry.rs::observe_connectivity_changed` (~line 401-406) because projected connectivity is already `Live` (`from`-state mismatch → `CorruptLog`).

**Result: a transient storage failure during a multi-delta report turns the log unreplayable.** This is a regression introduced by B3 (the old single-delta code didn't have this because it returned after one append).

**Location**: `core/src/session/ingest.rs` equal-generation branch (~line 215-269), the sequential `storage.append(...).await?` calls at lines 43/62/82 (within the branch), and the warm-path-after-success-only behavior.

**Fix options** (pick one, justify in Implementation notes):
- **(a) Warm the registry after each successful append, then re-derive from the warmed state.** After appending connectivity, observe the event into the registry (or re-read current state), then derive activity/metadata deltas against the UPDATED state. A retry after partial failure would then see connectivity as already-applied and skip it, appending only the remaining deltas. This is the most robust — it makes retry idempotent.
- **(b) Make the multi-delta append a single storage transaction** (if the `Storage` port supports a batch/transactional append). The `Storage::append` is currently one-event-at-a-time; this would need a new batch method or a transactional wrapper. Heavier change.
- **(c) Document that partial failure leaves the caller responsible for re-deriving from a fresh `rebuild_from_log` before retry.** Weakest — pushes the problem to every caller and is easy to get wrong.

Prefer **(a)** — it makes the write path self-correcting on retry without a storage-port change. The warm-after-each-append also keeps the registry consistent with the durable log mid-sequence. The acceptance `ingest_observation` pattern does one append then returns, so it doesn't have this issue; sessions is the first multi-append writer.

## Important (related, from same re-review)

### B5-note — Pre-fix generation-bump events become unreplayable (persistence compat break)
B2 added a required `initial_state` field to `SessionGenerationBumped`. Wire-decoding is additive (old tags preserved), but replay now REQUIRES `initial_state` and rejects events missing it as `CorruptRecord`. Pre-fix generation-bump events (written by the original `story-v0-core-sessions-ingest` before commit `06e6251`) would be rejected.

**Disposition**: Patchbay is pre-release with no production logs — any existing dev logs are disposable. This is a documented reset boundary, not a migration. If there are dev `.db` files lying around, they should be deleted (not migrated). Note this in the implementation notes and the feature body. If a future release ships before this and logs exist, THEN it's a migration concern — but v0.1.0 hasn't shipped, so no migration needed now.

## Acceptance Criteria

- [ ] A multi-delta report where the 2nd append fails does NOT leave an unreplayable log on retry
- [ ] Retry after partial failure is idempotent (re-derives against warmed/updated state, doesn't duplicate the already-applied delta)
- [ ] The warm path keeps the registry consistent with the durable log mid-sequence (if option a)
- [ ] New test: inject a storage failure on the 2nd append of a multi-delta report, retry, then `rebuild_from_log` succeeds
- [ ] Existing tests pass; `cargo build`, `cargo test -p patchbay-core`, `cargo clippy --all-targets` clean

## Notes

- This is a regression introduced by the B3 fix, found in the post-fix re-review (cross-model openai-codex/gpt-5.6-sol, fresh context).
- The B1-B4 fixes are all confirmed RESOLVED by the same re-review; only this new blocker (B5) remains.
- Depends on `story-fix-sessions-ingest-correctness` (the B3 fix that introduced this).
- `CARGO_HOME=/tmp/cargo-home` for all cargo commands.
