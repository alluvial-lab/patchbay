# Patchbay Architecture

Patchbay separates the coordination core from control surfaces and adapters. The architecture is defined by planes so implementation modules can evolve without collapsing examples into core concepts.

## Planes

### Human control surface plane

The human control surface plane contains web, CLI, future mobile, desktop, notification, and approval surfaces. It is the lead product plane.

The first surface is a responsive web cockpit using the shared TypeScript operator domain. The CLI provides administrative and scriptable control. The future Expo app reuses the same domain and protocol client.

### Operation plane

The operation plane represents authorized control-plane requests through an actor-neutral vocabulary, while v0 admits only operator-originated Operations. It includes spawn, attach, instruct, cancel, interrupt, query, approval response, elicitation response, reconfiguration, and session-management Operations; non-operator Operation senders remain reserved seams. Reserved-but-not-validatable seams such as `agent-send` and `adapter-utility-exec` are named for non-operator routing and standalone adapter utility-exec pressure, but v0 submissions reject with `validation_failed`.

Every accepted Operation initially reuses the canonical `CommandState` registry in `docs/PROTOCOL.md` by documented refinement equivalence. Control-surface-local submission states are separate and never become durable core states. A future rename to `OperationState` must update prose, generated contracts, formal models, conformance vectors, and implementations together.

### Runtime/session plane

The runtime/session plane contains harness sessions, agent processes, shell jobs, containers, worktrees, CI tasks, or other execution contexts. Patchbay observes and controls these through adapters.

Spawn authority is fleet-level by default in v0: a spawn grant authorizes spawning across any adapter/supervisor the operator can reach, before a target session exists. Adapter-level spawn grants remain expressible through the existing target-scope flexibility when narrower authority is desired. Spawn is one `OperationKind`; spawn variants (worktree, same-dir, session, process, cloud environment) are described by payload `target_spec.shape` from a reserved open shape registry, not by per-variant OperationKinds. See `docs/PROTOCOL.md` for the spawn authority model and `docs/SECURITY.md` for the descendant-grant and revocation rules.

### Adapter plane

Adapters translate between Patchbay concepts and external systems. Pi is the first adapter. Adapters declare capabilities and own external-system details.

Adapters are not allowed to introduce core-only assumptions such as shared cwd semantics, harness-specific message formats, or project-specific workflow state into the Patchbay core.

#### Adapter registration and lifecycle

An adapter is a **principal** with an explicit registration lifecycle, symmetric with the web-server-as-principal model. At attach time an adapter submits attachment evidence verified by an adapter-specific trust root (the Pi adapter uses configured local material; future adapters may use mTLS or OAuth) and a capability manifest. The core records the adapter id, capability manifest, attach LSN, and adapter generation (adapter-reported, monotonic per adapter, used to reject stale events from a prior adapter attachment). Sessions discovered or reported by the adapter inherit the adapter's authenticated channel.

The adapter lifecycle is audited:

- **Attach** — registration with identity proof and capability manifest.
- **Detach** — clean detachment; the core marks affected sessions `stale` or `offline`.
- **Failure** — loss detected via timeout; the core degrades affected sessions honestly rather than fabricating liveness.
- **Capability redeclaration** — allowed with audit; when an adapter loses a capability it previously had, the core records the change and degrades affected sessions per the rules in `docs/PROTOCOL.md`.

The trust-root mechanism is adapter-specific; the core validates attachment evidence but does not mandate a single mechanism. An adapter that cannot provide attachment evidence cannot register (fail-closed).

### Operation, Observation, and Elicitation plane

This plane defines Operation acceptance, delivery, reply/response correlation, idempotency, retries, expiration, failure semantics, source-authenticated Observations, and durable Elicitations. Its state machines and failure vocabulary are owned by `docs/PROTOCOL.md` until generated contracts take over as the derived boundary artifact.

### State and snapshot plane

This plane defines authoritative state for actors, sessions, and resources. Session connectivity/activity axes are owned by `docs/PROTOCOL.md`; control surfaces compose those axes and must display stale, offline, and unknown states distinctly from live states.

Snapshots repair missed streams and reconnect gaps when the adapter or core can provide an authoritative snapshot. Adapters with partial or no snapshot capability degrade as defined in `docs/PROTOCOL.md`.

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
│  actor registry                                          │
│  durable events/inboxes                                  │
│  operation routing                                       │
│  authority/grants                                        │
│  snapshots                                               │
│  leases                                                  │
└───────┬───────────────┬───────────────────────┬──────────┘
        │               │                       │
