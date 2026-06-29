---
source_handle: typespec
fetched: 2026-06-28
source_url: https://typespec.io/docs/extending-typespec/emitters-basics/
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: TypeSpec emitter docs

## Summary

TypeSpec documents emitters as libraries that turn TypeSpec programs into generated artifacts. Its standard library includes emitters for OpenAPI 3.0, JSON Schema, and Protobuf. The docs frame TypeSpec as a way to use one source of truth for data shapes while letting emitters select the parts of a TypeSpec program that fit each output format.

## Key passages

1. From the emitters page introduction:

> TypeSpec emitters are libraries that utilize various TypeSpec compiler APIs to reflect on the TypeSpec compilation process and generate artifacts. The TypeSpec standard library includes emitters for OpenAPI version 3.0, JSON Schema, and Protocol Buffers (Protobuf).

2. From the same introduction:

> One of the main advantages of TypeSpec is its ease of use as a single source of truth for all data shapes, and the simplicity of creating an emitter contributes significantly to this.

3. From "Emitter design":

> TypeSpec is designed to support many protocols and many output formats. It is important that an emitter is designed to select only the part of the TypeSpec spec that makes sense for them.
