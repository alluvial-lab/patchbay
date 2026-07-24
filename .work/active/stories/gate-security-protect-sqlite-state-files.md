---
id: gate-security-protect-sqlite-state-files
kind: story
stage: drafting
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
