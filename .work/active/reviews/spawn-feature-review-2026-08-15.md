---
id: spawn-feature-review-2026-08-15
kind: story
stage: done
tags: [review, spawn, feature]
parent: research-handoff-spawn
created: 2026-08-15
updated: 2026-08-15
---

# Integrated feature review — spawn logical-target and generation lifecycle

## Verdict

**NEEDS FIX — do not advance `research-handoff-spawn` to `done`.**

The core safety spine is strong: fresh generation 1 and exact N→N+1 continuation acceptance, compound authority, atomic claim/fence creation, durable offer responsibility, stale-evidence quarantine, successor staging, external-runtime ownership, claim poisoning, and one authority-bearing promotion decision compose coherently and survive hot/restart replay. No core authority bypass, duplicate-generation publication, raw-stale-event replay, or partial promotion was found.

The feature is not integrated enough to close, however. Four MATERIAL findings remain at the operator and substrate boundaries: command diagnostics do not fold the atomic promotion; web presentation can erase durable claim-poison retry risk when a later terminal transition arrives; target abandonment is only a claim-fold vocabulary shape rather than the promised operator recovery lifecycle; and four safety-critical contract leaves have no retained independent review artifact despite their own required thorough/deep-review handoff.

Review mode: independent fresh-context integrated feature review of baseline `f71dcd93420a8150e35f1ebb4c00b6c2cb8c8dda` (`main == origin/main` before this report), all 16 child stories, retained child reviews, foundation documents, core/server/storage/protobuf seams, operator-domain, web, CLI, Pi boundary, formal artifacts, and full clean-tree verification. No implementation or story file was modified.

## Findings

### MATERIAL 1 — command diagnostics ignore the atomic promotion completion source

**Locations:** `core/src/diagnostics/mod.rs:175-216,217-234,463-510`; `core/src/acceptance/index.rs:602-650`; `server/tests/spawn_completion.rs:2352-2388`.

`SpawnPromotionCommitted` is the sole managed source that makes the command completed together with descendant authority, session publication, and claim consumption. `CommandIndex` correctly has a dedicated promotion fold, but `DiagnosticsProjection` classifies `SpawnPromotionCommitted` as an inert sibling. Its spawn-claim handler also processes only `Accepted`; it ignores disposition changes. Consequently `QueryDiagnostics` / `patchbay-cli inspect-command` can replay a fully promoted spawn and still report the last ordinary `Delivered` or `Running` transition, even while the same result's audit page contains the promotion's `CommandCompleted/spawn_completion` record.

The integration test checks diagnostics only at the deliberately deferred pre-promotion prefix (`server/tests/spawn_completion.rs:2352-2369`), then completes promotion without asserting post-promotion diagnostics. That is why the full suite remains green.

This violates the single-source replay contract and produces contradictory operator evidence after a successful fresh spawn or continuation, including after core restart.

**Required direction:** make diagnostics fold the generated promotion envelope into the same completed command timeline semantics as `CommandIndex`, with the promotion event as terminal source; add hot, bounded-as-of, and restart command-inspection tests before and after promotion. Claim disposition should also remain inspectable rather than being silently ignored.

### MATERIAL 2 — a later terminal transition erases web-visible claim poison and retry risk

**Locations:** `web-cockpit/src/domain/model.ts:124-139,757-781,910-945`; `web-cockpit/src/ui/operation-delivery.ts:41-43,142-180`; `docs/UX.md:22-33`.

The browser projection has no independent claim-disposition or retry-risk field. On a poison event, `foldSpawnClaim` temporarily writes `FailureCode.EXECUTION_OUTCOME_UNKNOWN` into `CommandView.failureCode`. A subsequent command transition overwrites that same field with its command failure. Delivered cancellation or expiry therefore ends as only `cancelled` or `expired`, despite the durable claim remaining `poisoned_pending_reconciliation` and the external spawn effect possibly existing.

