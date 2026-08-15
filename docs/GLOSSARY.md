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

A core-assigned, nonzero, opaque storage-continuity epoch persisted per authority domain. It is created when that storage lineage is first opened and remains stable across ordinary process restarts. Snapshots carry it so reconciliation can reject derived state from another storage history; equality is a compatibility fence, not an ordering, authority, or bearer-secret mechanism. A history-discontinuity rollover and a distinct process-incarnation/fencing identity for HA remain reserved seams. See Generation for the unified entry covering all generation scopes.

## Cursor

A log sequence number expressing authoritative knowledge of the durable log up
to that point. Control surfaces and adapters carry cursors for reconnect
reconciliation. The operational-resource projection also reconstructs one
internal domain-qualified applied cursor from the shared log: it records the
highest contiguous LSN the projection has validated, makes covered re-feed
inert, and is neither a wire field nor a persisted checkpoint.

## Generation

A new lifetime of an entity that retains its identity. Patchbay uses generation at four scopes, each with a different assigner — the assigner is the structurally important fact and what the verification properties check:

- **Core generation** — a nonzero storage-continuity epoch, **core-assigned and durably persisted when an authority-domain storage lineage is first opened**. Ordinary process restarts reuse it. A destructive restore, divergent fork, authoritative-store replacement, or future multi-core promotion must explicitly roll the epoch before serving derived snapshots/cursors; process-incarnation fencing for HA is a distinct future concept.
- **Runtime-session generation** — an incarnation of one runtime session, **adapter-reported on replacement**. Used to tombstone a superseded session so late events/replies binding to it are `stale_event` audit records and cannot mutate the live generation.
- **Operator-session generation** — a core-assigned, monotonic incarnation of one authenticated browser or CLI operator session. All-session revocation persists an invalidated-through floor; restart replay preserves the floor while opaque session ids remain process-local.
- **Adapter generation** — an incarnation of the adapter process, **adapter-reported on re-attach**. Used to reject stale events from a prior adapter attachment.

The four scopes share the concept (a new lifetime) but differ in who can observe the restart, so they differ in assigner. The qualifier (core / runtime-session / operator-session / adapter) is the collision-protection discipline.

## Logical target

A core-owned stable identity that binds a managed spawn lineage across runtime-session generations. It has one current generation, at most one reserved successor, and retained tombstones. Intentional restart names both the logical target and exact current external runtime in a new spawn continuation; only atomic promotion moves current from N to N+1.

## Restart as continuation

An intentional replacement expressed as a new `spawn` Operation with a new command/idempotency key and exact continuation prior. The generic core owns claim/fence/evidence/promotion semantics; the adapter owns how it terminates, replaces, and reconciles the runtime. The adapter fills the generated continuation-context status (`resumed`, `new_context`, or `unknown`) on the exact successor report; staged/promotion evidence preserves it without a core default. It never guarantees arbitrary process-state restoration.

## Session report source cursor

Adapter-authenticated ordering evidence for one complete runtime-session report: `(adapter_generation, revision)` inside one runtime-session generation. The positive adapter-assigned revision increases strictly within the current producer epoch; a newer authenticated adapter generation or runtime-session generation may restart it. The cursor is durably restored in the session projection and snapshot. It is not a core log cursor, LSN/revision, timestamp, or bearer credential.

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

A declaration an adapter makes about the targets, Operations, and guarantees it supports: generated target categories; supported OperationKinds (and, for `spawn`, supported `target_spec.shape` values); streaming, cancellation, and session-replacement support; a runtime-session snapshot tier; exact per-`ResourceKind` snapshot tiers and resource projection contracts; idempotency strength; attachment method; diagnostic reporting; and known failure modes. Capability declarations are advisory for control-surface UX only — they are not an authority gate and not a delivery gate. The adapter is the authority on its own support, reported at delivery time.

## Adapter target category

The closed generated registry that classifies the canonical Patchbay contract an adapter target uses. `runtime_session` and `operational_resource` are admitted. `knowledge_bundle` is wire-present but registration-rejected, with OKF v0.2 named as the candidate format for a future promotion. Adapter-owned provider, pool, and window names are `ResourceKind`s beneath `operational_resource`, not target categories.

## Resource projection contract

The per-resource declaration that binds one exact `ResourceKind` to the mandatory `operational_resource` composition target plus payload and domain-projection schema descriptors. A local known compositor may interpret the projection inside Patchbay's canonical identity/revision/staleness/authority/attention/Operation wrapper. The contract does not load adapter UI code or grant authority.

## Schema descriptor

A bounded non-empty schema reference plus a generated payload content type. Exact matching establishes that an envelope uses the format declared in the manifest; it does not prove opaque bytes semantically satisfy that schema. Typed decoders remain responsible for fail-closed semantic validation.

## Correlation context

The authority/session scope in which a reply's typed correlation reference must resolve to a known prior command or message id. A reply cannot forge correlation across id spaces (a reply id cannot masquerade as a command id) or across session/authority contexts. Response Operations to Elicitations use a typed correlation reference to a known `ElicitationId` in the same authority/session/responder context. `TypedCorrelation` reserves both correlation shapes across disjoint id spaces as stated-normative; its model has no promoted formula until independent attempted correlation evidence is represented. See `docs/PROTOCOL.md` Operations, Observations, Elicitations, payloads, and correlation.

## Event

A durable record of an accepted state transition.

## Grant

