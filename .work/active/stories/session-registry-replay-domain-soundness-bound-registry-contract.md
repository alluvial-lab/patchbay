---
id: session-registry-replay-domain-soundness-bound-registry-contract
kind: story
stage: done
tags: [protocol, foundation]
parent: session-registry-replay-domain-soundness
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Bound session-registry contract

## Checkpoint

Make each `SessionRegistry` an explicitly authority-domain-bound projection,
then give its owned event fold exact identity-plus-payload redelivery semantics.
Route report ingestion and target resolution through that domain boundary, and
fold every successful append into the supplied hot projection before returning.

## Acceptance evidence

- Construction rejects an empty authority domain; session-event/security-clamp
  folds, report lookup/ingest, adapter-stale derivation, and runtime-session
  resolution reject a different domain before mutation or append.
- An exact projection-owned `(authority_domain_id, LSN, StoredEventPayload)`
  redelivery is inert. Reusing the same event identity with different kind or
  bytes, replaying an unseen older owned event, or registering the same live
  slot at a new LSN is corrupt history rather than a content-blind no-op.
- The old per-record `event_lsn <= last_authoritative_lsn` skips and partial
  tombstone/registration duplicate checks are removed; one event ledger owns
  replay equality for registrations, generation bumps, state/metadata changes,
  and security-lockdown clamps.
- Every single- and multi-delta report append uses the same append-then-fold
  helper. A returned success has warmed the supplied projection; a failed
  append has not, and a fold failure after commit forces rebuild-before-reuse.
- The duplicate inherent session resolver is removed; the acceptance-owned
  `TargetResolver` is the one runtime-session resolution boundary.
- The production `CoreDecisionGate` and rebuild-before/after adapter-service
  path remain intact and independently responsible for cross-request
  serialization; the core writer makes no global multi-projection concurrency
  claim.

## Ordering constraints

This checkpoint is first. It establishes the constructor, lookup, replay, and
writer contract consumed by the integration/property evidence checkpoint. It
is semantically independent of `replay-integrity-prefix-discipline`: that
feature validates newly read complete prefixes, while this checkpoint validates
exact redelivery at the session projection boundary.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (explicit caller selection for the protocol/security/durability contract); direct-read implementation with no nested delegation.
- Review weight: `thorough` (explicit caller selection; retained for the feature review boundary).
- Files changed: `core/src/session/{mod,registry,ingest,replay,resolver}.rs`, `core/src/{target,diagnostics/mod}.rs`, `server/src/{state,adapter_service}.rs`, and constructor/contract coverage in `core/tests/{sessions_registry,sessions_ingest,sessions_replay_resolver,replay_integrity,conformance_vectors,diagnostics_projection,resource_acceptance,resource_resolver}.rs` plus `server/src/adapter_service/tests.rs`.
- Tests added/strengthened: constructor/domain binding; exact raw-envelope replay for every session mutation and security lockdown; conflicting, unseen-old, duplicate-fact, and malformed validate-before-mutate cases; immediate single/multi-delta warming; cross-domain report/stale reconciliation with both logs inspected; post-commit fold failure; domain-aware lookup/resolution; hot/cold equality.
- Simplification: removed domainless `Default`/constructors, the duplicate inherent resolver, registration/tombstone content-blind replay shortcuts, four record-LSN no-ops, pre-supersession LSN-only inertness, and caller-managed single-delta warming; all report appends now use `append_and_apply`.
- Discrepancies from design: current HEAD's `DiagnosticsProjection` now embeds `SessionRegistry`, so its constructor and callers also had to become domain-aware; the landed shared `validate_next_replay_event` cold-prefix boundary was preserved unchanged and remains outside the exact-owned-event ledger.
- Adjacent issues parked: none.

## Verification evidence

- `cargo check --workspace --all-targets` — pass.
- `cargo test -p patchbay-core --test sessions_registry` — 15 passed.
- `cargo test -p patchbay-core --test sessions_ingest` — 17 passed.
- `cargo test -p patchbay-core --test sessions_replay_resolver` — 9 passed.
- Constructor-neighbor coverage: `diagnostics_projection` (6), `replay_integrity` (7), `resource_acceptance` (4), `resource_resolver` (5), and `conformance_vectors` (1) all passed.
- `cargo test -p patchbay-core-server concurrent_conflicting_model_reports_leave_a_replayable_log` — pass; evidence remains scoped to the server `CoreDecisionGate` composition root.
- `git diff --check` — pass. Scoped formatting was reconciled against the existing unformatted baseline without global churn; the required global check is recorded at feature verification.
