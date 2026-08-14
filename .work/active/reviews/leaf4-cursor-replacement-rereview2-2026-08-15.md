---
id: leaf4-cursor-replacement-rereview2-2026-08-15
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

# Deep re-review — cursor authoritative-replacement contract (Leaf 4), pass 3

**Verdict: NITs.** Fix round 2 closes all three pass-2 MATERIAL findings. The supported TypeScript surface rejects structural and subclass no-ops; both instrumented tearing schedules fail conformance; and fetching/staged known reconciliation rejects before any port effect. All six acceptance rows and the prior mutation families remain live. The only finding is a non-blocking wording/assurance limit: ordinary JavaScript runtime metaprogramming can still replace the exported class or factory behavior, so the guarantee is type-level rather than a same-realm tamper boundary.

## Phase 1 — completeness

| Pass-2 finding | Pass-3 evidence | Disposition |
|---|---|---|
| Export remained subclassable | A direct object literal fails with TS2740; direct subclassing fails with TS2675; a conventional generic mixin fails with TS2345; and `ReturnType<typeof AuthoritativeCursorReplacement.create<...>>` retains the nominal class and rejects a no-op with TS2740. The committed `@ts-expect-error` assertions remain live. | **CLOSED** |
| Store conformance accepted observable tearing | The pass-2 next-metadata/old-projection store and a fresh next-membership/old-cursor+leaf store both invoked the earliest-visible-mutation hook and were rejected with `the executed overlapping reader observed a torn cursor projection snapshot`. | **CLOSED at the documented interleaving** |
| Pending known-suffix guard lacked oracles | Removing the complete guard made both fetching and staged tests fail (12/14 cursor tests passed, 2 failed). Clean tests prove zero fetch, publication, or CAS calls and preserve the exact pre-attempt record. | **CLOSED** |

## Acceptance sweep

| Acceptance row | Clean evidence | Verdict |
|---|---|---|
| External continuity survives Patchbay generation replacement | N/N+1 decorated scopes share a key and state; another external continuity id does not. | **PASS** |
| Known suffix is idempotent and preserves unrelated state | Exact merged projection, one publication, response-loss retry, and unchanged write count are asserted. | **PASS** |
| Unknown cursor retains old projection stale while staging | Fetch-time and staged snapshots retain the complete old projection and expose only pending evidence. | **PASS** |
| Exact atomic replacement removes omissions and installs all fields | The sole commit snapshot checks exact membership, cursor, leaf, epoch, and stale-member removal. | **PASS** |
| Crash prefixes expose old-stale or complete-new only | Before-CAS, after-CAS ambiguity, and identical retry assertions remain green. | **PASS** |
| Forbidden mutation families fail | All seven requested prior families were independently re-injected and killed below. | **PASS** |

## Finding

### NIT — Non-subclassability is a TypeScript guarantee, not a JavaScript tamper boundary

**Location:** `operator-domain/src/reconciliation/external_cursor.ts:156-180`; `operator-domain/tests/external-cursor.test.ts:260-291`

The supported typed construction path is now closed as intended. Harder runtime probes nevertheless produced working no-ops:

- `Object.assign(Object.create(AuthoritativeCursorReplacement.prototype), noOpMethods)` type-checks as the concrete class because the standard `Object.create` return is `any`;
- `noOpMethods as unknown as ConcreteReplacement` deliberately opts out of checking;
- direct assignments to the exported prototype methods compile without a cast and replace behavior on a genuine factory-created instance; and
- the writable static `create` method can similarly be monkey-patched to return an `Object.create` no-op.

`structuredClone` rejected the genuine function-bearing instance, an ordinary constructor mixin failed, and the exported factory-result type did not re-widen the private members. A consumer-defined `Pick` can erase the nominal fields, but that value is not assignable back to the concrete exported type without the same `any`/assertion escape.

This is not a material downstream implementation seam: each successful route deliberately uses JavaScript metaprogramming or disables the type system, and same-realm code can already lie through injected ports or decline to call the transition owner. The feature's adapter-honesty boundary does not treat hostile adapter code as sandboxed. Still, “construction is closed” should be understood and, when next touched, described as **closed under supported TypeScript construction**, not runtime tamper-proof. Freezing the class/prototype/instances would narrow accidental monkey-patching but would not turn same-realm JavaScript into a security boundary.

