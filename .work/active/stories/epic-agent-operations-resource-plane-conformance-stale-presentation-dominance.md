---
id: epic-agent-operations-resource-plane-conformance-stale-presentation-dominance
kind: story
stage: done
tags: [verification, protocol]
parent: epic-agent-operations-resource-plane-conformance
depends_on: [epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Prove stale resource presentation dominance

## Checkpoint

Add the promoted stale-never-live vector with both server/core and web-cockpit
implementation checks. The server half begins with current cached resource
state, applies adapter disconnect/reconnect degradation, and materializes a
stale/unknown resource snapshot. The TypeScript half loads the vector's
proto-shaped view and proves that reconciliation state, tombstones, and
freshness dominate adapter-owned domain health in both model predicates and
rendered output.

Extend the existing fast-check style in `web-cockpit/tests/model.test.ts` and
`resource-view.test.ts`. Generate only internally valid `ResourceView` values,
then independently compute whether current presentation is permitted. Explicit
mutants that ignore `reconciled`, `tombstoned`, or `freshness`, or that render
adapter-domain `health = serving` as current, must fail.

## Primary files

- `contracts/vectors/resource-stale-never-live.json` (new)
- `server/tests/conformance_vectors.rs`
- `web-cockpit/tests/conformance-vectors.test.ts`
- `web-cockpit/tests/model.test.ts`
- `web-cockpit/tests/resource-view.test.ts`
- `web-cockpit/tests/reconcile.test.ts`

## Acceptance evidence

- Adapter disconnect turns cached current resource state stale (or preserves
  no-payload unknown); the resource snapshot never fabricates current health.
- `rendersResourceCurrent(view)` is true exactly for reconciled,
  non-tombstoned `CURRENT` records and false for every generated stale,
  unknown, unreconciled, retired, or disconnected case.
- Rendered stale data is labeled last reported, unknown has no current domain
  health/meter, and no disallowed case receives current freshness/live styling,
  even when the decoded adapter projection says `serving` or `ok`.
- Stream-break then snapshot repair follows the same dominance rule without a
  half-reconciled model.
- Each presentation mutant is killed by the generated independent oracle.

## Ordering constraints

Depends on both the execution bridge and durable reconnect checkpoint so the
vector proves the real degradation source before asserting presentation. This
is verification of the existing cockpit contract, not new UI design.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` at high reasoning, explicit caller selection for stale-state dominance evidence.
- Review weight: `thorough`, explicit caller override; left at `review` for the verification deep lane.
- Files changed: new `resource-stale-never-live.json`, server/web conformance runners, and `web-cockpit/tests/resource-view.test.ts` plus generated vector traceability.
- Tests added: real authenticated adapter attach/report/delivery-stream-drop followed by durable resource replay and ResourceSnapshot materialization; vector-driven model+DOM stale/unknown assertions with deliberately `serving` domain health; a 100-run fast-check property over internally valid freshness/reconciliation/tombstone/health combinations; explicit current-eligibility mutants.
- Mutation evidence: (1) removed the production resource `adapter_stale_event` from abnormal stream-drop reconciliation; the server vector timed out waiting for stale and failed. (2) changed `rendersResourceCurrent` to freshness-only; both fast-check and the strengthened vector failed on unreconciled/tombstoned current witnesses. (3) changed DOM effective freshness to promote adapter-owned `health=serving`; the vector failed because stale rendered current. All mutations were reverted.
- Verification: both package checks reported exact ids; `cargo test --workspace`, clippy with warnings denied, contracts build/vector/drift/presentation/model checks, and web cockpit 105/105 passed.
- Simplification: reused the real attachment stream drop and canonical resource renderer; no connectivity enum, UI state, or test-only production helper was introduced.
- Discrepancies from design: the generated property lives in `resource-view.test.ts`, where both model predicate and DOM can be judged together, rather than splitting duplicate generators across model/resource-view files. Existing reconcile fast-check coverage continues to cover stream-break and unequal-horizon repair.
- Adjacent issues parked: none.

## Deep-lane review (2026-08-04)

Converged at pass 6 (clean pass — no receiver-confirmed material current-cycle blocker). Deep-lane cross-model review ran 6 fresh-context passes with adversarial mutation testing of every promoted vector/property/traceability/assurance claim; all material blockers were fixed and each drift class is now data-driven-guarded. See the parent feature body `## Deep-lane review (2026-08-04)` for the full convergence record. Advanced to `done`.
