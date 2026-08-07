---
id: epic-token-commune-observer-snapshot-mapping-projection-contract
kind: story
stage: implementing
tags: [adapter, protocol]
parent: epic-token-commune-observer-snapshot-mapping
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Preserve the projection input and schema honesty contract

## Checkpoint

Define the pure endpoint-state input and tighten the existing token-commune JSON contracts so projection cannot erase native health detail or conflate an unavailable source, an unlisted provider, and a listed contribution with no capacity readings. Preserve the foundation's two ResourceKinds, four schema refs, PARTIAL tiers, gateway methods, and swappable `ResourceIdentitySynthesizer`.

The gateway DTO correction is additive to the stable method seam: `GatewayContributionHealth` becomes a discriminated value that retains `exhaustedUntil` or `reason` instead of validating and discarding them. The resource/projection schemas add the explicit synthesized-sub-key labeling and source-state taxonomy required by the parent design; they do not add a ResourceKind or change a descriptor.

## Files

- `token-commune-adapter/src/gateway_client.ts`
- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/schemas/provider-pool-payload.schema.json`
- `token-commune-adapter/schemas/provider-pool-projection.schema.json`
- `token-commune-adapter/schemas/member-draw-payload.schema.json`
- `token-commune-adapter/schemas/member-draw-projection.schema.json`
- `token-commune-adapter/tests/gateway_client.test.ts`
- `token-commune-adapter/tests/resource_contract.test.ts`

## Acceptance evidence

- Exhausted-until and auth-broken-reason survive runtime decoding and schema validation byte-for-byte.
- The schema distinguishes `reported`, `not-reported`, and `unavailable` source slices and requires explicit snapshot-local synthesized contribution-key provenance.
- Per-reading nullable fields remain nullable; no provider-level capacity percentage or summary field exists.
- The two kinds, four schema refs, and PARTIAL manifest declarations remain unchanged.
- Mutations that drop health detail, treat absent health metadata as fresh, or weaken required source-state fields fail the relevant test.

## Ordering

This checkpoint establishes the typed and schema source of truth consumed by envelope construction and both kind mappings.
