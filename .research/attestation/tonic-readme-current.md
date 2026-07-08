---
source_handle: tonic-readme-current
fetched: 2026-07-07
source_url: https://raw.githubusercontent.com/hyperium/tonic/master/README.md
provenance: source-direct
---

# Attestation: tonic README

## Summary

The tonic README describes tonic as a Rust gRPC over HTTP/2 implementation focused on performance, interoperability, flexibility, async/await support, and production Rust systems. It notes that the master branch is preparing breaking changes and points readers to the 0.14.x branch for released code. It says tonic is composed of a generic gRPC implementation, a high-performance HTTP/2 implementation, and code generation powered by prost. Its feature list includes bidirectional streaming, async I/O, interoperability, TLS, load balancing, metadata, authentication, and health checking.

## Key passages

1. The opening states: "A rust implementation of gRPC, a high performance, open source, general RPC framework that puts mobile and HTTP/2 first."

2. The README note says the master branch is preparing breaking changes and that for the most recently released code, readers should look to the 0.14.x branch.

3. It says tonic is "a gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility" and has first-class support for async/await.

4. The overview says tonic has three main components: the generic gRPC implementation, the high-performance HTTP/2 implementation, and code generation powered by `prost`.

5. The overview says the HTTP/2 implementation is based on `hyper`, a fast HTTP/1.1 and HTTP/2 client and server built on tokio.

6. The overview says codegen contains tools to build clients and servers from Protobuf definitions.

7. The feature list includes "Bi-directional streaming", "High performance async io", "Interoperability", "TLS backed by rustls", "Load balancing", "Custom metadata", "Authentication", and "Health Checking".

8. The README states tonic's MSRV is `1.88`.
