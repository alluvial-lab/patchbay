---
id: feature-v0-web-server
kind: feature
stage: drafting
tags: [security, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-protocol-seam]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: TypeScript web server

## Brief

Build the TypeScript web server that terminates HTTP/HTTPS for the browser cockpit. The web server is a control surface, not a core: it never writes the durable log or makes authority decisions. It owns operator sessions, cookies, and CSRF protection, and speaks the generated Protobuf/Connect contract to the Rust coordination core.

The web server is an authenticated endpoint/principal with respect to the core, subject to the same grant and audit rules as other control surfaces. It translates browser-facing requests into core protocol calls, and streams core events back to the browser. The browser runs the shared TypeScript operator domain (protocol client, delivery/reconnect state machines, presentation model) as a client of the web server.

v0.1.0 may run the web server as a thin HTTP→protocol translator with the operator domain executing only in the browser; promoting delivery/reconnect state machines or SSR to the server is a reserved seam.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: on the phone-usable critical path, between the protocol seam and the web cockpit. The cockpit cannot run until the web server terminates HTTP and speaks to the core.

## Foundation references

- `docs/ARCHITECTURE.md` — v0.1.0 process topology (two-process split: Rust core + TS web server), reserved seams (server-side operator-domain reuse, web↔core internal protocol)
- `docs/SECURITY.md` — operator sessions, CSRF, web server as principal
- `docs/PROTOCOL.md` — authority grants, audit
- `docs/UX.md` — shared presentation-component layer (named seam, implementation deferred)
- `contracts/ts/` — generated TS bindings (the starting contract for the web server's types)
- `contracts/proto/patchbay/*.proto` — generated contract source
- Formal model: `csrf_browser.qnt` — `CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority`
