# Patchbay Glossary

## Adapter

A boundary component that connects Patchbay to an external runtime, harness, tool, service, or control surface. Pi is the first adapter target.

## Actor

A represented participant in Patchbay: operator, agent, adapter, daemon, service, or control-surface endpoint.

## Authority domain

A bounded Patchbay control context within which grants, revocation, routing authority, and any exclusive coordination claims are evaluated against one authoritative core state. V0 has one operator and one authority domain; future multi-human or federated deployments must define how authority domains are created, joined, delegated, audited, and isolated.

## Command

Operator intent that may cause action. Commands require target identity, authority, validation, and idempotency semantics.

## Control surface

A human-facing interface such as web, CLI, future mobile app, desktop app, notification surface, or approval UI.

## Endpoint

A concrete connection or addressable runtime instance for an actor.

## Event

A durable record of an accepted state transition.

## Grant

An authority relationship permitting an actor or endpoint to perform specific actions on a target.

## Idempotency key

A stable key that lets Patchbay recognize a retry of the same command and prevent accidental double-application at the coordination boundary.

## Lease

A time-bounded exclusive claim over a resource or coordination role.

## Operator

The human using Patchbay to inspect, control, approve, or coordinate agent sessions and runtime work.

## Patchbay core

The coordination layer that owns actor/session registry, durable events, command state, authority checks, snapshots, and leases.

## Runtime session

An external session, process, harness, job, or agent context controlled through an adapter.

## Snapshot

An authoritative state view used to recover from missed events, reconnects, and stale UI state.

## Stale

A state where cached information exists but has not been confirmed by a sufficiently recent authoritative snapshot or live signal.
