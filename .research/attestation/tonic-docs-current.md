---
source_handle: tonic-docs-current
fetched: 2026-07-07
source_url: https://docs.rs/tonic/latest/tonic/
provenance: source-direct
---

# Attestation: docs.rs — tonic latest crate documentation

## Summary

The docs.rs page for the latest `tonic` crate reports version 0.14.6. It describes tonic as a Rust implementation of gRPC over HTTP/2 focused on performance, interoperability, flexibility, async/await support, and production Rust systems. The docs list a transport feature that enables client and server implementations based on hyper, tower, and tokio, and name `Streaming` as a struct for streaming requests and responses.

## Key passages

1. The page header reports `tonic-0.14.6` / `tonic 0.14.6`.

2. The crate description says tonic is "A Rust implementation of gRPC" and "a gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility."

3. The description says the library was created to have first-class async/await support and to be a core building block for production systems written in Rust.

4. The `transport` feature is documented as enabling a fully featured client and server implementation based on `hyper`, `tower`, and `tokio`, and as enabling `server` and `channel` features by default.

5. The `server` feature enables the full server portion of the transport feature; the `channel` feature enables the full channel portion.

6. The structure section says the transport module contains a fully featured HTTP/2.0 `Channel` and `Server` built on top of tokio, hyper, and tower.

7. The module/type index lists `Streaming` with the description "Streaming requests and responses."

8. The documentation says both servers and clients can configure maximum message encoding and decoding size, with a default decoding limit of 4MB.
