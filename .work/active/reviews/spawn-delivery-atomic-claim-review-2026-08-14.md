---
id: spawn-delivery-atomic-claim-review-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: spawn-delivery-atomic-claim-idempotency-generation
created: 2026-08-14
updated: 2026-08-14
---

# Thorough review — Unit 2 atomic accepted claim and N-delivery fence

**Verdict: BLOCKER.** Commit `6f986fb` establishes a real single-writer transactional exclusivity boundary, derives prior-work effects from the durable in-transaction prefix, kills the requested exclusivity/fence/dedup/guard mutations, and replays claim/fence state consistently. It does not yet close the security-critical end-to-end carriage and completion boundary: delivery strips the persisted claim and compound provenance, and the legacy broad-Grant completion tail now treats continuation claims as ordinary spawns. The dedicated writer also admits payload/claim disagreement that the RPC pipeline happens to prevent outside the transaction.

Review mode: fresh-context delegated story review, effective weight `thorough`, one rigorous pass over `185fd1c..6f986fb` plus current consumer context.

## Findings

### BLOCKER — Delivery discards the exact claim and both-Grant provenance

**Location:** `contracts/proto/patchbay/adapter_control.proto:125-128`; `server/src/adapter_service.rs:575-599,618-640`

The accepted `SpawnClaimEvent` durably contains `SpawnClaimAccepted`, but `accepted_operation_for_delivery` extracts only `accepted.accepted_operation`. The wire `Delivery` can carry only that `Operation` and its event id. Consequently the adapter receives neither `SpawnGenerationClaim` nor `ContinuationAuthorityProvenance`: it cannot consume the persisted logical target, exact prior, claimed N+1 generation, or replacement Grant id, and any continuation implementation must reconstruct or obtain them from newer state. This directly contradicts the story's checked acceptance evidence that delivery carries persisted claim/provenance and never reconstructs a generation. The adjacent deployment-authority consumer accepts `SpawnClaimAccepted`, but this delivery path cannot supply it.

**Concrete fix:** extend the generated delivery contract with the exact durable accepted-spawn envelope (or a typed delivery-source union), populate it directly from the `SpawnClaimEvent` bytes, and make the adapter consume that envelope rather than only the nested Operation. Add hot-path and restart tests asserting byte/semantic equality for the full claim, spawning Grant, replacement Grant, exact prior, and claimed generation; mutate/truncate each field and require delivery/adapter authorization to fail.

### BLOCKER — Continuations remain eligible for the legacy broad-Grant completion path

**Location:** `core/src/authority/spawn_tail.rs:179-182,484-506`

`SpawnDescendantTail::observe_spawn_claim` unwraps every accepted claim to `AcceptedOperation` and feeds it to the same legacy completion path as an ordinary spawn. That path retains only `authorizing_grant_id`; it drops `compound_authority` and the exact prior. A continuation can therefore arm legacy audit/descendant-Grant/completion actions from later Result/session facts without proving that the accepted replacement Grant is still live. This is the Unit-1 pass-1 blocker in a new envelope: durable carriage exists, but the completion consumer throws it away.

**Concrete fix:** until Unit 7's atomic promotion owner lands, make the legacy tail ignore managed continuation claims (`expected_prior.is_some()`) rather than translating them to ordinary accepted spawns. Add a regression proving that continuation acceptance plus otherwise qualifying Result/session evidence yields no legacy audit, descendant Grant, or completion action, including after replacement-Grant revocation; retain an explicit fresh-spawn compatibility test if that bridge is still required.

### MATERIAL — The dedicated writer admits a claim that disagrees with the durable spawn intent

**Location:** `core/src/storage/rusqlite.rs:2636-2654,2710-2755`; `core/src/session/spawn_claim.rs:615-660,727-799`

