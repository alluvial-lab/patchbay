---
id: epic-token-commune-observer-snapshot-mapping
kind: feature
stage: done
tags: [adapter, protocol]
parent: epic-token-commune-observer
depends_on: [epic-token-commune-observer-adapter-foundation]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-07
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
- Collapse the token-commune resource space to the minimum honest kinds (a
  provider-pool kind and a member-draw kind) rather than mirroring every
  upstream concept; defer richer kinds until the upstream inventory endpoint
  exists.

## Foundation references

- `docs/ARCHITECTURE.md` — "Operational resource plane" (resource identity,
  snapshots/revisions; resource domain health is adapter-owned payload).
- `docs/PROTOCOL.md` — snapshot tiers; `ResourceReport` snapshot vs delta mode;
  completeness omission semantics (PARTIAL omission degrades current→stale).
- `docs/SECURITY.md` — resource envelopes are metadata only and remain inside
  the canonical no-secret/no-data-plane boundary.
- `docs/VERIFICATION.md` — `ResourceSnapshotCompletenessHonesty` is
  implementation-checked; new adapter honesty tests need independent,
  mutation-sensitive oracles rather than self-defining assertions.
- `contracts/proto/patchbay/resources.proto` — `ResourceReport`,
  `ResourceViewReport`, `ResourceIdentity`, mutation variants.
- `contracts/proto/patchbay/common.proto` — `PayloadEnvelope`, `ResourceKind`,
  `ResourceId`.
- Upstream implementation seam: `token-commune-adapter/src/gateway_client.ts`,
  `identity.ts`, `resource_contract.ts`, `manifest.ts`, and the four schemas in
  `token-commune-adapter/schemas/`.

## Key design decisions (inherited)

- **Identity = composite local IDs now; stable source-issued IDs are an external
  prerequisite.** Reuse `ResourceIdentitySynthesizer`: provider pools hash the
  canonical gateway deployment + provider; member draw additionally hashes the
  member display name. Emitted resource ids remain `local:`-prefixed and
  swappable rather than being silently reinterpreted as source ids.
- **Completeness honesty:** `/commune/pool` omits contribution IDs/owners and
  may return empty capacity; `/commune/status` drops contributions without
  telemetry. The projection distinguishes a listed contribution with no
  readings from an omitted provider and never claims an authoritative-complete
  collection.
- **Member identity:** `/commune/me` returns a display name, not a stable member
  ID. Display-name changes create a new local identity; this projector cannot
  claim terminal replacement of the old identity.
- **Manifest contract is fixed:** reuse exactly
  `token-commune.provider-pool` and `token-commune.member-draw`, both PARTIAL,
  and the four `patchbay.token_commune.*.v1` schema refs. No kind, tier, or
  descriptor is re-derived here.

## Design decisions

- **Projection input models endpoint availability explicitly:** every consumed
  endpoint is `reported(value)` or `unavailable`; empty arrays remain real
  reported values. This lets one pure call express a partial poll without
  using exceptions, clocks, network state, or cached core state.
- **Provider discovery uses current provider-bearing evidence:** provider-pool
  identities are the union of providers present in reported pool rows, status
  contribution rows (plus the reported Anthropic status axis), and live model
  catalog rows. Fingerprint probes alone do not create a provider-pool identity.
  A provider discovered outside `/commune/pool` carries `not-reported` pool
  listing state, never a fabricated zero-contribution claim.
- **Models fold into provider-pool:** model availability governs the same
  provider's operator usability and the selected cockpit is per-provider.
  Keeping models as a source-state sub-envelope avoids a third ResourceKind and
  preserves the foundation manifest. Model id/provider/availability are native;
  omitted `upstreamModel` stays `null`.
- **No pool capacity aggregate:** payload and projection retain each anonymous
  contribution and its complete per-window reading array. They contain no
  provider-level `usedFraction`, remaining percentage, selected window, or
  "highest 5h" value. The cockpit may later select a labeled real 5h reading
  for display; this capability does not.
- **Health distribution is count + native rows:** the projection counts fresh,
  exhausted, and auth-broken contributions, while each exhausted row retains
  its `exhaustedUntil` and each auth-broken row retains its `reason`. The
  gateway DTO is tightened from a lossy enum to a discriminated value because
  the foundation decoder currently validates then discards those fields.
  Method names, endpoint paths, descriptors, and kind identities remain stable.
