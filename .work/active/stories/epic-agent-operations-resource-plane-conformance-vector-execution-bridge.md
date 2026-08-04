---
id: epic-agent-operations-resource-plane-conformance-vector-execution-bridge
kind: story
stage: implementing
tags: [verification, protocol]
parent: epic-agent-operations-resource-plane-conformance
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Make the shared conformance corpus executable

## Checkpoint

Extend the existing `contracts/vectors/` envelope and
`contracts/scripts/check-vectors.mjs` rather than creating a resource-only
harness. Add `implementation_checks: [{ runner, case }]`, require every promoted
vector to register at least one implementation check, dispatch the existing
checker to generic Rust core/server and web-cockpit runner entry points, and
report the exact vector/check ids executed. Existing draft metadata-only vectors
remain valid until they are promoted.

Register the resource-plane property ids in `docs/VERIFICATION.md` and the
existing checker registry. This checkpoint creates execution plumbing only; it
does not make a green metadata check count as implementation evidence.

## Primary files

- `contracts/vectors/README.md`
- `contracts/scripts/check-vectors.mjs`
- `core/tests/conformance_vectors.rs` (new, generic runner)
- `server/tests/conformance_vectors.rs` (new, generic runner)
- `web-cockpit/tests/conformance-vectors.test.ts` (new, generic runner)
- `core/Cargo.toml`, `server/Cargo.toml`
- `docs/VERIFICATION.md`

## Acceptance evidence

- Promoted vectors without an implementation check, unknown runner ids, duplicate
  runner/case registrations, and unhandled case ids fail closed.
- Each runner reads required input and expected fields from the JSON vector; a
  case cannot pass by replaying a hard-coded fixture unrelated to its vector.
- `check-vectors.mjs` runs registered implementation checks and reports their
  ids; metadata-only draft vectors retain current behavior.
- Static invariant expectation checkers remain distinct from implementation
  execution and reject promoted expected outcomes that contradict their named
  property.
- The existing generated traceability table remains the single vector registry.

## Ordering constraints

Root checkpoint. The authority, reconnect, and stale-presentation checkpoints
must register through this bridge and must not add another vector directory,
manifest, or package-specific source of truth.