┌───────▼──────┐ ┌──────▼──────┐        ┌───────▼──────────┐
│ Pi adapter   │ │ shell/job    │        │ future adapters  │
│ first target │ │ adapter      │        │ harness/tool/etc │
└──────────────┘ └─────────────┘        └──────────────────┘
```

## V0 component slice

The v0 executable slice is a single-authority deployment that proves the core control loop without implementing the whole future platform.

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

V0 architecture decisions:

- The coordination core is singular and authoritative. Split deployments may place the web surface, CLI, core, and adapter processes on different machines, but there is no HA core or multi-writer state in v0.
- Persistence is local and durable through a storage port owned by the core. The first backend can be embedded, but event and snapshot semantics must remain independent of the backend.
- The Pi adapter is the only required runtime adapter. Other adapters remain future examples and must not shape the v0 core ontology.
- The web cockpit is the first operator surface. The CLI exists for setup, administration, debugging, and scripted access, not as a second independent product surface with divergent semantics.
- Leases remain in the architecture and verification vocabulary, but are not required for the v0 executable skeleton unless a later scoped feature promotes a specific lease-backed workflow.

### V0 process topology

V0 runs two logical processes, not one:

- **Rust coordination core** — the single authoritative process. Owns the durable event log, Operation acceptance, authority checks, snapshots, and the storage port. Does not terminate HTTP in v0.
- **TypeScript web server** — a control-surface process that terminates HTTP/HTTPS for the browser cockpit, owns operator sessions, cookies, and CSRF protection, and speaks the generated Protobuf/Connect contract to the Rust core.

The web server is a **control surface, not a core**. It is an authenticated endpoint/principal with respect to the core, subject to the same grant and audit rules as other control surfaces. The Rust core remains the single authoritative coordination process; the web server never writes the durable log or makes authority decisions.

The browser runs the shared TypeScript operator domain (protocol client, delivery/reconnect/session state machines, presentation model) as a client of the web server. The future Expo app and CLI reuse the same operator domain and the same protocol semantics. The presentation model is refined in `docs/UX.md` as the **shared presentation-component layer** — a named architectural seam that binds canonical protocol states to skin-able presentable primitives, making the surface-neutral UX conformance floor enforceable; its implementation is deferred (see `docs/UX.md`).

Reserved seams:

- **Server-side operator-domain reuse**: v0 may run the web server as a thin HTTP→protocol translator with the operator domain executing only in the browser; promoting delivery/reconnect state machines or SSR to the server is reserved for when a concrete need arrives.
- **Web↔core internal protocol design**: the specific RPC surface, streaming/event channel, operator-session/CSRF evidence crossing, and web-surface authentication to the core are designed in a follow-on feature (see `feature-web-core-protocol-seam`).
- **Split deployment**: the web server, CLI, core, and adapters may run on different machines. V0 may colocate them on one host for installation simplicity, but that colocation is a deployment convenience, not the architecture. The Rust coordination core remains the network-reachable fixed point and the single durable writer.

This topology is consistent with the single-authoritative-core commitment: there is one writer to the durable log (the Rust core), and the HTTP-terminating process is a control surface whose authority is delegated and revocable.

### V0 persistence topology

The v0 persistence layer is a single-writer, local-first, port-isolated store owned by the coordination core:

- **Single writer**: one authoritative core process appends to the durable event log. No multi-writer coordination, no HA, no split-brain recovery.
- **Embedded default backend**: the first backend may be embedded in the core process (e.g. a local file or embedded database). Domain semantics must not depend on the backend choice.
- **Storage port**: the core reads and writes through a storage port; adapters and control surfaces never touch persistence directly. This is the Ports & Adapters boundary for durability.
- **Log + snapshots**: the durable event log is the source of truth; snapshots are derived checkpoints used to bound recovery replay cost. A snapshot is never an alternate ordering authority.
- **Crash recovery**: on restart the core replays the log (or loads the latest snapshot then replays the tail) to reconstruct in-memory state up to the last committed log sequence number. Accepted Operations are restored; no accepted Operation disappears silently.
- **No remote replication**: v0 does not require WAL shipping, remote replicas, or storage-engine hot swap. Those are reserved seams for HA/federated deployments.

Revision and cursor semantics for events and snapshots are defined in `docs/PROTOCOL.md`. V0 does not require multi-region replication, point-in-time cloning, or cross-backend snapshot portability.

Future architecture planes remain valid direction, but v0 implementation should prefer seams over breadth: define the port, registry, or capability boundary needed for later growth without implementing native mobile, HA, multi-operator coordination, or arbitrary adapters.

## Data flow

1. A control surface submits an operator-originated Operation through the shared TypeScript client. (Non-operator Operation submitters remain reserved seams in v0.)
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

## Pi-first migration path

The Pi adapter provides the first real runtime integration. It exposes Pi sessions to Patchbay without making Pi session semantics global. Pi-specific features appear as adapter capabilities, not as core protocol requirements.

The migration target is functional parity with the operator's current Remote Pi workflow and a UX quality bar closer to a mature remote agent app. The v0 Pi adapter parity checklist, capability mapping, and migration-decision criteria live in `docs/ADAPTER-PI.md`.
