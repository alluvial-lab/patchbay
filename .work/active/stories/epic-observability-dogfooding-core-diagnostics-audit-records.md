---
id: epic-observability-dogfooding-core-diagnostics-audit-records
kind: story
stage: done
tags: [observability, dogfooding, security]
parent: epic-observability-dogfooding-core-diagnostics
depends_on: []
release_binding: v0.2.0
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

## Reopened (2026-07-26, orchestrator wave-1 verification)

The first implementation wired only the login producer family
(`LoginSucceeded`/`LoginFailed`). The acceptance evidence requires producers
covering bootstrap/session/grant/authorization/command lifecycle, stale-event,
adapter lifecycle, and authenticated control-surface integrity decisions. The
implementing worker recorded this itself as a deviation ("broader canonical
producer migration ... remain follow-up work"), so the `done` transition was
premature. A follow-up worker owns completing the producer migration; this
checkpoint returns to `done` only when that coverage is green.

Additionally, wave integration verification found `pi-adapter` e2e failing
(`accepted.acceptedLsn` undefined) after the audited-append change landed —
diagnosis and fix ride with the same follow-up.

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

## Follow-up completion (2026-07-26)

The bounced verification is complete. The `pi-adapter` regression was rooted in
an outdated e2e operation fixture, not the audited-append response path:
`563b3b6` made `validity_window` and `submitted_at` required at acceptance, while
the fixture still constructed the pre-enforcement Operation shape. The test
assertion remains unchanged; the fixture now supplies an active, long-lived
window. Running the real server + adapter e2e after that correction returned an
accepted result with `acceptedLsn` and completed the full restart/reconnect flow.

Production composition now installs `core::storage::AuditedStorage` at the
server root. Its source-event classification upgrades ordinary production
appends to `append_audited` / `append_dedup_audited`, so source and audit rows
commit in the same SQLite writer transaction. Rejected submissions and
control-surface integrity failures use the durable-first `AuditSink` directly.
The control service's generated `RecordControlSurfaceAudit` ingress replaces
caller attribution, timestamp, and session material with verified context and a
hash before appending.

### Producer coverage inventory

| Decision point | Audit event kind(s) | Producer / file |
| --- | --- | --- |
| Bootstrap setup accepted/rejected and expiry | `BOOTSTRAP_STARTED`, `BOOTSTRAP_EXPIRED` | `server/src/admin_service.rs` |
| Bootstrap durable completion | `BOOTSTRAP_COMPLETED` | `core/src/storage/audited.rs` for `OperatorRecord` |
| Login success/failure | `LOGIN_SUCCEEDED`, `LOGIN_FAILED` | `server/src/service.rs` |
| Operator session issue/revoke | `OPERATOR_SESSION_CREATED`, `OPERATOR_SESSION_REVOKED` | `server/src/admin_service.rs`, `server/src/service.rs` |
| Control-surface integrity ingress | `CSRF_CHECK_FAILED`, `ORIGIN_CHECK_FAILED`, `FETCH_METADATA_CHECK_FAILED` | `web-server/src/middleware/csrf-auth.ts`, `web-server/src/routes/rpc.ts`, `server/src/service.rs` |
| Grant issuance/revocation | `GRANT_CREATED`, `GRANT_REVOKED` | `core/src/storage/audited.rs` for grant/revocation source events |
| Accepted command submission | `COMMAND_SUBMISSION_ACCEPTED` | `core/src/storage/audited.rs` for `Operation` |
| Authorization/validation rejection | `COMMAND_SUBMISSION_REJECTED` | `server/src/service.rs` |
| Command lifecycle transitions | `COMMAND_DELIVERED`, `COMMAND_RUNNING`, `COMMAND_COMPLETED`, `COMMAND_REJECTED`, `COMMAND_FAILED`, `COMMAND_EXPIRED`, `COMMAND_CANCELLED`, `COMMAND_SUPERSEDED` | `core/src/storage/audited.rs` for `CommandTransition` |
| Stale generation / late-event rejection | `STALE_EVENT_IGNORED` | `server/src/adapter_service.rs` and observation classification in `core/src/storage/audited.rs` |
| Adapter attach | `ADAPTER_ATTACHED` | `core/src/storage/audited.rs` for redacted registration Observation |
| Adapter detach/failure | `ADAPTER_DETACHED`, `ADAPTER_FAILED` | `server/src/adapter_service.rs` |

All allowlisted audit DTOs remain structurally redacted; no payload, prompt,
credential, token, cookie, or attachment descriptor is copied into an audit
draft or diagnostic stderr line.

### Follow-up verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cd pi-adapter && npm test` — passed (full suite, including real-process e2e).
- `cd e2e && npm test` — passed.
- `cd contracts/ts && npm run build && npm run check:vectors && npm run check:models` — passed.
- `cd web-server && npm test` — passed (24 tests, including durable integrity-ingress call assertions).

Test integrity: no assertions were weakened, skipped, or deleted. The only e2e
change supplies fields required by the already-landed validity-window security
contract; the original `acceptedLsn` assertion remains the regression guard.
