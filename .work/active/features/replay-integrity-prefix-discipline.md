---
id: replay-integrity-prefix-discipline
kind: feature
stage: implementing
tags: [verification, protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Replay integrity: gap-free LSN + reject Unspecified (cross-projection)

## Brief
Close the replay-integrity gap split out of `authority-provenance-hardening`. Absorbs:

- `backlog-authority-replay-gap-detection` — **OPEN**: authority replay checks `event_lsn <= previous_lsn`, not `== previous_lsn + 1` (gap-free), and `StoredEventKind::Unspecified` is silently ignored (`registry.rs:59-67`); a gapped sequence could resurrect a revoked grant. Contradicts the gap-free LSN contract (`PROTOCOL.md:444-448`). *Src:* authority review Phase 2.

## Direction
This is **cross-cutting, not authority-only** — sessions and acceptance replay share the same `<=` check and the `Unspecified` no-op. Define a shared contiguous-prefix + gap-free replay discipline across authority/session/resource projections: require `event_lsn == previous_lsn + 1` (or document that storage guarantees gap-free delivery) and reject `Unspecified` as `CorruptLog` (Fail Fast). Add tests: gapped LSN sequence, Unspecified-kind event. Couples with `resource-reconciliation-followups` (its applied-prefix cursor is the resource-plane instance of this same invariant) and the sessions replay-equality work.

## Foundation references
- `docs/PROTOCOL.md` — gap-free LSN contract (`:444-448`); event-kind registry
- Code: `core/src/authority/replay.rs`, `core/src/authority/registry.rs`, sibling replay paths (sessions/acceptance)

## Design decisions

- **Use one replay-integrity validator at every full authority-domain log boundary.** The storage port still guarantees ordered, gap-free committed LSNs, but a corrupt database or faulty `Storage` implementation must not be trusted by a projection. Each full-prefix consumer validates domain, concrete event kind, and exact successor LSN before any fold mutates state.
- **Cold replay is a contiguous prefix, not merely a sorted subsequence.** A full rebuild starts with `previous_lsn = 0`, so the first event must be LSN 1 and every later event must equal `previous_lsn + 1`. A future snapshot-tail caller passes the snapshot LSN as `previous_lsn`; an empty prefix remains valid. Overflow, zero, duplicate, reversed, and skipped LSNs are corruption.
- **`StoredEventKind::Unspecified` is corrupt log history, while an unknown numeric kind or malformed event identity is a corrupt record.** Both fail before projection-specific decode. Known concrete sibling event kinds remain valid and are ignored by projections that do not own them.
- **The aggregate server rebuild/catch-up and standalone projection rebuilds share the same rule.** The production composition root currently has its own `<= previous_lsn` check, while command, Elicitation, authority, operator, session, resource, security, and adapter rebuilds duplicate or omit that check. The shared validator replaces those local sequence checks; the server advances `last_applied_lsn` only after every projection accepts the event.
- **Do not apply strict-prefix validation to filtered streams.** Authorized subscription filters, audit pages, and adapter-specific subsets may legitimately omit unrelated LSNs. Only consumers of the complete authority-domain sequence (or a complete tail after an explicit cursor) use this validator.
- **Related replay work remains dependency-independent.** `session-registry-replay-domain-soundness` owns content-equality on exact event redelivery; `resource-reconciliation-followups` owns prefix-covered catch-up redelivery and its applied cursor. This feature owns fail-closed validation of a newly read complete prefix. None is an implementation prerequisite for the others, so the feature keeps `depends_on: []` and records the semantic seam instead of creating an artificial queue block.
- **Assurance remains honestly implementation-checked.** This work enforces already-committed protocol semantics and adds mutation-sensitive Rust evidence. It does not promote `IdempotentLogReplay`, `SnapshotConsistentPrefix`, or a new formal/conformance property; a later verification promotion must follow the model/vector ceremony in `docs/VERIFICATION.md`.
- **Autopilot rationale and capability.** The review-vetted 2026-08-09 direction settled the product choice. Remaining choices are reversible module/error-shape details, resolved toward one small validator and no new schema, cursor type, or persistence mechanism. The caller selected `openai-codex/gpt-5.6-sol` for this normative cross-projection design and `review_weight: thorough`; implementation and final review must retain that weight.

## Codebase mapping

Direct reading covered all foundation docs, the storage port/SQLite gap-free property and recovery helper, every `rebuild*_from_log` entry point, the aggregate `ProjectionState::{rebuild,catch_up,diagnostics_at}` folds, projection event-kind dispatch, and existing authority/acceptance/session/resource replay tests. No exploratory subagent was used because the caller explicitly prohibited nested delegation; the replay surface was enumerable by `read_after`, `rebuild`, and `StoredEventKind` searches.

## UI fallback

No UI surface. This feature changes startup/catch-up integrity and error behavior only; no mockup is required.

## Architectural choice

### Options considered

1. **Trust the storage port and document gap-free delivery only.** This is the smallest code change, and SQLite already has gap-free property evidence. It fails the safety objective: a faulty port, corrupt read, or future backend could return `[1, 3]`, and a projection would accept a non-prefix that can omit a revocation or terminal event.
2. **Chosen: one shared full-prefix validator used by every complete-log consumer.** Validate the `(authority_domain_id, LSN)` identity and generated event kind before projection-specific application, then advance the caller's cursor only after a successful fold. This removes duplicate checks, catches an initial or interior gap, and preserves projection-owned error types.
3. **Give each projection a stateful replay cursor or bespoke check.** This can express the same rule but repeats protocol semantics and risks drift. Reusing the resource feature's applied cursor now would also conflate strict cold replay with its distinct prefix-covered redelivery rule.

The chosen approach follows the durable-log projection and fail-fast boundary patterns. The trickiest unit is **wiring the validator to every complete-log path without applying it to filtered streams or advancing a cursor after a failed fold**; it is designed before the test matrix.

## Implementation Units

### Unit 1: Canonical contiguous-prefix validator

**Files**: `core/src/storage/prefix.rs` (new), `core/src/storage/mod.rs`, `core/src/storage/port.rs`, `core/src/storage/recovery.rs`

**Story**: `replay-integrity-prefix-discipline-shared-replay-boundary`

```rust
// core/src/storage/prefix.rs
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplayIntegrityError {
    #[error("corrupt replay record: {0}")]
    CorruptRecord(String),
    #[error("corrupt replay log: {0}")]
    CorruptLog(String),
}

impl ReplayIntegrityError {
    pub fn map<T>(
        self,
        corrupt_record: impl FnOnce(String) -> T,
        corrupt_log: impl FnOnce(String) -> T,
    ) -> T;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedReplayEvent {
    pub lsn: u64,
    pub kind: StoredEventKind,
}

pub fn validate_next_replay_event(
    authority_domain_id: &AuthorityDomainId,
    previous_lsn: u64,
    event: &RecordedEvent,
) -> Result<ValidatedReplayEvent, ReplayIntegrityError>;
```

**Implementation notes**:

- Validate a present, non-empty event authority domain and exact equality with `authority_domain_id`; require a present LSN; parse `StoredEventKind` from the generated enum; reject `Unspecified`; then require `event_lsn == previous_lsn.checked_add(1)`. All checks occur before returning the event for a fold.
- Classify missing/empty identity and unknown numeric enum values as `CorruptRecord`; classify wrong domain, `Unspecified`, successor overflow, and non-contiguous LSN as `CorruptLog`. Error text includes expected domain/LSN and actual safe values so restart failure is diagnosable without payload disclosure.
- Export the helper from `storage::mod` because it validates values returned by the storage port and must also be consumed by the separate server crate. It is workspace-internal behavior, not a designated v1 public API.
- Tighten `Storage::read_after` documentation: a complete authority-domain read returns every committed event after the cursor in exact LSN order. Do not claim filtered reads are contiguous.
- In `recover`, validate `tail` beginning at the snapshot cursor (or 0) before returning raw recovery state. This makes snapshot-tail recovery honor the same rule; it does not make snapshots an ordering authority.

**Acceptance criteria**:

- [ ] `[1, 2, 3]` from cursor 0 and `[6, 7]` from snapshot cursor 5 validate; empty input is valid.
- [ ] First-event gaps, interior gaps, duplicates, reversals, LSN 0, and successor overflow fail as `ReplayIntegrityError::CorruptLog` before application.
- [ ] Wrong-domain and `Unspecified` events fail as corrupt log; missing identity/LSN and unknown numeric event kinds fail as corrupt record.
- [ ] Known concrete sibling kinds pass prefix validation unchanged.

### Unit 2: Complete-log consumers and fail-closed event dispatch

**Files**: `core/src/acceptance/replay.rs`, `core/src/acceptance/elicitation.rs`, `core/src/acceptance/index.rs`, `core/src/authority/replay.rs`, `core/src/authority/registry.rs`, `core/src/authority/operator.rs`, `core/src/authority/spawn_tail.rs`, `core/src/session/replay.rs`, `core/src/session/registry.rs`, `core/src/resource/replay.rs`, `core/src/resource/registry.rs`, `core/src/security/replay.rs`, `core/src/security/projection.rs`, `core/src/adapter/mod.rs`, `core/src/diagnostics/mod.rs`, `server/src/state.rs`

**Story**: `replay-integrity-prefix-discipline-shared-replay-boundary`

```rust
let validated = validate_next_replay_event(
    authority_domain_id,
    previous_lsn,
    &event,
)
.map_err(|error| {
    error.map(DomainError::CorruptRecord, DomainError::CorruptLog)
})?;

projection.observe(&event)?;
previous_lsn = validated.lsn; // only after the fold succeeds
```

**Implementation notes**:

- Replace local `event_identity`/`<= previous_lsn` replay checks in the command index, Elicitation slot, authority, session, resource, and security rebuilds. Add the same validation to operator and adapter rebuilds, which currently rely on projection dispatch without a shared sequence check.
- Where a domain exposes `CorruptRecord`/`CorruptLog`, preserve that distinction with `ReplayIntegrityError::map`. Add `AdapterError::CorruptLog` rather than flattening adapter replay gaps into a registration error. Diagnostics/server surfaces may wrap the typed error as their existing `CorruptEvent`/`StorageError::CorruptRecord`/`String`, but must retain the common error text and fail before mutation.
- Replace `server/src/state.rs::validate_next_event` with the shared validator in startup rebuild and `catch_up`. Validate once before any of the aggregate projections observe the event; update the global cursor only after all observers succeed. `diagnostics_at` is also a complete `read_through(..., 0, as_of_lsn)` fold and must validate from 0.
- Update direct event receivers that currently treat `StoredEventKind::Unspecified` as a sibling no-op (`CommandIndex`, Elicitation slots, authority/operator/spawn-tail, session, resource, security, adapter, diagnostics) to return their corruption error without mutating state. Continue to ignore every known concrete kind the receiver does not own; do not replace exhaustive generated-enum matches with wildcards.
- Keep filtered subscription/audit-page/service helpers out of this validator. Their authorization/filter contracts, not gap-free projection replay, define which LSNs appear.
- Do not add a persistent cursor to individual projection structs. Cold standalone rebuild uses local `previous_lsn`; the server already owns its applied prefix. The resource follow-up may consume the helper for new-event validation but retains ownership of prefix-covered duplicate semantics.

**Acceptance criteria**:

- [ ] Every complete-log startup, standalone rebuild, resource event-slice rebuild, snapshot-tail recovery, server catch-up, and as-of diagnostics fold rejects a missing LSN between two returned events.
- [ ] A gapped authority prefix cannot reconstruct a live grant past an omitted revocation; no projection returns a partial success value.
- [ ] An `Unspecified` event is rejected before any projection changes, while a valid mixed-kind contiguous prefix still reconstructs every owning projection and lets siblings ignore the event.
- [ ] Server `last_applied_lsn` is unchanged when prefix validation or any fold fails.
- [ ] Searches leave no bespoke full-replay `lsn <= previous_lsn` gate or silent `StoredEventKind::Unspecified` replay branch outside the shared rule.

### Unit 3: Cross-projection and mutation-sensitive evidence

**Files**: `core/tests/replay_integrity.rs` (new), `core/tests/recovery.rs`, `server/src/state.rs` (test module)

**Story**: `replay-integrity-prefix-discipline-cross-projection-evidence`

```rust
// Test-only storage returns caller-supplied RecordedEvent values so corruption
// can be injected after the production append boundary.
struct ScriptedReplayStorage {
    events: Vec<RecordedEvent>,
    snapshot: Option<StoredSnapshot>,
}
```

**Implementation notes**:

- Use a fake `Storage` read path because production append correctly rejects `Unspecified` and SQLite normally allocates gap-free LSNs. The test must exercise replay's independent defense rather than weaken append validation or mutate a real database.
- Add a table-driven integration matrix over every exported complete-log rebuild (command, Elicitation, authority, operator, session, resource, security, adapter) with harmless concrete sibling events at LSN 1 and 3, then with `Unspecified` at LSN 1. Each path must fail in its corruption family.
- Add a server composition-root regression proving both startup rebuild and catch-up reject before advancing `last_applied_lsn`. Preserve a valid mixed-kind prefix case so the fix cannot accidentally reject sibling events.
- Add pure property coverage over bounded LSN sequences: exact `1..=n` succeeds; injecting one skipped LSN anywhere fails. Add an `Unspecified` injection case. The independent oracle computes the mathematical expected successor and does not call the production predicate.
- Record two mutation witnesses in test names/comments: weakening equality back to `actual > previous` must fail the gap case; removing the `Unspecified` rejection must fail the kind case. Do not claim these tests are formal/model promotion.

**Acceptance criteria**:

- [ ] Fixed regressions cover initial/interior gaps and `Unspecified` across the named projections and production aggregate path.
- [ ] The bounded property test catches the old monotonic-only implementation and does not derive its oracle from `validate_next_replay_event`.
- [ ] Valid contiguous replay, snapshot-tail replay, projection determinism, storage gap-free allocation, and existing resource/session/authority/acceptance suites remain green.
- [ ] Because this child is tagged `[verification]`, any later story-level review uses the project deep lane and attacks both mutation witnesses before advancing it to `done`.

## Implementation Order

1. `replay-integrity-prefix-discipline-shared-replay-boundary` — land the common validator, replace complete-log checks, and fail closed on `Unspecified`.
2. `replay-integrity-prefix-discipline-cross-projection-evidence` — add the fake-storage matrix, independent property oracle, and aggregate cursor regressions.
3. Run focused and workspace verification, advance the children by their evidence policy, then review the integrated feature at the caller's explicit `thorough` weight until a pass has no receiver-confirmed material current-cycle blocker.

One feature-owning worker should carry both checkpoints. The files overlap at the replay boundary, and splitting ownership would add handoff risk without creating parallel-safe implementation units.

## Simplification

- Delete the duplicated `event_identity`/monotonic-order helpers and the server-local `validate_next_event`; retain one generated-enum/domain/LSN validator.
- Reuse `RecordedEvent`, `AuthorityDomainId`, `StoredEventKind`, existing domain corruption variants, and the server's existing cursor. Add no schema, migration, snapshot namespace, second log, or configurable replay mode.
- Keep projection payload decoding and state-machine checks projection-owned. The shared helper validates framing/prefix only; it must not become a generic projection framework.
- Do not manufacture tests for each event kind or every projection branch. One all-entrypoint corruption matrix, one valid mixed prefix, and one independent sequence property protect the stable boundary.

## Testing

- **Boundary/property tests** protect exact successor arithmetic, domain identity, concrete generated event kinds, cursor-start semantics, and the two claim-breaking mutations.
- **Projection interface tests** protect that standalone rebuilds actually invoke the shared rule and preserve their typed corruption classification.
- **Server integration tests** protect validate-before-fold and advance-after-success in the production aggregate path.
- **Existing storage tests** remain the independent writer-side evidence that successful appends allocate `1..=N`; replay tests are the reader-side defense and do not replace them.
- **No formal/vector promotion**: run model/vector metadata checks for drift, but report this feature as implementation-checked only.
- **No low-value tests**: do not duplicate the pure validator case in every existing replay file or test generated enum conversion itself.

## Verification commands

```bash
cargo test -p patchbay-core --test replay_integrity
cargo test -p patchbay-core --test recovery
cargo test -p patchbay-core-server state::tests::replay_integrity
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
node contracts/scripts/check-models.mjs
node contracts/scripts/check-vectors.mjs
```

## Risks

- **Filtered-stream false positives are the main misuse risk.** A filter can legitimately return LSN 1 then 3. The validator is named and documented for complete-log replay only; code review and call-site tests must reject using it in subscription/audit pagination.
- **Snapshot-tail off-by-one can reject valid recovery or admit a gap.** The previous LSN is the snapshot cursor, not zero and not `cursor + 1`; focused recovery tests cover `[cursor + 1, cursor + 2]` and a missing `cursor + 1`.
- **Call-site omission can leave one rebuild weaker than production startup.** The inventory is defined by full `read_after`/`read_through` consumers, and the test matrix covers each exported rebuild plus the aggregate server path. Fallback if another full-log consumer appears is to route it through the helper, not add another local check.
- **Replay failure may happen after a durable write.** This feature fails closed and leaves the server cursor unchanged; existing rebuild-before-reuse policy remains the recovery path. It does not attempt log repair or silently skip corruption.
- **Future multi-domain storage must allocate a gap-free sequence per authority domain.** The validator already keys the expectation by domain, matching the committed `(authority_domain_id, LSN)` shape. Cross-domain global ordering remains outside this feature.
- **Independent advisory design review was unavailable by instruction.** The risk is mitigated by direct exhaustive call-site mapping, explicit mutation witnesses, and mandatory `thorough` integrated review; design-time peer absence is non-blocking.

## Extension pressure classification

- **Committed current behavior enforced:** each authority-domain durable log is a gap-free LSN prefix; event identity remains `(authority_domain_id, LSN)`; every durable event has a concrete generated `StoredEventKind`; malformed or incomplete replay fails closed.
- **Reserved seams preserved:** nonzero snapshot-tail cursors, typed future checkpoint namespaces, per-domain allocation for future multiple authority domains, and the resource plane's prefix-covered redelivery cursor. None is implemented or foreclosed here.
- **Explicitly rejected for this feature:** trusting one storage backend as the only integrity gate, skipping a missing LSN, treating `Unspecified` as an ignorable sibling kind, imposing contiguity on filtered streams, inventing a global cross-domain LSN, and adding a parallel replay framework.
- **Parked-idea pressure:** multi-human/federated authority is preserved by validating the full domain-qualified identity; desktop/mobile/skin and agent-mesh ideas are unaffected.

## Other agent review

- Invoked because: replay gaps can resurrect revoked authority and the rule spans independently callable projections.
- Fixed/active blockers: the design validates before fold, advances cursors only after success, preserves known sibling-kind no-ops, distinguishes complete from filtered reads, and uses independent mutation oracles.
- Parked: formal/conformance promotion remains with a separately scoped verification ceremony; resource prefix-covered redelivery remains in `resource-reconciliation-followups`.
- Rejected: storage-only enforcement and a shared stateful cursor, because they respectively miss corrupt reads and conflate strict replay with redelivery semantics.
- Skipped/degraded: the delegated endpoint explicitly forbids nested subagents and peeragent, so no independent design-time pass ran. This is non-blocking by policy. The effective implementation/feature/final completion review weight remains `thorough` (source: explicit operator selection).
