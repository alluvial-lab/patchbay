---
id: test-tempfile-root-cause-scoping
kind: story
stage: implementing
tags: [testing, ops, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Test tempfile root-cause scoping (the opt-in wrapper is not a root fix)

## Why this exists (the outage)
On 2026-08-10 the scoped autopilot drain was killed mid-run by **ENOSPC at 100% disk** — the recurring leaked `.tmp*` SQLite triples (the `test-tempfile-hygiene` failure mode) filled `/tmp` again. `test-tempfile-hygiene` was marked `done`, but what landed is a **mitigation, not a root fix**, and the drain itself bypassed it.

## What landed (the mitigation — insufficient)
`scripts/test-rust` does `export TMPDIR="$test_tmp"` (→ `target/test-tmp`) + cleanup traps + the Phase-8 process-group/signal fix. **It only scopes temps when tests run *through* that wrapper.**

## Why the leak persists (root cause)
1. **The fix is opt-in.** The drain's review workers ran `cargo test` *directly in git worktrees* (`cd /tmp/patchbay-fix-… && CARGO_TARGET_DIR=… cargo test`) — `TMPDIR` was never set, so every `NamedTempFile::new()` went to `/tmp`. Any direct `cargo test` / CI / worktree run bypasses the wrapper and leaks.
2. **`NamedTempFile::new()` / `tempdir()` is still called directly in 9+ test files**, all defaulting to the system tempdir: `core/tests/storage_proptest.rs` (lines 310, 750, 1056, 1076), `core/tests/recovery.rs`, `core/tests/audit_records.rs`, `core/tests/rusqlite_storage.rs`, `server/tests/grpc_smoke.rs`, `server/tests/spawn_completion.rs`, `server/src/adapter_service/tests.rs`.
3. **The C1 finding was never addressed:** `core/src/storage/rusqlite.rs:735` `open_in_memory()` is **still an un-`#[cfg(test)]`-gated `pub fn`** calling `NamedTempFile::new()` — still leaking one file per call (its own doc admits it).

## Fix (scope at the test-support layer, not an external wrapper)
Make temp scoping hold **regardless of invocation** (wrapper, direct `cargo test`, CI, worktrees):
- A `patchbay_test_support` helper (or a test-binary init / `#[ctor]`) that sets `TMPDIR` to a scoped, cleanable, per-run root **before any tempfile is created**, and routes the existing direct `NamedTempFile::new()` / `tempdir()` calls through it (or relies on the env). `tempfile` reads `TMPDIR` per-call, so a process-wide set at test init is sufficient.
- `#[cfg(test)]`-gate or relocate `open_in_memory()` so it cannot leak from a production call path (the C1 finding).
- Verify by running `cargo test` *directly* (not via the wrapper) and confirming nothing lands in `/tmp`.

## Priority
High — blocks test-running reliability. The 2026-08-10 ENOSPC killed an ~8h drain run. Land this before any further test-heavy drain.

## Relation to test-tempfile-hygiene
`test-tempfile-hygiene` shipped the wrapper-script mitigation (real, retained); this story closes the root cause it left open. Its "done" was premature — the thorough review passed the wrapper without catching that it doesn't cover direct `cargo test` runs or the un-gated `open_in_memory()`.
