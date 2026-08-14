---
id: research-handoff-spawn-cursor-authoritative-replacement-contract
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-identity-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-14
---

# External cursor authoritative-replacement contract

## Checkpoint

Define the spawn-side adapter-neutral cursor contract consumed by the Pi redesign. External persisted-state cursors are scoped by verified external continuity identity, not by Patchbay generation. A known cursor may apply a suffix. An unknown cursor requires a staged exact-set/tree rebuild and atomic replacement of projection + leaf + cursor + epoch.

A full fetch cannot be applied as upserts over an old projection: omissions must remove stale projected entries before the new cursor becomes authoritative.

## Design

**Files**
- `contracts/proto/patchbay/adapter.proto` — adapter-neutral cursor replacement capability/epoch shape where wire carriage is required.
- New `operator-domain/src/reconciliation/external_cursor.ts` — generated-contract-consuming state-machine interface shared by adapter profiles without making the generated-artifact package own domain logic.
- Contract tests for exact-set replacement, crash prefixes, and cursor scoping.

```ts
export interface ExternalCursorScope {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly externalContinuityId: string;
}

export interface ProjectionReplacement<Entry, Cursor, Leaf> {
  readonly replacementEpoch: bigint;
  readonly exactEntries: readonly Entry[];
  readonly cursor: Cursor;
  readonly leaf: Leaf;
}
```

The following Pi redesign defines `externalContinuityId` from verified Pi session identity and implements storage/reconciliation. This leaf does not import Pi session paths, `get_entries`, or JSONL into core ontology.

## Acceptance evidence

- [x] Cursor scope survives Patchbay generation replacement when verified external continuity remains the same.
- [x] Known-cursor suffix applies idempotently without replacing unrelated state.
- [x] Unknown cursor keeps the old projection stale while a replacement is staged.
- [x] Atomic replacement removes entries absent from the authoritative exact set/tree and installs cursor/leaf/epoch together.
- [x] Crash before commit preserves old stale state/cursor; crash after commit exposes only the complete replacement.
- [x] Upsert-only, clear-before-fetch, cursor-before-projection, and generation-keyed-continuity mutations fail.

## Ordering constraint

Independent early leaf after logical identity. Every spawn reconnect operation and the Pi cursor redesign consume it; this story owns no Pi implementation.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected strongest worker for the security-critical cursor/authority contract).
- Review weight: `thorough` (caller override; implementation stops at `stage: review` for the independent completeness → adversarial deep lane).
- Dispatch rationale: direct-read only. The story, both binding feature designs, foundation docs, generated-contract seam, and the bounded existing operator-domain package made the integration points explicit; this delegated worker did not attempt a prohibited recursive spawn.
- Files changed:
  - `operator-domain/src/reconciliation/external_cursor.ts` — generated-`AdapterId`-consuming `ExternalCursorScope`, required `AuthoritativeCursorReplacement` interface, length-framed continuity key, injected atomic CAS storage port, known-suffix transition, staged stale replacement epoch, exact-set validation, and all-fields-at-once commit.
  - `operator-domain/tests/external-cursor.test.ts` — acceptance and crash-prefix contract tests plus mutation-sensitive oracles.
  - `operator-domain/src/index.ts` and `operator-domain/package.json` — root and explicit reconciliation-subpath exports for downstream adapter consumers.
- Mechanism: current projection state keeps `{exactEntries, leaf, cursor, replacementEpoch}` in one record. Unknown-cursor recovery first atomically marks that record stale while retaining it, then executes an injected complete-fetch callback and stores the validated candidate only as `fetching`/`staged` pending state. Commit performs one store CAS from the stale staged record to the exact replacement, so omissions disappear and no leaf/cursor/epoch can lead projection membership. Store failures injected before and after CAS model both crash prefixes without filesystem or clock dependencies.
- Acceptance tests:
  - `cursor continuity scope ignores Patchbay generation replacement` proves N/N+1 decorations resolve the same verified external-continuity key while a different continuity id does not.
  - `known cursor suffix is idempotent and preserves unrelated projection members` proves stable-id response-loss replay is inert and preserves members absent from the suffix.
  - `unknown cursor marks and retains the old projection stale before complete fetch` proves no clear/current exposure during fetch or staging.
  - `atomic authoritative replacement removes omissions and installs projection leaf cursor and epoch together` proves omitted stale membership disappears in the sole commit record.
  - `injected crashes expose either the old stale record or the complete replacement` proves before-CAS and after-CAS outcomes and idempotent post-commit retry.
  - Focused conflict tests reject duplicate exact identities and conflicting same-id suffix content without promoting partial state; the required interface shape is compile/runtime exercised.
- Mutation evidence (each mutation was injected into production code, the named focused test failed, and `git restore` returned the staged baseline; no mutant was committed):
  - upsert-only merge at replacement commit → omission/atomic-replacement test failed because `stale` survived;
  - clear-before-fetch → staging test failed because the old exact entries became empty during fetch;
  - cursor/leaf/epoch install with the old projection → crash-prefix test failed because the after-commit state exposed the new cursor over old membership;
  - generation appended to the continuity key → scope test failed because N and N+1 produced different keys.
