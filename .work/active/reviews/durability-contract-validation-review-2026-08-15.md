---
id: durability-contract-validation-review-2026-08-15
kind: story
stage: done
tags: [review, adapter]
parent: capability-manifest-durability-and-reconciliation-depth-contract-validation
created: 2026-08-15
updated: 2026-08-15
---

# Thorough review — AdapterAssuranceManifestV1 contract validation

## Verdict

**MATERIAL** — return `capability-manifest-durability-and-reconciliation-depth-contract-validation` to `implementing`.

Commit `514b8d7` establishes the complete generated six-dimension V1 registry, strict fresh-attach validation, conservative Pi declaration, and most of the intended replay/advisory evidence. Two material gaps remain: a current V1 record can enter the older category-compatibility replay path, and the promoted advisory vector does not execute the adapter-authoritative delivery-outcome facet it claims.

Review mode: independent fresh-context story review, effective weight `thorough`, one rigorous pass, implementation range `1a7c88a..514b8d7`.

## Findings

### Material

1. **Current V1 can be normalized by the legacy category replay path** (`core/src/adapter/capability.rs:74-79,103`). `legacy_session_only` is selected from `Replay + empty target_categories + empty resource_capabilities` without requiring the assurance block to be absent. Consequently, a categoryless capability carrying a complete current `assurance.v1` rejects under `Attach` but succeeds under `Replay`, is normalized to `RuntimeSession`, and also bypasses the current session-snapshot/category relationship check. This contradicts the binding rule that a record with `assurance.v1` receives the exact current validator and that compatibility applies only when assurance is absent (`.work/active/features/capability-manifest-durability-and-reconciliation-depth.md:164-167`). A temporary regression probe reproduced the divergence: the replay rejection assertion failed because production accepted the current V1 shape. **Concrete fix:** require `capability.assurance.is_none()` (or an equivalent explicit pre-assurance classification) in the legacy category compatibility predicate, then commit a regression test/vector case proving categoryless current V1 rejects in both contexts while a genuinely pre-assurance, categoryless v0.2 record still normalizes on replay.

2. **The promoted advisory vector's delivery-authority claim is not implementation-exercised** (`contracts/vectors/adapter-assurance-advisory-only.json:62-69`; `core/tests/conformance_vectors.rs:2118-2175`). The Rust runner proves maximal assurance cannot bypass a missing Grant and proves a conservative/empty advertised Operation set does not block core acceptance. It stops at `SubmissionOutcome::Accepted`; it never sends the Operation through adapter delivery or applies an adapter result. The vector nevertheless claims `adapter_delivery_outcome_remains_authoritative: true`, but the Rust implementation check never reads that field. Changing only that expected value to `false` left the requested `rust-core:adapter_assurance_advisory_only` implementation check green; only the JavaScript static expectation checker would reject the metadata change. Thus a capability-derived delivery suppression/outcome mutant at the actual delivery seam would not be tested by this promoted implementation check. **Concrete fix:** extend the vector with a production delivery scenario (or add a second registered server runner) in which an attached adapter with an empty conservative advertised set still receives the grant-authorized Operation and its returned acceptance/`unsupported_command` result alone determines the durable delivery outcome. Assert the vector field from that observed result and kill a mutation that substitutes or suppresses the adapter outcome from capability data.

### Nits

None.

## Checklist disposition

- **Dimension registry — PASS.** The `.proto` owns deduplication strength, continuation proof, cursor support, generation fencing, reconciliation strength, and unproven-outcome action. Rust and TypeScript generated artifacts carry the oneof, optional scalar presence, and closed enums; generated drift passes.
- **Fresh fail-fast validation — PASS.** Missing assurance/contract/booleans, unknown or sentinel enums, dual legacy/current dedup, and invalid generated-enum sets reject. The server test proves invalid initial attach/redeclaration writes no registration Observation, publishes no replacement token, and preserves the prior token. Rejection audit writes remain the existing intentional audit behavior.
- **Replay-only normalization — MATERIAL.** Assurance-absent v0.2 dedup values normalize to a complete conservative view and the same bytes reject at Attach, but current V1 can incorrectly enter the separate legacy category normalization path (finding 1).
- **Advisory invariant — PASS in production diff.** Searches and direct inspection found no assurance/capability-to-Grant or capability-to-authority translation. A synthetic capability-derived Grant mutation was killed by the advisory vector. The delivery-outcome evidence claimed by that vector remains incomplete (finding 2).
- **Conformance registry/counts — PARTIAL.** Counts moved 57→59 vectors and 26→28 executed implementation checks; both new checks run. The complete-manifest check is substantive. The advisory check is substantive for Grant/acceptance but vacuous for its claimed adapter-authoritative delivery-outcome facet.
- **Pi E2E compatibility — PASS.** `pi-adapter/src/core_client.ts:408-423` declares `AT_PATCHBAY_BOUNDARY`, three explicit `false` booleans, reconciliation `NONE`, and `MANUAL_REQUIRED`; no uncertain support is promoted to true. The real core/adapter generation-bump, reconnect, and restart E2E passes.

## Mutation matrix

Every mutation/probe was applied alone on the main tree, followed by `git restore`, a clean `git diff`, and no commit. The final full suite ran after all restoration.

| Mutation or probe | Focused oracle | Result |
|---|---|---|
| Regression probe: current complete V1 with no target categories must reject under both Attach and Replay | temporary `current_v1_cannot_enter_legacy_category_normalization_on_replay` test | **DEFECT REPRODUCED** — Attach rejected, Replay accepted; replay assertion failed at exit 101 |
| Infer omitted `cursor_support` as `false` | `complete_assurance_manifest_requires_explicit_false_and_known_non_sentinel_values` | **KILLED** — omitted cursor was accepted and the test failed |
| Normalize a missing/unknown contract branch through legacy replay | `attach_rejects_missing_unknown_version_and_dual_declarations` | **KILLED** |
| Ignore non-sentinel legacy tag 7 when current V1 is present | `attach_rejects_missing_unknown_version_and_dual_declarations` | **KILLED** |
| Synthetic capability→Grant translation for a maximal declaration | requested `adapter-assurance-advisory-only` Rust conformance check | **KILLED** — the formerly denied submission became accepted |
| Silently drop an unknown `known_failure_modes` member | `invalid_manifest_shapes_fail_closed` | **KILLED** |
| Flip only `adapter_delivery_outcome_remains_authoritative` to false | requested `adapter-assurance-advisory-only` Rust implementation check | **SURVIVED** — confirms the runner never observes/asserts that delivery facet; the umbrella JS checker is only a static metadata guard |

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace targets, tests, doctests, and warnings-denied clippy passed.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 59 vectors, 19 promoted vectors, 28 executed implementation checks, 38 registered mutation witnesses; generated drift, model traceability, and TypeScript build passed.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 27/27 tests.
4. `cd pi-adapter && npm test`: **PASS** — 60/60 tests, including the real core/adapter E2E.

The tracked tree was clean before review mutations, after every restoration, before and after the final four-group suite, and before this review file. `git diff --check` passed. Disk discipline was observed without a temporary worktree; initial `/` availability was 54G.

## Recommendation

**Return to `implementing`.** Restrict every legacy replay exception to demonstrably pre-assurance records and add a real adapter-delivery oracle to the promoted advisory vector. After those two material fixes, rerun the focused replay/vector mutations and all four clean-tree verification groups before advancing to `done`.