The transactional writer stages `SpawnClaimRegistry` and `CommandIndex`, but `validate_accepted_decision` validates the claim only against its own fields. It never decodes the nested spawn request or binds request intent to claim semantics. A dedicated-route caller can therefore commit, for example, a fresh payload with an N→N+1 continuation claim/fence/provenance, a continuation payload naming a different prior, or the same Grant id in both authority slots. Replay accepts that internally self-consistent claim while delivery exposes a contradictory Operation. The RPC pipeline currently prevents these candidates before the writer, but the dedicated storage boundary does not meet the requested promotion/quarantine/staged fail-closed class bar on its own.

**Concrete fix:** reuse the canonical spawn payload and authority-carriage validators in the dedicated candidate validation and explicitly bind request intent/prior to the claim: fresh iff `expected_prior=None` and generation 1; continuation request prior equals claim prior, compound exact prior, and fence prior; claimed generation is exact N+1; both required Grant ids are present and distinct. Stage this full candidate against the transaction prefix before insert. Add no-write tests for one-field disagreement mutations.

## Checklist disposition

| Requirement | Result |
|---|---|
| Atomic distinct-claim exclusivity | **PASS** — the SQLite writer transaction serializes and validates the durable claim prefix; the barrier race has one append and one conflict. |
| Exact retry and changed-payload inertness | **PASS** — idempotency reconciliation precedes claimability and returns the original accepted bytes. |
| Fresh 1 / exact N→N+1 | **PASS for the pipeline-produced candidate**; dedicated-route intent binding is the MATERIAL gap above. |
| Guard removal and durable round-trip/replay | **PASS** for claim, both Grant ids, exact prior, and claim-registry replay. |
| Atomic exact-N fence and prior-work effects | **PASS** — the accepted event carries the transaction-derived stable-order effect list and activates the claim fence. |
| New N work / exact retry behavior | **PASS** — new records reject `superseded/replacement_pending`; an existing exact record deduplicates. |
| Delivery barrier race | **PASS** — controlled before/after gate ordering; no pre-fence N record is offered by the fence-first branch. |
| Fence/claim restart replay | **PASS**. |
| Generic claim-route exclusion | **PASS** for raw, dedup, audited, batch, and decision wrappers through the shared special-kind rejection. |
| Persisted claim/provenance reaches adapter delivery | **FAIL / BLOCKER**. |
| Continuation excluded from broad-Grant legacy completion | **FAIL / BLOCKER**. |

## Mutation matrix

All mutations were made on the main tree, tested with one focused command, and immediately reverted with `git restore`. The tree was clean after every restore.

| Mutation | Result | Focused oracle |
|---|---|---|
| Remove transactional claim conflict enforcement and staged claim-exclusivity validation | **KILLED** — two owners appended; assertion observed `2` vs `1` | `cargo test -p patchbay-core --test spawn_claim_acceptance distinct_continuations_race_to_exactly_one_durable_owner -- --exact` |
| Defer pending-replacement activation beyond the accepted-event fold | **KILLED** — race branch remained `Accepted` instead of `Superseded` | `cargo test -p patchbay-core --test spawn_claim_acceptance acceptance_fence_barrier_has_one_before_or_after_winner_and_exact_retry_survives -- --exact` |
| Disable spawn idempotency reconciliation before claimability | **KILLED** — retry returned `SpawnClaimConflict` instead of the original duplicate | `cargo test -p patchbay-core --test spawn_claim_acceptance exact_retry_returns_original_claim_and_changed_payload_is_inert -- --exact` |
| Re-introduce the Unit-1 continuation guard | **KILLED** — continuation became `Rejected` instead of `Accepted` | `cargo test -p patchbay-core --test acceptance_pipeline continuation_acceptance_round_trips_compound_claim_while_fresh_uses_single_grant -- --exact` |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 35/35 tests.

## Recommendation

**Return `spawn-delivery-atomic-claim-idempotency-generation` to `implementing`.** Preserve the working storage exclusivity/fence transaction and mutation oracles, but do not advance until delivery carries the exact accepted envelope, continuation cannot enter the legacy broad-Grant completion path, and the dedicated writer rejects payload/claim disagreement before durability.
