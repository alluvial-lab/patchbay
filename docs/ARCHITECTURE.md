# Patchbay Architecture

Patchbay separates the coordination core from control surfaces and adapters. The architecture is defined by planes so implementation modules can evolve without collapsing examples into core concepts.

## Planes

### Human control surface plane

The human control surface plane contains web, CLI, future mobile, desktop, notification, and approval surfaces. It is the lead product plane.

The first surface is a responsive web cockpit using the shared TypeScript operator domain. The CLI provides administrative and scriptable control. The future Expo app reuses the same domain and protocol client.

### Operation plane

The operation plane represents authorized control-plane requests through an actor-neutral vocabulary, while v0.1.0 admits only operator-originated Operations. It includes spawn, attach, instruct, cancel, interrupt, query, approval response, elicitation response, reconfiguration, and session-management Operations; non-operator Operation senders remain reserved seams. Reserved-but-not-validatable seams such as `agent-send` and `adapter-utility-exec` are named for non-operator routing and standalone adapter utility-exec pressure, but v0.1.0 submissions reject with `validation_failed`.

Every accepted Operation initially reuses the canonical `CommandState` registry in `docs/PROTOCOL.md` by documented refinement equivalence. Control-surface-local submission states are separate and never become durable core states. A future rename to `OperationState` must update prose, generated contracts, formal models, conformance vectors, and implementations together.

### Runtime/session plane

The runtime/session plane contains harness sessions, agent processes, shell jobs, containers, worktrees, CI tasks, or other execution contexts. Patchbay observes and controls these through adapters.

v0.1.0 commits one explicit adapter-scoped spawn boundary: `OperationKind = spawn` addresses one attached adapter before a target session exists, and the operation-aware resolver rejects runtime-session, operational-resource, fleet-supervisor, and authority-domain spawn targets before durable acceptance. Spawn variants (worktree, same-dir, session, process, cloud environment) remain payload `target_spec.shape` values from the reserved open shape registry, not per-variant OperationKinds. Fleet-default adapter selection is a reserved rework seam, and broadcasting a non-idempotent spawn is explicitly excluded. See `docs/PROTOCOL.md` for target and authority semantics and `docs/SECURITY.md` for descendant grants and revocation.

### Operational resource plane

The operational resource plane contains non-session targets whose state materially governs what an operator's agents can do or requires human action to keep agent work operating: provider-capacity pools, contribution/credential health, model availability, and similar adapter-owned resources. Resources use stable resource identity, snapshots/revisions, Observations, queries, grants, and attention without inheriting runtime-session generation or connectivity/activity semantics.

Resource domain health remains adapter-owned payload state. For example, an exhausted model contribution is not an offline runtime session. The coordination core owns durable Operations, authority, correlation, and reconciliation around a resource but does not interpret allocation policy, quota mathematics, or adapter-specific health variants. The adapter capability manifest binds each exact resource kind to its snapshot tier and payload/domain-projection schemas; a local compositor may interpret the domain projection only inside the canonical Patchbay wrapper.

The core's revisioned `ResourceRegistry` is a projection separate from the
runtime-session registry while sharing the authority-domain durable log and
composite target resolver. Authenticated typed snapshot/delta reports normalize
to atomic `RESOURCE_STATE` events; live catch-up and restart replay fold the same
event. The projection owns active membership, current/stale/unknown cache
freshness, per-adapter-kind completeness/revision, terminal exact-identity
tombstones, explicit replacement links, and one domain-qualified cursor for the
highest contiguous authority-domain log prefix it has observed. Every known
durable event kind advances that cursor; only `RESOURCE_STATE` changes resource
records. The cursor is reconstructed projection metadata, not a wire field,
checkpoint, or second persistence store, and opaque generic Observations cannot
be interpreted as resource state.

### Adapter plane

Adapters translate between Patchbay concepts and external systems. Pi is the first session adapter. token-commune is the second reference adapter and the first materially non-session resource adapter. Adapters declare capabilities and own external-system details.

Adapters are not allowed to introduce core-only assumptions such as shared cwd semantics, harness-specific message formats, or project-specific workflow state into the Patchbay core.

The capability manifest declares a generated target category (`runtime_session` or `operational_resource`) and, for resources, exact open `ResourceKind` declarations with per-kind snapshot tier and payload/domain-projection schema bindings. The reserved `knowledge_bundle` category is wire-present for an OKF-v0.2 candidate contract but is rejected at registration until promoted. Core registration validates and stores this projection once; exact `(adapter_id, resource_kind)` lookup is the admission boundary consumed by resource report ingress. Capability declarations remain advisory and never replace grants or adapter-authoritative delivery outcomes.

