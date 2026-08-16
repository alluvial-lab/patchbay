---
id: pi-control-session-integrity-review-2026-08-15
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-control-session-integrity
created: 2026-08-15
updated: 2026-08-15
---

# Review: Pi control handshake, materialization, and tree integrity

## Verdict

**MATERIAL** — return the story to `implementing`.

The challenged handshake, canonical cwd comparison, current-leaf correlation, physical seal/prefix checks, three-way file classification, redacted failure values, and the four requested existing mutation oracles are strong. The default challenge source is Node's cryptographic `randomBytes`; a clean probe generated 4,096 distinct 256-bit challenges. Two current-cycle gaps remain: the strict validator accepts Pi-permissive invalid shapes/references as materialized or resumed, and BLOCKER 10's continuation-policy admission is not implemented at all despite being marked accepted.

## Findings

### MATERIAL — the strict validator accepts invalid current-v3 fields and dangling control references

**Locations:** `pi-adapter/src/session_file.ts:401-421`, `pi-adapter/src/session_file.ts:441-509`, `pi-adapter/src/session_file.ts:843-857`

The raw parser closes the top-level entry-type set and checks required message fields, but it does not validate the types of installed Pi v3 optional message fields. For example, `validateMessage` ignores `AssistantMessage.responseId`, whose installed Pi type is `string | undefined`. A temporary regression changed an otherwise valid assistant entry to `responseId: 42`, supplied the identical mutated entry through the RPC oracle, and expected `ENTRY_SHAPE_INVALID`; `classifyPiSessionMaterialization` instead returned `materialized`. Pi's permissive loader and raw-vs-RPC equality therefore agree on an invalid current-schema shape, so RPC equality is not an independent shape oracle.

Control references have the same gap. `validateSecondaryReferences` covers label, compaction, branch-summary, and tool-call references, but a `patchbay.control.reload-completion.v1` suffix only checks that `requestEntryId` is a bounded string. A temporary resumed-session probe appended a reload completion naming `missing-request`, followed it with the exact current handshake marker, and supplied exact raw/RPC entries and leaf. `verifyResumedSessionExtension` returned `materialized` instead of `REFERENCE_INVALID`. This directly violates the promised validation of “other entry references where present” and permits `resumed` over forged control history.

**Concrete fix:** make the installed Pi v3 schema one exact runtime validator: validate every supported optional nested field when present (including assistant response/deferred/diagnostic fields, content signatures, bash optionals, and complete `Usage` objects), while preserving fields Pi legitimately permits to be absent. Track earlier control requests by entry id and require every reload completion to reference an earlier request whose command id, nonce, and prior epoch match. Add both reviewer witnesses as permanent regressions and mutation kills; keep raw-vs-RPC equality as a second check, never the schema oracle.

### MATERIAL — BLOCKER 10 has classification but no pre-launch continuation-policy gate

**Location:** `pi-adapter/src/session_file.ts:62-190`

The new API exposes only `classifyPiSessionMaterialization`, exact-seal verification, and post-launch resumed-extension verification. No production code in `pi-adapter/` accepts or distinguishes `require_resume` from `allow_new_context`; repository search found neither policy term. Consequently this commit cannot prove that a `memory_only` or `invalid` session fails before a successor launch, nor that `allow_new_context` omits the resume selector and can report only `new_context`. The checked story evidence closes only the classifier half of BLOCKER 10. A later supervisor could consume the classification correctly, but no current oracle makes that required control decision non-vacuous.

**Concrete fix:** add a small adapter-local, discriminated continuation-admission function consumed before any launch effect. It must require a materialized, freshly revalidated seal for `require_resume`; return an explicit pre-launch refusal for `memory_only`/`invalid`; and force the fresh-session/no-resume path plus only `new_context` for `allow_new_context`. Test it with an injected launch spy that remains untouched on every refused case and with an offline runtime fixture only. If this policy is intentionally deferred to the supervisor story, remove the checked acceptance claim here and keep this story active until that consuming boundary and its mutation oracle exist.

## Mutation matrix

Every source mutation/probe was performed on the main tree, followed by `git restore` for the touched file. The tree was clean after every probe and before the full suite.

| Mutation / probe | Focused oracle | Result |
|---|---|---|
| Remove the configured-project cwd comparison | `wrong initialized cwd cannot pass with correct generic RPC path and id` | **Killed** — expected rejection disappeared |
| Ignore launch-nonce mismatch | `stale challenge, launch nonce, and extension epoch are rejected` | **Killed** — expected rejection disappeared |
| Classify a missing file as `materialized` | `declared path without a regular non-empty file stays memory_only despite in-memory entries` | **Killed** — exact three-way assertion failed |
| Accept an unseen/orphan parent | `strict parser rejects malformed lines, duplicate ids, orphan/forward/self parents, and multiple roots` | **Killed** — orphan classified `materialized` |
| Default challenge entropy/freshness probe | 4,096 calls to clean `generateControlChallenge()` | **Pass** — 4,096 unique 43-character base64url values from 32 random bytes |
| Invalid installed-v3 optional field: assistant `responseId: 42`, identical in raw and RPC | temporary expected-`ENTRY_SHAPE_INVALID` regression | **Survived / gap** — classified `materialized` |
| Reload completion references absent request, followed by exact current handshake marker | temporary expected-`REFERENCE_INVALID` resumed regression | **Survived / gap** — classified `materialized` |
| Search for `require_resume` / `allow_new_context` production admission | `pi-adapter/**/*.ts` | **Missing / gap** — no policy input, decision, or launch-before-effect oracle exists |

## Clean-tree verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS** (57 vectors, 17 promoted, 26 implementation checks, 38 registered mutation witnesses killed).
3. `cd operator-domain && npm run build && npm test` — **PASS** (27/27).
4. `cd pi-adapter && npm test` — **PASS** (54/54, including the real-process E2E).
5. Final `git diff --check` and clean-tree check — **PASS**.

Proto generation/drift, Pi-only naming in `pi_adapter.proto`, bounded streaming-time extension commands, default challenge entropy, raw-value error redaction, and the existing injected offline `ModelRuntime` fixtures produced no additional finding.

## Recommendation

**Return to implementing.** Complete the exact current-v3 schema/control-reference validator and add a non-vacuous pre-launch continuation-policy gate with the named regressions. Rerun the four clean-tree groups and submit the story for the next `thorough` convergence pass.
