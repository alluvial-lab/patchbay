---
id: gate-docs-security-deployment-topology
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

# Security deployment floor permits unsupported v0 topologies

## Drift category
foundation-doc-assertion

## Location
- Doc: `docs/SECURITY.md:248-250`
- Contradicting source: `server/src/main.rs:92-100`

## Current doc text
> Allowed v0.1.0 deployments:
>
> - VM or container behind local access controls, with HTTPS required for non-localhost browser sessions;
> - LAN, VPN, or reverse-proxy deployment with HTTPS and authenticated browser sessions;
> - split deployment where adapters run near runtimes and the core remains the single authority.

## Contradiction
The shipped core rejects every non-loopback `PATCHBAY_BIND_ADDR` and explicitly reports that split deployment with TLS is a future milestone. The listed non-localhost and split v0.1.0 topologies are therefore not supported by the executable core bring-up boundary.

## Required edit
Restrict the v0.1.0 allowed-deployment list to the loopback/colocated topology actually accepted by the core, and move LAN/VPN/reverse-proxy and split deployment to a reserved future deployment seam.

## Completion
Corrected `docs/SECURITY.md` to require the core-dependent processes to be
loopback/colocated and to reserve core LAN/VPN exposure, reverse-proxy TLS
termination, and split deployment. The gate evidence was incomplete: the web
server does accept a browser's direct TLS connection on a non-loopback bind, so
the corrected text retains that shipped access while making clear it does not
make the core network-reachable.