- **Anonymous contribution keys are explicitly non-authoritative:** a sub-key
  is `local:anonymous-contribution:<24-hex-digest>:<1-based-occurrence>`, where
  the digest covers gateway deployment key, provider, and canonical row
  content. Exact duplicate rows get deterministic occurrence suffixes. Each row
  labels the key `synthesized-content-hash` with `snapshot-local` stability and
  unavailable attribution. It is never a `ResourceIdentity`, never an owner,
  and may change when row content changes.
- **Current evidence emits upsert; absence relies on PARTIAL omission:** this
  projector emits no `unknown` mutation because every identity discoverable in
  current typed endpoint state can produce a schema-valid payload with explicit
  unavailable/not-reported companion slices. It emits no tombstone/replacement
  because no consumed read endpoint provides terminal retirement evidence.
  Unlisted identities are omitted from a PARTIAL snapshot view, which makes the
  existing core degrade cached payload `current → stale`.
- **Schema validation precedes protobuf construction:** the four existing JSON
  schemas remain the local semantic source. The envelope builder validates the
  complete constructed object against the selected schema, then encodes JSON
  bytes and selects content type/schema ref from `TOKEN_COMMUNE_RESOURCES`.
  Validation failure aborts the whole pure call before a report can be emitted.
- **Dispatch rationale:** direct-read only. This is a bounded private TypeScript
  package with one generated protocol seam; the gateway DTOs, schemas, manifest,
  identity implementation, core omission tests, and cockpit honesty brief
  answered the concrete unknowns. Exploratory fan-out would duplicate that map.

## Architectural choice

Three approaches were considered:

1. **One pure whole-snapshot projector with explicit endpoint states (chosen).**
   The caller provides adapter context, current endpoint results, and the
   foundation identity synthesizer; one function returns a generated
   `ResourceReport` snapshot containing both PARTIAL views. A small dedicated
   envelope module performs schema binding. This makes fixtures independent of
   polling and makes omission behavior visible in one stable seam. It costs a
   little explicit source-state vocabulary inside payloads.
2. **One projector per HTTP response, emitting live deltas.** Each successful
   fetch could immediately mutate its resource slice. That reduces latency but
   turns cross-endpoint composition, omission, and source-time ordering into
   polling-runtime concerns, and it cannot honestly express a snapshot of the
   two declared views. It also risks treating a failed companion read as a
   silently retained current value.
3. **Stateful adapter-side resource registry.** Cache endpoint results and diff
   them before building reports. This could issue explicit removals, but it
   duplicates the core's registry/reconciliation machinery and tempts the
   adapter to infer tombstones from an upstream contract that is only partial.

The whole-snapshot projector is the least irreversible option and preserves the
Ports & Adapters boundary: HTTP is already decoded at the gateway port, mapping
is pure, and the polling runtime owns when to call and emit it.

The trickiest unit is provider-pool projection. Four non-atomic endpoints carry
different provider evidence; `/status` contribution ids cannot be joined to
anonymous `/pool` rows; health has native detail; and model/fingerprint
coverage differs by provider. The design therefore makes source availability,
joinability, and contribution telemetry explicit before designing the simpler
member-draw mapping.

## Stable projection interface

**File:** `token-commune-adapter/src/snapshot_projection.ts`

