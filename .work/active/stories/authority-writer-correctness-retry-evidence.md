---
id: authority-writer-correctness-retry-evidence
kind: story
stage: implementing
tags: [security, foundation]
parent: authority-writer-correctness
depends_on: [authority-writer-correctness-ingest-contract]
release_binding: null
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
