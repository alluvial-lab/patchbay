---
id: feature-session-model-field-proto-contract
kind: story
stage: implementing
parent: feature-session-model-field
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Story: Add the durable session-model contract

Add the opaque, adapter-reported `model` string to session registration,
generation-bump, snapshot, and adapter report messages. Add the
`SessionModelChanged` durable session-state mutation and regenerate both
contract targets. Roll `docs/PROTOCOL.md` forward with the committed opaque
current-model contract and richer-metadata reserved seam. Add the draft
identity-preservation conformance vector and regenerate its traceability table.

## Acceptance evidence

- `buf generate`, followed by restoring `contracts/rust/src/gen` and `cargo build -p patchbay-contracts`, produces the committed generated bindings; `contracts/ts` builds.
- The generated `SessionStateEvent` oneof exposes `model_changed`; registration, generation replacement, snapshot, and adapter-report types expose `model`.
- The draft vector constrains the mutation and its unchanged identity tuple under `LabelsCannotOverrideIdentity`; `npm run check:vectors` passes.

## Ordering

This is the contract checkpoint. The core projection and all consumers depend
on its field names and generated types.