This is exactly the distinction the feature introduced: command terminality does not release or summarize claim state. It also conflicts with the foundation UX requirement that retry safety not be derived from `CommandState` alone and with the feature's stated reuse of canonical claim-poison/retry-risk presentation. No web test exercises a poisoned claim event followed by cancellation/expiry/failure.

**Required direction:** retain generated claim disposition/retry risk independently from `failureCode`; render the persistent `execution_outcome_unknown` warning alongside the terminal command outcome; cover poison→cancelled, poison→expired, poison→failed, later proved-none release, promotion, and reconnect replay. Do not expose adapter-private evidence bytes.

### MATERIAL 3 — target abandonment is schema/fold-only, not an operator recovery lifecycle

**Locations:** `.work/active/features/research-handoff-spawn.md:59,72-81,109,306-320,361-372`; `contracts/proto/patchbay/control.proto:15-37`; `contracts/proto/patchbay/operations.proto:132-151`; `core/src/session/spawn_claim.rs:654-715,1208-1218`; `core/tests/spawn_claim_registry.rs:1629-1642`.

The design makes operator target abandonment the last safe escape from an unreconcilable poisoned claim. It says abandonment retires the logical target, makes any candidate audit-only, permanently consumes the generation, and clears the replacement fence.

The implementation has only the generated `TargetAbandoned` disposition and a direct fold test. There is:

- no ControlService method or admitted Operation payload for abandonment;
- no server/storage writer constructing an authenticated, audited abandonment decision;
- no CLI or web action;
- no logical-target fold that retires the target or makes a staged candidate audit-only.

Production code only consumes or query-enumerates `TargetAbandoned`; it never constructs or appends it. The only `SpawnClaimAbandonmentEvidence` constructor is in the unit test. The current validator merely checks that the referenced event is prior in the same domain, not that it is a typed authenticated operator-abandonment decision. Even if such a disposition were appended internally, the claim fold would clear the fence while leaving logical-target retirement to nobody.

This is fail-safe for duplicate launch—the claim remains stuck—but operationally incomplete. When later exact adapter evidence cannot prove no effect or identify a runtime, the operator has no promised resolution path.

**Required direction:** either implement one authenticated, grant-checked, audited, atomic target-abandonment decision that updates target/candidate/claim/fence semantics and all operator projections, or explicitly reclassify abandonment as deferred and stop claiming the failure lifecycle is complete. The current half-contract must not be exposed through a generic append.

### MATERIAL 4 — four required deep contract-leaf reviews are not retained

**Locations:** `.work/CONVENTIONS.md:71-78`; contract-leaf review handoffs at `research-handoff-spawn-logical-target-identity-contract.md:65-66`, `research-handoff-spawn-continuation-payload-authority-contract.md:57-58`, `research-handoff-spawn-claim-registry-contract.md:66-67`, and `research-handoff-spawn-crash-external-effect-evidence-contract.md:66-67`.

All 16 child stories are `done`; dependencies resolve, and the child graph is acyclic. Retained standalone review files exist for 12 children. None exists anywhere in git history under `.work/active/reviews/` for these four safety-critical leaves:

1. `research-handoff-spawn-logical-target-identity-contract` (`[verification]`);
2. `research-handoff-spawn-continuation-payload-authority-contract`;
3. `research-handoff-spawn-claim-registry-contract` (`[verification]`);
4. `research-handoff-spawn-crash-external-effect-evidence-contract` (`[verification]`).

Their completion commits state that deep review converged and their bodies record review fixes, but the independent finding/verdict/mutation evidence is not inspectable. Three carry `[verification]`, for which project conventions require the deep two-phase convergence lane; all four explicitly stopped at review for independent adjudication. This feature itself disclaims an earlier unretained review as evidence, so accepting the same traceability gap for these leaves would be inconsistent.

**Required direction:** retain the actual review evidence if it can be recovered without invention; otherwise rerun independent fresh-context deep reviews and commit their reports. Do not reconstruct missing reviewer claims from completion commit messages alone.

## Integrated lifecycle assessment

