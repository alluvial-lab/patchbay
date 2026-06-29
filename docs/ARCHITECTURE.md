# Patchbay Architecture

Patchbay separates the coordination core from control surfaces and adapters. The architecture is defined by planes so implementation modules can evolve without collapsing examples into core concepts.

## Planes

### Human control surface plane

The human control surface plane contains web, CLI, future mobile, desktop, notification, and approval surfaces. It is the lead product plane.

The first surface is a responsive web cockpit using the shared TypeScript operator domain. The CLI provides administrative and scriptable control. The future Expo app reuses the same domain and protocol client.

### Operator intent plane

The operator intent plane represents prompts, commands, approvals, cancels, resumes, compactions, session switches, and other human-directed actions.

Every accepted operator intent has a durable command state from the canonical `CommandState` registry in `docs/PROTOCOL.md`. Control-surface-local submission states are separate and never become durable core states.

### Runtime/session plane

The runtime/session plane contains harness sessions, agent processes, shell jobs, containers, worktrees, CI tasks, or other execution contexts. Patchbay observes and controls these through adapters.

### Adapter plane

Adapters translate between Patchbay concepts and external systems. Pi is the first adapter. Adapters declare capabilities and own external-system details.

Adapters are not allowed to introduce core-only assumptions such as shared cwd semantics, harness-specific message formats, or project-specific workflow state into the Patchbay core.

### Message and command plane

This plane defines delivery, command acceptance, reply correlation, idempotency, retries, expiration, and failure semantics. Its state machines and failure vocabulary are owned by `docs/PROTOCOL.md` until generated contracts take over as the derived boundary artifact.

Live streaming is an optimization. Durable acceptance and snapshot recovery carry correctness.

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
│  command routing                                         │
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
│  command acceptance + idempotency                        │
│  authority checks                                        │
│  durable event log + snapshots                           │
└──────────────────────────────┬───────────────────────────┘
                               │ adapter capability boundary
                               ▼
┌──────────────────────────────────────────────────────────┐
│ Pi adapter                                                │
│  session discovery/status                                 │
│  message/prompt delivery                                  │
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

- **Rust coordination core** — the single authoritative process. Owns the durable event log, command acceptance, authority checks, snapshots, and the storage port. Does not terminate HTTP in v0.
- **TypeScript web server** — a control-surface process that terminates HTTP/HTTPS for the browser cockpit, owns operator sessions, cookies, and CSRF protection, and speaks the generated Protobuf/Connect contract to the Rust core.

The web server is a **control surface, not a core**. It is an authenticated endpoint/principal with respect to the core, subject to the same grant and audit rules as other control surfaces. The Rust core remains the single authoritative coordination process; the web server never writes the durable log or makes authority decisions.

The browser runs the shared TypeScript operator domain (protocol client, delivery/reconnect/session state machines, presentation model) as a client of the web server. The future Expo app and CLI reuse the same operator domain and the same protocol semantics.

Reserved seams:

- **Server-side operator-domain reuse**: v0 may run the web server as a thin HTTP→protocol translator with the operator domain executing only in the browser; promoting delivery/reconnect state machines or SSR to the server is reserved for when a concrete need arrives.
- **Web↔core internal protocol design**: the specific RPC surface, streaming/event channel, operator-session/CSRF evidence crossing, and web-surface authentication to the core are designed in a follow-on feature (see `feature-web-core-protocol-seam`).
- **Split deployment**: the web server may run near the operator and the core elsewhere once the internal protocol seam is designed; v0 may colocate them on one host for simplicity.

This topology is consistent with the single-authoritative-core commitment: there is one writer to the durable log (the Rust core), and the HTTP-terminating process is a control surface whose authority is delegated and revocable.

### V0 persistence topology

The v0 persistence layer is a single-writer, local-first, port-isolated store owned by the coordination core:

- **Single writer**: one authoritative core process appends to the durable event log. No multi-writer coordination, no HA, no split-brain recovery.
- **Embedded default backend**: the first backend may be embedded in the core process (e.g. a local file or embedded database). Domain semantics must not depend on the backend choice.
- **Storage port**: the core reads and writes through a storage port; adapters and control surfaces never touch persistence directly. This is the Ports & Adapters boundary for durability.
- **Log + snapshots**: the durable event log is the source of truth; snapshots are derived checkpoints used to bound recovery replay cost. A snapshot is never an alternate ordering authority.
- **Crash recovery**: on restart the core replays the log (or loads the latest snapshot then replays the tail) to reconstruct in-memory state up to the last committed log sequence number. Accepted commands are restored; no accepted command disappears silently.
- **No remote replication**: v0 does not require WAL shipping, remote replicas, or storage-engine hot swap. Those are reserved seams for HA/federated deployments.

Revision and cursor semantics for events and snapshots are defined in `docs/PROTOCOL.md`. V0 does not require multi-region replication, point-in-time cloning, or cross-backend snapshot portability.

Future architecture planes remain valid direction, but v0 implementation should prefer seams over breadth: define the port, registry, or capability boundary needed for later growth without implementing native mobile, HA, multi-operator coordination, or arbitrary adapters.

## Data flow

1. A control surface submits operator intent through the shared TypeScript client.
2. The coordination core validates identity, authority, target session, idempotency key, and command shape.
3. Accepted intent is durably recorded before delivery is attempted.
4. The target adapter receives the command or reports an explicit failure.
5. Events and replies correlate to the accepted command id.
6. Control surfaces update from event streams when available.
7. On reconnect, control surfaces request snapshots and reconcile local presentation state.

## Boundary rules

- The coordination core owns durable command state and authority checks.
- Adapters own external-runtime protocol details.
- Control surfaces never infer authoritative state from optimistic UI alone.
- `docs/PROTOCOL.md` is the prose source of truth for state registries until generated contracts exist.
- Generated contracts or central schemas define wire shapes and derive command/session/failure variants from the canonical protocol registry.
- Formal models define product semantics for delivery, authority, identity, snapshots, and leases using the canonical protocol variables.

## Pi-first migration path

The Pi adapter provides the first real runtime integration. It exposes Pi sessions to Patchbay without making Pi session semantics global. Pi-specific features appear as adapter capabilities, not as core protocol requirements.

The migration target is functional parity with the operator's current Remote Pi workflow and a UX quality bar closer to a mature remote agent app.
