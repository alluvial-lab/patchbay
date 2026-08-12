---
id: snapshot-core-generation-semantics
kind: feature
stage: done
tags: [foundation, protocol]
parent: null
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
---

# Snapshot / core-generation semantics

## Brief
Define core generation's role in snapshot/recovery, split out of `storage-recovery-correctness`. Absorbs:

- `backlog-core-generation-persistence` — **OPEN**: session + resource snapshot materialization still sets `core_generation: None` (`server/src/state.rs:304-310,383-389`); foundation calls the field reserved (`PROTOCOL.md:455-458`, `GLOSSARY.md:27-39`). *Src:* docs-audit 2026-07-27.

## The contradiction this must resolve first (load-bearing)
Foundation defines core generation as core-assigned **on restart** and says snapshots from another generation are rejected (`GLOSSARY.md:27-39`, `VERIFICATION.md:218-225`). But a recovery checkpoint is necessarily written by the **previous** incarnation. If a new core increments generation and rejects the previous generation's snapshot, **every restart discards the checkpoint and replays the full log** — which makes `recovery-checkpoint-writer` ineffective. This contradiction blocks both storage children until resolved.

## Direction
Decide explicitly: either (a) core generation is a **durable storage-continuity epoch** (not a per-process restart counter), so a restart continues the same epoch and accepts its own prior checkpoint; or (b) recovery checkpoints require a separately specified compatibility rule that survives an incarnation change. Then implement persistence + cross-incarnation validation for the wire-present field once restart ambiguity makes it load-bearing. Do not ship `recovery-checkpoint-writer` against an unresolved answer.

## Foundation references
- `docs/PROTOCOL.md` (`:455-458`), `docs/GLOSSARY.md` (`:27-39`), `docs/VERIFICATION.md` (`:218-225`)
- Code: `server/src/state.rs`

## Design decisions

