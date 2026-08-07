---
id: epic-token-commune-observer-adapter-foundation
kind: feature
stage: implementing
tags: [adapter, protocol, integration]
parent: epic-token-commune-observer
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# token-commune adapter foundation

## Brief

Stand up the token-commune adapter as a long-lived TypeScript process, a sibling
to `pi-adapter/`, that attaches to the Rust coordination core as an
**operational-resource** adapter. This is the integration foundation every
other feature in the epic builds on. It delivers: process bootstrap and
configuration; the capability manifest declaring token-commune's `ResourceKind`s
at honest snapshot tiers with exact payload/projection schema descriptors;
attach/registration lifecycle reusing the fixed `AdapterControlService`
contract; the consumer-owned port (gateway client) over token-commune's HTTP
read API; scoped gateway-credential handling (adapter-local, fully redacted);
and the documented external API contract boundary with the full list of upstream
prerequisites that gate stronger tiers.

It does NOT cover the live polling/projection engine (snapshot-mapping +
polling-ingestion), the cockpit panel, or any mutation — those are later
features / the control-attention epic.

## Epic context

- Parent epic: `epic-token-commune-observer`
- Position in epic: **foundation feature** — declares the capability manifest
  (ResourceKinds, snapshot tiers, schemas) and the attach lifecycle. Every other
  feature depends on its declared kinds, its gateway client port, and its
  credential handling.

## Simplification opportunity

- Reuse the Pi adapter's reusable machinery unchanged: ConnectRPC
  `AdapterControlService` client, attach/evidence/token flow, single-flight
  reattachment, `IngestObservation` auth/retry wrapper, `ReceiveDeliveries`
  subscription + reconnect policy, abort/signal lifecycle, idempotent disposal,
  local JSONL diagnostics (rotation/queue/redaction/non-interference), and the
  core-diagnostics forwarding architecture with a token-commune-specific
  diagnostic-code registry.
- Do NOT carry over Pi's `SessionRegistry`, `PiSession`, transcript projection,
  model/activity reports, or `DeliveryTranslator` — they are structural examples
  only.

## Foundation references

- `docs/ARCHITECTURE.md` — "Adapter plane", "Adapter registration and
  lifecycle", "Operational resource plane" (adapter-shaped projections compose
  above the canonical wrapper).
- `docs/SPEC.md` — "v1 adapter proof" (Pi + token-commune prove the boundary);
  "Personal deployments compose communal services"; "The data plane stays
  outboard".
- `docs/PROTOCOL.md` — adapter capability manifest, registration lifecycle,
  snapshot tiers (authoritative/partial/none).
- `docs/SECURITY.md` — adapter attachment material and gateway credentials are
  on the no-log/no-diagnostic list; loopback/colocated posture in v0.x.
- Blueprint: `pi-adapter/src/main.ts`, `pi-adapter/src/core_client.ts`,
  `pi-adapter/src/adapter_diagnostics.ts`, `pi-adapter/src/core_diagnostics_forwarder.ts`.
- Proto: `contracts/proto/patchbay/adapter_control.proto` (`Attach`,
  `IngestObservation`, `ReceiveDeliveries`, `ReportDiagnostics`),
  `contracts/proto/patchbay/adapter.proto` (`AdapterCapability`,
  `ResourceCapability`, `ResourceProjectionContract`).

## Key design decisions (inherited from epic `## Design decisions`)

- **Snapshot tier = PARTIAL today.** token-commune's external API has no
  completeness contract, no pool ID, omits contribution IDs/owners from
  `/commune/pool`, and provides no atomic snapshot envelope. AUTHORITATIVE is
  reserved pending upstream additions (see External collaboration boundary).
- **Adapter lives in patchbay's repo** as `token-commune-adapter/`, consuming
  token-commune's external API over the network (no filesystem coupling to
  `packages/shared`).
- **Read-only observer keeps `ReceiveDeliveries` open** for liveness/degradation
  detection (the core infers adapter loss from stream drop). v1 has no operation
  translator; an unexpected delivery is acknowledged and failed as unsupported
  rather than silently ignored. This also reserves the seam for the
  control-attention epic.
