---
id: epic-v0-core
kind: epic
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-v0-1-0-implementation
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Epic: Rust coordination core

## Brief

Build the single authoritative coordination core in Rust. This is the root of the v0.1.0 build-out — every other layer depends on it. The core owns the durable event log, operation acceptance with idempotency, authority checks, snapshots, and crash recovery. It is the only writer to the durable log; the web server, CLI, and adapters are clients.

The core implements the protocol semantics defined in `docs/PROTOCOL.md`: the `CommandState` lifecycle (accepted → delivered → ... → terminal), idempotent retry by command id + idempotency key, session connectivity × activity axes, generation monotonicity, snapshot/cursor reconciliation, and the failure vocabulary. It reads and writes through a storage port (Ports & Adapters) so domain semantics remain independent of the backend choice; the first backend may be embedded (file or embedded database).

## Why this is epic-sized

The core contains four feature-sized sub-arcs, each with its own formal-model backing and distinct design surface. `epic-design` decomposes this into child features (each with its own design gate, alternatives evaluation, and formal-model evaluation), not just child stories. This mirrors how `epic-foundation-hardening` decomposed the *design* of these same concerns into separate features — the implementation deserves the same treatment.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: root of the critical path. Nothing else starts until the core exists. The protocol seam, Pi adapter, web server, web cockpit, and CLI all depend on it.

## Formal-model backing

The 8 promoted (genuinely checked) properties directly constrain core internals:

- `command_lifecycle.qnt`: `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted` — terminal transition, boundary deduplication, no accepted→completed adjacency
- `session_generation.qnt`: `GenerationMonotonic` — session generation never decreases
- `csrf_browser.qnt`: `CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority` — browser-session/CSRF boundary

The 39 stated-normative properties (demoted during verification-claim-correction) are obligations the core must eventually satisfy but that do not yet have checked formulas; they inform the design without carrying checked-model status.

## Decomposition

Four feature-sized sub-arcs, each with its own formal-model backing and design surface. `epic-design` will confirm this decomposition and declare child-feature dependencies.

### Child features (provisional — epic-design confirms)

- `feature-v0-core-persistence` — durable event log, storage port, embedded backend, snapshots, crash recovery — formal backing: `BoundaryDedup` (promoted); crash/replay/snapshot convergence (stated-normative, v1 gate) — depends on: `[]`
- `feature-v0-core-acceptance` — CommandState machine, operation submission, idempotency, retry, terminal races — formal backing: `TerminalFinality`, `NoAcceptedToCompleted` (promoted) — depends on: `[feature-v0-core-persistence]`
- `feature-v0-core-authority` — grants, revocation, spawn authority, descendant grants, audit — formal backing: `RevokedSessionCannotCommand` (promoted); `authority.qnt` (stated-normative) — depends on: `[feature-v0-core-persistence]`
- `feature-v0-core-sessions` — session registry, connectivity × activity axes, generation monotonicity, replacement, stale/offline/unknown — formal backing: `GenerationMonotonic` (promoted) — depends on: `[feature-v0-core-persistence]`

### Decomposition risks

- **Persistence is the root.** Acceptance, authority, and sessions all read and write through the storage port and depend on the event log. The persistence feature must land (or at least its port interface must be designed) before the others can proceed.
- **Cross-cutting formal properties.** Some promoted properties span sub-arcs (e.g. `NoAcceptedToCompleted` touches acceptance + persistence). `epic-design` should map which properties each child feature owns vs. which are integration properties verified at the epic boundary.
- **CSRF/browser properties.** The four `csrf_browser.qnt` promoted properties are web-server-facing, not core-internal. They belong to `feature-v0-web-server`, not this epic; they're listed above only because they're part of the v0.1.0 promoted set. `epic-design` should clarify the boundary.

## Foundation references

- `docs/PROTOCOL.md` — Command lifecycle state, OperationKind registry, acceptance semantics, idempotency and retry, snapshots and streams, persistence and recovery, authority grants
- `docs/ARCHITECTURE.md` — v0.1.0 component slice, process topology, persistence topology
- `docs/SECURITY.md` — threat model, grants, revocation, audit
- `docs/VERIFICATION.md` — property-graded assurance, promoted vs stated-normative tiers
- `contracts/proto/patchbay/*.proto` — generated contract source (operations, sessions, observations, authority, elicitations, common, adapter)
- `contracts/rust/` — generated Rust bindings (the starting contract for the core's types)
- Formal models in `contracts/` — `command_lifecycle.qnt`, `session_generation.qnt`, `csrf_browser.qnt`, `elicitation_lifecycle.qnt`, `authority.qnt`
