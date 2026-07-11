---
id: feature-v0-cli
kind: feature
stage: drafting
tags: [ux, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-protocol-seam]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: CLI

## Brief

Build the diagnostic CLI for setup, administration, debugging, and scripted access. The CLI is not a second independent product surface with divergent semantics — it speaks the same protocol semantics as the web cockpit, just through a different surface. It reuses the shared TypeScript operator domain and protocol client.

v0.1.0 CLI scope (per `docs/UX.md`): setup/configuration, adapter enrollment, session inspection, command submission for scripting, audit queries, and diagnostic commands (`audit-query`, `inspect-command`, `session-health`, `adapter-status`). The CLI is the operator's tool for the things that are awkward in a web UI — initial setup, scripted automation, and deep debugging.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: side branch off the protocol seam. Independent of the web server and cockpit; can proceed in parallel with the web chain once the seam exists.

## Foundation references

- `docs/UX.md` — CLI section, diagnostic commands, surface-neutral conformance floor
- `docs/ARCHITECTURE.md` — shared TypeScript operator domain, CLI as a control surface
- `docs/PROTOCOL.md` — OperationKind registry, authority, audit
- `docs/SPEC.md` — v0.1.0 observability scope (CLI diagnostic commands as projections of the durable event log + audit records)