```ts
import type { Timestamp } from "@bufbuild/protobuf/wkt";
import type { ResourceReport } from "@patchbay/contracts";
import type {
  GatewayFingerprints,
  GatewayMe,
  GatewayModels,
  GatewayPool,
  GatewayStatus,
} from "./gateway_client.js";
import type { ResourceIdentitySynthesizer } from "./identity.js";

export type EndpointSnapshot<T> =
  | { readonly status: "reported"; readonly value: T }
  | { readonly status: "unavailable" };

export interface TokenCommuneGatewaySnapshot {
  readonly status: EndpointSnapshot<GatewayStatus>;
  readonly pool: EndpointSnapshot<GatewayPool>;
  readonly me: EndpointSnapshot<GatewayMe>;
  readonly fingerprints: EndpointSnapshot<GatewayFingerprints>;
  readonly models: EndpointSnapshot<GatewayModels>;
}

export interface TokenCommuneSnapshotProjectionInput {
  readonly adapterId: string;
  readonly adapterGeneration: number;
  readonly observedAt: Timestamp;
  readonly identities: ResourceIdentitySynthesizer;
  readonly gateway: TokenCommuneGatewaySnapshot;
}

export class SnapshotProjectionError extends Error {
  readonly name = "SnapshotProjectionError";
  constructor(
    readonly code:
      | "invalid-context"
      | "identity-mismatch"
      | "contract-validation-failed",
  ) {
    super(`token-commune snapshot projection ${code}`);
  }
}

export function projectTokenCommuneSnapshot(
  input: TokenCommuneSnapshotProjectionInput,
): ResourceReport;
```

The function reads no clock, performs no fetch, stores no previous identity,
and emits no RPC. `polling-ingestion` supplies an explicit protobuf timestamp
and passes the returned report to `IngestObservation.resource_report`.
`adapterId` must equal every synthesized identity's outer adapter id;
`adapterGeneration` must be a positive safe integer.

A valid call always returns one snapshot report with exactly two views in
`TOKEN_COMMUNE_RESOURCES` order. Each view declares
`AdapterSnapshotSupport.PARTIAL`, even when its mutation list is empty.

## Domain contract

### Gateway health fidelity

**File:** `token-commune-adapter/src/gateway_client.ts`

```ts
export type GatewayContributionHealth =
  | { readonly state: "fresh" }
  | { readonly state: "exhausted"; readonly exhaustedUntil: string }
  | { readonly state: "auth_broken"; readonly reason: string };
```

Both `/commune/pool` rows and `/commune/status` health use this value. The
boundary continues to reject missing/invalid timestamps and reasons; it no
longer throws away validated detail or renames native `auth_broken` into a
lossy scalar.

### Provider-pool shapes

**File:** `token-commune-adapter/src/resource_contract.ts`

```ts
export interface AnonymousPoolContribution {
  readonly subKey: string;
  readonly subKeySource: "synthesized-content-hash";
  readonly subKeyStability: "snapshot-local";
  readonly attribution: "unavailable";
  readonly declaredShare: number;
  readonly health: GatewayContributionHealth;
  readonly telemetryState: "readings" | "no-readings";
  readonly capacityReadings: readonly GatewayCapacityReading[];
  readonly fingerprint: GatewayPoolFingerprint;
}

export type ContributionListing =
  | { readonly status: "reported"; readonly contributions: readonly AnonymousPoolContribution[] }
  | { readonly status: "not-reported"; readonly contributions: readonly [] }
  | { readonly status: "unavailable"; readonly contributions: readonly [] };

export type ProviderStatusTelemetry =
  | {
      readonly status: "reported";
      readonly gatewayOk: boolean;
      readonly anthropicHealth: GatewayContributionHealth | null;
      readonly joinability: "unjoinable-with-pool-rows";
      readonly contributions: readonly GatewayStatusContribution[];
    }
  | { readonly status: "not-reported" | "unavailable"; readonly contributions: readonly [] };

export type ProviderModelCatalog =
  | { readonly status: "reported"; readonly models: readonly GatewayModel[] }
  | { readonly status: "unavailable"; readonly models: readonly [] };

export type ProviderFingerprint =
  | {
      readonly status: "reported";
      readonly probe: "anthropic" | "openai-codex";
      readonly value: GatewayFingerprintState;
    }
  | {
      readonly status: "unknown";
      readonly probe: "anthropic" | "openai-codex" | null;
      readonly reason: "probe-unavailable" | "not-probed";
    };

export interface ProviderPoolPayload {
  readonly identityStrategy: "composite-local";
  readonly gatewayDeploymentKey: string;
  readonly provider: string;
  readonly contributionListing: ContributionListing;
  readonly statusTelemetry: ProviderStatusTelemetry;
  readonly modelCatalog: ProviderModelCatalog;
  readonly fingerprint: ProviderFingerprint;
  readonly limitations: {
    readonly snapshotCompleteness: "partial";
    readonly contributorAttribution: "unavailable";
    readonly contributionIdentity: "snapshot-local-synthesized";
    readonly statusPoolJoin: "unavailable";
    readonly capacityAggregation: "none";
  };
}

export interface ProviderPoolProjection {
  readonly provider: string;
  readonly contributionListing: ContributionListing;
  readonly credentialHealthCounts: {
    readonly fresh: number;
    readonly exhausted: number;
    readonly authBroken: number;
  };
  readonly totalDeclaredShare: number;
  readonly statusTelemetry: ProviderStatusTelemetry;
  readonly modelCatalog: ProviderModelCatalog;
  readonly fingerprint: ProviderFingerprint;
  readonly capacityAggregation: "none";
}
```

