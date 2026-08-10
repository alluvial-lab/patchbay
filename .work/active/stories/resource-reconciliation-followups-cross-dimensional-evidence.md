---
id: resource-reconciliation-followups-cross-dimensional-evidence
kind: story
stage: done
tags: [adapter, protocol, testing]
parent: resource-reconciliation-followups
depends_on: [resource-reconciliation-followups-applied-prefix-semantics]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Generate cross-dimensional resource reconciliation evidence

## Checkpoint
Extend the existing bounded resource reconciliation property surface rather
than creating a second generic sequence test. Generated 1–20-step traces must
combine adapter-generation transitions, same-event distinct replacements,
post-terminal mutation attempts, and obsolete catch-up redelivery. After every
accepted durable prefix, compare the hot projection, two fresh replays, and a
second application of the same prefix. For every rejected candidate, prove both
the durable prefix and the complete projection are byte-for-byte/structurally
unchanged.

Add one exact `resource-replay-prefix-idempotent` executable example under the
existing stated-normative `IdempotentLogReplay` property. The vector is promoted
only after its product-seam runner and static expectation agree through review;
that yields promoted-vector + implementation-checked evidence, not a promoted
model or checked-normative claim. The independent oracle must decide outcomes
from raw trace facts and must not call production normalization, generation,
tombstone, or prefix predicates.

## Primary files
- `core/tests/resource_reconciliation.rs`
- `core/tests/resource_state.rs`
- `core/tests/resource_replay.rs`
- `core/tests/conformance_vectors.rs`
- `contracts/vectors/resource-replay-prefix-idempotent.json` (new)
- `contracts/scripts/check-vectors.mjs`
- `docs/VERIFICATION.md`

## Acceptance evidence
- The existing 100-case, 1–20-step property now varies generation and includes
  accepted replacement and rejected terminal/lower-generation actions; it does
  not duplicate the existing mode/tier omission truth-table test.
- Every accepted step establishes hot = replay = second fresh replay, and
  replaying the already-covered prefix into a clone is idempotent.
- Every negative step preserves the exact durable event sequence, applied
  cursor, resources, views, revisions, freshness, and tombstones.
- The deterministic `IdempotentLogReplay` vector runner distinguishes a
  prefix-covered lower-generation no-op from lower-generation new-event
  corruption and checks atomic replacement plus terminal non-resurrection.
- Mutation-sensitive assertions fail when prefix coverage is moved after the
  generation guard, per-record skipping is restored, replacement ceases to be
  atomic, terminal records can resurrect, or rejected candidates append/advance.
- `docs/VERIFICATION.md` records the stronger implementation evidence without
  promoting a formal property or overstating the vector's authority.

## Ordering constraint
Begins only after
`resource-reconciliation-followups-applied-prefix-semantics`; it consumes that
checkpoint's fixed prefix/no-op/corruption contract.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-sol`; caller-selected because generated replay integrity, terminal-state rejection, and full-prefix atomicity are load-bearing.
- Review weight: `thorough` from the explicit caller selection; feature review is intentionally deferred to a fresh reviewer at `stage: review`.
- Files changed: `core/tests/resource_reconciliation.rs`, `core/tests/conformance_vectors.rs`, `contracts/vectors/resource-replay-prefix-idempotent.json`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`.
- Tests added/removed: replaced the prior generic bounded report sampler with one 100-case, 1–20-action cross-dimensional trace; added the exact Rust product runner for the promoted replay-prefix vector; retained the independent completeness truth table and focused resource tests.
- Simplification: reused the existing reconciliation property and `IdempotentLogReplay` registry instead of adding a second random sampler, property id, model, or replay mechanism.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification evidence: `cargo test -p patchbay-core --test resource_state --test resource_replay --test resource_ingest --test resource_reconciliation`; `cargo test -p patchbay-core --test conformance_vectors -- --nocapture`; `cargo clippy -p patchbay-core --test resource_reconciliation --test conformance_vectors -- -D warnings`; `node contracts/scripts/check-vectors.mjs`; `node contracts/scripts/check-models.mjs`; `node contracts/scripts/check-generated-drift.mjs`.
