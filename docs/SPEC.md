# Patchbay Specification

## Definition

Patchbay is a deployment-neutral human control plane for operating agent sessions across machines. It provides durable operator intent, recoverable session state, authority checks, and adapter-neutral routing between human control surfaces and runtime/session adapters.

## Starting scope

Patchbay starts with:

- a responsive web cockpit as the first human control surface;
- a shared TypeScript operator domain used by web now and by an Expo mobile app later;
- a deployment-neutral coordination core;
- a Pi adapter as the first workflow migration target;
- formal models for delivery, identity, authority, snapshots, and leases;
- generated or centrally defined protocol contracts before broad implementation.

Patchbay does not start with a native mobile app, swarm orchestration, project-management assumptions, or hard dependency on a specific harness.

## Deployment assumptions

Patchbay components may run wherever the operator chooses:

- local workstation;
- VM;
- container or Podman/Docker service;
- home server;
- cloud host;
- split deployment with adapters near runtimes and control surfaces elsewhere.

The core model does not assume shared filesystem access, shared process trees, colocated sessions, or a single machine. Adapters declare the capabilities and state they expose.

## Primary stack choices

### Core daemon

The coordination core is Rust-oriented because Patchbay needs a small trusted state, routing, and authority surface with strong typing and property-test support.

### Operator domain and control surfaces

The operator-facing domain is TypeScript-oriented:

- shared protocol client;
- delivery/reconnect/session state machines;
- web cockpit;
- later Expo mobile app.

The web cockpit is responsive and mobile-first so phone, laptop, and desktop use the same initial control surface.

### Protocol contracts

Patchbay prefers a single protocol-contract source of truth. Protobuf + Buf is the default candidate for generated cross-language contracts and breaking-change checks. JSON Schema remains acceptable for JSON-native surfaces where human-readable wire payloads are required.

### Formal specifications

Patchbay uses TLA+ or Quint for dynamic state-machine behavior and Alloy for relational invariants. The verification document defines which properties are modeled and which are intentionally outside the formal boundary.

## Adapter posture

Adapters are replaceable edges. The first adapter targets Pi workflows so the operator can migrate from current remote-control habits. Future adapters may target other harnesses, shell jobs, CI jobs, project tools, notification systems, or human approval surfaces.

Adapters report:

- actor/session identity;
- capabilities;
- live/stale/offline status;
- command acceptance/failure;
- event streams where available;
- authoritative snapshots where possible.

Adapters do not define Patchbay's core ontology.

## Core concepts

- **Operator** — a human who controls sessions through Patchbay.
- **Actor** — a human, agent, daemon, service, or adapter endpoint represented in the system.
- **Control surface** — web, CLI, mobile, desktop, notification, or other human-facing UI.
- **Runtime session** — an external session, process, harness, job, or agent context controlled through an adapter.
- **Adapter** — integration boundary between Patchbay and an external runtime, harness, tool, or surface.
- **Message** — information delivered to an actor or session.
- **Command** — operator intent that may cause an action.
- **Reply** — correlated answer to a previous message or command.
- **Snapshot** — authoritative state view for a session, actor, or resource.
- **Grant** — authority relationship allowing one actor/control surface to perform actions on a target.
- **Lease** — time-bounded exclusive claim over a resource or coordination role.
- **Event** — durable record of an accepted state transition.

## Non-goals

Patchbay does not verify LLM output quality, replace cryptographic primitives, guarantee OS background execution, or impose a project/workflow substrate. Those concerns belong to adapters, deployment configuration, or separate tools.