## Instrumentation assurance

The hook remains optional. A deterministic store that omitted it, allowed the suite's immediate overlapping load to observe the complete pre-CAS state, then exposed a membership-first torn state later before settling, passed all five conformance labels. That result is expected and honestly bounded by:

- the store-port comment (`external_cursor.ts:97-102`),
- the suite comment (`external_cursor.ts:459-463`), and
- the story's residual handoff.

Without the hook, the suite proves stale-version rejection, one immediate overlapping read (which may be the complete pre-state), the complete settled post-state, ambiguous retry, and one racing-writer schedule. It does **not** prove every in-process window or a cross-process/host atomicity claim. A durable store claiming the stronger property must wire the hook at its real visibility boundary and run readers across that boundary. This is an explicit assurance limit, not a material overclaim.

## Mutation and probe matrix

All source/test mutations ran in a detached temporary worktree and were restored after each run. Type/runtime probes were outside the repository. The clean full suite ran on the main worktree.

| Mutation or probe | Result | Oracle / observation |
|---|---|---|
| Direct structural no-op object | **KILLED** | TS2740: private dependencies/nominal members absent. |
| Direct no-op subclass | **KILLED** | TS2675: private constructor forbids extension. |
| Generic constructor-mixin subclass | **KILLED** | TS2345: private constructor is not assignable to the mixin's public constructor constraint. |
| Factory `ReturnType` alias assigned a no-op | **KILLED** | TS2740; the alias retains the concrete nominal type. |
| Bare `structuredClone` of a genuine instance | **KILLED** | Clone rejects function-bearing injected dependencies and does not yield a usable replacement. |
| `Object.create` + own no-op methods | **BYPASS / NIT** | Works via standard-library `any`; deliberate runtime metaprogramming, not supported construction. |
| Cast through `unknown` | **BYPASS / NIT** | Works by explicitly disabling TypeScript assignability. |
| Prototype or static-factory monkey-patch | **BYPASS / NIT** | Both produced working no-ops with zero dependency calls; same-realm runtime tampering is outside the stated trust proof. |
| Pass-2 metadata-first/old-projection tearing store, instrumented | **KILLED** | Conformance rejected the overlapping hybrid. |
| Fresh membership-first/old-cursor+leaf tearing store, instrumented | **KILLED** | Conformance rejected the different overlapping hybrid. |
| Delayed tearing store with instrumentation omitted | **SURVIVED / DOCUMENTED** | Immediate reader saw complete pre-state; later unobserved tear settled complete. The suite expressly disclaims unexecuted windows. |
| Pending/freshness guard removed from `reconcileKnown` | **KILLED** (12/14 passed) | Fetching and staged guard tests failed before their zero-effect/preservation assertions. |
| Replacement changed to old-plus-new upsert | **KILLED** (12/14 passed) | Exact replacement and crash-prefix tests failed. |
| Old projection cleared before complete fetch | **KILLED** (10/14 passed) | Staging, crash, conflict, and known-vs-stage assertions failed. |
| New cursor/leaf/epoch installed over old membership | **KILLED** (12/14 passed) | Exact replacement and crash-prefix complete-record assertions failed. |
| Patchbay generation appended to continuity key | **KILLED** (13/14 passed) | N/N+1 continuity equality failed. |
| Staged epoch/entry/leaf correlation guard removed | **KILLED** (13/14 passed) | Same-epoch conflict/preservation test failed. |
| `MemoryStore` ignored expected CAS version | **KILLED** (10/14 passed) | Three barrier races and stale-version conformance failed. |
| Fetching replacement left `current` instead of `stale` | **KILLED** (8/14 passed) | Staging, crash, conflict, and race invariants failed. |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23.
4. `cd pi-adapter && npm test` — **PASS**, 29/29.

The main worktree was clean after verification and before this review file was written.

## Final recommendation

**Advance `research-handoff-spawn-cursor-authoritative-replacement-contract` to `done`.** Fix round 2 closes every material pass-2 gap. Retain the runtime-metaprogramming limitation as a NIT; it does not justify another thorough convergence pass or block the downstream Pi implementation.
