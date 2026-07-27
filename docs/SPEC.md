# Patchbay Specification

## Definition

Patchbay is a deployment-neutral human control plane for operating agent sessions and the operational resources that govern their availability, capability, and safe control across machines. It provides durable operator intent, recoverable target state, authority checks, and adapter-neutral routing between human control surfaces and session/resource adapters.

## Starting scope

Patchbay starts with:

- a responsive web cockpit as the first human control surface;
- a shared TypeScript operator domain used by web now and by an Expo mobile app later;
- a deployment-neutral coordination core;
- a Pi adapter as the first workflow migration target;
- formal models for delivery, identity, authority, snapshots, and leases;
- generated or centrally defined protocol contracts before broad implementation.

Patchbay does not start with a native mobile app, swarm orchestration, project-management assumptions, or hard dependency on a specific harness.

## Versioned product horizon

Patchbay uses SemVer to distinguish the initial operator's walking skeleton from the intended public product:

- **`v0.1.0` — initial-operator walking skeleton.** One operator controls Pi-backed sessions through the responsive web cockpit and diagnostic CLI. This milestone proves the durable control loop and gets the initial operator operational; it is a personal/internal milestone rather than a public distribution milestone, does not require completed publication legal review, and is not the product ceiling.
- **`v0.x` — public-preview hardening and agent-operations expansion.** Deployment, migrations, public compatibility surfaces, adapter boundaries, the operational-resource plane, executable conformance, and token-commune reference-adapter evidence mature. Breaking changes remain permitted when explicit migrations and release notes accompany them.
- **`v1.0.0` — reliable self-hosted public product.** Additional operators can independently deploy and operate Patchbay through a supported reference path. One human operator controls each deployment. Pi sessions and token-commune resources provide the two reference adapter shapes. Multi-human shared deployments remain a post-v1 seam.
- **Post-v1 reserved capabilities.** Multi-human authority workflows, federation, HA/multi-core coordination, replication, zero-downtime upgrades, and broader surface and adapter ecosystems are promoted only by demonstrated product pressure.

### v1 adapter proof

`v1.0.0` targets Pi plus token-commune as its materially distinct second reference adapter. Pi exercises session, transcript, runtime-generation, and interactive-control semantics; token-commune exercises provider-pool resources, authoritative metadata reads, capacity/credential health, adapter-shaped projections, and administrative attention/control without carrying LLM traffic through Patchbay. The pair must prove through executable conformance that the public adapter boundary supports both session and resource targets without either adapter's concepts entering the core ontology.

Patchbay does not accept an obligation to build uncompensated first-party-provider integrations, but its public adapter contract must allow adopters and providers to implement them. token-commune remains an independently deployable product with its own API, authorization, CLI, and fallback UI; Patchbay consumes its external contract rather than its internal modules.

### Post-v0.1 agent-operations direction

The post-v0.1 product direction remains personal and operator-led while expanding beyond session-only control:

- **Sessions remain primary.** Runtime sessions retain their canonical identity, connectivity/activity axes, transcript/Observation flows, and lifecycle semantics.
- **Operational resources become first-class targets.** A resource is admitted when its state materially affects what the operator can ask an agent to do or requires human action to keep agent work operating. Resource identity and domain health are distinct from runtime-session identity and connectivity/activity; adapters must not fabricate session state for resources.
- **Adapter-shaped projections sit above the conformance floor.** Shared presentation primitives continue to enforce authority, delivery, failure, reconnect, and stale-state honesty. A reference adapter may add a richer domain projection—such as token-commune capacity, draw, contribution-health, and fingerprint views—without defining new core protocol states.
- **Personal deployments compose communal services.** Each near-term Patchbay deployment has one human operator and uses that operator's upstream credential. token-commune retains member/admin authorization at the gateway; Patchbay grants add local defense in depth rather than replacing upstream roles.
- **The data plane stays outboard.** Patchbay may observe and control token-commune metadata and administrative resources, but model prompts, responses, routing, allocation, and provider credentials remain on token-commune's data/onboarding paths.
- **Coordination remains deferred.** Multi-human Patchbay authority, delegation, quorum, federation, agent-to-agent messaging, and shared-work routing are reserved for demonstrated pressure after the personal resource-and-session cockpit is in daily use.

This direction is not a general monitoring mandate. Arbitrary service telemetry that does not govern agent capability/availability or require operator action remains outside Patchbay's product boundary.

### v1 supported deployment floor

