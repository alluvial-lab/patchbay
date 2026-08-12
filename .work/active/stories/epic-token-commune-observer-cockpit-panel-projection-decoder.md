---
id: epic-token-commune-observer-cockpit-panel-projection-decoder
kind: story
stage: done
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-08-07
updated: 2026-08-08
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

## Implementation notes

- Added the pure `@patchbay/operator-domain` package and exact dual-descriptor decoder for the two manifest-bound token-commune kinds.
- Boundary parsing validates closed object shapes, bounded strings, closed discriminants, safe counts, finite fractions, RFC 3339 timestamps, anonymous contribution telemetry, draw reports, catalog rows, and literal `capacityAggregation: "none"`; invalid results retain neither bytes nor decoded identity-bearing rows.
- The upstream-rejected bare `gpt-5.6` alias fails closed per the explicit implementation brief rather than being rendered as a Patchbay alias.
- Verification: `cd operator-domain && npm test` — 7/7 tests passed, including descriptor mismatch-before-parse and byte non-retention witnesses.
