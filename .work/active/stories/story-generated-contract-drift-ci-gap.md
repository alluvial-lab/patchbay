---
id: story-generated-contract-drift-ci-gap
kind: story
stage: done
tags: [cleanup, contracts, ci]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-26
---

# Generated-contract drift is real and CI doesn't check it

`contracts/ts` `npm run check:drift` fails on a clean tree (verified at
`be51167`, pre-observability-epic): `buf generate` output is not byte-identical
to the committed prost/protoc-gen-es artifacts — it reorders ~1362 lines of
`contracts/rust/src/gen/patchbay/patchbay.rs` and adds a trailing newline to
`contracts/ts/src/gen/patchbay/admin_pb.ts`. Root cause is likely a generator
version skew between whatever produced the committed artifacts and the pinned
toolchain, or an unpinned plugin.

CI (`.github/workflows/ci.yml`) runs `check:vectors`, `check:models`, and
`check:presentation` but NOT `check:drift`, so the rot was invisible.

Fix: regenerate with the pinned toolchain, commit the result, and add
`check:drift` to CI. Until then, every contract change has to manually
distinguish "my drift" from "pre-existing drift" — the core-diagnostics
implementation had to do exactly that.

Surfaced during `epic-observability-dogfooding` wave-1 verification
(2026-07-26).

## Implementation summary + inline review (2026-07-26)

Root cause confirmed: the committed artifacts were produced by a different
generator build than the current toolchain — the diff was purely cosmetic
(prost message reordering, rustfmt-style match-arm wrapping, a `// @generated`
header, one trailing TS newline), not semantic. Rather than chase the original
generator version, the current pinned toolchain's output became the new
baseline.

- Regenerated `contracts/rust/src/gen` + `contracts/ts/src/gen` with buf
  1.71.0 / protoc-gen-prost 0.5.0 / protoc-gen-es 2.12.1 (`137625e`).
- CI (`contracts-and-conformance` job): buf pinned to 1.71.0, Rust toolchain
  added, `protoc-gen-prost` installed pinned via
  `cargo install protoc-gen-prost --version 0.5.0 --locked` (validated
  locally — the crate's lockfile resolves), and `npm run check:drift` wired
  in. The two stale NOTE blocks documenting the gap were removed.
- `contracts/README.md` records the generator-of-record pins and the
  bump-protocol (regenerate + commit baseline in the same change).

Verification: `npm run check:drift` green against the committed baseline;
cargo build/test (32 suites) + clippy green on the regenerated bindings;
check:vectors, check:models, check:presentation green.

Bounded inline review (standalone-story lane): the regeneration diff is
cosmetic-only — every struct appears as a matched -/+ reorder pair with no
content change; CI YAML structure preserved; no test weakened. Approved.

Follow-up (not blocking): CI's `cargo install protoc-gen-prost` compiles
uncached on every run (~minutes); a Rust cache step would amortize it.
