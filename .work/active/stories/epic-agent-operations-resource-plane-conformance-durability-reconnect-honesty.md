---
id: epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty
kind: story
stage: implementing
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
