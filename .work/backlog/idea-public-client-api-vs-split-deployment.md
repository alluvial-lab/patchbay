---
id: idea-public-client-api-vs-split-deployment
created: 2026-07-30
updated: 2026-07-30
tags: [protocol, security, deployment, seam]
---

# Public client API promotion ≠ multi-host reachability

Surfaced during combined-surface vision review (round 5, Kimi). Independent of
the vision's fate — it's a seam-conflation the docs should not repeat.

## The finding

Two distinct seams got conflated in the combined-surface vision's "multi-host
surfaces" title:

1. **Public client API / auth topology.** Separating public client methods
   from core administration, defining browser/extension-host/CLI auth
   profiles, CORS/CSP/webview messaging, SecretStorage. This delivers
   **multi-client on one host** — multiple client processes on the
   operator's machine reaching the colocated web server.
2. **Split deployment / network-reachable core.** Separate-machine components
   and a network-reachable core — a reserved seam that "require an explicit
   transport/TLS design" (ARCHITECTURE.md:175).

The combined-surface vision's seam #1 (public client API promotion) claimed
to make Patchbay "reachable from multiple hosts." It does not. An IDE extension
on the operator's machine can already reach a loopback core through the web
server; a cockpit on a *different* machine cannot, no matter how clean the
auth topology is. The vision's worked example ("the operator fires agents
across machines and walks away") needs the network seam, not just the auth seam.

## Why it matters

This is the round-four failure mode in miniature: a mechanism named ("public
client API promotion") that does not deliver the outcome claimed for it
("reachable from multiple hosts"). Any future proposal that uses "multi-host"
language needs to name *which* seam — auth topology or split deployment — and
not lean on one for the other's outcome.

## Also noted

The public-client-API seam is also mis-scoped as "promote `ControlService`."
`ControlService` mixes submission, password verification, enrollment,
principal revocation, lockdown, diagnostics, and security snapshots
(`contracts/proto/patchbay/control.proto`). Exposing the whole internal
service is not a simple promotion — a dedicated public operator-facing boundary
(a BFF on the web server, or a narrow public subset) is a more honest design
than wholesale exposure. The browser already reaches the web server remotely
over TLS without making the core network-reachable (ARCHITECTURE.md:175); an
IDE extension could likewise talk to a public web-server API or use its
extension host as a broker.

## Source

Combined-surface vision review round 5 (Kimi K3, finding 4), against
`docs/ARCHITECTURE.md:167-175` and `contracts/proto/patchbay/control.proto`.
