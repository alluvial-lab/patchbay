---
id: feature-command-state-ssot
kind: feature
stage: drafting
tags: [prose, protocol, foundation, verification]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton]
---

# Feature: Define canonical command, session, and failure state machines

Review found command, session, and presentation states duplicated across README, architecture, protocol, verification, and UX with divergent members. Consolidate these into one source of truth.

## Scope

- Core command lifecycle states and allowed transitions.
- Control-surface-local states such as draft/submitting.
- Session liveness states such as live, idle, working, stale, offline, unknown.
- Failure/outcome vocabulary across transport, acceptance, delivery, execution, and presentation.
- Cancellation, expiration, supersession, running, and completion race semantics.

## Acceptance criteria

- `docs/PROTOCOL.md` owns the canonical state machines.
- `docs/UX.md`, `docs/ARCHITECTURE.md`, and `docs/VERIFICATION.md` reference rather than redefine them.
- `docs/GLOSSARY.md` defines ambiguous terms such as `superseded`, `unknown`, and `running`.
- The state machines are concrete enough to become TLA+/Quint variables and generated TS/Rust enums.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
