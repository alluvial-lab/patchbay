---
id: pi-resource-reload-rereview-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-resource-reload-rehydration
created: 2026-08-16
updated: 2026-08-16
---

# Thorough rereview — Pi Unit 5 idle-only bounded reload and rehydration

**Verdict: CLEAN.** Commit `9ba5b0e` closes both pass-1 MATERIAL findings. Persisted post-effect recovery preserves the closed `PiReloadAmbiguousError` / `execution_outcome_unknown` result even when stale-state reporting fails, and the real Pi process oracle now independently proves that the reloaded entrypoint changes while both a direct transitive module and an installed-package `/dist` alias remain cached. No material finding, scope overclaim, vacuous oracle, or nit remains in the convergence scope.

Review mode: independent fresh-context delegated story rereview, effective weight `thorough`, convergence pass 2 over fix commit `9ba5b0e` after the pass-1 baseline `70afe23`. No temporary worktree was created. All temporary mutations and the fresh test probe ran on the main tree and were reverted with `git restore` before full verification.

## Findings

None.

### Pass-1 disposition

- **Post-effect misclassification/raw-error leak — CLOSED.** Both a recovered matching pair and conflicting persisted evidence enter the post-effect ambiguity boundary. A `markRehydrating()` failure cannot replace the canonical ambiguity, expose its raw message, or cause a second `ctx.reload()` invocation. The delivery oracle fixes the external classification at `EXECUTION_OUTCOME_UNKNOWN`, stale connectivity, and the bounded `pi_reload_rehydration_outcome_unknown` diagnostic.
- **Missing `/dist` oracle — CLOSED.** The isolated real-process fixture resolves `@patchbay/reload-dist-probe` through a temporary `node_modules/.../dist/index.mjs`, rewrites it from A to B while the child remains alive, and observes `{ entrypointVersion: B, dependencyVersion: A, distVersion: A }` after reload. This is an executable installed-package alias/cache boundary, not a numeric unknown-enum label or a mutation of Pi's installed package.

## Mutation matrix

| Probe / mutation | Result | Focused oracle |
|---|---|---|
| Revert persisted recovery to the pass-1 raw `markRehydrating()` path outside ambiguity conversion | **KILLED** — focused regression failed because raw `Error` escaped instead of `PiReloadAmbiguousError`; that raw value follows the generic `execution_failed` fallback rather than the required ambiguity classification | `persisted post-effect reporting failures remain redacted execution ambiguity` |
| Simulate broad installed-package `/dist` refresh in the real-process fixture | **KILLED** — focused test failed with actual `distVersion: B` versus independent expected `A` | `real Pi reload refreshes the entrypoint but leaves transitive and installed-package dist artifacts old` |
| Remove streaming admission rejection | **KILLED** — streaming reload completed instead of rejecting before marker/effect | busy-admission reload-controller test |
| Treat any historical settlement as sufficient after a later `auto_retry_start` | **KILLED** — the later retry was admitted instead of returning `busy_unsettled` | busy-admission reload-controller test, permanent historical-settlement case |
| Fresh probe: produce a request-only ambiguous attempt, append a second conflicting same-command request, then recover again | **PASS** — both attempts remained `PiReloadAmbiguousError`; prompt count stayed at one, with zero handshakes and zero publications after the conflict | temporary focused reload-controller test |

After restoration, `git status --short` and `git diff --check` were clean before this review artifact was written.

## Full clean verification

1. **Rust group:** `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. **Contracts group:** `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 59 vectors, 31 implementation checks, and 38 mutation witnesses.
3. **Operator-domain group:** `cd operator-domain && npm run build && npm test` — **PASS, 32/32**.
4. **Pi-adapter group:** `cd pi-adapter && npm test` — **PASS, 120/120**, including the real Pi entrypoint/transitive/package-`/dist` oracle.
5. **Web cockpit:** `cd web-cockpit && npm test` — **PASS, 148/148**.
6. **CLI:** `cd cli && npm test` — **PASS, 53/53**, plus the real-core resource projection.
7. **token-commune adapter:** `cd token-commune-adapter && npm test` — **PASS, 63/63**, including both real-core flows.

`/` retained 53 GiB free after verification.

## Recommendation

**Approve the fix and allow the caller to advance `research-handoff-pi-adapter-capability-resource-reload-rehydration` from `review` to `done`.** Preserve the sanitized post-effect ambiguity boundary and the independent real-process package-`/dist` oracle. This reviewer intentionally did not modify the code or story file.
