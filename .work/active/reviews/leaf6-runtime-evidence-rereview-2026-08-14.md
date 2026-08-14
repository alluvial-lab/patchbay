---
id: leaf6-runtime-evidence-rereview-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-runtime-evidence-promotion-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-14
updated: 2026-08-14
---

# Deep re-review — runtime-evidence-promotion-contract (Leaf 6), 2026-08-14

Independent fresh-context `openai-codex/gpt-5.6-sol` thorough re-review of `eee06b2` plus fix commit `cc7cbb1` (base `6342016`), in the required completeness → adversarial order. **Verdict: BLOCKER (5), MATERIAL (0), NIT (0).** The fix genuinely lands the dedicated SQLite transactions, exact attachment comparison, exact descendant-Grant binding, generic-route rejection, four-view aggregate publication, and server recovery. It does **not** close the leaf: managed session evidence can still use ordinary registration/bump ingress, stale SessionReports still bypass the outer quarantine contract, a pre-delivery successful Result can later authorize promotion, typed quarantine accepts forged classification context, and two required security mutants survive the claimed mutation suite.

## Prior BLOCKER closure matrix

| Prior BLOCKER | Verdict | Re-review evidence |
|---|---|---|
| 1. Exact attachment + exclusive managed staging | **FAIL / partial** | Both classifier branches now require adapter id, adapter generation, and exact current attachment id, and the server stages reports carrying `spawn_origin`. But exported ordinary core ingestion still accepts a valid `spawn_origin` and emits `SessionRegistered`/`SessionGenerationBumped`; the server selects that ordinary path solely from `spawn_origin.is_none()`. |
| 2. No raw stale runtime evidence | **FAIL / partial** | Runtime-targeted generic Observations are classified before production observation ingestion, and late terminal candidates no longer persist raw. Stale SessionReports without `spawn_origin` still use the old reject + standalone-audit path rather than `QuarantinedRuntimeEvidence`; no outer source is written. |
| 3. Typed quarantine validation everywhere | **FAIL / partial** | Generic SQLite/AuditedStorage routes reject the special kind, malformed wire rejects, the dedicated append validates canonical audit framing and a real durable attachment, and it recomputes the disposition. It compares only the recomputed disposition, not the complete classification context; forged logical/current context commits. |
| 4. Descendant authority bound to exact Operation | **PASS** | Actor, endpoint, empty endpoint class, exact target, deterministic id, canonical eight-kind set, timestamps, parent provenance, continuation provenance, parent spawn scope/kind/liveness, and exact-prior replacement scope/kind/liveness are revalidated. One-dimension input mutations exist. |
| 5. Dedicated replayable promotion append only | **FAIL / partial** | Every enumerated generic production route rejects, the fixture is authority-valid, and SQLite stages all four projections before insert. But the semantic replay check accepts a Result committed before delivery responsibility and later uses it to complete promotion, contradicting the required delivered/running-before-success history. |
| 6. One aggregate publication path including claims | **FAIL for verification; implementation present** | `ProjectionState` owns `SpawnClaimRegistry`; rebuild/catch-up use `fold_spawn_promotion_ordered`; staged views publish under all guards; restart/catch-up evidence is real. Omission mutants die, but swapping the authority and session folds survives the test named as the order oracle. |

## BLOCKER findings

### 1. Managed spawn SessionReports still have ordinary registration/generation-bump bypasses

**Severity: BLOCKER**  
**Anchors:** `server/src/adapter_service.rs:950-1057`; `core/src/session/ingest.rs:105-182,395-460`; `server/src/adapter_service/tests.rs:1745-1849`; `core/tests/sessions_ingest.rs:454-480`.

The server chooses the security-critical managed path only when `report.spawn_origin.is_some()`. Otherwise it calls exported `session::ingest_session_report`, whose validator accepts `spawn_origin = Some(valid CommandId)` and whose no-current / greater-generation branches still emit `SessionRegistered` and `SessionGenerationBumped` carrying that correlation.

Two adversarial probes confirmed the bypass:

