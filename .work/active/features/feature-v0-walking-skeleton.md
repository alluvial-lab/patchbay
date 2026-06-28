---
id: feature-v0-walking-skeleton
kind: feature
stage: drafting
tags: [prose, foundation]
parent: epic-foundation-hardening
depends_on: [story-bootstrap-substrates]
---

# Feature: Define the v0 walking skeleton

The foundation docs currently describe the Patchbay platform direction but not the first executable slice. Define v0 narrowly enough that implementation can begin without overbuilding.

## Required decisions

- Operator scope: single operator vs multi-operator for v0.
- Deployment topology: single authoritative core vs split/HA.
- First persistence backend and crash-recovery expectations.
- First adapter and command kinds.
- Required control surfaces for v0: web, CLI, or both.
- Explicit exclusions: native mobile, HA, multi-operator provisioning, arbitrary adapters, leases if deferred.

## Acceptance criteria

- `docs/SPEC.md` states the v0 walking skeleton and explicit non-goals.
- `docs/ARCHITECTURE.md` shows the v0 component slice separately from future architecture.
- `README.md` accurately reflects current status and v0 milestone.
- Follow-on work can tell whether it is inside or outside v0.
