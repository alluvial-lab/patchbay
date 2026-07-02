---
id: story-fix-formal-model-disclosure-drift
kind: story
stage: implementing
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