| Lifecycle / contract | Result | Evidence |
|---|---|---|
| Fresh pre-accept rejection | **PASS** | Boundary/schema/target/Grant failures reject before claim durability; adapter target is explicit. |
| Fresh acceptance and delivery | **PASS** | Generation 1 claim, exact accepted envelope, atomic idempotency/claim decision, and durable offer marker precede stream yield. |
| Fresh successor and promotion | **PASS in core** | Exact runtime report stages without publication; required phase chain plus success Result reaches one audited `SpawnPromotionCommitted`; authority/session/claim/command aggregate publication is ordered. Operator diagnostics remain wrong per MATERIAL 1. |
| Continuation acceptance | **PASS** | Exact current prior, adapter-spawn Grant plus exact-prior session-management Grant, same verified issuer boundary, new command/key, N+1 claim, and replacement fence are composed under one decision gate. |
| Continuation quiesce/stage/promote | **PASS in core** | Prior must be current but unavailable/unknown; ordered quiesce→terminate→launch→identity→handshake→stage→success evidence is required; promotion-time replacement authority is rechecked. |
| Accepted but not durably offered | **PASS** | No external-effect evidence is invented; marker-free claim remains active/deliverable, while a committed offer marker is the conservative responsibility line. |
| Ambiguous result / disconnect | **PASS in core; MATERIAL at presentation** | Exact claim poisons, suppresses redelivery, retains fence/reservation across restart; web can later hide poison per MATERIAL 2. |
| Proved no external effect | **PASS** | Closed proof vocabulary, current attachment/exact claim/phase checks, and continuation prior-N liveness prevent silence-as-proof. |
| Identified runtime reconciliation | **PASS** | Exact original-claim and reverse-owner checks reserve/reconcile without auto-launch; poisoned claim can later promote after fresh handshake/success evidence. |
| Stale/mismatched ingress | **PASS** | Shared classifier and outer quarantine prevent nested raw evidence from normal projection mutation; source attachment/order are fenced. |
| Target abandonment | **INCOMPLETE** | Claim enum/fold test only; no operator decision or target retirement. MATERIAL 3. |
| Replay/checkpoint/crash | **PASS for implemented core paths** | Gap-free replay, explicit managed checkpoint provenance, promotion source+audit atomicity, crash-before/ack-loss prefixes, legacy one-way repair, and restart convergence are covered. |
| Cursor replacement/reconnect | **PASS** | Known suffix is idempotent; unknown cursor uses staged complete replacement with one CAS winner; omitted stale members are removed atomically. |
| Concrete Pi process supervision | **DEFERRED BY SCOPE** | `pi-adapter/src/delivery.ts:52-53,86-87` still rejects spawn. The feature explicitly assigns Pi-only blockers and process mechanics to `research-handoff-pi-adapter-capability`; this is not counted as a defect in the shared-contract feature, but no claim of concrete Pi spawn execution is warranted yet. |

## Cross-story and foundation alignment

### Passed

- The six contract leaves precede operational consumers; all 16 child dependencies exist, no cycle was found, and every child is `done`.
- Generated protobufs remain the wire source of truth across Rust and TypeScript; drift regeneration is clean.
- OperationKind, state, target, failure, and stored-event registries remain generated/central rather than scattered hand copies.
- Adapter-neutrality is preserved: project/cwd/native Pi cursor/process details stay in adapter-owned payload/evidence boundaries.
- Authority remains deny-by-default: continuation cannot launder broad spawn authority into exact-session replacement authority, and promotion rechecks the accepted replacement Grant rather than substituting another id.
- Durable-log projection rules align with `ARCHITECTURE`, `PROTOCOL`, and `SECURITY`: accepted-before-delivery, source-authenticated evidence, one domain LSN order, outer quarantine, staging before publication, and authority-before-completion.
- `VERIFICATION` is appropriately honest: `GenerationMonotonic` is checked-model, richer exact-promotion scenarios/invariant are bounded implementation/model evidence, and restart/descendant-grant claims are not mislabeled checked-normative.
- The Pi translator's current rejection is consistent with the feature's explicit downstream handoff, despite the broader brief wording.

### Not aligned

