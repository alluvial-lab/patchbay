---
id: feature-pi-parity-checklist
kind: feature
stage: drafting
tags: [prose, adapter, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-session-identity-adapter-contract]
---

# Feature: Define Pi migration and parity checklist

Pi is the first adapter because it lets the operator migrate from current Remote Pi-style workflows, but parity is not yet itemized. Define the migration floor without making Pi the core ontology.

## Scope

- Current Remote Pi workflow inventory.
- Required Pi adapter capabilities for v0.
- Session discovery, send prompt, stream/read replies, reconnect recovery, working/idle/stale/offline status.
- Commands such as cancel, compact, new/resume only as adapter-declared capabilities.
- Unsupported or deferred Remote Pi features.
- Mapping from Pi session metadata to Patchbay session identity.

## Acceptance criteria

- Add a Pi parity checklist to `docs/SPEC.md`, `docs/ARCHITECTURE.md`, or a dedicated adapter doc.
- The checklist is sufficient to decide when the operator can switch workflows.
- Pi-specific operations are represented as adapter capabilities, not core protocol states.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
