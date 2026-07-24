# Generated Protobuf Contracts

Treat `.proto` as the wire-contract source, generate both Rust and TypeScript boundary types from it, and reject generated-output drift instead of maintaining handwritten cross-language DTO copies.

## Rationale

Patchbay crosses Rust core/server and TypeScript control-surface/adapter boundaries. A single wire schema prevents language-specific copies of message fields and enum encodings from diverging. Generated output is committed for review, but is an artifact rather than an editing surface.

## Examples

### One Buf generation configuration targets both languages

**File**: `contracts/buf.gen.yaml:4`

```yaml
plugins:
  - local: protoc-gen-prost
    out: rust/src/gen
  - local: ./ts/node_modules/.bin/protoc-gen-es
    out: ts/src/gen
    opt:
      - target=ts
      - import_extension=js
```

A single generation invocation writes both language artifacts from `contracts/proto/`.

### Rust build regenerates from the same proto inputs

**File**: `contracts/rust/build.rs:10`

```rust
let out_dir = PathBuf::from("src/gen/patchbay");
let mut config = prost_build::Config::new();
config
    .out_dir(out_dir)
    .compile_protos(&protos, &["../proto"])?;
```

The Rust crate builds its committed generated module from the canonical schema inputs.

### TypeScript exposes generation and drift checks as package commands

**File**: `contracts/ts/package.json:18`

```json
"gen": "cd .. && buf generate",
"check:drift": "node ../scripts/check-generated-drift.mjs"
```

The TypeScript package provides the same regeneration path and a verification path that fails when committed output no longer matches it.

## When to Use

- A contract is consumed across Rust and TypeScript or over an RPC boundary.
- Wire field identity, enum values, and encoding must remain compatible.
- A contract change needs reproducible artifacts and reviewable generated output.

## When NOT to Use

- For an internal, single-language domain type that never crosses a boundary.
- To make `.proto` own product naming or formal invariants; it owns wire shape, not every kind of authority.
- By hand-editing anything below `contracts/rust/src/gen/` or `contracts/ts/src/gen/`.

## Common Violations

- Adding a parallel handwritten TypeScript or Rust DTO for a protobuf message.
- Editing generated files instead of updating `.proto` and running `buf generate`.
- Updating generated output in only one language.
- Skipping the drift check after a schema or generator change.