`v1.0.0` is a reliable self-hosted product, not merely source code plus a Dockerfile. It provides one supported reference deployment path with documented installation, TLS/reverse-proxy guidance, operator and adapter enrollment/revocation, versioned configuration and storage migrations, upgrade and rollback expectations, backup/restore, diagnostics and health checks, and tested crash recovery. Deployment-neutrality means domain semantics do not depend on that packaging or one storage backend; it does not require v1 to support every topology.

HA, federation, zero-downtime upgrades, multiple storage backends, and orchestration-specific packaging remain preserved post-v1 seams.

### v1 public compatibility contract

At `v1.0.0`, SemVer compatibility covers:

- the adapter protocol and capability contract;
- explicitly documented public operator APIs;
- supported persisted-data migration paths, without treating raw database tables as a public API;
- documented configuration keys and environment variables;
- script-facing CLI commands, exit codes, and machine-readable output;
- canonical protocol semantics for identity, generations, acceptance, idempotency, correlation, authority, and reconnect.

Internal module APIs, raw database schema, UI structure, human-readable CLI formatting, undesignated internal web/core calls, and formal checker/file layout remain private implementation details.

### v1 assurance policy

Patchbay uses a property-graded hybrid. Every public safety claim requires executable implementation evidence. Formal coverage additionally blocks `v1.0.0` for command terminal races, session-generation isolation, crash/replay/snapshot convergence, and multi-surface Elicitation races. Formal models for multi-human delegation, lease exclusivity, federation, HA, and split-brain behavior gate those future capabilities rather than the v1 release.

A model does not become product evidence merely because a checker accepts it. A formally gated release property must represent the claimed failure boundary, use a property name that matches its formula, survive adversarial mutation/non-vacuity checks, trace to an executable implementation vector, and run both the real checker and implementation test in CI.

## v0.1.0 walking skeleton

The first executable Patchbay milestone is deliberately narrow: one operator controls Pi-backed runtime sessions through a responsive web cockpit, with a CLI available for setup, administration, debugging, and scripted inspection.

v0.1.0 includes:

