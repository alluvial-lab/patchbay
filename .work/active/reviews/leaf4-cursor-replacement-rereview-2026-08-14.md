---
id: leaf4-cursor-replacement-rereview-2026-08-14
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

# Deep re-review — cursor authoritative-replacement contract (Leaf 4), pass 2

**Verdict: MATERIAL.** The fix connected every committed acceptance test to the concrete exported state machine, removed the old disconnected interface/machine pair, added the requested conflict/race cases, and killed both pass-1 surviving mutants. All six original acceptance rows and all seven previously killed mutants still have live oracles. However, the no-op escape remains through ordinary TypeScript subclassing, the reusable store suite accepts a deterministically snapshot-tearing store, and a fresh safety-critical guard mutant survives all committed tests.

## Phase 1 — completeness of the two claimed fixes

| Pass-1 finding | Pass-2 evidence | Disposition |
|---|---|---|
| Exported contract connected; no-op impossible to bless | All cursor acceptance tests instantiate `AuthoritativeCursorReplacement`; the surface-only no-op test and `ExternalCursorProjectionMachine` export are gone. A temporary type-checked behavioral probe nevertheless extended the public class, overrode all three binding methods as no-ops, invoked them through an `AuthoritativeCursorReplacement`-typed reference, and made zero store/fetch/publish/value calls. | **PARTIAL / MATERIAL remains** |
| Epoch/CAS conflict and race oracles | Same-epoch entry/leaf and post-commit cursor/content tests exist. Three barrier-controlled race families and the exported store suite exist. Removing the correlation guard failed 1/20; weakening `MemoryStore` CAS failed 4/20. | **Requested mutants KILLED, but conformance remains incomplete** |

## Findings

### MATERIAL — The exported concrete contract remains bypassable by a no-op subclass

**Location:** `operator-domain/src/reconciliation/external_cursor.ts:135-310`

`AuthoritativeCursorReplacement` has a public constructor and overrideable public methods. Its private dependency fields reject a structurally unrelated object literal, but they do not prevent a downstream consumer from extending the exported class. A temporary probe compiled under the package's normal TypeScript configuration:

```ts
class NoOpReplacement extends AuthoritativeCursorReplacement<Scope, Entry, Cursor, Leaf> {
  override async reconcileKnown() { return []; }
  override async stageReplacement() {
    return { replacementEpoch: 1n, entries: [], leaf: "noop" };
  }
  override async commitReplacement() {}
}

const subject: AuthoritativeCursorReplacement<Scope, Entry, Cursor, Leaf> =
  new NoOpReplacement(throwingStore, throwingFetch, throwingPublish, values);
```

All three methods completed and no injected dependency was called. This is the exact type-compliant bypass the pass-2 brief required trying. The fix therefore narrows the old object-literal hole but does not make the composed transition owner mandatory.

**Required direction:** make construction non-subclassable outside the module, for example with a private constructor plus exported static factory, while retaining a private nominal member. Keep the concrete transition methods as the only constructible adapter-facing behavior and add a compile-fail/type-test for both object-literal and subclass no-ops.

### MATERIAL — The reusable atomic-store suite blesses observable torn snapshots

**Location:** `operator-domain/src/reconciliation/external_cursor.ts:401-515`

The suite awaits each `compareAndSwap` before loading the record. Its “all-or-nothing snapshots” case therefore checks only the settled final record, not what concurrent readers can observe while the write is in flight.

A temporary type-compliant `TearingStore` deterministically did the following on a valid CAS:

1. exposed the next `recordVersion` and `freshness` with the old projection;
2. yielded one event-loop turn;
3. installed the complete next record.

That store passed all four exported conformance cases, including exactly one racing writer and the ambiguous-retry case. A direct concurrent load during the same CAS observed a hybrid that equaled neither the old nor new record. This violates the port's stated complete-record atomicity and would allow a downstream file store to pass the advertised suite while exposing cursor/freshness ahead of projection membership.

**Required direction:** add a real overlapping-reader oracle, with implementation-provided pause/fault instrumentation where black-box timing cannot force the critical window. Concrete durable stores must also exercise their actual locking/rename boundary (and cross-process contention when they claim it), not only final-state equality after a Promise resolves. Phrase the reusable suite's assurance no more strongly than the interleavings it executes.

### MATERIAL — A fresh pending-replacement safety mutant survives

