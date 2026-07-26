---
id: epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion
kind: story
stage: implementing
tags: [observability, dogfooding, protocol]
parent: epic-observability-dogfooding-cockpit-diagnostics
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
---

# Adapter diagnostic contract and audited core ingestion

## Checkpoint

Extend the generated adapter and diagnostics contracts with adapter-declared
diagnostic reporting, a typed diagnostic Observation payload, and the narrow
adapter-facing report RPC. Validate authenticated reports at the core boundary,
then atomically append the safe source Observation and its correlated typed
audit record through the core-diagnostics audited-append path. Extend the
replayable adapter-status projection with bounded recent diagnostic records.

This checkpoint owns Units 1-3 and the core/contract share of Unit 8 in the
parent feature. The parent design is authoritative for exact wire fields,
validation, redaction, pagination, and error behavior.

## Acceptance evidence

- Buf generation, lint, breaking, and generated-drift checks produce matching
  Rust and TypeScript contracts from the proto sources.
- Authenticated valid reports append one safe Observation plus one
  `ADAPTER_DIAGNOSTIC_REPORTED` audit record atomically; the audit references
  the source event and carries verified adapter/session attribution.
- Unknown enums, malformed payload envelopes, invalid/oversized codes or
  counts, incomplete target identity, adapter/generation mismatch, and
  warn/error reports without a canonical failure code return
  `validation_failed` without an append.
- Authentication failures remain transport authentication failures, and
  storage failure never returns a false accepted result.
- Replay and live catch-up produce the same bounded recent-diagnostics slice in
  `AdapterStatusPage`; diagnostic Observations never mutate adapter/session
  liveness or command state.
- Sentinel prompt, token, attachment, descriptor, message, stack, and cause
  values cannot enter source payloads, audit records, or query responses.

## Ordering constraints

No sibling dependency. It consumes the upstream core-diagnostics audited append,
audit registry, projection, and `QueryDiagnostics` surface; do not create a
second diagnostics log, table, or query service.
