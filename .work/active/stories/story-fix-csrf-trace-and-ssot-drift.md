---
id: story-fix-csrf-trace-and-ssot-drift
kind: story
stage: review
tags: [verification, foundation]
parent: feature-formal-model-seed
depends_on: []
created: 2026-07-01
updated: 2026-07-02
gate_origin: null
release_binding: null
---

# Story: Close the CSRF attempted-evidence trace gap + fix vocabulary-table SSOT drift

Second feature-level deep re-review (`openai-codex/gpt-5.5`, xhigh) found two important findings (not blockers). The promoted checks all genuinely hold (mutation-test proven); these are a deeper trace-fidelity gap and draft-property metadata drift. Filed together because both are small consistency fixes.

## Important findings

### I1 — CSRF `attemptedProof` is still action-recorded, not environment pre-state
`specs/seed/csrf_browser.qnt`. The B2-trace fix moved invariants from `lastProof` to `attemptedProof`, closing the recorded-trace lie. But `attemptedProof` is still assigned by the same action being checked, so a *combined* mutation (drop the proof check AND set `attemptedProof` to the bound proof) still passes `[ok]`.
- **Confirmed empirically**: `serverAccepts = authenticated and active` (drop proof) + `attemptedProof' = csrfProofs.get(session)` (lie) → `csrf_rejects_missing_proof` stays `[ok]`.
- **Root cause**: `attemptedProof` is action-assigned state, not pre-state/environment input. The accepting action can rewrite it while accepting.
- **Fix**: split request capture from server processing. Model the raw submitted evidence as PRE-STATE (e.g. a `pendingSession`/`pendingProof` pair set by a separate `receiveRequest` action, or a nondeterministic environment value the `submit` action reads but cannot rewrite). The accepting action reads the pending evidence; the invariant checks `accepted.implies(pendingProof == csrfProofs.get(pendingSession))`. Verify with the combined mutation test: breaking `serverAccepts` AND lying about the evidence must now fail the invariant.
- **Note**: this is the `idea-csrf-trace-fidelity` backlog pattern applied properly. The current fix is partial; this completes it.

### I2 — Vocabulary-table SSOT drift (draft properties)
`.work/active/features/feature-formal-model-seed.md`. Three mismatches between the feature body vocabulary table and the `@promotion` blocks / VERIFICATION.md:
- `TimeoutNeitherSuccessNorDenial`: table says `command_lifecycle.qnt`; VERIFICATION.md says "not in command_lifecycle.qnt — it concerns the submission/transport layer." Fix: change the table's model column to `<transport model (future)>` or note `reserved (not in any v0 model)`.
- `CompoundIssuer`: table says backend `tlc`; `authority.qnt` `@promotion` says `apalache`. Fix: align (draft placeholder — pick `apalache` to match the `@promotion` block, or mark `<TBD>`).
- `RevocationPreventsFuture`: table says `apalache`; `authority.qnt` `@promotion` says `apalache-temporal`. Fix: align to `apalache-temporal`.
- These are draft properties (not safety-failing), but the feature claims the vocabulary table is the SSOT, so the drift is a real consistency finding.

## Nit (from review, recorded here)
- `ActorIdsUniqueAssert` checks the same constraint as `fact ActorIdsUnique`. The model comment claims the assert "verifies non-vacuity" — it doesn't (a fact-consequence check can't establish non-vacuity without a separate `run`). Reword the comment to drop the non-vacuity claim, OR add an explicit non-vacuity `run` command if that claim matters. Low priority.

## Acceptance criteria
- [ ] I1: CSRF invariant consults pre-state environment evidence (not action-assigned `attemptedProof`); combined mutation (drop proof check + lie about evidence) → `[violation]`.
- [ ] I2: vocabulary table matches `@promotion` blocks and VERIFICATION.md for all three drift properties.
- [ ] All promoted checks still `[ok]`/`UNSAT` after the fix.

## Implementation notes
- The I1 fix is the `idea-csrf-trace-fidelity` pattern's correct completion. After this, the backlog item can note the pattern is now applied properly in csrf_browser.qnt and should be reused for authority.qnt's CompoundIssuer when promoted.

## Implementation notes

- Files changed: `specs/seed/csrf_browser.qnt` (I1), `specs/seed/patchbay-relational.als` (nit), `.work/active/features/feature-formal-model-seed.md` (I2), regenerated `csrf_browser.emitted.tla`.
- Tests added: none (verification is by running the checkers + mutation tests).
- Fixes:
  - **I1 (CSRF attempted-evidence)**: split request capture from server processing. Added an `arriveRequest` action that sets the RAW submitted evidence (`attemptedSession`/`attemptedProof`) as pre-state (plus `requestPending`); `submitStateChangingRequest` now takes no evidence args — it READS `attemptedSession`/`attemptedProof` but does NOT rewrite them. The accepting action can no longer lie about what was submitted. `arriveRequest` also resets `accepted=false` so the invariants reason about the current request, not a stale prior acceptance. The combined mutation (drop the proof check) now reliably fails `csrf_rejects_missing_proof` (`[violation]`) while `csrf_rejects_unauthenticated` stays `[ok]` (discriminating).
  - **Root-cause during implementation (test-integrity)**: the initial split broke the invariants (`[violation]` on the unchanged model) because `accepted` was a lingering outcome while `attemptedSession`/`attemptedProof` got overwritten by a new `arriveRequest`. Fixed by resetting `accepted=false` in `arriveRequest`. This is exactly the test-integrity discipline: the counterexample surfaced a real model bug (stale `accepted`), not a bad property.
  - **I2 (SSOT drift)**: fixed three vocabulary-table rows — `TimeoutNeitherSuccessNorDenial` model → `<transport model (future)>`; `CompoundIssuer` backend → `apalache` (matches @promotion); `RevocationPreventsFuture` backend → `apalache-temporal` (matches @promotion).
  - **Nit**: rewrote the `ActorIdsUniqueAssert` comment to drop the overstated non-vacuity claim (a fact-consequence check doesn't establish non-vacuity by itself).
- Mutation-test results (acceptance criterion):
  - I1: drop the proof check (`serverAccepts` ignores proof) → `csrf_rejects_missing_proof` `[violation]`, `csrf_rejects_unauthenticated` `[ok]` (discriminating) ✓
  - B1/B4 regression sweep: still genuine (`[violation]`) — not regressed by the CSRF edits.
  - All 3 CSRF invariants `[ok]` on the unchanged model; Alloy `ActorIdsUniqueAssert` UNSAT (0 skolems).
- Discrepancies from design: none — the I1 fix is the `idea-csrf-trace-fidelity` pattern's correct completion as the story specified.
- Adjacent issues parked: none new. The `idea-csrf-trace-fidelity` backlog pattern is now properly applied in csrf_browser.qnt; the note there should be updated to reflect the pre-state split is the correct shape (the authority.qnt CompoundIssuer, when promoted, should use the same environment-evidence split).
- Verification: all promoted CSRF invariants `[ok]`; combined mutation `[violation]`; SSOT consistency grep confirms draft authority backends match across feature body / @promotion / VERIFICATION.md.
