---
id: feature-v0-core
kind: feature
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-v0-1-0-implementation
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: Rust coordination core

## Brief

Build the single authoritative coordination core in Rust. This is the root of the v0.1.0 build-out — every other layer depends on it. The core owns the durable event log, operation acceptance with idempotency, authority checks, snapshots, and crash recovery. It is the only writer to the durable log; the web server, CLI, and adapters are clients.

The core implements the protocol semantics defined in `docs/PROTOCOL.md`: the `CommandState` lifecycle (accepted → delivered → ... → terminal), idempotent retry by command id + idempotency key, session connectivity × activity axes, generation monotonicity, snapshot/cursor reconciliation, and the failure vocabulary. It reads and writes through a storage port (Ports & Adapters) so domain semantics remain independent of the backend choice; the first backend may be embedded (file or embedded database).

This is the largest piece of the v0.1.0 build-out. It has the strongest formal-model backing and the most internal surface area. `feature-design` may decompose it into child stories (persistence/event-log, operation acceptance + idempotency, authority, snapshots + crash recovery) if the scope warrants.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: root of the critical path. Nothing else starts until the core exists. The protocol seam, Pi adapter, web server, web cockpit, and CLI all depend on it.
- This feature may decompose into child stories during `feature-design`.

## Formal-model backing

The 8 promoted (genuinely checked) properties directly constrain core internals:

- `command_lifecycle.qnt`: `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted` — terminal transition, boundary deduplication, no accepted→completed adjacency
- `session_generation.qnt`: `GenerationMonotonic` — session generation never decreases
- `csrf_browser.qnt`: `CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority` — browser-session/CSRF boundary

The 39 stated-normative properties (demoted during verification-claim-correction) are obligations the core must eventually satisfy but that do not yet have checked formulas; they inform the design without carrying checked-model status.

## Foundation references

- `docs/PROTOCOL.md` — Command lifecycle state, OperationKind registry, acceptance semantics, idempotency and retry, snapshots and streams, persistence and recovery, authority grants
- `docs/ARCHITECTURE.md` — v0.1.0 component slice, process topology, persistence topology
- `docs/SECURITY.md` — threat model, grants, revocation, audit
- `docs/VERIFICATION.md` — property-graded assurance, promoted vs stated-normative tiers
- `contracts/proto/patchbay/*.proto` — generated contract source (operations, sessions, observations, authority, elicitations, common, adapter)
- `contracts/rust/` — generated Rust bindings (the starting contract for the core's types)
- Formal models in `contracts/` — `command_lifecycle.qnt`, `session_generation.qnt`, `csrf_browser.qnt`, `elicitation_lifecycle.qnt`, `authority.qnt`