The projection intentionally repeats raw contribution rows because the local
cockpit decoder consumes the projection envelope, not the opaque resource
payload. It may count health and sum native `declaredShare`; it may not reduce
capacity readings. Status contribution ids/readings stay in a separately
labeled unjoinable slice and are never attached to anonymous pool rows.

Provider fingerprint mapping is exact: provider `anthropic` selects the
Anthropic probe, `openai-codex` selects Codex, and every other provider is
`unknown/not-probed`. If the fingerprint endpoint is unavailable, a supported
provider is `unknown/probe-unavailable`; no last-known probe value is copied.

Models stay in this kind rather than a separate sub-envelope/ResourceKind: they
are filtered by exact provider string, sorted deterministically by `(id,
provider)`, retain the live id and availability boolean, and retain
`upstreamModel: null` when omitted by `/v1/models`.

### Member-draw shapes

**File:** `token-commune-adapter/src/resource_contract.ts`

```ts
export interface MemberDrawPayload {
  readonly identityStrategy: "composite-local";
  readonly gatewayDeploymentKey: string;
  readonly memberDisplayName: string;
  readonly provider: string;
  readonly reports: readonly GatewayDrawReport[];
  readonly limitations: {
    readonly snapshotCompleteness: "partial";
    readonly stableMemberIdentity: "unavailable";
  };
}

export interface MemberDrawProjection {
  readonly memberDisplayName: string;
  readonly provider: string;
  readonly reports: readonly GatewayDrawReport[];
}
```

Reports are grouped by exact provider only to select the resource identity;
all same-provider rows remain in the array. `limitFraction`, `fromDecree`,
provider-native `consumedUnits`, nullable `drawUnits`, `exceeded`, `enforceable`,
and nullable `resetsAt` pass through. There is no cross-provider aggregate,
selected row, inferred calibration, or derived enforcement state.

## Completeness and null-state taxonomy

The report/output rules are deliberately narrower than the core mutation
vocabulary:

| Input evidence | Mutation | Meaning |
|---|---|---|
| Provider appears in reported pool, status, or model evidence | provider-pool `upsert` | Current typed evidence exists; unavailable companion sources are explicit inside the payload. |
| `/commune/me` reports one or more rows for `(displayName, provider)` | member-draw `upsert` | All same-provider draw rows are current evidence. |
| No current identity-bearing evidence for a prior resource | no mutation in that PARTIAL view | Core degrades a cached current resource to stale; adapter does not encode stale in domain JSON. |
| Identity is known but no classifiable payload exists | not produced by current input contract | Future identity-only upstream evidence may use `unknown`; current endpoints either produce a valid explicit source-state payload or no identity. |
| Upstream proves permanent retirement/replacement | not available in current contract | No tombstone is emitted. A future complete inventory/stable-id contract must explicitly promote this path. |
| Gateway URL/member display changes | a different synthesized upsert; old id omitted | Old cached resource becomes stale under PARTIAL semantics; no silent merge or false replacement. |

The three capacity cases remain observably distinct:

1. **Contribution present, no readings:** contribution exists in a reported
   listing with `telemetryState = "no-readings"` and
   `capacityReadings = []`. This is not zero use and not an omitted
   contribution.
2. **Readings present, 5h absent:** `telemetryState = "readings"` and the array
   contains its real windows (for example `7d`, `7d_sonnet`, `parallel`) but no
   synthetic `5h` object. Individually missing measurement fields remain
   explicit `null`.
