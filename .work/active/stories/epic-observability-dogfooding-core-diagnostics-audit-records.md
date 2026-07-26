---
id: epic-observability-dogfooding-core-diagnostics-audit-records
kind: story
stage: done
tags: [observability, dogfooding, security]
parent: epic-observability-dogfooding-core-diagnostics
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-26
---

# Durable canonical audit records

## Checkpoint

Establish the generated `AuditEventKind`/`AuditRecord` contract derived from
`docs/SECURITY.md`, retrofit versioned SQLite migrations, and persist redacted
audit records through the core's single writer. Security-relevant durable source
events and their distinct audit records commit atomically; rejected decisions
may create an audit record without command state. Replace the login-only sink
with the feature design's required durable `AuditSink` composition and wire the
canonical core/server/control-surface producers.

This checkpoint owns Units 1 (audit subset), 2, 3, and its share of Unit 6 in the
parent feature. The parent design is authoritative for exact types, paths,
redaction rules, and migration behavior.

## Acceptance evidence

- Generated Rust/TypeScript audit contracts and Buf drift checks are green.
- Legacy unversioned and fresh databases migrate to schema version 2 without
  losing events, idempotency rows, or snapshots; future/malformed schemas fail
  without mutation.
- Property/integration tests prove all-or-nothing source+audit commit, durable
  reopen, bounded/filterable audit pages, and domain/cursor isolation.
- Producers cover login/bootstrap/session/grant/authorization/command lifecycle,
  stale-event, adapter lifecycle, and authenticated control-surface integrity
  decisions using verified attribution.
- Sentinel values for every SECURITY no-log field are absent from SQLite bytes,
  queryable audit records, and stderr diagnostics.
- Production composition cannot start with stderr-only auditing.

## Ordering constraints

No sibling dependency. Complete this checkpoint before the diagnostics query
surface consumes `AuditQuery`, `AuditPage`, `AuditSink`, or the versioned audit
storage port.

## Implementation notes

- Added `diagnostics.proto` as the generated source for the canonical audit
  vocabulary and redacted `AuditRecord` contract, including the query/result
  wire types and the `StoredEventKind::AUDIT_RECORD` discriminator. Rust and
  TypeScript artifacts are regenerated from the shared schema; Rust build
  generation explicitly permits the intentionally large diagnostics oneofs.
- Added versioned SQLite migrations (`0 -> 1 -> 2`) with `PRAGMA user_version`,
  schema-shape validation, future-version fail-closed behavior, WAL/FULL
  durability, and a derived `audit_records` index. The index is transactionally
  maintained and every read validates its indexed columns against the encoded
  audit event before returning a page.
- Added typed `AuditRecordDraft`, atomic source-plus-audit and deduplicated
  append storage operations, descending bounded filter/cursor reads, and
  reopen/future-schema/audited-append evidence in `core/tests/audit_records.rs`.
- Added the core `AuditSink` family: durable sink, explicit diagnostic stderr
  sink, and a required durable-first fanout. Control-service login auditing now
  composes the durable sink before its legacy stderr-compatible observer.
- Existing storage test doubles were extended to forward the new optional
  storage operations; no production tests were weakened or removed.

Verification evidence for this checkpoint:

- `cargo test -p patchbay-core --test audit_records` — passed (3 tests).
- `cargo test -p patchbay-core --test rusqlite_storage --test storage_port_smoke` — passed (27 tests).
- `cargo test -p patchbay-core-server --test grpc_smoke --test trust_boundary` — passed (14 tests).
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cd contracts/ts && npm run build && npm run check:vectors && npm run check:models` — passed.
