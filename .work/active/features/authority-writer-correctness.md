---
id: authority-writer-correctness
kind: feature
stage: done
tags: [security, foundation]
parent: null
depends_on: [authority-descendant-grant-completion]
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
---

# Authority writer correctness (pre-append conflict check + durable idempotency)

## Brief
Close the authority-ingest durability hazard split out of `authority-provenance-hardening`. Absorbs:

- `backlog-authority-ingest-pre-append-conflict-check` — **OPEN** (highest durability hazard in the set): authority ingest appends *before* the conflict check (which runs in `observe`), so a conflicting re-ingest poisons the durable log and an identical retry appends a second event. `current_grant` only chooses audit kind (`ingest.rs:39-48`); append-before-observe (`ingest.rs:179-187`); descendant grants share the non-dedup path (`ingest.rs:75-78`). *Src:* authority review Phase 1+2.

## Direction
Pre-append check-and-append: read the current projected grant → identical content returns the existing event id (no-op) → different content rejects before append (`CorruptLog`) → only append if absent. Use the deterministic `descendant_grant_id` as the dedup key. This needs a storage-level atomic check-and-append (like `append_dedup`) or a serialized authority writer — `Storage::append` is not atomic with the projection read. The existing "warm-after-write" test does NOT retry the writer (false confidence); add a writer-retry regression. Resolve together with `authority-descendant-grant-completion` (where the live writer-coordination layer lands).

## Foundation references
- `docs/PROTOCOL.md` — durable log integrity; authority lifecycle
- Code: `core/src/authority/ingest.rs`, `core/src/storage/` (append/append_dedup)

## Design decisions
- **Keep the 2026-08-09 review-vetted authority model fixed.** This feature changes only creation-writer correctness for the existing normal and same-actor descendant grant records. It does not reopen descendant allowed kinds, provenance, completion order, delegation, or two-lever revocation.
- **Make grant identity enforcement a storage transaction, not a projection/gate convention.** `CoreDecisionGate` remains useful production coordination, but correctness must hold for two independent projections and for callers that do not share that process-local gate. A dedicated storage primitive atomically claims `(authority_domain_id, GrantId)`, appends the source and its creation audit only when absent, and returns the original source `EventId` when exact content already exists.
- **Use one identity namespace for normal and descendant grants.** The key is the complete non-empty `GrantId` inside its authority domain; `descendant_grant_id` supplies that key for auto-issued grants. Record kind is part of compared content, not part of the key, so a normal `Grant` and `DescendantGrant` cannot coexist under one id.
- **Define identical content as the exact canonical stored envelope.** Compare the Protobuf-encoded `StoredEventPayload` after the authority writer has normalized the requested domain and passed canonical registry validation. Reordered repeated fields or any changed identity, authority, target, kinds, provenance, lifecycle, timestamp, or audit link are conflicts rather than silent normalization.
- **An exact retry is a total durable no-op.** It returns the existing source `EventId` and writes neither a second grant event nor a misleading `GrantCreated`/`GrantChanged` audit. The first creation still commits its source, identity claim, and `GrantCreated` audit in one transaction. If retry-attempt auditing is later required, it needs a distinct truthful attempt kind rather than another creation audit.
- **Treat same-id changed content as corrupt authority history.** The storage port reports the existing identity conflict without appending; `ingest_grant` and `ingest_descendant_grant` map it to `AuthorityError::CorruptLog`. Grant mutation continues through explicit revocation events; a future mutable-grant operation would need its own event contract rather than reusing creation.
- **Preserve pre-v5 durable grants through a checked migration.** The SQLite migration backfills a grant-identity index from the authoritative log, points exact historical duplicates at the earliest source event, and fails startup on conflicting same-id history. Every open validates the index against grant/descendant source events; migration/index maintenance consumes no event LSN and never replaces the log as source of content truth.
- **Formal status stays unchanged.** This adds implementation evidence for durable grant-writer idempotency and conflict rejection. It does not promote `authority.qnt`, `RetryAfterTerminalReturnsExisting`, `IdempotentLogReplay`, or any conformance vector.
- **Exploration posture:** direct-read only. The feature is bounded to authority ingest, storage port/decorator/SQLite writer, their focused tests, and the completed spawn driver. The delegated endpoint prohibits nested subagents and peer mechanisms.

