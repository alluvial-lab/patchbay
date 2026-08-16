---
id: spawn-feature-rereview-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn
created: 2026-08-15
updated: 2026-08-15
---

# Integrated feature re-review — spawn logical-target and generation lifecycle

## Verdict

**MATERIAL — do not advance `research-handoff-spawn` to `done`.**

Pass 2 confirms that promotion-backed command completion, independent cockpit retry risk, the authenticated target-abandonment lifecycle, and the four retrospective contract-leaf reviews are real. The pass-1 core-spine assessment also remains intact: the focused regressions and full clean-tree suite found no authority bypass, replay divergence, duplicate generation publication, claim/fence regression, or cursor replacement regression.

One operator-boundary part of MATERIAL 1 is still not closed. Diagnostics now tracks claim disposition internally and emits an overloaded command-history marker, but `CommandInspection` and `patchbay-cli inspect-command` do not expose the actual generated `SpawnClaimDisposition`. The operator therefore still cannot inspect whether the claim is active, poisoned, released, promoted, or target-abandoned.

Review mode: independent fresh-context feature re-review, effective weight `thorough`, convergence pass 2, diff `b84af41..6e6ccf0`. The main tree was used for focused probes and one-at-a-time mutations; every mutation was immediately reverted with `git restore`. No temporary worktree was created. No code or story file was modified by this review.

## Findings

### MATERIAL 1 — command inspection still drops the exact claim disposition

**Locations:** `core/src/diagnostics/mod.rs:92-97,582-600,855-864`; `contracts/proto/patchbay/diagnostics.proto:161-177`; `cli/src/commands/diagnostics.ts:328-339,501-510`.

`DiagnosticsProjection` now correctly retains `CommandTimeline.claim_disposition`, validates changes through the shared adjacency registry, and updates it on promotion. However, `inspect_command` does not copy that field into `CommandInspection`, because the generated message has no claim-disposition field. The CLI consequently renders only command state/failure history.

The substitute history entries are not an inspectable disposition contract. Poison and target abandonment both appear as the unchanged command state plus `execution_outcome_unknown`; release appears as the unchanged state plus an unspecified failure; and the entries carry no discriminator saying they are claim events. This cannot distinguish all five generated dispositions or reliably state the current one. The private Rust field proves the fold knows the answer, not that the operator can inspect it.

This is material because claim state is deliberately independent from `CommandState` and determines whether another generation is unsafe, whether the replacement fence remains, and whether the last-resort abandonment action is applicable. The web cockpit now carries that distinction, but the pass-1 diagnostic/CLI boundary remains incomplete.

**Required direction:** add the generated current `SpawnClaimDisposition` (or an equivalently explicit typed claim-lifecycle projection) to `CommandInspection`, render it in both JSON and text CLI output, and assert exact active/poisoned/released/promoted/target-abandoned visibility across hot, bounded-as-of, and restart inspection. Do not encode claim state as a synthetic command failure.

No other blocker, material finding, or nit survived this pass.

## Pass-1 MATERIAL closure matrix

| Pass-1 MATERIAL | Status | Re-review evidence |
|---|---|---|
| **M1 — promotion and claim-aware diagnostics** | **PARTIAL — not closed** | Promotion completion is genuinely fixed: the focused end-to-end spawn test showed hot, exact bounded-as-of, and restarted inspection at `completed`, with the exact `SpawnPromotionCommitted` event as terminal/history source. Making the promotion diagnostics arm inert failed that test (`delivered` instead of `completed`). Exact claim disposition is still private/dropped per the finding above. |
| **M2 — independent cockpit claim retry risk** | **CLOSED** | `CommandView.spawnClaimDisposition` is independent from `failureCode`; poison→cancelled/expired/failed retained `poisoned_pending_reconciliation`; proved-none release cleared only retry risk; promotion set `promoted`; reconnect replay matched. The focused three-sequence probe and promotion probe passed, and the full cockpit suite passed 141/141. |
| **M3 — authenticated atomic target abandonment** | **CLOSED** | `AbandonSpawnTarget` verifies the compound issuer, derives exact durable claim scope, requires live `session-management` authority, samples one decision time, and calls the sole atomic source+audit writer. Happy path, unauthenticated/wrong-kind/wrong-target/revoked/expired denial, exact retry, wrong target/scope, generic-route rejection, target retirement, current/candidate audit-only retention, claim consumption, fence clearing, restart equality, and no revival passed. Removing the Grant check made the denial test accept unauthorized abandonment; removing the retired-target reservation guard made the restart/no-revival test fail. The RPC naming and shape are coherent: the request selects claim/target/reason but carries no self-asserted issuer or Grant, and the result returns durable decision/audit ids and selected provenance. Generated drift and vector checks are green. |
| **M4 — retained independent contract-leaf reviews** | **CLOSED** | All four committed artifacts exist: Leaf 1 (`4667dfc`), Leaf 2 (`4eb4ccd`), Leaf 3 (`a064bab`), and Leaf 5 (`6e6ccf0`). Each explicitly disclaims reconstruction of missing prior claims, identifies landed/current review scope, uses fresh-context retrospective completeness plus adversarial mutation matrices, records one-at-a-time main-tree restores, and reaches CLEAN with clean four-group evidence. |

