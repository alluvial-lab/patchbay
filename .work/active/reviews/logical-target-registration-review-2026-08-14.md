---
id: logical-target-registration-review-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-logical-target-registration
created: 2026-08-14
updated: 2026-08-14
---

# Thorough review — Unit 3 claimed-successor staging and external-runtime reservation

**Verdict: MATERIAL.** Commit `03b9ea0` correctly routes exact managed fresh-generation-1 and continuation-N+1 reports through the shared classifier into one dedicated staged-evidence append, keeps the successor non-current until promotion, transactionally preserves one exact external-runtime owner, retains tombstone reservations, and kills all four requested reviewer mutations. The post-poison/post-promotion exact-retry path is race-safe under the shared decision gate, but it performs an unbounded full authority-log read while holding that global gate. That is the review's explicit material performance/availability failure.

Review mode: fresh-context delegated story review, effective weight `thorough`, one rigorous pass over `a8d0818..03b9ea0` plus current promotion, replay, checkpoint, and ordinary-ingress consumers.

## Findings

### MATERIAL — A late exact retry scans the complete authority log while holding the global decision gate

**Location:** `server/src/adapter_service.rs:419-451,1054,1217-1235`

For every correlated managed report that no longer classifies as `ClaimedSuccessor`—the expected state after claim poison or promotion—`existing_staged_successor_retry` calls `read_after(domain, Lsn { value: 0 })`, materializes every event in the authority domain, walks the complete vector, and decodes each staged-successor envelope. There is no cursor, lookup index, size bound, or early keyed read. Because `ingest_observation` holds `CoreDecisionGate` from line 1054 through this scan, repeated authenticated retries cost O(total durable history) each and serialize unrelated submission, revocation, delivery, attachment, and promotion decisions behind that work. A current adapter can therefore turn an ordinary lost-response retry into a growing global availability cliff.

The reconciliation is otherwise correct: it requires exact durable claim/report/source-attachment equality, returns only the original event id, does not construct a new `ClaimedSuccessor`, and cannot race the promotion fold because `SpawnCompletionDriver` uses the same decision gate. Those facts do not make the scan bounded.

**Concrete fix:** add a dedicated read-only staged-successor reconciliation port backed by an exact durable lookup keyed at least by authority domain and claim operation, with canonical envelope bytes/hash and original event id recorded atomically by `append_spawn_successor_staged_idempotent`. Serialize lookup/index maintenance through the storage writer or preserve the current shared-gate order, but never reconstruct staging authority. Add (1) a bounded-work oracle showing retry lookup does not grow with unrelated log length, (2) a poison/promotion barrier test proving reconciliation returns the original id on either side of promotion, and (3) an exact-mismatch test proving the lookup cannot admit or append a new stage.

## Checklist disposition

| Requirement | Result |
|---|---|
| Managed staging only; prior stays current | **PASS** — `ClaimedSuccessor` uses only `append_spawn_successor_staged_idempotent`; managed reports publish no `SessionRegistered`/`SessionGenerationBumped`, and N remains current until promotion. |
| Exact reverse key and one logical owner | **PASS** — the authority-domain-bound registry keys adapter/deployment/runtime/generation, rejects a second owner before append, and retains current/candidate/tombstone ownership through hot fold, replay, checkpoint recovery, and restart. |
| Atomic fresh-target rejection | **PASS** — staging folds target creation and reservation on a cloned identity projection, so a duplicate cannot leak an empty fresh target. |
| Wrong-shape rejection / enumerate-first | **PASS** — Operation correlation, attachment, adapter, deployment, runtime framing, expected prior, and generation all feed the shared classifier; ordinary ingress separately rejects `spawn_origin`. |
| Unmanaged first registration | **PASS** — an authenticated no-claim/no-origin report still follows the ordinary registration path. |
| Exact retry after poison/promotion | **FAIL / MATERIAL on boundedness** — equality and gate ordering are correct, but the new full-prefix scan is unbounded under the global decision gate. |
| Typed duplicate mapping | **PASS** — `LogicalTargetError` maps to typed `StorageError::DuplicateNativeReference`, then `FAILED_PRECONDITION` with the canonical `duplicate-native-reference` label; no secret-bearing field is surfaced. |
| Generic-route exclusivity | **PASS** — raw, audited, batch, decision, and dedup generic routes reject `SpawnSuccessorEvidenceStaged`; only the dedicated writer admits it. |
| In-transaction pre-state validation | **PASS** — the dedicated SQLite writer replays and validates adapter/session/claim pre-state inside its transaction before insert. |

## Mutation matrix

All mutations were made on the main tree, exercised with one focused test, and immediately reverted with `git restore`. Each requested mutant was killed, and the tree was clean after every restore.

| Mutation | Result | Focused oracle |
|---|---|---|
| Bypass managed staging and send `ClaimedSuccessor` through ordinary session ingestion | **KILLED** — the continuation oracle observed a `SessionState` registration instead of staged evidence | `cargo test -p patchbay-core-server exact_continuation_report_stages_n_plus_one_without_publishing_it` |
| Remove staged external-runtime reservation from the session fold | **KILLED** — the duplicate-owner candidate no longer returned `DuplicateNativeReference` | `cargo test -p patchbay-core --test runtime_evidence_promotion duplicate_staged_runtime_rejection_is_atomic_for_a_fresh_hot_fold` |
| Drop prior external-runtime ownership when promotion tombstones N | **KILLED** — `owner_of(generation_one)` became `None` | `cargo test -p patchbay-core --test logical_target_identity slot_transitions_are_exact_and_tombstones_retain_ownership` |
| Weaken the shared classifier by omitting exact claimed-generation equality | **KILLED** — the wrong-generation report classified as `ClaimedSuccessor` | `cargo test -p patchbay-core --test runtime_evidence_promotion classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation` |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 killed mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 38/38 tests, including the real-core loop.

The worktree was clean after mutation restoration and after all full-suite commands.

## Recommendation

**Return `research-handoff-spawn-logical-target-registration` to `implementing`.** Preserve the staging-only classifier boundary, dedicated transactional append, exact reverse reservation, and current mutation oracles. Replace the late-retry full-prefix scan with a bounded exact reconciliation lookup, add the promotion-order/bounded-work evidence above, then re-run the thorough review before advancing to `done`.
