---
id: epic-foundation-hardening
kind: epic
stage: implementing
tags: [foundation]
depends_on: []
parent: null
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Epic: Foundation hardening after adversarial review

Patchbay's initial docs establish the right product direction, but review found that implementation should not begin until the starting slice, protocol state machines, security model, persistence/snapshot semantics, and verification source-of-truth are sharpened.

This epic tracks the refinement program that converts the current foundation docs from a strong vision into an executable starting-state project.

## Review synthesis

Fresh-context top-down and bottom-up review converged on these concerns:

- Define a concrete v0 walking skeleton rather than an entire platform program.
- Consolidate command/session/failure state into one source of truth.
- Specify persistence, ordering, snapshots, and crash recovery before relying on durable acceptance.
- Define security principals, grants, device/browser identity, threat model, and revocation.
- Pin session identity/generation semantics so wrong-session prevention is implementable.
- Define adapter capability tiers and Pi parity without letting Pi become the core ontology.
- Decide how prose, formal models, generated contracts, and conformance vectors relate.
- Split UX presentation state into session liveness vs command delivery and make v0 screens actionable.
- Decide whether leases are v0 scope or deferred.

## Acceptance criteria

- Foundation docs define a buildable v0 slice with explicit exclusions.
- `docs/PROTOCOL.md` contains canonical state-machine and identity semantics rather than scattered enum-like lists.
- `docs/SECURITY.md` or equivalent defines v0 threat model and principal/grant posture.
- `docs/ARCHITECTURE.md` defines v0 persistence/topology/snapshot ordering assumptions.
- `docs/VERIFICATION.md` maps v0 models to normative artifacts and conformance checks.
- `docs/UX.md` has v0 cockpit acceptance criteria and separates session and command presentation states.
- Follow-on implementation can begin without inventing protocol semantics ad hoc.

## Session note (2026-06-28)

Progress this session:

- Closed `story-bootstrap-substrates` to `done` after validating `.work/`, `.research/`, rules, and `work-view` behavior.
- Authored, implemented, reviewed, and closed `feature-v0-walking-skeleton`; v0 is now single-operator, single-core, web+CLI, local durable event/snapshot store, Pi-adapter-first, with native mobile/HA/multi-human/arbitrary adapters deferred.
- Authored, implemented, reviewed, fixed, re-reviewed, and closed `feature-command-state-ssot`; `docs/PROTOCOL.md` now owns `SubmissionOutcome`, `CommandState`, `LocalSubmissionState`, `SessionConnectivityState`, `SessionActivityState`, failure vocabulary, transition/race semantics, and extension-pressure classification.
- Fresh-context review initially found a protocol blocker around pre-acceptance rejection vs durable `CommandState`; fixed by splitting `SubmissionOutcome` from durable command state and clarifying audit records are not command records.
- System/relay interruption occurred after the first command-state review; uncommitted fixes survived, were re-reviewed, and were committed.

Current ready queue after the command-state gate closed:

- `feature-extension-seams-non-foreclosure`
- `feature-persistence-snapshot-model`
- `feature-research-contract-tooling`
- `feature-research-web-control-security`
- `feature-session-identity-adapter-contract`
- `feature-ux-v0-acceptance`

Suggested continuation: pick `feature-extension-seams-non-foreclosure` if you want to classify extension seams before more foundation prose, or `feature-persistence-snapshot-model` if you want to continue straight down the core protocol/persistence dependency chain.

Operational note: `.pi/` remains untracked and was intentionally left untouched; it appears related to relay/mesh pairing state.