1. A temporary core test passed a syntactically valid managed `spawn_origin` directly to `ingest_session_report` and required rejection/no writes. The assertion failed: ordinary ingestion accepted and persisted it.
2. The production managed-report server fixture was rerun with only `spawn_origin` removed. Ingress succeeded and returned stored kind `7` (`SessionState`) instead of kind `18` (`SpawnSuccessorEvidenceStaged`).

Thus a buggy/forged managed report can make the candidate runtime current/live without `SpawnPromotionCommitted` or descendant authority, and the core API still retains the exact bypass named in prior BLOCKER 1.

**Concrete fix:** make ordinary `core::session::ingest_session_report` reject every non-`None` `spawn_origin`; make server ingress apply the shared claim/generation fence before ordinary registration/bump selection, including preventing an active claimed candidate from escaping merely by omitting its correlation. Add direct core and authenticated server tests for valid-origin rejection and omitted-origin active-claim non-publication.

### 2. Stale SessionReports do not route through atomic outer quarantine

**Severity: BLOCKER**  
**Anchors:** `server/src/adapter_service.rs:881-896,950-1092`; `core/src/session/ingest.rs:130-210`; `server/src/adapter_service/tests.rs:2024-2114`.

Only reports carrying `spawn_origin` enter `classify_session_report` and the typed quarantine append. Ordinary stale source-order and stale runtime-generation reports fall through to `ingest_session_report`, then produce a standalone `record_adapter_audit` and an RPC error. The existing authenticated stale-report test confirms zero `QuarantinedRuntimeEvidence` events for its two admitted stale candidates.

A temporary assertion requiring the two stale reports to appear only as outer quarantine envelopes failed `left: 0, right: 2`. This is not raw-Observation persistence, but it is still the old parallel stale-evidence semantic the resolved design forbids: the candidate and its audit are not one atomic replay envelope.

**Concrete fix:** classify every authenticated runtime SessionReport before ordinary ingestion. Route stale source-order, tombstoned/lower-generation, mismatched, and stale-producer candidates through the typed quarantine constructor and dedicated atomic append; ordinary ingestion should receive only authenticated `Current` reports or explicitly admissible unmanaged first registration. Add hot/replay tests over the actual authenticated server routes, not only manually constructed quarantine envelopes.

### 3. A Result committed before delivery can later mint authority and complete the command

**Severity: BLOCKER**  
**Anchors:** `core/src/acceptance/observation.rs:191-208`; `core/src/session/runtime_evidence.rs:156-177,234-245,1133-1160,1330-1376`; `core/src/acceptance/index.rs:203-244,252-306`.

Observation ingestion appends a raw Result before it checks whether the implied lifecycle transition is allowed. `next_spawn_promotion` records every correlated successful spawn Result regardless of command state at that Result LSN. Promotion validation checks only that delivered/running lifecycle evidence and the Result are all prior to promotion; it never requires the lifecycle evidence to precede the Result.

An adversarial fixture committed:

```text
LSN 6: successful spawn Result while command is accepted
LSN 7: accepted -> delivered
LSN 8: delivered -> running
LSN 9: staged successor
```

The dedicated SQLite promotion append accepted this prefix and committed promotion+audit. The temporary test requiring rejection failed. A report rejected as premature at ingress therefore remains durable evidence and can become authority-bearing later.

**Concrete fix:** validate the implied transition before appending an otherwise rejected Result, and independently require `max(delivered/running evidence LSN) < successful Result LSN` in the self-contained promotion envelope and each consuming projection. The command fold should require the Result to have qualified as deferred success at its own replay position, not merely find the command delivered/running at promotion time. Add result-before-delivery and result-before-running mutation fixtures.

### 4. Dedicated quarantine append accepts invented logical/current/claim context

**Severity: BLOCKER**  
**Anchors:** `core/src/session/runtime_evidence.rs:877-960`; `core/src/storage/rusqlite.rs:1624-1762`; `core/tests/runtime_evidence_promotion.rs:1542-1594`.

The typed validator checks candidate/external-target equality and reason/disposition compatibility. SQLite rebuilds the durable prefix and recomputes `RuntimeGenerationDisposition`, but then compares only that disposition to the framed disposition. For `Unknown` and ordinary `IdentityMismatch`, `classification.classified_target.logical_target_id`, `current`, `tombstone`, and `active_claim` are not compared with the rebuilt session/claim projections.

