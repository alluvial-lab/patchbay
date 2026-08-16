---
id: pi-control-session-integrity-rereview-2026-08-15
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-control-session-integrity
created: 2026-08-15
updated: 2026-08-15
---

# Rereview: Pi control handshake, materialization, and tree integrity

## Verdict

**CLEAN** — advance the story to `done`.

Both pass-1 MATERIAL findings are closed. The strict current-v3 boundary rejects the reviewer's invalid optional-field and dangling reload-completion witnesses before raw/RPC equality can legitimize them; reload completions now require one earlier, uncompleted request with exact command-id, nonce, and prior-epoch correlation. The exported continuation-admission boundary refuses `require_resume` from `memory_only` or `invalid` without invoking its launch continuation, while `allow_new_context` supplies no non-null resume selector and constrains the successor status to generated `NEW_CONTEXT`.

## Findings

None.

The admission decision is a real typed Unit-2 boundary rather than a classifier-only promise: `admitPiContinuationBeforeLaunch`, its discriminated plan/result types, and its launch callback are exported from `pi-adapter/src/session_file.ts` and exercised through effect spies. The future Unit-3 story depends on this checkpoint and explicitly consumes the fixed pre-termination admission order. Its downstream ownership of process quiescence/termination, seal revalidation, generated evidence-envelope provenance, launch, and post-launch proof was not treated as a gap.

The refusal tuple matches Leaf 5's closed admissible cell: `QUIESCING_PRIOR + PROVED_NONE + EXECUTION_FAILED + exact_supervisor_pre_launch_failure`. Unit 2 returns the adapter-local decision and `not_attempted` launch-effect fact; Unit 3 remains responsible for supplying the generated current-adapter producer, attachment, adapter-id/generation proof payload, exact claim, and renewed prior-N liveness before any core release.

## Mutation matrix

Every source mutant was applied one at a time on the main tree and reverted with `git restore`. The tree was checked clean after each mutation and before the full suite. The reviewer-only permissive-load probe used one self-cleaning `/tmp` script and did not modify tracked files.

| Mutation / probe | Focused oracle | Result |
|---|---|---|
| Accept assistant `responseId: 42` while raw JSONL and RPC entries agree exactly | `strict Pi v3 validator rejects invalid assistant optional fields and nested signatures` | **Rejected** as `ENTRY_SHAPE_INVALID`; removing the `responseId` type guard was **killed** because the witness became `materialized`. |
| Accept a reload completion naming an absent request before an exact current handshake | `reload completions require an earlier exactly matching request entry` | **Rejected** as `REFERENCE_INVALID`; removing completion/request correlation was **killed** because the resumed witness became `materialized`. |
| Pi-permissive fresh shape: insert a truthy scalar `42` as an interior JSONL line; installed Pi `parseSessionEntries` admits it | Reviewer-only runtime probe against `classifyPiSessionMaterialization` | **Rejected** as `JSON_INVALID`. |
| Route `require_resume` through the `allow_new_context` launch branch | `require_resume admission refuses before launch without a fresh materialized seal` | **Killed** — the spy-observed result changed from `refused`/zero launches to `launch_admitted`. |
| Remove the configured-project cwd comparison while retaining generic RPC path/id agreement | `wrong initialized cwd cannot pass with correct generic RPC path and id` | **Killed** — expected `CWD_MISMATCH` rejection disappeared. |
| Remove launch-nonce correlation | `stale challenge, launch nonce, and extension epoch are rejected` | **Killed** — the stale-nonce case no longer rejected. |
| Classify absent/empty session files as `materialized` | `declared path without a regular non-empty file stays memory_only despite in-memory entries` | **Killed** — the exact three-way classification assertion failed. |
| Stop rejecting a non-null parent not present in the earlier-entry set | `strict parser rejects malformed lines, duplicate ids, orphan/forward/self parents, and multiple roots` | **Killed** — the orphan witness became `materialized`. |
| Clean admission happy paths | `continuation admission selects resume only for require_resume and fresh context only when allowed` | **Pass** — only materialized `require_resume` produced a canonical selector/`RESUMED`; `allow_new_context` produced `resumeSelector: null`/`NEW_CONTEXT`. |

## Full clean-tree verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS** (57 vectors, 17 promoted, 26 implementation checks, 38 mutation witnesses).
3. `cd operator-domain && npm run build && npm test` — **PASS** (27/27).
4. `cd pi-adapter && npm test` — **PASS** (60/60, including the real-process E2E).
5. Final `git diff --check` and pre-review-file clean-tree check — **PASS**.

## Recommendation

**Advance to `done`.** Pass 2 produced no MATERIAL, BLOCKER, redaction, or vacuous-oracle finding. Unit 3 may consume the exported admission plan and retain its designed responsibility for the actual process and evidence-envelope boundary.