- **Gateway credential = adapter-local, fully redacted** (0600 file / env / OS
  secret store, decided in this feature's design pass), never in durable log,
  Observations, resource payloads, or diagnostics.

## External contract boundary (consumer-owned port)

The gateway client port targets these current token-commune read endpoints (any
valid member key): `/commune/status`, `/commune/pool`, `/commune/me`,
`/commune/events` (latest 50, no cursor), `GET /commune/fingerprint` (Anthropic
+ Codex only), `/v1/models`. Auth is `Authorization: Bearer <member-key>` or
`x-api-key`. The port isolates the rest of the adapter from upstream shape
changes.

This feature documents the external prerequisites (stable pool/member IDs,
complete inventory endpoint, event cursor/replay, scoped read-only credentials,
snapshot completeness contract, full lifecycle-event coverage) recorded in the
parent epic's "External collaboration boundary"; the adapter consumes only what
exists today and degrades honestly on the rest.

## Design decisions

- **Adapter package boundary:** implement a self-contained sibling package at
  `token-commune-adapter/` and reproduce the proven Pi adapter lifecycle shape
  without importing `pi-adapter/` or extracting a shared runtime package in this
  feature. This keeps Pi stable and avoids making a generic abstraction before
  the second adapter proves the truly shared seam; a later behavior-preserving
  extraction can follow concrete duplication.
- **ResourceKind set:** declare exactly `token-commune.provider-pool` and
  `token-commune.member-draw`, both `PARTIAL`. Provider-pool owns the shared
  provider/contribution/capacity/model/fingerprint view; member-draw is separate
  because it is credential-relative state with a different identity and future
  grant/redaction pressure. Events remain Observations, not resource kinds.
- **No contribution ResourceKind:** do not create per-contribution resources.
  `/commune/pool` omits contribution ids and owners, while `/commune/status`
  exposes capacity ids that cannot be joined reliably to pool rows when a
  provider has multiple contributions. Provider-pool payloads therefore retain
  anonymous contribution rows and honest aggregate counts until upstream adds
  stable attributed inventory.
- **Identity synthesis:** use a swappable `ResourceIdentitySynthesizer` whose
  default `composite-local` implementation hashes a canonicalized gateway base
  URL into a deployment key and combines it with provider (and, for member draw,
  the `/commune/me` display name). The emitted id is prefixed `local:` so a
  future source-id strategy is explicit rather than silently reinterpreting
  durable ids. URL or display-name changes can create replacement resources;
  this limitation is carried in payload metadata and conformance work.
- **Gateway credential store:** require a regular 0600 file named by
  `PATCHBAY_TOKEN_COMMUNE_MEMBER_KEY_FILE`; do not accept the raw gateway key in
  ordinary environment configuration and do not add a platform-specific OS
  secret-store dependency. The adapter reads the file once at bootstrap,
  rejects symlinks and group/world permission bits, never logs the path or
  value, injects the value only as `Authorization: Bearer`, and registers the
  exact value with local diagnostic redaction. The credential-source port keeps
  OS secret-store promotion reversible.
- **External response boundary:** hand-own narrow decoded DTOs in Patchbay and
  validate every HTTP response from `unknown`; do not import token-commune's
  `packages/shared`. Redirects are rejected so bearer material cannot cross an
  origin, response bodies are bounded, and errors expose only endpoint,
  category, and status—not response bodies or headers.
- **Read-only capability posture:** the manifest declares no supported
  OperationKinds, `streaming_support = false`, no runtime-session tier, and no
  cancellation/replacement guarantees. `ReceiveDeliveries` still stays open as
  the attachment-liveness mechanism. Any delivered Operation is durably
  acknowledged and then rejected with `unsupported_command`; no translator or
  `running` transition exists.
- **Dispatch rationale:** direct-read design was sufficient after mapping the
  Pi process, generated adapter/resource contracts, core manifest validator,
  resource ingress, and the already-surveyed gateway endpoints. Exploratory
  fan-out would have repeated locked grounding rather than answered a distinct
  unknown.

## Architectural choice

Three approaches were considered:

1. **Self-contained sibling adapter (chosen).** Add a TypeScript package beside
   `pi-adapter/`, with token-commune-owned config, gateway port, manifest,
   diagnostics registry, core client, and process composition root. It copies
   the established control-loop architecture but carries no Pi session or
   transcript concepts. This optimizes isolation and delivery speed at the cost
   of limited lifecycle/diagnostics duplication.
2. **Extract a shared adapter-runtime package first.** Move attachment,
   diagnostics, retry, and delivery-stream machinery out of `pi-adapter/` and
   make both adapters consume it. This could reduce duplication, but it expands
   the write and regression surface before token-commune proves which details
   are genuinely generic, and would make this external-boundary feature also a
   Pi refactor.
3. **Integrate token-commune in the Rust core.** This minimizes processes but
   violates the adapter plane, places external HTTP and credential concerns in
   the trusted core, and forecloses deployment-neutral outboard adapters.

The sibling package is the least irreversible sound choice. Its seams are
consumer-owned and can later move unchanged into a shared package if two real
implementations demonstrate value.

The trickiest unit is the external contract/resource-contract seam: nullable
per-contribution/per-window capacity, credential-relative draw, and missing
stable ids must become exact stable Patchbay schemas without claiming a
complete collection. It is designed first below; process wiring depends on it.

## Stable resource contract

### Resource registry

**File:** `token-commune-adapter/src/resource_contract.ts`

```ts
export const TOKEN_COMMUNE_RESOURCE_KINDS = {
  providerPool: "token-commune.provider-pool",
  memberDraw: "token-commune.member-draw",
} as const;

export const TOKEN_COMMUNE_SCHEMAS = {
  providerPoolPayload: "patchbay.token_commune.provider_pool.payload.v1",
  providerPoolProjection: "patchbay.token_commune.provider_pool.projection.v1",
  memberDrawPayload: "patchbay.token_commune.member_draw.payload.v1",
  memberDrawProjection: "patchbay.token_commune.member_draw.projection.v1",
} as const;

export type TokenCommuneResourceKind =
  (typeof TOKEN_COMMUNE_RESOURCE_KINDS)[keyof typeof TOKEN_COMMUNE_RESOURCE_KINDS];

export interface ProviderPoolPayload {
  identityStrategy: "composite-local";
  gatewayDeploymentKey: string;
  provider: string;
  contributions: readonly GatewayPoolContribution[];
  models: readonly GatewayModel[];
  fingerprint: GatewayFingerprintSummary;
  sourceStatus: GatewayStatusSummary;
  limitations: {
    snapshotCompleteness: "partial";
    contributorAttribution: "unavailable";
    contributionIdentity: "unjoinable";
  };
}

export interface ProviderPoolProjection {
  provider: string;
  contributionCount: number;
  totalDeclaredShare: number;
  healthCounts: { fresh: number; exhausted: number; authBroken: number };
  anonymousContributions: readonly GatewayPoolContribution[];
  models: readonly GatewayModel[];
  fingerprint: GatewayFingerprintSummary;
}

export interface MemberDrawPayload {
  identityStrategy: "composite-local";
  gatewayDeploymentKey: string;
  memberDisplayName: string;
  provider: string;
  reports: readonly GatewayDrawReport[];
  limitations: {
    snapshotCompleteness: "partial";
    stableMemberIdentity: "unavailable";
  };
}

export interface MemberDrawProjection {
  memberDisplayName: string;
  provider: string;
  reports: readonly GatewayDrawReport[];
  enforcementState: "unknown" | "within-limit" | "exceeded";
}
```

**Files:**

- `token-commune-adapter/schemas/provider-pool-payload.schema.json`
- `token-commune-adapter/schemas/provider-pool-projection.schema.json`
- `token-commune-adapter/schemas/member-draw-payload.schema.json`
- `token-commune-adapter/schemas/member-draw-projection.schema.json`

The four Draft 2020-12 JSON schemas are the local semantic source for the JSON
bytes emitted later by snapshot mapping. `TOKEN_COMMUNE_SCHEMAS` is the single
registry from which manifest descriptors and payload-envelope construction take
schema refs. Nullable upstream fields (`usedFraction`, `usedUnits`,
`limitUnits`, `resetsAt`, `drawUnits`, fingerprint hold/capture fields) remain
nullable; missing telemetry is never normalized to zero. Provider ids remain
validated non-empty strings rather than a copied closed enum so an upstream
provider addition does not require a false protocol change.

### Identity synthesis

**File:** `token-commune-adapter/src/resource_identity.ts`

```ts
export interface SynthesizedResourceIdentity {
  adapterId: string;
  resourceKind: TokenCommuneResourceKind;
  resourceId: string;
}

export interface ResourceIdentitySynthesizer {
  providerPool(provider: string): SynthesizedResourceIdentity;
  memberDraw(memberDisplayName: string, provider: string): SynthesizedResourceIdentity;
}

export function createCompositeLocalIdentitySynthesizer(input: {
  adapterId: string;
  gatewayBaseUrl: URL;
}): ResourceIdentitySynthesizer;
```

The implementation canonicalizes an `http:` or `https:` gateway URL after
rejecting username/password/query/fragment, normalizes the base path and
trailing slash, and SHA-256 hashes it to `gatewayDeploymentKey`. Resource ids
use `local:provider-pool:<deployment-hash>:<provider>` and
`local:member-draw:<deployment-hash>:<member-hash>:<provider>` with percent-safe
provider text and a hash of the display name. `adapter_id` remains the outer
identity dimension; the synthesizer never substitutes labels for that verified
id. A future source-id implementation can satisfy the same interface and emit
`source:` ids, with replacement/migration designed explicitly.

**Acceptance criteria:**

- [ ] One registry owns both ResourceKinds and all four schema refs.
- [ ] Both manifest declarations are `PARTIAL`, JSON, and target
      `OPERATIONAL_RESOURCE`; session snapshot support stays unspecified.
- [ ] The four schemas reject omitted required fields and preserve every
      nullable capacity/draw field as nullable.
- [ ] Identity is deterministic for canonical-equivalent URLs and differs when
      adapter, gateway deployment, kind, provider, or member display changes.
- [ ] No resource or projection schema can contain credentials, prompts,
      responses, provider onboarding material, or arbitrary diagnostic data.

## Implementation units

### Unit 1: Package bootstrap and fail-fast configuration

**Files:**

- `token-commune-adapter/package.json`
- `token-commune-adapter/tsconfig.json`
- `token-commune-adapter/src/config.ts`
- `token-commune-adapter/src/main.ts`

**Story:** `epic-token-commune-observer-adapter-foundation-contract-foundation`

```ts
export interface TokenCommuneAdapterConfig {
  coreAddress: string;
  adapterId: string;
  adapterGeneration: number;
  authorityDomainId: string;
  attachmentEvidence: string;
  gatewayBaseUrl: URL;
  gatewayCredentialFile: string;
  pollIntervalMs: number;
  diagnosticPath: string;
}

export function loadTokenCommuneAdapterConfig(
  env?: NodeJS.ProcessEnv,
): TokenCommuneAdapterConfig;
```

Environment keys are `PATCHBAY_CORE_ADDR`,
`PATCHBAY_ADAPTER_ATTACHMENT_SECRET`, `PATCHBAY_ADAPTER_ID` (default
`token-commune`), `PATCHBAY_ADAPTER_GENERATION` (default `1`),
`PATCHBAY_AUTHORITY_DOMAIN_ID` (default `default`),
`PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL`,
`PATCHBAY_TOKEN_COMMUNE_MEMBER_KEY_FILE`,
`PATCHBAY_TOKEN_COMMUNE_POLL_INTERVAL_MS` (default `30000`, positive safe
integer), and `PATCHBAY_ADAPTER_LOG` (existing XDG-state fallback). URL,
generation, cadence, ids, and required material fail before any network or log
work. The polling feature consumes the cadence but this feature only validates
and carries it.

**Acceptance criteria:**

- [ ] The package builds under Node 22/strict TypeScript and depends only on
      generated Patchbay contracts plus ConnectRPC/Protobuf runtime libraries.
- [ ] Malformed or incomplete config fails before credential or network access;
      errors contain environment key names but no values.
- [ ] `main.ts` installs one abort controller for SIGINT/SIGTERM and disposes
      process resources idempotently.

### Unit 2: Capability manifest and schema descriptors

**Files:**

- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/src/resource_identity.ts`
- the four `token-commune-adapter/schemas/*.schema.json` files listed above
- `token-commune-adapter/tests/resource_contract.test.ts`

**Story:** `epic-token-commune-observer-adapter-foundation-contract-foundation`

```ts
export function tokenCommuneCapabilityManifest(): AdapterCapability;
```

The manifest is exact:

- target categories: `[OPERATIONAL_RESOURCE]`;
- supported OperationKinds: `[]`;
- resource capabilities: both registry kinds at `PARTIAL` with their exact JSON
  payload and projection descriptors;
- `streamingSupport`, `cancellationSupport`, and
  `sessionReplacementSupport`: `false`;
- `sessionSnapshotSupport`: `UNSPECIFIED`;
- idempotency strength: `NONE` because this adapter declares no executable
  Operations;
- attachment: `configured-local-material`, empty redacted descriptor, binary
  descriptor content type;
- known failures: `UNSUPPORTED_COMMAND`, `ADAPTER_UNAVAILABLE`,
  `TRANSPORT_TIMEOUT`, `EXECUTION_FAILED`;
- diagnostic codes: values from the token-commune diagnostic registry.

**Acceptance criteria:**

- [ ] The generated manifest satisfies the core's fresh-attach validator and has
      no runtime-session category or resource declaration mismatch.
- [ ] Adding a ResourceKind or schema ref requires changing the one registry;
      tests derive expectations from it instead of restating a second list.
- [ ] No mutation/query OperationKind is declared merely because the delivery
      stream is open.

### Unit 3: 0600 credential source and diagnostic redaction

**Files:**

- `token-commune-adapter/src/gateway_credential.ts`
- `token-commune-adapter/src/adapter_diagnostics.ts`
- `token-commune-adapter/src/core_diagnostics_forwarder.ts`
- `token-commune-adapter/tests/gateway_credential.test.ts`
- `token-commune-adapter/tests/adapter_diagnostics.test.ts`
- `token-commune-adapter/tests/core_diagnostics_forwarder.test.ts`

**Story:** `epic-token-commune-observer-adapter-foundation-credential-diagnostics`

```ts
export interface GatewayCredential {
  apply(headers: Headers): void;
  redactionSecrets(): readonly string[];
  dispose(): void;
}

export async function loadGatewayCredential(path: string): Promise<GatewayCredential>;

export const TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES = {
  "adapter.started": "token_commune_adapter_started",
  "adapter.stopping": "token_commune_adapter_stopping",
  "adapter.attach.failed": "token_commune_adapter_attach_failed",
  "credential.load.failed": "token_commune_credential_load_failed",
  "gateway.auth.failed": "token_commune_gateway_auth_failed",
  "gateway.request.failed": "token_commune_gateway_request_failed",
  "gateway.response.invalid": "token_commune_gateway_response_invalid",
  "delivery.subscription.failed": "token_commune_delivery_subscription_failed",
  "delivery.subscription.retrying": "token_commune_delivery_subscription_retrying",
  "delivery.unsupported": "token_commune_delivery_unsupported",
} as const;
```

Credential loading uses `lstat` followed by an open/read/stat consistency check,
requires a regular non-symlink file with no group/world permission bits, accepts
one trimmed trailing newline, rejects empty/multiline material, and never embeds
path or key in thrown/logged details. `GatewayCredential.apply` is the only code
that constructs the bearer header. Disposal drops references (while documenting
that JavaScript strings cannot promise memory zeroization).

The diagnostics files retain the Pi architecture: bounded/rotating 0600 JSONL,
non-interference, exact-secret plus pattern redaction, structural local records,
and bounded non-retrying forwarding. Token-commune diagnostics use adapter or
resource context only; they have no arbitrary metadata/message/body/header/path
field. The manifest and forwarder share the one code registry above.

**Acceptance criteria:**

- [ ] Symlink, non-regular, empty, multiline, and group/world-readable key files
      fail closed with fixed non-secret errors.
- [ ] Key, bearer header, attachment evidence, and credential path are absent
      from local JSONL, forwarded reports, errors, snapshots, and test output.
- [ ] Diagnostic sink/forwarder failure cannot change attach, gateway, delivery,
      or disposal outcomes; forwarding is bounded, rate-limited, and no-retry.

### Unit 4: Consumer-owned gateway client port

**File:** `token-commune-adapter/src/gateway_client.ts`

**Story:** `epic-token-commune-observer-adapter-foundation-gateway-client`

```ts
export interface TokenCommuneGatewayClient {
  getStatus(signal?: AbortSignal): Promise<GatewayStatus>;
  getPool(signal?: AbortSignal): Promise<GatewayPool>;
  getMe(signal?: AbortSignal): Promise<GatewayMe>;
  getEvents(signal?: AbortSignal): Promise<GatewayEventsPage>;
  getFingerprints(signal?: AbortSignal): Promise<GatewayFingerprints>;
  getModels(signal?: AbortSignal): Promise<GatewayModels>;
}

export type GatewayErrorKind =
  | "unauthorized"
  | "forbidden"
  | "transport"
  | "timeout"
  | "http"
  | "invalid-response";

export class GatewayClientError extends Error {
  readonly kind: GatewayErrorKind;
  readonly endpoint: GatewayEndpoint;
  readonly status?: number;
}

export function createHttpTokenCommuneGatewayClient(options: {
  baseUrl: URL;
  credential: GatewayCredential;
  fetch?: typeof globalThis.fetch;
  maxResponseBytes?: number;
}): TokenCommuneGatewayClient;
```

Patchbay-owned DTOs preserve the consumed fields:

- status: `ok`, Anthropic contribution health, and capacity snapshots;
- pool: provider, declared share, contribution health, nullable multi-window
  capacity readings, and safe fingerprint summary;
- me: display name and provider draw reports (`limitFraction`, `fromDecree`,
  `consumedUnits`, nullable `drawUnits`, `exceeded`, `enforceable`, nullable
  `resetsAt`);
- events: latest-only page (`events`, `historyMode: "latest-50-no-cursor"`) with
  event id/time/kind/provider/nullable contribution id/message;
- fingerprints: Anthropic/Codex safe state summary (template source, capture
  time/presence, hold reason/time/diff presence), deliberately discarding raw
  fingerprint templates/captures at this boundary;
- models: id, provider, surface, upstream model, context window, max tokens,
  reasoning, availability. Model ids are opaque data; the adapter never invents
  or rewrites aliases.

Every method performs GET with `redirect: "error"`, `accept: application/json`,
and bearer auth, bounds bytes before JSON parse, validates the decoded object,
and returns immutable normalized data. It never sends the key as a query
parameter or `x-api-key`. HTTP/parse errors do not retain response bodies or
headers.

**Acceptance criteria:**

- [ ] A mock gateway proves every exact path, GET method, bearer header, abort
      propagation, redirect rejection, and response-size bound.
- [ ] Runtime decoders accept real nullable/multi-window shapes and reject
      unknown/missing discriminants, non-finite/out-of-range fractions, invalid
      timestamps, and malformed arrays before mapping code sees them.
- [ ] Duplicate provider rows remain explicit anonymous rows; the client does
      not invent a join between `/status` contribution ids and `/pool` rows.
- [ ] Real ids such as `gpt-5.5`, `gpt-5.3-codex-spark`,
      `claude-sonnet-4-5`, `token-commune/glm-5`, and
      `token-commune/kimi-for-coding` pass through unchanged; unsupported
      `gpt-5.6` aliases are not introduced.

### Unit 5: Core attachment client and reattachment lifecycle

**File:** `token-commune-adapter/src/core_client.ts`

**Story:** `epic-token-commune-observer-adapter-foundation-attachment-lifecycle`

```ts
export interface CoreClientOptions {
  coreAddress: string;
  adapterId: string;
  authorityDomainId: string;
  attachmentEvidence: string;
}

export class PatchbayCoreClient {
  constructor(options: CoreClientOptions, diagnostics?: AdapterDiagnostics);
  setDiagnostics(diagnostics: AdapterDiagnostics): void;
  attach(adapterGeneration: number): Promise<EventId>;
  acknowledgeDelivery(operation: Operation, deliveryEventId?: EventId): Promise<EventId | undefined>;
  rejectUnsupported(operation: Operation): Promise<EventId | undefined>;
  receiveDeliveries(cursor: bigint, signal?: AbortSignal): AsyncIterable<Delivery>;
  reportDiagnostic(report: AdapterDiagnosticReport, signal?: AbortSignal): Promise<AdapterDiagnosticReportResult>;
}
```

This is the Pi client pattern narrowed to token-commune: ConnectRPC transport,
attachment evidence/token interceptor, generated registration message with
`tokenCommuneCapabilityManifest()`, token-required attach success,
single-flight same-generation reattach on `Unauthenticated`, one retry for
post-attach calls/streams, and best-effort diagnostics bypass. It omits session,
transcript, running, and result helpers. Attachment descriptor bytes stay empty;
attachment evidence is sent only in its required transport/request locations.

**Acceptance criteria:**

- [ ] Attach carries exact adapter/domain/generation/manifest evidence and
      requires both accepted result and issued attachment token.
- [ ] Concurrent auth failures trigger one same-generation reattach; a newer
      token fences stale retries.
- [ ] Diagnostic reporting neither refreshes auth nor competes recursively with
      the control loop.

### Unit 6: Process lifecycle and unsupported-delivery stream

**File:** `token-commune-adapter/src/main.ts`

**Stories:**

- `epic-token-commune-observer-adapter-foundation-attachment-lifecycle`
- `epic-token-commune-observer-adapter-foundation-unsupported-delivery-loop`

```ts
export interface AdapterProcessOptions extends TokenCommuneAdapterConfig {
  gateway: TokenCommuneGatewayClient;
  diagnostics?: AdapterDiagnostics;
  forwardDiagnostics?: boolean;
}

export class AdapterProcess {
  constructor(options: AdapterProcessOptions);
  start(): Promise<void>;
  run(signal?: AbortSignal): Promise<void>;
  dispose(): Promise<void>;
}
```

`start` attaches before reporting started. `run` maintains one delivery stream,
retains the last acknowledged delivery LSN, treats a clean finite stream end as
`Unavailable`, and reconnects only the same retryable Connect failures used by
Pi. For each delivery it requires an Operation, records only safe ids/enums,
acknowledges the delivery, advances the cursor, and reports
`UNSUPPORTED_COMMAND`; it never reports `running` or invokes the gateway. The
gateway is composed now so later polling consumes the stable port, but no poll
is started in this feature. `dispose` aborts the stream and idempotently flushes
and closes diagnostics.

**Acceptance criteria:**

- [ ] An idle `ReceiveDeliveries` remains open until abort; finite completion is
      retried rather than treated as healthy liveness.
- [ ] An unexpected committed Operation transitions through acknowledgement to
      canonical `unsupported_command` rejection exactly once and advances the
      local cursor only after acknowledgement succeeds.
- [ ] Stream replacement/unauthenticated reconnect does not re-acknowledge
      already acknowledged delivery history.
- [ ] Signal shutdown and repeated disposal leave no active RPC, timer, file
      handle, or diagnostic drain.

### Unit 7: Bounded integration evidence

**Files:**

- `token-commune-adapter/tests/core_client.test.ts`
- `token-commune-adapter/tests/gateway_client.test.ts`
- `token-commune-adapter/tests/main.test.ts`
- `token-commune-adapter/tests/e2e.test.ts`

**Story:** `epic-token-commune-observer-adapter-foundation-unsupported-delivery-loop`

The E2E test starts the real Rust core with disposable storage, attaches the
adapter, reads back the durable registration manifest, opens the idle delivery
stream, submits one adapter-targeted committed `query` Operation that the
read-only adapter does not declare, and proves acknowledgement plus
`unsupported_command` terminalization and clean shutdown. Snapshot mapping,
polling, cockpit, and promoted conformance vectors remain downstream work.

**Acceptance criteria:**

- [ ] `npm test` builds and runs unit/interface tests plus one serial real-core
      lifecycle test.
- [ ] The durable registration contains exactly the two PARTIAL resource kinds,
      four descriptors, empty OperationKind set, and redacted attachment
      descriptor.
- [ ] The test fixture scans diagnostics/core-visible payloads and finds neither
      gateway key nor attachment evidence.

## External API contract boundary

The consumer-owned port uses only these network endpoints, all GET and all
bearer-authenticated with the operator's member/admin key:

| Endpoint | Consumed contract | Known limitation |
|---|---|---|
| `/commune/status` | gateway status, Anthropic health, capacity snapshots | capacity ids do not provide pool-row ownership/join semantics |
| `/commune/pool` | provider rows, declared share, health, nullable per-window capacity, fingerprint summary | no pool id, contribution id, owner, completeness envelope, or atomic revision |
| `/commune/me` | display name and provider draw reports | display name is not stable identity; duplicate provider rows are possible |
| `/commune/events` | latest event page | latest 50 only; no cursor, pagination, replay, or gap repair |
| `/commune/fingerprint` | Anthropic and Codex watchdog state | only those providers; raw templates/captures are discarded by this adapter port |
| `/v1/models` | advertised model metadata and current availability | point-in-time list; model ids are opaque and aliases are not synthesized |

Authentication uses `Authorization: Bearer <member-key>` exclusively even
though the current gateway also accepts `x-api-key`. The adapter sends no LLM
request/response traffic and consumes no token-commune filesystem module or
provider credential.

### Confirmed external prerequisites

The adapter remains honest-limited until token-commune supplies these external
contracts. They are coordination inputs for token-commune's own repository, not
Patchbay implementation children:

1. **Lead: per-pool contributor attribution** — each pool needs member identity,
   `declaredShare`, contribution health, and a way to identify the current
   member's contributions. This subsumes complete inventory and stable member
   identity pressure.
2. A source-issued gateway/pool id stable across hostname or tailnet changes.
3. A complete contribution/provider inventory endpoint with stable
   `contribution_id`, owner reference, provider, health, declared share,
   contribute-only flag, latest capacity, and an omitted-vs-zero distinction.
4. A stable externally exposed member id; `/commune/me` currently exposes only
   a display name.
5. Collision-resistant source contribution ids (UUID/ULID rather than the
   current member-name/time construction).
6. A snapshot envelope with revision id, server timestamp,
   `complete|partial`, omission reasons, per-reading freshness, and preferably
   one atomic inventory response.
7. Cursor-based event retrieval with pagination/replay, documented retention,
   dedup id, gap behavior, and either polling lag semantics or push delivery.
8. Full lifecycle-event emission, including currently declared but un-emitted
   `window_exhausted` and `calibration`, plus history/reconciliation guarantees.
9. Scoped read-only credentials, explicit member-vs-admin read/redaction policy,
   and documented issuance/rotation/revocation. Today any member key can read all
   metadata and also authorize inference and mutations.

AUTHORITATIVE promotion requires an explicit upstream contract change, manifest
registry update, mapping/conformance evidence, and protocol-change review; it
cannot arise from better local polling.

## Implementation order

1. `contract-foundation` — package/config, ResourceKind/schema registry, JSON
   schemas, identity strategy, and exact manifest.
2. `credential-diagnostics` — 0600 credential source plus local/core diagnostic
   redaction and code registry.
3. `gateway-client` — authenticated, bounded, runtime-validated HTTP port.
4. `attachment-lifecycle` — narrowed core client and process composition.
5. `unsupported-delivery-loop` — long-lived delivery liveness, unsupported
   outcome, real-core evidence, and shutdown verification.

## Simplification

- Reuse the Pi architecture without importing any Pi session/runtime modules and
  without changing Pi behavior in this feature.
- Keep only two honest resource kinds; events are Observations and fingerprint,
  models, anonymous contributions, and draw stay projections rather than new
  core categories.
- Do not introduce a generic adapter-runtime package, HTTP framework, generated
  upstream client, operation translator, poll scheduler, second persistence
  store, or credential database.
- No existing tests or code are removed. A future shared-runtime extraction is
  considered only after the sibling implementation demonstrates stable exact
  duplication.

## Testing

- **Manifest/identity interface tests** protect the stable seams consumed by
  snapshot mapping and polling: exact kinds/tiers/descriptors, canonical URL
  identity, and collision dimensions.
- **Credential/redaction regression tests** protect the highest-consequence
  boundary: permission checks and absence of gateway/attachment secrets from
  every diagnostic/core-visible representation.
- **Gateway client contract tests** protect the external API boundary and its
  nullable/multi-window behavior using a fake `fetch`; they avoid asserting
  private parser structure.
- **Core/process tests** protect attach token/reattach/cursor behavior and the
  read-only delivery outcome. One real-core E2E test proves the generated
  contract seam; broader conformance remains in the epic's conformance feature.
- No test is added for trivial getters, enum indexing, or each diagnostic event.
  No existing test is obsolete in this new sibling package.

## Risks

- **Riskiest assumption — upstream shape stability:** token-commune has no
  versioned OpenAPI/schema contract. A valid upstream change can fail decoding
  and stop a polling cycle. Fallback: fail closed, emit a bounded diagnostic,
  preserve core cached state as stale, and update only the gateway adapter/port.
- **Identity churn:** gateway URL or member display-name change yields new local
  ids and later mapping must tombstone/replace rather than merge silently.
  Fallback: the strategy interface admits source ids; payloads expose the local
  strategy and limitations.
- **Ambiguous contribution joins:** same-provider rows cannot be safely matched
  between endpoints. Fallback: anonymous provider aggregates only; no ordinal or
  `/status` id is promoted to durable contribution identity.
- **Plaintext local secret:** 0600 protects against accidental cross-user read,
  not a compromised host/account. Fallback: gateway key rotation/revocation;
  OS-secret-store implementations can replace the credential source later.
- **Lifecycle duplication drift:** copied Pi patterns may diverge. Fallback:
  interface/E2E evidence pins behavior now; extract a shared package only after
  both implementations expose an exact common contract.
- **No heartbeat:** a held-open delivery stream detects closure but not every
  black hole. Heartbeat/last-report-age policy remains the protocol's named
  reserved seam and is not fabricated here.

## Other agent review

- Invoked because: external HTTP/credential boundary, durable identity choice,
  multi-window nullable capacity, and manifest/conformance implications warrant
  independent scrutiny.
- Skipped/degraded: this delegated worker surface exposes no subagent or peer
  review mechanism, so no independent design pass could be commissioned. The
  design proceeded non-blockingly with direct contract/source verification as
  allowed by the advisory policy; this is not labeled cross-model.
- Fixed/active blockers: none found in the direct pre-mortem.
- Parked/rejected proposals: none; downstream mapping, polling, cockpit, and
  conformance remain explicitly outside this feature.

## UI surface

This feature adds no human control surface. The parent epic's selected cockpit
mock belongs to `epic-token-commune-observer-cockpit-panel`; no fallback mockup
is needed here.

## Extension pressure classification

- **Committed post-v0.1 direction:** outboard read-only token-commune adapter;
  exact provider-pool/member-draw ResourceKinds; PARTIAL tiers; local JSON
  projection contracts; configured-local attachment; held-open delivery stream.
- **Reserved seams:** source-issued gateway/pool/member/contribution ids,
  AUTHORITATIVE tiers, OS secret-store credential source, upstream push/cursor
  delivery, future read-only query or control Operation declarations, and a
  shared adapter-runtime package after demonstrated reuse.
- **Explicitly rejected for this feature:** embedding gateway HTTP/credentials
  in the Rust core, importing token-commune repository internals, per-row fake
  contribution identities, claiming upstream streaming/authoritative state, or
  loading adapter-provided renderer code.

## Review handoff

Effective implementation review weight is **thorough** (source: explicit caller).
Feature review and the autopilot final-completion review must receive that value
unchanged. Reviewer findings are proposals for receiver adjudication; a pass is
cross-model only when the harness actually selects a different model class.
