---
id: epic-token-commune-observer
kind: epic
stage: review
tags: [adapter, protocol, ux, integration]
parent: null
depends_on: [epic-agent-operations-resource-plane]
release_binding: null
gate_origin: null
created: 2026-07-26
updated: 2026-07-26
---

# token-commune observer adapter

## Brief

Build token-commune as Patchbay's first materially non-session reference adapter and the first consumer of the operational resource plane. The adapter is outboard: it authenticates to token-commune with the operator's member-scoped or admin-scoped gateway credential, reads metadata-only pool state, and reports resource snapshots and Observations to Patchbay. LLM request and response traffic remains entirely on token-commune's gateway data plane and never traverses Patchbay.

The first delivery is deliberately read-only and independently useful. The cockpit presents provider pools, contribution health, model availability, member draw, fingerprint state, and capacity/lifecycle events alongside Pi sessions through an adapter-shaped resource projection. token-commune's own CLI and `/ui` remain independent fallbacks. The adapter must state polling, snapshot completeness, event-gap, credential-scope, and staleness behavior honestly rather than claiming an event stream or authoritative reconstruction that the upstream contract cannot supply.

## Strategic decisions

- **Is token-commune merely a candidate?** No. It is the selected second reference adapter for the current v1 direction and the concrete design-pressure system for Patchbay's resource plane; conformance evidence must still prove that claim.
- **What ships first?** A read-only observer that is useful before any admin mutation API or Elicitation flow exists.
- **Where does integration code live?** Patchbay owns the reference adapter and a consumer-owned port over token-commune's external API. Required token-commune API work is coordinated in that repository rather than coupled through its internal implementation modules.
- **How are humans represented?** Each Patchbay deployment remains personal and uses its operator's token-commune credential. The shared gateway retains upstream member/admin policy; Patchbay grants add local defense in depth rather than replacing gateway authorization.
- **How rich is the UI?** The adapter gets a purpose-built pool/resource panel composed with Patchbay's shared primitives; it is not reduced to a generic session list.

## Design decisions

Resolved during epic decomposition (2026-08-05) from a codebase + external-API mapping: the Pi adapter blueprint, the core/server adapter-ingress map, and a token-commune external API survey. These align the child features; routine/signature-level choices are deferred to each feature's design pass.

- **Snapshot tier = PARTIAL today, not authoritative.** The pitch's claim that "the gateway's state is fully queryable" is not supported by token-commune's current external API: no pool ID, no complete inventory endpoint, `/commune/pool` omits contribution IDs/owners and may return empty capacity, no atomic snapshot envelope, no completeness declaration. AUTHORITATIVE is reserved pending upstream additions. (See External collaboration boundary.)
- **Identity = composite local IDs now; stable source-issued IDs are an external prerequisite.** token-commune exposes no stable pool/member IDs; `/commune/me` returns a display name. Identities are synthesized from gateway-deployment + provider/contribution, with documented collision/durability risk and a swappable synthesis for future upstream IDs.
- **Adapter lives in patchbay's repo** as `token-commune-adapter/`, consuming token-commune's external API over the network. Matches the Pi precedent and the brief; no filesystem coupling to token-commune `packages/shared`.
- **Read-only observer keeps `ReceiveDeliveries` open** for liveness/degradation detection (the core infers adapter loss from stream drop). v1 has no operation translator; unexpected deliveries are acknowledged and failed as unsupported rather than silently ignored. This also reserves the seam for the control-attention epic.
- **Gateway credential = adapter-local, fully redacted** (0600 file / env / OS secret store), never in durable log, Observations, resource payloads, or diagnostics. The exact store is a feature-design choice in `adapter-foundation`.
- **No upstream read-scope distinction today.** Any member key reads all metadata and also authorizes inference/mutations; the adapter holds an overprivileged key now and applies Patchbay grants as local defense-in-depth. Scoped read-only credentials are an external prerequisite.

## Arc position

