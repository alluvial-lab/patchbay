---
id: story-fix-sessions-tombstone-key
kind: story
stage: implementing
tags: [protocol, bug, verification, foundation]
parent: feature-v0-core-sessions
depends_on: [story-v0-core-sessions-registry]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Story: Fix session tombstone key to include full identity

## Scope

One correctness blocker found in the feature-level deep review of `feature-v0-core-sessions`. The tombstone map is keyed by `(runtime_session_id, generation)` only, omitting `adapter_id` and `deployment_scope` — half of the canonical session identity tuple. This causes cross-adapter tombstone collisions.

## Blocker

### B4 — Tombstone keys omit adapter_id + deployment_scope (cross-adapter collision)

Live sessions are correctly keyed by `(adapter_id, deployment_scope, runtime_session_id)` (`SessionLiveKey`). But tombstones are keyed by `(runtime_session_id, generation)` only (`SessionTombstoneKey`). The protocol identity tuple is `(adapter_id, deployment_scope, runtime_session_id, session_generation)` — runtime session IDs are **adapter-reported** and need not be globally unique.

**Concrete failure**: Two different sessions (adapter A/scope A and adapter B/scope B) that happen to share a `runtime_session_id` value collide:
1. Register adapter A/scope A/runtime `r`/generation 1.
2. Register adapter B/scope B/runtime `r`/generation 1.
3. Bump A to generation 2 — A's generation 1 is tombstoned under key `(r, 1)`.
4. `resolve` for B generation 1 calls `is_tombstoned(r, 1)` → returns true → **B is falsely rejected as a stale target.**
5. A later B state event at generation 1 hits `is_stale_replay`, finds A's tombstone, and either no-ops or returns a tombstone identity-collision `CorruptLog`.

**Location**: `core/src/session/registry.rs` `SessionTombstoneKey` (~line 63-65), `is_tombstoned` (~line 212), `get_tombstone` (~line 199), `is_stale_replay` (~line 483).

**Fix**: Extend `SessionTombstoneKey` to include `adapter_id` and `deployment_scope` (mirror `SessionLiveKey`). Update `get_tombstone`, `is_tombstoned`, and the tombstone insertion in `observe_generation_bumped` to key by the full identity. Update `is_stale_replay` to look up by full identity (it already compares `adapter_id`/`deployment_scope` against the found tombstone — with the full key, the lookup itself is correct). Update `resolver.rs` `is_tombstoned` call to pass the full identity.

## Acceptance Criteria

- [ ] Two sessions sharing a `runtime_session_id` but differing in adapter_id or deployment_scope do NOT collide on tombstones
- [ ] Bumping generation on adapter A does NOT affect resolution of adapter B's session with the same runtime_session_id
- [ ] `is_tombstoned` / `get_tombstone` / `is_stale_replay` all key by full identity `(adapter_id, deployment_scope, runtime_session_id, generation)`
- [ ] Existing tests still pass; new test reproduces the cross-adapter collision scenario
- [ ] `cargo build`, `cargo test -p patchbay-core`, `cargo clippy --all-targets` clean

## Notes

- Correctness bug found at feature review (Phase 2 adversarial, cross-model openai-codex/gpt-5.6-sol).
- The fix is localized to `registry.rs` (the `SessionTombstoneKey` struct + the three methods that use it) + `resolver.rs` (the `is_tombstoned` call site). No proto change.
- The `SessionTombstone` struct already carries `adapter_id` and `deployment_scope` fields — only the KEY needs extending.
