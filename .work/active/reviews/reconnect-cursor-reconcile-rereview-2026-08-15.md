---
id: reconnect-cursor-reconcile-rereview-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-reconnect-cursor-reconcile
created: 2026-08-15
updated: 2026-08-15
---

# Thorough rereview — Unit 10 reconnect and cursor convergence

## Verdict

**CLEAN** — advance `research-handoff-spawn-reconnect-cursor-reconcile` to `done`.

Pass 2 found no material findings or nits. The fix closes pass 1's stream-first continuity gap: the production `Reconciler` validates matching positive session/resource `core_generation` baselines and binds that storage-lineage anchor before both initial and clean-tail resumed subscriptions. The projection rejects unanchored cached prefixes and requires exact authority-domain/core-generation equality plus a strictly newer LSN before snapshot replacement.

Review mode: independent fresh-context story rereview, effective weight `thorough`, pass 2 over `d21cc52..58dcb1b` with focused source mutations and the full clean-tree suite.

## Findings

None.

## Checklist disposition

- **Lineage anchoring:** pass. Initial and resumed subscription turns each perform the two-view snapshot handshake before opening the stream. Session/resource baseline domains, response/payload LSN framing, positive generations, and generation equality validate before the presentation anchor changes. An unanchored non-empty prefix rejects, generation 99 cannot replace generation 7, and a resumed-only handshake-removal mutant is detected even when the handshake values are equal.
- **Clean-tail resumption:** pass. Legitimate same-lineage resumption receives LSN 1 then LSN 2 from cursors 0 then 1 without marking the model stale. The two snapshot reads per subscription turn are the documented cost of binding a stream whose event envelope carries no storage-continuity field.
- **Surface regression:** pass. Equal-LSN remembered events remain inert, equal-LSN snapshots reject, exact snapshot replacement deletes omitted cached N, and replacement is non-mutating on every rejected authority comparison.
- **External cursor authority:** pass. The sealed `AuthoritativeCursorReplacement` retains the verified external-continuity scope across Patchbay runtime generation, rejects cross-native continuity reuse, publishes one complete replacement, atomically installs exact membership/leaf/cursor/epoch, removes omissions, and exposes only old-stale or complete-new across commit failures.
- **Core and adapter authority:** pass. Continuation replay reconstructs the staged prefix and promotion deterministically; both live Grants and exact prior authority remain required; N tombstones only at N+1 promotion. Current attachment generation/token fencing remains independent from runtime generation, and abnormal stream detach degrades N without allocating or tombstoning N+1.
- **Oracle quality:** pass. Every requested mutant produced a focused failure at the intended production or sealed-port seam; no verdict relies only on an unchanged happy-path suite.

## Mutation matrix

Every mutant was applied alone on the main tree, run with a focused test, restored with `git restore`, and followed by a clean status check. No temporary worktree was created.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Restore the old optional cached-generation comparison, allowing an unanchored LSN-3 prefix to adopt generation-99 snapshots at LSN 9 | web model generation-99 regression | **KILLED** — the expected `no storage-lineage anchor` rejection disappeared. |
| Bind lineage only when `cursor === 0`, removing the handshake solely from resumed subscription turns while the baseline generation remains equal | web clean-tail resumption test | **KILLED** — only one lineage bind was observed; the required bind-before-every-turn assertion failed. |
| Accept an equal-LSN remembered stream event (`<=` → `<`) | web newer-core snapshot convergence test | **KILLED** — cached N was reinserted beside N+1. |
| Accept an equal-LSN snapshot (`<=` → `<`) | web newer-core snapshot convergence test | **KILLED** — the required non-newer rejection disappeared. |
| Commit new epoch/leaf/cursor while retaining the old exact membership | Leaf-4 exact replacement test | **KILLED** — omitted stale membership remained and the complete-record oracle failed. |
| Make the concrete cursor store expose a cursor/version update before the complete projection | reusable Leaf-4 atomic-store overlapping-reader test | **KILLED** — the reader observed a torn cursor projection snapshot. |
| Target abnormal-detach degradation at runtime generation N+1 instead of exact current N | server abnormal-delivery-stream-drop test | **KILLED** — replay rejected the fabricated successor generation as corrupt. |
| Remove verified external continuity identity from the cursor scope key | Leaf-4 continuity-scope test | **KILLED** — cross-native cursor state became visible. |

Focused clean authority checks also passed: all three `continuation_` promotion/replay tests, current attachment/runtime-generation independence, stale attachment-process fencing, known-cursor idempotency, external exact replacement/crash behavior, atomic-store conformance, and the operator-domain convergence vector.

## Full clean-tree suite

Run after every mutation was restored:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace targets/tests/doctests, including 35 runtime-evidence/promotion tests and 82 server unit tests; warnings-denied clippy passed.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — generated bindings clean; 57 vectors, 17 promoted, 26 implementation checks, and 38 mutation witnesses; model traceability current.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 27/27.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including real-core reconnect/restart E2E.
5. `cd web-cockpit && npm test`: **PASS** — 135/135.
6. `cd cli && npm test`: **PASS** — 48/48 plus the real-core resource projection.

The tracked tree was clean before mutation work, after every restore, before the full suite, and after the full suite. `git diff --check` passed. `/` retained 57G free.

## Recommendation

**Advance to done.** The pass-1 material finding is closed, legitimate reconnect remains green, every requested regression mutant stays killed, and the full clean-tree suite passes.
