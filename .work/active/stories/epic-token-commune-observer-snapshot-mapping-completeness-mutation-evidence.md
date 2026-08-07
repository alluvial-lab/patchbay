---
id: epic-token-commune-observer-snapshot-mapping-completeness-mutation-evidence
kind: story
stage: done
tags: [adapter, protocol]
parent: epic-token-commune-observer-snapshot-mapping
depends_on: [epic-token-commune-observer-snapshot-mapping-member-draw-projection]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Prove PARTIAL omission and null-state honesty with fixtures

## Checkpoint

Close the pure projection seam with fixture-driven, mutation-sensitive evidence for every honesty invariant. Pin the exact taxonomy: typed current evidence yields upsert; this observer emits no `unknown` because current endpoint evidence always either constructs a schema-valid payload or supplies no identity; it emits no tombstone/replacement because the upstream reads carry no terminal retirement evidence; identities omitted from either PARTIAL snapshot view are left unlisted so the existing core degrades cached current payload to stale.

## Files

- `token-commune-adapter/tests/fixtures/snapshot_projection.ts`
- `token-commune-adapter/tests/snapshot_projection.test.ts`
- `token-commune-adapter/tests/gateway_client.test.ts`
- `token-commune-adapter/tests/resource_contract.test.ts`

## Acceptance evidence

- Independent fixtures distinguish: listed contribution with zero readings; readings present but no `5h`; endpoint source unavailable; and resource omitted so core freshness, not adapter payload, owns stale.
- Snapshot views are always PARTIAL and deterministic, with upserts only for current identities and no unknown/tombstone arms.
- Mutation witnesses fail for pool aggregation, null-to-zero coercion, health-detail loss, model alias/upstream fabrication, draw collapse, probe overclaim, descriptor mismatch, and PARTIAL-to-AUTHORITATIVE promotion.
- A direct malformed-reading fixture fails closed before report return, while the gateway decoder mutation proves malformed wire readings cannot reach projection normally.
- Full package build/tests and `git diff --check` pass; no polling timer, core RPC, cockpit code, event Observation, or conformance-vector promotion enters this feature.

## Ordering

Depends on both completed kind mappings and is the final implementation checkpoint before feature-level thorough review.

## Implementation notes

- Added independent fixture evidence for reported-empty, unavailable-source, current upsert, PARTIAL omission, no-readings, zero telemetry, missing-5h, nullable calibration, probe coverage, and malformed constructed evidence.
- Every current mutation is asserted as `upsert`; the projector has no emitted `unknown`, tombstone, or replacement path, and omitted identities remain absent from PARTIAL views for core-owned stale degradation.
- Mutation witnesses were executed and reverted: PARTIAL→AUTHORITATIVE failed the completeness test; collapsing same-provider draws to one row failed the row-retention test; fabricating an Anthropic `reported` fingerprint for an unprobed provider failed the exact-probe test. Each mutant exited non-zero as intended.
- Verification: standalone `npm run build`, full `npm test` (32 tests), and `git diff --check` passed.