- `PROTOCOL`/feature failure recovery names explicit reconciliation/abandonment, but no complete operator abandonment decision exists (MATERIAL 3).
- `UX` requires persistent, capability-aware retry-risk truth, but the browser has only one overwritable failure field (MATERIAL 2).
- The durable completion source is authoritative in aggregate command state but inert in the command-diagnostics projection (MATERIAL 1).
- The substrate's retained-evidence discipline is not met for four contract leaves (MATERIAL 4).

## Verification evidence

All successful commands ran on the unmodified baseline tree; tracked status remained clean afterward.

1. **Rust:** `cargo fmt --all -- --check`; `cargo build --workspace --all-targets`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings` — **PASS**. Workspace tests: 617 passed, 0 failed across 56 test binaries/doctest groups.
2. **Rust property feature:** `PROPTEST_CASES=256 cargo test --workspace --features proptest` — **PASS**, 617 test functions, 0 failed, with property cases raised to 256.
3. **Formal registry:** `./formal/run-model-checks.sh` — **PASS**, 20/20 (all Quint typechecks plus promoted bounded checks).
4. **Exact promotion model:** all six `session_generation_promotion` Quint scenarios — **PASS**; `promotion_fold_exact_and_atomic` via Apalache through 10 steps — **PASS**; fresh TLA+ compile matches the committed 585-line inspection body byte-for-byte.
5. **Contracts:** TypeScript build, generated drift, vectors, model registry, presentation conformance, and presentation meta-test — **PASS**. 57 vectors, 17 promoted vectors, 26 implementation checks, 38 mutation witnesses, 54 promotion blocks, five presentation registries, axe/contrast/meta checks green.
6. **Operator domain:** build and test — **PASS**, 27/27.
7. **Web server:** test and real-core smoke — **PASS**, 31/31 plus authenticated core reachability.
8. **Web cockpit:** browser/type build and test — **PASS**, 135/135.
9. **CLI:** test, real-core resource projection, and core smoke — **PASS**, 48/48 plus both process smokes.
10. **Pi adapter:** build and test — **PASS**, 38/38, including the real AgentSession/core generation-bump, reconnect, and core-restart loop. The suite explicitly confirms current Pi spawn rejection.
11. **Token-commune adapter regression:** build and test — **PASS**, 63/63.
12. **Composed E2E:** `e2e` walking skeleton — **PASS**: core → Pi adapter/AgentSession → CLI lifecycle + lockdown → restart → bootstrap exit.
13. **Repository hygiene:** pre-report `git status --short`, `git diff --check`, generated-contract diff, and dependency/DAG validation — **PASS**; `/` retained 55G free.

Green verification does not clear the findings: the missing after-promotion diagnostics oracle, poison-plus-terminal web oracle, and production abandonment path are absent from the tested behavior.

## Cumulative operational risk

The core fails safely under ambiguity: it over-poisons rather than duplicates a runtime, retains exact ownership, and refuses unsafe redelivery. The residual risk is operator divergence and unrecoverable safe stalls rather than silent duplicate publication inside the reviewed core. A successful promotion can look nonterminal in diagnostics; a poisoned cancellation/expiry can look merely terminal in the browser; and an irreconcilable poison has no abandonment escape. Together those gaps make incident handling materially worse precisely when the feature is in its highest-risk state.

No additional authority or data-integrity blocker was found in the implemented core path. The concrete Pi spawn supervisor remains the largest downstream execution risk and must consume these contracts rather than bypass them.

## Recommendation

**Return the feature to implementation/review, without changing child history in this review commit.** Before the next integrated pass:

1. make command diagnostics promotion- and claim-aware;
2. preserve/render claim poison independently from terminal command failure;
3. complete or explicitly defer the authenticated atomic target-abandonment lifecycle;
4. restore or rerun the four missing contract-leaf deep reviews;
5. add end-to-end oracles for post-promotion command inspection, poison→terminal presentation/reconnect, and abandonment target/candidate/fence behavior;
6. rerun the same full suite and a fresh integrated rereview.

Do not advance `research-handoff-spawn` to `done` on this pass.
