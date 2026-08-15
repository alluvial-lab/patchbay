---
id: research-handoff-spawn-generation-monotonicity-tombstoning
kind: story
stage: implementing
tags: [protocol, verification, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-registration]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Atomic promotion fold, exact generations, and tombstones

## Redesign disposition

Rewritten. The old adapter report-driven generation advance is superseded. Only `SpawnPromotionCommitted` can publish a managed generation.

## Checkpoint

Implement each projection's fail-closed fold of the single promotion event. For fresh spawn, validate `∅→1`. For continuation, validate exact current `N→N+1`, exact accepted/staged claim, external-runtime reservation, and exact prior. The same event tombstones N, installs N+1 current, consumes the claim/fence, installs descendant authority, and completes the Operation according to projection ownership.

There is no state in which N+1 is current without descendant authority. Staged candidates remain non-live. Runtime session id may change; logical target remains stable.

## Design

**Files**
- `core/src/session/{events,registry,logical_target,spawn_claim,replay}.rs` — session/claim promotion fold.
- `core/src/authority/{events,projection,replay}.rs` — descendant authority fold.
- `core/src/acceptance/{index,replay,transitions}.rs` — completed command fold.
- `server/src/{checkpoint,snapshot}.rs` — promotion-aware recovery state.
- `specs/seed/session_generation.qnt` plus conformance/property tests.

Each projection independently validates the promotion event against its pre-state. Unknown/malformed event shape, missing claim/stage/Grant, wrong transition, duplicate runtime ownership, or conflicting replay fails before any installed mutation.

## Acceptance evidence

- [ ] Managed current state changes only through promotion, never a raw/staged report.
- [ ] Fresh accepts only generation 1; continuation accepts only exact N+1 with changing runtime id allowed.
- [ ] N tombstone and N+1 current/authority/completion become observable only from one complete promotion event.
- [ ] Promotion permanently consumes the claim and replacement fence while retaining exact provenance/tombstone/reverse-index history.
- [ ] Equal/lower/unclaimed-greater evidence cannot promote.
- [ ] Replay/checkpoint produce the exact same logical targets, claims, Grant, command state, and tombstones.
- [ ] Formal/model and executable mutations catch missing strict advance, exact pre-state, uniqueness, or authority-bearing atomicity.

## Ordering constraint

Consumes staged claimed-successor evidence. The stale fence implements every ingress against this finalized projection before completion is enabled.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` — caller-selected for the security-critical generation-lifecycle boundary; direct-read implementation extended the landed Leaf 6/Unit 3 fold rather than introducing another owner.
- Review weight: `thorough` (caller override); independent review follows at the feature/orchestrator layer.
- Files changed: `core/src/session/{runtime_evidence,registry,logical_target}.rs`, `core/tests/runtime_evidence_promotion.rs`, `server/src/snapshot.rs`, and `specs/seed/session_generation.{qnt,emitted.tla}`.
- Mechanism: the ordered authority → session → claim → command fold now performs a read-only exact projected pre-state check before staging any mutation: the durable claim/fence must remain active or poisoned, projected current must equal exact prior N (or be absent for fresh), the staged reservation must equal claimed N+1, and current/candidate reverse ownership must still belong to the same logical target. The existing per-projection checks remain independent. Promotion then retains both session and logical-target tombstones and both reverse-index reservations.
- Checkpoint/replay: checkpoint validation now accepts a managed successor whose native runtime id changes while requiring the session tombstone to match the logical-target tombstone/current lineage. Logical-target checkpoint hydration rejects duplicate generation or non-increasing tombstone histories and preserves tombstoned reverse ownership.
- Tests added/strengthened: fresh/continuation promotion fixtures now cover a changed native runtime id; the dedicated transactional append covers valid exact promotion, wrong current N, projected-claim/N+1 mismatch, double promotion, and intervening candidate release with zero-write rejection; separate authority/session/claim/command replay equality and late-correlation reverse-index rejection are asserted; checkpoint decode covers changed-runtime tombstone and reservation retention. The Quint promotion module uses separate attempted evidence and fold actions, an independent exact+atomic oracle, and six concrete runs for fresh, continuation, wrong prior, unclaimed greater, duplicate promotion, and external-runtime exclusivity.
- Mutation evidence: pending final mutation runs after the implementation baseline is committed; required pre-state, tombstone-reservation, and model-guard/authority mutants are enumerated above and will be recorded before `stage: review`.
- Verification: focused Rust promotion/logical-target/snapshot tests and Quint compile/tests plus bounded exact-atomic and promoted monotonic checks pass. The four required full verification groups are pending the final clean-tree run.
- Simplification: reused `fold_spawn_promotion_ordered`, `ProjectionState`, the claim registry, reverse index, dedicated append, and existing projection observers; no second promotion reactor, mutable side table, proto change, or generated-contract run was added.
- Discrepancies from design: the named authority/claim/command promotion folds and aggregate recovery plumbing were already landed by Leaf 6 and Unit 3, so Unit 4 extends their shared pre-state gate and assurance instead of duplicating files or events. The committed generated Quint inspection artifact is updated with the authored model even though the story named only the `.qnt` source.
- Adjacent issues parked: none.
