---
id: durability-feature-review-2026-08-16
kind: story
stage: done
tags: [review, adapter]
parent: capability-manifest-durability-and-reconciliation-depth
created: 2026-08-16
updated: 2026-08-16
---

# Thorough feature review — capability-manifest durability and reconciliation depth

## Verdict

**MATERIAL** — return `capability-manifest-durability-and-reconciliation-depth` with focused scope.

The two child leaves integrate cleanly around one generated V1 contract and one Rust validated assurance type. Complete declarations, explicit false/none uncertainty, strict attach/redeclaration validation, replay-only legacy normalization, canonical diagnostics, conservative current adapter manifests, generated reconciliation vocabulary, and the advisory authority/delivery boundary are present and green. One feature-level presentation claim remains unimplemented: the closed reconciliation-action qualifier is applied to accepted external ambiguity but not to the feature's promised pre-acceptance `SubmissionOutcome.UNKNOWN` path.

Review mode: independent fresh-context feature review, effective weight `thorough`, pass 1. Scope was proportionate to the two already-reviewed children: integration seams and feature acceptance were inspected without repeating their line-level reviews. Disk discipline was observed without a temporary worktree; `/` had 54G free at review start.

## Findings

### Material

1. **Pre-acceptance unknown never consumes the generated reconciliation action** (`.work/active/features/capability-manifest-durability-and-reconciliation-depth.md:60,181-187`; `web-cockpit/src/ui/session-detail.ts:567-580`; `web-cockpit/src/ui/operation-delivery.ts:175-190`; `cli/src/output.ts:60-61`; `docs/UX.md:32`). The feature promises `NONE → unknown` and `MANUAL_REQUIRED → manual-required` for `SubmissionOutcome.UNKNOWN` when adapter reconciliation is the limiting factor, and the rolling UX assertion says conformant surfaces apply that qualifier to both submission unknown and accepted `execution_outcome_unknown`. Production currently calls `unknownOutcomeQualifier` only from the accepted-command failure banner for `FailureCode.EXECUTION_OUTCOME_UNKNOWN`; web submission feedback and CLI submission output emit fixed generic unknown guidance without accepting or looking up the owning adapter's canonical assurance. Thus the closed qualifier is end-to-end only for accepted external ambiguity, not for the feature's first mapping row, and `docs/UX.md` overstates shipped behavior. **Required direction:** either wire the owning adapter's validated generated action into the pre-acceptance unknown presentation when the limiting adapter can be identified, with focused web/CLI evidence, or narrow the feature mapping and rolling assertions if that branch is semantically inapplicable. The generated contract and Pi-profile seam need not change.

### Nits

1. **Carried from the consumer child review:** the CLI's production enum-sentinel rejection is correct, but its focused test does not table-drive `UNSPECIFIED`/unknown numerics across all three assurance enums (`cli/src/commands/diagnostics.ts:515-523`; `cli/tests/output-diagnostics.test.ts:431-456`). This is defense-in-depth coverage only and does not block the current fix scope.

No other material, important, or nit findings.

## Acceptance and integration disposition

