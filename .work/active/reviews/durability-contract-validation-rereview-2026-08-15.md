---
id: durability-contract-validation-rereview-2026-08-15
kind: story
stage: done
tags: [review, adapter]
parent: capability-manifest-durability-and-reconciliation-depth-contract-validation
created: 2026-08-15
updated: 2026-08-15
---

# Thorough re-review — AdapterAssuranceManifestV1 contract validation

## Verdict

**CLEAN** — advance `capability-manifest-durability-and-reconciliation-depth-contract-validation` to `done`.

Pass 2 reviewed fix commit `2164cec` against the pass-1 findings and the integrated implementation from `514b8d7`. Both material gaps are closed: current V1 records cannot enter the categoryless legacy replay path, and the advisory vector now executes the authenticated adapter delivery/result seam through durable command replay.

Review mode: independent fresh-context story re-review, effective weight `thorough`, pass 2. Disk discipline was observed without a temporary worktree; initial `/` availability was 54G.

## Findings

No blocker, material, important, or nit findings.

### Pass-1 closure

1. **V1 routing strictness — CLOSED.** `legacy_session_only` now requires `capability.assurance.is_none()`. The committed regression rejects categoryless current V1 under both Attach and Replay while preserving categoryless pre-assurance Replay normalization. A fresh probe additionally confirmed that categoryless V1 with an unknown assurance enum and V1 combined with legacy tag 7 reject under both validation contexts.
2. **Adapter-authoritative delivery outcome — CLOSED.** The new `rust-server:adapter_assurance_delivery_outcome` check attaches a conservatively declared adapter with an empty advertised Operation set, accepts an independently Grant-authorized `instruct`, observes its production delivery, ingests its acknowledgement and `unsupported_command` Result through authenticated adapter ingress, and rebuilds the durable command as `REJECTED` / `UNSUPPORTED_COMMAND`. The former outcome-flip survivor is now killed by this implementation check.

## Mutation matrix

Every temporary probe/mutant was applied alone on the main tree, followed by `git restore`, `git diff --exit-code`, and a clean status check. The final full suite ran only after all restorations.

| Mutation or probe | Focused oracle | Result |
|---|---|---|
| Fresh baseline variants: categoryless V1 with unknown `reconciliation_strength`; V1 plus non-sentinel legacy tag 7; each under Attach and Replay | temporary `current_v1_routing_fresh_variants_reject_attach_and_replay` | **PASS** — all four context/shape combinations rejected |
| Remove `assurance.is_none()` from legacy category normalization | `current_v1_cannot_enter_legacy_category_normalization_on_replay` | **KILLED** — Replay accepted the categoryless current V1 and the regression failed at exit 101 |
| Flip `adapter_delivery_outcome_remains_authoritative` to `false` | requested `rust-server:adapter_assurance_delivery_outcome` | **KILLED** — actual durable `REJECTED` / `UNSUPPORTED_COMMAND` disagreed with the flipped expectation; exit 101 |
| Infer omitted `cursor_support` as `false` | `complete_assurance_manifest_requires_explicit_false_and_known_non_sentinel_values` | **KILLED** — omitted cursor was accepted and the explicit-presence assertion failed |
| Normalize a missing/unknown contract branch through legacy Replay | `attach_rejects_missing_unknown_version_and_dual_declarations` | **KILLED** — unknown contract entered Replay and the test failed |
| Ignore non-sentinel legacy tag 7 beside current V1 | `attach_rejects_missing_unknown_version_and_dual_declarations` | **KILLED** — dual declaration was accepted and the test failed |
| Synthesize a Grant from advertised `supported_operation_kinds` for the maximal declaration | requested `rust-core:adapter_assurance_advisory_only` | **KILLED** — the formerly denied submission became accepted; exit 101 |
| Silently drop unknown `known_failure_modes` members | `invalid_manifest_shapes_fail_closed` | **KILLED** — the unknown failure-mode case was accepted and the test failed |

Clean focused confirmation also passed for the committed V1 routing regression and requested server delivery-outcome check.

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace targets, tests, doctests, and warnings-denied clippy passed.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 59 vectors, 19 promoted vectors, 29 executed implementation checks, and 38 killed registered mutation witnesses; generated drift, model traceability, and TypeScript build passed.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 27/27 tests.
4. `cd pi-adapter && npm test`: **PASS** — 60/60 tests, including the real core/adapter generation-bump, reconnect, and restart E2E.

The tracked tree was clean before probes, after every restoration, before and after all four verification groups, and immediately before this review file. `git diff --check` passed.

## Recommendation

**Advance to `done`.** The thorough pass-2 convergence condition is met: both receiver-confirmed pass-1 materials are closed, all seven relevant mutants are killed, the promoted evidence now has a non-vacuous delivery/result oracle, and the full four-group clean-tree suite is green.
