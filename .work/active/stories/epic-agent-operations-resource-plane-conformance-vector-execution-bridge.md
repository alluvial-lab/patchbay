---
id: epic-agent-operations-resource-plane-conformance-vector-execution-bridge
kind: story
stage: done
tags: [verification, protocol]
parent: epic-agent-operations-resource-plane-conformance
depends_on: []
release_binding: v0.2.0
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

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` at high reasoning, explicitly selected by the autopilot caller for the high-stakes executable-evidence surface.
- Review weight: `thorough`, explicit caller override; this verification story remains at `review` for the project deep lane.
- Files changed: `contracts/scripts/check-vectors.mjs`, `contracts/vectors/README.md`, `core/tests/conformance_vectors.rs`, `server/tests/conformance_vectors.rs`, `web-cockpit/tests/conformance-vectors.test.ts`, Rust dev-dependency manifests/lockfile, and `docs/VERIFICATION.md` registry/traceability prose.
- Tests added: generic core/server/web runner entry points that deserialize the real corpus, validate requested registrations, fail unknown cases, and emit exact machine-readable execution ids; the umbrella checker groups each runner once and compares exact requested/executed sets before regenerating traceability.
- Mutation evidence: temporarily promoted `command-acceptance.json` without `implementation_checks`; `check-vectors.mjs` exited 1 and left `docs/VERIFICATION.md` byte-identical. Requested an unregistered `command-acceptance:unregistered` core case; the runner test exited 101 before reporting an executed id. Both mutations were reverted.
- Verification: `cargo test --workspace` passed; `cargo clippy --all-targets -- -D warnings` passed; contracts build/vector/drift/presentation/model checks passed after the expected one-time generated model table refresh; web cockpit passed 103 tests.
- Simplification: extended the one existing checker/corpus and used test-only package runners; no resource-only manifest, runtime framework, or second traceability registry was introduced.
- Discrepancies from design: runner case dispatch is intentionally empty at this root checkpoint and fails closed; concrete cases land in the dependent evidence stories. The exact accounting protocol and field deserialization are already executable.
- Adjacent issues parked: none.

## Deep-lane review (2026-08-04)

Converged at pass 6 (clean pass — no receiver-confirmed material current-cycle blocker). Deep-lane cross-model review ran 6 fresh-context passes with adversarial mutation testing of every promoted vector/property/traceability/assurance claim; all material blockers were fixed and each drift class is now data-driven-guarded. See the parent feature body `## Deep-lane review (2026-08-04)` for the full convergence record. Advanced to `done`.