- Simplification: no Protobuf fields were added. The cursor state machine is adapter-local/domain behavior, existing generated `AdapterId` supplies the only shared boundary scalar, and the downstream Pi story already owns its genuine Pi replacement wire envelopes. This avoids importing Pi paths, `get_entries`, JSONL, or a premature generic wire DTO into core ontology.
- Discrepancies from design: `contracts/proto/patchbay/adapter.proto` remains unchanged because no generic wire carriage is required at this leaf; this follows the story's wire-only-when-genuine rule. The design's required TypeScript interface is unchanged and is backed by a reusable transition owner plus injected storage port.
- Adjacent issues parked: none.
- Verification (all required groups passed):
  1. PASS — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.
  2. PASS — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` (54 vectors read, 17 promoted, 38 registered mutation witnesses killed; generated artifacts clean).
  3. PASS — `cd operator-domain && npm run build && npm test` (16 tests passed, including 7 cursor-contract tests).
  4. PASS — `cd pi-adapter && npm test` (29 tests passed).

### Fix round — exported state-machine binding and CAS/epoch oracles

- Trigger: the 2026-08-14 thorough review accepted all six original acceptance rows but returned two MATERIAL gaps: a no-op could satisfy the disconnected exported interface, and the same-epoch correlation/CAS-version mutants survived.
- Finding 1 — concrete exported consumer contract:
  - Replaced the structurally implementable `AuthoritativeCursorReplacement` interface plus separately exported `ExternalCursorProjectionMachine` with one exported concrete `AuthoritativeCursorReplacement` transition owner. Private constructor dependencies make its consumer type nominal enough that the former no-op object literal no longer satisfies it.
  - Its public methods now match the binding surface: `reconcileKnown(scope, cursor)`, `stageReplacement(scope)`, and `commitReplacement(scope, replacement)`. The staged result additionally carries its required epoch so a consumer can construct the correlated commit without inventing state.
  - Adapter profiles may inject only `ExternalCursorFetchPort`, `ExternalCursorPublishPort`, `ExternalCursorValueContract`, and `AtomicExternalCursorProjectionStore`. Fetch and publication behavior remain adapter-owned; stale/current, epoch correlation, exact replacement, retry, and CAS transitions cannot be reimplemented downstream.
  - Every cursor acceptance/crash test now drives the exported concrete object. The former surface-only test and blessed empty `commitReplacement()` implementation were deleted.
- Finding 2 — committed conflict/race/store oracles:
  - Added explicit staged same-epoch different-entry and different-leaf commit attempts, then different-cursor and different-content attempts after a successful commit. Every attempt asserts rejection, no publication, and exact preservation of the complete authoritative pre-attempt record.
  - Added barrier-controlled races for two known suffixes, known suffix versus replacement staging, and two complete replacement fetches. Each race asserts exactly one CAS success, one stale-version rejection, and a final complete snapshot equal to the winner rather than a last-writer overwrite or hybrid record.
  - Added exported, framework-neutral `assertAtomicExternalCursorProjectionStoreConformance`. Every concrete CAS store must run it from its own tests; the suite checks stale-expected-version rejection, all-or-nothing complete snapshots, ambiguous post-commit retry, and racing-writer behavior. The committed `MemoryStore` executes all four cases.
- Files changed:
  - `operator-domain/src/reconciliation/external_cursor.ts` — concrete composed consumer object, narrow fetch/publish/value/storage ports, binding-aligned methods, and reusable CAS-store conformance suite.
  - `operator-domain/tests/external-cursor.test.ts` — exported-object behavioral acceptance suite, same-epoch/post-commit preservation oracles, three controlled race families, and the first concrete-store conformance execution.
- Mutation evidence (mutants were applied only to the worktree over the staged fixed baseline and then reverted with `git restore`; neither mutant was committed):
  - Removed the staged epoch/exact-entry/leaf correlation guard in `commitReplacement` → `same-epoch and post-commit conflicts preserve the authoritative pre-attempt record` failed with `Missing expected rejection` (19/20 operator-domain tests passed, 1 failed). **KILLED.**
  - Removed `expectedRecordVersion` enforcement from `MemoryStore.compareAndSwap` → both-winner assertions failed for the suffix and replacement-fetch races, the known-vs-stage stale-version oracle failed, and the reusable suite failed `stale expected recordVersion must reject` (16/20 passed, 4 failed). **KILLED.**
- Discrepancies/rationale:
  - The binding feature's illustrative TypeScript uses an interface. This fix deliberately preserves its three-method surface as a concrete class instead, because continuing to permit independent implementations would recreate the reviewed no-op seam and require a larger implementation-conformance program. The class is still adapter-neutral and package-private to this repository's unpublished boundary.
  - `stageReplacement` returns the binding's `{ entries, leaf }` plus `replacementEpoch`; the epoch was already mandatory in `ProjectionReplacement` and exposing the state-machine-owned value removes caller invention.
  - No Protobuf or foundation-doc change was required. Publication remains an injected adapter-owned acknowledgement port; this leaf implements no adapter-specific envelope or Pi behavior.
- Full clean verification after both mutation restores:
  1. **PASS** — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.
  2. **PASS** — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` (54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses; generated bindings clean).
  3. **PASS** — `cd operator-domain && npm run build && npm test` (20/20 tests; 11 cursor-contract/store-conformance tests).
  4. **PASS** — `cd pi-adapter && npm test` (29/29 tests).
- Residual handoff: downstream concrete CAS stores must execute the exported store-conformance suite, and the Pi story must supply only the four narrow ports. This leaf intentionally does not implement or claim Pi publication/storage conformance.