Depends on `epic-agent-operations-resource-plane`. It is the implementation consumer that validates resource identity, snapshots, polling/Observation ingestion, adapter credential handling, and adapter-shaped cockpit projection. `epic-token-commune-control-attention` depends on this observer and adds mutations and human-action workflows.

## Capability outline

- token-commune adapter registration, attachment evidence, lifecycle, and scoped gateway credential handling;
- gateway/provider/contribution resource discovery and stable identity mapping;
- read-only queries for pool state, personal draw, model availability, fingerprint status, and recent events;
- explicit polling-to-Observation ingestion with deduplication, gap behavior, source timestamps, and stale-state handling;
- resource snapshots at the strongest tier the upstream API can actually satisfy;
- member and admin read views governed by both upstream credentials and Patchbay grants;
- responsive token-commune resource panel and CLI projections;
- adapter conformance vectors and end-to-end tests proving reconnect, snapshot, source authentication, redaction, and adapter-failure behavior;
- a documented external API contract boundary with token-commune, including any required cursor, identity, or read-scope additions.

## Decomposition

Split by capability along the adapter's natural seams: the integration foundation (attach + manifest + credential + external API port) lands first; the projection core (endpoint state → honest PARTIAL resource snapshots) consumes it; the polling runtime (schedule + Observations + gap/staleness) drives the projection; the cockpit panel renders the flowing state; conformance evidence closes the arc. The chain is largely linear — inherent to a single cohesive adapter — but feature-design can pipeline design-of-next while implementing-current.

### Child features

- `epic-token-commune-observer-adapter-foundation` — process, attach/registration lifecycle, capability manifest (ResourceKinds + PARTIAL tiers + schemas), gateway client port, credential handling, documented external contract boundary. depends on: `[]`
- `epic-token-commune-observer-snapshot-mapping` — endpoint state → canonical identities + PARTIAL snapshot reports with honest completeness/omission. depends on: `[adapter-foundation]`
- `epic-token-commune-observer-polling-ingestion` — polling schedule + `IngestObservation` reports + PoolEvent→Observation + dedup/gap/stale-on-disconnect. depends on: `[adapter-foundation, snapshot-mapping]`
- `epic-token-commune-observer-cockpit-panel` — surface-declared resource panel + CLI projections + grant-gated views. depends on: `[snapshot-mapping, polling-ingestion]`
- `epic-token-commune-observer-conformance` — conformance vectors + real-core E2E proving honest reconnect/snapshot/gap/redaction/adapter-failure. depends on: `[adapter-foundation, snapshot-mapping, polling-ingestion, cockpit-panel]`

### Simplification arcs

- All five reuse the core's already-generic resource ingestion/reconciliation/freshness/tombstone machinery and the Pi adapter's attach/report/diagnostics/delivery-stream machinery — no new core RPC, resource enum, registry, or storage path.
- The token-commune resource space is collapsed to the minimum honest kinds rather than mirroring every upstream concept; richer kinds wait for the upstream inventory endpoint.

### Decomposition risks

- **Linear critical path** (foundation → mapping → ingestion → cockpit → conformance): limited parallelism, inherent to a single-adapter epic. Mitigation: autopilot pipelines design/implement across the chain.
- **External-API dependency**: mapping + ingestion depend on token-commune's current API shape (no stable IDs, latest-50 events, poll-only). The consumer-owned port isolates this; conformance pins the assumed behavior; stronger tiers are external prerequisites.
- **Identity durability**: composite IDs are emitted durably; a future upstream stable-ID addition requires a migration. Mitigation: identity synthesis is designed to be swappable.
- **Pitch-vs-reality gap**: the pitch assumed authoritative snapshots and a queryable-complete gateway; the external API supports only PARTIAL today. The decomposition is scoped to what is honestly buildable now; authoritative/streaming/cursor tiers are reserved pending upstream work.

## External collaboration boundary

Patchbay work items cannot own token-commune's repository state. Any gateway API additions must be scoped and delivered in token-commune's own substrate; this epic consumes only an explicit external contract, never `packages/shared` internals by filesystem coupling. The 2026-08-05 external API survey confirmed the concrete prerequisites below; the adapter consumes only what exists today and degrades honestly on the rest.

