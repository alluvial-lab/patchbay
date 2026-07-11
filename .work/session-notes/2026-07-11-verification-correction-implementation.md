# Session Note — Verification Claim Correction: Implementation + 6-Round Review Convergence

## Context

Picked up `epic-public-product-contract-verification-claim-correction` (designed the prior session) and drove it through implementation and a deep-review convergence loop. The feature makes every Patchbay formal-verification artifact claim only what its formula, modeled failure boundary, and independent evidence support — demoting overclaiming/defective properties, removing misleading formulas, narrowing retained semantics, and reconciling stale prose.

## What shipped

**8 child stories** (Units 1–8), all `done`. The original design had 6; Units 7–8 were added by the review loop.

| Unit | Story | Theme |
|---|---|---|
| 1 | `…-command-lifecycle` | Demote 5 overclaiming `command_lifecycle.qnt` properties |
| 2 | `…-session-elicitation` | Demote 4 overclaiming session/elicitation properties |
| 3 | `…-alloy-and-toys` | Demote `ActorIdsUnique`; relocate 4 toy artifacts to skill dirs |
| 4 | `…-draft-formulas` | Remove 11 misleading draft `val`/`temporal` definitions |
| 5 | `…-prose` | Fix stale prose across PROTOCOL/VERIFICATION/ADAPTER-PI/SPEC/GLOSSARY |
| 6 | `…-retained-semantics` | Narrow 7 retained promoted properties' `semantics:` text |
| 7 | `…-trace-fidelity-demotion` | Demote 4 trace-fidelity-defective promoted properties (round-2 finding) |
| 8 | `…-mutation-fragility-demotion` | Remove 9 leftover formulas + demote 9 mutation-fragile survivors (round-4 finding) |

**Final tiers:** 8 promoted / 39 stated-normative = 47 total (was 32/15). 24 properties demoted, 24 defective `val`/`temporal` formulas removed entirely (not stubbed to `true`).

**The 8 promoted survivors** (each independently mutation-tested in round 5 and confirmed to catch its claim-breaking mutation):
- `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted` (command_lifecycle — transition/structural invariants)
- `GenerationMonotonic` (session_generation — non-decrease; strict-supersession is action-enforced, not checked)
- `CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority` (csrf_browser — deliberately fixed to inspect `attemptedSession`/`attemptedProof` raw evidence, not recorded trace)

## The 6-round review loop

Two-phase fresh-context deep review on `openai-codex` (different model class from the umans orchestrator). The loop escalated three times before converging:

| Round | Found | Outcome |
|---|---|---|
| 1 | 4 prose drifts (hand-authored VERIFICATION.md descriptions using pre-narrowing wording) | fixed inline |
| 2 | 4 trace-fidelity-defective promoted properties (invariants inspect state the accepting action writes) | Unit 7 demoted them |
| 3 | claimed 9 more survivors mutation-fragile; host **wrongly** disputed (tested wrong branch) | routed, then retracted |
| 4 | host retracted dispute; 9 confirmed fragile + 9 leftover formulas from Units 1-2 | Unit 8 demoted + removed |
| 5 | 8 survivors confirmed sound; 2 prose drifts Unit 8 left behind | fixed inline |
| 6 | clean confirmation | **converged** |

## Key lessons (recorded in the feature's review history)

1. **Run the *right* mutation.** The host's round-3 dispute was wrong because it tested the already-terminal branch (which the `__saved_` baseline catches) instead of the first-terminal branch (which the reviewer used). A mutation that breaks the property's *actual claim* is the test that matters, not just any mutation. The host must reproduce the reviewer's exact mutation before disputing it.

2. **Demotion campaigns must include a prose-reconciliation pass.** Rounds 1, 3, and 5 each found hand-authored prose drift that mechanical demotions left behind. The generated VERIFICATION.md tables regenerate from `@promotion` blocks, but hand-authored lists drift. The discipline: after any demotion, grep every demoted property name across *all* docs and check no surviving prose calls it "checked-model" or "verified."

3. **Phase 7 verification must check the removal discipline, not just the tier change.** Units 1-2 demoted `@promotion` status to draft but left the `val`/`temporal` definitions in place — so `quint verify --invariant command_durability` still *passed* on a draft property, exactly the vacuous-pass the feature exists to prevent. The host's Phase 7 verification missed this across 6 original waves. The check: after a demotion, confirm the `val`/`temporal` definition is actually gone, not just that `status:` changed.