3. **Resource stale:** the provider identity is absent from a later PARTIAL
   view. The old payload remains in the core wrapper with
   `ResourceFreshnessState.STALE`; no JSON field pretends the old readings were
   freshly observed.

An empty reported `/commune/pool` is not an authoritative empty inventory. It
produces no pool-derived providers; providers seen in other current evidence
carry `contributionListing = not-reported`, and prior identities absent from all
current evidence are omitted/staled. An unavailable endpoint is distinct from
its reported empty response.

## Envelope construction

**File:** `token-commune-adapter/src/resource_envelope.ts`

```ts
import type { PayloadEnvelope } from "@patchbay/contracts";

export type TokenCommuneResourceName = keyof typeof TOKEN_COMMUNE_RESOURCES;
export type EnvelopeRole = "payload" | "projection";

export function encodeResourceEnvelope(
  resource: TokenCommuneResourceName,
  role: EnvelopeRole,
  value: unknown,
): PayloadEnvelope;
```

The module registers the four JSON schema documents with Ajv 2020 once, keeps
Ajv as a runtime dependency, and selects a validator and descriptor through the
existing resource registry. It emits `PayloadContentType.JSON`; it never accepts
a caller-provided schema ref. Validation errors are reduced to a fixed
`contract-validation-failed` code without embedding payloads, credentials, or
arbitrary upstream messages.

JSON is deterministic because mapper objects use fixed field order and arrays
are sorted before encoding. The design does not introduce a general canonical
JSON framework: the content-hash helper canonicalizes only the explicit
anonymous contribution row shape, and the report tests compare decoded values
rather than relying on incidental object insertion order.

## Implementation units

### Unit 1: Projection contract fidelity

**Files:**

- `token-commune-adapter/src/gateway_client.ts`
- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/schemas/provider-pool-payload.schema.json`
- `token-commune-adapter/schemas/provider-pool-projection.schema.json`
- `token-commune-adapter/schemas/member-draw-payload.schema.json`
- `token-commune-adapter/schemas/member-draw-projection.schema.json`
- `token-commune-adapter/tests/gateway_client.test.ts`
- `token-commune-adapter/tests/resource_contract.test.ts`

**Story:** `epic-token-commune-observer-snapshot-mapping-projection-contract`

**Implementation notes:** preserve foundation registries and methods; replace
only the lossy health value and pre-mapping schema shapes. Schemas require all
nullable fields explicitly rather than allowing omission, and source states are
closed discriminated unions.

**Acceptance criteria:**

- [ ] Native health metadata survives decoding and schema validation.
- [ ] Schema-invalid health/readings fail at the gateway boundary.
- [ ] Existing kind/tier/descriptor manifest tests remain literal and green.
- [ ] Schema roots contain no provider-level capacity percentage.

### Unit 2: Manifest-bound envelope/report construction

**Files:**

- `token-commune-adapter/src/resource_envelope.ts`
- `token-commune-adapter/src/snapshot_projection.ts`
- `token-commune-adapter/package.json`
- `token-commune-adapter/package-lock.json`
- `token-commune-adapter/tests/snapshot_projection.test.ts`

**Story:** `epic-token-commune-observer-snapshot-mapping-envelope-construction`

**Implementation notes:** construct all domain values first, validate all four
candidate arrays/objects, and only then create the protobuf report. This keeps a
failed mapping atomic at the pure function boundary.

**Acceptance criteria:**

- [ ] Both view descriptors exactly match the manifest registry.
- [ ] Every returned report is snapshot mode with two PARTIAL views.
- [ ] Invalid context/identity/schema fails before return.
- [ ] No time, I/O, polling, or RPC dependency enters the projector.

### Unit 3: Provider-pool mapping, model fold, and fingerprint honesty

**Files:**

- `token-commune-adapter/src/snapshot_projection.ts`
- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/tests/fixtures/snapshot_projection.ts`
- `token-commune-adapter/tests/snapshot_projection.test.ts`

**Story:** `epic-token-commune-observer-snapshot-mapping-provider-pool-projection`

**Implementation notes:** form the provider union, group and canonical-sort
rows, allocate duplicate occurrence suffixes, filter source slices by exact
provider, and construct one upsert per provider. Never join `/status` ids to
pool rows.

