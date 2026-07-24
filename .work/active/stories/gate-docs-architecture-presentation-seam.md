---
id: gate-docs-architecture-presentation-seam
kind: story
stage: implementing
tags: [documentation]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: docs
created: 2026-07-24
updated: 2026-07-24
---

# Architecture defers an implemented presentation layer

## Drift category
foundation-doc-assertion

## Location
- Doc: `docs/ARCHITECTURE.md:165`
- Contradicting source: `docs/UX.md:41-47`; `contracts/scripts/check-presentation.mjs`

## Current doc text
> The presentation model is refined in `docs/UX.md` as the **shared presentation-component layer** — a named architectural seam that binds canonical protocol states to skin-able presentable primitives, making the surface-neutral UX conformance floor enforceable; its implementation is deferred (see `docs/UX.md`).

## Contradiction
`docs/UX.md` says this layer is implemented as the registry-derived static check and skin-able CSS/showcase artifacts, with `node contracts/scripts/check-presentation.mjs` as its executable check. The referenced implementation therefore contradicts the assertion that it remains deferred.

## Required edit
Replace the deferred-implementation assertion with the active v0.1.0 implementation boundary, retaining only genuinely reserved runtime consumer guarantees as future work.
