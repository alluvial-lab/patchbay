---
id: epic-observability-dogfooding-core-diagnostics-query-surface
kind: story
stage: done
tags: [observability, dogfooding, protocol]
parent: epic-observability-dogfooding-core-diagnostics
depends_on: [epic-observability-dogfooding-core-diagnostics-audit-records]
release_binding: v0.2.0
gate_origin: null
created: 2026-07-25
updated: 2026-07-26
---

# Typed core-diagnostics query surface

## Checkpoint

Build the generated, bounded audit-history/command-inspection/adapter-status
query contract; replayable safe diagnostics projection; and principal-gated
`ControlService.QueryDiagnostics` execution path. Each request is an authorized
`OperationKind::Query` with the normal durable lifecycle and a correlated typed
result Observation. Retries return the original durable result, not a
recomputed newer projection.

This checkpoint owns Units 1 (query/result subset), 4, 5, and its share of Unit
6 in the parent feature. The parent design is authoritative for exact types,
paths, page limits, error behavior, lifecycle ordering, and safe field lists.

## Acceptance evidence

- Rust/TypeScript query contracts regenerate from proto and give downstream CLI
  and cockpit features stable `AuditPage`, `CommandInspectionResult`, and
  `AdapterStatusPage` types.
- Replay and incremental catch-up produce identical redacted command/adapter
  projections at the same LSN; adapter state is unknown after restart until a
  current attachment is authenticated.
- gRPC tests prove compound-issuer authorization, authority-domain query target,
  `accepted -> delivered -> completed|failed`, correlated durable result,
  exact-retry replay, and payload-conflict rejection.
- Unknown filters/enums, invalid domains/time windows/cursors, and zero or
  oversized limits reject before acceptance; valid no-match queries complete
  with typed empty results.
- Command prompt/payload/idempotency data and adapter attachment descriptors can
  never appear in results.
- Draft query-lifecycle/redaction vectors, workspace tests, generated drift, and
  the restart-capable real-process e2e pass.

## Ordering constraints

Consumes the audit registry, storage queries, and durable sink established by
`epic-observability-dogfooding-core-diagnostics-audit-records`; do not duplicate
those types or add a second persistence reader.

## Implementation notes

- Added the replayable `core::diagnostics` projection. Command timelines fold
  explicit Operation and CommandTransition records, while adapter status folds
  canonical redacted registrations and reports UNKNOWN after projection
  rebuild until a fresh attachment is observed. Projection output never copies
  attachment descriptor bytes.
- Added fail-fast query validation for the generated `DiagnosticsQuery`
  payload, authority-domain target, committed `QUERY` kind, enum/filter values,
  timestamps, domain-scoped cursors, and audit/command/adapter page bounds.
- Implemented `ControlService.QueryDiagnostics`. It authenticates the same
  compound issuer path as other control RPCs, accepts through the normal
  operation path using a narrow authority-domain resolver, persists delivered,
  typed result Observation, and completed lifecycle checkpoints under the
  submit gate. Exact retries locate and return the original durable result;
  payload conflicts remain pre-completion validation rejections.
- Added gRPC lifecycle/retry evidence and core projection replay/redaction
  tests. The command and adapter response families are typed generated result
  oneofs; audit reads remain behind the storage port.

Verification evidence for this checkpoint:

- `cargo test -p patchbay-core --test diagnostics_projection` — passed (2 tests).
- `cargo test -p patchbay-core-server --test grpc_smoke` — passed (8 tests).
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
