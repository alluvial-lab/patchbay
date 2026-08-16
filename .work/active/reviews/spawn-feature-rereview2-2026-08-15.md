---
id: spawn-feature-rereview2-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn
created: 2026-08-15
updated: 2026-08-15
---

# Integrated feature re-review pass 3 — spawn logical-target and generation lifecycle

## Verdict

**CLEAN.** The remaining pass-2 MATERIAL is closed, all pass-1 MATERIAL closures remain intact, and no new material boundary hole appears in the r2 diff (`7624b52..98b974d`, implementation commit `98b974d`). This thorough convergence pass supports advancing `research-handoff-spawn` to `done`.

Review mode: independent fresh-context feature convergence review, effective weight `thorough`, pass 3. The prior integrated assessment and pass-2 closure matrix stand; this pass was bounded to the r2 fix, focused lifecycle probes, regression spot-checks, and the requested full suite. No temporary worktree, code change, or story-file change was made.

## Findings

No BLOCKER, MATERIAL, or NIT finding survived this pass.

## Remaining MATERIAL closure confirmation

`CommandInspection.spawn_claim_disposition` is a direct generated `SpawnClaimDisposition`, appended as protobuf field 8. `DiagnosticsProjection::inspect_command` copies the canonical claim fold's current value and uses `UNSPECIFIED` only when no spawn claim exists; it does not derive claim state from `CommandState` or synthetic failure history.

All five exact dispositions are covered across the relevant hot/replay boundaries:

- `active`, `poisoned_pending_reconciliation`, and `released_no_external_effect`: direct diagnostics hot fold and fresh replay projection;
- `promoted`: hot service projection, bounded-as-of projection, and reopened-core restart projection;
- `target_abandoned`: hot projection and storage-backed restart replay.

The CLI imports the same generated enum and renders every value as canonical lower-snake-case in JSON and under `CLAIM_DISPOSITION` in text. Its five-case test covers active, poisoned, released, promoted, and target-abandoned; absent/non-spawn state remains `null`/`-` through the generated unspecified sentinel.

The requested independent poison → inspect probe passed:

```text
cargo test -p patchbay-core --test spawn_claim_registry \
  command_inspection_projects_exact_claim_disposition_hot_and_restart -- --exact
# 1 passed; 0 failed
```

## Proto and generated-contract check

**PASS.** `diagnostics.proto` adds one append-only field and the required `sessions.proto` import, with naming consistent across protobuf (`spawn_claim_disposition`), generated Rust, generated TypeScript (`spawnClaimDisposition`), core, web, and CLI. Rust and TypeScript artifacts regenerate without diff. Drift, vectors, models, TypeScript build, presentation conformance, and presentation meta-tests are green: 57 vectors, 17 promoted vectors, 26 implementation checks, 38 mutation witnesses, and 54 model-promotion blocks.

## Closed-MATERIAL regression spot-checks

- **M2 cockpit disposition independence — PASS.** Focused poison→cancelled/expired/failed, reconnect replay, and warning-render tests passed (3/3). The full cockpit suite passed 141/141, retaining claim poison independently from terminal command failure and clearing retry risk only on a later disposition.
- **M3 abandonment lifecycle — PASS.** The focused atomic abandonment test passed, including fence clearing, target retirement, exact claim disposition, restart equality, and retry idempotence. Workspace abandonment tests, cockpit retirement/action behavior, and the typed CLI action also passed in the full suites.
- **M4 retained reviews — unchanged/closed.** The r2 diff does not touch or undermine the four retained retrospective review artifacts.

The concrete Pi spawn supervisor remains deferred by the already-settled feature boundary and was not reopened.

## Full suite results

All four prescribed verification groups plus web cockpit and CLI passed:

1. **Rust group:** `cargo fmt --all -- --check`; workspace all-target build; workspace tests; warnings-denied all-target clippy — **PASS**. This includes 40 spawn-claim tests, 12 spawn-completion tests, and both core and server abandonment suites.
2. **Contracts group:** generated drift, vectors, models, TypeScript build, presentation conformance, and presentation meta-tests — **PASS**.
3. **Operator-domain group:** build and tests — **PASS, 27/27**.
4. **Pi-adapter group:** build and tests — **PASS, 38/38**, including the real core/AgentSession generation-bump, reconnect, and restart loop.
5. **Web cockpit:** browser/type build and tests — **PASS, 141/141**.
6. **CLI:** build/tests and real-core resource projection — **PASS, 50/50** plus the process projection probe; core smoke — **PASS**.
7. **Hygiene:** `git diff --check`, generated-path diff, tracked status, and disk check — **PASS**; the tree was clean before writing this review and `/` retained 55G free.

A first focused web command was invoked from the repository root and stopped before testing because the repository intentionally has no root `package.json`; the correctly scoped `web-cockpit` rerun passed. This was a command-location error, not a product failure.

## Recommendation

**Advance `research-handoff-spawn` from `review` to `done`.** The thorough convergence requirement is satisfied: pass 3 found no current-cycle material blocker in the r2 boundary fix, and the required verification is green.
