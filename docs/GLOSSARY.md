# Patchbay Glossary

## Adapter

A boundary component that connects Patchbay to an external runtime, harness, tool, service, or control surface. Pi is the first adapter target.

## Actor

A represented participant in Patchbay: operator, agent, adapter, daemon, service, or control surface.

## Audit record

A durable security or operational record of a decision, attempt, or observation. Audit records are distinct from command/session state-transition events and may record rejected attempts that never created command records.

## Authority domain

A bounded Patchbay control context within which grants, revocation, routing authority, and any exclusive coordination claims are evaluated against one authoritative core state. V0 has one operator and one authority domain; future multi-human or federated deployments must define how authority domains are created, joined, delegated, audited, and isolated.

## Command

Operator intent that may cause action. Commands require target identity, authority, validation, and idempotency semantics.

## Control surface

A human-facing interface such as web, CLI, future mobile app, desktop app, notification surface, or approval UI.

## Core generation

A marker of the coordination core's current incarnation, used to reject snapshots or events from a prior incarnation outright during reconciliation. See Generation for the unified entry covering all three scopes.

## Cursor

A log sequence number a control surface or adapter holds to express that it has authoritative knowledge of the durable log up to that point, used to drive reconciliation on reconnect.

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

## Adapter capability

A declaration an adapter makes about the commands and guarantees it supports: command kinds; streaming, cancellation, and session-replacement support (boolean); snapshot support (authoritative / partial / none); idempotency strength (none / at-Patchbay-boundary / end-to-end); attachment method; and known failure modes. Capability declarations are advisory for control-surface UX only — they are not an authority gate and not a delivery gate. The adapter is the authority on its own support, reported at delivery time.

## Correlation context

The authority/session scope in which a reply's typed correlation reference must resolve to a known prior command or message id. A reply cannot forge correlation across id spaces (a reply id cannot masquerade as a command id) or across session/authority contexts. See `docs/PROTOCOL.md` Messages, commands, and replies.

## Event

A durable record of an accepted state transition.

## Grant

An authority relationship permitting a subject (an actor, optionally narrowed to an endpoint or endpoint class) to perform specific command kinds against a target.

## Idempotency key

A stable key that lets Patchbay recognize a retry of the same command and prevent accidental double-application at the coordination boundary.

## LSN

Log sequence number. A monotonic, gap-free number assigned by the coordination core to each accepted state-transition event at durable-commit time. The canonical ordering for first-terminal-commit-wins and for snapshot reconciliation.

## Lease

A time-bounded exclusive claim over a resource or coordination role.

## Operator

The human using Patchbay to inspect, control, approve, or coordinate agent sessions and runtime work.

## Operator session

An authenticated browser or CLI session for the operator, represented by a server-side record and bound to an endpoint. It is the continuity mechanism for a control surface, not a substitute for command grants.

## Patchbay core

The coordination layer that owns actor/session registry, durable events, command state, authority checks, snapshots, and leases.

## Principal

A security-facing shorthand for an actor or endpoint being authorized. Patchbay foundation docs prefer the more precise terms actor, device, endpoint, operator session, runtime session, and grant.

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
