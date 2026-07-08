---
source_handle: patchbay-architecture-v0-topology
fetched: 2026-07-07
source_path: docs/ARCHITECTURE.md
provenance: source-direct
---

# Attestation: Patchbay Architecture — v0 topology and boundary rules

## Summary

The architecture document defines a v0 two-process topology with a Rust coordination core and a TypeScript web server. The Rust core is the single authoritative process and writer for durable coordination state. The TypeScript web server terminates browser HTTP/HTTPS, owns browser-facing operator-session concerns, and speaks the generated Protobuf/Connect contract to the Rust core as a control-surface process.

## Key passages

1. Under "V0 process topology", the document states that v0 runs two logical processes: a "Rust coordination core" and a "TypeScript web server".

2. The Rust coordination core is described as "the single authoritative process" that owns "the durable event log, Operation acceptance, authority checks, snapshots, and the storage port" and "does not terminate HTTP in v0".

3. The TypeScript web server is described as terminating "HTTP/HTTPS for the browser cockpit", owning "operator sessions, cookies, and CSRF protection", and speaking "the generated Protobuf/Connect contract to the Rust core".

4. The document states: "The web server is a control surface, not a core" and says the web server "never writes the durable log or makes authority decisions".

5. The reserved seams include "Web↔core internal protocol design", specifically the "RPC surface, streaming/event channel, operator-session/CSRF evidence crossing, and web-surface authentication to the core".

6. Under "Boundary rules", the document states that the coordination core owns durable Operation state and authority checks, adapters own external-runtime details, and control surfaces never infer authoritative state from optimistic UI alone.
