# Patchbay Architecture

Patchbay separates the coordination core from control surfaces and adapters. The architecture is defined by planes so implementation modules can evolve without collapsing examples into core concepts.

## Planes

### Human control surface plane

The human control surface plane contains web, CLI, future mobile, desktop, notification, and approval surfaces. It is the lead product plane.

The first surface is a responsive web cockpit using the shared TypeScript operator domain. The CLI provides administrative and scriptable control. The future Expo app reuses the same domain and protocol client.

### Operator intent plane

The operator intent plane represents prompts, commands, approvals, cancels, resumes, compactions, session switches, and other human-directed actions.

Every accepted operator intent has a durable state: accepted, delivered, rejected, expired, failed, superseded, or completed.

### Runtime/session plane

The runtime/session plane contains harness sessions, agent processes, shell jobs, containers, worktrees, CI tasks, or other execution contexts. Patchbay observes and controls these through adapters.

### Adapter plane

Adapters translate between Patchbay concepts and external systems. Pi is the first adapter. Adapters declare capabilities and own external-system details.

Adapters are not allowed to introduce core-only assumptions such as shared cwd semantics, harness-specific message formats, or project-specific workflow state into the Patchbay core.

### Message and command plane

This plane defines delivery, command acceptance, reply correlation, idempotency, retries, expiration, and failure semantics.

Live streaming is an optimization. Durable acceptance and snapshot recovery carry correctness.

### State and snapshot plane

This plane defines authoritative state for actors, sessions, and resources. Control surfaces must display stale, offline, and unknown states distinctly from live states.

Snapshots repair missed streams and reconnect gaps.

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
- Generated contracts or central schemas define wire shapes.
- Formal models define product semantics for delivery, authority, identity, snapshots, and leases.

## Pi-first migration path

The Pi adapter provides the first real runtime integration. It exposes Pi sessions to Patchbay without making Pi session semantics global. Pi-specific features appear as adapter capabilities, not as core protocol requirements.

The migration target is functional parity with the operator's current Remote Pi workflow and a UX quality bar closer to a mature remote agent app.