4. **Scope routing vs. absorption.** The host initially routed the 9-property trace-fidelity finding to the v1 gate (`executable-release-assurance`) as "model-architecture work, not claim correction." That was correct for the *re-architecture* (attempted-evidence state) but wrong for the *demotion* of properties with confirmed mutation failure — demoting properties whose formulas don't independently support their claims is squarely this feature's job. The operator caught the lifecycle inconsistency (feature marked `done` while a review loop was still open) and the scope question.

## What got dropped vs. reduced

- **Dropped entirely:** 4 toy artifacts (relocated to skill example dirs); 24 defective `val`/`temporal` formulas (removed, not stubbed).
- **Reduced to claims only:** 24 properties demoted (property ids survive as stated-normative obligations via `@promotion` blocks; checked-model status and formulas removed).
- **Kept as structures:** all 7 Quint model files + the Alloy file remain in `specs/seed/`. 5 of 7 now have zero promoted properties (hollow reservations: model vocabulary + property ids, no executable checking). The design framed this as "demotion-and-honesty, not a rewrite" — preserve the structures and vocabulary for the v1 gate to inherit. Operator confirmed this is the right resting point.

## Routed to the v1 formal gate

`epic-public-product-contract-executable-release-assurance` (depends on this feature, still `drafting`) has a design-input section recording:
- Genuine formulas for the 24 demoted properties.
- The attempted-evidence model re-architecture: introduce immutable raw submitted-value state across the elicitation/subscription/reply-correlation model families (the CSRF models already do this; the others don't).
- The independent-oracle question is part of what makes a checker "real" rather than metadata.

`idea-csrf-trace-fidelity` (backlog) carries the full pattern description + the 4-round convergence record.

## Tooling notes

- **Quint** is at `~/.npm-global/bin/quint` (v0.32.0), NOT on default PATH. Prefix: `export PATH="$HOME/.npm-global/bin:$PATH"`.
- **Checker command order is load-bearing:** `check-vectors.mjs` exits 0 on regeneration (only exits 1 for validation failures); `check-models.mjs` exits 1 on the first run after a tier change (regenerates the model table), then 0 on the second. Sequence: `check-vectors` → `check-models` → `check-models`.
- `check-models.mjs` reads the `semantics:` field from `@promotion` blocks into the generated VERIFICATION.md table row — narrowing a block's semantics auto-regenerates its row. Do not hand-edit generated rows.
- `check-models.mjs` does NOT enforce the demotion discipline (no requirement for `demotion_reason` on drafts, no validation that draft invocations are `<TBD>`, no check that executable definitions are absent for stated-normative properties). Filed as `idea-check-models-draft-discipline-enforcement` (backlog) — needs the 3-way distinction (formula-less reservations / demoted-but-retained / forbidden vacuous stubs).

## Current State

- **Feature**: `epic-public-product-contract-verification-claim-correction` at `stage: done` (not archived — parent epic still `implementing`).
- **8 child stories**: all `done`.
- **Parent epic** `epic-public-product-contract`: still `implementing`; its other 5 child features (public-compatibility, self-hosted-operations, adapter-portability-proof, publication-governance, executable-release-assurance) are all `drafting`.
- **Working tree**: clean (modulo gitignored `_apalache-out/`).
- **Final commit**: `a8a5a5d` (fast-advance 8 stories to done).

## Commit chain (this session)

```
eae524e — Unit 1 (command-lifecycle)
9faa6c0 — Unit 2 (session-elicitation)
6dd69ca — Unit 3 (alloy-and-toys)
c4e8279 — Unit 4 (draft-formulas)
48fe336 — Unit 5 (prose)
15276ce — Unit 6 (retained-semantics)
6676bb7 — advance feature to review (premature — bounced later)
036c283 — review round 1 (4 prose fixes)
6dd69ca..15276ce — Units 1-6
19f54c4 — Unit 7 (trace-fidelity, round-2 finding)
1e8d215 — review round 2 (bounce done->implementing, file Unit 7)
a68ee66 — cleanup (refine draft-discipline backlog, drop superseded note)
139ac0b — disposition round 3 (route trace-fidelity to v1 gate)
434d461 — review round 4 (file Unit 8)
4702b9a — Unit 8 (mutation-fragility, round-4 finding)
b2f377e — review round 5 (fix prose drift, roll handoffs)
3fd764d — review round 6 (converged, feature done)
a8a5a5d — fast-advance 8 stories to done
```
