---
id: feature-v0-protocol-seam
kind: feature
stage: drafting
tags: [protocol, adapter, security]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-core]
created: 2026-06-28
updated: 2026-07-11
gate_origin: null
release_binding: null
research_origin: null
---

# Feature: Web↔core internal protocol seam

With the v0 process topology settled (a TypeScript web server terminates HTTP for the browser cockpit and speaks the generated Protobuf/Connect contract to the Rust coordination core), the internal seam between the web server and the core needs genuine design work. This is a design-bearing feature, not prose — it pins RPC shapes, a streaming channel, an internal auth boundary, and failure modes.

## Scope

- The RPC surface the web server calls on the core: command submission, snapshot/cursor reconciliation, session list, grant/revocation operators, audit queries, adapter attach/detach.
- How operator-session and CSRF evidence crosses the seam: does the web server forward a verified operator-session id, or present its own service principal + a delegated operator claim? What does the core trust?
- Streaming/event channel from core to web server (and on to the browser): Connect streaming, gRPC bidirectional, or SSE-over-Connect. Reconnect and cursor resumption across the seam.
- Web server as a principal: its own grant to the core, its own endpoint/device record, audit of its calls.
- Failure modes across the seam: web server crash, core unreachable, partial submission (`SubmissionOutcome = unknown`/`failed`), backpressure on event streams.
- How the browser's operator domain composes with the web server's translations (and what stays in the browser vs. what the web server owns).
- Relationship to the shared Protobuf+Buf contract: is the internal surface the *same* contract as the browser-facing one, or a superset restricted to control-surface principals?

## Status

Promoted from backlog on 2026-07-11 into `epic-v0-1-0-implementation`. The foundation-hardening work it was waiting on (security threat model, persistence/snapshot, session-identity/adapter-contract) is now `done`, so the seam can be designed against settled foundations. Depends on `feature-v0-core` because the seam is the first consumer of the core's RPC surface.

## Expected output

A designed feature at `stage: implementing` with the seam specified, ready for Rust core + TS web server implementation. Likely spawns child stories for the core-side RPC handler and the web-server-side client/translator.

## Related

- `docs/ARCHITECTURE.md` "V0 process topology" — the committed two-process topology this seam realizes.
- `feature-security-threat-model` — grant shape, operator-session, revocation, audit.
- `feature-persistence-snapshot-model` — cursor/LSN reconciliation the web server must carry.
- `feature-session-identity-adapter-contract` — session identity/generation the seam must preserve.
- `feature-research-contract-tooling` — Protobuf+Buf as the contract source.
