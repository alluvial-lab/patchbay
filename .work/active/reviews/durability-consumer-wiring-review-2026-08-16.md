---
id: durability-consumer-wiring-review-2026-08-16
kind: story
stage: done
tags: [review, adapter]
parent: capability-manifest-durability-and-reconciliation-depth-consumer-wiring
created: 2026-08-16
updated: 2026-08-16
---

# Thorough review — Assurance-manifest consumer wiring

## Verdict

**NITs** — advance `capability-manifest-durability-and-reconciliation-depth-consumer-wiring` to `done`.

Commit `7b26cd3` cleanly carries the generated assurance manifest through canonical redacted diagnostics, declares conservative evidence-backed Pi and token-commune profiles, and combines the canonical failure term with V1 deduplication in the web retry presentation. No material advisory-rule violation, redaction leak, uncertainty-as-true declaration, replay divergence, authority translation, or current product defect survived the pass. One low-risk focused-test coverage nit remains.

Review mode: independent fresh-context story review, effective weight `thorough`, one rigorous pass, implementation range `5289bec..7b26cd3`.

## Findings

### Nit

1. **The CLI's enum-sentinel rejection is correct but not pinned by its focused test** (`cli/src/commands/diagnostics.ts:515-523`; `cli/tests/output-diagnostics.test.ts:431-456`). `requiredGeneratedEnumLabel` correctly rejects `UNSPECIFIED` and unknown enum numerics, but the committed fail-closed test covers a missing V1 branch and one omitted optional boolean only. Temporarily removing the `value === 0` guard left both named CLI assurance tests green. A clean-tree direct probe confirmed that production currently rejects an unspecified deduplication strength, so this is not a current behavior defect. **Concrete fix:** when this test area is next touched, table-drive `UNSPECIFIED` and unknown numerics for `deduplication_strength`, `reconciliation_strength`, and `unproven_outcome_action` through `adapterStatusPageView` and assert rejection.

No blocker, material, or important findings.

## Checklist disposition

- **Diagnostics carriage — PASS.** `AdapterCapabilitySummary` reserves the removed tag/name and carries `AdapterAssuranceManifest` once at tag 14. `DiagnosticsProjection` retains `AdapterRecord`, revalidates registrations under Replay, and emits `validated_capability.assurance().to_wire_v1()`. The raw replay-only tag, attachment descriptor bytes, and opaque profile bytes are absent from the generated diagnostics shape.
- **Advisory rule — PASS.** The web retry path first classifies the canonical failure and only then reads `assurance.v1.deduplication_strength`; a maximal capability on `cancelled` produces no retry decision. Pre-execution failures retain their canonical safe-to-retry treatment. No assurance field enters Grant matching, Operation completion, or delivery suppression.
- **Emitters — PASS.** Pi declares Patchbay-boundary deduplication, three explicit `false` evidence flags, reconciliation `none`, and `manual_required`. token-commune declares deduplication/reconciliation `none`, all evidence flags `false`, and action `none`; its partial/latest-50 visibility is not promoted to cursor or outcome authority. Exact constructor tests pin every V1 field.
- **Surfaces — PASS.** CLI JSON and human tables show generated dimension names and only `unknown` / `manual-required`; raw secret-bearing bytes remain unavailable. Web diagnostics narrow the generated V1 branch once, fail closed on incomplete values, and pass the owning adapter's assurance into session and resource retry presentation without rewriting canonical outcomes.
- **Docs — PASS.** `ARCHITECTURE`, `PROTOCOL`, `VERIFICATION`, `UX`, and `GLOSSARY` state the current generated registry, conservative declarations, advisory boundary, canonical diagnostics, and qualifier semantics accurately. `ADAPTER-PI` and `RUNBOOK` are consistent with those assertions.
- **Server lifecycle paths — PASS.** The authenticated regression covers fresh attach, exact same-generation redeclaration, newer-generation replacement, token invalidation, replay, and one registration append per accepted declaration. The adjacent invalid-assurance regression proves no registration append, no replacement token, and preservation of the prior current token.

## Mutation matrix

Each mutant was applied alone on the main tree, run with a focused oracle, reverted with `git restore`, and followed by a clean status check. Clean focused confirmations passed after restoration.

| Mutation or probe | Focused oracle | Result |
|---|---|---|
| Re-source diagnostics deduplication from raw replay-only tag 7 instead of the canonical validated assurance | `adapter_projection_uses_canonical_assurance_and_redacts_raw_fields` | **KILLED** — diagnostics exposed numeric `777` instead of normalized `none`; exit 101 |
| Remove the canonical-failure guard so maximal capability alone creates a retry decision | `retry safety combines canonical failure with generated assurance and never capability alone` | **KILLED** — `cancelled` incorrectly rendered safe-to-retry; exit 1 |
| Promote Pi `cursor_support` from uncertain `false` to `true` | `Pi manifest declares one complete conservative assurance V1` | **KILLED** — exact V1 object comparison failed; exit 1 |
| Fresh probe: accept enum sentinel `0` in the CLI generated-enum labeler | the two focused CLI assurance tests | **SURVIVED** — tests cover missing branch/boolean but not sentinel enums; recorded as the nit above |
| Clean direct probe: pass `UNSPECIFIED` deduplication through `adapterStatusPageView` | one-off production-path probe after restoration | **PASS** — production rejected with `unknown or unspecified deduplication strength` |

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace targets, tests, doctests, and warnings-denied clippy passed, including 84 server unit tests.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 59 vectors, 19 promoted vectors, 29 executed implementation checks, and 38 killed registered mutation witnesses.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 27/27 tests.
4. `cd pi-adapter && npm test`: **PASS** — 61/61 tests, including the real-core loop.
5. `cd web-cockpit && npm test`: **PASS** — 143/143 tests.
6. `cd cli && npm test`: **PASS** — 51/51 unit tests plus the real-core resource projection.
7. `cd token-commune-adapter && npm test`: **PASS** — 63/63 tests, including both real-core flows.

`cargo fmt --all -- --check` and `git diff --check` passed. The tracked tree was clean before review mutations, after every restoration, before and after the full suite, and immediately before writing this review. Disk discipline was observed without a temporary worktree; `/` retained 54G free.

## Recommendation

**Advance to `done`.** The required consumer-wiring behavior is correct, all material preferred mutants were killed, and the full clean-tree suite is green. The surviving CLI sentinel-test probe is a narrow defense-in-depth coverage nit, not a current-cycle blocker.