## UI alignment
No UI surface or presentation contract changes. The returned `EventId`, grant projection, and audit projection retain their existing shapes; mockups are skipped.

## Architectural choice

### Option A — projection check under `CoreDecisionGate`
Catch the projection up, compare the current grant, and append while holding the production gate. This is small and builds on the completed live writer coordination, but it makes a server composition detail the core writer's safety proof. A second projection, a focused core caller, a future composition root, or a post-commit fold failure can bypass or stale that assumption.

### Option B — reuse command `append_dedup`
Convert `GrantId` into the existing idempotency-key/target pair and reuse `append_dedup_audited`. This supplies an atomic transaction, but it conflates operator-command retry keys with immutable authority identity, appends one audit per duplicate submission, and leaves historical grant events outside the dedup registry. Those semantics do not match an exact grant-creation no-op.

### Option C — dedicated atomic grant-identity append (chosen)
Add a narrow storage-port operation for immutable grant creation. The backend atomically checks the domain-qualified `GrantId`, compares the referenced authoritative source envelope, and either commits source + identity claim + one creation audit, returns the existing source id with no writes, or rejects different content. SQLite uses a validated/backfilled identity index referencing the event log; the port remains backend-neutral and never asks storage to synthesize grant payloads.

**Choice:** Option C. It is the only option that makes pre-append conflict rejection and retry identity true at the persistence boundary while preserving existing audit coupling. The shared gate remains defense-in-depth and ordering coordination for the staged spawn workflow, not the grant writer's correctness premise.

## Trickiest unit first
The riskiest unit is the **atomic identity/source/audit transaction plus legacy-index validation**. It must prevent a concurrent same-id append, return the earliest existing source id after an ambiguous commit, distinguish exact content from a conflict across normal/descendant kinds, keep the creation audit atomic only on the first append, and open existing v4 databases without making old grants invisible to the new identity check.

## Implementation units

### Unit 1: Add the atomic grant-identity storage contract and SQLite implementation
**Story:** `authority-writer-correctness-atomic-storage`

**Files:**
- `core/src/storage/port.rs`
- `core/src/storage/mod.rs`
- `core/src/storage/audited.rs`
- `core/src/storage/rusqlite.rs`
- `core/tests/rusqlite_storage.rs`
- `core/tests/audit_records.rs`
- focused `Storage` test doubles only where the new fail-closed method is consumed

Add a validated grant key and an outcome that distinguishes first commit from exact existing identity:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrantIdentityKey(String);

