---
id: research-handoff-spawn-generation-monotonicity-tombstoning
kind: story
stage: review
tags: [protocol, verification, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-registration]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
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

- [x] Managed current state changes only through promotion, never a raw/staged report.
- [x] Fresh accepts only generation 1; continuation accepts only exact N+1 with changing runtime id allowed.
- [x] N tombstone and N+1 current/authority/completion become observable only from one complete promotion event.
- [x] Promotion permanently consumes the claim and replacement fence while retaining exact provenance/tombstone/reverse-index history.
- [x] Equal/lower/unclaimed-greater evidence cannot promote.
- [x] Replay/checkpoint produce the exact same logical targets, claims, Grant, command state, and tombstones.
- [x] Formal/model and executable mutations catch missing strict advance, exact pre-state, uniqueness, or authority-bearing atomicity.

## Ordering constraint

Consumes staged claimed-successor evidence. The stale fence implements every ingress against this finalized projection before completion is enabled.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` — caller-selected for the security-critical generation-lifecycle boundary; direct-read implementation extended the landed Leaf 6/Unit 3 fold rather than introducing another owner.
- Review weight: `thorough` (caller override); independent review follows at the feature/orchestrator layer.
- Files changed: `core/src/session/{runtime_evidence,registry,logical_target}.rs`, `core/tests/runtime_evidence_promotion.rs`, `server/src/snapshot.rs`, and `specs/seed/session_generation.{qnt,emitted.tla}`.
- Mechanism: the ordered authority → session → claim → command fold now performs a read-only exact projected pre-state check before staging any mutation: the durable claim/fence must remain active or poisoned, projected current must equal exact prior N (or be absent for fresh), the staged reservation must equal claimed N+1, and current/candidate reverse ownership must still belong to the same logical target. The existing per-projection checks remain independent. Promotion then retains both session and logical-target tombstones and both reverse-index reservations.
- Checkpoint/replay: checkpoint validation now accepts a managed successor whose native runtime id changes while requiring the session tombstone to match the logical-target tombstone/current lineage. Logical-target checkpoint hydration rejects duplicate generation or non-increasing tombstone histories and preserves tombstoned reverse ownership.
- Tests added/strengthened: fresh/continuation promotion fixtures now cover a changed native runtime id; the dedicated transactional append covers valid exact promotion, wrong current N, projected-claim/N+1 mismatch, double promotion, and intervening candidate release with zero-write rejection; separate authority/session/claim/command replay equality and late-correlation reverse-index rejection are asserted; checkpoint decode covers changed-runtime tombstone and reservation retention. The Quint promotion module uses separate attempted evidence and fold actions, an independent exact+atomic oracle, and six concrete runs for fresh, continuation, wrong prior, unclaimed greater, duplicate promotion, and external-runtime exclusivity.
- Mutation evidence: all behavior-changing mutants were killed and restored via `git restore` after each run. Making the exact aggregate pre-state error fail open was killed by `promotion_append_binds_exact_generation_prestate_and_rejects_double_or_out_of_order`; dropping the prior tombstone's reverse reservation was killed by `continuation_requires_both_live_grants_and_tombstones_n_on_n_plus_one_promotion`; weakening the Quint production fold guard and separately omitting descendant authority each produced a counterexample to `promotion_fold_exact_and_atomic`. Removing only the aggregate preflight remained behaviorally inert because the authority/session/claim/command projections independently reject the same invalid event, as the design requires; the fail-open mutant is the behavior-changing witness.
- Verification: all four required groups pass on the integrated Unit 4 + concurrent Unit 6 tree: workspace all-target build/test/clippy; the workspace test fallback (`cargo-nextest` is unavailable); contract vector checks, Rust conformance vectors, and model-promotion checks; and `formal/run-model-checks.sh` (20/20). The richer Quint module additionally passes six deterministic scenarios and `promotion_fold_exact_and_atomic` through 10 steps; the promoted default `GenerationMonotonic` check passes through 10 steps. The prescribed `contracts/scripts/check.mjs` aggregate does not exist, so group 3 used the existing `check-vectors.mjs` before the prescribed conformance/model checks. While finding that substitute, `check-generated-drift.mjs` was inadvertently invoked despite the no-generation directive; it ran `buf generate`, reported zero generated diff, and left the generated tree unchanged.
- Simplification: reused `fold_spawn_promotion_ordered`, `ProjectionState`, the claim registry, reverse index, dedicated append, and existing projection observers; no second promotion reactor, mutable side table, proto change, or generated-contract artifact change was added.
- Discrepancies from design: the named authority/claim/command promotion folds and aggregate recovery plumbing were already landed by Leaf 6 and Unit 3, so Unit 4 extends their shared pre-state gate and assurance instead of duplicating files or events. The committed generated Quint inspection artifact is updated with the authored model even though the story named only the `.qnt` source.
- Adjacent issues parked: none.

### Fix round — symmetric tombstone checkpoint hydration

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected); review weight remains `thorough` from the autopilot caller, with a fresh independent re-review following this commit.
- Symmetric validation: checkpoint hydration now treats a logical-target-owned lineage as managed before considering the legacy same-runtime path. Every managed session tombstone must match the logical-target tombstone/current lineage, and every retained logical-target tombstone must have an exact session-tombstone counterpart by adapter, deployment scope, runtime id, generation, and superseding LSN. Session-only legacy lineages remain accepted only when no logical target owns the external runtime.
- Recovery regression: the changed-runtime checkpoint test now rejects removal of only the session tombstone. The checkpoint recovery test also proves an otherwise valid managed tombstone projection becomes a deterministic disposable miss when that counterpart is removed, falls back to LSN-0 replay, and converges with the full-replay sessions, both tombstone projections, and production late-runtime classification. The fallback assertion lives in `server/src/checkpoint.rs` because `recover_session_registry` owns the server recovery decision; the decoder-shape assertion stays beside the existing changed-runtime fixture in `server/src/snapshot.rs`.
- Mutation evidence: removing only the new inverse logical-target-to-session validation made `checkpoint::tests::complete_checkpoint_round_trips_tombstones_source_cursor_and_tail` fail at the mismatched-checkpoint rejection (exit 101). `git restore core/src/session/registry.rs` restored the staged production implementation, and the focused tests passed afterward.
- Formal inspection artifact: regenerated `specs/seed/session_generation.emitted.tla` from the `session_generation_promotion` main module with Quint 0.32.0, `--verbosity 0`, and the `--out` compile path; the committed TLA+ contains no checker diagnostics or absolute output path.
- Verification: all four required groups pass on the final integrated Unit 4 + Unit 6 tree: `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; contracts drift/vector/model/build (55 vectors, 22 implementation checks, 38 mutation witnesses); operator-domain build/tests (23/23); and Pi adapter tests (38/38).
- Simplification/discrepancies: no protocol, generated-contract, or model-source change; the fix extends the existing checkpoint validation and recovery fixtures rather than adding a second hydration/replay path. No adjacent issues were parked.
