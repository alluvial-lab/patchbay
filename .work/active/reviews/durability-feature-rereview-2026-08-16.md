---
id: durability-feature-rereview-2026-08-16
kind: story
stage: done
tags: [review, adapter]
parent: capability-manifest-durability-and-reconciliation-depth
created: 2026-08-16
updated: 2026-08-16
---

# Thorough feature re-review — capability-manifest durability and reconciliation depth

## Verdict

**CLEAN** — the pass-1 material finding is closed at `5ddd677` and the feature has converged.

This was independent fresh-context feature re-review pass 2 at effective weight `thorough`, constrained to the pass-1 `SubmissionOutcome.UNKNOWN` finding, its claimed CLI sentinel-coverage closure, one prior-PASS seam spot-check, and full integrated verification. Converged pass-1 rows were not reopened. Disk discipline was observed: `/` had 54G free at start, no temporary worktree was created, both probes were restored with `git restore`, and the tracked tree was clean before this review record.

## Findings

No blocker, material, important, or nit findings.

## Convergence evidence

### UNKNOWN-outcome qualifier — PASS

The shared operator-domain derivation in `operator-domain/src/reconciliation/outcome_qualifier.ts` validates and consumes the generated `AdapterAssuranceManifestV1.unproven_outcome_action`. For `SubmissionOutcome.UNKNOWN`, `ReconciliationAction.NONE` maps to `unknown`, `MANUAL_REQUIRED` maps to `manual-required`, and absent/malformed/unavailable assurance maps conservatively to `manual-required`. Every non-UNKNOWN submission outcome returns no qualifier, so a capability declaration alone cannot create a presentation decision.

The web cockpit retains the submitted Operation's adapter id, resolves its canonical diagnostics capability, and uses the shared derivation for typed and transport-inferred UNKNOWN feedback. The CLI performs its best-effort adapter-status lookup only after an UNKNOWN adapter-targeted submission, passes the returned generated capability to shared presentation, and conservatively falls back when lookup or declaration evidence is unavailable. Accepted `execution_outcome_unknown` presentation uses the same conservative action derivation. The advisory boundary remains intact: none of these paths authorizes, terminalizes, suppresses delivery, or supplies an execution outcome.

- **Required pass-1 probe re-injected:** replaced the shared action derivation with unconditional `manual-required`; focused `operator-domain` regression failed as required (`expected 'unknown', actual 'manual-required'`). Restored with `git restore`.
- **Fresh pass-2 probe:** removed the `SubmissionOutcome.UNKNOWN` gate so capability could qualify an accepted outcome; the same focused regression failed as required (`capability alone cannot qualify a proven submission outcome`, expected `undefined`, actual `manual-required`). Restored with `git restore`.
- **Post-restore focused run:** `operator-domain` outcome-qualifier test passed.

### CLI sentinel coverage — PASS

`cli/tests/output-diagnostics.test.ts` now table-drives both `UNSPECIFIED` and unknown numeric rejection for all three generated assurance enums: deduplication strength, reconciliation strength, and unproven-outcome action. The focused cases and the complete CLI suite pass.

### Prior PASS spot-check: Pi-profile readiness — PASS

`pi-adapter/src/core_client.ts` constructs the current declaration from generated assurance types, keeps continuation proof, cursor, and generation fence explicitly false, keeps reconciliation at `NONE`, and declares the conservative manual action. The dependent Pi manifest-profile checkpoint imports this generic registry once, retains Pi-only vocabulary in its opaque generated profile seam, and defers stronger activation to lifecycle conformance. No parallel durability/reconciliation registry or Pi-specific core assurance branch was introduced by the r1 fix.

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace builds, tests, doctests, and warnings-denied clippy passed; the server unit suite included 84 tests.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — generated drift clean; 59 vectors, 19 promoted vectors, 29 implementation checks, and 38 killed mutation witnesses.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 28/28 tests.
4. `cd pi-adapter && npm test`: **PASS** — 61/61 tests, including the real core/adapter loop.
5. `cd web-cockpit && npm test`: **PASS** — 144/144 tests.
6. `cd cli && npm test`: **PASS** — 53/53 unit tests plus the real-core resource projection.
7. `cd token-commune-adapter && npm test`: **PASS** — 63/63 tests, including both real-core flows.

`cargo fmt --all -- --check` and `git diff --check` also passed. The tracked tree remained clean after both restored probes and the full suite.

## Recommendation

**Approve as converged.** The r1 fix closes the sole pass-1 material finding, the carried CLI sentinel nit is covered, the sampled Pi-profile seam remains ready, and thorough pass 2 found no material current-cycle blocker.
