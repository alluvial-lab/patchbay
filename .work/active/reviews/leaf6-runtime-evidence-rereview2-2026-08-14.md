---
id: leaf6-runtime-evidence-rereview2-2026-08-14
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

# Deep re-review pass 3 — runtime-evidence-promotion-contract (Leaf 6), 2026-08-14

**Verdict: BLOCKER (2), MATERIAL (0), NIT (0).** Independent fresh-context `openai-codex/gpt-5.6-sol` thorough pass 3 over `eee06b2 + cc7cbb1 + 4d18ce0`, in completeness → adversarial order. All five round-2 blockers are genuinely closed in the production paths, and required mutants (b)/(d) now die. The leaf still cannot advance: exact retries of two ordinary managed evidence facts can be durably accepted and then treated as corrupt history, making the promotion/recovery spine fail closed forever.

## Round-2 closure matrix

| Round-2 blocker | Verdict | Production-path evidence |
|---|---|---|
| 1. Managed ordinary-ingress bypass | **PASS** | `core::session::ingest_session_report` rejects every `spawn_origin`; authenticated server ingress classifies before ordinary registration/bump selection; valid-origin direct core ingestion writes nothing, and omitted-origin active-claim ingress returns outer quarantine without `SessionRegistered`/`SessionGenerationBumped`. |
| 2. Stale SessionReports not outer-quarantined | **PASS** | Actual authenticated server ingress puts delayed same-producer, old-producer, and old-runtime reports behind the dedicated atomic quarantine append. The hot/replay fixture finds two SessionReport candidates only as `QuarantinedRuntimeEvidence` sources plus their source-linked audits, with no session-state mutation. |
| 3. Result-before-delivery could later mint authority | **PASS** | Observation transition validity precedes append; the producer qualifies Result at its replay LSN; envelope, authority, session, claim, and command consumers enforce lifecycle-before-Result. The LSN `6 Result → 7 delivered → 8 running → 9 staged` prefix rejects. Legitimate `delivered → running → Result` still promotes. |
| 4. Quarantine classification-context forgery | **PASS** | The dedicated SQLite append rebuilds adapters, sessions/logical ownership/tombstones, and claims, reconstructs the complete canonical context, and requires exact equality. One-field fake owner/current/tombstone/claim probes reject atomically. |
| 5. Mutants (b)/(d) survived | **PASS** | Nested-Observation redispatch mutates independently seeded command pre-state and fails the all-family oracle. A source-order authority/session swap does not compile because the private authority-installed witness does not exist before authority installation. |

## BLOCKER findings

### 1. An exact managed SessionReport retry appends a second staged successor and poisons session replay

**Severity: BLOCKER**  
**Anchors:** `server/src/adapter_service.rs:1089-1100`; `core/src/session/logical_target.rs:210-233`; `core/src/session/runtime_evidence.rs:205-213`.

The managed ingress has no retry reconciliation for `SpawnSuccessorEvidenceStaged`. After the first exact report reserves the candidate, the same authenticated report still classifies `ClaimedSuccessor`, is appended again through generic `Storage::append`, and only then is the session projection rebuilt. The rebuild rejects the second event with `CandidateAlreadyReserved`; the RPC returns `FAILED_PRECONDITION`, but the second staged event is already durable.

A temporary extension of the real authenticated server fixture sent the exact managed report twice. The second call returned `logical target already has a reserved candidate`, and the durable log contained **two** `SpawnSuccessorEvidenceStaged` events. Thus an ordinary transport retry or another pre-promotion report creates a prefix that session restart/catch-up cannot replay. `next_spawn_promotion` independently rejects the same prefix as “duplicate staged successor.”

**Concrete fix:** give staged-successor evidence one durable idempotency identity (at minimum exact authority domain + claim operation + claimed external runtime) and reconcile retries before append under the decision gate, preferably through a dedicated atomic/idempotent storage method rather than generic append. Exact retries must return/reuse the original staged event or fold as a defined no-op; changed later reports need an explicit replace/quarantine rule. Conflicts must fail before durability. Add actual authenticated-server retry, hot/replay, completion-driver, and restart tests.

### 2. A duplicate qualifying successful Result is accepted, then permanently rejected as corrupt by the promotion producer

**Severity: BLOCKER**  
**Anchors:** `core/src/acceptance/observation.rs:164-188`; `core/src/acceptance/index.rs:238-250`; `core/src/session/runtime_evidence.rs:166-184`; `server/src/spawn_completion.rs:139-153`.

Successful spawn Results received while the command is delivered/running are accepted as deferred evidence. `CommandIndex` treats repeats idempotently through its set, but `next_spawn_promotion` returns a fence error as soon as it sees a second qualifying Result for the claim. The completion driver maps that to corrupt-log failure and stops; bootstrap encounters the same durable prefix again.

A temporary production-producer probe inserted two byte-identical qualifying successful Results after delivered/running and before the staged report. `next_spawn_promotion` returned `Err` solely because the second Result existed. The server path has no duplicate-result rejection before append, so a response-lost adapter retry can create this prefix during the normal pre-promotion window. This is not the dishonest-adapter trust assumption: repeating an already accepted fact is ordinary delivery/retry behavior and must not brick recovery.

**Concrete fix:** make repeated qualifying success evidence deterministic and idempotent. Either reject/reconcile it before append against the caught-up command projection, or have the producer retain the earliest exact qualifying Result and reject only genuinely conflicting evidence. Add same-result retry before staging, after staging/before promotion, completion-driver, and restart tests. No admitted durable retry may later be reclassified as corrupt history.

