---
id: feature-v0-core-acceptance
kind: feature
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-v0-core
depends_on: [feature-v0-core-persistence]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: Operation acceptance and command lifecycle

## Brief

Build the operation acceptance pipeline and command lifecycle state machine. A command accepted by Patchbay is durably recorded before delivery; after acceptance it remains visible as a `CommandState` until and after it reaches a terminal state. Acceptance creates a command record only after boundary validation, authority checking (via a grant-check port owned by the authority feature), idempotency reconciliation, and target identity binding (via a session-registry port owned by the sessions feature).

This feature owns the `CommandState` lifecycle (accepted → delivered → ... → terminal), idempotency-key dedup at the boundary, payload-equivalence checking, terminal-race resolution (first durable terminal commit wins via LSN ordering), and the failure vocabulary. It also owns observation ingestion — how adapter-reported Observations (output/events/status/terminal candidates) are written to the event log and reflected in command state. Elicitation lifecycle handling folds into this feature as part of the operation/observation/elicitations plane; if the scope is too large, `feature-design` may spawn a child story for elicitation specifically.

## Epic context

- Parent epic: `epic-v0-core`
- Position in epic: depends on persistence (for the event log and LSN). Interacts with authority (grant-check port) and sessions (target-identity port) through Ports & Adapters — those features implement ports this feature defines, so they can proceed in parallel.

## Formal-model backing

- `TerminalFinality` (promoted, `command_lifecycle.qnt`) — once a command reaches a terminal CommandState, later events do not mutate it
- `NoAcceptedToCompleted` (promoted, `command_lifecycle.qnt`) — a command cannot transition directly from `accepted` to `completed`; it must pass through `delivered` (or `running`)
- `BoundaryDedup` (promoted, `command_lifecycle.qnt`) — shared with persistence; the dedup boundary this feature enforces
- Idempotent retry, terminal races, session-identity binding — stated-normative obligations (v1 formal gate owns the real properties)

## Foundation references

- `docs/PROTOCOL.md` — Command lifecycle state; OperationKind registry; Submission outcome and local submission state; Acceptance semantics; Idempotency and retry; Cancellation, expiration, supersession, and race semantics; Failure and outcome vocabulary
- `docs/ARCHITECTURE.md` — Operation plane; Operation, Observation, and Elicitation plane
- `docs/VERIFICATION.md` — `TerminalFinality`, `NoAcceptedToCompleted`, `BoundaryDedup` promoted properties
- `contracts/proto/patchbay/operations.proto` — `Operation`, `OperationKind`, `OperationState`, `SubmissionOutcome`, `SubmissionResult`, `FailureCode`
- `contracts/proto/patchbay/observations.proto` — `Observation`, `ObservationKind`
- `contracts/proto/patchbay/elicitations.proto` — `Elicitation`, `ElicitationState`, `ResponseContract`
- `specs/seed/command_lifecycle.qnt` — `state`, `idemKey`, `appliedKeys`, `applyCount`, `lsn`, `terminalLsn`
- `specs/seed/elicitation_lifecycle.qnt` — stated-normative elicitation obligations
