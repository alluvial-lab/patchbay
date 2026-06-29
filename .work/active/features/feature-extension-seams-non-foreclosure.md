---
id: feature-extension-seams-non-foreclosure
kind: feature
stage: drafting
tags: [prose, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Feature: Define extension seams and non-foreclosure rules

Patchbay should start with a narrow v0 without accidentally closing off future inclusions the operator has not thought of yet. Define a durable extensibility discipline for foundation docs and protocol design.

## Scope

- Distinguish committed v0 behavior, reserved extension seams, and explicitly rejected directions.
- Identify core seams that must stay extensible:
  - principals and authority domains;
  - adapters and adapter capabilities;
  - human control surfaces;
  - transports and deployment topologies;
  - storage/persistence backends;
  - protocol contract versions;
  - formal-model/checker backends;
  - notification providers;
  - third-party tool integrations;
  - offline/queued operator intent;
  - encryption and key-management upgrades;
  - federation / relay / multi-core topology;
  - multi-human coordination and approval workflows.
- Add an extension pressure-test checklist to foundation docs or agent guidance.
- Require capability registries/manifests where future variants are likely.
- Ensure v0 assumptions are labeled v0-only rather than silently becoming permanent architecture.

## Acceptance criteria

- Foundation docs describe the non-foreclosure discipline.
- `AGENTS.md` or a foundation doc includes an extension pressure-test checklist for future design work.
- Relevant hardening items know to classify decisions as v0 fixed, reserved seam, or explicitly rejected.
- The parked `idea-multi-human-coordination` is treated as one pressure-test input, not as a v0 requirement.
- The parked `idea-desktop-app-surface` is treated as one pressure-test input: v0 ships web cockpit + CLI, and a native desktop app is a reserved future control surface. Ensure capability/registry design does not assume web+CLI only.
