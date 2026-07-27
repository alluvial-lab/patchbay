# Patchbay Glossary

## Adapter

A boundary component that connects Patchbay to an external runtime, harness, tool, service, or control surface. Pi is the first adapter target.

## Actor

A represented participant in Patchbay: operator, agent, adapter, daemon, service, or control surface.

## Audit record

A durable security or operational record of a decision, attempt, or observation. Audit records are distinct from command/session state-transition events and may record rejected attempts that never created command records.

## Authority domain

A bounded Patchbay control context within which grants, revocation, routing authority, and any exclusive coordination claims are evaluated against one authoritative core state. v0.1.0 has one operator and one authority domain; future multi-human or federated deployments must define how authority domains are created, joined, delegated, audited, and isolated.

## Command

A Patchbay lifecycle record for an accepted authorized request, currently used by the checked `CommandState` formal model. The actor-neutral protocol vocabulary is Operation; `CommandState` remains the checked lifecycle registry until a coordinated rename. A harness slash-command is different: it is payload text interpreted by a harness and has no Patchbay authority by itself.

## Control surface

A human-facing interface such as web, CLI, future mobile app, desktop app, notification surface, or approval UI.

## Core generation

A marker of the coordination core's current incarnation, used to reject snapshots or events from a prior incarnation outright during reconciliation. See Generation for the unified entry covering all three scopes.

## Cursor

A log sequence number a control surface or adapter holds to express that it has authoritative knowledge of the durable log up to that point, used to support reconciliation on reconnect.

## Generation

A new lifetime of an entity that retains its identity. Patchbay uses generation at three scopes, each with a different assigner — the assigner is the structurally important fact and what the verification properties check:

- **Core generation** — the coordination core's own incarnation, **core-assigned on restart**. Used to reject snapshots or events from a prior core incarnation outright during reconciliation.
- **Session generation** — an incarnation of one runtime session, **adapter-reported on replacement**. Used to tombstone a superseded session so late events/replies binding to it are `stale_event` audit records and cannot mutate the live generation.
- **Adapter generation** — an incarnation of the adapter process, **adapter-reported on re-attach**. Used to reject stale events from a prior adapter attachment.

The three scopes share the concept (a new lifetime) but differ in who can observe the restart, so they differ in assigner. The qualifier (core / session / adapter) is the collision-protection discipline.

## Device

A physical or virtual host that can run one or more endpoints, such as a browser on a laptop, a CLI on a VM, or an adapter process near a runtime.

## Endpoint

A concrete connection or addressable runtime instance for an actor on a device.

## Elicitation

A durable pending response solicitation from one actor/system component to another. It opens a response slot rather than answering a prior request. v0.1.0 Elicitations bind to the operator actor (not a specific endpoint), deliver by subscription fan-out to all subscribed surfaces, and clear everywhere on first answer. The opener is always an adapter/agent/harness in v0.1.0; the core does not open Elicitations. Core prompts (lockdown, expired/revoked sessions, CSRF rejection) are NOT Elicitations. See `docs/PROTOCOL.md`.

## ElicitationId

A new id space, adapter-assigned when a pending response slot is opened. The core assigns an LSN when it durably records the Elicitation; it does not assign the `ElicitationId` in v0.1.0. Separate from CommandId/MessageId/ReplyId/EventId to prevent forgery and preserve initiation-vs-response direction.

## ElicitationState

The lifecycle registry for an Elicitation. Initial state is `opened`; transitions include `opened` → `pending` or direct `opened` → terminal, and `pending` → terminal (`answered`, `declined`, `expired`, `cancelled`, `withdrawn`, `superseded`, `stale`). First durable terminal commit wins; first valid answer clears the Elicitation for all subscribed surfaces. The Elicitation lifecycle properties are currently stated-normative with no executable formula: their seed formulas inspected state recorded by the accepting action rather than independent attempted evidence and were not mutation-survivable oracles. The v1 formal gate owns the genuine formulas. None of these properties are checked-normative until promoted conformance vectors land.

## Adapter capability

A declaration an adapter makes about the Operations and guarantees it supports: supported OperationKinds (and, for `spawn`, supported `target_spec.shape` values); streaming, cancellation, and session-replacement support (boolean); snapshot support (authoritative / partial / none); idempotency strength (none / at-Patchbay-boundary / end-to-end); attachment method; and known failure modes. Capability declarations are advisory for control-surface UX only — they are not an authority gate and not a delivery gate. The adapter is the authority on its own support, reported at delivery time.

## Correlation context

The authority/session scope in which a reply's typed correlation reference must resolve to a known prior command or message id. A reply cannot forge correlation across id spaces (a reply id cannot masquerade as a command id) or across session/authority contexts. Response Operations to Elicitations use a typed correlation reference to a known `ElicitationId` in the same authority/session/responder context. `TypedCorrelation` reserves both correlation shapes across disjoint id spaces as stated-normative; its model has no promoted formula until independent attempted correlation evidence is represented. See `docs/PROTOCOL.md` Operations, Observations, Elicitations, payloads, and correlation.

## Event

A durable record of an accepted state transition.

## Grant

An authority relationship permitting a subject (an actor, optionally narrowed to an endpoint or endpoint class) to perform specific OperationKinds against a target scope. Spawn grants are fleet-level by default in v0.1.0; successful spawn records an auto-issued descendant grant for the spawned session. See `docs/PROTOCOL.md`.

## Harness slash-command

Text such as `/compact`, `/model`, `/review`, or `!cmd` carried inside an Operation payload and interpreted by a harness. It is not a Patchbay protocol kind and has no Patchbay authority by itself.

