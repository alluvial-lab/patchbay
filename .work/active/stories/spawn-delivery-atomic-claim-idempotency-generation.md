---
id: spawn-delivery-atomic-claim-idempotency-generation
kind: story
stage: review
tags: [adapter, protocol, security, verification]
parent: research-handoff-spawn
depends_on: [fleet-spawn-target-resolution]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
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

- [x] Two concurrent distinct claims for one expected generation produce at most one accepted record and one delivery.
- [x] Exact retry returns the same claim and both continuation Grant provenance ids.
- [x] Changed payload under the same key rejects without projection mutation.
- [x] Claim activation, exact-N pending-replacement fence, and complete prior-work effect list are atomic and replay-identical.
- [x] N-bound work cannot be accepted/offered after the fence; a barrier race has an explicit before/after winner, and no affected pre-fence work is omitted.
- [x] Terminal command state alone never releases the claim; active/poisoned records block new claimants.
- [x] Delivery carries the persisted claim/provenance and never reconstructs a generation.

## Ordering constraint

Consumes completed contracts and operation-aware compound resolution. Claimed-successor staging follows.

## Implementation notes

- Execution capability: inline implementation in the host worker; stopped at `review` for independent review.
- Added a dedicated storage acceptance route that reconciles idempotency before claimability, rebuilds claim and command pre-state inside one SQLite transaction, derives the complete stable-order prior-work effect list, validates both staged projections, and atomically commits the accepted claim envelope, idempotency row, and acceptance audit. Generic storage routes reject accepted claim sources.
- Removed Unit 1's continuation guard. Fresh and continuation spawn now persist only the `SpawnClaimAccepted` source envelope. Continuation round-trips the spawning Grant, replacement Grant, exact N prior, N+1 claim, pending-replacement fence, and transaction-derived supersede/quiesce effects; exact retry returns those original durable bytes.
- `CommandIndex`, diagnostics, completion compatibility, projection recovery, and adapter delivery decode the accepted Operation from the claim envelope. A shared `CoreDecisionGate` linearizes offer eligibility against claim activation; command replay suppresses superseded and quiescing N-bound work. Fresh claimed-successor staging now creates the logical-target record from the first authenticated deployment-scoped report when no prior generation exists.
- Dedicated evidence covers distinct-claim concurrency, exact retry and changed-payload conflict, complete effect derivation, replay/restart fence reconstruction, generic-route exclusion, canonical post-fence rejection, and deterministic delivery-first/fence-first barrier outcomes. Existing completion/evidence integration fixtures were migrated to the dedicated writer.
- Controlled mutations, all reverted with `git restore`: removing claim exclusivity produced two accepted owners and failed `distinct_continuations_race_to_exactly_one_durable_owner`; removing idempotency defenses appended a duplicate claim and failed `exact_retry_returns_original_claim_and_changed_payload_is_inert`; dropping prior-work fence effects offered `n-bound-race` after activation and failed the delivery barrier oracle; truncating compound provenance failed the continuation durable round-trip oracle.
- Verification group 1 — `cargo test --workspace --all-features`: **PASS**, including spawn claim race/replay, server barrier, full completion, and execution-evidence integrations. `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Verification group 2 — repository model runner `./formal/run-model-checks.sh`: **PASS**, 20/20 checks. (The requested legacy `./scripts/verify-models.sh` path is not present in this checkout.)
- Verification group 3 — contract generation/build/drift, vectors, model traceability, presentation checks, and generated-tree diff: **PASS**. No `.proto` or generated contract changes.
- Verification group 4 — `npm --prefix {pi-adapter,cli,web-cockpit,web-server,e2e} test`: **PASS** (35, 46 plus real-core resource, 128, 31, and walking-skeleton suites respectively); `cargo fmt --all --check`/`git diff --check`: **PASS**.
- `pi-adapter` source changes present in the shared checkout belong to the parallel deployment-authority worker and were not modified or included in this story's commit.
