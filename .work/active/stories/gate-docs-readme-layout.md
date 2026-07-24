---
id: gate-docs-readme-layout
kind: story
stage: done
tags: [documentation]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: docs
created: 2026-07-24
updated: 2026-07-24
---

# README repository layout still calls shipped components future

## Drift category
readme-staleness

## Location
- Doc: `README.md:131-139`
- Contradicting source: repository root directories `core/`, `server/`, `web-cockpit/`, `pi-adapter/`, `cli/`, `contracts/`, and `specs/`

## Current doc text
> Planned future areas include:
>
> ```text
> specs/       TLA+/Quint and Alloy models
> contracts/   protocol IDL / schemas and generated conformance vectors
> crates/      Rust coordination core and daemon
> packages/    shared TypeScript client and operator domain
> apps/        responsive web cockpit, later Expo mobile app
> adapters/    Pi adapter first, additional adapters later
> ```

## Contradiction
The v0.1.0 repository ships `specs/`, `contracts/`, the Rust core/server in `core/` and `server/`, and TypeScript cockpit, Pi adapter, and CLI packages in `web-cockpit/`, `pi-adapter/`, and `cli/`. Conversely, the listed `crates/`, `packages/`, `apps/`, and `adapters/` paths do not exist. The current-status section already calls the walking skeleton implemented, so this is stale repository guidance rather than future intent.

## Required edit
Replace the planned layout block with the actual v0.1.0 repository layout and reserve only genuinely future directories/capabilities without presenting them as current paths.

## Completion
Corrected `README.md` to list the shipped `specs/`, `contracts/`, `core/`,
`server/`, `web-server/`, `web-cockpit/`, `pi-adapter/`, and `cli/` areas.
Additional adapters and native control surfaces are now described as reserved
seams rather than nonexistent current paths.