- **Operator scope:** one human operator. The model keeps actor, endpoint, grant, and audit concepts explicit so future multi-human coordination is possible, but v0.1.0 does not provision multiple humans or shared authority domains.
- **Deployment topology:** one authoritative coordination core process. Adapters and control surfaces may run in separate processes, but v0.1.0 does not provide high availability, clustering, split-brain resolution, or multiple authoritative cores.
- **Persistence:** a local durable event and snapshot store behind ports. The first backend may be embedded and file- or database-backed, but domain semantics must not depend on a specific storage engine.
- **First adapter:** Pi. Patchbay exposes Pi sessions through adapter-declared capabilities rather than making Pi concepts part of the core ontology.
- **Initial OperationKinds:** initial `OperationKind` registry: committed `spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, and `session-management`, plus reserved `agent-send` and `adapter-utility-exec` (rejected with `validation_failed` in v0.1.0); prompt text, slash-commands, images, and structured user input are payloads carried by `instruct` or response Operations. Observations carry output/events/status; they are not OperationKinds. Broader OperationKind families wait until the protocol registry and conformance vectors exist.
- **Control surfaces:** responsive web cockpit first, with CLI support for administration, debugging, and scripted access. Native mobile, desktop, notifications, and third-party surfaces are future work.
- **Verification floor:** protocol contracts plus checked-model seed coverage for terminal finality, boundary deduplication, the no-`accepted → completed` adjacency, generation non-decrease, and the browser-session/CSRF boundary. Accepted-command durability, the full retry and session-identity obligations, snapshots, crash recovery, general authority, fleet-spawn authority, Elicitation-responder authority, non-cascading spawn-grant revocation, descendant-grant creation, typed reply/response correlation, Elicitation lifecycle and timeout/grant behavior, and subscription audit/cursor-replay/grant authorization stay stated-normative until models represent their claimed failure boundaries with independent attempted evidence and mutation-survivable oracles. Checked-normative status additionally requires promoted conformance vectors. v0.1.0 does not implement leases; lease-safety properties are a stated-normative precondition for future lease-backed behavior (see `docs/PROTOCOL.md` § Leases), required before any such promotion, but leases are outside the v0.1.0 executable skeleton unless explicitly promoted.

v0.1.0 explicitly excludes:

- native mobile or Expo app delivery;
- multi-operator provisioning, handoff workflows, shared authority administration, or third-party human coordination;
- high availability, replicated cores, or split-brain recovery (fleet-level spawn authority IS in v0.1.0 scope — it is single-operator, single-core, not HA/multi-core);
- non-operator Operation senders (agent→agent, adapter→operator service Operations) — a reserved seam, rejected with `validation_failed` in v0.1.0;
- arbitrary adapter ecosystem support beyond the Pi adapter seam;
- general project-management or workflow-substrate features;
- lease-backed exclusive coordination unless a later foundation feature explicitly promotes leases into the first executable slice.

Follow-on work is inside v0.1.0 only when it is required to make this slice usable, verifiable, and recoverable. Work that broadens operators, adapters, deployment topology, surfaces, or coordination modes is outside v0.1.0 unless it preserves an explicit seam without implementing the broader capability.

## Deployment assumptions

Patchbay components may run wherever the operator chooses:

- local workstation;
- VM;
- container or Podman/Docker service;
- home server;
- cloud host;
- split deployment with adapters near runtimes and control surfaces elsewhere.

The core model does not assume shared filesystem access, shared process trees, colocated sessions, or a single machine. Adapters declare the capabilities and state they expose.

## v0.1.0 performance posture

v0.1.0 commits to **no quantitative performance target** — no p99/p95 latency budget, no throughput floor, no concurrent-session cap, no event-stream lag bound, no WAL-append latency target. v0.1.0 is single-operator, single-core, and local-first; there is no load profile to target and no second operator to contend with. Setting fabricated numbers now would constrain the implementation before a real usage pattern exists to measure against.

What v0.1.0 does carry is a **qualitative responsiveness floor**, stated v0.1.0-only: the operator should not *perceive* Patchbay as laggy or lossy during normal single-operator Remote-Pi-style workflows — sending a prompt, switching sessions, reconnecting after a drop, viewing delivery state. This is the product-feel floor already implied by [`docs/UX.md`](UX.md) ("confidence and continuity of a mature first-party remote agent app," "low-friction reconnect," "fast switching"); this section makes it an explicit posture rather than an accidental one. It is testable as "feels responsive under normal single-operator use," not against a number.

Consequences for downstream work:
- Performance budgets, SLAs, and quantitative targets are **deferred** until a real load profile exists (multi-operator contention, HA, replicated cores, or a measured bottleneck demanding a budget). Adding them is a future scoping act, not a v0.1.0 obligation.
- Revisit-triggers that reference "v0.1.0 latency targets" (e.g., the `v0-stack-tooling` research finding on SQLite + `synchronous=FULL`) test against this qualitative floor in actual single-operator use, not against a committed number. A revisit fires if the implementation *feels* unresponsive or drops accepted work under normal use, not if it misses a fabricated budget.
- Observability surfaces (`feature-observability-operator-admin`) answer "is this fast enough?" operationally against this floor, not by reporting against a spec'd SLA.

This posture is v0.1.0-only. "v0.1.0 has no quantitative performance target" is the v0.1.0 scope statement; it is not a timeless "Patchbay has no performance requirements" architecture claim. A future milestone that needs a budget adds it as a scope act.

## v0.1.0 observability scope

Observability in v0.1.0 is an **honest partial over existing core state**, not a separate observability subsystem. The durable event log remains the source of truth for command/session/adapter state ([`docs/PROTOCOL.md`](PROTOCOL.md) Snapshots and streams; Persistence and recovery), while security audit decisions are emitted as redacted process stderr/stdout lines ([`docs/SECURITY.md`](SECURITY.md) Audit events). v0.1.0 adds no second writer and no metrics pipeline.

Committed v0.1.0 observability:
- redacted security audit lines on process stderr/stdout;
- the CLI `session-health` projection plus its script-facing output (see [`docs/UX.md`](UX.md) CLI);
- the web cockpit shows current `CommandState` and last transition (per the UX delivery-state floor); it does not carry a trace-timeline UI in v0.1.0.

Deferred to post-v0.1.0 (reserved seams, not silently absent): the durable, queryable audit log and core-diagnostics support for `audit-query`, `inspect-command`, and `adapter-status` (the v0.1.0 CLI carries honest stubs that exit non-zero with a prerequisite message); a per-command delivery-trace timeline UI; metrics (counters/histograms/throughput); a dedicated health/status dashboard; raw `event-inspect <lsn>`; SIEM export and long-retention compliance archives. Quantitative performance budgets/SLAs are deferred per the v0.1.0 performance posture above.

Explicitly rejected for v0.1.0: a dedicated per-command trace storage (would violate the single source of truth and the single-writer invariant); a metrics pipeline as the primary v0.1.0 observability substrate (premature for single-operator v0.1.0). The committed slice answers current session health and command state; detailed audit/command/adapter diagnostic queries wait for the reserved core-diagnostics capability.

This scope is v0.1.0-only. A future milestone that needs query- or monitoring-oriented observability promotes the reserved seams through a scope act.

## Post-v0.1.0 observability scope (dogfooding)

`epic-observability-dogfooding` is the scope act anticipated above. It promotes the reserved seams needed for live single-operator inspection while dogfooding, under the standing constraints: the durable event log remains the single source of truth, observability reads are projections with no second writer, and no metrics pipeline is introduced.

Committed post-v0.1.0 observability:

- **core-diagnostics**: durable, queryable projections over the existing event log (audit records, command history, adapter status), backing the CLI `audit-query`, `inspect-command`, and `adapter-status` commands (the v0.1.0 honest stubs are fulfilled);
- **adapter-process durable diagnostics**: the adapter writes a durable, configurable diagnostics log (attach, delivery, observation, and lifecycle errors) instead of losing them on process exit;
- **adapter diagnostics as payload, surfaced in the cockpit**: adapters may report diagnostics to the core, which records them and presents adapter health, connection state, and recent diagnostic events within the cockpit's existing views.

Still deferred (reserved seams): the per-command delivery-trace timeline UI; metrics (counters/histograms/throughput); a dedicated health/status dashboard; raw `event-inspect <lsn>`; SIEM export and long-retention compliance archives; the no-lifecycle bypass read of the audit log. Still rejected: dedicated per-command trace storage; a metrics pipeline as the primary observability substrate.

This is dogfooding-scope observability for the single operator, not the v1.0.0 supported-diagnostics contract; documented diagnostics and health checks for other self-hosting operators remain with `epic-public-product-contract-self-hosted-operations`.

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

Patchbay uses Protobuf schemas managed by Buf as the v0.1.0 boundary-contract source for durable protocol messages, command/event payloads, and the wire encoding of shared enum vocabularies across the Rust coordination core and TypeScript operator domain.

- `.proto` files are the source for wire contracts and boundary DTOs (including the wire encoding of enum vocabularies), not the full internal domain model and not the canonical registry of protocol variant names.
- Rust types are generated via prost/prost-build; TypeScript types via Protobuf-ES. Generated outputs are artifacts, never hand-edited.
- `buf.gen.yaml` is checked in; `buf lint` and `buf breaking` run locally and in CI.
- JSON Schema / TypeBox / Zod are reserved for JSON-native local validation surfaces, not as the cross-language protocol source.
- TypeSpec is a reserved future direction if Patchbay later needs OpenAPI, JSON Schema, and Protobuf emitted as peer outputs from one authoring language.

`.proto` is authority for wire shape only (see `docs/VERIFICATION.md` Artifact authority order). Product intent and vocabulary naming remain prose authority, and invariants remain model authority.

### Formal specifications

Patchbay uses TLA+ or Quint for dynamic state-machine behavior and Alloy for relational invariants. The verification document defines which properties are modeled and which are intentionally outside the formal boundary.

## Adapter posture

Adapters are replaceable edges. The first adapter targets Pi workflows so the operator can migrate from current remote-control habits. token-commune is the second reference adapter and first operational-resource adapter. Future adapters may target other harnesses, shell jobs, CI jobs, project tools, notification systems, resource systems, or human approval surfaces.

Adapters report:

- actor and target identity, using runtime-session identity for sessions and resource identity for operational resources;
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

- **Operator** — a human who controls sessions and operational resources through Patchbay.
- **Actor** — a human, agent, daemon, service, or adapter endpoint represented in the system.
- **Control surface** — web, CLI, mobile, desktop, notification, or other human-facing UI.
- **Runtime session** — an external session, process, harness, job, or agent context controlled through an adapter.
- **Operational resource** — an adapter-reported non-session target whose state materially governs agent availability, capability, or safe control, such as a provider-capacity pool; its domain health is not session connectivity/activity.
- **Adapter** — integration boundary between Patchbay and an external runtime, harness, resource system, tool, or surface.
- **Operation** — an authorized control-plane request by an actor to a target. v0.1.0 Operations are operator-originated; non-operator senders are a reserved seam.
- **Observation** — a source-authenticated fact/event/output/status emission that does not grant authority. Live streams are delivery optimizations.
- **Elicitation** — a durable pending response solicitation opened by an adapter/agent/harness; v0.1.0 binds to the operator actor and delivers by subscription fan-out.
- **Payload** — content carried inside an Operation, Observation, or Elicitation; not a standalone authority primitive.
- **Command** — a Patchbay lifecycle record for an accepted authorized request; retained as the checked `CommandState` legacy/refinement term. `Operation` is the actor-neutral vocabulary that maps to it by refinement equivalence.
- **Snapshot** — authoritative state view for a session, actor, or resource.
- **Grant** — authority relationship allowing one actor/control surface to perform OperationKinds on a target.
- **Lease** — time-bounded exclusive claim over a resource or coordination role.
- **Event** — durable record of an accepted state transition.

## Non-goals

Patchbay does not verify LLM output quality, replace cryptographic primitives, guarantee OS background execution, impose a project/workflow substrate, ship native mobile in v0.1.0, support multiple human operators in v0.1.0, or provide HA/multi-core coordination in v0.1.0. Those concerns belong to adapters, deployment configuration, separate tools, or later milestones.

## Non-foreclosure discipline

Patchbay is deliberately a narrow v0.1.0 that must not accidentally close off future directions the operator has not yet thought through. This section states the discipline; the standing checklist future design work runs before committing a decision lives in `AGENTS.md` ("Extension pressure-test checklist"), and the cross-cutting per-seam registry lives in `docs/PROTOCOL.md` ("Extension seams registry").

### Three-way classification

Every design decision is one of:

- **Committed v0.1.0** — shipped behavior. It lives in the single source-of-truth registry for its kind (OperationKind enum, Session/Operation/Elicitation state registry, adapter capability manifest, failure vocabulary, response_contract registry). Where it carries a normative safety/security claim, it has checked-model + conformance-vector coverage before v0.1.0 treats it as product behavior (see `docs/VERIFICATION.md` property-graded baseline). Promotion to committed is the act of adding it to the registry with its coverage.
- **Reserved seam** — v0.1.0 does not implement it, but the design keeps the door open and names the seam. Where wire/forward-compatibility matters (future OperationKinds, future response_contract kinds, future non-operator senders), the reserved value is wire-present in the registry and submission rejects with `validation_failed`/`unsupported_command` in v0.1.0 rather than the value being absent. Promotion from reserved to committed is a registry/classification update, not a reversal.
- **Explicitly rejected** — v0.1.0 declines the direction, with rationale recorded. Promotion of a rejected direction is a reversal (a real change of mind with a protocol-change ceremony), not a gap that was merely waiting to be filled.

### Non-foreclosure rule

- **Label v0.1.0 assumptions as v0.1.0-only.** Write "v0.1.0 has one operator," "v0.1.0 ships a web cockpit and CLI," "v0.1.0 has one authority domain." Do not write the timeless "Patchbay has one operator" / "Patchbay ships a web cockpit" form, which silently promotes a v0.1.0 scope choice to permanent architecture.
- **Name reserved seams; do not omit them.** A reserved seam is present in the relevant registry (wire-present where forward-compatibility matters) and documented as reserved. Omitting a future direction from the registry forecloses it more than naming it reserved, because adding it later looks like new scope rather than the planned promotion of a seam.
- **Record rejected directions with rationale.** A rejected direction is written down with why, so a future promotion is visibly a reversal requiring a ceremony, not a quiet drift back in.
- **Treat parked ideas as pressure-test inputs, not v0.1.0 requirements.** `idea-multi-human-coordination`, `idea-desktop-app-surface`, `idea-agent-to-agent-mesh-seam`, and `idea-operator-customizable-ux-skins` inform the seam inventory; none is a v0.1.0 obligation. Each reserved-seam row that corresponds to a parked idea links back to it.

### Forward-compatibility hygiene

Where a wire/identity shape matters for a future variant, the v0.1.0 shape carries the future-relevant demarcator even though v0.1.0 has a single value, so the future capability arrives as a layer on top rather than a retroactive data migration:

- Event, cursor, and revision identity is the `(authority_domain_id, LSN)` tuple, not a bare LSN. v0.1.0 has one authority domain, but the key shape includes the domain demarcator so federation is additive.
- Reserved enum values (`agent-send`, `adapter-utility-exec`, `freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`) are wire-present in v0.1.0 proto and rejected at submission, so promoting them is a validation-rule change, not a wire-format change.
- Adapter capability manifests declare capability fields (`supported_operation_kinds`, snapshot tier, `session_replacement`, etc.) rather than the core assuming a single adapter's shape.
- Model intent stays portable across checker backends (Quint primary, TLA+ semantic baseline, Alloy relational) so a tool switch is not a model rewrite.

### What this discipline is not

This is a labeling and registry discipline, not a promise to implement any reserved seam. A reserved seam may never ship. The discipline's guarantee is narrower and load-bearing: a future direction that the operator chooses to pursue can be added as a registry/classification update rather than discovered to be impossible because v0.1.0 baked in a single-value assumption.
