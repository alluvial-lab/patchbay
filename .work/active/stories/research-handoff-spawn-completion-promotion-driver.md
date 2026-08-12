---
id: research-handoff-spawn-completion-promotion-driver
kind: story
stage: implementing
tags: [protocol, security, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-stale-event-fencing, research-handoff-spawn-idempotency-duplicate-handling]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
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

- [ ] Result/report in either order cannot promote until delivered/running lifecycle, exact claim, staged successor, uniqueness reservation, complete authority provenance, and non-poisoned effect state exist.
- [ ] Continuation promotion fails if the exact-prior replacement Grant is absent/revoked/expired/mismatched at promotion.
- [ ] One atomic event is the first durable fact exposing descendant Grant, N tombstone/N+1 current, claim promotion, and completed state.
- [ ] Crash before atomic append leaves no promotion; crash after append replays the complete promotion.
- [ ] Driver bootstrap repairs/normalizes each legacy crash prefix once without duplicate Grant/audit/terminal writes.
- [ ] Generic Result ingestion cannot terminalize spawn and durable qualifying success still suppresses unsafe redelivery.
- [ ] Mutations promoting before authority, omitting either Grant provenance, or reviving revoked prior authority fail.

## Ordering constraint

Depends on complete generation/stale fence and duplicate/poison behavior. Restart orchestration cannot proceed until this owner is implemented and verified.
