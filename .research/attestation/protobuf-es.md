---
source_handle: protobuf-es
fetched: 2026-06-28
source_url: https://protobufes.com/reference/generated-code/
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: Protobuf-ES generated code docs

## Summary

Protobuf-ES documents generation of plain TypeScript artifacts from `.proto` files through `buf generate` or `protoc` with `protoc-gen-es`. The generated TypeScript includes message types, schema exports, typed fields, discriminated oneofs, arrays, objects, and runtime helpers for binary and JSON conversion.

## Key passages

1. From the generated-code page introduction:

> Generated Protobuf-ES files are intentionally plain TypeScript: message types, schema exports, typed fields, discriminated oneofs, arrays, and objects. Running `buf generate` or `protoc` with `protoc-gen-es` produces one `_pb` file per `.proto` file. Treat generated files as build artifacts: regenerate them instead of editing them by hand.

2. From "Messages":

> Each Protobuf message becomes a TypeScript type plus a schema export.

3. From the runtime-helper example text:

> Pass the schema to runtime helpers such as `create()`, `fromBinary()`, `toBinary()`, `fromJson()`, and `toJson()`.
