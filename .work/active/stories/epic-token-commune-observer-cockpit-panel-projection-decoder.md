---
id: epic-token-commune-observer-cockpit-panel-projection-decoder
kind: story
stage: implementing
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Shared manifest-bound token-commune decoder

## Checkpoint

Create the bounded Patchbay-owned `@patchbay/operator-domain` package and its local known decoder for the exact provider-pool and member-draw kinds plus both manifest descriptors. Decode schema-bound JSON into closed surface types without importing adapter implementation or retaining invalid raw bytes.

## Primary files

- `operator-domain/package.json`
- `operator-domain/tsconfig.json`
- `operator-domain/src/token-commune.ts`
- `operator-domain/tests/token-commune.test.ts`
- web/CLI package manifests and lockfiles

## Acceptance evidence

- Exact `patchbay.token_commune.*.v1` payload/projection descriptor pairs decode only under the matching resource kind.
- Wrong kind/content type/schema ref is unsupported before payload parse.
- Malformed strings, fractions, counts, timestamps, health rows, model rows, draw rows, or `capacityAggregation` fail closed with no raw-byte retention.
- Provider-pool and member-draw variants remain adapter-owned surface values, not protocol enums.

## Ordering

First checkpoint. The pool compositor depends on these stable decoded types.
