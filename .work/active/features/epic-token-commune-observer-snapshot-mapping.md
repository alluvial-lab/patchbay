---
id: epic-token-commune-observer-snapshot-mapping
kind: feature
stage: drafting
tags: [adapter, protocol]
parent: epic-token-commune-observer
depends_on: [epic-token-commune-observer-adapter-foundation]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# token-commune resource snapshot mapping

## Brief

The **projection capability**: given fetched token-commune endpoint state,
produce honest resource snapshot reports for the ResourceKinds declared in the
manifest. This feature owns the domain mapping — token-commune's pool/provider/
contribution/model/fingerprint/draw state → canonical `(adapter_id,
resource_kind, resource_id)` identities + PARTIAL snapshot reports with
payload/projection envelopes matching the manifest schemas. It is the pure
projection function (input = gateway state, output = `ResourceReport` snapshot);
the polling loop that drives it lives in `polling-ingestion`.

It delivers: stable composite resource-identity synthesis (token-commune exposes
no stable pool/member IDs and `/commune/me` returns a display name, so identities
are synthesized from gateway-deployment + provider/contribution with documented
collision/durability risk); per-kind snapshot materialization at the declared
PARTIAL tier; honest completeness/omission semantics (distinguish zero-telemetry
from an omitted resource; never claim authoritative); and payload/projection
envelope construction bound to the manifest schema descriptors.

It does NOT cover the polling schedule, event Observations, or gap/stale runtime
(`polling-ingestion`), or the cockpit.

## Epic context

- Parent epic: `epic-token-commune-observer`
- Position in epic: **projection core** — consumes the manifest kinds + gateway
  client from `adapter-foundation`; produces the `ResourceReport` snapshots that
  `polling-ingestion` emits and `cockpit-panel` renders.

## Simplification opportunity

- Reuse the core's already-generic resource ingestion/reconciliation/freshness/
  tombstone/replacement machinery — this feature only constructs the report; it
  does not extend core resource semantics.
- Collapse the token-commune resource space to the minimum honest kinds (e.g.
  a provider-pool kind and a member-draw kind) rather than mirroring every
  upstream concept; defer richer kinds until the upstream inventory endpoint
  exists.

## Foundation references

- `docs/ARCHITECTURE.md` — "Operational resource plane" (resource identity,
  snapshots/revisions; resource domain health is adapter-owned payload).
- `docs/PROTOCOL.md` — snapshot tiers; `ResourceReport` snapshot vs delta mode;
  completeness omission semantics (PARTIAL omission degrades current→stale).
- `contracts/proto/patchbay/resources.proto` — `ResourceReport`,
  `ResourceViewReport`, `ResourceIdentity`, mutation variants.
- `contracts/proto/patchbay/common.proto` — `ResourceKind` (open string),
  `ResourceId`.
- External contract: token-commune `packages/shared/src/types.ts`
  (`CapacitySnapshot`, `MemberUsage`, `PoolModel`, `PoolEvent`, `Member`,
  `Contribution`).

## Key design decisions (inherited)

- **Identity = composite local IDs now; stable source-issued IDs are an external
  prerequisite.** Synthesize `(adapter_id="token-commune", gateway-deployment,
  resource_kind, resource_id)` where resource_id is a composite of
  provider/contribution. Flag durability: emitted IDs are durable; a future
  upstream stable-ID addition requires a migration. Design the synthesis to be
  swappable.
- **Completeness honesty:** `/commune/pool` omits contribution IDs/owners and
  may return empty capacity; `/commune/status` drops contributions without
  telemetry. The projection must distinguish "zero telemetry" from "omitted" and
  never synthesize an authoritative-complete view from partial reads.
- **Member identity:** `/commune/me` returns a display name, not a stable member
  ID. A member-draw resource cannot rely on a source-stable member ID today;
  record the limitation and degrade honestly.

<!-- The design pass fills in the exact ResourceKind set, identity synthesis,
schema descriptors, completeness rules, and implementation units. -->
