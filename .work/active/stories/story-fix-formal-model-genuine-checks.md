---
id: story-fix-formal-model-genuine-checks
kind: story
stage: implementing
tags: [verification, bug, protocol, foundation]
parent: feature-formal-model-seed
depends_on: []
created: 2026-07-01
updated: 2026-07-01
gate_origin: null
release_binding: null
---

# Story: Fix self-defining properties in the seed formal models

Deep cross-model review (`openai-codex/gpt-5.5`, xhigh) of `feature-formal-model-seed` found that several promoted properties are **self-defining or vacuously true** — they pass for the wrong reason and would let real safety violations through undetected. This is the exact failure mode the verification program exists to prevent. Each was confirmed empirically (mutation test: break the predicate, invariant still passes).

## Blockers (must fix — these properties are promoted and CANNOT stand as product semantics)

### B1: `TypedCorrelation` is self-referential (`reply_correlation.qnt`)
The invariant's last conjunct is `replyIds.forall(r => recordedReplyOk(r))`, and `recordedReplyOk` calls `typedReferenceOk` — the *same* helper used in the action's `replyRecordable` filter. The invariant checks the exact predicate that gates recording.
- **Confirmed**: mutating `typedReferenceOk` to `true` (which would allow any reply to correlate to any id — total anti-forgery break) leaves `typed_correlation` still `[ok]`.
- **Fix**: separate the action's implementation predicate from an INDEPENDENT property oracle. The invariant must check raw id-space/context facts (`commandIds.contains(corrId)`, `replyContext.get(replyId) == commandContext.get(corrId)`, `corrType` ∈ {command,message}, reply-id ∉ command/message id spaces) — NOT via the helper that the action also uses. Then verify the mutation test FAILS (mutating the oracle breaks the invariant).

### B2: `CsrfRejectsMissingProof` (and the other CSRF invariants) are self-referential (`csrf_browser.qnt`)
`serverAccepts` (the action's acceptance rule) uses `validCsrfProof`, and the invariant `csrf_rejects_missing_proof` checks `accepted.implies(validCsrfProof(...))` — the same predicate.
- **Confirmed pattern** (same as B1): if `validCsrfProof` were broken (always true), acceptance lets anything through AND the invariant still passes.
- **Fix**: the invariant must independently assert raw facts not used by acceptance — e.g. `accepted.implies(csrfProofs.keys().contains(lastSession) and lastProof == csrfProofs.get(lastSession) and active(lastSession))`. Verify the mutation test fails.

### B3: `LateGenerationInert` is vacuously true (`session_generation.qnt`)
The agent collapsed 4 permissive actions into one conditional `step`. The `"late"` event kind is a dead stutter branch — it doesn't actually model a late tombstoned event binding to a generation. The property passes because generation only changes when `lsn` changes (structural), not because late events are proven inert.
- **Confirmed**: removing `"late"` from `EVENT_KINDS` leaves `late_generation_inert` still passing.
- **Fix**: model `late` as a real attempted event that binds to a `(sid, gen)` where `tombstoned.get((sid,gen))` is true, records a stale/audit outcome, and assert that specific transition cannot mutate live `generation`/`identityGeneration`. The property must be checked against a genuinely-exercised late path.

### B4: `GenerationMonotonic` proves weaker semantics than claimed (`session_generation.qnt`)
The property proves non-decrease + "no generation change when LSN unchanged" — NOT "supersession requires strictly-greater generation" (which is the PROTOCOL claim). The action's `if gen > generation` guard already enforces strictness by construction, so the property is partly self-defining.
- **Confirmed**: a temp mutation allowing `gen >= current` (equal reports superseding) still passed.
- **Fix**: track the attempted report generation/outcome as state; assert: if a report is attempted with `gen <= live`, then live generation and tombstone/live-target state do NOT supersede; if generation changes, the attempted `gen > old`.

### B5: Alloy `AuthorityGraphAcyclicAssert` is vacuous AND contradicts PROTOCOL (`patchbay-relational.als`)
`fact DelegationRemovedV0 { no Grant }` removes ALL grants, but `docs/PROTOCOL.md:290-307` says v0 HAS grants (only *delegation* — the parent-grant edge — is absent). The assert proves an empty graph is acyclic (theatre), and the model contradicts PROTOCOL's grant model.
- **Fix**: allow `Grant` atoms (v0 has grants). Model the DELEGATION edge (parent grant) as the absent-in-v0 relation, and assert acyclicity over THAT edge (the reserved seam). OR demote `AuthorityGraphAcyclicAssert` to `status: draft` / reserved until delegation is modeled, with a note explaining why (acyclicity is only meaningful once delegation exists).

### B6: Alloy `SenderMatchesClaimAssert` checks a fact (`patchbay-relational.als`)
`fact SenderMatchesClaim { all m: Message | m.sender = m.claimedSender }` forces the equality, then the assert checks the same. Checking a fact = tautology. Worse, PROTOCOL says sender identity comes from verified context, not self-asserted payload.
- **Fix**: remove the `fact`. Model `sender` (verified) and `claimedSender` (self-asserted) as independent fields; assert `all m: Message | m.sender = m.claimedSender` WITHOUT the fact forcing it. This makes it a genuine check that the consistency holds across all instances. (Note the Alloy skill's caveat: the *binding* of authenticated identity to transport is dynamic and belongs in `authority.qnt` — but the relational consistency shape IS a legitimate Alloy check IF the fact doesn't force it.)

## Acceptance criteria

- [ ] B1–B4 fixed; for each, the **mutation test** is run (break the predicate/invariant path → invariant FAILS; restore → invariant passes). Record the mutation-test result in implementation notes.
- [ ] B5/B6 fixed or demoted with documented reason.
- [ ] All checked properties still `[ok]`/`UNSAT` AFTER the fixes (re-run the full suite).
- [ ] No property passes for the wrong reason — the genuine-checking discipline from the design holds.

## Implementation notes

- The genuine-checking discipline is load-bearing: the action must be PERMISSIVE (allow the bad thing to be attempted) and the invariant must check the property via an INDEPENDENT path (not the same helper the action uses). The test: mutate the predicate that gates the action → if the invariant still passes, it's self-defining.
- This story supersedes the Unit 3/4 agents' "honest encoding" claims — their filter-in-action approach used the SAME predicate in both places, which is the bug.
- Reference: `specs/seed/command_lifecycle.qnt` (Unit 1) did this correctly — `boundary_dedup` checks `applyCount.get(k) <= 1` via a permissive `receive` action; the invariant is independent of the action's guard.