Adapter-shaped domain projections compose above, not instead of, the canonical Patchbay wrapper. The core and shared presentation floor continue to own resource identity, authority domain, revision/staleness, attention, correlation, and Operation delivery/failure semantics. A surface uses a local known decoder/compositor for the manifest-bound projection schema and nests that data beneath the wrapper. Patchbay does not load adapter-provided renderer code, HTML, CSS, or policy plugins, and schema-reference matching does not claim semantic validation of opaque bytes.

#### Adapter registration and lifecycle

An adapter is a **principal** with an explicit registration lifecycle, symmetric with the web-server-as-principal model. At attach time an adapter submits attachment evidence verified by an adapter-specific trust root (the Pi adapter uses configured local material; future adapters may use mTLS or OAuth) and a capability manifest. The core records the adapter id, capability manifest, attach LSN, and adapter generation (adapter-reported, monotonic per adapter, used to reject stale events from a prior adapter attachment). Sessions and resources discovered or reported by the adapter inherit the adapter's authenticated channel. The canonical durable registration Observation produced by `Attach` is replayed into the adapter target registry, so a durably registered adapter remains eligible for explicit adapter-scoped spawn resolution after an ordinary core restart. The attachment token and live delivery subscription are process-local liveness/delivery concerns, not durable routing authority: they are not persisted, and actual spawn delivery waits or fails through the existing adapter delivery behavior until a current live attachment can receive it.

The adapter lifecycle is audited:

- **Attach** — registration with identity proof and capability manifest.
- **Detach** — clean detachment; the core marks affected sessions and resources `stale` or `offline` under their respective state contracts.
- **Failure** — loss detected via timeout; the core degrades affected sessions and resources honestly rather than fabricating liveness or health.
- **Capability redeclaration** — allowed with audit; the core compares the prior and incoming validated manifests and atomically couples registration to any required resource degradation before publishing the replacement attachment. Removed, down-tiered, schema-incompatible, and newer-generation resource views degrade per `docs/PROTOCOL.md`; a failed batch leaves no usable replacement token.

The trust-root mechanism is adapter-specific; the core validates attachment evidence but does not mandate a single mechanism. An adapter that cannot provide attachment evidence cannot register (fail-closed).

### Operation, Observation, and Elicitation plane

This plane defines Operation acceptance, delivery, reply/response correlation, idempotency, retries, expiration, failure semantics, source-authenticated Observations, and durable Elicitations. Its state machines and failure vocabulary are owned by `docs/PROTOCOL.md` until generated contracts take over as the derived boundary artifact.

### State and snapshot plane

This plane defines authoritative state for actors, sessions, and resources. Session connectivity/activity axes are owned by `docs/PROTOCOL.md`; control surfaces compose those axes and must display stale, offline, and unknown states distinctly from live states. Resource domain state is carried by an adapter-owned schema and never coerced into those session axes; its snapshot still carries Patchbay revision, authority-domain, source, and staleness context.

Snapshots repair missed streams and reconnect gaps when the adapter or core can provide an authoritative snapshot. Adapters with partial or no snapshot capability degrade as defined in `docs/PROTOCOL.md`. A resource adapter may claim only the tier supported by the complete external view it can actually reconstruct. Every core-materialized session/resource snapshot carries the authority domain's persisted nonzero storage-continuity epoch. Stored session checkpoints use a private typed, versioned envelope and are usable only when the envelope kind/version, storage-row anchor, and embedded authority-domain/epoch/LSN anchors match exactly; freshness is then decided separately by LSN. Legacy undiscriminated bytes and resource payloads are disposable misses rather than session authority. `LoadSnapshot` explicitly selects and echoes the public session or resource view. Resource snapshots materialize on demand from the durable resource projection; the checkpoint namespace remains session-only until a future composite or per-projection namespace is justified.

### Authority and identity plane

This plane defines actor identity, device/session identity, grants, revocation, delegated authority, and spoofing resistance.

Patchbay treats all external actor identities as claims until verified by the relevant adapter, trust root, or deployment policy.

### Coordination plane

This plane defines leases, ownership claims, handoffs, locks, and coordination metadata. It prevents two actors from simultaneously owning an exclusive resource inside the modeled boundary.

### Deployment plane