A temporary test started from a valid unknown-target quarantine, injected a fabricated logical owner plus fabricated `current` context, and called the dedicated append with a valid durable current attachment and canonical audit. The append succeeded; the assertion requiring rejection failed. The outer event is projection-inert, but its security/audit classification can lie, which is the exact semantic-validity portion of prior BLOCKER 3.

**Concrete fix:** reconstruct the complete expected `RuntimeEvidenceClassificationContext` from the durable sessions, logical-target ownership, tombstones, and claims, then require exact equality (or explicit canonical per-disposition absence/presence rules) for `classified_target`, `current`, `tombstone`, and `active_claim`. Add one-field-at-a-time forged-context tests, including fake logical owner and fake/mismatched active claim.

### 5. Required quarantine-redispatch and fold-reordering mutants survive

**Severity: BLOCKER (trustworthy verification)**  
**Anchors:** `core/tests/runtime_evidence_promotion.rs:942-1036,1097-1160`; `core/src/session/runtime_evidence.rs:410-487`; `core/src/acceptance/index.rs:49-84`.

The story claims mutation-strength coverage for nested quarantine redispatch and fold reorder/omission. Omission checks are real, but two exact code mutants survived:

- `CommandIndex` was temporarily changed to decode `QuarantinedRuntimeEvidence` and recursively dispatch a nested Observation as a normal `Observation`. `every_quarantine_family_is_outer_only_across_all_normal_hot_and_replay_folds` still passed because its command projection has no accepted command/pre-state for the nested successful Result to mutate.
- `fold_spawn_promotion_ordered` was temporarily changed to run the session fold before authority installation. `aggregate_promotion_fold_requires_and_publishes_all_four_views_in_order` still passed because it checks final state, not fold order.

These are required mutations (b) and (d), not optional polish. A `[verification]` leaf cannot claim convergence while named security mutants survive.

**Concrete fix:** give every nested candidate an independently seeded pre-state that would mutate if recursively dispatched (accepted/delivered spawn command, live session/report target, pending Elicitation, delivery state, diagnostics/authority witness as applicable). Make fold ordering structurally observable/enforced—for example, pass an authority-installed witness into the session phase or use phase-typed staged folds—then run an actual authority↔session swap mutant and require failure.

## Mutation-test matrix

All code mutations were temporary and restored immediately with `git restore`; the tree was clean after every run.

| Mutant / adversarial shape | Focused oracle | Result |
|---|---|---|
| Remove exact `attachment_event_id` comparison from `source_matches_current_attachment` | `classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation` | **KILLED** — wrong attachment became `ClaimedSuccessor`; test failed at line 869. |
| Omit authority fold and authority postcondition | `aggregate_promotion_fold_requires_and_publishes_all_four_views_in_order` | **KILLED** — missing-authority call incorrectly succeeded. |
| Omit claim fold and claim postcondition | same aggregate test | **KILLED** — final disposition remained `Active`, expected `Promoted`. |
| Remove generic `SpawnPromotionCommitted` rejection from rusqlite guard | `storage_stamps_and_commits_complete_promotion_plus_audit_atomically` | **KILLED** — generic `append` succeeded. |
| Swap session fold before authority fold | aggregate order test | **SURVIVED** — test remained green. |
| Decode quarantine and recursively dispatch nested Observation in `CommandIndex` | outer-only all-family test | **SURVIVED** — test remained green. |
| SQLite trigger aborts audit insert after promotion source/reservation work | `promotion_audit_failure_rolls_back_source_and_grant_identity_reservation` | **KILLED by existing fault injection** — source and identity reservation rolled back; complete retry used LSN 10/11. |
| Valid managed `spawn_origin` passed to ordinary core ingestion | temporary direct boundary test | **ADMITTED** — expected rejection/no write failed. |
| Remove `spawn_origin` from authenticated managed server report | managed staging integration test | **ADMITTED AS `SessionState`** — returned kind 7 instead of staged kind 18. |
| Require stale authenticated SessionReports to have outer quarantine sources | stale session-ingress integration test | **MISSING** — zero quarantine sources for two stale candidates. |
| Successful Result before delivered/running, then later lifecycle + stage | temporary dedicated-storage promotion test | **ADMITTED** — promotion committed. |
| Unknown quarantine with forged logical owner/current context | temporary dedicated-quarantine test | **ADMITTED** — typed append committed. |

