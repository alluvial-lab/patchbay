---
id: pi-feature-rereview-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability
created: 2026-08-16
updated: 2026-08-16
---

# Pi managed-lifecycle feature re-review — pass 2

## Verdict

**MATERIAL**

The code, activation vector, journal behavior, mutation oracles, and integrated suites close the two prior BLOCKERs and the journal MATERIAL. One current-state sentence in the required three-document set still describes the pre-activation Pi assurance, so the documentation/manifest contract has not fully converged.

Review mode: independent fresh-context feature re-review, effective weight `thorough`, pass 2, convergence-scoped to fix commit `ce33f12` against prior review `.work/active/reviews/pi-feature-review-2026-08-16.md`.

## Findings

### MATERIAL 1 — the generic assurance evidence paragraph still describes pre-activation Pi

**Location:** `docs/VERIFICATION.md:319`

The paragraph says Pi “advertises only Patchbay-boundary deduplication and manual-required ambiguity.” The activated production manifest also advertises continuation proof, cursor support, generation fencing, and bounded reconciliation, as the corrected Pi-specific section at `docs/VERIFICATION.md:325-335`, `docs/PROTOCOL.md:752`, `docs/ADAPTER-PI.md:104`, the activation vector, and `pi-adapter/src/core_client.ts` now state.

This is stale current-state evidence prose in one of the three documents explicitly required to match the activated manifest. Roll this sentence forward to the complete activated V1 declaration; retain the distinction between authoritative transcript replacement and bounded adapter-wide Operation-outcome reconciliation.

No additional BLOCKER, MATERIAL, or NIT finding was found.

## Closure confirmation

### 1. Reconciliation honesty and activation version

- Production manifest, activation vector, constructor tests, E2E assertion, and the corrected Pi-specific protocol/verification sections declare `BOUNDED` plus `MANUAL_REQUIRED`.
- Authoritative language is confined to exact-set transcript-membership replacement inside its verified continuity scope. Launch/reload ambiguity remains `execution_outcome_unknown` and manual.
- No adapter-wide `AUTHORITATIVE` declaration remains in Pi production source, profile semantics, diagnostics carriage, or the corrected current-Pi sections. The only Pi-source `AUTHORITATIVE` injection is the deliberate mutation witness.
- The overclaim mutant was killed by the focused production-manifest oracle (mutation **23/28**).
- `pi-managed-lifecycle.v2` is coherent: the build-bound evidence set gained bounded-journal retention while the unchanged opaque profile wire schema and generic assurance contract correctly remain V1.

**Result:** prior reconciliation BLOCKER is closed in implementation and its dedicated contract text; Finding 1 is the remaining broader documentation drift.

### 2. Journal bounds and recovery

- Active journal records have a 64 MiB fail-closed bound; a staged payload above the former 2 MiB cliff is exercised.
- Full staged publication is retained through promotion, then compacted only after publication/cursor commit.
- Completed receipts are count/time pruned at 128 records / 24 hours on commit and startup; completed receipts are excluded from active recovery scans.
- Safely terminal pre-launch claims are abandoned only after core failure publication succeeds.
- `launch_attempted / may_exist` ambiguity and poison are not terminal receipts and are not eligible for abandonment or completed-history pruning.
- Restart/count/time pruning tests passed. The pruning-bypass mutant was killed (mutation **24/28**), and the safe-abandonment retention mutant was killed (mutation **25/28**).
- Fresh independent probe advanced an ambiguous claim beyond the configured retention window, invoked startup-style reconciliation/pruning, and confirmed the claim and its journal file remained present and poisoned.

**Result:** prior journal MATERIAL is closed.

### 3. Required docs

- `docs/PROTOCOL.md:752` correctly describes activated `spawn`/`pi-rpc`, replacement/continuation/cursor/fence support, bounded reconciliation, manual ambiguity, and strict materialization limits.
- `docs/ADAPTER-PI.md:97,104-106` correctly describes activated delivery, bounded-vs-authoritative scope, 64 MiB active records, publication-commit compaction, 24-hour/128-receipt pruning, safe abandonment, and ambiguous-claim retention.
- `docs/VERIFICATION.md:325-365` correctly labels the evidence implementation-checked/draft-vector only and names bounded reconciliation, journal retention, and both new mutants.
- `docs/VERIFICATION.md:319` remains stale as described in Finding 1.

### 4. Prior BLOCKER 6 / 9 / 10 spot checks

- **BLOCKER 6:** exact unknown-cursor replacement still removes omitted membership; cursor durability and skipped-publication mutants remain killed.
- **BLOCKER 9:** wrong initialized cwd with otherwise matching RPC path/id still fails the challenged marker proof; generic-RPC-trust mutant remains killed.
- **BLOCKER 10:** memory-only `require_resume` still rejects before termination or successor launch and safely abandons its no-effect journal; memory-only-resume mutant remains killed.

All three prior Pi-side closure rows remain closed.

## Verification results

| Verification | Result |
|---|---|
| Focused convergence tests | **PASS** — manifest honesty, journal retention, exact cursor replacement, wrong-cwd handshake, and memory-only resume refusal each passed as a single focused test. |
| Fresh ambiguous-retention probe | **PASS** — expiry did not purge the unresolved poisoned claim. |
| Pi mutation suite | **PASS** — **28/28 killed**, including authoritative-overclaim, pruning-bypass, and safe-abandonment-retention witnesses; sources restored and rebuilt. |
| Rust group | **PASS** — `cargo fmt --check`, workspace all-target build, full workspace tests/doctests, warnings-denied all-target clippy. |
| Contracts group | **PASS** — generated drift, 60 vectors / 19 promoted / 33 implementation checks / 38 registered mutation witnesses, model traceability, TypeScript build, presentation and presentation meta-tests. |
| Operator-domain group | **PASS** — **32/32**. |
| Pi-adapter group | **PASS** — **127/127**, including both real-core flows and the real Pi lifecycle. |
| Web cockpit | **PASS** — **148/148**. |
| CLI | **PASS** — **53/53** plus real-core resource projection. |
| token-commune adapter | **PASS** — **63/63**, including both real-core flows. |
| Hygiene | **PASS** — `git diff --check`; no Patchbay core or Pi RPC test process remained. |

## Recommendation

Return the feature for one bounded documentation correction at `docs/VERIFICATION.md:319`; no code or protocol-shape change is required. Because the effective review weight is `thorough`, run a final convergence re-review after that correction. Do not advance the feature to `done` on this pass.
