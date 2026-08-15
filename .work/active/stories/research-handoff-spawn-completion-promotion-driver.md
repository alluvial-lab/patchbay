---
id: research-handoff-spawn-completion-promotion-driver
kind: story
stage: review
tags: [protocol, security, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-stale-event-fencing, research-handoff-spawn-idempotency-duplicate-handling]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
---

# Atomic spawn promotion completion driver

## Checkpoint

Own the previously unassigned completion seam: `server/src/spawn_completion.rs` and `core/src/authority/spawn_tail.rs`. Migrate grant-before-completed behavior so a managed fresh/continuation spawn produces one `SpawnPromotionCommitted` event only after every success fact and descendant authority record is complete.

For continuation the fold requires the accepted adapter-spawn Grant provenance **and** the exact-prior session-management Grant provenance; it rechecks the latter is live at promotion. Revocation/expiry suppresses promotion rather than allowing descendant authority to revive a revoked target.

## Design

**Files**
- `core/src/authority/spawn_tail.rs` — fold accepted claim/provenance, lifecycle, Result, staged successor, effect state, reverse reservation, and promotion readiness.
- `server/src/spawn_completion.rs` — sole driver under `CoreDecisionGate`.
- `core/src/storage/port.rs`, audited decorator, SQLite backend — atomic promotion source+audit append with exact event linkage.
- `server/src/main.rs` — bootstrap repair before listeners.
- `server/tests/spawn_completion.rs` and authority/replay tests.

```rust
pub enum SpawnCompletionAction {
    CommitPromotion(SpawnPromotionPlan),
    PoisonClaim(SpawnPoisonPlan),
}
```

The managed path no longer emits independent completion audit → descendant Grant → completed events. It constructs the complete descendant Grant (including both continuation Grant ids), asks storage to atomically stamp/append the promotion and audit, then reads the committed event back through the same folds. Stderr/public success occurs only after the event folds successfully.

One-way replay migration handles real legacy prefixes from the previous completion driver: evidence-only, audit-only, audit+grant, and completed. It verifies the old rules, never reclassifies partial legacy state as a new managed claimed successor, and either repairs under an explicit legacy normalization or leaves bounded deferred evidence. No duplicate Grant or terminal transition is emitted.

## Acceptance evidence

- [x] Result/report in either order cannot promote until delivered/running lifecycle, exact claim, staged successor, uniqueness reservation, complete authority provenance, and non-poisoned effect state exist.
- [x] Continuation promotion fails if the exact-prior replacement Grant is absent/revoked/expired/mismatched at promotion.
- [x] One atomic event is the first durable fact exposing descendant Grant, N tombstone/N+1 current, claim promotion, and completed state.
- [x] Crash before atomic append leaves no promotion; crash after append replays the complete promotion.
- [x] Driver bootstrap repairs/normalizes each legacy crash prefix once without duplicate Grant/audit/terminal writes.
- [x] Generic Result ingestion cannot terminalize spawn and durable qualifying success still suppresses unsafe redelivery.
- [x] Mutations promoting before authority, omitting either Grant provenance, or reviving revoked prior authority fail.

## Ordering constraint

