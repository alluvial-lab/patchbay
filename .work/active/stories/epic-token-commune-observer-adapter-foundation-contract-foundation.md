---
id: epic-token-commune-observer-adapter-foundation-contract-foundation
kind: story
stage: implementing
tags: [adapter, protocol, integration]
parent: epic-token-commune-observer-adapter-foundation
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# Establish the token-commune package and stable resource contract

## Design checkpoint

Create the strict Node 22 TypeScript package at `token-commune-adapter/`, its
fail-fast environment config, the single ResourceKind/schema registry, four
Draft 2020-12 JSON schemas, the swappable composite-local identity synthesizer,
and `tokenCommuneCapabilityManifest()`.

The exact declared kinds are:

- `token-commune.provider-pool` — `PARTIAL`;
- `token-commune.member-draw` — `PARTIAL`.

Both use JSON payload/projection descriptors from the feature body's
`TOKEN_COMMUNE_SCHEMAS` registry. The manifest targets only
`OPERATIONAL_RESOURCE`, declares no OperationKinds, no session snapshot tier,
no streaming/cancellation/replacement support, and no end-to-end idempotency.

Provider-pool is one resource per gateway deployment + provider and retains
anonymous contribution rows. Member-draw is one credential-relative resource
per deployment + member display name + provider. Do not create per-contribution
resources: current endpoints cannot reliably join ids, owners, declared share,
and capacity rows.

## Acceptance evidence

- `npm run build` passes with strict/no-unchecked/exact-optional TypeScript.
- Config tests cover every required/defaulted environment key and fail before
  network access without echoing values.
- Manifest tests prove exact categories, kinds, PARTIAL tiers, schema refs,
  content types, empty OperationKind set, known failure modes, and empty
  attachment descriptor.
- Schema tests prove required fields and nullable capacity/draw fields; no schema
  admits credentials, prompts/responses, provider onboarding material, or
  arbitrary diagnostics.
- Identity tests prove canonical URL determinism and adapter/gateway/kind/
  provider/member collision fencing.

## Ordering constraint

This checkpoint owns the stable seams used by every later story and by the
`snapshot-mapping` and `polling-ingestion` features. Do not implement HTTP
polling or resource reports here.
