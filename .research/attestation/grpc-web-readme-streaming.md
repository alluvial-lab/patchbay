---
source_handle: grpc-web-readme-streaming
fetched: 2026-07-07
source_url: https://raw.githubusercontent.com/grpc/grpc-web/master/README.md
provenance: source-direct
---

# Attestation: grpc-web README — streaming support

## Summary

The grpc-web README describes grpc-web as a JavaScript implementation of gRPC for browser clients. It states that grpc-web clients connect to gRPC services via a special proxy by default. Its streaming-support section says grpc-web currently supports unary RPCs and server-side streaming RPCs, with server-side streaming only in `grpcwebtext` mode. It explicitly says client-side and bidirectional streaming are not currently supported.

## Key passages

1. The README opening describes grpc-web as "A JavaScript implementation of gRPC for browser clients."

2. The opening says grpc-web clients connect to gRPC services via a special proxy, with Envoy as the default.

3. Under "Streaming Support", the README states: "gRPC-web currently supports 2 RPC modes".

4. The two supported modes listed are Unary RPCs and Server-side Streaming RPCs.

5. The server-side streaming bullet notes: "Only when `grpcwebtext` mode is used."

6. The README states: "Client-side and Bi-directional streaming is not currently supported".

7. In the wire-format section, `mode=grpcwebtext` is described as base64-encoded and supporting both unary and server streaming calls.
