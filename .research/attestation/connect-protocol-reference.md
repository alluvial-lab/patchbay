---
source_handle: connect-protocol-reference
fetched: 2026-07-07
source_url: https://connectrpc.com/docs/protocol/
provenance: source-direct
---

# Attestation: Connect protocol reference

## Summary

The Connect protocol reference specifies the Connect protocol for RPCs over HTTP. It says the protocol does not depend on HTTP-version-specific framing details. With Protobuf schemas, it supports unary, client-streaming, server-streaming, and bidirectional-streaming RPCs; bidirectional streaming requires HTTP/2; the other RPC types also support HTTP/1.1. Unary RPCs use ordinary Protobuf or JSON content types and meaningful HTTP status codes, while streaming RPCs use Connect-specific content types and binary envelopes.

## Key passages

1. The introduction states: "This document specifies the Connect protocol for making RPCs over HTTP" and says the protocol does "not depend on framing details specific to a particular HTTP version."

2. The design goals include staying "conceptually close to gRPC’s HTTP/2 protocol, so Connect implementations can support both protocols."

3. The design goals also include depending only on widely implemented HTTP features and specifying behavior in high-level semantics so implementations can use off-the-shelf networking libraries.

4. The summary says: "When used with Protocol Buffer schemas, the Connect protocol supports unary, client streaming, server streaming, and bidirectional streaming RPCs, with either binary Protobuf or JSON payloads."

5. The summary states: "Bidirectional streaming requires HTTP/2, but the other RPC types also support HTTP/1.1."

6. The summary says the protocol does not use HTTP trailers, so it works with any networking infrastructure.

7. Unary RPCs are described as using `application/proto` and `application/json` content types; request/response paths are derived from the Protobuf schema, bodies are valid Protobuf or JSON, and responses have meaningful HTTP status codes.

8. Streaming RPCs are described as using `application/connect+proto` and `application/connect+json` content types, with each request and response message wrapped in binary framing data, and errors sent in the last portion of the body.

9. The outline states that clients send HTTP requests to servers; unary requests contain exactly one message, while streaming requests contain zero or more messages. Servers return HTTP responses to clients; streaming responses contain one or more enveloped messages.
