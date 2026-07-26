---
id: idea-generated-contract-drift-ci-gap
tags: [cleanup, contracts, ci]
created: 2026-07-25
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
