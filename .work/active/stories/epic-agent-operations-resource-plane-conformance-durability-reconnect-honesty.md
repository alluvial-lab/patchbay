---
id: epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty
kind: story
stage: review
tags: [verification, protocol]
parent: epic-agent-operations-resource-plane-conformance
depends_on: [epic-agent-operations-resource-plane-conformance-vector-execution-bridge]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Prove durable resource reconnect and completeness honesty

## Checkpoint

Extend `snapshot-reconciliation.json` with the resource view and add the
resource completeness vector. Execute authenticated report → one durable
`RESOURCE_STATE` append → hot fold → replay → `LoadSnapshot(RESOURCE)` through
the shared runners. Cover authoritative, partial, none, and live-delta semantics
without allowing a partial/none adapter to manufacture current state.

Expand the existing `core/tests/resource_reconciliation.rs` proptest from one
branch to arbitrary bounded report traces. A separate reference model consumes
raw generated report mode/tier/listed identities/cache presence/generation and
predicts tombstone/stale/unknown/current behavior; production normalization and
two durable replays must match it. Mutation spot-checks must demonstrate that
swapping authoritative/partial/none branches, treating delta omission as
snapshot omission, applying before append, allowing generation rollback, or
resurrecting a tombstone fails the oracle.

## Primary files

- `contracts/vectors/snapshot-reconciliation.json`
- `contracts/vectors/resource-snapshot-completeness-honesty.json` (new)
- `core/tests/conformance_vectors.rs`
- `server/tests/conformance_vectors.rs`
- `core/tests/resource_reconciliation.rs`
- `core/tests/resource_replay.rs`
- `server/tests/grpc_smoke.rs`

## Acceptance evidence

- Authoritative omission terminally tombstones an existing cached identity;
  partial/none omission only stales records with both cached envelopes and
  preserves no-payload unknown as unknown; delta omission is inert at every
  tier.
- A report overclaiming the manifest tier rejects before resource append or hot
  projection mutation.
- Every accepted report is durably appended before fold, assigns revisions from
  the committed LSN, and yields equivalent hot/replayed/snapshot projections.
- Wrong-domain, stale-generation, non-increasing-LSN, contradictory prior
  revision, and tombstone-resurrection traces fail closed without partial fold.
- The extended snapshot vector proves a resource view is selected/echoed and an
  older cached view cannot replace current authority.
- Each tier/source/durability mutant is killed by the reference oracle.

## Ordering constraints

Depends on the shared execution bridge. Keep the authority-domain log,
`ResourceRegistry`, and `LoadSnapshot` as the existing machinery; do not add a
resource event store or checkpoint namespace.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` at high reasoning, explicit caller selection for durability/reconnect verification.
- Review weight: `thorough`, explicit caller override; left at `review` for the project deep lane.
- Files changed: `snapshot-reconciliation.json`, new `resource-snapshot-completeness-honesty.json`, core/server conformance runners, `core/tests/resource_reconciliation.rs`, and generated conformance traceability.
- Tests added: explicit RESOURCE snapshot discriminator/materialization execution; deterministic authoritative/partial/none/delta truth-table vector; a 100-case bounded arbitrary report-trace property over cached and no-payload identities with a raw mode/tier/listing oracle; hot/replay/replay-twice convergence; failed-append-before-fold property; omission mutant oracles.
- Mutation evidence: (1) changed authoritative omission to stale; both vector and generated trace property failed. (2) changed partial/none omission to tombstone; the first vector oracle was found insufficient, strengthened to assert non-tombstoned weak-tier records, then the mutation failed. (3) treated delta omission as snapshot omission; vector failed. (4) folded a normalized resource event before an injected failed append; failed-append property failed. (5) disabled the source-generation rollback fence; the generation regression failed. (6) disabled terminal upsert resurrection rejection; the replacement regression failed. (7) forced materialized ResourceSnapshot LSN to zero; snapshot-reconciliation vector failed. All production mutations were reverted.
- Verification: both focused implementation checks reported exact ids; arbitrary trace and focused regressions passed; `cargo test --workspace`, clippy with warnings denied, contracts build/vector/drift/presentation/model checks, and 103 web-cockpit tests passed.
- Simplification: reused `ResourceRegistry`, one authority-domain log, normal replay, and `ProjectionState` materialization; no resource event store, checkpoint namespace, or production conformance abstraction was added.
- Discrepancies from design: generated traces deliberately keep authoritative omission as the optional terminal step so later generated steps do not become invalid resurrection attempts; deterministic rejection regressions cover rollback/resurrection. The vectors remain draft until final promotion.
- Adjacent issues parked: none.
