---
id: story-fix-formal-model-disclosure-drift
kind: story
stage: review
tags: [verification, foundation]
parent: feature-formal-model-seed
depends_on: []
created: 2026-07-01
updated: 2026-07-01
gate_origin: null
release_binding: null
---

# Story: Fix disclosure drift and emitted-artifact issues in the seed models

Deep cross-model review found several disclosure/drift issues that are not safety-critical but must be corrected before the feature advances — the feature's own acceptance criteria (consistency between `docs/VERIFICATION.md`, the feature body, and the `@promotion` blocks) are not met.

## Important findings

### I1: Feature body vocabulary-table backend drift
`.work/active/features/feature-formal-model-seed.md` lines ~66-67 still show `tlc` for `GenerationMonotonic` and `LateGenerationInert`, but the `@promotion` blocks in `session_generation.qnt` and the `docs/VERIFICATION.md` Seed-models table say `apalache-temporal`. The orchestrator's VERIFICATION.md update didn't propagate back to the feature body table.
- **Fix**: update the feature body vocabulary table so every `command_lifecycle.qnt`/`session_generation.qnt` temporal property reads `apalache-temporal` (not `tlc`), matching the `@promotion` blocks and VERIFICATION.md. Run a consistency grep across all three sources.

### I2: Malformed emitted TLA+ header (`reply_correlation.emitted.tla`)
The emitted TLA+ for `reply_correlation` has `MODULE reply_correlation ---` (no leading dashes) unlike the others (`--- MODULE csrf_browser ---`). The `sed` extraction in the agent's emit command grabbed a slightly different line.
- **Fix**: regenerate `reply_correlation.emitted.tla` with the same emit command used for the others; verify the header matches the `---- MODULE <name> ----` TLA+ form (or at minimum is consistent with the other emitted files). Re-verify it's a generated artifact (do-not-hand-edit header present).

### I3: Draft snapshot model omits normative variables (`snapshot_recovery.qnt`)
VERIFICATION's snapshot/recovery normative-variable list includes `SessionGeneration`, `AdapterGeneration`, `MessageId`, `ReplyId`, `CorrelationRef`, `SessionId`, `ActorId`, `RecoveredInbox`, `RecoveredSessionView` — several are absent from the draft model. Not blocking (it's draft), but the model under-claims the variable space.
- **Fix**: either add the missing variables as placeholders (typed but minimal), or narrow the model header comment to explicitly state which VERIFICATION variables are elided and why (draft — full variable set deferred to the follow-on implementation item).

### I4: Authority draft has dead actions (`authority.qnt`)
`rotateSession` and `revokeTarget` are defined but not reachable from `step`, despite reserved revocation/session properties.
- **Fix**: either wire them into `step` (so the model exercises revocation), or remove them and note that revocation dynamics are deferred to the follow-on authority item (consistent with the draft deferral reason).

## Acceptance criteria

- [ ] I1: feature body vocabulary table backends match `@promotion` blocks and VERIFICATION.md (grep confirms no `tlc` for temporal properties across all three sources).
- [ ] I2: `reply_correlation.emitted.tla` header consistent with the other emitted files.
- [ ] I3: snapshot_recovery draft either has the variables or an explicit elision note.
- [ ] I4: authority draft has no dead actions (wired in or removed with note).

## Implementation notes

- Files changed: `.work/active/features/feature-formal-model-seed.md` (I1), `specs/seed/authority.qnt` (I4), `specs/seed/snapshot_recovery.qnt` (I3). I2 (`reply_correlation.emitted.tla` header) was already fixed during the genuine-checks story implementation — all 4 emitted TLA+ artifacts now have consistent `---- MODULE <name> ----` headers.
- Tests added: none (substrate-hygiene fixes; verification is parse+compile for the draft models + consistency grep for I1).
- Fixes:
  - **I1**: updated the feature body vocabulary table — `GenerationMonotonic` and `LateGenerationInert` rows changed `tlc` → `apalache-temporal`, matching the `@promotion` blocks and `docs/VERIFICATION.md`. All 7 checked temporal properties are now consistently `apalache-temporal` across all three sources. (The remaining `tlc` references in the repo are all in DRAFT `@promotion` blocks / the draft rows of the feature body table — those are legitimate placeholder backends for not-yet-checked properties, not drift. The I1 criterion was about *checked* temporal properties, and those are all consistent.)
  - **I2**: already fixed (emitted TLA+ headers consistent).
  - **I3**: added an explicit ELIDED-variables note to the snapshot_recovery model header documenting which VERIFICATION normative variables are deferred (SessionGeneration, AdapterGeneration, MessageId, ReplyId, CorrelationRef, SessionId, ActorId, RecoveredInbox, RecoveredSessionView) and why (draft — view-variable reconciliation deferred to the follow-on promotion item).
  - **I4**: removed the dead `rotateSession` and `revokeTarget` actions from `authority.qnt`; added a deferral note explaining revocation/session-rotation dynamics (and the `RevocationPreventsFuture` reserved property) are deferred to the follow-on authority implementation item, consistent with the draft status. The `RevocationGeneration` and `SessionGeneration` state variables remain so the follow-on can wire them in. `step` now only calls the live `attemptSubmit`.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: `quint parse` + `quint compile` exit 0 for both draft models (authority, snapshot_recovery); I1 consistency grep confirms all checked temporal properties are `apalache-temporal` across feature body / @promotion blocks / VERIFICATION.md; I4 grep confirms 0 dead actions in authority.
