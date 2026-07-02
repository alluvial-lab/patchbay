---
id: story-fix-alloy-relational-assertions
kind: story
stage: done
tags: [verification, bug, foundation]
parent: feature-formal-model-seed
depends_on: []
created: 2026-07-01
updated: 2026-07-01
gate_origin: null
release_binding: null
---

# Story: Fix the failing Alloy relational assertions (B5/B6 regression)

Deep adversarial re-review (`openai-codex/gpt-5.5`, xhigh) found that the B5/B6 "fixes" from `story-fix-formal-model-genuine-checks` made things worse: removing the forcing facts (`fact DelegationRemovedV0 { no Grant }` and `fact SenderMatchesClaim`) turned the asserts from *vacuously true* into *actually false*. Alloy now finds counterexamples (the asserts are `SAT`, not `UNSAT`). The `@promotion` blocks claim `expected: pass` but the checks fail. This is the worst outcome for a safety-claiming artifact — provably wrong.

Confirmed by the host via `--type text` output: both asserts produce a skolem witness (counterexample).

## Blockers

### B5 — `AuthorityGraphAcyclicAssert` now FAILS (counterexample exists)
`specs/seed/patchbay-relational.als`. With `fact DelegationRemovedV0` removed, grants are present, but the subject→issuer graph is unconstrained. Alloy finds a counterexample with self-grants (`issuer = subject = Actor$2`), which is a 1-cycle. The assert is `SAT` (fails).
- **Root cause**: the assert checks acyclicity of the issuer graph, but nothing in the model prevents self-grants or cycles. v0 PROTOCOL does NOT state that grants form an acyclic issuer graph (grants are issued by actors to subjects; there's no delegation/parent-grant edge in v0). So the assert is checking an invented rule.
- **Fix**: this property is NOT a v0 safety claim. PROTOCOL makes `AuthorityGraphAcyclic` a *reserved seam* (meaningful only once delegation exists, which is explicitly out of v0). **Demote** `AuthorityGraphAcyclicAssert` to `status: draft` / `tier: stated-normative` with a `@promotion` block noting it's reserved for the delegation follow-on (acyclicity is only meaningful once a parent-grant edge exists). Remove the `check` command (or keep it but mark the property draft). Do NOT leave a promoted assert that fails.

### B6 — `SenderMatchesClaimAssert` now FAILS (counterexample exists)
`specs/seed/patchbay-relational.als`. With `fact SenderMatchesClaim` removed, nothing forces `sender = claimedSender`. Alloy finds `Message$0` with `sender=Actor$0, claimedSender=Actor$1` (they differ). The assert is `SAT` (fails).
- **Root cause**: the assert checks a *consistency shape* (sender == claimedSender), but in a relational snapshot with no dynamics, there's no constraint making them equal — they're independent fields. The property only holds if SOME constraint forces equality, and the only thing that did was the (removed) fact, which made it a tautology.
- **Assess honestly**: is `SenderMatchesClaim` a genuine v0 safety property, or is it inherently a *dynamic* property (the binding of authenticated identity to transport is CompoundIssuer-style, which the Alloy brief explicitly says belongs in `authority.qnt`, not Alloy)? Per the Alloy skill's caveat: "this models the *consistency shape* (sender ≠ self-asserted). The *binding* of an authenticated identity to a transport/session is a dynamic verification action — that belongs in the TLA+/Quint model, not Alloy." A relational snapshot can't prove the binding without a fact forcing it.
- **Fix**: **demote** `SenderMatchesClaimAssert` to `status: draft` / `tier: stated-normative` with a `@promotion` note recording that the *relational consistency shape* requires a dynamic binding proof (CompoundIssuer in `authority.qnt`) to be meaningful, and is reserved for the authority follow-on. OR, if a genuine relational check is wanted, redefine it as a shape that IS constrainable relationally (e.g. "every Message's sender is a known Actor" — trivially true given the sig, so also weak). The honest conclusion is likely: this property is not checkable as a relational invariant without becoming tautological; demote it.

## Important (also from the re-review, filed here for cohesive fix)

### B2-trace — CSRF invariant trace-fidelity weakness
`specs/seed/csrf_browser.qnt`. The B2 fix removed the helper self-reference, but the invariant trusts `lastProof` as a faithful copy of submitted evidence. The reviewer's deeper mutation: set `validCsrfProof(...) = true` AND `lastProof' = csrfProofs.get(session)` (the action lies about what was submitted) → `csrf_rejects_missing_proof` stayed `[ok]`.
- **Fix**: add explicit attempted/request evidence state (e.g. `attemptedSession`, `attemptedProof` — the raw submitted values, distinct from `lastSession`/`lastProof` which the action records). The invariant should check `accepted.implies(attemptedProof == csrfProofs.get(attemptedSession) and attemptedSession == lastSession ...)`. Verify with a mutation test that an action lying about the submitted proof is caught.

### B4-overclaim — `GenerationMonotonic` semantics overclaim
`specs/seed/session_generation.qnt`. The checked property is non-decrease, but the `@promotion` `semantics` field still says "supersession requires strictly-greater generation; equal/lower generation reports leave the live target unchanged." The reviewer's mutation: change the guard to `gen >= generation.get(sid)` (allowing equal supersession) → `generation_monotonic` stayed `[ok]` (because non-decrease still holds). So strict-supersession is unchecked but claimed.
- **Fix**: narrow the `@promotion` `semantics` field to match what's actually checked: "the live session generation never decreases" — and add a note that strict-supersession (equal/lower reports are no-ops) is a structural property of the action guard, not a checked temporal property (flagged to `idea-tlc-temporal-workaround`). Do NOT leave the semantics field claiming strict-supersession.

## Acceptance criteria

- [ ] B5: `AuthorityGraphAcyclicAssert` demoted to `status: draft` with a note (reserved for delegation follow-on); no promoted Alloy assert that fails.
- [ ] B6: `SenderMatchesClaimAssert` demoted to `status: draft` with a note (dynamic binding, reserved for authority follow-on); no promoted Alloy assert that fails.
- [ ] B2-trace: CSRF invariant uses attempted-evidence state; mutation test (action lies about submitted proof) → `[violation]`.
- [ ] B4-overclaim: `generation_monotonic` `@promotion` `semantics` narrowed to "non-decrease"; strict-supersession noted as structural.
- [ ] All remaining *promoted* Alloy asserts `UNSAT` (only `ActorIdsUniqueAssert` should remain promoted); all promoted Quint properties still `[ok]`.
- [ ] `docs/VERIFICATION.md` Seed-models table updated: `patchbay-relational.als` shows only `ActorIdsUnique` as checked; the other two as stated (draft).
- [ ] Feature body vocabulary table updated to match.

## Implementation notes

- The lesson: removing a forcing fact without adding a real constraint turns a vacuous-true into an actual-false. A genuine Alloy check must either (a) check a property that's true because of OTHER genuine constraints in the model (not the assert's own fact), or (b) be demoted if no such constraint exists. Neither B5 nor B6 has a relational constraint making it true without a tautological fact — so both demote.
- Measurement-discipline note: `--type json` / file-count is an unreliable UNSAT check; use `--type text` and look for a skolem witness (`skolem $<AssertName>_...`) — its presence means a counterexample was found (assert FAILS).

## Implementation notes

- Files changed: `specs/seed/patchbay-relational.als` (B5/B6 demotion), `specs/seed/session_generation.qnt` (B4 semantics narrowing), `specs/seed/csrf_browser.qnt` (B2-trace attempted-evidence), `docs/VERIFICATION.md` (table updates), `.work/active/features/feature-formal-model-seed.md` (vocabulary table). Regenerated `session_generation.emitted.tla` + `csrf_browser.emitted.tla`.
- Tests added: none (verification is by running the checkers + mutation tests).
- Fixes:
  - **B5 (`AuthorityGraphAcyclicAssert`)**: demoted to `status: draft` / `tier: stated-normative`. The assert is commented out and its `check` command removed — only `ActorIdsUniqueAssert` remains as a promoted `check`. Reason recorded: acyclicity is only meaningful once a delegation/parent-grant edge exists, which is out of v0 (PROTOCOL line 305); with grants present but no delegation edge, the assert is either vacuous (empty graph) or false (unconstrained self-grants). Reserved for the delegation follow-on.
  - **B6 (`SenderMatchesClaimAssert`)**: demoted to `status: draft` / `tier: stated-normative`. Assert commented out, `check` removed. Reason recorded: sender==claimedSender is a DYNAMIC consistency property (the binding is CompoundIssuer-style, belongs in `authority.qnt` per the Alloy brief's caveat); in a static snapshot, sender/claimedSender are independent fields — nothing forces equality except a fact, which makes the assert a tautology. Reserved for the authority follow-on.
  - **B2-trace**: added `attemptedSession`/`attemptedProof` state (raw submitted values, distinct from `lastSession`/`lastProof`). Rewrote all 4 CSRF invariants to consult the attempted evidence, not the recorded trace. Now an action that lies about the recorded trace (or drops the proof check) is caught via the `attemptedProof` oracle.
  - **B4-overclaim**: narrowed the `GenerationMonotonic` `@promotion` `semantics` field to "the live session generation never decreases (checked). Strict-supersession ... is NOT a checked temporal property" — matching what's actually checked.
  - **Measurement-discipline**: switched the Alloy `invocation` field to `--type text` (the reliable UNSAT method via skolem-witness check); recorded the `--type json`/file-count unreliability in the model comments.
- Mutation-test results (acceptance criterion):
  - B2 helper: break `validCsrfProof`→true → `csrf_rejects_missing_proof` `[violation]` ✓
  - B2-trace: break `serverAccepts` to drop the proof check (accept any proof) → `csrf_rejects_missing_proof` `[violation]` ✓ (and `csrf_rejects_unauthenticated` correctly stays `[ok]` — discriminating genuine check)
  - B5/B6: no longer promoted; the only promoted Alloy assert (`ActorIdsUniqueAssert`) is `UNSAT` (0 skolems = no counterexample).
  - B1/B3/B4 (from prior story): still genuine — not regressed by these edits (re-confirmed: `typed_correlation` `[ok]`, `late_generation_inert` `[ok]`, `generation_monotonic` `[ok]`, and their mutation tests still reproduce `[violation]`).
- Discrepancies from design: none — the demotions are the honest resolution the fix story specified.
- Verification: all promoted Quint properties `[ok]`; the one promoted Alloy assert `UNSAT`; both B2 mutation tests `[violation]`; VERIFICATION.md + feature body vocabulary tables updated to match (patchbay-relational.als shows only `ActorIdsUnique` checked; `AuthorityGraphAcyclic`/`SenderMatchesClaim` stated draft).

## Review (2026-07-01)

**Verdict**: Approve - fast-lane advance

**Lane**: fast (story with green verification; no `--deep` requested). Substrate mode.

**Blockers**: none. **Important**: none. **Nits**: none.

**Notes**: Re-verified everything myself with the reliable `--type text` (skolem-witness) method — the same measurement discipline this story's B5/B6 fix required (the prior false-UNSAT was a `--type json`/file-count error). The one promoted Alloy assert (`ActorIdsUniqueAssert`) is genuinely UNSAT (0 skolems); only 1 `check` command remains (B5/B6 demoted, no promoted assert can fail). All 4 promoted Quint checks `[ok]`. Re-ran all mutation tests myself: B2-trace (break `serverAccepts`) → `csrf_rejects_missing_proof` `[violation]` while `csrf_rejects_unauthenticated` stays `[ok]` (discriminating — proves the check targets the proof, not auth); B2 helper → `[violation]`; B1 regression → `[violation]` (not regressed). The attempted-evidence fix is the genuine-checking discipline correctly extended to the evidence layer. With this story done, all 4 child stories of `feature-formal-model-seed` are terminal, so the parent re-advances to review.
