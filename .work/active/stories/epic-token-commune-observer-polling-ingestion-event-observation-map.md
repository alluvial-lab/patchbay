---
id: epic-token-commune-observer-polling-ingestion-event-observation-map
kind: story
stage: done
tags: [adapter, protocol]
parent: epic-token-commune-observer-polling-ingestion
depends_on: [epic-token-commune-observer-polling-ingestion-report-emission]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# token-commune pool-event status Observation mapping

## Checkpoint

Add one adapter-owned upstream event-kind disposition registry, the pool-event/
gap schema registry, two closed JSON schemas, and pure mappers from gateway
events/gap evidence to generic generated `Observation`s. The gateway decoder
and mapper derive from the disposition registry. Map exactly `capacity_shift`,
`auth_broken`, `windfall`, `fingerprint`, and `member` as source-authenticated
`STATUS` emissions targeted at the synthesized
`token-commune.provider-pool` operational resource.

Every pool-event payload retains source event id/time, provider, nullable
contribution id, and bounded message plus explicit
`deliveryModel: polling`/`historyMode: latest-50-no-cursor`. The Observation's
`observed_at` is the upstream `occurredAt`, not poll completion. Keep
`window_exhausted` and `calibration` decodable but return the declared-only
result and diagnostic; do not claim production coverage or silently map them.

## Files

- `token-commune-adapter/src/event_observation.ts`
- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/schemas/pool-event-observation.schema.json`
- `token-commune-adapter/schemas/event-gap-observation.schema.json`
- `token-commune-adapter/src/core_client.ts`
- `token-commune-adapter/tests/event_observation.test.ts`

## Acceptance evidence

- Independent literal fixtures cover the five emitted kinds, exact STATUS kind,
  resource tuple, sender/domain, JSON descriptors, and upstream timestamp.
- Declared-only/unknown kinds and malformed/schema-invalid values cannot reach
  core ingress.
- Gap payloads expose only observed window sizes/overlap/reason and never an
  estimated missed count or authoritative continuity claim.
- Payloads contain no credential/attachment/provider-secret or LLM data-plane
  content.

## Ordering constraint

Depends on report ingress so observations are emitted only after their resource
report. The latest-window tracker decides which mapped facts are eligible next.

## Implementation notes

Added the single event-kind disposition registry, two closed Draft 2020-12 JSON
schemas, and pure pool-event/gap mappers. Exactly the five production kinds map
to STATUS with authenticated adapter sender, exact provider-pool resource scope,
adapter-owned schema refs, polling/latest-50 labels, and source `occurredAt`.
`window_exhausted` and `calibration` remain decodable declared-only outcomes;
unknown kinds and malformed values fail closed. Independent literal tests also
prove gap payloads expose measured window evidence without a missed count or
authoritative continuity claim.
