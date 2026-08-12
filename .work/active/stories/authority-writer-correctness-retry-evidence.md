---
id: authority-writer-correctness-retry-evidence
kind: story
stage: done
tags: [security, foundation]
parent: authority-writer-correctness
depends_on: [authority-writer-correctness-ingest-contract]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Ambiguous-response, concurrency, audit, and driver evidence

## Checkpoint
Prove the actual hazard is closed at stable boundaries: a committed grant whose acknowledgement is lost retries to the same source id, independent projections cannot race into two identities, audit cardinality stays truthful, and the completed spawn driver repairs an ambiguous descendant write without duplicating it.

## Design contract

**Files:**
- `core/tests/authority_ingest.rs`
- `core/tests/rusqlite_storage.rs`
- `core/tests/audit_records.rs`
- `server/tests/spawn_completion.rs`
- minimal storage wrappers inside those test modules only

## Required evidence
- Replace the false-confidence warm/redelivery-only claim with writer-level retries. Retain replay redelivery tests only for the projection contract they genuinely prove.
- Wrap the real atomic backend so the first `append_grant_audited` commits and then reports a synthetic retryable write failure. Retry through a fresh `AuthorityRegistry` and assert the original source id plus an unchanged event/audit prefix for a normal grant and a deterministic descendant grant.
- Use barriers to race exact and conflicting calls through two independent projections sharing one real SQLite store. Do not use timing sleeps or a shared `CoreDecisionGate` as the oracle.
- Query the audit index: one first-creation `GrantCreated` linked to the source, no audit on exact retry, conflict, or ambiguous-response retry.
- Seed the completed `SpawnCompletionDriver` with a descendant append that committed but reported failure. A fresh bootstrap must observe the existing descendant, finish only missing completion work, and leave one descendant source and one descendant-creation audit.
- Keep model/vector metadata unchanged; this is mutation-sensitive implementation evidence, not formal promotion.

## Acceptance evidence
- [ ] Committed-but-reported-failed normal and descendant writers retry to the exact existing `EventId` after fresh projection/restart.
- [ ] Exact and conflicting races cannot append two same-id grant sources or poison authority replay.
- [ ] Audit queries show one creation audit total per grant identity across all retry/conflict/repair paths.
- [ ] Spawn completion repairs the ambiguous descendant-write response and still exposes terminal completion last.
- [ ] Mutations restoring projection-read + append, duplicate retry audit, partial-content comparison, or separate normal/descendant namespaces fail the focused tests.

## Ordering
Depends on `authority-writer-correctness-ingest-contract`. This is the final child checkpoint; green verification advances it directly to done and makes the parent eligible for thorough integrated review.

## Verification

```bash
cargo test -p patchbay-core --test rusqlite_storage --test audit_records --test authority_ingest --test authority_replay --test authority_proptest
cargo test -p patchbay-core-server --test spawn_completion
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix contracts/ts run check:models
npm --prefix contracts/ts run check:vectors
```

## Implementation run contract
- Capability: Sol xhigh for the security/durability contract.
- Effective review weight inherited from parent: thorough.
- Findings are proposals; the receiver adjudicates materiality from evidence.
- Do not edit docs, backlog, other items, models, vectors, or generated contracts for this checkpoint.

## Implementation notes
- Added a real-backend acknowledgement-loss wrapper: the atomic grant transaction commits once, the caller receives a synthetic retryable write failure, and a fresh normal or descendant projection retries to the exact original source `EventId` with an unchanged prefix and one creation audit.
- Added barrier-controlled exact and changed-content races through two independent `AuthorityRegistry` instances sharing one SQLite store. Exact attempts converge on one id; changed candidates leave one replayable winner and one pre-append `CorruptLog`. No `CoreDecisionGate` is present in the test or writer proof.
- Renamed the prior warm-event redelivery test to state its honest projection-only scope. Audit queries now assert one linked `GrantCreated` across success, exact retry, conflict, and acknowledgement-loss repair.
- Extended `SpawnCompletionDriver` evidence with a descendant transaction that commits and loses its response. The interrupted prefix contains one completion provenance audit and one descendant source but no terminal transition; fresh bootstrap appends only the missing terminal transition and retains one descendant-creation audit.
- Repaired two stale generic storage property fixtures: arbitrary bytes are no longer generated under grant discriminators because v5 correctly validates those identity-bearing envelopes on reopen. Dedicated grant storage tests own that contract. The checkpoint restart seed now uses `ingest_grant` so its v5 identity index is truthful.
- Verification: focused authority/storage/audit/server tests passed; `cargo test --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `npm --prefix contracts/ts run check:models` passed; after installing declared local package dependencies and building the contract/operator packages, `npm --prefix contracts/ts run check:vectors` passed with 21 implementation checks and 37 killed mutation witnesses; `git diff --check` passed.
- Formatting discrepancy: `cargo fmt --all -- --check` remains red from the existing repository baseline beginning in untouched `core/src/acceptance/elicitation.rs`; unrelated whole-file formatting was not applied.
- Design discrepancies/blockers: none material. The generated model/vector metadata was not changed or regenerated. Node conformance initially lacked local `node_modules`/built file-dependency artifacts; dependency installation plus ordinary package builds resolved the environment without tracked changes.
