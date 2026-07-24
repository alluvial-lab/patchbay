---
id: story-protocol-idl-generation-wiring
kind: story
stage: done
tags: [protocol, foundation]
parent: feature-protocol-idl-and-conformance
depends_on: [story-protocol-idl-proto-package]
created: 2026-07-06
updated: 2026-07-06
gate_origin: null
release_binding: v0.1.0
---

# Story: Wire up buf generate for Rust + TypeScript

Implements Unit 2 of `feature-protocol-idl-and-conformance`.

## Scope

Wire up `buf generate` to produce Rust (prost) and TypeScript (Protobuf-ES) code. Install `buf` (and `protoc` if required) — document the install in `contracts/README.md`. Run `buf generate` and commit the generated code to `contracts/rust/src/gen/` and `contracts/ts/src/gen/`. Create the Rust crate skeleton (`Cargo.toml` with prost/prost-build deps, `build.rs`, `lib.rs` re-exporting generated modules) and the TS package skeleton (`package.json` with `@bufbuild/protobuf` deps, `tsconfig.json`, `src/index.ts`). Verify `cargo build` and `npm run build` both succeed.

See the feature body's Unit 2 for the file list and acceptance criteria.

## Acceptance criteria

- [ ] `buf generate` runs and produces Rust + TS code.
- [ ] `cargo build` succeeds in `contracts/rust/`.
- [ ] `npm run build` succeeds in `contracts/ts/`.
- [ ] Generated code is committed (not gitignored).
- [ ] `contracts/README.md` documents how to install buf + regenerate.

## Notes

If `buf`/`protoc` cannot be installed in the sandbox, fall back to `protoc` + prost-build directly, or to documenting the generation setup + committing hand-verified generated code, and file a follow-on story for live generation wiring. Don't block the whole feature on tooling install — surface it to the operator.

## Review (2026-07-06)

**Verdict**: Approve (fast-lane via feature review)

**Notes**: Reviewed as part of the feature-protocol-idl-and-conformance deep-lane review (gpt-5.5 fresh context). Initial review returned Request changes (3 important findings: failure-vector operation_state contradiction, reply-correlation mis-typing, missing drift check); all fixed in commit 9a2854f; targeted re-review returned READY. Builds pass (cargo build, npm run build); check-vectors.mjs passes (12 vectors); check:drift detects generated-code modifications. Story advanced implementing → review; rolled up to feature.
