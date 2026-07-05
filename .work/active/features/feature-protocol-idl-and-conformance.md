---
id: feature-protocol-idl-and-conformance
kind: feature
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-foundation-hardening
depends_on: [feature-verification-contract-authority, feature-session-identity-adapter-contract, feature-operator-presence-and-action-inventory]
created: 2026-06-28
updated: 2026-07-05
gate_origin: null
release_binding: null
---

# Feature: Author v0 protocol IDL and conformance vectors

Patchbay's generated-contract posture requires actual schema/IDL artifacts, generated boundary types, and conformance vectors before Rust core, TypeScript operator domain, or adapters implement durable protocol behavior.

## Scope

- Create the v0 protocol IDL/schema using the contract source selected by `feature-verification-contract-authority`.
- Define initial wire contracts for actors/endpoints, sessions, commands, replies, events, snapshots, grants, and adapter capabilities that are in v0 scope.
- Establish generation targets for Rust core types and TypeScript client/operator-domain types.
- Produce golden conformance vectors for command acceptance, reply correlation, snapshot reconciliation, terminal-commit race resolution, and failure/outcome mapping.
- Document how generated contracts relate to prose semantics and formal models.

### Normative registry inheritance

This feature **inherits the normative action registry** from `feature-operator-presence-and-action-inventory`. It does **not** invent a separate command/action-kind list. The product-vocabulary registry is authored in `docs/PROTOCOL.md` (Operation, Observation, Elicitation, Payload, `OperationKind` registry, `ElicitationState` lifecycle, `response_contract` registry, the five id spaces, and the Presence/Subscription axes). This feature's `.proto` enum/wire representation derives from that registry: if `.proto` needs a new action kind, the product-vocabulary registry in `docs/PROTOCOL.md` changes first, then `.proto`, models, vectors, and implementation follow.

The original Q4 ("what are the command/action kinds?") is **dissolved**: it is answered by consuming `OperationKind`, `ElicitationState`, `response_contract.contract_kind`, and Presence/Subscription registries from the foundation work rather than by introducing a parallel enum here.

## Acceptance criteria

- `contracts/` contains the v0 IDL/schema and generation instructions.
- Rust and TypeScript generation targets are documented, even if generated code packages are created in later implementation work.
- Conformance vectors exist in a stable location and are referenced from `docs/VERIFICATION.md`.
- Terminal-commit race vectors cover completion before cancellation, cancellation before completion, expiration before late completion, retry after terminal, late terminal candidate as audit/reconciliation only, and replay of the same committed prefix.
- No hand-written DTO set is introduced as the durable source of truth.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