This plane defines how Patchbay components run: daemon, container, VM, local service, sidecar, or split deployment. The architecture does not require one deployment topology.

### Verification plane

This plane contains TLA+/Quint models, Alloy models, protocol contracts, conformance vectors, and property tests.

## Component view

```text
┌──────────────────────────────────────────────────────────┐
│ Human control surfaces                                   │
│  web cockpit     CLI     future Expo app     notifications│
└───────────────┬──────────────────────────────────────────┘
                │
┌───────────────▼──────────────────────────────────────────┐
│ Shared TypeScript operator domain                        │
│  protocol client                                         │
│  delivery state machines                                 │
│  reconnect/snapshot state                                │
│  stale/live/working/offline presentation model            │
└───────────────┬──────────────────────────────────────────┘
                │
┌───────────────▼──────────────────────────────────────────┐
│ Patchbay coordination core                               │
│  actor/session/resource registry                         │
│  durable events/inboxes                                  │
│  operation routing                                       │
│  authority/grants                                        │
│  snapshots                                               │
│  leases                                                  │
└───────┬───────────────┬───────────────────────┬──────────┘
        │               │                       │
┌───────▼──────┐ ┌──────▼──────┐        ┌───────▼──────────┐
│ Pi adapter   │ │ token-commune│        │ future adapters  │
│ sessions     │ │ resources    │        │ harness/tool/etc │
└──────────────┘ └─────────────┘        └──────────────────┘
```

## Versioned deployment horizon

The first executable slice and the public product share the same durable-core architecture but carry different support commitments:

- **`v0.1.0`** gets the initial operator operational with one authoritative core, one operator, Pi, a responsive web cockpit, a diagnostic CLI, and durable local persistence behind ports.
- **`v0.x`** hardens packaging, migrations, public boundaries, executable conformance, the operational-resource plane, and adapter portability while contracts may still evolve through explicit breaking changes.
- **`v1.0.0`** supports independent operators self-hosting Patchbay through one tested reference deployment path. Each v1 deployment has one human operator. Pi plus token-commune prove the adapter boundary across runtime-session and operational-resource shapes.

The v1 reference support boundary includes installation, TLS/reverse-proxy guidance, identity and adapter enrollment/revocation, versioned configuration and storage migrations, upgrade/rollback expectations, backup/restore, diagnostics, and crash recovery. The architecture remains deployment-neutral even though only one golden deployment path is required to be supported. token-commune remains separately deployable: a personal Patchbay adapter reaches it through its external API using that operator's scoped gateway credential, and its LLM traffic never enters Patchbay. HA, federation, multi-human shared deployments, multiple storage backends, zero-downtime upgrades, and orchestration-specific packaging remain post-v1 seams.

## v0.1.0 component slice

The `v0.1.0` executable slice is a single-authority deployment that proves the core control loop without implementing the whole future platform.

```text
┌────────────────────────────┐   ┌────────────────────────────┐
│ Responsive web cockpit     │   │ CLI                         │
│ session list, composer,    │   │ setup, admin, debugging,    │
│ delivery states            │   │ scripted access             │
└──────────────┬─────────────┘   └──────────────┬─────────────┘
               │ shared TypeScript protocol     │ same protocol semantics
               └───────────────┬────────────────┘
                               ▼
┌──────────────────────────────────────────────────────────┐
│ Single Patchbay coordination core                        │
│  actor/session registry                                  │
│  operation acceptance + idempotency                      │
│  authority checks                                        │
│  durable event log + snapshots                           │
└──────────────────────────────┬───────────────────────────┘
                               │ adapter capability boundary
                               ▼
┌──────────────────────────────────────────────────────────┐
│ Pi adapter                                                │
│  session discovery/status                                 │
│  operation/prompt delivery                                 │
│  cancel/interrupt where supported                         │
│  replies/events/snapshots                                 │
└──────────────────────────────────────────────────────────┘
```

v0.1.0 architecture decisions:

- The coordination core is singular and authoritative. v0.1.0 ships its core-dependent processes colocated on one host: the core listener is loopback-only and the web server, CLI, and Pi adapter reach it through that listener. There is no HA core or multi-writer state in v0.1.0; split deployment is a reserved seam.
- Persistence is local and durable through a storage port owned by the core. The first backend can be embedded, but event and snapshot semantics must remain independent of the backend.
- The Pi adapter is the only required runtime adapter. Other adapters remain future examples and must not shape the v0.1.0 core ontology.
- The web cockpit is the first operator surface. The CLI exists for setup, administration, debugging, and scripted access, not as a second independent product surface with divergent semantics.
- Leases remain in the architecture and verification vocabulary, but are not required for the v0.1.0 executable skeleton unless a later scoped feature promotes a specific lease-backed workflow.