### Required mutation table (a–f)

| Required mutation | Re-review status |
|---|---|
| (a) Publish N+1 without descendant authority | **Killed** by authority-omission mutant and final authority/session assertions. |
| (b) Dispatch nested quarantine candidate normally | **Not killed**; exact nested-Observation redispatch survived. |
| (c) Split source/audit / crash after source | **Killed** by the SQLite audit-insert failure trigger; source and reservation roll back. |
| (d) Reorder/omit authority/session/claim/command folds | **Partial**: authority and claim omissions die; authority/session reorder survives. |
| (e) Wrong attachment/generation/claim/prior/deployment/generation | **Killed for the named classifier dimensions**, including the manually removed exact attachment-id comparison. |
| (f) Promote through a generic route | **Killed** by manually removing the backend generic guard; focused storage test fails. |

## Additional fixture assessment

- Fully authority-valid integrated promotion: **present and genuinely replayable**.
- Continuation with both Grants plus promotion-time expiry/revocation: **present**.
- N→N+1 tombstoning: **present**.
- Grant-identity reservation rollback: **present and physically transactional**.
- Malformed quarantine wire rejection: **present across enumerated generic SQLite routes**.
- Real server catch-up/restart aggregate: **present**.
- Missing adversarial fixtures: pre-delivery Result ordering, full quarantine-context forgery, valid managed-origin direct-core rejection, stale SessionReport outer routing, and mutation-sensitive nested redispatch/order witnesses.

## Storage and atomicity assessment

The physical SQLite mechanisms are sound:

- promotion source, descendant identity reservation, completion audit, and audit index use one `rusqlite::Transaction` and one commit;
- quarantine source and canonical stale audit use one transaction;
- an injected audit insert failure rolls back both source and grant-identity reservation;
- generic promotion/quarantine paths reject in the production backend and `AuditedStorage` routes exercised by the tests;
- promotion is staged through authority/session/claim/command projections before insertion.

There is no physical source-without-audit, audit-without-source, or reservation-without-insert crash prefix in the production SQLite path. The remaining blockers are semantic boundary failures: a premature Result is judged replayable, quarantine classification context is only partially recomputed, and some admitted stale reports never enter the atomic quarantine transaction at all.

## Ordered publication and legacy-repair assessment

`ProjectionState` now owns claims, clones every affected projection, validates the full suffix, acquires every publication guard before assignment, assigns authority before targets and commands, and advances the cursor last with no suspension point. Hot catch-up and restart replay converge in the real server test. No partial public view was found.

The pre-promotion repair tail does not observe the `SpawnClaim` event or any later event once the first claim is present. Current managed ingress appends its claim before delivered/running, Result, and staged successor facts, so it cannot arm the legacy tail. A mixed upgraded history may still carry already accumulated pre-claim legacy facts into repair; that is the permitted real-data normalization seam, not same-operation dual live semantics. The implementation note's phrase “histories without `SpawnClaim`” is therefore broader than the code's actual prefix isolation, but the current managed path is genuinely unreachable and this is not a finding.

## Full verification suite

All requested baseline commands passed on the restored clean tree:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 38 declared mutation witnesses killed, generated bindings clean.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 9/9.
4. `cd pi-adapter && npm test` — **PASS**, 29/29 including the real core/adapter restart e2e.

Green baseline suites do not override the surviving manually injected mutants or the adversarial fixtures above.

## Final recommendation

**Return `research-handoff-spawn-runtime-evidence-promotion-contract` to `implementing`.** Fix only the bounded convergence scope above: close ordinary managed report/bump ingress, route stale SessionReports through typed atomic quarantine, enforce delivered/running-before-success ordering, validate the complete quarantine context against durable projections, and strengthen the two surviving mutation oracles. Re-run this thorough deep lane after the fixes; do not advance Leaf 6 to done yet.
