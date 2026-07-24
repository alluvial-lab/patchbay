---
id: gate-docs-architecture-web-core-seam
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

# Architecture defers the shipped web-to-core protocol seam

## Drift category
foundation-doc-assertion

## Location
- Doc: `docs/ARCHITECTURE.md:168`
- Contradicting source: `web-server/src/main.ts:226-241`; `web-server/src/routes/rpc.ts:112-123`

## Current doc text
> **Web↔core internal protocol design**: the specific RPC surface, streaming/event channel, operator-session/CSRF evidence crossing, and web-surface authentication to the core are designed in a follow-on feature (see `feature-web-core-protocol-seam`).

## Contradiction
The shipped web server verifies credentials with the core through `verifyOperatorPassword`, receives core-issued principal/session evidence, and its RPC bridge requires enrolled control-surface principal evidence. These are the concrete web-to-core RPC and authentication boundary that the architecture says remains a follow-on.

## Required edit
Replace the deferred-seam assertion with the implemented v0.1.0 web-to-core boundary and name only remaining extension work as reserved.
