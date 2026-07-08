---
source_handle: connect-introduction-multiprotocol
fetched: 2026-07-07
source_url: https://connectrpc.com/docs/introduction/
provenance: source-direct
---

# Attestation: Connect introduction — multi-protocol support and supported implementations

## Summary

The Connect introduction describes Connect as a family of libraries for browser and gRPC-compatible HTTP APIs, generated from Protocol Buffer schemas. It states that Connect servers and clients support gRPC, gRPC-Web, and the Connect protocol; that Connect clients can call any gRPC server; and that server and client APIs for errors, headers, trailers, and streaming are protocol-agnostic. Its implementation list names Go, TypeScript/JavaScript, Swift, Kotlin, and Python, but not Rust.

## Key passages

1. The introduction says: "Connect is a family of libraries for building browser and gRPC-compatible HTTP APIs" where users write a Protocol Buffer schema, implement application logic, and Connect generates code for marshaling, routing, compression, content type negotiation, and idiomatic type-safe clients.

2. Under "Seamless multi-protocol support", it states: "Connect servers and clients support three protocols: gRPC, gRPC-Web, and Connect’s own protocol."

3. The same section states: "Any gRPC client, in any language, can call a Connect server, and Connect clients can call any gRPC server."

4. It says Connect supports its own HTTP-based protocol over HTTP/1.1, HTTP/2, and HTTP/3, including streaming, with JSON- and binary-encoded Protobuf supported by default.

5. It states: "By default, Connect servers support ingress from all three protocols. Clients default to using the Connect protocol, but can switch to gRPC or gRPC-Web with a configuration toggle — no further code changes required. The APIs for errors, headers, trailers, and streaming are all protocol-agnostic."

6. The listed implementation sections are Go, TypeScript and JavaScript, Swift and Kotlin, and Python. The text describes Go and TypeScript/JavaScript as stable, Swift as stable, Kotlin and Python as beta, and then says the project would eventually like to bring Connect to more languages and frameworks.
