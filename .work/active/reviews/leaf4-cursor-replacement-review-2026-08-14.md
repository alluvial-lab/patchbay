---
id: leaf4-cursor-replacement-review-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-cursor-authoritative-replacement-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-14
updated: 2026-08-14
---

# Deep review — cursor authoritative-replacement contract (Leaf 4), 2026-08-14

**Verdict: MATERIAL.** The implemented state machine matches the binding Leaf-4 behavior and all six acceptance rows have real, non-vacuous behavioral oracles. The clean implementation also resisted direct concurrency probes. However, the exported downstream contract is satisfied by a no-op implementation, and two required adversarial mutants survive the committed tests. The verification-tagged contract is therefore not complete enough to hand to the Pi consumer yet.

## Completeness verdict

| Acceptance row | Evidence | Verdict |
|---|---|---|
| External continuity survives Patchbay generation replacement | `operator-domain/tests/external-cursor.test.ts:106-119` compares N/N+1 scope keys, reads N state through N+1, and separates a different continuity id. | **REAL / PASS** |
| Known suffix is idempotent and preserves unrelated state | `operator-domain/tests/external-cursor.test.ts:121-145` checks the complete post-state, retained unrelated member, and unchanged write count on response-loss retry. | **REAL / PASS** |
| Unknown cursor holds the old projection stale while staging | `operator-domain/tests/external-cursor.test.ts:147-172` observes state inside the fetch and after staging; both retain the old projection and expose no staged cursor as current. | **REAL / PASS** |
| Exact atomic replacement removes omissions and installs all fields | `operator-domain/tests/external-cursor.test.ts:174-204` observes the sole commit record, checks exact membership, and proves the omitted stale member is absent. | **REAL / PASS** |
| Crash prefixes expose old-stale or complete-new only | `operator-domain/tests/external-cursor.test.ts:206-234` injects before/after-CAS failures, checks both complete records, and retries the committed epoch inertly. | **REAL / PASS** |
| Required forbidden mutations fail | Independent mutations for upsert-only, clear-before-fetch, cursor-before-projection, and generation-keyed continuity were all killed by the focused tests. | **REAL / PASS** |

The source matches feature §8: `externalCursorScopeKey` uses adapter, deployment, and verified external continuity only; known suffixes merge by stable identity; unknown recovery creates a stale `fetching`/`staged` epoch; commit replaces the exact record through one CAS; and no candidate leaf/cursor becomes current before commit. A focused grep found no Pi session path, `get_entries`, JSONL, or other Pi ontology in `operator-domain/src/reconciliation/external_cursor.ts`.

## Findings

### MATERIAL — The exported consumer contract admits a trivially compliant no-op

**Location:** `operator-domain/src/reconciliation/external_cursor.ts:21-29`, `operator-domain/src/reconciliation/external_cursor.ts:114-279`, `operator-domain/tests/external-cursor.test.ts:260-272`

`AuthoritativeCursorReplacement` is the interface the downstream Pi story says it will implement, but `ExternalCursorProjectionMachine` does not implement that interface: the known and staging method names/signatures differ. The committed “method surface” test then constructs an implementation whose `commitReplacement()` is empty and passes because it asserts only the unconstrained `reconcileKnown()` return value. Such an implementation type-checks while never marking stale, replacing omissions, committing a cursor, or using CAS.

This is a real consumer seam, not merely the usual fact that TypeScript cannot prove implementation correctness: the strong state machine and the declared Pi-facing interface are disconnected, and the test explicitly blesses the disconnected no-op.

**Concrete fix:** make the exported Pi-facing object the concrete composed state machine (or a factory-owned wrapper around it), align its public methods with the binding interface, and restrict Pi injection to narrow fetch/publish/value/storage ports. Replace the surface-only test with behavioral contract tests run against that exported object. If independent implementations remain allowed, export a reusable conformance harness and require every implementation, including Pi, to execute the full acceptance suite.

### MATERIAL — Same-epoch conflict and CAS race guarantees have no committed oracle

**Location:** `operator-domain/src/reconciliation/external_cursor.ts:63-75`, `operator-domain/src/reconciliation/external_cursor.ts:205-220`, `operator-domain/src/reconciliation/external_cursor.ts:260-269`, `operator-domain/tests/external-cursor.test.ts:67-80`, `operator-domain/tests/external-cursor.test.ts:236-258`