Depends on complete generation/stale fence and duplicate/poison behavior. Restart orchestration cannot proceed until this owner is implemented and verified.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; direct-read implementation for the security-critical promotion boundary. The active autopilot caller supplied one cohesive story owner, and the recursion guard precluded implementation fan-out; an independent thorough review follows at the feature boundary.
- Review weight: `thorough`, supplied by the autopilot caller.
- Files changed: `core/src/authority/{spawn_tail,registry}.rs`, `server/src/spawn_completion.rs`, `core/tests/{authority_spawn_tail,runtime_evidence_promotion}.rs`, `server/tests/spawn_completion.rs`, and the rolling-foundation assertions in `docs/{ARCHITECTURE,PROTOCOL,SECURITY,VERIFICATION}.md`. This story file is the only `.work/` mutation.
- Mechanism: extended the existing completion owner rather than adding a reactor. `SpawnDescendantTail` now records managed ownership per exact domain/command, emits `CommitPromotion` only after the same authority-readiness validator used by transactional replay reports both accepted Grant slots live at the candidate's sampled time, and suppresses revoked/expired provenance without substituting a Grant id. `SpawnCompletionDriver` always folds the command-scoped tail, drives the existing dedicated `append_spawn_promotion_audited` path, leaves suppressed candidates staged for explicit reconciliation, and excludes their permanently dead accepted provenance from later producer scans so one suppressed claim cannot head-of-line block unrelated ready promotions.
- One-way migration: every fresh or continuation `SpawnClaim` removes any Operation-shaped legacy progress and cannot emit a separate completion audit, descendant Grant, or generic completed transition. The legacy audit → Grant → terminal actions remain available to unrelated pre-managed histories, including evidence-only, audit-only, audit+grant, completed, and migrated duplicate-descendant prefixes; replay writes only the missing suffix and never duplicates an existing Grant or terminal.
- Atomic/crash evidence: driver tests inject failure before the promotion transaction and lost acknowledgement after commit. The first prefix contains no promotion/audit; the second contains the complete source/audit pair. Restart commits or replays exactly one authority-bearing promotion, and `ProjectionState` reconstructs the ordered authority → session → claim → command aggregate.
- Readiness/liveness evidence: result-first and report-first authenticated orders both remain non-terminal until all evidence exists. A driver bootstrap after Result but before staged successor stays quiescent. Fresh accepted-authority revocation suppresses the real driver; continuation replacement-Grant expiry and revocation suppress the shared decision. Existing exact-scope/provenance, monotonic generation, reverse-reservation, poisoning, and four-view fold tests remain green.
- Tests added/changed: managed-vs-legacy ownership and mixed-prefix authority-tail tests; continuation promotion-time liveness decision and suppressed-claim producer-exclusion tests; result/report order and staged-successor driver assertions; before/after atomic crash test; real-driver revocation suppression test. Existing generic Result exclusivity/redelivery, legacy crash-prefix, aggregate publication, authority laundering, and continuation provenance suites were retained and passed.
- Mutation kills: (1) weakening staged-successor readiness to admit a default candidate failed `managed_evidence_retries_complete_once_and_restart_as_a_replayable_prefix` (exit 101); (2) hiding legacy `Operation` events as managed failed `crash_prefixes_repair_to_one_audit_grant_and_terminal_transition` (exit 101); (3) converting suppressed expired/revoked authority into `CommitPromotion` failed `managed_completion_decision_suppresses_expired_or_revoked_exact_prior_authority` (exit 101); (4) removing the deferred Result branch and admitting successful spawn Result through the generic atomic transition writer failed the staged-successor driver oracle with an observed premature completed transition (exit 101). Each mutant was applied alone and removed with `git restore`; restored focused suites passed.
- Full verification group 1: **PASS** — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` (including 14 authority-tail, 34 runtime-evidence/promotion, 39 spawn-claim, 82 server-unit, and 12 server spawn-completion tests; doctests pass; clippy warnings denied).
- Full verification group 2: **PASS** — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` (55 vectors, 17 promoted, 22 implementation checks, 38 killed registered mutation witnesses, 54 model-promotion blocks; generated paths clean).
- Full verification group 3: **PASS** — `cd operator-domain && npm run build && npm test` (23/23).
- Full verification group 4: **PASS** — `cd pi-adapter && npm test` (38/38, including real core/adapter restart E2E).
- Additional checks: `cargo fmt --all -- --check`, `git diff --check`, focused restored mutation oracles, and no-proto/generated-diff checks passed.
- Discrepancies from design: the storage atomic-promotion port/backend, aggregate `ProjectionState`, staging/reservation/effect machinery, and pre-listener bootstrap already existed from prerequisite units and were consumed rather than duplicated; no storage, `server/src/main.rs`, proto, or generated-contract production change was required. The legacy enum retained its three explicit repair variants and gained boxed `CommitPromotion` instead of introducing a parallel poison action because Unit 6 already owns typed poison/reconciliation decisions.
- Simplification and adjacent issues: replaced the domain-global managed-history exclusion with per-command ownership, factored promotion-time authority checks so driver and storage replay share one validator, and removed no behavior outside the completion seam. No adjacent issue was parked and no design change was required.
