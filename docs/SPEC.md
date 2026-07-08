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
- **Verification floor:** protocol contracts, checked-model seed coverage for command acceptance, idempotent retry, and session identity, and stated-normative draft model obligations for snapshots and authority before those semantics are treated as product behavior. Checked-normative status additionally requires promoted conformance vectors. v0 does not implement leases; lease-safety properties are a stated-normative precondition for future lease-backed behavior (see `docs/PROTOCOL.md` § Leases), required before any such promotion, but leases are outside the v0 executable skeleton unless explicitly promoted.

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

## V0 performance posture

V0 commits to **no quantitative performance target** — no p99/p95 latency budget, no throughput floor, no concurrent-session cap, no event-stream lag bound, no WAL-append latency target. v0 is single-operator, single-core, and local-first; there is no load profile to target and no second operator to contend with. Setting fabricated numbers now would constrain the implementation before a real usage pattern exists to measure against.

What v0 does carry is a **qualitative responsiveness floor**, stated v0-only: the operator should not *perceive* Patchbay as laggy or lossy during normal single-operator Remote-Pi-style workflows — sending a prompt, switching sessions, reconnecting after a drop, viewing delivery state. This is the product-feel floor already implied by [`docs/UX.md`](UX.md) ("confidence and continuity of a mature first-party remote agent app," "low-friction reconnect," "fast switching"); this section makes it an explicit posture rather than an accidental one. It is testable as "feels responsive under normal single-operator use," not against a number.

Consequences for downstream work:
- Performance budgets, SLAs, and quantitative targets are **deferred** until a real load profile exists (multi-operator contention, HA, replicated cores, or a measured bottleneck demanding a budget). Adding them is a future scoping act, not a v0 obligation.
- Revisit-triggers that reference "v0 latency targets" (e.g., the `v0-stack-tooling` research finding on SQLite + `synchronous=FULL`) test against this qualitative floor in actual single-operator use, not against a committed number. A revisit fires if the implementation *feels* unresponsive or drops accepted work under normal use, not if it misses a fabricated budget.
- Observability surfaces (`feature-observability-operator-admin`) answer "is this fast enough?" operationally against this floor, not by reporting against a spec'd SLA.

This posture is v0-only. "v0 has no quantitative performance target" is the v0 scope statement; it is not a timeless "Patchbay has no performance requirements" architecture claim. A future milestone that needs a budget adds it as a scope act.

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

## Non-foreclosure discipline

Patchbay is deliberately a narrow v0 that must not accidentally close off future directions the operator has not yet thought through. This section states the discipline; the standing checklist future design work runs before committing a decision lives in `AGENTS.md` ("Extension pressure-test checklist"), and the cross-cutting per-seam registry lives in `docs/PROTOCOL.md` ("Extension seams registry").

### Three-way classification

Every design decision is one of:

- **Committed v0** — shipped behavior. It lives in the single source-of-truth registry for its kind (OperationKind enum, Session/Operation/Elicitation state registry, adapter capability manifest, failure vocabulary, response_contract registry). Where it carries a normative safety/security claim, it has checked-model + conformance-vector coverage before v0 treats it as product behavior (see `docs/VERIFICATION.md` property-graded baseline). Promotion to committed is the act of adding it to the registry with its coverage.
- **Reserved seam** — v0 does not implement it, but the design keeps the door open and names the seam. Where wire/forward-compatibility matters (future OperationKinds, future response_contract kinds, future non-operator senders), the reserved value is wire-present in the registry and submission rejects with `validation_failed`/`unsupported_command` in v0 rather than the value being absent. Promotion from reserved to committed is a registry/classification update, not a reversal.
- **Explicitly rejected** — v0 declines the direction, with rationale recorded. Promotion of a rejected direction is a reversal (a real change of mind with a protocol-change ceremony), not a gap that was merely waiting to be filled.

### Non-foreclosure rule

- **Label v0 assumptions as v0-only.** Write "v0 has one operator," "v0 ships a web cockpit and CLI," "v0 has one authority domain." Do not write the timeless "Patchbay has one operator" / "Patchbay ships a web cockpit" form, which silently promotes a v0 scope choice to permanent architecture.
- **Name reserved seams; do not omit them.** A reserved seam is present in the relevant registry (wire-present where forward-compatibility matters) and documented as reserved. Omitting a future direction from the registry forecloses it more than naming it reserved, because adding it later looks like new scope rather than the planned promotion of a seam.
- **Record rejected directions with rationale.** A rejected direction is written down with why, so a future promotion is visibly a reversal requiring a ceremony, not a quiet drift back in.
- **Treat parked ideas as pressure-test inputs, not v0 requirements.** `idea-multi-human-coordination`, `idea-desktop-app-surface`, `idea-agent-to-agent-mesh-seam`, and `idea-operator-customizable-ux-skins` inform the seam inventory; none is a v0 obligation. Each reserved-seam row that corresponds to a parked idea links back to it.

### Forward-compatibility hygiene

Where a wire/identity shape matters for a future variant, the v0 shape carries the future-relevant demarcator even though v0 has a single value, so the future capability arrives as a layer on top rather than a retroactive data migration:

- Event, cursor, and revision identity is the `(authority_domain_id, LSN)` tuple, not a bare LSN. V0 has one authority domain, but the key shape includes the domain demarcator so federation is additive.
- Reserved enum values (`agent-send`, `adapter-utility-exec`, `freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`) are wire-present in v0 proto and rejected at submission, so promoting them is a validation-rule change, not a wire-format change.
- Adapter capability manifests declare capability fields (`supported_operation_kinds`, snapshot tier, `session_replacement`, etc.) rather than the core assuming a single adapter's shape.
- Model intent stays portable across checker backends (Quint primary, TLA+ semantic baseline, Alloy relational) so a tool switch is not a model rewrite.

### What this discipline is not

This is a labeling and registry discipline, not a promise to implement any reserved seam. A reserved seam may never ship. The discipline's guarantee is narrower and load-bearing: a future direction that the operator chooses to pursue can be added as a registry/classification update rather than discovered to be impossible because v0 baked in a single-value assumption.
