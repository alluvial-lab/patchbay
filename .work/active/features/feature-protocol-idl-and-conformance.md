---
id: feature-protocol-idl-and-conformance
kind: feature
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-foundation-hardening
depends_on: [feature-verification-contract-authority, feature-session-identity-adapter-contract]
created: 2026-06-28
updated: 2026-06-29
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

## Acceptance criteria

- `contracts/` contains the v0 IDL/schema and generation instructions.
- Rust and TypeScript generation targets are documented, even if generated code packages are created in later implementation work.
- Conformance vectors exist in a stable location and are referenced from `docs/VERIFICATION.md`.
- Terminal-commit race vectors cover completion before cancellation, cancellation before completion, expiration before late completion, retry after terminal, late terminal candidate as audit/reconciliation only, and replay of the same committed prefix.
- No hand-written DTO set is introduced as the durable source of truth.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