### v0.1.0 process topology

v0.1.0 runs three logical processes:

- **Rust coordination core** — the single authoritative process. Owns the durable event log, Operation acceptance, authority checks, snapshots, and the storage port. Does not terminate browser-facing HTTP/HTTPS in v0.1.0 (its gRPC listeners speak h2c to control surfaces and adapters).
- **TypeScript web server** — a control-surface process that terminates HTTP/HTTPS for the browser cockpit, owns operator sessions, cookies, and CSRF protection, and speaks the generated Protobuf/Connect contract to the Rust core.
- **TypeScript Pi adapter** — a runtime-adapter process that hosts Pi `AgentSession` runtimes, translates delivered Operations into Pi actions, and reports source-authenticated session state, results, and transcript Observations to the Rust core.

The web server is a **control surface, not a core**. It is an authenticated endpoint/principal with respect to the core, subject to the same grant and audit rules as other control surfaces. The Rust core remains the single authoritative coordination process; the web server never writes the durable log or makes authority decisions.

The browser runs its own TypeScript client domain (protocol client, delivery/reconnect/session state machines, presentation model) as a client of the web server. The CLI implements a separate client domain over the same generated contracts and protocol semantics; a future Expo app would do likewise. The presentation model is refined in `docs/UX.md` as the **shared presentation-component layer**. v0.1.0 implements that layer as a registry-derived static check plus skin-able CSS and showcase artifacts that bind canonical protocol states to presentable primitives. Executable runtime consumer assertions remain a reserved seam (see `docs/UX.md`).

Reserved seams:

- **Server-side operator-domain reuse**: v0.1.0 may run the web server as a thin HTTP→protocol translator with the operator domain executing only in the browser; promoting delivery/reconnect state machines or SSR to the server is reserved for when a concrete need arrives.
- **Web↔core internal protocol**: v0.1.0 ships a generated Protobuf/Connect boundary for `Submit`, `LoadSnapshot`, and `Subscribe`. The web server verifies the configured operator password with the core, receives core-issued control-surface principal and operator-session evidence, and forwards that evidence on its session-authenticated RPC bridge; state-changing browser calls pass CSRF protection before forwarding. Additional internal RPC surface and protocol evolution are reserved seams.
- **Split deployment**: v0.1.0 ships the core, web server, CLI, and Pi adapter colocated on one host, with the core bound only to loopback. A browser may reach the colocated web server directly over TLS, but that does not make the core network-reachable. Separate-machine components and a network-reachable core are reserved seams that require an explicit transport/TLS design.

This topology is consistent with the single-authoritative-core commitment: there is one writer to the durable log (the Rust core), and the HTTP-terminating process is a control surface whose authority is delegated and revocable.

### v0.1.0 persistence topology

The v0.1.0 persistence layer is a single-writer, local-first, port-isolated store owned by the coordination core:

- **Single writer**: one authoritative core process appends to the durable event log. No multi-writer coordination, no HA, no split-brain recovery.
- **Embedded default backend**: the first backend may be embedded in the core process (e.g. a local file or embedded database). Domain semantics must not depend on the backend choice.
- **Storage port**: the core reads and writes through a storage port; adapters and control surfaces never touch persistence directly. This is the Ports & Adapters boundary for durability.
- **Log + snapshots**: the durable event log is the source of truth; snapshots are derived checkpoints used to bound recovery replay cost. A snapshot is never an alternate ordering authority. The store persists one nonzero core-assigned continuity epoch per authority domain; all materialized snapshots carry it.
- **Crash recovery**: on ordinary restart the core reloads the same durable continuity epoch and replays the log (or loads a typed/versioned exact domain/epoch/LSN-compatible checkpoint then replays the tail) to reconstruct in-memory state up to the last committed log sequence number. The exported recovery boundary accepts only a caller-decoded, validator-approved typed checkpoint; wrong type/version/domain/epoch/LSN or payload returns no checkpoint and replays from LSN 0. Accepted Operations are restored; no accepted Operation disappears silently.
- **History discontinuity**: v0.1.0 has no epoch-rotation API. A future destructive restore, divergent fork, authoritative-store replacement, multi-core promotion, or zero-downtime deployment must explicitly roll the continuity epoch before serving snapshots/cursors. HA process-incarnation fencing is a separate reserved identity and must not overload the storage-continuity field; an ordinary backup/restore that continues the same history may retain it.
- **No remote replication**: v0.1.0 does not require WAL shipping, remote replicas, or storage-engine hot swap. Those are reserved seams for HA/federated deployments.

