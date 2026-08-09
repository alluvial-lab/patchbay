---
id: test-tempfile-hygiene
kind: story
stage: implementing
tags: [testing, ops]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
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
- Consider a `.gitignore`d `tmp/` location convention for all test scratch.

## Why not fixed inline

The leak was cleaned manually (2026-07-24, /tmp back to 7%); the structural
fix touches the whole test harness layout and deserves its own pass.