## Regression spot-checks

| Previously passing contract | Result |
|---|---|
| Continuation compound authority | **PASS.** Exact-prior replacement-Grant selection/replay and expired, revoked, wrong-subject, wrong-endpoint, and wrong-generation rejection remained green; broad spawn authority alone remains insufficient. |
| Poison across restart | **PASS.** `reconciled_ambiguity_poisons_once_survives_restart_and_suppresses_relaunch` and server stream-loss/restart coverage remained green. |
| Cursor authoritative replacement | **PASS.** Known-suffix idempotence, omitted stale-member removal, exact atomic replacement publication, and CAS race coverage all remained green in operator-domain. |

The explicitly downstream concrete Pi spawn supervisor remains deferred by feature scope and is not a finding here.

## Focused probes and mutation sensitivity

- Promotion inspection probe: `cargo test -p patchbay-core-server --test spawn_completion adapter_scoped_delivery_result_report_restart_and_descendant_submit` — **PASS** on the restored tree. An inert `SpawnPromotionCommitted` diagnostics arm was **KILLED**, exit 101 (`delivered` rather than `completed`).
- Cockpit retry-risk probe: the poison→cancelled/expired/failed, proved-none release, reconnect replay, and post-poison promotion tests — **PASS**.
- Abandonment focused suites: `cargo test -p patchbay-core-server --test spawn_target_abandonment` and `cargo test -p patchbay-core --test spawn_target_abandonment` — **PASS**, 2/2 each.
- Grant-check removal mutant — **KILLED**: the wrong-kind/target/revoked/expired denial test received a successful unauthorized abandonment.
- Retired-target revival mutant — **KILLED**: the restart/no-revival test observed `Ok(())` instead of `RetiredTarget`.
- After each mutant, `git restore --worktree` plus a path-scoped quiet diff confirmed exact restoration before the next probe.

## Full suite results

All requested four standard groups plus web cockpit and CLI passed; the broader pass-1 suite was also rerun:

1. **Rust standard group:** `cargo fmt --all -- --check`; workspace all-target build; workspace tests; warnings-denied all-target clippy — **PASS**.
2. **Rust property run:** `PROPTEST_CASES=256 cargo test --workspace --features proptest` — **PASS**.
3. **Formal:** `./formal/run-model-checks.sh` — **PASS, 20/20**.
4. **Contracts standard group:** generated drift, vectors, models, TypeScript build, presentation conformance, and presentation meta-tests — **PASS**: 57 vectors, 17 promoted, 26 implementation checks, 38 mutation witnesses, 54 promotion blocks, five presentation registries.
5. **Operator-domain standard group:** build/tests — **PASS, 27/27**.
6. **Pi-adapter standard group:** build/tests — **PASS, 38/38**, including the real AgentSession/core generation-bump, reconnect, and core-restart loop.
7. **Web cockpit:** browser/type build and tests — **PASS, 141/141**.
8. **CLI:** build/tests, real-core resource projection, and core smoke — **PASS, 49/49** plus both process probes.
9. **Web server:** build/tests and authenticated real-core smoke — **PASS, 32/32** plus smoke.
10. **token-commune adapter:** build/tests — **PASS, 63/63**.
11. **Composed E2E:** walking skeleton — **PASS**.
12. **Hygiene:** `git diff --check`, full tracked diff, and status were clean after all probes; `/` had 55G free before and after.

Green verification does not close the finding because neither the diagnostic wire shape nor the CLI output contains the exact claim disposition; existing tests assert only overloaded history evidence.

## Recommendation

**Return the feature with the narrow M1 operator-diagnostics scope above.** Preserve M2, M3, M4, and all already-passing core lifecycle rows. After adding explicit generated claim-disposition inspection and hot/bounded/restart CLI assertions, rerun the focused diagnostics probe and the required thorough convergence pass. Do not advance `research-handoff-spawn` to `done` on this pass.
