---
id: gate-docs-architecture-split-topology
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

# Architecture claims v0.1 supports split deployment

## Drift category
foundation-doc-assertion

## Location
- Doc: `docs/ARCHITECTURE.md:150`; `docs/ARCHITECTURE.md:169`
- Contradicting source: `server/src/main.rs:92-100`

## Current doc text
> Split deployments may place the web surface, CLI, core, and adapter processes on different machines, but there is no HA core or multi-writer state in v0.1.0.
>
> **Split deployment**: the web server, CLI, core, and adapters may run on different machines. v0.1.0 may colocate them on one host for installation simplicity, but that colocation is a deployment convenience, not the architecture.

## Contradiction
The v0.1.0 core validates `PATCHBAY_BIND_ADDR` with `local_network_address`, which rejects every non-loopback address and reports that split deployment with TLS is a future milestone. Separate-machine adapters and control surfaces cannot reach this loopback-only core through the shipped bring-up path.

## Required edit
State that v0.1.0's executable deployment is loopback/colocated and retain split deployment as a future architectural seam rather than a current v0.1.0 capability.

## Completion
Corrected `docs/ARCHITECTURE.md` to make the core-dependent v0.1.0 topology
loopback and colocated, with split deployment reserved pending an explicit
transport/TLS design. The correction retains the shipped direct-TLS browser
access to a colocated web server; that access does not make the core
network-reachable.