**Acceptance criteria:**

- [ ] Raw contribution × window readings and nulls round-trip exactly.
- [ ] Health counts match rows while until/reason details remain native.
- [ ] Model-only/status-only providers use `not-reported`, not empty-as-zero.
- [ ] Fingerprint probe coverage is exact and non-probed providers are unknown.
- [ ] No capacity aggregate or fabricated model/upstream alias appears.

### Unit 4: Member-draw mapping and whole-report assembly

**Files:**

- `token-commune-adapter/src/snapshot_projection.ts`
- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/tests/fixtures/snapshot_projection.ts`
- `token-commune-adapter/tests/snapshot_projection.test.ts`

**Story:** `epic-token-commune-observer-snapshot-mapping-member-draw-projection`

**Implementation notes:** group only by provider for identity, retain duplicate
rows, and deterministically order provider mutations/reports. Empty/unavailable
`/me` yields no member upsert.

**Acceptance criteria:**

- [ ] Per-provider identities and arrays preserve every native draw field.
- [ ] `fromDecree`, nullable draw/reset, and false enforcement values survive.
- [ ] No per-member or cross-provider aggregate is emitted.
- [ ] Display-name churn creates a new identity without tombstone/replacement.

### Unit 5: Completeness and mutation-sensitive fixture evidence

**Files:**

- `token-commune-adapter/tests/fixtures/snapshot_projection.ts`
- `token-commune-adapter/tests/snapshot_projection.test.ts`
- `token-commune-adapter/tests/gateway_client.test.ts`
- `token-commune-adapter/tests/resource_contract.test.ts`

**Story:** `epic-token-commune-observer-snapshot-mapping-completeness-mutation-evidence`

**Implementation notes:** fixtures are independent expected endpoint/output
examples, not production registry values reflected back as their own oracle.
Each load-bearing honesty rule has a named mutation witness.

**Acceptance criteria:**

- [ ] Upsert/omission/no-unknown/no-tombstone taxonomy is pinned.
- [ ] No-readings, missing-5h, unavailable-source, and stale-via-omission are distinct.
- [ ] Mutants for aggregation, coercion, health loss, draw collapse, probe/model
      fabrication, descriptor swap, and tier promotion fail.
- [ ] Strict build, package tests, and `git diff --check` pass.

## Implementation order

1. `projection-contract`
2. `envelope-construction` depends on `projection-contract`
3. `provider-pool-projection` depends on `envelope-construction`
4. `member-draw-projection` depends on `provider-pool-projection`
5. `completeness-mutation-evidence` depends on `member-draw-projection`

The chain is a set of durable design/verification checkpoints, not five worker
assignments. One feature owner should normally implement the package coherently.

## Simplification

- Reuse the foundation's kinds, schema refs, manifest, identity strategy,
  generated contracts, gateway methods, and the core's PARTIAL omission logic.
- Keep one projector and one small envelope module; do not add an adapter-side
  registry, polling state, cache, generic projection framework, new RPC,
  ResourceKind, model resource, contribution resource, or fingerprint resource.
- Remove the existing lossy health scalar and derived member
  `enforcementState`; neither earns its keep because each erases native data or
  collapses multiple reports.
- Replace whole-status/whole-fingerprint duplication in every provider payload
  with provider-scoped, explicitly joined-or-unjoinable source slices.
- No existing core tests are duplicated. This feature tests the adapter's pure
  output; the downstream conformance feature owns real-core promoted evidence.

## Testing

- **Stable-interface fixtures:** one all-sources fixture exercises multiple
  providers, duplicate anonymous rows, `5h`/`7d`/`7d_sonnet`/`parallel`, all
  nullable reading fields, all health variants, model availability/null
  upstream ids, both probes, an unprobed provider, multiple draw providers, and
  duplicate same-provider draw rows. It protects the public projector seam.
- **Completeness fixtures:** reported-empty and unavailable endpoints produce
  independent expected view/mutation lists. They protect PARTIAL omission and
  ensure the adapter never emits AUTHORITATIVE, unknown, or tombstone from
  current reads.
- **Regression/mutation tests:** explicitly break one honesty rule at a time:
  add a root capacity aggregate; coerce null to zero; drop health detail; join
  status ids by ordinal; fabricate `5h`; collapse draw; claim probes for all
  providers; rewrite a model id/upstream alias; swap descriptors; or strengthen
  completeness. Each mutation must make a stable assertion fail.
- **Boundary validation tests:** malformed raw readings fail in
  `gateway_client`; a cast malformed typed object also fails constructed schema
  validation before report return. This protects fail-closed behavior without
  testing every trivial branch.
- **Identity tests:** canonical gateway/provider/member identities reuse the
  foundation tests; projection adds only deterministic snapshot-local sub-key
  tests and verifies those keys never become resource ids.
- **No low-value tests:** no per-getter, JSON key-order, generated-message, or
  every-null-combination tests. Existing schema required-field mutation loops
  expand only for the new load-bearing fields.

## Risks

- **Riskiest assumption — a composite resource can be current while one source
  slice is unavailable.** Core freshness applies to the whole upsert, not each
  nested endpoint. The payload therefore marks each slice explicitly; the
  cockpit must not present an unavailable capacity/model/fingerprint slice as
  current merely because the wrapper is current. If this proves too subtle,
  the safe fallback is to omit that provider mutation for the poll, making the
  whole cached resource stale rather than inventing per-slice freshness.
- **Anonymous sub-key churn:** content-derived keys change with health/readings
  and cannot support durable row identity. They are visibly snapshot-local and
  not used for routing, grants, tombstones, or cross-poll joins. Future stable
  source ids replace the sub-key strategy only through an explicit schema/
  migration design.
- **Provider union over/under-inclusion:** status/model evidence may name a
  provider not represented in pool rows, while a partial endpoint may omit a
  prior provider. Explicit `not-reported` state prevents zero-inventory claims;
  PARTIAL omission prevents terminal deletion. Fixtures pin both cases.
- **Schema evolution inside existing `.v1` refs:** the package is private and
  no external consumer exists, so compatibility is not earned; tightening the
  not-yet-consumed shape in place is simpler than parallel v2 contracts. The
  two downstream features must consume the landed v1 shape. Publication later
  makes descriptor evolution a compatibility decision.
- **Runtime schema packaging:** Ajv and cross-file `$ref` resolution must work
  from built `dist/`, not only source tests. The envelope checkpoint runs the
  built package tests. If JSON import/copy behavior is unreliable, precompiled
  validators are the bounded fallback; hand-copied validation rules are not.
- **No terminal retirement evidence:** URL/member-name churn can leave stale old
  identities indefinitely. That is honest under PARTIAL. Cleanup waits for an
  upstream stable inventory or explicit operator migration rather than using a
  guessed tombstone.

## Other agent review

- Invoked because: this is a protocol-bound projection with durable identities,
  source-completeness semantics, and multiple mutation-sensitive honesty rules.
- Skipped/degraded: this delegated worker surface exposes no subagent or peer
  review mechanism, so an independent design advisory pass could not be
  commissioned. Per Part IV this is non-blocking at design time; direct source,
  schema, generated-contract, core-reconciliation, and mockup-honesty evidence
  was verified instead. This is not labeled independent or cross-model review.
- Fixed/active blockers: the direct pre-mortem removed the lossy health scalar,
  removed derived member enforcement aggregation, made endpoint source state
  explicit, and prohibited inferred tombstones.
- Parked: per-slice core freshness, stable source contribution/member ids, and
  authoritative inventory remain upstream/protocol promotions rather than
  current-cycle additions.
- Rejected: per-response deltas and an adapter-side resource cache because they
  move reconciliation policy out of the core and make omission less honest.

## UI surface

This feature adds no human control surface. It is the pure data producer for the
parent epic's selected cockpit mock; no feature-level fallback mockup is needed.
The locked no-aggregate and per-provider honesty model is incorporated above.

## Extension pressure classification

- **Committed post-v0.1 direction:** pure token-commune projection into the two
  manifest-declared PARTIAL kinds; provider-scoped raw contribution/window
  readings; native health/draw/model/fingerprint metadata; schema-bound JSON
  envelopes; PARTIAL omission as stale degradation.
- **Reserved seams:** stable source pool/member/contribution ids; an identity-only
  discovery source that could justify `unknown`; explicit retirement/replacement
  evidence; AUTHORITATIVE inventory; per-source/core nested freshness; stronger
  published schema-version compatibility.
- **Explicitly rejected for this feature:** pool-level capacity percentages,
  fabricated `5h` readings, contribution attribution or joins by ordinal,
  aggregate member draw, model alias synthesis, fingerprint probes for
  unsupported providers, inferred tombstones, adapter-side durable projection
  state, and new core resource kinds/states.
- **Non-foreclosure check:** all token-commune details remain adapter-owned under
  the existing operational-resource manifest. No Pi/core enum, surface-only
  state, second operator assumption, federation key change, or parked UI/mesh
  direction is introduced.

## Implementation summary

- **Execution capability:** `openai-codex/gpt-5.6-sol` (explicit caller override), executed by one owning worker with no sub-worker fan-out. The five-story chain shared the same projection/schema/test write set, so cohesive sequential ownership avoided integration handoffs.
- Added `src/resource_envelope.ts` and `src/snapshot_projection.ts`: one pure, timestamp-injected projector over typed endpoint states, plus manifest-bound Ajv JSON validation and generated Protobuf report construction.
- Tightened gateway health decoding and the four existing `.v1` JSON contracts in place. The two existing ResourceKinds, four schema refs, and PARTIAL manifest declarations remain the single source of truth.
- Provider-pool output preserves anonymous contribution × window readings and nulls, native exhausted/auth-broken detail, unjoinable status rows, exact model ids/null upstream ids, and only the Anthropic/Codex fingerprint probes. Anonymous sub-keys are labeled snapshot-local content hashes and never become resource identities or ownership.
- Member-draw output preserves every same-provider report and all native provenance/calibration fields without aggregate enforcement state.
- Completeness is deliberately narrow: current classifiable evidence emits only upserts; absent identities are omitted from PARTIAL views; no `unknown`, tombstone, replacement, provider-level capacity percentage, or selected-window aggregate is emitted.
- **Verification:** final standalone `npm run build` passed; final `npm test` passed 32/32; `git diff --check` passed; the feature-transition worktree was clean before this item update.
- **Mutation evidence:** three production mutants were applied, observed failing, and reverted: PARTIAL→AUTHORITATIVE, same-provider draw collapse to one row, and fabricated reported fingerprint evidence for a no-probe provider. The corresponding focused tests each exited non-zero; the restored tree then passed the full suite.
- **Deviations/blockers:** none. Polling, RPC emission, cockpit code, Observations, and conformance-vector promotion remain outside this feature as designed.

## Review handoff

Effective implementation review weight is **thorough** (source: explicit caller).
Child stories close directly on green verification; the integrated feature then
runs review → receiver adjudication → fix/verify → fresh-context review until a
pass yields no receiver-confirmed material current-cycle blockers. Reviewer
findings are proposals, not authority. The active autopilot final-completion
review must receive the same `thorough` weight unchanged, and a pass is labeled
cross-model only when the harness actually selects a different model class.

## Review (thorough, 2026-08-07)

Cross-model (gpt-5.6-sol vs zai/kimi host), convergence.

- **Pass 1 (BLOCK):** 2 blockers + 1 important. (1) provider canonicalization gap — whitespace-differing providers (`"zai"` vs `" zai "`) produced duplicate `ResourceIdentity` mutations, which core ingress rejects (failing the whole poll); (2) schema permitted contradictory telemetry taxonomy (`telemetryState:"readings"` with empty `capacityReadings` and vice-versa), breaking the no-readings-vs-readings distinction; (3) time validation weaker than declared/core (Ajv accepted arbitrary date strings + out-of-range Protobuf seconds). All fixed at `d378ce6` — provider normalized at gateway ingress + duplicate-identity rejection; telemetry discriminated union (schema + TS); ajv-formats RFC 3339 + Protobuf seconds bounds. 32→34 tests; mutation-checked.
- **Pass 2 (APPROVE):** all three fixes verified correct + mutation-sensitive (5 independent mutation checks); no-aggregate rule, attribution, fingerprint honesty, non-joinability all re-confirmed intact; 34/34 tests, clean build.

Converged. Advanced to `done`.
