---
id: epic-observability-dogfooding-core-diagnostics
kind: feature
stage: drafting
tags: [observability, dogfooding]
parent: epic-observability-dogfooding
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
---

# Core-diagnostics query capability

## Brief

The core's durable event log is the source of truth for command, session, and
adapter state, and security audit records exist only as redacted process
stderr/stdout lines. Neither is queryable: the CLI diagnostic commands are
stubs, and the cockpit can only show current state.

This feature builds core-diagnostics: durable, queryable read projections over
existing core state, exposed through the core's service surface for control
clients. Two halves:

1. **Durable audit records** — audit decisions (authentication, authorization,
   command lifecycle, adapter attach/detach, stale-event rejection) are
   persisted by the core to storage behind the existing ports, applying the
   canonical SECURITY redaction list. The core remains the single writer;
   stderr lines remain as process diagnostics.
2. **Diagnostics query surface** — read endpoints answering audit history,
   command history/inspection, and adapter status as projections over the
   durable log and audit records. Queries route through the core as control
   operations; no surface touches persistence directly (the no-lifecycle
   bypass-read seam stays reserved).

It does NOT cover: the CLI commands that consume this surface
(`epic-observability-dogfooding-cli-diagnostics`), adapter-reported
diagnostics ingestion (`epic-observability-dogfooding-cockpit-diagnostics`),
metrics, or `event-inspect <lsn>` (reserved).

## Epic context

- Parent epic: `epic-observability-dogfooding`
- Position in epic: foundation feature — both consumer features
  (CLI diagnostics, cockpit diagnostics) depend on its query surface and
  contract types. Priority 2 in the epic's seed order.

## Simplification opportunity

- Consolidates audit emission behind one sink abstraction: the current
  `StderrLoginAuditSink` (and any peers) becomes one sink implementation
  alongside the durable sink, rather than audit callsites growing ad-hoc
  output channels. Design should check whether the login-audit-specific trait
  generalizes to the full audit-event vocabulary in SECURITY.md.
- Removes the spec/code divergence where the durable queryable audit log was
  committed in prose but absent in code.

## Foundation references

- `docs/SPEC.md` — post-v0.1.0 observability scope (core-diagnostics)
- `docs/PROTOCOL.md` — Snapshots and streams; Persistence and recovery;
  extension seams registry (flipped C rows)
- `docs/SECURITY.md` — Audit events (canonical redaction list, audit
  vocabulary)
- `docs/ARCHITECTURE.md` — Ports & Adapters (storage port the durable sink
  sits behind)

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will
fill in the query operation shapes, storage schema, and implementation units. -->
