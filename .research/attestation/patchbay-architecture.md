---
source_handle: patchbay-architecture
fetched: 2026-07-07
source_path: docs/ARCHITECTURE.md
provenance: source-direct
---

# Attestation: Patchbay architecture foundation

## Summary

The architecture document defines Patchbay's v0 as a two-logical-process topology: a Rust coordination core that is the single authoritative durable writer and a TypeScript web server that terminates browser HTTP, owns operator web security/session concerns, and speaks generated Protobuf/Connect contracts to the core. It also defines persistence as a single-writer local-first log plus snapshots behind a storage port.

## Key passages

1. V0 process topology: the Rust coordination core owns the durable event log, Operation acceptance, authority checks, snapshots, and storage port, and does not terminate HTTP in v0.

2. V0 process topology: the TypeScript web server terminates HTTP/HTTPS for the browser cockpit, owns operator sessions, cookies, and CSRF protection, and speaks the generated Protobuf/Connect contract to the Rust core.

3. The web server is a control surface, not a core; it is an authenticated endpoint/principal with respect to the core and never writes the durable log or makes authority decisions.

4. The browser runs the shared TypeScript operator domain as a client of the web server; server-side operator-domain reuse is reserved.

5. The web↔core internal protocol design is a reserved seam for a follow-on feature, including RPC surface, streaming/event channel, operator-session/CSRF evidence crossing, and web-surface authentication to the core.

6. V0 persistence is single-writer, local-first, and port-isolated: one authoritative core process appends to the durable event log through a storage port, snapshots are derived checkpoints, and crash recovery replays the log or snapshot plus tail.