Revision and cursor semantics for events and snapshots are defined in `docs/PROTOCOL.md`. v0.1.0 does not require multi-region replication, point-in-time cloning, or cross-backend snapshot portability.

Future architecture planes remain valid direction, but v0.1.0 implementation should prefer seams over breadth: define the port, registry, or capability boundary needed for later growth without implementing native mobile, HA, multi-operator coordination, or arbitrary adapters.

## Data flow

1. A control surface submits an operator-originated Operation through the shared TypeScript client. (Non-operator Operation submitters remain reserved seams in v0.1.0.)
2. The coordination core validates identity, authority, target scope, idempotency key, `OperationKind`, and payload shape.
3. Accepted Operations are durably recorded before delivery is attempted.
4. The target adapter receives the Operation or reports an explicit failure; adapters and actors emit source-authenticated Observations.
5. Adapter/agent/harness openers create Elicitations over authenticated adapter channels; the core durably records them and fans them out to subscribed operator surfaces.
6. Replies, response Operations, and Observations correlate to the accepted Operation/Elicitation they answer by typed reference.
7. Control surfaces update from event streams when available; streams are delivery optimizations, not authoritative alone.
8. On reconnect, control surfaces submit their cursor and reconcile against snapshots and core records rather than optimistic UI state.

## Boundary rules

- The coordination core owns durable Operation state and authority checks.
- Adapters own external-runtime protocol details.
- Control surfaces never infer authoritative state from optimistic UI alone.
- `docs/PROTOCOL.md` is the prose source of truth for state registries until generated contracts exist.
- Generated contracts or central schemas define wire shapes and derive Operation/session/failure variants from the canonical protocol registry.
- Formal models define product semantics for delivery, authority, identity, snapshots, and leases using the canonical protocol variables.

The production core also composes one fail-closed descendant-completion log consumer. It rebuilds and repairs the authority-domain prefix under the same `CoreDecisionGate` used by control and adapter decisions before service projections are constructed or listeners bind, then runs continuous catch-up as a peer of both serving futures. The consumer is storage-port based and uses the shared gap-free replay validator. Its canonical authority/command folds require the exact prior parent grant, verified accepted spawn, valid delivered/running lifecycle, spawn-scope-contained session fact, successful result, audit, grant, and terminal order; revocation command effects share that lifecycle winner. The committed live handoff is an explicitly addressed adapter target; broader historical/future scope containment in the fold is not a fleet-selection implementation. It writes bounded redacted `spawn_completion_deferred` audit evidence without terminalizing, then writes the spawn-completion audit and descendant grant before the final terminal transition, process-fails on malformed history or loss of durable audit, and suppresses adapter redelivery after qualifying durable deferred success rather than risking duplicate non-idempotent spawns. The staged audit is durable provenance only; stderr completion is emitted after final transition durability.

## Reference adapter paths

### Pi-first session migration

The Pi adapter provides the first real runtime integration. It exposes Pi sessions to Patchbay without making Pi session semantics global. Pi-specific features appear as adapter capabilities, not as core protocol requirements.

The migration target is functional parity with the operator's current Remote Pi workflow and a UX quality bar closer to a mature remote agent app. The v0.1.0 Pi adapter parity checklist, capability mapping, and migration-decision criteria live in `docs/ADAPTER-PI.md`.

### token-commune operational-resource path

token-commune provides the first materially non-session reference integration. An outboard adapter consumes the gateway's external metadata/control API and reports provider pools, contribution health, model availability, member draw, fingerprint state, and lifecycle events as resource snapshots, query results, Observations, and attention. A read-only observer lands before administrative mutations; later grant-gated Operations may drive gateway actions only through explicit upstream contracts with honest semantic completion and idempotency behavior.

Each near-term Patchbay deployment remains personal: a member deployment uses that member's token-commune credential, while an admin deployment uses an admin credential. token-commune remains authoritative for gateway roles and pool policy; Patchbay grants constrain local operator actions. token-commune's CLI and embedded UI remain independent fallbacks, and model prompts/responses never cross the Patchbay adapter boundary.
