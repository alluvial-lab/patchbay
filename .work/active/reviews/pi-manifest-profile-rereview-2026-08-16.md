---
id: pi-manifest-profile-rereview-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-manifest-profile
created: 2026-08-16
updated: 2026-08-16
---

# Thorough rereview — opaque Pi runtime profile semantic oracle

## Verdict

**CLEAN** — the pass-1 MATERIAL is closed at fix commit `a451b83`.

The convergence-scoped review found no remaining material, blocker, or nit. The new generated-profile fixtures cross the protobuf boundary, retain four representative semantic violations after wire round-trip, and kill complete bypass of `validatePiRuntimeProfile`. A distinct fresh semantic probe and a prior conservative-activation mutant were also killed. Previously converged profile opaqueness, vocabulary placement, assurance consumption, generated contracts, and documentation were not reopened.

Review mode: independent fresh-context rereview, effective weight `thorough`, pass 2, fix commit `a451b83`.

## Findings

None.

## Convergence evidence

- **Validator-bypass oracle:** removing the complete `validatePiRuntimeProfile(profile)` call and rebuilding caused the focused three-test profile selection to fail: 2 passed, 1 failed. The semantic-invalid test reported `Missing expected exception: unspecified scalar`.
- **Four decodable-but-invalid fixtures:** under the bypass mutant, a direct generated-contract probe round-tripped and decoded all four payloads while preserving their invalid state: `transport=UNSPECIFIED`; missing `sessionDurability`; duplicate `SKILL` resource; and omitted `CONTEXT_FILE` resource. Thus the fixtures are valid protobuf encodings and exercise adapter-owned semantics rather than framing failure.
- **Fresh probe:** adding an extra `UNSPECIFIED` live-event caveat survived protobuf round-trip and was rejected by the clean decoder with `invalid live event caveats`.
- **Prior-kill spot check:** changing `sessionReplacementSupport` from `false` to `true` made the focused conservative-manifest test fail with `true !== false`.
- Every production mutant was independently restored with `git restore`. The tracked tree was clean before the full suite.

## Mutation matrix

| Mutation / probe | Focused oracle | Result |
|---|---|---|
| Remove `validatePiRuntimeProfile(profile)` | Build plus profile-focused Node test selection | **KILLED** — semantic-invalid test failed; 2 passed, 1 failed. |
| Round-trip each of the four review fixtures while the validator is bypassed | Direct generated `fromBinary` / `toBinary` / `fromBinary` probe followed by bypassed decode | **CONFIRMED** — all four decoded and retained the intended invalid semantic state. |
| Fresh: append `PiLiveEventCaveat.UNSPECIFIED` | Direct clean-decoder probe after generated wire round-trip | **KILLED** — rejected as invalid live-event caveats. |
| Prior kill: set `sessionReplacementSupport=true` | Focused conservative Pi manifest test | **KILLED** — declaration-gate assertion failed. |

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 59 vectors, 19 promoted vectors, 29 implementation checks, and 38 mutation witnesses.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 28/28.
4. `cd pi-adapter && npm test`: **PASS** — 63/63, including the real-core loop and the new semantic-invalid oracle.
5. `cd web-cockpit && npm test`: **PASS** — 144/144.
6. `cd cli && npm test`: **PASS** — 53/53 plus the real-core resource projection.
7. `cd token-commune-adapter && npm test`: **PASS** — 63/63, including both real-core flows.

`git diff --check` passed. Root filesystem availability was 54 GiB before and after review; no temporary worktree was created.

## Recommendation

**Approve.** The thorough convergence requirement is satisfied: pass 2 found no material current-cycle blocker. The active workflow driver may advance `research-handoff-pi-adapter-capability-manifest-profile` from `review` to `done`.