Two adversarial mutants survived all 16 operator-domain tests:

1. removing the staged epoch/exact-entry/leaf correlation check in `commitReplacement`; and
2. weakening the only store implementation so it ignores `expectedRecordVersion`.

The test titled “staged exact identity conflicts” checks duplicate entry ids, not different content competing for one epoch. No test drives a stale CAS version or racing writers. In a reviewer probe, the clean machine correctly rejected conflicting same-epoch fetch/commit content and a strong CAS allowed one racing suffix winner. Replacing that store with a type-compliant CAS that ignored the expected version let both writers report success and lost one suffix update. Thus the current source logic is sound under its semantic port assumption, but the promised atomicity/conflict proof is absent and a downstream store can pass the available contract tests while violating it.

**Concrete fix:** add barrier-controlled tests for two suffixes, known-suffix versus replacement staging, and two replacement fetches; require exactly one stale-version CAS winner with no lost update. Add explicit same-epoch different-entry and different-leaf attempts against a staged record, plus a post-commit retry with a different cursor/content; each must fail without changing the authoritative pre-attempt record. Provide a reusable store-conformance suite that every concrete CAS store must run, including stale-expected-version rejection, all-or-nothing snapshots, ambiguous post-commit retry, and racing-writer behavior.

## Mutation matrix

Mutations ran in a detached temporary worktree and were restored after every run. The final suites ran on the clean target tree.

| Mutation | Result | Focused oracle / observation |
|---|---|---|
| Full-fetch replacement changed to upsert old + new | **KILLED** | Atomic replacement and crash-prefix tests exposed retained stale members. |
| Old projection cleared before complete fetch | **KILLED** | Staging, crash-prefix, and fail-closed tests exposed the cleared old state. |
| Cursor made current over the old projection | **KILLED** | Atomic replacement and crash-prefix complete-record assertions failed. |
| Scope key included Patchbay generation | **KILLED** | N/N+1 continuity-key equality failed. |
| Projection/leaf/epoch installed while retaining old cursor | **KILLED** | Atomic replacement and crash-prefix cursor assertions failed. |
| Projection/cursor/epoch installed while retaining old leaf | **KILLED** | Atomic replacement and crash-prefix leaf assertions failed. |
| Replacement remained current instead of holding stale | **KILLED** | Unknown-cursor tests failed through the pending/current invariant. |
| Staged epoch/content correlation removed before commit | **SURVIVED** | All 16 tests passed; no conflicting same-epoch commit oracle exists. |
| Test CAS ignored `expectedRecordVersion` | **SURVIVED** | All 16 tests passed; no stale-version or racing-writer oracle exists. A reviewer race probe then demonstrated a lost suffix update. |

## Adversarial notes

- **Retry semantics:** the clean machine treats an identical committed epoch as inert and rejects conflicting committed content. A delayed competing fetch with different content for the same pending epoch also fails closed.
- **Concurrency:** with a correct CAS, concurrent known suffixes, known-versus-stage, and replacement staging cannot both overwrite one record; one stale writer rejects. The missing assurance is at the concrete-store contract boundary described above.
- **Identity:** entry identity and full entry equality are separate, explicit injected functions; cursor and leaf equality are also explicit. No value-versus-stable-identity ambiguity was found in the machine.
- **Empty exact set:** adapter-neutral authoritative state may legitimately be empty, so a blanket rejection would be wrong. The Pi consumer must distinguish a valid full wipe from corrupt/incomplete Pi evidence through its strict complete-tree validator before calling this contract.
- **Pi isolation:** no Pi vocabulary leaked into the shared contract.

## Full verification suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses, generated bindings clean.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 16/16.
4. `cd pi-adapter && npm test` — **PASS**, 29/29.

The repository tree was clean before this review file was written.

## Final recommendation

**Return `research-handoff-spawn-cursor-authoritative-replacement-contract` to `implementing`.** Connect the exported consumer contract to the strong state machine and add executable same-epoch/CAS concurrency conformance. Re-run the thorough review after those material verification gaps are closed.