### Confirmed external prerequisites

**Lead prerequisite (operator-confirmed 2026-08-05): per-pool contributor attribution.** The operator's primary unmet need is to see *who* is contributing and *how much* (`declaredShare`) to each pool, and to identify their own contributions. `/commune/pool` lists every contribution but omits owners entirely, and no admin read endpoint adds attribution (all read endpoints require only "any member"). The observer therefore ships **honest-limited** — unattributed contribution counts/aggregates per pool — and gains a contributor view as an additive promotion once token-commune exposes, per pool, contributors with member identity + `declaredShare` + health. This subsumes the inventory-endpoint and stable-member-identity items below.

For reliable resource snapshots:
- A source-issued gateway/pool identifier stable across hostname/tailnet changes (none exists today).
- A complete contribution/provider inventory endpoint exposing stable `contribution_id`, owner reference, provider, health, declared share, contribute-only status, and latest capacity — distinguishing zero telemetry from omitted.
- A stable externally-exposed member identity (`/commune/me` returns a display name today).
- Stronger contribution-ID generation (UUID/ULID; today it is member-name + `Date.now() % 100000`, with collision risk).
- A snapshot envelope/completeness contract (revision id, server timestamp, `complete|partial` declaration, omission reasons, per-reading freshness; ideally one atomic endpoint).

For durable Observations:
- Cursor-based event retrieval (today: latest 50, no cursor/pagination/replay).
- Documented polling semantics or push delivery (SSE/webhook); defined max lag, retention, dedup id, gap behavior.
- Full lifecycle-event coverage (today `window_exhausted` and `calibration` are declared but have no production emitter).
- A history/reconciliation guarantee so a reconnecting observer can detect missed events.

For least-privilege integration:
- Scoped read-only credentials (today a member key also authorizes inference, contribution registration, and fingerprint mutation).
- Explicit member-vs-admin read policy and any required redaction.
- Documented credential issuance/rotation/revocation procedure (today bootstrap keys are local/out-of-band).

## Scope boundaries

- No admin mutations, contribution approval, decree changes, or fingerprint acceptance in this epic.
- No OAuth/device-flow secret transport through Patchbay.
- No multi-human shared Patchbay authority domain.
- No LLM traffic proxying, prompt/response capture, routing decisions, or allocation policy in Patchbay.
- No claim that polling is streaming or that recent-event reads repair unlimited history.

## Simplification opportunity

Reuse Patchbay's adapter lifecycle, query lifecycle, Observation ingestion, resource snapshot path, redaction rules, and presentation primitives. Keep token-commune policy in the gateway and avoid duplicating allocation, capacity, or role logic in Patchbay. Retain token-commune's `/ui` and CLI as boring independent fallbacks rather than replacing them.

## Mockups

The token-commune pool/resource panel is the epic's one net-new screen surface, owned by the `cockpit-panel` feature. The epic-level UI alignment pass ran (2026-08-05); `.mockups/screens/epic-token-commune-observer-cockpit-panel/option-7.html` is the **selected MVP direction** (calm per-pool list; see the feature's `## Mockups` for the locked honesty model and parked drill-down). Mockups inherit design-system tokens from `.mockups/design-system/tokens.css` and the cockpit-composition primitives from the resource-plane epic. `option-1`..`option-6` remain as exploratory iteration history.

## Extension pressure classification

- **Committed post-v0.1.0 direction:** token-commune as the second reference adapter; outboard metadata-only integration; personal per-operator deployments; a rich resource panel.
- **Reserved seams:** upstream push/webhook delivery, third-party packaging of the adapter, cross-deployment shared presence, and generic dynamic adapter UI modules.
- **Explicitly rejected for this epic:** Patchbay in token-commune's LLM data path, copying gateway policy into Patchbay, shared filesystem imports from token-commune internals, or presenting quota health as runtime-session liveness.
