---
id: epic-observability-dogfooding-core-diagnostics-audit-records
kind: story
stage: implementing
tags: [observability, dogfooding, security]
parent: epic-observability-dogfooding-core-diagnostics
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
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
