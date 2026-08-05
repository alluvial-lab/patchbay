---
id: epic-token-commune-observer-adapter-foundation
kind: feature
stage: drafting
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

<!-- The design pass on this feature (`/agile-workflow:feature-design`) fills in
interfaces, signatures, the credential-store choice, the manifest kind/tier/
schema declarations, and implementation units. -->
