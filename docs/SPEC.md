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

## V0 walking skeleton

The first executable Patchbay milestone is deliberately narrow: one operator controls Pi-backed runtime sessions through a responsive web cockpit, with a CLI available for setup, administration, debugging, and scripted inspection.

V0 includes:

- **Operator scope:** one human operator. The model keeps actor, endpoint, grant, and audit concepts explicit so future multi-human coordination is possible, but v0 does not provision multiple humans or shared authority domains.
- **Deployment topology:** one authoritative coordination core process. Adapters and control surfaces may run in separate processes, but v0 does not provide high availability, clustering, split-brain resolution, or multiple authoritative cores.
- **Persistence:** a local durable event and snapshot store behind ports. The first backend may be embedded and file- or database-backed, but domain semantics must not depend on a specific storage engine.
- **First adapter:** Pi. Patchbay exposes Pi sessions through adapter-declared capabilities rather than making Pi concepts part of the core ontology.
- **Initial OperationKinds:** initial `OperationKind` registry: committed `spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, and `session-management`, plus reserved `agent-send` and `adapter-utility-exec` (rejected with `validation_failed` in v0); prompt text, slash-commands, images, and structured user input are payloads carried by `instruct` or response Operations. Observations carry output/events/status; they are not OperationKinds. Broader OperationKind families wait until the protocol registry and conformance vectors exist.
- **Control surfaces:** responsive web cockpit first, with CLI support for administration, debugging, and scripted access. Native mobile, desktop, notifications, and third-party surfaces are future work.
- **Verification floor:** protocol contracts, checked-model seed coverage for command acceptance, idempotent retry, and session identity, and stated-normative draft model obligations for snapshots and authority before those semantics are treated as product behavior. Checked-normative status additionally requires promoted conformance vectors. Lease modeling remains required before lease-backed behavior ships, but leases are outside the v0 executable skeleton unless explicitly promoted.

V0 explicitly excludes:

- native mobile or Expo app delivery;
- multi-operator provisioning, handoff workflows, shared authority administration, or third-party human coordination;
- high availability, replicated cores, or split-brain recovery (fleet-level spawn authority IS in v0 scope — it is single-operator, single-core, not HA/multi-core);
- non-operator Operation senders (agent→agent, adapter→operator service Operations) — a reserved seam, rejected with `validation_failed` in v0;
- arbitrary adapter ecosystem support beyond the Pi adapter seam;
- general project-management or workflow-substrate features;
- lease-backed exclusive coordination unless a later foundation feature explicitly promotes leases into the first executable slice.

Follow-on work is inside v0 only when it is required to make this slice usable, verifiable, and recoverable. Work that broadens operators, adapters, deployment topology, surfaces, or coordination modes is outside v0 unless it preserves an explicit seam without implementing the broader capability.

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

Patchbay uses Protobuf schemas managed by Buf as the v0 boundary-contract source for durable protocol messages, command/event payloads, and the wire encoding of shared enum vocabularies across the Rust coordination core and TypeScript operator domain.

- `.proto` files are the source for wire contracts and boundary DTOs (including the wire encoding of enum vocabularies), not the full internal domain model and not the canonical registry of protocol variant names.
- Rust types are generated via prost/prost-build; TypeScript types via Protobuf-ES. Generated outputs are artifacts, never hand-edited.
- `buf.gen.yaml` is checked in; `buf lint` and `buf breaking` run locally and in CI.
- JSON Schema / TypeBox / Zod are reserved for JSON-native local validation surfaces, not as the cross-language protocol source.
- TypeSpec is a reserved future direction if Patchbay later needs OpenAPI, JSON Schema, and Protobuf emitted as peer outputs from one authoring language.

`.proto` is authority for wire shape only (see `docs/VERIFICATION.md` Artifact authority order). Product intent and vocabulary naming remain prose authority, and invariants remain model authority.

### Formal specifications

Patchbay uses TLA+ or Quint for dynamic state-machine behavior and Alloy for relational invariants. The verification document defines which properties are modeled and which are intentionally outside the formal boundary.

## Adapter posture

Adapters are replaceable edges. The first adapter targets Pi workflows so the operator can migrate from current remote-control habits. Future adapters may target other harnesses, shell jobs, CI jobs, project tools, notification systems, or human approval surfaces.

Adapters report:

- actor/session identity;
- supported OperationKinds (and, for `spawn`, supported `target_spec.shape` values);
- capabilities;
- protocol-derived connectivity and activity status;
- Operation acceptance/failure and Observations;
- event streams where available;
- authoritative snapshots where possible;
- Elicitations opened over the adapter's authenticated channel;
- presence/subscription facts for reconnect reconciliation.

Adapters do not define Patchbay's core ontology.

## Core concepts

- **Operator** — a human who controls sessions through Patchbay.
- **Actor** — a human, agent, daemon, service, or adapter endpoint represented in the system.
- **Control surface** — web, CLI, mobile, desktop, notification, or other human-facing UI.
- **Runtime session** — an external session, process, harness, job, or agent context controlled through an adapter.
- **Adapter** — integration boundary between Patchbay and an external runtime, harness, tool, or surface.
- **Operation** — an authorized control-plane request by an actor to a target. V0 Operations are operator-originated; non-operator senders are a reserved seam.
- **Observation** — a source-authenticated fact/event/output/status emission that does not grant authority. Live streams are delivery optimizations.
- **Elicitation** — a durable pending response solicitation opened by an adapter/agent/harness; v0 binds to the operator actor and delivers by subscription fan-out.
- **Payload** — content carried inside an Operation, Observation, or Elicitation; not a standalone authority primitive.
- **Command** — a Patchbay lifecycle record for an accepted authorized request; retained as the checked `CommandState` legacy/refinement term. `Operation` is the actor-neutral vocabulary that maps to it by refinement equivalence.
- **Snapshot** — authoritative state view for a session, actor, or resource.
- **Grant** — authority relationship allowing one actor/control surface to perform OperationKinds on a target.
- **Lease** — time-bounded exclusive claim over a resource or coordination role.
- **Event** — durable record of an accepted state transition.

## Non-goals

Patchbay does not verify LLM output quality, replace cryptographic primitives, guarantee OS background execution, impose a project/workflow substrate, ship native mobile in v0, support multiple human operators in v0, or provide HA/multi-core coordination in v0. Those concerns belong to adapters, deployment configuration, separate tools, or later milestones.
