---
id: fleet-target-resolution-review-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: fleet-spawn-target-resolution
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-14
updated: 2026-08-14
---

# Thorough review — Unit 1 target resolution and compound authority

**Verdict: BLOCKER.** Commit `f217140` correctly makes both halves of the live in-memory authority decision load-bearing, resolves continuations only from the exact current logical-target generation, uses one sampled decision time under the production `CoreDecisionGate`, and rejects incompatible spawn target kinds before append. However, the continuation guard was removed before the resolved claim and replacement-Grant provenance became part of the durable acceptance decision. The accepted record therefore preserves only the broad adapter-spawn Grant, does not activate the claim projection, and remains eligible for the legacy broad-grant completion path. This is an accepted-but-invalid state and fails the story's binding carriage, exclusivity, replay, and un-guarding requirements.

## Findings

### BLOCKER — Accepted continuation discards the exact claim and replacement Grant before durability

**Location:** `core/src/acceptance/pipeline.rs:386-413`; `core/src/session/spawn_claim.rs:235-246`; `server/src/state.rs:1152-1161`; `core/src/authority/spawn_tail.rs:179,473-536`; vacuous oracle at `core/tests/acceptance_pipeline.rs:1283-1295`

The resolver and second authority decision produce the required `SpawnGenerationClaim` and `ContinuationAuthorityProvenance`, and `validate_spawn_authority_carriage` verifies them. The pipeline then drops `target_binding` and `authorization.continuation_authority` and appends an ordinary `AcceptedOperation` containing only `authorizing_grant_id = spawn-grant`.

That violates the binding requirement that accepted continuation state carry both selected Grant ids and the exact prior generation:

- `SpawnClaimRegistry` folds `SpawnClaim` and `SpawnPromotionCommitted`, not the ordinary `Operation` event that this path writes. The first accepted continuation therefore does not consume the exact N→N+1 claim. Until another unit separately manufactures a claim event, a distinct continuation command can resolve the same still-current prior and be accepted as another available claim.
- Restart replay can recover the payload's prior and broad spawn Grant, but cannot recover which exact replacement Grant authorized acceptance. The accepted authority decision is therefore not replayable as the compound decision that actually occurred.
- The existing `SpawnDescendantTail` still consumes ordinary spawn `Operation` events and records only the broad spawning Grant in `AcceptedSpawn`. It has no replacement Grant id to preserve or recheck, so this newly unguarded continuation can enter the legacy completion machinery without the required exact-prior provenance.
- The acceptance test calls the compound check but then decodes the durable envelope and asserts only the broad `authorizing_grant_id`. The current provenance drop passes that test, making the durable-carriage oracle vacuous.

**Concrete fix:** do not admit continuation as an ordinary `StoredEventKind::Operation`. Either (a) land the atomic Unit-2 acceptance writer now so the deduplicating transaction persists one `SpawnClaimAccepted` decision containing the normalized `AcceptedOperation`, exact generated claim, `ContinuationAuthorityProvenance`, pending-replacement fence, and prior-work effects, or (b) restore the continuation guard until that writer exists. The managed continuation must not remain eligible for the legacy broad-grant completion path. Add production-composition tests proving: the durable accepted envelope round-trips both Grant ids plus the exact prior; replay reconstructs the same claim/provenance; two distinct commands for one prior cannot both accept; and removing persisted replacement provenance fails even when the live authority calls still execute.

## Checklist disposition

| Review requirement | Result |
|---|---|
| Fresh adapter-spawn Grant selection | **PASS** — deterministic lowest live UTF-8 Grant id. |
| Continuation compound live decision | **PASS in memory** — exact-prior `session-management` Grant selected for the same verified issuer/domain at the same sampled time. |
| Both Grant ids + exact prior in accepted envelope | **FAIL / BLOCKER** — only the broad spawn Grant is durable. |
| Missing/revoked/expired/wrong subject/endpoint/generation rejection | **PASS** before append in focused authority tests. |
| Runtime/resource/fleet/domain/malformed/mixed target rejection | **PASS** by the operation-aware resolver before acceptance; adapter shape support remains correctly deferred to delivery. |
| Exact current prior; tombstoned/stale/mutated prior inert | **PASS** for resolution through exact `record.current == prior`; one-field mutations reject. |
| Racing/competing continuation claim | **FAIL / BLOCKER** — acceptance writes no claim event, so the claim query has no new owner to observe. |
| Un-guarding safety | **FAIL / BLOCKER** — the old guard is gone before durable compound carriage exists. |
| Consumption of landed claim/promotion machinery | **PARTIAL** — resolution reads the claim registry, but accepted continuation bypasses its durable acceptance event. |
| Replay/determinism | **PASS** for live Grant selection and exact target lookup; **FAIL** for durable compound-decision replay because the replacement Grant id is discarded. |

## Mutation matrix

All reviewer mutations were temporary, restored with `git restore`, and followed by a clean focused pass.

| Mutation | Result | Focused oracle |
|---|---|---|
| Remove the adapter-spawn half by trusting caller-provided spawn provenance in `check_resolved_at` | **KILLED** | `continuation_compound_authority_requires_exact_live_replacement_and_selects_canonically` failed at the replacement-only witness: “replacement Grant alone is insufficient…” |
| Remove the exact-prior replacement half by returning the selected spawn Grant for continuation | **KILLED** | The same test failed at the broad-spawn-only witness: “the broad spawn Grant alone is insufficient”. |
| Restore clean source | **PASS** | The focused authority test passed after restoration. |
| Discard claim/replacement provenance after the checks | **SURVIVES in committed code** | No mutation was necessary: `pipeline.rs:407-413` already does this, while `acceptance_pipeline.rs:1283-1295` still passes because it asserts only the broad Grant. |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 29/29 tests.

The worktree was clean after both mutation restores and before this review file was written.

## Final recommendation

**Return `fleet-spawn-target-resolution` to `implementing`.** Keep continuation guarded until acceptance atomically persists the claim and complete compound authority, or pull that atomic acceptance slice forward. Re-run the thorough review after the accepted envelope, claim exclusivity, and replay oracles are real.
