---
id: gate-security-protect-sqlite-state-files
kind: story
stage: done
tags: [security]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: security
created: 2026-07-24
updated: 2026-07-24
---

# Protect SQLite state files from permissive process umasks

## Severity
Medium

## Domain
Data Protection

## Location
`core/src/storage/rusqlite.rs:111`

## Evidence
```rust
let write_db = Connection::open(path).map_err(map_write_err)?;
```

A direct launch under the repository's normal umask created the database, WAL, and SHM files with mode `0644`. The event log stores full accepted Operations (including prompt payloads), password verifiers, and principal-credential hashes, so a traversable state directory can expose sensitive durable state to other local users.

## Remediation direction
Create/use a dedicated private state directory, enforce restrictive permissions for the database and SQLite sidecars, and fail closed or emit a security-grade startup error when an existing state path is accessible beyond the service account. Add a Unix permission regression test and ensure the default database artifact is ignored from version control.

## Implementation

- Added a Unix persistence-adapter pre-open step using `OpenOptionsExt::mode(0o600)`, followed by `fchmod(0600)` on the opened database inode. New files are private regardless of a permissive umask, and an existing permissive database is tightened before SQLite reads it.
- Permission/open failures surface as non-retryable `StorageError::WriteFailed`, so core startup fails rather than continuing with an unsecured state file.
- Verified empirically while both SQLite connections remain live that the database, WAL, and SHM artifacts are all mode `0600`; SQLite derives the sidecar modes from the secured database file on Unix.
- Added regression coverage for both new database/sidecar creation and an existing `0644` database being tightened on open.
- Ignored the default `*.sqlite3`, `*.sqlite3-wal`, and `*.sqlite3-shm` artifacts from version control.

Implementation discovery: `RusqliteStorage::open` accepts an operator-selected path whose parent may intentionally be an existing repository or service directory. Forcibly chmodding that parent would be unsafe and outside the requested repair. Protecting every sensitive artifact at `0600` removes the cross-user read exposure while preserving deployment-neutral path selection; a dedicated managed state-directory convention belongs to deployment packaging rather than the storage port.

Execution capability: direct host ownership of the bounded persistence-adapter hardening and Unix regression tests.

## Extension pressure classification

Committed v0.1.0 behavior: the embedded Unix SQLite adapter protects its database and sidecars as owner-only files. The storage port and domain semantics remain backend-neutral; permissions for future non-Unix or alternate persistence adapters remain adapter/deployment responsibilities rather than core protocol primitives.

## Verification

- `cargo test -p patchbay-core --test rusqlite_storage` — 20/20 passed, including database/WAL/SHM `0600` and existing-file tightening assertions.
- `cargo test --workspace` — all workspace unit, integration, property, RPC, and doc-test suites passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.

## Bounded review

Reviewed creation and reopen behavior, Unix umask semantics, SQLite WAL/SHM inheritance, startup error mapping, cross-platform cfg isolation, and repository artifact hygiene. The empirical sidecar assertion confirms the inheritance assumption against the bundled SQLite version. No material blocker remains.
