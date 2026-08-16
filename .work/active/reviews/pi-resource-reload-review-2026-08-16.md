---
id: pi-resource-reload-review-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-resource-reload-rehydration
created: 2026-08-16
updated: 2026-08-16
---

# Thorough review — Pi Unit 5 idle-only bounded reload and rehydration

**Verdict: MATERIAL.** Commit `fa5be0f` substantially closes the active-work race, requires strict materialization and two correlated markers, proves a new challenged extension epoch, rebinds subscriptions, reconciles the authoritative cursor before success, keeps process/session/generation identity stable, and blocks unmanaged sessions. Two current-cycle issues remain: the persisted post-effect recovery branch can lose its mandatory ambiguity classification, and the claimed Pi `/dist` boundary has no non-vacuous real-process oracle.

Review mode: independent fresh-context delegated story review, effective weight `thorough`, one rigorous pass over `4599156..fa5be0f`. No subagent was needed or attempted. No temporary worktree was created.

## Findings

### MATERIAL 1 — Post-effect recovery can fall through as a proved execution failure

**Locations:** `pi-adapter/src/reload_controller.ts:193-211`; `pi-adapter/src/main.ts:1128-1132`

When a prior attempt already left one physically materialized matching request/completion pair, `recoverPersistedReload` correctly chooses rehydration without calling `ctx.reload()` again. However, the recovery branch calls `markRehydrating()` outside the ambiguity-conversion `try/catch` used by the new-command branch. If that report/rebind-preparation callback fails, the controller leaks the raw error. `classifyDeliveryFailure` then applies its generic fallback, `EXECUTION_FAILED / delivery_execution_failed`, even though reload is proven possible (indeed, both post-effect markers are present). The adjacent marker-conflict catch has the same masking risk: a failure from `markRehydrating()` can replace the already-derived `PiReloadAmbiguousError`.

A temporary clean-tree regression probe preinstalled the exact persisted pair, made `markRehydrating()` fail, and required `PiReloadAmbiguousError`. It failed: the controller returned raw `Error("session report failed")`. This violates the story's post-effect rule and can make retry guidance less honest.

**Concrete fix:** once any persisted reload marker/effect evidence exists, wrap both stale-state reporting and rehydration so every non-rejection failure remains `PiReloadAmbiguousError`; do not let a reporting failure overwrite an existing ambiguity. Add a focused recovery regression plus delivery-classification assertion for `EXECUTION_OUTCOME_UNKNOWN` and stale connectivity. Preserve the current no-second-`ctx.reload()` behavior.

### MATERIAL 2 — The `/dist` process-replacement claim has a vacuous test label, not the required real-process proof

**Locations:** `pi-adapter/tests/reload_controller.test.ts:85-179,312-320`; `.work/active/stories/research-handoff-pi-adapter-capability-resource-reload-rehydration.md:80,95,102`; `pi-adapter/src/core_client.ts:724-740`

The real offline Pi test changes only a temporary TypeScript entrypoint and its temporary `.mjs` transitive dependency. It correctly proves entrypoint `A→B` and cached dependency `A`, but it never changes or probes a Pi/runtime installed-package `/dist` alias. The separately titled `unknown/transitive or runtime-dist scope` test sends only numeric enum value `999` and asserts `UNKNOWN_SCOPE`; it exercises neither a transitive-dependency selector nor runtime `/dist`. Thus the profile's process-replacement declaration is honestly narrow and source-grounded, but the checked acceptance statement that Pi/runtime `/dist` “remains old” is not implementation-checked by the named real-process oracle. The story's broad-scope mutation evidence is correspondingly overstated.

**Concrete fix:** extend an isolated real-process fixture to change a runtime-package `/dist` probe while the child stays alive, reload, and independently assert that the running alias remains the old value while the entrypoint refreshes. Keep the existing transitive `.mjs` assertion. If an isolated `/dist` mutation cannot be made safe and deterministic, remove the implementation-checked claim and record the boundary only as loader-source/profile evidence; do not retain the current test title or mutation claim as proof.

