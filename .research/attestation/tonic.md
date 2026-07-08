---
source_handle: tonic
fetched: 2026-07-07
source_url: https://docs.rs/tonic/latest/tonic/
provenance: source-direct
---

# Attestation: tonic gRPC Rust docs

## Summary

The tonic docs describe tonic as a Rust gRPC-over-HTTP/2 implementation focused on performance, interoperability, flexibility, async/await support, production systems, transport features built on hyper/tower/tokio, code generation via tonic-build, a batteries-included server builder, and explicit streaming request/response types. The fetched docs.rs latest page identified the documented crate as `tonic-0.14.6`.

## Key passages

1. From the crate description:

> A Rust implementation of gRPC, a high performance, open source, general RPC framework that puts mobile and HTTP/2 first.

2. From the crate description:

> tonic is a gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility.

3. From the crate description:

> This library was created to have first class support of async/await and to act as a core building block for production systems written in Rust.

4. From "Getting Started":

> Follow the instructions in the tonic-build crate documentation.

5. From "Feature Flags":

> transport: Enables the fully featured, batteries included client and server implementation based on hyper, tower and tokio.

6. From "Feature Flags":

> server: Enables just the full featured server portion of the transport feature.

7. From "Generic implementation":

> The main goal of tonic is to provide a generic gRPC implementation over HTTP/2 framing.

8. From `tonic::transport::Server`:

> A default batteries included transport server.

9. From `tonic::transport::Server`:

> This builder exposes easy configuration parameters for providing a fully featured http2 based gRPC server.

10. From `tonic::codec::Streaming`:

> Streaming requests and responses. This will wrap some inner Body and Decoder and provide an interface to fetch the message stream and trailing metadata.

11. From `tonic::codec::Streaming::message`:

> Fetch the next message from this stream.

12. From `tonic::Request` trait implementations:

> IntoStreamingRequest for Request<T> where T: Stream + Send + 'static.

13. From the fetched docs.rs page metadata:

> tonic-0.14.6
