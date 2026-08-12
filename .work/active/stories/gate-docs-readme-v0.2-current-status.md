---
id: gate-docs-readme-v0.2-current-status
kind: story
stage: drafting
tags: [documentation]
parent: null
depends_on: []
release_binding: null
gate_origin: docs
created: 2026-08-11
updated: 2026-08-11
---

# README current-status and repository map stop at v0.1.0

## Drift category
readme-staleness

## Location
- Doc: `README.md:9`
- Contradicting source: `token-commune-adapter/package.json:1`, `docs/VISION.md:52`

## Current doc text
> Patchbay now contains the implemented `v0.1.0` walking skeleton … a Pi adapter …

The product-direction diagram and repository layout likewise omit the now-implemented token-commune adapter and operational-resource plane.

## Contradiction

The current repository and foundation docs include a materially distinct token-commune resource adapter plus the committed operational-resource plane shipped in v0.2.0. The README presents the v0.1.0 inventory as current status rather than as a bounded historical milestone.

## Required edit

Roll the current-status paragraph, product-direction diagram, and repository layout forward to v0.2.0 truth while retaining the explicitly historical v0.1.0 milestone section where useful.

## Release disposition

Parked unbound under the operator's low-risk gate policy; it does not block v0.2.0 shipment.
