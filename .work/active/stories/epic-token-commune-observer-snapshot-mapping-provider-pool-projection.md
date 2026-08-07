---
id: epic-token-commune-observer-snapshot-mapping-provider-pool-projection
kind: story
stage: implementing
tags: [adapter, protocol]
parent: epic-token-commune-observer-snapshot-mapping
depends_on: [epic-token-commune-observer-snapshot-mapping-envelope-construction]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Project honest per-provider pool snapshots

## Checkpoint

Map reported `/commune/pool`, `/commune/status`, `/commune/fingerprint`, and `/v1/models` state into one provider-pool upsert per provider observed by pool, status, or catalog evidence. Preserve anonymous contribution rows, native health details, and every per-contribution/per-window `CapacityReading`; never synthesize a pool-level capacity percentage or placeholder window. Keep `/status` telemetry explicitly unjoinable with `/pool` rows. Fold live models by provider into this existing kind and preserve omitted `upstreamModel` as `null`.

Generate snapshot-local anonymous contribution sub-keys from canonical gateway deployment + provider + row content + duplicate occurrence. Prefix them `local:` and label them `synthesized-content-hash` / `snapshot-local`; never promote them to `ResourceIdentity` or attribution.

## Files

- `token-commune-adapter/src/snapshot_projection.ts`
- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/tests/fixtures/snapshot_projection.ts`
- `token-commune-adapter/tests/snapshot_projection.test.ts`

## Acceptance evidence

- A listed contribution with `readings: []` remains an upserted contribution with explicit `no-readings`; a contribution with only `7d` has no fabricated `5h` reading.
- Exhausted-until and auth-broken-reason remain on their native contribution rows while distribution counts agree with the rows.
- Provider projection roots contain no capacity aggregate, remaining percentage, selected window, or derived highest-5h value.
- Models retain live id/provider/availability and `upstreamModel: null`; model-only and status-only providers are represented with honest `not-reported` pool state rather than fake zero contributions.
- Anthropic and OpenAI Codex use their named probes; every other provider is explicitly `unknown/not-probed`.
- Reordering source rows is deterministic, exact duplicates get distinct labeled sub-keys, and health/reading changes cannot be mistaken for source-stable identity.

## Ordering

Depends on manifest-bound envelope construction. The member-draw checkpoint follows after the common report assembly and deterministic mapping conventions are established.