- **`core_generation` is a durable storage-continuity epoch, not a process-start counter.** The core assigns one nonzero opaque `uint64` when an authority-domain storage lineage is first opened, persists it, and reuses it across ordinary process restarts. A checkpoint written by the previous process is therefore compatible after restart when its domain, epoch, and LSN anchor match.
- **Equality, not ordering, is the compatibility rule.** A durable session checkpoint must first carry the supported typed/versioned session envelope, then its embedded authority domain and core generation must exactly equal the current durable values and its embedded snapshot LSN must exactly equal the storage row's `(authority_domain_id, LSN)` anchor. Snapshot freshness remains a separate LSN comparison. Legacy undiscriminated bytes, wrong projection type/version/domain/generation, undecodable payload, or an LSN mismatch makes the derived checkpoint unusable; the log remains authoritative and the caller repairs from current materialization or full replay.
- **Assignment is random and persistence is atomic.** The server composition boundary supplies an OS-random nonzero 63-bit candidate (bounded to SQLite's positive integer range); the storage port atomically inserts the first candidate or returns the existing value. Concurrent initializers converge on the stored winner. The generation is an equality fence, not a secret or authorization token.
- **History discontinuity is a reserved rollover seam.** v0.1.0 exposes no generation-rotation API. A future destructive restore, fork into a divergent history, multi-core promotion, or authoritative-store replacement must explicitly roll the durable epoch before serving snapshots/cursors. An ordinary backup/restore that continues the same history may retain it. Process-incarnation fencing for HA/zero-downtime work is a separate future concept and must not overload this field.
- **Existing generation-less snapshots are disposable derived data.** They are rejected and rebuilt from the durable log; no dual-read compatibility path or payload migration is introduced. Durable events and other substantial state are preserved.
- **Do not pre-decide the whole-core checkpoint namespace.** This feature retains the current session-only durable snapshot slot and on-demand resource materialization, but the bytes in that slot now require a private typed/versioned session envelope so another projection cannot decode by structural Protobuf overlap. `recovery-checkpoint-writer` still owns the later choice between a typed composite whole-core checkpoint and per-projection namespaces; either can consume the same durable epoch and anchor rule.
- **Assurance stays honestly graded.** Implementation and the existing promoted snapshot-reconciliation example will exercise persistence, carriage, restart continuity, and mismatch fallback. `SnapshotCrossDomainRejected` remains stated-normative until its formal property is genuinely promoted; this feature does not relabel implementation evidence as checked-model or checked-normative.
- **Execution posture and review policy.** Direct-read only: the storage/state/snapshot surface was enumerable, and the delegated endpoint forbids nested agents. Effective `review_weight` is `thorough` (source: explicit operator selection) for implementation, feature review, and final completion review.

## Codebase mapping

Direct reading covered the storage port and SQLite writer/migrations, the generic recovery helper, the server aggregate projection and `LoadSnapshot` boundary, session/resource snapshot Protobuf fields, restart/storage tests, the promoted snapshot-reconciliation vector/runner, and `specs/seed/snapshot_recovery.qnt`. The current contradiction is real: both materializers emit `core_generation: None`, while the draft model already preserves `CoreGeneration` through `crash`/`restart` but prose still defines it as restart-assigned. No exploratory fan-out ran because the caller prohibited nested delegation and no unmapped surface remained.

## UI fallback

No UI surface. This changes durable snapshot identity and restart validation only; no mockup is required.

## Architectural choice

### Options considered

1. **Chosen — one durable storage-continuity epoch per authority domain.** Persist an opaque nonzero generation beside the authority-domain log, keep it stable across process restarts, and require exact equality at snapshot validation. This makes checkpoints useful after crash/restart with one rule and leaves a named rollover seam for real history discontinuities.
2. **Increment on every process restart and admit compatible predecessor checkpoints.** This retains the glossary's current process-incarnation meaning, but requires an exception such as “accept generation N-1.” It is fragile across repeated failed starts or a restart that never writes a new checkpoint, and generalizing it to “any prior generation with a valid LSN” makes the generation fence redundant.
3. **Add separate continuity-epoch and process-incarnation fields now.** This is taxonomically clean for future HA, but v0.1.0 has one authoritative process and no process-incarnation consumer. It would add wire/schema/model concepts solely for a reserved capability; that work belongs to the future multi-core fencing ceremony.

Option 1 is the least-foreclosing sound choice: it solves the current recovery contradiction without a special compatibility window, does not pretend v0.1.0 has HA fencing, and preserves explicit promotion paths for both epoch rollover and process incarnation.

## Trickiest unit first

The hardest unit is atomic epoch initialization. A fresh process must propose a value without overwriting an existing lineage, two constructors must converge if they race, reopening the same database must return the identical value, and a second authority domain must have an independently keyed epoch. This must stay behind a narrow durability port rather than letting projection logic query SQLite or generate its own unstored value.

## Implementation Units

### Unit 1: Durable authority-domain core generation

**Files**: `core/src/storage/port.rs`, `core/src/storage/mod.rs`, `core/src/storage/rusqlite.rs`, `core/src/storage/audited.rs`, `server/src/identity.rs`, `server/src/state.rs`, `server/src/service.rs`, `server/src/admin_service.rs`

**Story**: `snapshot-core-generation-semantics-durable-epoch`

```rust
// core/src/storage/port.rs
pub trait CoreGenerationStore: Send + Sync {
    fn load_or_create_core_generation(
        &self,
        authority_domain_id: &AuthorityDomainId,
        candidate: Generation,
    ) -> impl std::future::Future<Output = Result<Generation, StorageError>> + Send;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    // existing variants...
    #[error("core generation must be in 1..=i64::MAX, got {0}")]
    InvalidCoreGeneration(u64),
}

// server/src/identity.rs
pub fn random_core_generation() -> Generation;

// server/src/state.rs
#[derive(Clone)]
pub struct ProjectionState {
    // existing projections...
    core_generation: Generation,
}

impl ProjectionState {
    pub fn core_generation(&self) -> &Generation;
}
```

**Implementation notes**:

- Keep `Storage` focused on event/snapshot operations and add the narrower `CoreGenerationStore` port. `ProjectionState`/`ControlServiceImpl` require both traits; adapter-only storage consumers do not acquire a generation dependency.
- `random_core_generation` uses the existing `OsRng`, retries zero, and masks to `1..=i64::MAX`. Randomness stays at the server composition/infrastructure boundary; domain code receives only a typed candidate.
- `RusqliteStorage` implements the port through its single writer actor. Schema migration 4 creates `authority_domain_metadata(authority_domain_id TEXT PRIMARY KEY, core_generation INTEGER NOT NULL CHECK(core_generation > 0))`; initialization performs insert-if-absent and select in one transaction. Validate the candidate before opening the transaction and validate the stored positive value on read.
- `AuditedStorage<S>` delegates `CoreGenerationStore` without creating an audit event: initialization metadata is not a log state transition and cannot consume an LSN. The durable log remains the sole ordering authority.
- `ProjectionState::rebuild*` loads/creates the generation before folding snapshots or materializing views and retains it unchanged for the state's lifetime. Constructor bounds in `service.rs`/`admin_service.rs` follow the narrow port. Only custom server storage wrappers that construct a `ProjectionState` need explicit delegation.
- Migration preflight follows the existing fail-before-mutation rule: a malformed/future metadata table or unsupported schema version is rejected without stamping a new version.

**Acceptance criteria**:

- [ ] The first initialization for a non-empty authority domain persists and returns the supplied nonzero candidate; repeated, concurrent, and post-reopen calls return the stored winner without changing it.
- [ ] Different authority-domain ids have independent rows; zero or values above `i64::MAX` fail before mutation.
- [ ] The v3→v4 migration preserves events, snapshots, idempotency keys, and audit rows and creates no event/LSN.
- [ ] Production `AuditedStorage<RusqliteStorage>` and every `ProjectionState`/control-service constructor compile through the narrow port; adapter-only test ports remain unaffected.

### Unit 2: Snapshot carriage and one compatibility boundary

**Files**: `server/src/snapshot.rs` (new), `server/src/lib.rs`, `server/src/state.rs`, `server/src/service.rs`

**Story**: `snapshot-core-generation-semantics-snapshot-compatibility`

```rust
// server/src/snapshot.rs
#[derive(Debug, thiserror::Error)]
pub enum SessionCheckpointRejection {
    #[error("session checkpoint payload is not decodable")]
    Decode,
    #[error("session checkpoint has an invalid authority-domain anchor")]
    AuthorityDomain,
    #[error("session checkpoint has an invalid core-generation anchor")]
    CoreGeneration,
    #[error("session checkpoint LSN does not match its storage anchor")]
    Lsn,
}

pub fn decode_compatible_session_checkpoint(
    stored: &StoredSnapshot,
    expected_domain: &AuthorityDomainId,
    expected_core_generation: &Generation,
) -> Result<SessionSnapshot, SessionCheckpointRejection>;
```

**Implementation notes**:

- The decoder checks, without normalization: stored `EventId` domain/positive LSN are present and match the expected domain; the private envelope has the supported session kind/version; the inner Protobuf payload decodes as `SessionSnapshot`; embedded domain and nonzero generation are present and exactly match current values; embedded `snapshot_lsn` exactly equals the stored row LSN. It does not decide freshness or inspect session content.
- Both `ProjectionState::materialize_session_snapshot` and `materialize_resource_snapshot` set `core_generation: Some(self.core_generation.clone())`. Session and resource views from one projection state therefore carry the same epoch.
- `ControlService::load_snapshot` routes stored session checkpoints through the decoder before its existing freshness decision. Compatible current checkpoints may be returned; incompatible or older checkpoints are derived-cache misses and fall back to a newly materialized current session view with the persisted generation. Resource reads remain on-demand and never decode the session slot.
- Keep typed envelope encoding and compatibility validation in a small module reusable by `recovery-checkpoint-writer`; do not add a generic snapshot framework or change the current storage namespace. The storage recovery helper requires a caller-supplied typed decoder/validator and may skip a log prefix only after it succeeds.
- Do not surface a checkpoint mismatch as loss of authoritative state. The request succeeds with repaired current materialization when the log can be folded; genuine storage/log failure remains an error.

**Acceptance criteria**:

- [ ] Every materialized session/resource snapshot carries the persisted nonzero generation and exact authority-domain/current-LSN anchor.
- [ ] A session checkpoint written before process shutdown is accepted after reopening the same database because the new process loads the same durable generation.
- [ ] Missing/zero/different generation, wrong/missing domain, corrupt payload, or embedded/stored LSN mismatch is never returned as authority; current materialization repairs the RPC response.
- [ ] A stale but otherwise compatible checkpoint remains subject to the existing LSN freshness rule; generation equality never makes an older view current.
- [ ] Resource snapshot reads remain discriminated and cannot decode the session checkpoint slot.

### Unit 3: Foundation, model, and executable continuity evidence

**Files**: `core/tests/rusqlite_storage.rs`, `core/tests/audit_records.rs`, `server/src/state.rs` (test module), `server/tests/grpc_smoke.rs`, `contracts/vectors/snapshot-reconciliation.json`, `server/tests/conformance_vectors.rs`, `specs/seed/snapshot_recovery.qnt`, `docs/PROTOCOL.md`, `docs/GLOSSARY.md`, `docs/VERIFICATION.md`, `docs/ARCHITECTURE.md`

**Story**: `snapshot-core-generation-semantics-continuity-evidence`

**Implementation notes**:

- Add storage interface tests for insert-once, concurrent candidates, domain isolation, invalid values, v3→v4 migration preservation, and file reopen persistence. The independent oracle is “the first committed metadata row wins,” not the production insert predicate.
- Add a real file-backed restart test: materialize a current session checkpoint, write it, drop/reopen storage, rebuild the control service, and verify the exact checkpoint generation remains compatible. Replace the payload at the same valid LSN with a different/missing generation and prove `LoadSnapshot` returns a newly materialized snapshot carrying the stored epoch instead.
- Extend the existing promoted `snapshot-reconciliation` executable example to constrain `SessionSnapshot.core_generation` and `ResourceSnapshot.core_generation`. Seed a deterministic generation through `CoreGenerationStore` before service construction and assert both returned views carry it. This strengthens field carriage only; it does not promote `SnapshotCrossDomainRejected`.
- Align `snapshot_recovery.qnt` comments and initialization with a nonzero durable continuity epoch preserved by `crash`/`restart`; keep cross-domain/generation mismatch rejection semantics. Parse/typecheck the model and retain the existing draft promotion metadata unless a separate genuine-checking promotion ceremony is explicitly scoped.
- Roll foundation assertions forward in place: `PROTOCOL` defines exact domain/epoch/LSN compatibility and removes “currently unset/reserved”; `GLOSSARY` removes restart-assigned process-incarnation wording; `VERIFICATION` defines `CoreGeneration` as the durable epoch and keeps the property honestly stated-normative; `ARCHITECTURE` names epoch validation beside the log anchor and reserves restore/fork/HA rollover/fencing. Do not append historical migration prose.

**Acceptance criteria**:

- [ ] Tests kill both load-bearing regressions: overwriting the stored generation on restart and ignoring an embedded generation mismatch.
- [ ] The existing promoted snapshot-reconciliation runner executes and reports the newly constrained session/resource generation fields without changing its property classification.
- [ ] Quint compiles with `CoreGeneration` stable across ordinary crash/restart; model metadata/traceability checks remain green and make no new checked claim.
- [ ] Foundation docs consistently define one committed v0.1.0 durable storage-continuity epoch, exact equality validation, the reserved rollover/process-fence seams, and the derived-checkpoint/log-authority relationship.
- [ ] `recovery-checkpoint-writer` can rely on a persisted `Generation`, a reusable session-checkpoint compatibility check, and explicit fallback-to-log behavior without reopening this semantic decision.

## Implementation Order

1. `snapshot-core-generation-semantics-durable-epoch` — establish atomic durable identity and carry it in `ProjectionState`.
2. `snapshot-core-generation-semantics-snapshot-compatibility` — populate snapshot fields and enforce domain/epoch/LSN compatibility at the stored-session boundary.
3. `snapshot-core-generation-semantics-continuity-evidence` — prove restart/mismatch behavior, strengthen the existing vector, and roll the foundation/model forward.
4. Run full verification, close child checkpoints by their evidence policy, then review the integrated feature at `thorough` weight until a pass yields no receiver-confirmed material current-cycle blockers.

One feature-owning worker should carry all three checkpoints. The storage port, server constructor, compatibility helper, and restart fixture overlap; separate implementation ownership would create non-green handoffs and increase semantic drift.

## Simplification

- Replace two `core_generation: None` branches and scattered ad hoc snapshot identity checks with one persisted value plus one session-checkpoint decoder.
- Keep process incarnation out of v0.1.0 instead of adding a second unused counter or an “accept predecessor” compatibility matrix.
- Reuse the wire-present `Generation` fields, existing authority-domain metadata shape, snapshot table, `StoredSnapshot`, server RNG infrastructure, and log replay fallback. Add one private typed/versioned storage envelope, but no public wire field, legacy dual reader, second log, or parallel authority source.
- Preserve the current session-only checkpoint namespace and give its bytes a minimal typed/versioned envelope; the downstream writer may replace the namespace with a composite or per-projection design when its recovery scope is decided.
- No valuable test is removed. Avoid testing generated accessors or every random value; protect persistence, exact anchor validation, restart continuity, and mismatch fallback.

## Testing

- **Storage interface/migration tests** protect atomic first-writer-wins persistence, per-domain isolation, SQLite range checks, reopen stability, and preservation of durable data.
- **Pure compatibility tests** protect exact domain/generation/LSN matching without RPC/auth fixture noise.
- **Real restart/RPC tests** protect the production composition path and prove an incompatible derived checkpoint cannot become authority while the log can repair it.
- **Existing promoted vector extension** protects generated field carriage across both view kinds; it is not evidence for a promoted cross-generation invariant.
- **Model/foundation checks** protect semantic alignment and honest assurance classification. `SnapshotCrossDomainRejected` stays stated-normative.
- **No broad performance claim**: replay-cost bounds belong to `recovery-checkpoint-writer`, which remains blocked until this feature is implemented and verified.

## Verification commands

```bash
cargo fmt --all -- --check
cargo test -p patchbay-core --test rusqlite_storage
cargo test -p patchbay-core --test audit_records
cargo test -p patchbay-core-server state::tests
cargo test -p patchbay-core-server --test grpc_smoke core_generation
cargo test -p patchbay-core-server --test conformance_vectors snapshot_reconciliation
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
quint compile specs/seed/snapshot_recovery.qnt
node contracts/scripts/check-models.mjs
node contracts/scripts/check-vectors.mjs
```

## Risks

- **Initialization could silently rotate on every restart.** The atomic insert-or-return contract, same-file reopen test, and control-service restart test make overwriting the row a material failure.
- **Independent stores can collide in a 63-bit random epoch.** The probability is negligible for the intended single-operator topology, but the value is not treated as cryptographic authority. Authority-domain equality and log/LSN validation remain required; future HA needs a designed fencing token rather than stronger claims about this marker.
- **A copied database retains its epoch.** That is correct only while it represents the same history. Divergent restore/fork/multi-writer operation must first promote and execute the reserved epoch-rollover/process-fencing ceremony; v0.1.0 must not imply clone safety or split-brain protection.
- **Legacy checkpoints are undiscriminated derived bytes.** Rejecting them causes a one-time full replay but cannot lose authoritative events. No raw-`SessionSnapshot` dual reader is retained: missing envelope/type/version/generation or any other incompatibility falls back intentionally to replay, not a compatibility shim.
- **The current snapshot slot is session-only.** This feature cannot by itself bound whole-core recovery. The downstream writer must choose checkpoint scope honestly and validate every included projection against the same domain/epoch/LSN anchor.
- **Promoted-vector scope could be overstated.** The vector extension proves field carriage in its executable example, not restart continuity or formal cross-generation rejection; docs and review must preserve that distinction.
- **Design-time independent review was unavailable by instruction.** Direct source/foundation mapping, explicit failure mutations, and mandatory `thorough` integrated review mitigate the non-blocking degradation.

## Extension pressure classification

- **Committed v0.1.0:** one core-assigned, storage-persisted, nonzero continuity epoch per authority domain; stable across ordinary process restarts; a private typed/versioned session-checkpoint envelope; exact type/version/domain/epoch/LSN compatibility; invalid derived checkpoints fall back to the durable log/current materialization.
- **Reserved seams:** explicit epoch rollover for history discontinuity; distinct process-incarnation/fencing identity for HA, multi-core, zero-downtime upgrade, or split-brain work; typed composite/per-projection checkpoint namespaces; multiple storage backends and replicas that preserve the same port semantics.
- **Explicitly rejected for this feature:** increment-on-every-restart plus predecessor exceptions, treating core generation as a bearer/security token, accepting generation-less checkpoints as authority, adding process-fencing wire state before a consumer exists, and making a checkpoint an independent ordering source.
- **Parked-idea pressure:** multi-human/federated work remains demarcated by authority domain and will require explicit rollover/fencing rules; desktop/mobile/skins and agent mesh are unaffected.

## Other agent review

- **Invoked because:** core generation is a durable recovery contract whose wrong meaning would nullify the checkpoint writer or admit foreign derived state.
- **Fixed/active blockers:** the design resolves the contradiction by separating storage continuity from process incarnation, requires atomic persistence and exact anchor validation, and names restore/fork/HA rollover as a reserved promotion seam.
- **Parked:** formal promotion of `SnapshotCrossDomainRejected` and process-incarnation fencing; neither is required to implement the current single-writer epoch honestly.
- **Rejected:** restart counters with compatibility windows and a second process-generation field now, for the reasons in Architectural choice.
- **Skipped/degraded:** the delegated endpoint explicitly prohibits nested subagents and peeragent, so no independent design-time pass ran. This is non-blocking under policy. The effective implementation/feature/final completion review weight remains `thorough` (source: explicit operator selection), and the 2026-08-09 review-vetted scope body remains the design input.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (explicit caller selection for the contract-bearing durable recovery epoch).
- Review weight: `thorough` (explicit operator selection); implementation stops at `stage: review` for the required fresh review.
- Files changed: `core/src/storage/{port,mod,recovery,rusqlite,audited}.rs`; `core/src/acceptance/replay.rs`; `server/src/{identity,state,snapshot,lib,service,admin_service}.rs`; `core/tests/{rusqlite_storage,audit_records,recovery,storage_proptest}.rs`; `server/tests/{grpc_smoke,conformance_vectors}.rs`; `contracts/vectors/snapshot-reconciliation.json`; `specs/seed/snapshot_recovery.qnt`; `docs/{PROTOCOL,GLOSSARY,VERIFICATION,ARCHITECTURE}.md`; this feature and its three children.
- Tests added/strengthened: atomic first-writer-wins/domain/range/reopen/migration coverage; exact checkpoint-anchor and wrong-projection envelope regressions; validator-aware recovery fallback for type/version/domain/epoch/LSN/payload mismatches; shared snapshot-carriage coverage; file-backed compatible-restart/stale/mismatch repair RPC evidence; and a promoted vector that stores and rejects its compatible stale session checkpoint while asserting deterministic session/resource generation carriage.
- Simplification: one narrow metadata port, one persisted equality fence, one minimal private checkpoint envelope, one reusable session-checkpoint decoder, and one validator-aware generic recovery boundary replace unset fields, structural Protobuf guessing, unsafe opaque prefix skipping, and scattered identity checks. No process counter, predecessor window, legacy dual reader, new public wire field, second ordering source, or generic checkpoint framework was added.
- Discrepancies from design: storage/migration tests and the real restart/mismatch test landed with the checkpoints they verify rather than waiting for the final evidence commit. Pass-1 review established that the current storage bytes needed a typed envelope even though the eventual composite/per-projection namespace remains downstream-owned; this is a framing correction, not a whole-core namespace decision. The server's rejection type implements `Error` directly because the server crate has no `thiserror` dependency. `cargo fmt --all -- --check` remains red on pre-existing workspace-wide rustfmt drift beginning in untouched `core/src/acceptance/elicitation.rs`; changed Rust files are checked independently and unrelated formatting is preserved.
- Adjacent issues parked: none.
- Integrated verification: `scripts/test-rust` passed the full Rust workspace; `cargo clippy --workspace --all-targets -- -D warnings` passed; the requested targeted core/server tests passed; `PATH="$HOME/.npm-global/bin:$PATH" quint compile specs/seed/snapshot_recovery.qnt` passed; `node contracts/scripts/check-models.mjs` and `node contracts/scripts/check-vectors.mjs` passed with 20 implementation checks and 37 mutation witnesses. The first raw full-test attempt raced clippy and exhausted temporary disk; the repository's scoped `scripts/test-rust` rerun passed, confirming an environmental rather than product failure.

## Review (2026-08-10) — pass 1 fix endpoint

**Verdict**: Request changes — all receiver-accepted pass-1 findings are fixed and verified, but the explicit `thorough` convergence policy requires the corrected snapshot to remain at `stage: review` for the next review pass.

**Blockers**: none unresolved in this fix endpoint
**Important**: none
**Nits**: none
**Rejected**: none; the receiver accepted all four supplied pass-1 proposals

**Pass-1 finding disposition and fixes**:

1. Added a private magic/kind/version checkpoint envelope plus `encode_session_checkpoint`; `LoadSnapshot` unwraps it back to the public generated `SessionSnapshot`. Legacy raw bytes are disposable, a typed resource envelope is rejected as the wrong kind, and direct `ResourceSnapshot` bytes have a regression proving they cannot decode as a session checkpoint.
2. Replaced exported opaque prefix-skipping recovery with generic typed `RecoveryState<T>`/`ValidatedSnapshot<T>` plus a mandatory decoder/validator. Missing/wrong row domain or nonpositive LSN is rejected before validation; any validator rejection for kind/version/embedded domain/epoch/LSN/payload returns no snapshot and full replay from LSN 0. The API is ready for `recovery-checkpoint-writer` to pass `decode_compatible_session_checkpoint` without moving server-specific types into core storage.
3. Strengthened SQLite v4 metadata preflight to require the exact two-column schema, declared `TEXT`/`INTEGER` types, domain `NOT NULL PRIMARY KEY`, the matching unique primary-key index, generation `NOT NULL`, no defaults, and the canonical positive check. Eight schema mutations prove missing/weakened PK, uniqueness, types, nullability, and positivity reject without repair or `user_version` change.
4. Made the promoted `snapshot-reconciliation` session case write its declared LSN-40 checkpoint in the typed envelope, independently decode it as compatible, then prove `LoadSnapshot` rejects it against revision 45 and returns the newer materialized view. The vector now records the seeded-compatible/not-returned expectation explicitly.

**Verification**: `scripts/test-rust` passed all Rust workspace tests, including the wrong-type unit regression, 10 recovery boundary tests, 8 audit/migration tests, 24 SQLite storage tests, 18 storage property/mutation tests, 21 gRPC smoke tests, and the server conformance runner. `cargo clippy --workspace --all-targets -- -D warnings`, `node contracts/scripts/check-models.mjs`, `node contracts/scripts/check-vectors.mjs` (21 implementation checks, 37 killed mutation witnesses), and `quint compile specs/seed/snapshot_recovery.qnt` passed. `git diff --check` and standalone rustfmt checks for the rewritten checkpoint/recovery files passed. Workspace-wide `cargo fmt --all -- --check` remains red on the pre-existing unrelated rustfmt baseline beginning in `core/src/acceptance/elicitation.rs`; no unrelated formatting was taken into this fix.

**Notes**: substrate feature review; effective weight `thorough` from the explicit operator request; pass-1 receiver/fix capability `openai-codex/gpt-5.6-sol` at xhigh. The caller prohibited nested agents and peeragent, so this endpoint performed no independent pass-2 review and intentionally made no item transition.

## Review closure — pass 2

- Fresh-context adversarial pass 2 found no material current-cycle blockers.
- Confirmed typed/versioned checkpoint framing, replay-from-zero rejection
  fallback, epoch persistence/concurrency, strengthened v4 schema preflight,
  stale-vector execution, and downstream checkpoint-writer readiness.
- Recurring findings: none. Both pass-1 checkpoint-boundary blockers were
  eliminated rather than suppressed.
- Rejected non-blocking trigger/index hardening as outside the owned-schema
  threat model and disproportionate to this cycle.
- Effective weight: `thorough` (explicit operator). Verdict: approved.