- **Generated registry and conservative declarations — PASS.** `contracts/proto/patchbay/adapter.proto` owns the frozen complete V1 shape and both closed enums. Pi emits Patchbay-boundary deduplication, three explicit false evidence flags, reconciliation `none`, and `manual_required`; token-commune emits all-conservative false/none values.
- **Fail-fast attach/redeclaration and replay-only normalization — PASS.** Attach validates before registration append or token publication. Current V1 is strict under Attach and Replay; only assurance-absent durable history receives conservative normalization, and diagnostics emit the normalized canonical V1.
- **Single source and cross-child seam — PASS.** Consumers import generated Protobuf types and the core reuses `ValidatedAdapterCapability` / `ValidatedAdapterAssurance`; no second validator implementation, flattened six-field diagnostics DTO, or local assurance enum/string registry exists. The dependent Pi manifest-profile story can add its opaque Pi-local profile while importing this exact generic assurance block.
- **Advisory invariant — PASS.** The repository-wide capability/assurance usage sweep found no capability-derived Grant, authority decision, Operation completion, or delivery suppression. Retry presentation first requires a canonical qualifying failure and then combines it with V1 deduplication; supported-operation/cancellation declarations only hide advisory UI actions. The promoted server vector proves a conservative manifest cannot suppress grant-authorized delivery or override the adapter's `unsupported_command` result.
- **Reconciliation declaration and accepted-ambiguity qualifier — PARTIAL / MATERIAL.** Reconciliation strength and action are declared and validated; `execution_outcome_unknown` receives `unknown`/`manual-required`. The promised `SubmissionOutcome.UNKNOWN` branch is missing as described above.
- **Rolling docs — MATERIAL DRIFT.** `ARCHITECTURE`, `PROTOCOL`, `VERIFICATION`, and `GLOSSARY` accurately describe the generated registry, conservative manifests, canonical diagnostics, and advisory boundary. `UX.md` asserts the missing pre-acceptance qualifier path as current surface behavior.
- **Substrate — PASS.** Both child stories are `done`; the initial contract review, clean pass-2 re-review, and consumer review are retained. The feature's extension-pressure section uses committed / reserved seam / explicitly rejected classifications and the central protocol seam registry carries the same three-way disposition.

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace targets, tests, doctests, and warnings-denied clippy passed; the server suite included 84 unit tests.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 59 vectors, 19 promoted vectors, 29 executed implementation checks, and 38 killed mutation witnesses; generated drift and traceability are current.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 27/27 tests.
4. `cd pi-adapter && npm test`: **PASS** — 61/61 tests, including the real core/adapter loop.
5. `cd web-cockpit && npm test`: **PASS** — 143/143 tests.
6. `cd cli && npm test`: **PASS** — 51/51 unit tests plus the real-core resource projection.
7. `cd token-commune-adapter && npm test`: **PASS** — 63/63 tests, including both real-core flows.

`cargo fmt --all -- --check` and `git diff --check` also passed. The tracked tree remained clean throughout the full suite and before this review file was written.

## Recommendation

**Return with focused scope.** Close or explicitly narrow the pre-acceptance `SubmissionOutcome.UNKNOWN` qualifier claim and roll `docs/UX.md` to the resulting shipped behavior. Then rerun the focused submission-unknown presentation tests plus the full clean-tree suite and commission thorough pass 2. The generic V1 contract and Pi manifest-profile consumption seam are otherwise ready.

## Fixed — 2026-08-16

The material finding is wired rather than narrowed. One shared operator-domain derivation now combines `SubmissionOutcome.UNKNOWN` with the owning adapter's generated, complete assurance V1 `unproven_outcome_action`; `NONE` renders `unknown`, `MANUAL_REQUIRED` renders `manual-required`, and missing/malformed/unavailable declarations default conservatively to `manual-required`. Non-unknown submission outcomes return no qualifier, so capability alone remains advisory and cannot create a presentation decision.

The web cockpit retains the submitted Operation's adapter id, looks up that adapter's canonical diagnostics capability, and applies the qualifier to both typed and transport-inferred unknown feedback. The CLI performs one best-effort canonical adapter-status query only after a typed unknown for adapter-targeted commands, passes the returned capability beside the outcome into JSON/human presentation, and falls back to conservative `manual-required` if the query or declaration is unavailable. Accepted `execution_outcome_unknown` presentation now shares the same conservative derivation. No protocol/generated artifact or foundation assertion changed; `docs/UX.md` now matches shipped behavior.

The carried nit is also closed: CLI diagnostics table-drive `UNSPECIFIED` and unknown numerics across deduplication strength, reconciliation strength, and unproven-outcome action.

Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the focused cross-surface review fix). Review weight remains caller-selected `thorough`; pass 2 is delegated to the parent autopilot driver.

Mutation evidence: replacing the shared qualifier derivation with an implementation that ignored the manifest and always returned `manual-required` failed the focused operator-domain regression (`expected 'unknown', actual 'manual-required'`). `git restore operator-domain/src/reconciliation/outcome_qualifier.ts` restored the staged implementation, after which focused operator-domain, web, and CLI tests passed.

Full verification:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace targets, tests, doctests, and warnings-denied clippy passed; the server suite included 84 unit tests.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 59 vectors, 19 promoted vectors, 29 implementation checks, 38 killed mutation witnesses, and generated drift clean.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 28/28 tests.
4. `cd pi-adapter && npm test`: **PASS** — 61/61 tests, including the real core/adapter loop.

Consumer suites: `cd web-cockpit && npm test`: **PASS** — 144/144; `cd cli && npm test`: **PASS** — 53/53 plus the real-core resource projection; `cd token-commune-adapter && npm test`: **PASS** — 63/63, including both real-core flows. `cargo fmt --all -- --check`, staged/unstaged `git diff --check`, and the focused post-mutation restores also passed.
