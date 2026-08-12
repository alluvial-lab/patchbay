---
id: spawn-delivery-atomic-claim-idempotency-generation
kind: story
stage: implementing
tags: [adapter, protocol, security, verification]
parent: research-handoff-spawn
depends_on: [fleet-spawn-target-resolution]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Atomic spawn claim and prior-generation delivery fence

## Redesign disposition

Rewritten after the adversarial review. The old rule releasing any non-success terminal claim is superseded. Claim disposition is independent from `CommandState`, and continuation acceptance fences new work to N.

## Checkpoint

Under one `CoreDecisionGate` decision, catch up the durable claim and prior-work projections, validate claimability, derive the complete prior-work effects, and atomically append the deduplicated accepted spawn envelope containing the exact claim, compound continuation provenance, pending-replacement fence, and those effects. For continuation, the same accepted event activates the exact-N fence and records supersession/quiesce dispositions before delivery can observe the claim.

A distinct competing command cannot claim the same generation. Exact retry returns the original accepted record/claim. Active or poisoned claims exclude every new claimant.

## Design

**Files**
- `core/src/acceptance/{pipeline,index}.rs` — claimability and exact retry result.
- `core/src/session/spawn_claim.rs` — fold accepted claim/fence.
- `core/src/storage/port.rs` plus SQLite implementation — atomic dedup + claim exclusivity + audited acceptance.
- `server/src/{state,service,adapter_service}.rs` — decision-gate catch-up and delivery consumption of persisted claim.
- Acceptance/delivery barrier-race and replay tests.

The claim's exclusive key is authority domain + logical target + expected prior generation. A fresh claim has no prior and generation `1`; continuation is exact `N→N+1`. No cached `current + 1` calculation is allowed after acceptance.

Fence behavior:
- new N-bound submissions after the accepted claim reject with `superseded/replacement_pending`;
- accepted but never-offered N work is durably superseded in the acceptance decision;
- delivered/running N work is not redelivered and is handed to quiesce/effect reconciliation;
- the fence remains through poison and clears only through no-effect release with renewed N evidence, promotion, or abandonment.

## Acceptance evidence

- [ ] Two concurrent distinct claims for one expected generation produce at most one accepted record and one delivery.
- [ ] Exact retry returns the same claim and both continuation Grant provenance ids.
- [ ] Changed payload under the same key rejects without projection mutation.
- [ ] Claim activation, exact-N pending-replacement fence, and complete prior-work effect list are atomic and replay-identical.
- [ ] N-bound work cannot be accepted/offered after the fence; a barrier race has an explicit before/after winner, and no affected pre-fence work is omitted.
- [ ] Terminal command state alone never releases the claim; active/poisoned records block new claimants.
- [ ] Delivery carries the persisted claim/provenance and never reconstructs a generation.

## Ordering constraint

Consumes completed contracts and operation-aware compound resolution. Claimed-successor staging follows.
