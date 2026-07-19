---
id: backlog-presentation-conformance-vector
kind: feature
stage: backlog
tags: [ux, verification, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-18
---

# Backlog: Machine-checkable presentation conformance vector

## Context

Surfaced during review of `feature-v0-presentation-component-layer` (review 2026-07-19, finding F2). The feature's Brief states the layer "makes the conformance floor machine-checkable (states cannot drift between surfaces)." `docs/UX.md:49` gives an either/or: the first real web cockpit must not proceed without *either* the component layer *or* an explicit conformance-test substitute.

The component layer ships the CSS class taxonomy (one class per registry member, dominance via `:has()` + wrapper modifiers, distinct delivery/liveness primitives) — which is the *substrate* a conformance check would assert against, and satisfies the UX.md either/or for v0.1.0. But no formal static conformance vector currently asserts the registry↔class↔showcase correspondence. The Brief's "machine-checkable" wording slightly over-claims what the CSS alone delivers; CSS cannot enforce that a consumer emits the right class or derives retry-safety correctly (that is a consumer/cockpit responsibility).

## Proposal (parked — not a v0.1.0 blocker)

A registry-derived, surface-neutral conformance vector/check that asserts:

- every member of `CommandState`, `SessionConnectivityState`, `SessionActivityState`, `ElicitationState` (per `docs/PROTOCOL.md`) has a corresponding CSS class binding;
- no invented/divergent state names exist in the layer;
- every locked primitive is exercised in the showcase;
- the retry-safety derivation (failure term × `idempotency_strength`) is represented for the full `docs/UX.md` matrix.

This would make the "machine-checkable" claim literal. Likely a small script in `contracts/scripts/` (alongside `check-vectors.mjs` / `check-models.mjs`) that reads the proto registries and asserts against `components.css` + `components.html`.

## Risk rationale (why parked, not blocking)

The cockpit consumes the CSS primitives directly and is not blocked by the absence of a conformance vector — UX.md:49's either/or is satisfied by the layer existing. The cost of the gap is: a future surface (CLI/Expo) or a future CSS edit could silently drift from the registry without a check catching it. Low likelihood in v0.1.0 (single consumer, fresh layer); higher value once a second surface appears. Promote when adding the second conformant surface or when hardening the v0.1.0 release assurance.

## Origin

- Review finding F2, `feature-v0-presentation-component-layer` review (2026-07-19).
- Related: `feature-ux-v0-acceptance` (done) named the conformance floor; `feature-command-state-ssot` exists to kill registry drift.
