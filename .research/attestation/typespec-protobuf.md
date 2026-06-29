---
source_handle: typespec-protobuf
fetched: 2026-06-28
source_url: https://typespec.io/docs/emitters/protobuf/guide/
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: TypeSpec Protobuf emitter guide

## Summary

TypeSpec's Protobuf guide says `@typespec/protobuf` can generate Protocol Buffers specifications from TypeSpec sources for gRPC or other Protobuf-compatible tools. The guide also notes proto3 as the target syntax and states that TypeSpec models must follow rules and limitations, including explicit field-index decorators for Protobuf fields.

## Key passages

1. From the guide introduction:

> TypeSpec includes a built-in emitter (`@typespec/protobuf`) that can generate Protocol Buffers specifications from TypeSpec sources. The Protobuf files generated can then be used to create gRPC services or any other tools that are compatible with Protocol Buffers.

2. From the note after the introduction:

> The Protobuf emitter is designed to work with Protocol Buffers 3 (proto3) syntax. Ensure that your workflow (including `protoc` version) supports proto3 to make full use of this emitter.

3. From "Fundamental Concepts":

> To successfully convert your TypeSpec models and interfaces to Protobuf, they must comply with certain rules and limitations.

4. From "Field Indices":

> Protobuf requires manual specification of the offset for each field within a Protobuf message. In TypeSpec, these field indices are specified using the `TypeSpec.Protobuf.field` decorator. To be converted into a Protobuf message, all fields within a model must have an attached `@field` decorator.
