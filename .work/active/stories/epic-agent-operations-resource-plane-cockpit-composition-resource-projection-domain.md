---
id: epic-agent-operations-resource-plane-cockpit-composition-resource-projection-domain
kind: story
stage: implementing
tags: [ux, protocol]
parent: epic-agent-operations-resource-plane-cockpit-composition
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Resource projection domain and local decoders

## Checkpoint

Add the cockpit-side `ResourceView` / `ResourceCollectionView` presentation
model and the closed local decoder registry designed in the parent feature.
The registry matches the exact resource kind plus both payload/projection
`(schema_ref, content_type)` descriptors before decoding versioned
`provider_pool` or `usage_window` JSON into the two selected local presentation
variants.

Unknown descriptors and malformed semantic payloads must preserve the canonical
resource identity/freshness/revision wrapper while installing no adapter-domain
projection. This checkpoint does not add navigation, provider/model concepts,
or dynamic adapter renderer code.

## Primary files

- `web-cockpit/src/domain/resource-projection.ts` (new)
- `web-cockpit/src/domain/model.ts`
- `web-cockpit/tests/resource-projection.test.ts` (new)
- `web-cockpit/tests/model.test.ts`

## Acceptance evidence

- Exact kind + dual-descriptor tests accept the two known v1 projections and
  reject a mismatch in any dimension.
- Semantic decoder tests reject malformed JSON, missing/bad fields, unknown
  health variants, non-finite or out-of-range percentages, and invalid counts
  without exposing raw bytes.
- Full `(adapter_id, resource_kind, resource_id)` keys remain collision-free.
- `rendersResourceCurrent` permits current styling only for reconciled,
  non-tombstoned canonical `CURRENT` records.

## Ordering

This is the first checkpoint. The resource reconciliation checkpoint depends on
these model and decoder contracts.
