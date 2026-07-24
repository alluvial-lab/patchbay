---
id: feature-v0-walking-skeleton
kind: feature
stage: done
tags: [prose, foundation]
parent: epic-foundation-hardening
depends_on: [story-bootstrap-substrates]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: v0.1.0
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

## Implementation notes

- Files changed: `docs/SPEC.md`, `docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`, `README.md`.
- Tests added: none; docs-only prose feature.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: proofread changed sections in context; verified `story-bootstrap-substrates` is done; checked required v0 terms and exclusions are present with `rg`; fresh-context review found no blockers and review follow-ups were addressed inline.

## Review (2026-06-28)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none outstanding; fresh-context review identified undefined `authority domain` and lease/v0 verification-scope ambiguity, both addressed inline in `docs/GLOSSARY.md`, `docs/SPEC.md`, and `docs/VERIFICATION.md`.
**Nits**: README lease wording was aligned with SPEC/ARCHITECTURE; duplicate README milestone summaries remain acceptable for README orientation.

**Notes**: Deep substrate review using fresh-context sub-agent (`umans/umans-glm-5.2`). Acceptance criteria met: SPEC defines the v0 walking skeleton and non-goals, ARCHITECTURE separates the v0 component slice, README reflects status and milestone, and follow-on work has an inside/outside-v0 rule.
