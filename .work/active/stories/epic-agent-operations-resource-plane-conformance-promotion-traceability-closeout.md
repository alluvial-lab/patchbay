---
id: epic-agent-operations-resource-plane-conformance-promotion-traceability-closeout
kind: story
stage: implementing
tags: [foundation, verification, protocol, security]
parent: epic-agent-operations-resource-plane-conformance
depends_on: [epic-agent-operations-resource-plane-conformance-authority-source-isolation, epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty, epic-agent-operations-resource-plane-conformance-stale-presentation-dominance]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Promote and close resource-plane conformance evidence

## Checkpoint

After every implementation runner and property oracle is green and mutation
sensitive, promote the designed vectors, regenerate the one conformance
traceability table, and roll `docs/VERIFICATION.md` forward with an exact
claim-by-claim evidence map. Run the umbrella vector executor plus focused/full
Rust and TypeScript suites. Do not call these properties model-checked,
checked-normative, or release-verified: this feature supplies promoted
executable examples and implementation property evidence; no new formal model
is part of this design.

Treat any vector/property that can survive its named claim-breaking mutation as
a blocker. Fix the production boundary or restructure the oracle; never weaken
expected outcomes, delete the mutation, or retain a metadata-only green check.

## Primary files

- `contracts/scripts/check-vectors.mjs`
- `contracts/vectors/*.json` (only the designed modified/new vectors)
- `docs/VERIFICATION.md`
- verification files from the preceding checkpoints

## Acceptance evidence

- All modified/new resource vectors are `promotion_status: promoted`, trace to a
  registered property id, have at least one successful implementation check,
  and pass a property-specific static expected-outcome checker.
- The generated table maps every vector to exact `.proto` fields and lists no
  missing/unknown runner or property registration.
- A reviewed mutation ledger in the feature implementation notes records the
  concrete claim-breaking mutant killed for each of the six coverage areas.
- `node contracts/scripts/check-vectors.mjs` executes, rather than merely
  validates metadata for, every promoted resource vector.
- Workspace Rust tests/clippy, web-cockpit tests, generated drift,
  model-metadata, vector, and presentation checks pass with no weakened or
  skipped tests.
- Foundation wording distinguishes promoted vector examples and
  implementation-checked properties from formal/model and release-verification
  tiers.

## Ordering constraints

Final checkpoint; depends on authority/source isolation, durable reconnect, and
stale-presentation evidence. Child stories remain subject to the project's deep
review lane because all carry `[verification]`.
