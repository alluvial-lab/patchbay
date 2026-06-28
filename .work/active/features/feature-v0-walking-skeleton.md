---
id: feature-v0-walking-skeleton
kind: feature
stage: implementing
tags: [prose, foundation]
parent: epic-foundation-hardening
depends_on: [story-bootstrap-substrates]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
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

## Outline

Target files:

- `docs/SPEC.md` — add a concrete v0 walking skeleton section and reconcile starting scope/non-goals.
- `docs/ARCHITECTURE.md` — add a v0 component slice separate from the future architecture planes.
- `README.md` — update current status and first milestone language.

V0 decisions to encode:

- One human operator for v0; multi-human coordination is explicitly deferred but not foreclosed.
- Single authoritative coordination core for v0; split/HA deployments are deferred.
- Local durable event/snapshot store for v0; backend is abstracted behind ports so the first implementation does not leak storage assumptions into domain logic.
- Pi adapter first; initial command kinds cover message/prompt send, cancel/interrupt where supported, and snapshot/status refresh.
- Responsive web cockpit first, CLI for admin/debug/scripted control; no native mobile app in v0.
- Leases are not in the v0 executable skeleton unless a later feature explicitly promotes them; lease semantics remain modeled for future coordination.

## Acceptance criteria

- `docs/SPEC.md` states the v0 walking skeleton and explicit non-goals.
- `docs/ARCHITECTURE.md` shows the v0 component slice separately from future architecture.
- `README.md` accurately reflects current status and v0 milestone.
- Follow-on work can tell whether it is inside or outside v0.

## Related parked ideas

- `idea-multi-human-coordination` — v0 remains single-operator unless this feature decides otherwise, but the foundation should not foreclose future multi-human authority domains, grants, audit, handoffs, or third-party coordination surfaces.