**Location:** `operator-domain/src/reconciliation/external_cursor.ts:159-166`; `operator-domain/tests/external-cursor.test.ts:460-507`

Removing the `freshness !== "current" || pendingReplacement` rejection from `reconcileKnown` left all 20 operator-domain tests green. With that guard removed, a known-suffix call that begins after a record is already `fetching` or `staged` can merge into the old projection and CAS a new `current` record that omits `pendingReplacement`, discarding the authoritative replacement candidate and epoch.

The existing suffix-versus-staging race does not cover this schedule: both operations load the original current record before the CAS barrier. It therefore proves one stale CAS loser only for simultaneous starts, not that a known suffix cannot overtake an already-visible stale epoch.

**Required direction:** add explicit fetching-state and staged-state calls to `reconcileKnown`; each must reject before fetch/publication/write and preserve the complete pre-attempt record. Re-run the guard-removal mutation and require it to fail.

## Mutation and adversarial probe matrix

All mutations/probes were temporary and restored before clean verification.

| Mutation or probe | Result | Pass-2 oracle / observation |
|---|---|---|
| Full replacement changed to upsert old + exact new | **KILLED** (18/20 passed) | Exact replacement and crash-prefix records retained `stale` and failed. |
| Old projection cleared when marking replacement `fetching` | **KILLED** (16/20 passed) | Staging/crash/conflict/race records exposed the clear. |
| New cursor/leaf/epoch installed over old projection membership | **KILLED** (18/20 passed) | Exact replacement and crash-prefix complete-record assertions failed. |
| Scope key included Patchbay generation | **KILLED** (19/20 passed) | N/N+1 continuity-key equality failed. |
| Exact entries/leaf/epoch installed while retaining old cursor | **KILLED** (18/20 passed) | Exact replacement and crash-prefix cursor assertions failed. |
| Exact entries/cursor/epoch installed while retaining old leaf | **KILLED** (18/20 passed) | Exact replacement and crash-prefix leaf assertions failed. |
| Fetching replacement remained `current` instead of `stale` | **KILLED** (14/20 passed) | Pending/current invariant and staging family failed. |
| Pass-1 survivor: staged epoch/entry/leaf correlation guard removed | **KILLED** (19/20 passed) | Same-epoch conflict expected rejection was missing. |
| Pass-1 survivor: `MemoryStore` ignored `expectedRecordVersion` | **KILLED** (16/20 passed) | Three race families and stale-version conformance failed. |
| Fresh mutant: known-suffix pending/freshness guard removed | **SURVIVED** (20/20 passed) | No committed oracle starts known reconciliation from an already-fetching/staged epoch. |
| No-op subclass of exported `AuthoritativeCursorReplacement` | **SURVIVED** | Compiled and behaviorally completed all three methods with zero dependency calls. |
| Snapshot-tearing store run through exported conformance suite | **SURVIVED** | Suite returned all four success labels; direct overlapping load observed a hybrid snapshot. |

## Adversarial port and integration notes

- **Publication ordering:** both racing known-suffix calls publish before either CAS. A clean-tree probe observed two acknowledged publication calls and one local CAS winner. This is safe only if the downstream durable consumer makes stable-id suffixes idempotent/reconcilable and the publication port's acknowledgement has the ordering semantics the Pi design assumes. Those guarantees are not proven by this leaf; the Pi consumer must test them rather than infer them from the local CAS race.
- **Fetch honesty:** a lying `fetchComplete` can return an incomplete but identity-unique set, and the machine will correctly treat that supplied set as exact. Adapter-specific complete-tree validation is necessarily outside this generic value contract and remains an explicit downstream Pi conformance obligation.
- **Factory/wrapper escape:** the old standalone machine/interface is gone; the remaining escape is subclassing the exported class itself.
- **Pi neutrality:** focused source search found no Pi session path, `get_entries`, JSONL, or other Pi-specific ontology in the shared contract.
- **Regression:** all six acceptance rows pass on the clean object, and every one of the seven pass-1 killed mutations was independently re-injected and killed again.

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 20/20.
4. `cd pi-adapter && npm test` — **PASS**, 29/29.

The worktree was clean after every mutation restore and before this review file was written.

## Final recommendation

**Return `research-handoff-spawn-cursor-authoritative-replacement-contract` to `implementing`.** Close the subclass escape, strengthen atomic-store conformance against overlapping reads, and add the already-pending known-suffix guard oracle. Then run pass 3 of the thorough convergence lane.
