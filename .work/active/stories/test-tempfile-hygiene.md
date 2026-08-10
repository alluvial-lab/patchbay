---
id: test-tempfile-hygiene
kind: story
stage: done
tags: [testing, ops]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-10
---

# Backlog: test-suite tempfile hygiene (201K leaked SQLite temp files filled /tmp)

Surfaced 2026-07-24 when `/tmp` (7.9G tmpfs) hit 100% and crashed the box
(ENOSPC): **201,272 leaked `.tmp*` files (6.6G)** at the `/tmp` top level —
SQLite db/-wal/-shm triples created via Rust `tempfile` by the core test
suites (storage/acceptance proptests are the prime suspects: hundreds of cases
× three files per run, leaked when a test process is killed, times out, or
panics before `TempDir`/`NamedTempFile` drop runs).

## Shape (at design time)

- Point test temp roots at a scoped, cleanable location (e.g.
  `target/test-tmp/` via `TMPDIR` for test harnesses or an explicit
  tempdir-per-binary root) instead of the shared system `/tmp`.
- Add a `cargo test` wrapper or xtask that cleans stale test-temp before runs.
- Investigate whether any production code path creates `NamedTempFile`s in the
  system tempdir (it should not — persistence lives under PATCHBAY_DB_PATH).
- **Production-code tempfile (2026-08-09 review — answers the bullet above):**
  `core/src/storage/rusqlite.rs:300` `open_in_memory()` uses
  `NamedTempFile::new()` (→ system tempdir) and is a `pub fn` **not
  `#[cfg(test)]`-gated** — its own doc comment admits it leaks one file per
  call ("acceptable for the test suite, not for production paths").
  `core/src/adapter/mod.rs:720` is `#[cfg(test)]`-gated (fine). The fix
  must additionally `#[cfg(test)]`-gate or relocate `open_in_memory()` so
  it cannot leak from a production call path; the `TMPDIR` /
  `target/test-tmp` redirect then covers its test-driven use.
- Consider a `.gitignore`d `tmp/` location convention for all test scratch.

## Why not fixed inline

The leak was cleaned manually (2026-07-24, /tmp back to 7%); the structural
fix touches the whole test harness layout and deserves its own pass.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-luna`-acceptable bounded story,
  delivered by the run's Sol endpoint; the change is cohesive test
  infrastructure plus one storage-lifetime correction.
- Review weight: `thorough` (explicit caller selection); standalone-story
  review remains the bounded inline lane.
- Files changed: `scripts/test-rust`, `core/src/storage/rusqlite.rs`, and
  `docs/RUNBOOK.md`.
- Tests added/removed: no new test module. The existing SQLite storage suite
  exercises the changed constructor; a process-level probe additionally
  confirmed its scoped temp directory was empty after `open_in_memory()` test
  shutdown.
- Simplification: `open_in_memory()` no longer calls `TempPath::keep()` and
  leaks each database. The writer actor retains the `NamedTempFile` guard until
  both SQLite use and the writer loop end, then normal shutdown removes it.
- Discrepancy from the recorded `#[cfg(test)]` refinement: Rust integration
  tests compile the library without `cfg(test)` and use this helper throughout,
  so gating it that way would remove the test interface. The implementation
  instead eliminates the production-call leak and documents the intentionally
  test-oriented API; the wrapper contains abnormal-termination residue under a
  cleanable repository-local root.
- Adjacent issues parked: none.

## Verification

- `./scripts/test-rust -p patchbay-core --test rusqlite_storage --quiet` — 20
  tests passed; `target/test-tmp/` was removed on exit.
- Scoped process probe with `TMPDIR=target/tempfile-lifetime-probe` and
  `empty_log_read_returns_empty` — one test passed and zero files remained.
- `cargo clippy -p patchbay-core --all-targets -- -D warnings` — passed.
- `bash -n scripts/test-rust` and `git diff --check` — passed.
- Bounded review fix: cleanup is registered only on shell exit, avoiding an
  INT/TERM trap that could remove the temp root while Cargo children are still
  shutting down.
- Repository-wide `cargo fmt --all --check` is pre-existingly red on unrelated
  committed server test formatting; the touched Rust hunk follows its file's
  current formatting style.

## Review

- Lane: bounded inline standalone-story review; no independent or cross-model
  reviewer is permitted for this item kind.
- Effective weight: `thorough` (explicit caller), bounded by the standalone
  lane rather than converted into feature-style independent passes.
- Pass 1 found and fixed a material cleanup-ordering race in signal traps; the
  wrapper now cleans only after Cargo and its children have exited.
- Closure check: focused SQLite test, scoped-root removal assertion, shell
  syntax, clippy, and diff hygiene are green. No material current-cycle blocker
  remains.
- Verdict: approved.

## Phase 8 completion-review fix

The consolidated completion pass found that terminating the wrapper could leave
Cargo alive while the EXIT trap removed its scratch root. `scripts/test-rust`
now runs Cargo in a managed process group, forwards INT/TERM, waits for group
termination, and only then cleans `target/test-tmp`. The executable
`scripts/test-rust-signal-test` proves wrapper TERM returns 143, leaves neither
Cargo nor its test child alive, and removes the scoped root. Shell syntax, the
signal regression, focused SQLite storage test, cleanup assertion, and diff
hygiene pass.