## Checklist disposition

| Requirement | Result |
|---|---|
| Complete idle admission and check-to-command serialization | **PASS.** The exclusive gate checks pending direct RPC/action reservations, current process/runtime identity, conflicting delivery, streaming, compaction, queue depth, and start/retry/compaction settlement epochs before marker/effect. Later stdin actions queue behind the same owner. |
| Materialized two-marker/new-epoch rehydration | **PASS with Material 1 exception.** Strict raw/RPC tree validation, exact marker correlation, challenged new epoch, subscription rebind, cursor acknowledgement, and no blind second reload are present; the post-effect reporting-failure classification is wrong. |
| Scope honesty | **PARTIAL / MATERIAL evidence gap.** The profile explicitly lists entrypoint + Pi-enumerated resources and process-replacement exclusions, and the real process proves a cached transitive dependency. `/dist` is not exercised. |
| Identity/generation invariants | **PASS.** PID, process token, logical target, runtime session, continuity scope, and generation remain unchanged; reload does not enter supervisor/journal generation transitions. |
| Managed-session boundary | **PASS.** `main.ts:763-775` requires the current `RpcPiSession`, registry entry, logical target, and session root before controller construction. Production preprovisioning cannot manufacture a managed logical target outside journal/promotion recovery. |
| Non-entrypoint enumerated resources | **PASS as a bounded declaration.** Skills, prompts, themes, and context files are named as Pi loader-enumerated profile values; no arbitrary dependency-graph refresh is claimed. |

## Mutation matrix

Every mutation was applied on the main tree, run with a focused test, and reverted with `git restore`. The tracked tree was clean after restoration.

| Mutation / probe | Result | Focused oracle |
|---|---|---|
| Remove streaming admission rejection | **KILLED** — streaming reload completed instead of rejecting | busy-admission reload-controller test |
| Remove physical request-marker verification before `ctx.reload()` | **KILLED** — in-memory request reached reload | control-extension in-memory-marker test |
| Remove completion-epoch/new-handshake equality check | **KILLED** — old-epoch handshake reported success | marker/epoch correlation test |
| Fresh stale-settlement mutant: after a settled activity, treat any historical settled epoch as sufficient for a later `auto_retry_start` | **KILLED** — pending retry was admitted | temporary prior-start → settled → retry probe in the busy-admission test |
| Clean negative probe: persisted completed reload + failing `markRehydrating()` must remain ambiguous | **SURVIVED / GAP** — raw `Error` escaped instead of `PiReloadAmbiguousError` | temporary recovery classification test; Material 1 |
| Inspect `/dist` scope oracle | **GAP** — real process changes only entrypoint + `.mjs`; numeric `999` test proves only unknown scope | test/source inspection; Material 2 |

## Full clean verification

All commands ran after restoring the clean implementation tree.

1. **Rust group:** `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. **Contracts group:** `npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 59 vectors, 31 implementation checks, and 38 registered mutation witnesses.
3. **Operator-domain group:** `npm run build && npm test` — **PASS, 32/32**.
4. **Pi-adapter group:** `npm test` — **PASS, 119/119**, including the real Pi reload process.
5. **Web cockpit:** `npm test` — **PASS, 148/148**.
6. **CLI:** `npm test` — **PASS, 53/53** plus the real-core resource projection.
7. **token-commune adapter:** `npm test` — **PASS, 63/63**, including both real-core flows.

`git diff --check` passed. The tracked tree was clean before this review file was written. `/` retained 53 GiB free; no temporary worktree was used.

## Recommendation

**Return `research-handoff-pi-adapter-capability-resource-reload-rehydration` to `implementing`.** Preserve the current admission gate, physical two-marker protocol, new-epoch handshake, managed-session restriction, subscription/cursor rehydration, and unchanged identity/generation behavior. Fix post-effect ambiguity preservation and replace the `/dist` scope label with a genuine real-process oracle (or explicitly downgrade that evidence claim), then rerun the thorough review before advancing to `done`.
