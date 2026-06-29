---
source_handle: prost
fetched: 2026-06-28
source_url: https://docs.rs/prost/latest/prost/
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: prost Rust protobuf docs

## Summary

The `prost` crate documents itself as a Protocol Buffers implementation for Rust. It generates Rust code from proto2 and proto3 files, aims for simple generated code, uses Rust derive attributes, retains comments, respects Protobuf packages for Rust module organization, and recommends `prost-build` for Cargo build-time `.proto` compilation.

## Key passages

1. From the crate description:

> `prost` is a Protocol Buffers implementation for the Rust Language. `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.

2. From the comparison list:

> Generates simple, idiomatic, and readable Rust types by taking advantage of Rust `derive` attributes.

3. From "Using prost in a Cargo Project":

> The recommended way to add `.proto` compilation to a Cargo project is to use the `prost-build` library.

4. From "Generated Code":

> `prost` generates Rust code from source `.proto` files using the `proto2` or `proto3` syntax. `prost`’s goal is to make the generated code as simple as possible.