## Idempotency key

A stable key that lets Patchbay recognize a retry of the same command and prevent accidental double-application at the coordination boundary.

## LSN

Log sequence number. A monotonic, gap-free number assigned by the coordination core to each accepted state-transition event at durable-commit time. The canonical ordering for first-terminal-commit-wins and for snapshot reconciliation.

## Message

Generic operator-originated no-grant Message is not a v0.1.0 action: it is rejected for v0.1.0 because no surveyed harness exposes it as a distinct operator action. The `message id` space remains reserved for future informational surfaces and current correlation-model compatibility. Contrast with `instruct` (an authorized Operation carrying prompt/input payload) and Elicitation (an agent/adapter-opened response slot).

## Lease

A time-bounded exclusive claim over a resource or coordination role. v0.1.0 does not implement leases; see `docs/PROTOCOL.md` § Leases for the precondition framing.

## Operator

The human using Patchbay to inspect, control, approve, or coordinate agent sessions and runtime work.

## Observation

A source-authenticated fact, event, output, status emission, reply-like result, or lifecycle/status fact emitted by an actor, adapter, core, runtime, or service. Observations do not grant authority to act. Live streams are delivery optimizations; durable core records and snapshots remain authoritative. See `docs/PROTOCOL.md`.

## Operation

An authorized control-plane request by an actor to an actor, core, adapter, fleet, session, service, or resource target. v0.1.0 Operations are operator-originated; non-operator Operation senders (agent→agent, adapter→operator service Operations) are a reserved seam. An accepted Operation reuses the `CommandState` lifecycle by documented refinement equivalence. See `docs/PROTOCOL.md`.

## OperationKind

A registry-owned kind of Operation: `spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management` (committed v0.1.0), plus reserved `agent-send` and `adapter-utility-exec`. Unknown or reserved-but-not-validatable kinds are `validation_failed` at submission. See `docs/PROTOCOL.md`.

## Operational resource

An adapter-reported non-session target whose state materially governs agent availability, capability, or safe control, or requires human action to keep agent work operating. Provider-capacity pools, contribution/credential health, and model availability are examples. Resource identity and domain health are distinct from runtime-session identity and connectivity/activity; an exhausted resource is not an offline session. Patchbay owns durable Operations, authority, correlation, reconciliation, and attention around the target while the adapter owns its domain schema and policy.

## Operator session

An authenticated browser or CLI session for the operator, represented by a server-side record and bound to an endpoint. It is the continuity mechanism for a control surface, not a substitute for command grants.

## Patchbay core

The coordination layer that owns actor/session registry, durable events, command state, authority checks, snapshots, and leases.

## Payload

The adapter-specific content or schema-bound body carried inside an Operation, Observation, or Elicitation — prompt text, slash-command text, tool-call arguments, function results, image/file references, question options, structured schemas, or adapter diagnostics. Payload does not itself grant authority, create lifecycle state, or define protocol kinds.

## Presence

A derived fact (from endpoint observations and session connectivity state), not a query target. One-shot "is session X present?" reads route through snapshot/status `query` Operations; there is no `query-presence` OperationKind. Single-operator v0.1.0 has no presence-leak threat; filter-scoped subscriptions for multi-operator presence-leak prevention are a reserved seam.

## Principal

A security-facing shorthand for an actor or endpoint being authorized. Patchbay foundation docs prefer the more precise terms actor, device, endpoint, operator session, runtime session, and grant.

## Response contract

A `response_contract` describes what kind of response is semantically required: committed v0.1.0 contract kinds are `approval` and `question`; reserved contract kinds are `freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`. UI hints (select-one, select-many, free-text, upload, draw) are optional open-set sub-fields of `question`/`approval`, not contract kinds. See `docs/PROTOCOL.md`.

## Subscription

A grant-checked transport-layer establishment that delivers an event/snapshot stream to a control surface. Subscriptions are grant-checked at establish, audited, reconciled by cursor on reconnect, and NOT durably recorded as Operations or entered into `OperationState`. This is the second authority mechanism (grant-checked-without-lifecycle) alongside grant-checked-with-lifecycle Operations/Elicitations.

## Revocation

A policy action that prevents future authority for an operator session, endpoint, grant, adapter, or target scope. Revocation does not erase command history; already accepted commands follow the relevant revocation policy.

## Revision

The log sequence number at which a specific view (command, session, actor, grant, audit record) was last durably updated. Used to decide whether a snapshot or cached view is older than the core's current state for that view.

## Running

A non-terminal command state meaning the target adapter or runtime reports active execution for an accepted command. Running does not imply success; it remains observable until a terminal state or policy-driven resolution is recorded.

## Runtime session

An external session, process, harness, job, or agent context controlled through an adapter.

## Security lockdown

An emergency posture where Patchbay rejects new commands, marks affected runtime sessions stale, requires fresh authentication or operator action, and records the reason in audit history.

## Snapshot

An authoritative state view used to recover from missed events, reconnects, and stale UI state.

## Stale

A state where cached information exists but has not been confirmed by a sufficiently recent authoritative snapshot or live signal.

## Superseded

A terminal command state meaning a newer accepted command or explicit policy decision replaced an earlier command. Superseded commands are not pending, cancelled, failed, or completed; they are visible historical records of work intentionally replaced before completion.

## Unknown

A state used when the control surface or Patchbay lacks enough authoritative information to classify a submission, command, session connectivity, or session activity. Unknown must not be rendered as success, failure, live, or denied without reconciliation against core state or snapshots.
