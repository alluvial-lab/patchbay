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
updated: 2026-08-14
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

### Fix round — thorough review findings (2026-08-14)

- Execution capability: `openai-codex/gpt-5.6-sol`; one cohesive security/protocol fix owner. Review weight: `thorough` from the autopilot caller; this item returns to `review` for the required fresh independent re-review.
- Delivery BLOCKER: added generated `Delivery.accepted_spawn`, regenerated the committed Rust/TypeScript bindings with `cd contracts/ts && npm run gen`, and changed the adapter-facing delivery tail to decode, validate, and carry the exact durable `SpawnClaimAccepted` envelope while retaining `Delivery.operation` as the compatibility view for ordinary/current consumers. Hot-path and restart tests compare semantic and encoded bytes for the complete claim, spawning Grant, replacement Grant, compound exact prior, and claimed generation. Truncating the claim or any named authority/generation field fails before an adapter can receive an authorized managed-spawn delivery. Per the caller's ownership boundary, no `pi-adapter/src` file was touched; the follow-up adapter worker will consume the now-generated `acceptedSpawn` field.
- Legacy-tail BLOCKER: `SpawnDescendantTail` now ignores accepted claims with `expected_prior.is_some()` before translating them to the one-Grant legacy completion context. A continuation plus delivered/result/session evidence remains inert even after the exact replacement Grant is revoked; a separate fresh managed-claim test preserves the existing compatibility bridge.
- Intent-binding MATERIAL: `validate_spawn_claim_accepted` now composes the canonical spawn-payload and authority-carriage validators with claim/fence validation. The dedicated writer's existing in-transaction staged `SpawnClaimRegistry` fold therefore validates the full payload/claim/provenance candidate against the durable prefix before insert. Fresh-vs-continuation intent, request/claim/compound/fence exact-prior equality, exact `N+1`, and distinct non-empty Grant ids are fail-closed. Four one-field disagreement tests assert a completely unchanged log.
- Unit 2 atomicity preservation: no production storage transaction, exclusivity, idempotency, prior-work effect derivation, or fence-ordering code changed; the confirmed four existing mutation oracles remain green.
- Tests added: `managed_spawn_delivery_preserves_the_exact_durable_envelope_hot_and_after_restart`; `managed_spawn_delivery_rejects_truncated_claim_and_authority_fields`; `managed_continuation_never_enters_the_legacy_one_grant_completion_tail`; `fresh_managed_claim_keeps_the_legacy_completion_bridge`; and four focused dedicated-writer no-write intent-binding tests.
- Mutation kills, all performed on the main tree and immediately reverted with `git restore`: removing `Delivery.accepted_spawn` population failed the hot/restart equality oracle; bypassing delivery-envelope validation failed the truncation oracle; removing the continuation early return produced `RecordAudit` and failed the legacy-tail oracle; removing canonical authority-carriage validation separately failed the fresh/continuation, payload-prior, and same-Grant focused no-write oracles; removing exact `N+1` validation failed the wrong-generation no-write oracle. The tree was restored after each focused run and no mutation was committed.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses).
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS** (23/23 tests).
- Verification group 4 — `cd pi-adapter && npm test`: **PASS** (35/35 tests).
- Simplification/discrepancies: one shared accepted-spawn validator now protects durability, replay, and delivery instead of adding a storage-only hand copy. The additive delivery field was chosen over a breaking `oneof` because current adapter source ownership is explicitly assigned to the follow-up worker; the exact envelope is nevertheless generated, populated from the durable event, and mandatory for managed-spawn authorization. No adjacent issue was parked from this fix round.