## Adversarial assessment

### Admissible SessionReport set

| Candidate | Result |
|---|---|
| Authenticated unmanaged first registration, no matching active/poisoned claim | **PASS** — `Unknown` is the explicit ordinary-registration admission. |
| Authenticated ordinary current report with increasing source cursor | **PASS** — `Current`, ordinary full-report update. |
| Authenticated unmanaged higher runtime generation, no managed lineage/claim | **PASS** — ordinary generation bump remains admitted. |
| Exact active managed claim + exact origin/attachment/generation | **PASS** — staged only. |
| Active managed claim candidate with origin omitted/changed | **PASS** — outer quarantine, never ordinary publication. |
| Current logical-target runtime after promotion, origin omitted | **PASS** — `Current`; no false managed rejection found. |
| Exact retry of the already staged managed report | **FAIL** — BLOCKER 1. |

An unrelated unmanaged report that is indistinguishable from a claimed successor at the claim's adapter/deployment/generation remains fail-closed while that claim is active; without correlation or a pre-known external identity it is not an admissible unambiguous first registration.

### Result ordering and acknowledgements

- Legitimate `delivered → running → successful Result → staged → promotion` remains green.
- Cross-adapter acknowledgement forgery is fenced by authenticated adapter/target equality. A current authenticated adapter can lie about accepting delivery, but adapter honesty is the explicitly documented external-effect trust assumption; no new core bypass was found.
- Authority, session, claim, and command projections all invoke the shared envelope/order validator; claim additionally checks referenced prefix bytes, and command requires the deferred-success fact at the Result's replay position.

### Quarantine reconstruction and races

The dedicated append derives context from the durable prefix inside the SQLite transaction; it does not trust request-owned context. The composition-root decision gate covers authenticated classification through append for session/observation ingress. A relevant concurrent prefix change therefore cannot commit a stale framed context silently; exact mismatch fails closed. No legitimate canonical absence/presence case was found to reject in the exercised current, unknown, tombstoned, identity-mismatch, claim-mismatch, stale-source, or stale-producer paths.

### Other boundaries

- No other production call site invokes exported ordinary `ingest_session_report`; its direct managed-origin probe rejects before state or storage work.
- Generic promotion/quarantine append, dedup, audited, decision-audited, and batch routes remain rejected. No new generic route was added in round 2.
- Restart/catch-up use the four-view ordered aggregate fold. The legacy repair tail stops observing after the first durable `SpawnClaim`; current managed ingress emits a claim before any report/result fact usable by that tail, so current managed history cannot enter legacy repair.
- `SpawnSuccessorEvidenceStaged` still uses generic append without idempotent identity; that is the production defect in BLOCKER 1, not a second authority publication path.

## Probe and mutation matrix

Every mutation was temporary and restored with a clean tree before the baseline suite.

| Probe / mutant | Oracle | Result |
|---|---|---|
| Valid managed `spawn_origin` passed directly to ordinary core ingestion | `ordinary_ingress_rejects_every_spawn_origin_before_append` | **REJECTED / PASS** — no write. |
| Active managed server report with `spawn_origin` omitted | real managed server fixture | **QUARANTINED / PASS** — no registration/bump publication. |
| Two authenticated stale reports over actual server route, hot + replay | authenticated source-order fixture | **OUTER-ONLY / PASS** — 2 quarantine sources, 2 linked audits, 0 session mutations. |
| Result at LSN 6, delivered 7, running 8, staged 9 | dedicated promotion append | **REJECTED / PASS**. |
| Forged logical owner/current/tombstone/claim context | dedicated quarantine append | **REJECTED / PASS** — no writes. |
| (b) Recursively dispatch nested quarantine Observation in `CommandIndex` | all-family outer-only test | **KILLED** — command deferred-success state changed and equality oracle failed. |
| (d) Swap session publication before authority installation | aggregate order test/build | **KILLED structurally** — private authority-installed witness is unavailable; mutant does not compile. |
| Remove omitted-origin active-claim fence before ordinary classification | real managed server fixture | **KILLED** — candidate published as `SessionState` instead of quarantine. |
| Weaken context equality by omitting `classified_target` | forged-context test | **KILLED** — fake logical owner committed and oracle failed. |
| Coordinated removal of shared/envelope/claim/command Result-order gates | result-before-lifecycle test | **KILLED** — forbidden promotion committed and oracle failed. |
| Exact authenticated managed SessionReport retry before promotion | temporary real server probe | **ADMITTED THEN REPLAY-POISONED / BLOCKER 1** — two staged sources durable. |
| Two exact qualifying successful Results before promotion | temporary production-producer probe | **PREFIX REJECTED AS CORRUPT / BLOCKER 2**. |

## Full verification suite

All commands ran on the restored clean tree:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted, 22 implementation checks, 38 mutation witnesses killed; generated bindings clean.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 9/9.
4. `cd pi-adapter && npm test` — **PASS**, 29/29 including the real core/adapter restart e2e.

The baseline suites do not exercise exact retries of staged successor evidence or duplicate deferred spawn success before promotion; their green result does not override the two adversarial probes.

## Final recommendation

**Return `research-handoff-spawn-runtime-evidence-promotion-contract` to `implementing`.** Preserve the round-2 fixes. Add bounded idempotent reconciliation for repeated staged successor and successful Result evidence so every admitted durable prefix remains replayable, then rerun the thorough convergence lane. Do not advance Leaf 6 to done yet.
