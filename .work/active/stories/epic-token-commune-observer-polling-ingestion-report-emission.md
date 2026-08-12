---
id: epic-token-commune-observer-polling-ingestion-report-emission
kind: story
stage: done
tags: [adapter, protocol]
parent: epic-token-commune-observer-polling-ingestion
depends_on: [epic-token-commune-observer-polling-ingestion-poll-runtime]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# token-commune projected report emission

## Checkpoint

Extend the gateway error boundary with normalized safe `Retry-After` advice and
wire each settled poll into the existing pure
`projectTokenCommuneSnapshot(...)`. Sample the injected completion clock for
`ResourceReport.observed_at`, preserve every nested capacity reading's upstream
`observedAt`, and send the generated report through a narrow
`PatchbayCoreClient.ingestResourceReport` method using the existing authenticated
attachment/reauth wrapper.

Events remain downstream and are processed only after report acceptance. A
failed source becomes `unavailable`; all five snapshot failures still produce
two empty PARTIAL views when core ingress is reachable, allowing core-owned
omission degradation. No cached gateway state, AUTHORITATIVE promotion,
tombstone, or synthetic zero belongs here.

## Files

- `token-commune-adapter/src/gateway_client.ts`
- `token-commune-adapter/src/poller.ts`
- `token-commune-adapter/src/core_client.ts`
- `token-commune-adapter/tests/gateway_client.test.ts`
- `token-commune-adapter/tests/poller.test.ts`

## Acceptance evidence

- Each cycle emits exactly the pure projector report through
  `ObservationRequest.resource_report` with the current adapter generation.
- Fake completion time becomes report time while capacity source times survive
  byte-for-byte.
- Partial and all-source failures remain schema-valid PARTIAL reports with no
  prior-source substitution.
- Delta-seconds/HTTP-date retry advice is normalized without retaining response
  bodies, headers, credentials, or arbitrary error text.

## Ordering constraint

Depends on the non-overlapping scheduler. The event mapper consumes this
report-before-event ordering but does not alter projection semantics.

## Implementation notes

Added safe delta-seconds/HTTP-date `Retry-After` normalization, exact
`ObservationRequest.resource_report` ingress on the authenticated attachment,
and report-before-event ordering. Every cycle passes fresh settled endpoint
states into `projectTokenCommuneSnapshot`; all-source failure still sends the
projector's empty two-view PARTIAL report. Tests independently preserve nested
capacity `observedAt` while asserting the fake completion clock is only the
report refresh timestamp.

Implementation discovery: the core previously treated every STATUS Observation
as a command-lifecycle candidate. Foundation semantics already permit generic
resource STATUS facts, so the acceptance boundary now admits only the narrow
uncorrelated, exact-resource, `FailureCode.UNSPECIFIED` shape and keeps malformed
or lifecycle-shaped STATUS evidence fail-closed. Its audit projection no longer
mislabels that shape as `CommandRunning`. Direct core acceptance tests cover the
canonical STATUS shape. Authenticated adapter-service coverage now attaches the
resource adapter, reports the target,
ingests the uncorrelated resource STATUS, and verifies one Observation with no
command transition plus cross-adapter and mixed-target rejection. The adapter's
real-process e2e still uses an empty event page and does not itself exercise
STATUS emission.