impl GrantIdentityKey {
    pub fn new(value: String) -> Option<Self>;
    pub fn as_str(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantAppendOutcome {
    Appended(AuditedAppend),
    Existing(EventId),
}

pub trait Storage: Send + Sync {
    fn append_grant_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        identity: &GrantIdentityKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> impl Future<Output = Result<GrantAppendOutcome, StorageError>> + Send;
    // existing methods unchanged
}
```

Add a backend-neutral conflict error carrying only safe identity/LSN context:

```rust
StorageError::GrantIdentityConflict {
    grant_id: String,
    existing_lsn: u64,
}
```

**Implementation notes:**
- The trait default is `UnsupportedOperation`; it must never fall back to `read_after + append`. Production `AuditedStorage<S>` delegates to an inner backend that implements the atomic primitive.
- Advance SQLite to schema v5 with `grant_identities(authority_domain_id, grant_id, source_lsn)`, primary-keyed by `(authority_domain_id, grant_id)`, uniquely referencing the source event. The table is an atomic uniqueness/index constraint; source bytes remain in `events`.
- On v4→v5 migration, scan `GRANT` and `DESCENDANT_GRANT` source envelopes in LSN order, extract the generated `GrantId`, and register the earliest exact source. Later byte-identical historical duplicates are accepted as legacy no-ops; another kind or different envelope under the same id fails migration as corrupt history. Do not append migration audits or consume LSNs.
- On every open, validate that each identity row references the correct same-domain grant source, every grant source is covered by the expected earliest identity row, and all later same-id sources are exact duplicates. Missing/extra/substituted/conflicting rows fail closed.
- In one SQLite writer transaction, decode only the candidate's identity boundary: require kind `Grant` or `DescendantGrant`, non-empty embedded `GrantId`, exact embedded/requested authority domain, and equality between the embedded id and `GrantIdentityKey`. Require the truthful creation audit pair (`GrantCreated/grant_created` or `GrantCreated/descendant_grant_created`), then encode the candidate and query the identity row joined to its source event. Equal bytes → `Existing(original EventId)` and no commit; unequal bytes → `GrantIdentityConflict` and no commit; absent → append source, insert identity row, replace `audit.source_event_id` with the committed source, append the audit, commit, return `Appended`. Storage validates identity/audit framing but never constructs or interprets grant policy.
- The database constraint and transaction are the race arbiter even though the current actor serializes writes. The first committed candidate for an absent id wins; a concurrent different candidate sees the winner and conflicts before its own append.
- Fail closed in the production decorator if generic `append`, `append_audited`, batch, or `append_decision*` is asked to write a `Grant`/`DescendantGrant`; those routes must not bypass identity enforcement. Raw `RusqliteStorage::append` remains a trusted corruption/legacy-fixture seam, not an authority-ingest API.
- Remove `Grant`/`DescendantGrant` audit inference from the generic decorator path once the domain writer supplies the exact creation draft to `append_grant_audited`; retain no parallel audit path.

**Acceptance criteria:**
- [ ] First normal or descendant identity with matching embedded/domain/key identity commits exactly one source plus one truthful linked creation audit and returns the source `EventId`; mismatched key/domain/audit framing writes nothing.
- [ ] Exact retry returns that same source `EventId` and leaves the complete event/audit prefix byte-for-byte unchanged.
- [ ] Same-domain/same-id different content—including normal-vs-descendant kind—returns `GrantIdentityConflict` before another event or audit is appended.
- [ ] Concurrent exact attempts converge on one source/audit and one shared source id; concurrent conflicting attempts leave one valid committed winner and one conflict.
- [ ] v4 migration/open makes historical grants visible to the identity check, preserves earliest ids for exact duplicates, and rejects conflicting or inconsistent index history.
- [ ] The production audited wrapper has no generic authority-creation bypass.

### Unit 2: Route both authority creation writers through the atomic contract
**Story:** `authority-writer-correctness-ingest-contract`

**Files:**
- `core/src/authority/ingest.rs`
- `core/src/authority/projection.rs` only for comments or simplification made possible by removing creation-time lookup
- `core/src/authority/mod.rs` only for exports required by the storage/writer contract
- `core/tests/authority_ingest.rs`
- `core/tests/authority_replay.rs`
- `core/tests/authority_proptest.rs`

Keep both public writer signatures stable:

```rust
pub async fn ingest_grant<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    grant: Grant,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection;

pub async fn ingest_descendant_grant<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    grant: DescendantGrant,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection;
```

Add one private shared append/read-back helper shaped around `GrantIdentityKey` and `GrantAppendOutcome`; do not create a second public authority service abstraction.

**Implementation notes:**
- Preserve all existing boundary validation before storage: requested/message domain, non-empty grant id, exact target shape, normal registry preflight, canonical descendant kinds, deterministic descendant id, exact prior same-domain completion audit/source linkage, and lifecycle/provenance validation.
- Build the full normalized `StoredEventPayload` before the atomic call. The identity key comes directly from the validated `GrantId`; descendant creation therefore uses the existing deterministic `descendant_grant_id` without another id scheme.
- Normal first creation supplies `AuditEventKind::GrantCreated` / `reason_code = "grant_created"`; descendant first creation supplies `GrantCreated` / `"descendant_grant_created"`. Delete the projection-driven `GrantChanged` selection. Exact retry supplies no new audit because storage returns `Existing` without writing.
- Map `StorageError::GrantIdentityConflict` to `AuthorityError::CorruptLog` with domain/grant/existing-LSN context. Other storage failures remain typed storage failures.
- For both `Appended` and `Existing`, read back the exact immutable source `EventId`, require its envelope to equal the candidate, and fold that committed record into the caller projection before returning. Never synthesize a warm event from the request. A post-commit fold/read failure remains recoverable: a retry reaches `Existing`, reads the same source, and rewarms a fresh projection.
- Descendant read-back continues to fold the exact linked completion source/audit prerequisites before the grant, so the canonical `AuthorityRegistry` replay validator—not the storage index—owns provenance semantics.
- Leave revocation lookup/writer behavior unchanged. `GrantLookup` remains needed there even though creation no longer consults `current_grant`.

**Acceptance criteria:**
- [ ] Normal and descendant exact retries return the original source `EventId`, append no second source/audit, and leave a fresh or already-warm projection equivalent to replay.
- [ ] Normal and descendant different-content retries return `AuthorityError::CorruptLog` while the durable prefix and projection remain at the first valid record.
- [ ] A normal grant cannot collide with a descendant grant under the same `GrantId` without a pre-append corruption error.
- [ ] Malformed descendant audit/provenance/kind/lifecycle candidates still fail before the identity table or log changes.
- [ ] `GrantChanged` is no longer emitted by immutable creation retry; revocation semantics and exact replay stay unchanged.

### Unit 3: Prove ambiguous-response, concurrency, audit, and live-driver retry behavior
**Story:** `authority-writer-correctness-retry-evidence`

**Files:**
- `core/tests/authority_ingest.rs`
- `core/tests/rusqlite_storage.rs`
- `core/tests/audit_records.rs`
- `server/tests/spawn_completion.rs`
- minimal storage test wrappers in those test modules only

**Implementation notes:**
- Replace the false-confidence warm/redelivery-only coverage with writer-level calls. Keep replay-redelivery evidence where it protects projection behavior, but do not present it as writer idempotency.
- Add a storage wrapper that lets the real atomic append commit once and then returns a synthetic retryable write failure to simulate lost acknowledgement. Retry through a fresh projection and assert the original source id and unchanged prefix for both normal and descendant grants.
- Race identical and conflicting creations through two independent `AuthorityRegistry` instances sharing one real SQLite store. Use barriers rather than timing sleeps; assert the storage transaction, not the projections or `CoreDecisionGate`, decides the result.
- Query the durable audit index: first creation has exactly one truthful linked `GrantCreated`; exact retry and conflict add none.
- Extend the completed `SpawnCompletionDriver` integration test with a descendant-append committed/response-lost prefix. A fresh bootstrap must fold the existing descendant, append only the final completion work still missing, and never emit a second descendant source or grant-created audit.
- Keep model/vector metadata unchanged and describe this as mutation-sensitive implementation evidence only.

**Acceptance criteria:**
- [ ] A committed-but-reported-failed normal or descendant creation retries to the exact existing `EventId` after a fresh projection/restart.
- [ ] Barrier-controlled exact and conflicting races cannot append two grant identities or poison replay.
- [ ] Audit queries show one creation audit total per grant identity across success, retry, conflict, and driver repair.
- [ ] Spawn completion repairs the ambiguous descendant-write response without duplicating the grant and still exposes completion last.
- [ ] A mutation that restores projection-read + plain append, adds a duplicate retry audit, compares only id/kind, or gives normal/descendant separate identity namespaces fails the focused evidence.

## Implementation order
1. `authority-writer-correctness-atomic-storage` — establish the backend-neutral atomic contract, v5 identity index, migration, and audit transaction.
2. `authority-writer-correctness-ingest-contract` — move both creation writers to the contract and map exact/conflict outcomes.
3. `authority-writer-correctness-retry-evidence` — exercise lost replies, independent-projection races, audit cardinality, and the live completion driver.
4. Review the parent as one integrated security/durability feature with effective weight `thorough`.

## Child dependency chain

```text
authority-writer-correctness-atomic-storage
  → authority-writer-correctness-ingest-contract
    → authority-writer-correctness-retry-evidence
```

All three ids were checked with `.work/bin/work-view --blocking <id>` before the edges were written; no reverse edge or cycle exists. The parent remains downstream of `authority-descendant-grant-completion`, whose verified implementation is already at review and therefore satisfies implementation dependency dispatch.

## Simplification
- Remove projection-based `GrantCreated`/`GrantChanged` selection from immutable creation. One storage outcome now owns absent/exact/conflict classification.
- Consolidate normal and descendant identity handling behind one storage primitive and one private ingest helper; do not add a spawn-only table/API or duplicate descendant writer.
- Make the original event log the only content truth. The SQLite identity table stores only key→source-LSN uniqueness and is rebuilt/validated from log records during migration/open.
- Reject generic production decorator routes for grant creation rather than retaining compatibility bypasses.
- Retain `GrantLookup` only for actual read-dependent operations such as revocation; do not delete it merely because creation stops using it.
- No production behavior, foundation registry, Protobuf contract, model, vector, or UI change is required.

## Testing
- **Storage contract tests** protect atomic absent/exact/conflict classification, source/audit coupling, shared normal/descendant namespace, and v4 migration/open validation.
- **Authority interface tests** protect the stable `ingest_* -> EventId` contract, pre-append validation, conflict-to-`CorruptLog` mapping, and projection equivalence after append or exact retry.
- **Ambiguous-response regressions** protect the actual durability hazard: commit succeeds but the caller cannot know it, then a fresh retry returns the original identity without another write.
- **Barrier-controlled races** prove correctness does not derive from one warm projection or the server gate.
- **Spawn-driver integration** protects composition with the completed audit → descendant → terminal staged workflow.
- **No new formal claim:** run existing model/vector checks as regression metadata only; do not promote their tier.

## Verification commands

```bash
cargo test -p patchbay-core --test rusqlite_storage --test audit_records --test authority_ingest --test authority_replay --test authority_proptest
cargo test -p patchbay-core-server --test spawn_completion
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
npm --prefix contracts/ts run check:models
npm --prefix contracts/ts run check:vectors
```

If repository-wide formatting still reports pre-existing untouched drift, verify every touched Rust file with the repository's bounded rustfmt convention and record the discrepancy rather than reformatting unrelated files.

## Risks
- **The identity index could become a second source of truth.** It must store only the key and referenced source LSN, compare against the joined immutable event, and validate/backfill from the log on open. Any disagreement is corruption, not a reason to trust the table over the log.
- **Legacy logs may contain duplicates from the old bug.** Exact duplicates are preserved and resolve to the earliest source id; conflicting duplicates already represent poisoned authority history and must fail startup for operator recovery rather than be guessed away.
- **Byte-strict content equality is intentionally unforgiving.** Callers must retry the same normalized generated record. Treating reordered repeated fields or refreshed timestamps as identical would require a separately specified canonical semantic projection and risks hiding a real authority change.
- **Audit semantics can regress during dedup reuse.** Command dedup deliberately records every submission audit; grant identity retry deliberately records no new creation audit. Reusing the wrong primitive would create false grant-change history.
- **A raw storage fixture can still inject authority corruption.** Production `AuditedStorage` rejects generic creation routes, while bare raw append remains necessary for migration/replay corruption tests. The safety claim is the public authority writers plus production composition, not arbitrary trusted-database mutation.
- **Custom storage fakes may initially return `UnsupportedOperation`.** This is fail-closed by design. Update only fakes that genuinely exercise authority creation; never add a non-atomic trait fallback to make tests compile.
- **The shared gate must not be mistaken for the proof.** The completion driver continues to hold it for staged exposure/order, but exact identity/conflict safety is enforced by the storage transaction and remains valid with independent projections.

## Extension pressure classification
- **Committed v0.1.0:** immutable domain-qualified grant identity; one namespace for normal/descendant records; exact retry returns the earliest existing source `EventId`; different same-id content is rejected before append; first source and creation audit commit atomically.
- **Reserved seams:** other immutable event-identity namespaces, additional storage backends implementing the same port, and an explicit future mutable-grant/change event if product pressure requires it.
- **Explicitly rejected for this feature:** projection-read + plain append as the safety mechanism, a server-gate-only correctness claim, separate normal/descendant identity namespaces, duplicate creation audits on retry, or silent same-id grant replacement.
- The design retains `(authority_domain_id, GrantId)` demarcation for future authority domains and does not touch the multi-human, desktop, agent-mesh, or skin seams.

## Advisory review record
- **Risk:** high — this is an authority-integrity, durability, crash ambiguity, schema migration, and audit-cardinality contract.
- **Design-time advisory:** not dispatched because the delegated endpoint explicitly prohibits nested subagents and peer mechanisms. Per the non-blocking design policy, direct foundation/code evidence and the prior 2026-08-09 review-vetted direction are used; this degradation does not block design.
- **Caller:** active autopilot caller `drain`; routine decisions were resolved with judgment and logged above. No contradictory state or semantic hard halt was found.
- **Execution capability contract:** Sol at xhigh reasoning for security/durability implementation, as explicitly selected by the caller.
- **Effective review weight:** `thorough` (explicit caller selection, unchanged). Run iterative fresh-context review until no receiver-confirmed material current-cycle blockers remain. Reviewer findings are proposals; the receiving orchestrator verifies and adjudicates each against repository evidence, fixes material blockers, and records/rejects lower-risk proposals without letting labels substitute for judgment.

## Implementation summary
- Execution used the explicitly selected Sol/xhigh security-and-durability posture in one direct feature-owning context; no nested agent, peer mechanism, backlog work, unrelated item, push, model/vector promotion, or generated-contract edit was used.
- `authority-writer-correctness-atomic-storage` (`245274c`) added the fail-closed storage contract, shared normal/descendant identity namespace, SQLite schema v5 key→earliest-source index, checked v4 backfill/every-open preflight, atomic source+identity+creation-audit transaction, exact retry no-op, pre-append content conflict, and production generic-route rejection.
- `authority-writer-correctness-ingest-contract` (`000b9dd`) routed both stable public authority writers through that primitive, removed creation-time projection lookup/`GrantChanged`, mapped identity conflict to corrupt authority history, read the exact returned source back before projection fold, and preserved the hardened descendant provenance/lifecycle validation.
- `authority-writer-correctness-retry-evidence` (`43ed3ee`) added committed-response-loss recovery for normal/descendant writers, barrier races through independent projections, audit-cardinality evidence, and live-driver repair from an ambiguous descendant prefix. The old redelivery test is now explicitly projection-only evidence.
- All three child stories advanced directly from `implementing` to `done` after their own verification and commit. Integrated acceptance now derives from the storage transaction and checked log/index relationship, never from `CoreDecisionGate`; the gate remains only the completed driver's exposure/order coordination.

## Integrated verification
- `cargo test -p patchbay-core --test rusqlite_storage --test audit_records --test authority_ingest --test authority_replay --test authority_proptest` — passed (10 audit, 14 authority-ingest, 14 authority-property, 6 replay, and 29 SQLite-storage tests).
- `cargo test -p patchbay-core-server --test spawn_completion` — passed (6 tests, including the committed-descendant/lost-ack repair).
- `cargo test --workspace` — passed after the final code/test changes.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `npm --prefix contracts/ts run check:models` — passed; formal status unchanged.
- `npm --prefix contracts/ts run check:vectors` — passed after installing declared local package dependencies and building the existing contract/operator packages; 21 implementation checks and 37 mutation witnesses passed, with no metadata regeneration.
- `git diff --check` — passed; worktree clean before the parent transition.
- `cargo fmt --all -- --check` — still reports the pre-existing repository formatting baseline beginning in untouched `core/src/acceptance/elicitation.rs`. A transient final rerun also hit a full shared target filesystem; removing only Cargo incremental/cache artifacts restored space and the focused suite passed. No unrelated source formatting or content was changed.

## Implementation reconciliation and discrepancies
- The hardened descendant-completion fold returned no issuance once a descendant source existed, which would reject an exact writer retry before storage. Its scoped helper now reconstructs the already-validated canonical issuance from durable prior facts for exact retry while retaining terminal/lifecycle suppression.
- The new every-open identity preflight correctly exposed stale generic property fixtures that generated arbitrary bytes under grant discriminators and a restart fixture that raw-appended grants. Those fixtures now use opaque non-grant kinds or `ingest_grant`; raw SQLite append remains available only for intentional corruption/migration seams.
- The new storage error variant required one exhaustive server status mapping. No public writer signature, protocol contract, generated artifact, formal claim, vector classification, revocation behavior, foundation assertion, or UI surface changed.
- No material design flaw or blocker was found.

## Review handoff
The integrated feature is at `review` by explicit stop boundary. Effective review weight remains `thorough`; fresh review should focus on transaction/index corruption handling, ambiguous-commit identity preservation, descendant retry provenance, generic bypass closure, and audit idempotency. Findings remain proposals for receiver adjudication.

## Review (2026-08-10) — pass 1 fix verification

**Verdict**: Request changes resolved; keep at `review` for the next thorough convergence pass.

**Effective weight**: `thorough` (explicit caller selection)

**Independent passes completed**: 1

**Closure state**: pass-1 receiver-accepted blockers are fixed and verified; a clean later pass is still required before `done`.

**Blockers accepted and fixed**:
1. Migrated byte-identical duplicate descendant sources now classify as legacy redelivery in `SpawnDescendantTail`: source-envelope bytes, not the later LSN-bearing fact, decide equality; the earliest fact remains retained and differing bytes fail closed. A file-backed complete v4 prefix now migrates, bootstraps quiescently, retries to the earliest `EventId`, and writes no event or audit.
2. Exact normal-grant retry now validates and folds the complete durable authority-domain prefix before returning. A fresh projection after a later revocation equals canonical replay, retains the earliest source id, and leaves source/audit cardinality unchanged.
3. The unused production `AuditedStorage::inner` raw-backend accessor was removed. The dedicated atomic grant writer remains the only production wrapper path for grant creation.
4. Schema preflight now validates the audit table against its declared version: v2 does not require the v3 `grant_id` column. A preservation fixture proves valid v2 events and audit rows migrate through v3, v4, and v5 without allocating an LSN or losing indexed audit content.

**Verification**:
- Focused authority/storage/server suite passed: 11 audit, 15 authority-ingest, 14 authority-property, 6 authority-replay, 11 spawn-tail, 29 SQLite-storage, and 7 spawn-completion tests.
- `cargo test --workspace` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `npm --prefix contracts/ts run check:models` passed.
- `npm --prefix contracts/ts run check:vectors` passed with 21 implementation checks and 37 killed mutation witnesses after restoring lockfile-declared local TypeScript dependencies/build artifacts.
- `git diff --check` passed.

**Disposition notes**: no lower-risk finding was parked, no unrelated item or foundation document was changed, and no nested agent, peer mechanism, or push was used. Per the caller's explicit boundary, this pass records fix verification only and retains `stage: review`; no follow-on independent pass is claimed in this commit.

## Review closure — pass 2

Fresh-context pass 2 approved with no material blocker or follow-up proposal. It rechecked legacy duplicate migration/bootstrap, revoked fresh-projection retry equality, v2→v5 migration, raw-bypass closure, atomic source/index/audit persistence, races, lost acknowledgements, and spawn integration. Effective weight: `thorough` (explicit operator). Verdict: approved.