An authority relationship permitting a subject (an actor, optionally narrowed to an endpoint or endpoint class) to perform specific OperationKinds against a target scope. v0.1.0 spawn Operations explicitly select one attached adapter and require a matching spawn grant; fleet-default target selection is reserved. Successful spawn records an auto-issued descendant grant for the spawned session. See `docs/PROTOCOL.md`.

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

## ResourceId

An adapter-local typed scalar naming one resource inside an adapter-owned resource kind. It is not globally routable by itself.

## ResourceKind

A non-empty open identifier for an adapter-owned resource collection or type. The adapter capability manifest owns the admitted set; ResourceKind is not a core enum.

## ResourceIdentity

The full routable operational-resource tuple `(adapter_id, resource_kind, resource_id)`. Equality, resolution, grant containment, delivery routing, and idempotency scoping use the complete tuple. It carries no runtime-session generation. Protobuf tag 8's `legacy_audit_resource_id` is an audit-only control-surface target and is not a ResourceIdentity.

## Resource freshness

Patchbay's confidence in one cached resource record: `current`, `stale`, or
`unknown`. It is reconciliation state, not adapter-owned domain health. An
exhausted provider pool may be current; a healthy-looking cached pool may be
stale.

## Resource view

The collection projection for one exact `(adapter_id, ResourceKind)`. Its
completeness (`authoritative`, `partial`, or `none`), source adapter generation,
observed time, and core-assigned revision LSN determine how reconnect omissions
are reconciled. A resource kind never inherits another kind's or the runtime
session's tier.

## Resource report

Authenticated typed adapter evidence with a reconnect `snapshot` or live
`delta` variant. Patchbay validates it against the current attachment and exact
manifest declaration, normalizes it into one durable `RESOURCE_STATE` event,
and folds only after append.

## Resource tombstone and replacement

A terminal retirement of one exact `ResourceIdentity`. The retired record stays
in snapshots/audit context but does not resolve. Replacement requires a
distinct same-adapter identity whose upsert commits atomically with the old
record's tombstone; the old record retains the `replaced_by` link.

## Resource snapshot

The stable-ordered authoritative Patchbay projection of resource records and
per-view revisions at one authority-domain LSN. It is selected explicitly by
`SnapshotViewKind = resource` and is distinct from `SessionSnapshot`; the
current implementation materializes it on demand from durable resource events.

## Operator session

An authenticated browser or CLI session for the operator, represented by a server-side record and bound to an actor, endpoint, device, endpoint generation, and core-assigned operator-session generation. Opaque session ids are process-local bearer references and are not restored after restart. It is the continuity mechanism for a control surface, not a substitute for command grants.

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

A policy action that prevents future authority for an operator session, principal, endpoint, device, grant, adapter, or target scope. Session/principal/endpoint/device revocation uses `continue` for already accepted work. Revocation does not erase command history; already accepted commands follow the relevant revocation policy. CLI self-lockout recovery uses a distinct unrevoked identity or fresh endpoint/device configuration; the one-time setup secret is not a reset mechanism.

## Revision

The core log sequence number at which a specific view (command, session, actor, grant, audit record) was last durably updated. Used to decide whether a snapshot or cached view is older than the core's current state for that view. This core-owned revision is distinct from the adapter-assigned revision inside a Session report source cursor: LSN orders durable arrival, while the source revision orders production before arrival.

## Running

A non-terminal command state meaning the target adapter or runtime reports active execution for an accepted command. Running does not imply success; it remains observable until a terminal state or policy-driven resolution is recorded.

## Runtime session

An external session, process, harness, job, or agent context controlled through an adapter.

## Security lockdown

A domain-keyed, durable emergency posture. While active, Patchbay rejects every new Operation before acceptance (including retries and `QueryDiagnostics`) with `authorization_denied/security_lockdown_active`, marks affected runtime sessions stale, invalidates existing operator-session generations, and records only a bounded reason code. Fresh login can inspect read-only snapshots/subscriptions but cannot submit or mutate. Exit is exclusively the configured bootstrap channel; v0.1.0 uses the loopback `AdminService` via `patchbay-cli lockdown-exit`, never routine web re-authentication.

## Bootstrap channel

The separately protected trust boundary used to establish or recover the operator's highest-authority state. In v0.1.0 the wire value is `loopback_admin`; it is distinct from routine ControlService/web login and is the only channel allowed to exit security lockdown.

## Snapshot

An authoritative state view used to recover from missed events, reconnects, and stale UI state. Durable session checkpoints are disposable derived snapshots stored in a private typed, versioned envelope containing complete live `SessionRegistry` state, retained generation tombstones, and explicit managed-lineage provenance. Only exact domain/core-generation/LSN and semantic validation may seed session tail replay: marked managed tombstones require exact session/logical-target symmetry, while unmarked logical-target tombstones are disposable rather than inferred from ambiguous shape. Legacy undiscriminated, otherwise incompatible bytes, or a derived seed that disagrees with its authoritative tail replay sessions from LSN 0. The best-effort latest-only writer targets each 256 observed events under healthy scheduling, but sibling projections still full-replay, so this is not a whole-core recovery bound. No checkpoint, replica, or projection is an independent ordering or authority source.

## Stale

A state where cached information exists but has not been confirmed by a sufficiently recent authoritative snapshot or live signal.

## Superseded

A terminal command state meaning a newer accepted command or explicit policy decision replaced an earlier command. Superseded commands are not pending, cancelled, failed, or completed; they are visible historical records of work intentionally replaced before completion.

## Unknown

A state used when the control surface or Patchbay lacks enough authoritative information to classify a submission, command, session connectivity, or session activity. Unknown must not be rendered as success, failure, live, or denied without reconciliation against core state or snapshots.
