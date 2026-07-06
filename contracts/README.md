# Patchbay protocol contracts

`contracts/` contains the v0 Protobuf IDL plus generated Rust and TypeScript boundary types. The `.proto` files are the source of truth for wire shape; generated code is committed so reviewers and downstream consumers can inspect the concrete wire types without running code generation.

## Install buf

This workspace was verified with `buf` installed from npm:

```sh
npm install -g @bufbuild/buf
export PATH="$HOME/.npm-global/bin:$PATH" # needed in this sandbox because npm global binaries live here
buf --version
```

The install produced `buf` 1.71.0 in this environment. If your npm global binary directory differs, add the directory reported by `npm bin -g` / your npm prefix to `PATH`.

`buf` uses its own Protobuf compiler for generation and linting, so no system `protoc` is required for `buf generate`. The Rust crate build uses `prost-build` with `protoc-bin-vendored`, so `cargo build` also does not require a system `protoc`.

## Regenerate contracts

From this directory:

```sh
buf generate
```

The checked-in `buf.gen.yaml` generates:

- Rust prost code into `rust/src/gen/` via `protoc-gen-prost`.
- TypeScript Protobuf-ES code into `ts/src/gen/` via `@bufbuild/protoc-gen-es`.

The TypeScript generator is installed by `npm install` in `contracts/ts/`. The Rust prost generator used by `buf generate` is `protoc-gen-prost`:

```sh
cargo install protoc-gen-prost
```

## Rust crate

Build the Rust contract crate with:

```sh
cd rust
cargo build
```

The crate is named `patchbay-contracts` and re-exports the generated `patchbay` module from `src/lib.rs`.

## TypeScript package

Install dependencies and build the TypeScript contract package with:

```sh
cd ts
npm install
npm run build
```

The package is named `@patchbay/contracts` and re-exports all generated Protobuf-ES modules from `src/index.ts`.

## Do not edit generated code

Do not edit files under `rust/src/gen/` or `ts/src/gen/` by hand. Edit `proto/patchbay/*.proto`, run `buf generate` from `contracts/`, then rebuild both packages.
